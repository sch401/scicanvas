#![cfg(feature = "serde")]
//! Q-Q 图的样本选择、参考线、轴标题及项目保存。
use scicanvas_core::{
    page::CurveStyle, plotspec::Transform, project_file, scene, Book, Column, DataKind, PlotType, Project, Role,
};

fn project(values: &[&str], x: Option<Column>) -> Project {
    let mut p = Project::default();
    let id = p.alloc_id();
    let mut book = Book::new(id, "正态性");
    book.columns.extend(x);
    let mut y = Column::new("A", Role::Y);
    y.long_name = "浓度".into();
    y.units = "mg/L".into();
    y.data = values.iter().map(|v| v.to_string()).collect();
    book.columns.push(y);
    p.books.push(book);
    p.plot_from_selection(id, &["A".into()], PlotType::QQ).unwrap();
    p
}

#[test]
fn qq_ignores_x_values_and_per_curve_plot_types() {
    for (kind, values) in [
        (DataKind::Num, ["", "NaN", "1", "1", "1"]),
        (
            DataKind::DateTime,
            ["2026-01-01", "2026-01-02", "2026-01-03", "2026-01-04", "2026-01-05"],
        ),
        (DataKind::Category, ["甲", "乙", "甲", "乙", "甲"]),
    ] {
        let mut x = Column::new("X", Role::X);
        x.data = values.into_iter().map(String::from).collect();
        x.data = x.data.converted(kind).unwrap().into();
        let mut p = project(&["8", "1", "3", "2", "6"], Some(x));
        let layer = p.graphs[0].first_layer_mut();
        layer.transform = Transform::Mean;
        layer.set_plot_style(
            "A",
            Some(CurveStyle {
                kind: Some(PlotType::Bar),
                ..Default::default()
            }),
        );
        let (_, plots) = p.graphs[0].first_layer().prepared_plots_in(&p.books[0], &|_| None);
        assert_eq!(plots[0].style.kind, PlotType::QQ);
        assert_eq!(
            plots[0].series.points.iter().map(|p| p[1]).collect::<Vec<_>>(),
            [1., 2., 3., 6., 8.]
        );
        assert_eq!(plots[0].paths.len(), 1);
        let sc = scene::layout_page(&p.graphs[0], &p, &[]);
        assert!(sc.layers[0].axes.x.lo < 0.0 && sc.layers[0].axes.x.hi > 0.0);
    }
}

#[test]
fn qq_skips_columns_with_fewer_than_three_finite_values() {
    for values in [&[][..], &["1"][..], &["NaN", "2", "inf", "3"][..]] {
        let p = project(values, None);
        let (_, plots) = p.graphs[0].first_layer().prepared_plots_in(&p.books[0], &|_| None);
        assert!(plots[0].series.points.is_empty());
        assert!(plots[0].paths.is_empty());
        assert!(plots[0].series.errors.is_empty());
        scene::layout_page(&p.graphs[0], &p, &[]);
    }
}

#[test]
fn qq_reference_line_is_rendered_and_sorted_points_have_no_source_row() {
    use scicanvas_core::coord::Coord;
    let p = project(&["1", "8", "2", "3", "20"], None);
    let graph = &p.graphs[0];
    let layer = graph.first_layer();
    let (_, plots) = layer.prepared_plots_in(&p.books[0], &|_| None);
    let plot = &plots[0];
    let sc = scene::layout_page(graph, &p, &[]);
    let view = &sc.layers[0];
    let map = view.axes.cartesian(view.plot, scicanvas_core::coord::YAxis::Left);
    let expected: Vec<_> = plot.paths[0].iter().map(|p| map.project(*p)).collect();
    let line = sc
        .items
        .iter()
        .find_map(|item| match &item.prim {
            scene::Prim::Polyline { pts, .. } if item.clip.is_some() => Some(pts),
            _ => None,
        })
        .unwrap();
    assert_eq!(line.len(), expected.len());
    // 整页排版在投影后缩放和平移，容许浮点运算顺序造成的亚像素舍入。
    for (actual, expected) in line.iter().flatten().zip(expected.iter().flatten()) {
        assert!((actual - expected).abs() < 1e-10, "{actual} != {expected}");
    }
    for p in &plot.paths[0] {
        assert!(p[1] >= view.axes.left_y.lo && p[1] <= view.axes.left_y.hi);
    }
    let [x, y] = map.project(plot.series.points[0]);
    let hit = scene::nearest_point(layer, view, &|id| p.book(id), x, y, 2.0).unwrap();
    assert_eq!(hit.row, None);
    assert_eq!([hit.x, hit.y], plot.series.points[0]);
}

#[test]
fn qq_axis_titles_and_v26_project_roundtrip() {
    let mut p = project(&["1", "3", "2", "8"], None);
    let layer = p.graphs[0].first_layer();
    assert_eq!(layer.x_axis_title(Some(&p.books[0])), "理论分位数");
    assert_eq!(layer.y_axis_title(Some(&p.books[0])), "浓度 (mg/L)");
    p.graphs[0].first_layer_mut().x_title = "自定义横轴".into();
    assert_eq!(p.graphs[0].first_layer().x_axis_title(Some(&p.books[0])), "自定义横轴");
    assert_eq!(project_file::from_scv(&project_file::to_scv(&p).unwrap()).unwrap(), p);
    assert_eq!(scicanvas_core::model::PROJECT_VERSION, 26);
    let legacy = project_file::to_json(&Project::sample()).unwrap();
    let mut legacy: serde_json::Value = serde_json::from_str(&legacy).unwrap();
    legacy["version"] = 25.into();
    let loaded = project_file::from_json(&legacy.to_string()).unwrap();
    assert_eq!(loaded.version, scicanvas_core::model::PROJECT_VERSION);
}

#[test]
fn qq_clears_measurement_errors_before_rendering() {
    let mut p = project(&["1", "8", "2", "3", "20"], None);
    for (key, role) in [("E", Role::YError), ("F", Role::XError)] {
        let mut error = Column::new(key, role);
        error.error_for = Some("A".into());
        error.data = vec!["10".to_string(); 5].into();
        p.books[0].columns.push(error);
    }
    let (_, plots) = p.graphs[0].first_layer().prepared_plots_in(&p.books[0], &|_| None);
    assert!(plots[0].series.errors.iter().all(|errors| *errors == [None, None]));
    let sc = scene::layout_page(&p.graphs[0], &p, &[]);
    assert!(!sc
        .items
        .iter()
        .any(|item| item.clip.is_some() && matches!(item.prim, scene::Prim::Segments { .. })));
}
