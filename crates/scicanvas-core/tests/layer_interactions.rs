use scicanvas_core::{
    analysis::SmoothMethod,
    ops::Source,
    page::Layer,
    plotspec::{PlotSpec, Transform},
    scene::{self, View},
    svg, PlotType, Project,
};

fn project() -> (Project, u64) {
    let mut p = Project::sample();
    let id = p.new_book();
    let book = p.book_mut(id).unwrap();
    book.name = "第二图层数据".into();
    // 独立解析参考：y = 3x + 7，11 个数据点。
    book.columns[0].data = (0..11).map(|i| i.to_string()).collect();
    book.columns[1].data = (0..11).map(|i| (3 * i + 7).to_string()).collect();
    let spec = PlotSpec::from_selection(book, &[], PlotType::Scatter).unwrap();
    p.graphs[0].page.add_layer(Layer {
        spec,
        ..Default::default()
    });
    p.graphs[0].page.columns = 2;
    (p, id)
}

#[test]
fn fit_smooth_and_stats_use_selected_layer_book_and_write_back_to_it() {
    let (mut p, id) = project();
    let original_book = p.book(1).unwrap().clone();
    let original_layer = p.graphs[0].page.layers()[0].clone();
    let src = Source::Layer { graph_id: 2, index: 1 };
    assert_eq!(p.resolve(&src).unwrap().book_id, id);
    assert_eq!(p.resolve(&Source::Graph(2)).unwrap().book_id, 1);
    let (gid, reports) = p.fit_poly(&src, 1).unwrap();
    assert_eq!(gid, 2);
    assert_eq!(reports.len(), 1);
    assert!((reports[0].fit.coef[0] - 7.0).abs() < 1e-10);
    assert!((reports[0].fit.coef[1] - 3.0).abs() < 1e-10);
    let keys = p.smooth(&src, SmoothMethod::MovingAverage, 3).unwrap();
    let layer = &p.graphs[0].page.layers()[1];
    assert!(layer.line_only.contains(&keys[0]));
    assert!(p.book(id).unwrap().column(&keys[0]).is_some());
    assert_eq!(p.graphs[0].page.layers()[0], original_layer);
    assert_eq!(p.book(1).unwrap(), &original_book);
    let stats = p.stats_on_columns(&src).unwrap();
    assert_eq!(p.book(stats).unwrap().columns[1].data, vec!["11"]);
}

#[test]
fn worksheet_results_find_nonfirst_layer_and_invalid_targets_are_empty_operations() {
    let (mut p, id) = project();
    let first = p.graphs[0].page.layers()[0].clone();
    let src = Source::Book {
        id,
        selected: vec!["B".into()],
        rows: None,
    };
    assert_eq!(p.fit_poly(&src, 1).unwrap().0, 2);
    assert_eq!(p.graphs.len(), 1);
    assert_eq!(p.graphs[0].page.layers()[0], first);
    let before = p.clone();
    assert!(p.fit_poly(&Source::Layer { graph_id: 2, index: 99 }, 1).is_err());
    assert_eq!(p, before);
    p.graphs[0].page.layers_mut()[1].transform = Transform::Mean;
    assert!(p.resolve(&Source::Layer { graph_id: 2, index: 1 }).is_err());
    assert!(p.resolve(&Source::Graph(2)).is_ok());
}

#[test]
fn each_layer_view_has_independent_axes_and_does_not_change_export() {
    let (p, _) = project();
    let graph = &p.graphs[0];
    let original = scene::build_page(graph, &p, 900.0, 500.0, None);
    let image = svg::render_page(graph, &p);
    let view = View {
        x: (2.0, 4.0),
        y: (10.0, 20.0),
    };
    let zoomed = scene::build_page_views(graph, &p, 900.0, 500.0, &[None, Some(view)]);
    assert_eq!(zoomed.layers[0], original.layers[0]);
    assert_eq!(zoomed.layers[1].view(), view);
    let [l, t, w, h] = zoomed.layers[1].plot;
    assert_eq!(zoomed.layers[1].data_at(l, t + h), (2.0, 10.0));
    assert_eq!(zoomed.layers[1].data_at(l + w, t), (4.0, 20.0));
    assert_eq!(svg::render_page(graph, &p), image);
}
