use scicanvas_core::{
    graph_theme::{GraphTheme, StyleClip},
    page::{CurveStyle, FunctionCurve, Layer},
    plotspec::DataRef,
    scene::{self, Prim},
    Project,
};
#[cfg(feature = "serde")]
#[test]
fn b5_function_numpy_samples_poles_save_and_range_changes() {
    let doc: serde_json::Value = serde_json::from_str(include_str!("reference/function_plots.json")).unwrap();
    for case in doc["cases"].as_array().unwrap() {
        let mut p = Project::default();
        let mut function = FunctionCurve::new(case["expr"].as_str().unwrap());
        function.points = 501;
        let id = p.plot_function(function, (-2., 2.), None).unwrap();
        assert!(p.books.is_empty());
        let sc = scene::layout_page(p.graph(id).unwrap(), &p, &[]);
        assert!(sc.items.iter().any(|i| matches!(i.prim, Prim::Polyline { .. })));
        let layer = p.graph(id).unwrap().first_layer();
        let mut temp = Project::default();
        let bid = temp.new_book();
        let (_, plots) = layer.prepared_plots_in(temp.book(bid).unwrap(), &|_| None);
        let points = &plots[0].series.points;
        let expected: Vec<_> = case["x"]
            .as_array()
            .unwrap()
            .iter()
            .zip(case["y"].as_array().unwrap())
            .filter_map(|(x, y)| Some([x.as_f64()?, y.as_f64()?]))
            .collect();
        assert_eq!(points.len(), expected.len());
        for (a, b) in points.iter().zip(expected) {
            assert!((a[0] - b[0]).abs() < 1e-12);
            assert!((a[1] - b[1]).abs() < 1e-10 * b[1].abs().max(1.));
        }
        if case["expr"] == "1/x" {
            assert!(plots[0].series.rows.windows(2).any(|r| r[1] > r[0] + 1));
        }
        let json = scicanvas_core::project_file::to_json(&p).unwrap();
        assert_eq!(p, scicanvas_core::project_file::from_json(&json).unwrap());
        p.graph_mut(id).unwrap().first_layer_mut().x_max = 4.;
        let (_, plots) = p
            .graph(id)
            .unwrap()
            .first_layer()
            .prepared_plots_in(temp.book(bid).unwrap(), &|_| None);
        assert_eq!(plots[0].series.points.last().unwrap()[0], 4.);
    }
}
#[test]
fn b5_functions_follow_data_and_log_spacing_and_survive_mapping_edits() {
    let mut p = Project::sample();
    let layer = p.graph_mut(2).unwrap().first_layer_mut();
    layer.functions.push(FunctionCurve::new("x*x"));
    let saved = layer.functions.clone();
    layer.edit_draft(|draft| draft.y_keys = vec!["B".into()]);
    assert_eq!(layer.functions, saved);
    let (_, plots) = p
        .graph(2)
        .unwrap()
        .first_layer()
        .prepared_plots_in(p.book(1).unwrap(), &|_| None);
    let data = &plots[0].series.points;
    let function = &plots.last().unwrap().series.points;
    assert_eq!(data.first().unwrap()[0], function.first().unwrap()[0]);
    assert_eq!(data.last().unwrap()[0], function.last().unwrap()[0]);
    let layer = p.graph_mut(2).unwrap().first_layer_mut();
    layer.x_auto = false;
    layer.x_log = true;
    layer.x_min = 1.;
    layer.x_max = 100.;
    layer.functions[0].points = 3;
    let (_, plots) = p
        .graph(2)
        .unwrap()
        .first_layer()
        .prepared_plots_in(p.book(1).unwrap(), &|_| None);
    assert!((plots.last().unwrap().series.points[1][0] - 10.).abs() < 1e-12);
    assert!(scicanvas_core::formula::compile_x("missing+x").is_err());
    assert!(scicanvas_core::formula::compile_x("mean(x)").is_err());
}
#[test]
fn b5_row_range_atomicity_and_raw_data() {
    let mut p = Project::sample();
    let raw = p.books[0].clone();
    let layer = p.graph_mut(2).unwrap().first_layer_mut();
    layer.set_source_rows(&DataRef::new(1, "B"), Some((1, 3))).unwrap();
    let before = layer.clone();
    assert!(layer.set_source_rows(&DataRef::new(1, "B"), Some((4, 2))).is_err());
    assert_eq!(*layer, before);
    assert_eq!(p.books[0], raw);
    let (_, plots) = p
        .graph(2)
        .unwrap()
        .first_layer()
        .prepared_plots_in(p.book(1).unwrap(), &|_| None);
    assert_eq!(
        plots.iter().find(|p| p.series.key == "B").unwrap().series.rows,
        vec![1, 2, 3]
    );
}
#[cfg(feature = "serde")]
#[test]
fn b5_themes_are_style_only_idempotent_and_roundtrip() {
    let source = Layer {
        line_width: 3.,
        frame: true,
        grid: true,
        ..Default::default()
    };
    let mut theme = GraphTheme::capture(&source);
    theme.name = "Science".into();
    let mut p = Project::sample();
    let before = p.graph(2).unwrap().first_layer().clone();
    theme.apply(p.graph_mut(2).unwrap().first_layer_mut());
    let after = p.graph(2).unwrap().first_layer().clone();
    theme.apply(p.graph_mut(2).unwrap().first_layer_mut());
    assert_eq!(after, *p.graph(2).unwrap().first_layer());
    assert_eq!(before.draft().y_keys, after.draft().y_keys);
    assert_eq!(
        (before.x_min, before.x_max, before.title),
        (after.x_min, after.x_max, after.title)
    );
    assert_eq!(theme, GraphTheme::from_json(&theme.to_json().unwrap()).unwrap());
    p.save_graph_theme(theme).unwrap();
    assert!(p.save_graph_theme(GraphTheme::nature()).is_err());
    assert!(p.remove_graph_theme(GraphTheme::BUILTIN).is_err());
    let json = scicanvas_core::project_file::to_json(&p).unwrap();
    assert_eq!(p, scicanvas_core::project_file::from_json(&json).unwrap());
}
#[test]
fn b5_curve_style_clip_excludes_source_labels_and_mappings() {
    let style = CurveStyle {
        legend_label: "source".into(),
        color_column: Some("A".into()),
        size_column: Some("B".into()),
        symbol_column: Some("C".into()),
        line_width: Some(3.),
        ..Default::default()
    };
    let StyleClip::Curve(clip) = StyleClip::curve(&style) else {
        panic!()
    };
    assert!(clip.legend_label.is_empty());
    assert_eq!(
        (clip.color_column, clip.size_column, clip.symbol_column),
        (None, None, None)
    );
    assert_eq!(clip.line_width, Some(3.));
}
