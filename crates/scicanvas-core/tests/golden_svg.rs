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
    cases.extend(drawing_cases());
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

fn drawing_project(kind: PlotType, keys: &[&str]) -> Project {
    let mut p = Project::default();
    let id = p.alloc_id();
    let mut book = presets::example();
    book.id = id;
    p.books.push(book);
    let keys: Vec<_> = keys.iter().map(|s| (*s).to_owned()).collect();
    let spec = PlotSpec::from_selection(&p.books[0], &keys, kind).unwrap();
    p.plot_spec(spec).unwrap();
    let layer = p.graphs[0].first_layer_mut();
    layer.title = "Patterns".into();
    layer.x_title = "X".into();
    layer.y_title = "Y".into();
    p
}
fn drawing_cases() -> Vec<(String, String)> {
    use scicanvas_core::page::{FillPattern, PatternKind, ShapeKind};
    let mut result = Vec::new();
    let mut p = drawing_project(PlotType::LineSymbol, &["B"]);
    let g = &mut p.graphs[0];
    g.first_layer_mut().legend = false;
    g.first_layer_mut().title = "Page shapes".into();
    for (kind, a, b) in [
        (ShapeKind::Line, [0.1, 0.12], [0.4, 0.12]),
        (ShapeKind::Arrow, [0.1, 0.3], [0.4, 0.45]),
        (ShapeKind::Rect, [0.5, 0.1], [0.82, 0.35]),
        (ShapeKind::Ellipse, [0.5, 0.55], [0.85, 0.82]),
    ] {
        let id = g.page.add_shape(kind, a, b).unwrap();
        let shape = g.page.shape_mut(id).unwrap();
        shape.stroke = [140, 30, 60];
        if matches!(kind, ShapeKind::Rect | ShapeKind::Ellipse) {
            shape.fill = Some(([30, 150, 180], 0.25));
        }
        shape.dashed = kind == ShapeKind::Ellipse;
    }
    result.push(("shape-objects".into(), svg::render_page(&p.graphs[0], &p)));
    let mut p = drawing_project(PlotType::Bar, &["B"]);
    let base = p.graphs[0].first_layer().clone();
    for _ in 1..6 {
        p.graphs[0].page.add_layer(base.clone());
    }
    p.graphs[0].page.columns = 3;
    p.graphs[0].page.height_mm = 140.;
    for (layer, kind) in p.graphs[0].page.layers_mut().iter_mut().zip(PatternKind::ALL) {
        layer.legend = false;
        layer.title = format!("{kind:?}");
        layer.set_plot_style(
            "B",
            Some(scicanvas_core::page::CurveStyle {
                pattern: Some(FillPattern {
                    kind,
                    spacing_pt: 4.,
                    ..Default::default()
                }),
                ..Default::default()
            }),
        );
    }
    result.push(("hatch-bars".into(), svg::render_page(&p.graphs[0], &p)));
    let mut p = drawing_project(PlotType::Area, &["B", "C"]);
    for (key, values) in [("B", vec![2., 3., 2., 4., 3.]), ("C", vec![-1., -2., -1., -3., -2.])] {
        p.books[0].columns.iter_mut().find(|c| c.key == key).unwrap().data =
            values.iter().map(ToString::to_string).collect();
    }
    for (key, kind) in [("B", PatternKind::Cross), ("C", PatternKind::Dots)] {
        p.graphs[0].first_layer_mut().set_plot_style(
            key,
            Some(scicanvas_core::page::CurveStyle {
                pattern: Some(FillPattern {
                    kind,
                    spacing_pt: 4.,
                    width_pt: 0.75,
                    ..Default::default()
                }),
                ..Default::default()
            }),
        );
    }
    result.push(("hatch-area".into(), svg::render_page(&p.graphs[0], &p)));
    result
}
