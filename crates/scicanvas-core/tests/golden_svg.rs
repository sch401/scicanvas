use scicanvas_core::{
    page::{LabelSource, LayerRect},
    plotspec::PlotSpec,
    presets, svg, Column, Graph, PlotType, Project, Role,
};
use std::{fs, path::PathBuf};

fn cases() -> Vec<(String, String)> {
    static INIT: std::sync::Once = std::sync::Once::new();
    INIT.call_once(|| assert!(scicanvas_core::text::initialize(scicanvas_core::text::Fonts::bundled()).is_ok()));
    let mut project = Project::default();
    let mut book = presets::example();
    // Include negative values and a missing row, not just the positive gallery example.
    book.columns[1].data = ["-2", "3", "", "5", "-1"].map(str::to_owned).to_vec().into();
    project.books.push(book);
    let mut cases = Vec::new();
    for preset in presets::all() {
        let mut project = project.clone();
        if matches!(preset.kind, PlotType::ErrorBand | PlotType::BubbleColor) {
            project.books[0] = presets::example_for(preset.kind);
        }
        let mut graph = Graph::new(20, preset.name);
        graph.first_layer_mut().spec = PlotSpec::from_selection(&project.books[0], &[], preset.kind)
            .unwrap()
            .into();
        if matches!(preset.kind, PlotType::PolarLine | PlotType::PolarScatter) {
            graph.first_layer_mut().projection = scicanvas_core::page::Projection::Polar { degrees: true };
        }
        cases.push((preset.id.to_owned(), svg::render_page(&graph, &project)));
    }

    for (key, role, values) in [
        ("D", Role::XError, ["0.1", "0.2", "0.1", "0.3", "0.2"]),
        ("E", Role::YError, ["0.5", "1", "", "0.4", "0.2"]),
    ] {
        let mut column = Column::new(key, role);
        column.error_for = Some(if role == Role::XError { "A" } else { "B" }.into());
        column.data = values.map(str::to_owned).to_vec().into();
        project.books[0].columns.push(column);
    }
    let mut graph = Graph::new(20, "Errors & labels <baseline>");
    graph.first_layer_mut().spec = PlotSpec::from_selection(&project.books[0], &["B".into()], PlotType::LineSymbol)
        .unwrap()
        .into();
    cases.push(("xy-errors".into(), svg::render_page(&graph, &project)));
    graph.first_layer_mut().labels.visible = true;
    graph.first_layer_mut().labels.source = LabelSource::XY;
    cases.push(("xy-labels".into(), svg::render_page(&graph, &project)));

    let mut inset = graph.first_layer().clone();
    inset.spec.set_plot_type(PlotType::Scatter).unwrap();
    inset.title = "Inset".into();
    inset.placement = Some(LayerRect {
        x: 0.5,
        y: 0.1,
        width: 0.45,
        height: 0.45,
    });
    inset.transparent = true;
    graph.page.add_layer(inset);
    cases.push(("inset".into(), svg::render_page(&graph, &project)));
    graph.page.layers_mut()[1].placement = None;
    graph.page.columns = 2;
    cases.push(("two-panels".into(), svg::render_page(&graph, &project)));
    graph.first_layer_mut().legend_position = Some([0.7, 0.7]);
    let id = graph.page.add_text([0.15, 0.06]).unwrap();
    graph.page.text_mut(id).unwrap().text = "Note_{1} α\n\\b{Result} R^{2}".into();
    cases.push(("annotation-legend".into(), svg::render_page(&graph, &project)));
    cases
}

fn directory() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden")
}

#[test]
fn svg_matches_reviewed_baselines() {
    let cases = cases();
    let mut expected: Vec<_> = cases.iter().map(|(name, _)| format!("{name}.svg")).collect();
    expected.sort();
    let mut actual: Vec<_> = fs::read_dir(directory())
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .filter(|name| name.ends_with(".svg"))
        .collect();
    actual.sort();
    assert_eq!(
        actual, expected,
        "Every preset and composed case needs a committed baseline"
    );
    for (name, rendered) in cases {
        let expected = fs::read(directory().join(format!("{name}.svg"))).unwrap();
        assert!(
            rendered.as_bytes() == expected,
            "SVG baseline changed: {name}; review the SVG diff before updating"
        );
    }
}

#[test]
#[ignore = "Explicit maintainer action; inspect generated SVG diffs before committing"]
fn update_svg_baselines() {
    fs::create_dir_all(directory()).unwrap();
    for (name, rendered) in cases() {
        fs::write(directory().join(format!("{name}.svg")), rendered).unwrap();
    }
}
