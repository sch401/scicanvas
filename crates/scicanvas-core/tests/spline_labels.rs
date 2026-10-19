use scicanvas_core::{
    history::History,
    ops::Source,
    page::LabelSource,
    plotspec::Transform,
    scene::{self, Prim, Scene},
    spline, svg, Column, PlotType, Project, Role,
};

fn project(kind: PlotType) -> (Project, u64) {
    let mut p = Project::default();
    p.new_book();
    let b = p.book_mut(1).unwrap();
    b.columns[0].data = ["0", "1", "2", "3"].map(str::to_string).to_vec().into();
    b.columns[1].data = ["0", "1", "1", "0"].map(str::to_string).to_vec().into();
    let (id, _) = p.plot_from_selection(1, &[], kind).unwrap();
    (p, id)
}
fn labels(sc: &Scene) -> Vec<&str> {
    sc.items
        .iter()
        .filter_map(|it| match &it.prim {
            Prim::Text { text, .. } if it.clip.is_some() => Some(text.as_str()),
            _ => None,
        })
        .collect()
}
fn near(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-10, "{a} != {b}");
}

#[test]
fn natural_spline_matches_analytic_and_scipy_reference() {
    let v = spline::natural(&[[0.0, 0.0], [1.0, 1.0], [2.0, 0.0]], 2).unwrap();
    near(v[1][1], 0.6875);
    near(v[3][1], 0.6875);
    // SciPy 1.17.1 CubicSpline(x,y,bc_type='natural'), generated locally.
    let points = [[0.0, 2.0], [0.7, -1.0], [1.8, 3.0], [3.0, 2.0], [5.0, 4.0]];
    let actual = spline::natural(&points, 4).unwrap();
    let expected = [
        2.0,
        0.9344254675337236,
        -0.004919251946042175,
        -0.6918043454527871,
        -1.0,
        -0.5830395090437095,
        0.5658733527584006,
        1.9318495381813094,
        3.0,
        3.386274852720998,
        3.131160666834885,
        2.560466147531328,
        2.0,
        1.683243215717613,
        2.0665636751058436,
        2.9166022969411527,
        4.0,
    ];
    for (p, y) in actual.iter().zip(expected) {
        near(p[1], y);
    }
    assert_eq!(spline::natural(&[], 16).unwrap(), Vec::<[f64; 2]>::new());
    assert!(spline::natural(&[[0.0, 1.0], [0.0, 2.0]], 16).is_err());
    assert!(spline::natural(&[[0.0, f64::NAN]], 16).is_err());
    let line = spline::natural(&[[0.0, 1.0], [2.0, 5.0]], 4).unwrap();
    near(line[1][1], 2.0);
}

#[test]
fn spline_autoscale_symbols_gaps_duplicates_and_fit_are_independent() {
    let (mut p, id) = project(PlotType::SplineSymbol);
    p.fit_poly(&Source::Graph(id), 1).unwrap();
    let sc = scene::layout_page(p.graph(id).unwrap(), &p, &[]);
    assert!(sc.ya.hi >= 1.15);
    assert!(sc
        .items
        .iter()
        .any(|it| matches!(&it.prim, Prim::Polyline { pts, .. } if pts.len() == 49)));
    assert!(sc
        .items
        .iter()
        .any(|it| matches!(&it.prim, Prim::Polyline { pts, .. } if pts.len() == 200)));
    assert!(sc
        .items
        .iter()
        .any(|it| matches!(&it.prim, Prim::Symbols { pts, .. } if pts.len() == 4)));
    let b = p.book_mut(1).unwrap();
    b.columns[0].data = ["2", "1", "1", "3"].map(str::to_string).to_vec().into();
    let sc = scene::layout_page(p.graph(id).unwrap(), &p, &[]);
    assert!(sc
        .items
        .iter()
        .any(|it| matches!(&it.prim, Prim::Polyline { pts, .. } if pts.len() == 33)));
    p.graph_mut(id).unwrap().first_layer_mut().line_only.clear();
    p.graph_mut(id).unwrap().first_layer_mut().y_keys = vec!["B".into()];
    p.book_mut(1).unwrap().columns[1].data.set_text(2, String::new());
    let sc = scene::layout_page(p.graph(id).unwrap(), &p, &[]);
    assert_eq!(
        sc.items
            .iter()
            .filter(|it| it.clip.is_some() && matches!(it.prim, Prim::Polyline { .. }))
            .count(),
        1
    );
}

#[test]
fn drop_lines_use_zero_or_log_boundary_and_svg_shares_segments() {
    let (mut p, id) = project(PlotType::VerticalDrop);
    p.book_mut(1).unwrap().columns[1].data = ["-2", "3", "4", "-1"].map(str::to_string).to_vec().into();
    for log in [false, true] {
        p.graph_mut(id).unwrap().first_layer_mut().y_log = log;
        let sc = scene::layout_page(p.graph(id).unwrap(), &p, &[]);
        let segs = sc
            .items
            .iter()
            .find_map(|it| match &it.prim {
                Prim::Segments { segs, .. } if it.clip.is_some() => Some(segs),
                _ => None,
            })
            .unwrap();
        assert_eq!(segs.len(), if log { 2 } else { 4 });
        let base = sc.py(if log { sc.ya.lo } else { 0.0 });
        let output = svg::render_page(p.graph(id).unwrap(), &p);
        for [x, y, x2, y2] in segs {
            near(*y, base);
            near(*x, *x2);
            assert!(output.contains(&format!("M{x:.2},{y:.2}L{x2:.2},{y2:.2}")));
        }
    }
}

#[test]
fn label_sources_keep_rows_escape_text_and_exclude_fit_samples() {
    let (mut p, id) = project(PlotType::SplineSymbol);
    p.fit_poly(&Source::Graph(id), 1).unwrap();
    let b = p.book_mut(1).unwrap();
    let mut text = Column::new("Z", Role::Ignore);
    text.data = ["A<&", "B", "C", "D"].map(str::to_string).to_vec().into();
    b.columns.push(text);
    b.columns[1].data.set_text(1, String::new());
    let l = p.graph_mut(id).unwrap().first_layer_mut();
    l.labels.visible = true;
    l.labels.source = LabelSource::Row;
    assert_eq!(
        labels(&scene::layout_page(p.graph(id).unwrap(), &p, &[])),
        ["1", "3", "4"]
    );
    let l = p.graph_mut(id).unwrap().first_layer_mut();
    l.labels.source = LabelSource::Column;
    l.labels.column = Some("Z".into());
    assert_eq!(
        labels(&scene::layout_page(p.graph(id).unwrap(), &p, &[])),
        ["A<&", "C", "D"]
    );
    assert!(svg::render_page(p.graph(id).unwrap(), &p).contains("A&lt;&amp;"));
    p.delete_columns(1, &["Z".into()]);
    assert!(labels(&scene::layout_page(p.graph(id).unwrap(), &p, &[])).is_empty());
    let l = p.graph_mut(id).unwrap().first_layer_mut();
    l.labels.source = LabelSource::XY;
    assert_eq!(labels(&scene::layout_page(p.graph(id).unwrap(), &p, &[]))[0], "(0, 0)");
    p.graph_mut(id).unwrap().first_layer_mut().transform = Transform::Mean;
    p.graph_mut(id).unwrap().first_layer_mut().labels.source = LabelSource::Row;
    assert!(labels(&scene::layout_page(p.graph(id).unwrap(), &p, &[])).is_empty());
}

#[test]
fn bar_labels_use_segment_values_positions_and_independent_layers() {
    let (mut p, id) = project(PlotType::StackedBar);
    let b = p.book_mut(1).unwrap();
    let mut c = Column::new("C", Role::Y);
    c.data = vec!["3".into(); 4].into();
    b.columns.push(c);
    let l = p.graph_mut(id).unwrap().first_layer_mut();
    l.y_keys.push("C".into());
    l.labels.visible = true;
    let sc = scene::layout_page(p.graph(id).unwrap(), &p, &[]);
    assert_eq!(labels(&sc), ["0", "1", "1", "0", "3", "3", "3", "3"]);
    let mut other = p.graph(id).unwrap().first_layer().clone();
    other.labels.visible = false;
    p.graph_mut(id).unwrap().page.add_layer(other);
    assert_eq!(labels(&scene::layout_page(p.graph(id).unwrap(), &p, &[])).len(), 8);
    p.graph_mut(id)
        .unwrap()
        .first_layer_mut()
        .set_plot_type(PlotType::PercentStackedBar)
        .unwrap();
    assert!(labels(&scene::layout_page(p.graph(id).unwrap(), &p, &[]))
        .iter()
        .all(|s| s.ends_with('%')));
    p.graph_mut(id)
        .unwrap()
        .first_layer_mut()
        .set_plot_type(PlotType::HorizontalBar)
        .unwrap();
    let sc = scene::layout_page(p.graph(id).unwrap(), &p, &[]);
    assert_eq!(labels(&sc), ["0", "1", "1", "0", "3", "3", "3", "3"]);
    for it in &sc.items {
        if let Prim::Text { x, y, .. } = it.prim {
            if let Some([l, t, w, h]) = it.clip {
                assert!(x >= l && x <= l + w && y >= t && y <= t + h);
            }
        }
    }
}

#[test]
fn type_labels_and_columns_restore_export_through_history() {
    let (mut p, id) = project(PlotType::Line);
    let original = p.clone();
    let mut h = History::default();
    h.begin(&p, "labels");
    let l = p.graph_mut(id).unwrap().first_layer_mut();
    l.set_plot_type(PlotType::SplineSymbol).unwrap();
    l.labels.visible = true;
    l.labels.offset = 12.0;
    h.changed();
    h.commit(&p);
    let after = p.clone();
    let output = svg::render_page(p.graph(id).unwrap(), &p);
    p = h.undo(&p).unwrap();
    assert_eq!(p, original);
    p = h.redo(&p).unwrap();
    assert_eq!(p, after);
    assert_eq!(svg::render_page(p.graph(id).unwrap(), &p), output);
}

#[cfg(feature = "serde")]
#[test]
fn v7_roundtrip_and_v6_defaults_preserve_old_plots() {
    use scicanvas_core::project_file;
    let (mut p, id) = project(PlotType::SplineSymbol);
    p.graph_mut(id).unwrap().first_layer_mut().labels.visible = true;
    assert_eq!(project_file::from_json(&project_file::to_json(&p).unwrap()).unwrap(), p);
    let (p, id) = project(PlotType::Line);
    let mut v: serde_json::Value = serde_json::from_str(&project_file::to_json(&p).unwrap()).unwrap();
    v["version"] = 6.into();
    v["graphs"][0]["page"]["layers"][0]
        .as_object_mut()
        .unwrap()
        .remove("labels");
    let loaded = project_file::from_json(&v.to_string()).unwrap();
    assert!(!loaded.graph(id).unwrap().first_layer().labels.visible);
    assert_eq!(
        svg::render_page(p.graph(id).unwrap(), &p),
        svg::render_page(loaded.graph(id).unwrap(), &loaded)
    );
    let mut bad = p;
    bad.graph_mut(id).unwrap().first_layer_mut().labels.size = f32::NAN;
    assert!(project_file::to_json(&bad).is_err());
}

#[test]
fn label_and_z_columns_are_not_curves_and_label_column_feeds_data_labels() {
    let (mut p, id) = project(PlotType::Scatter);
    let b = p.book_mut(1).unwrap();
    let mut name = Column::new("C", Role::Label);
    name.data = ["甲", "乙", "丙", "丁"].map(str::to_string).to_vec().into();
    let mut z = Column::new("D", Role::Z);
    z.data = ["5", "6", "7", "8"].map(str::to_string).to_vec().into();
    b.columns.push(name);
    b.columns.push(z);
    // 全表作图和分析只取 Y 列
    let (all, _) = p.plot_from_selection(1, &[], PlotType::Line).unwrap();
    assert_eq!(p.graph(all).unwrap().first_layer().y_keys, ["B"]);
    let src = Source::Book {
        id: 1,
        selected: vec![],
        rows: None,
    };
    assert_eq!(p.resolve(&src).unwrap().items.len(), 1);
    // 指定列文本而未指定列：用标签列
    let l = p.graph_mut(id).unwrap().first_layer_mut();
    l.labels.visible = true;
    l.labels.source = LabelSource::Column;
    l.labels.column = None;
    assert_eq!(
        labels(&scene::layout_page(p.graph(id).unwrap(), &p, &[])),
        ["甲", "乙", "丙", "丁"]
    );
    #[cfg(feature = "serde")]
    {
        let json = scicanvas_core::project_file::to_json(&p).unwrap();
        let back = scicanvas_core::project_file::from_json(&json).unwrap();
        let roles: Vec<Role> = back.book(1).unwrap().columns.iter().map(|c| c.role).collect();
        assert_eq!(roles[2..], [Role::Label, Role::Z]);
    }
}

#[test]
fn bspline_and_bezier_connections_smooth_data_lines_only() {
    let (mut p, id) = project(PlotType::Line);
    let b = p.book_mut(1).unwrap();
    b.columns[0].data = ["0", "1", "2", "3", "4", "5"].map(str::to_string).to_vec().into();
    b.columns[1].data = ["0", "2", "-1", "3", "0.5", "2"].map(str::to_string).to_vec().into();
    let lines = |p: &Project| -> Vec<usize> {
        scene::layout_page(p.graph(id).unwrap(), p, &[])
            .items
            .iter()
            .filter_map(|it| match &it.prim {
                Prim::Polyline { pts, .. } if it.clip.is_some() => Some(pts.len()),
                _ => None,
            })
            .collect()
    };
    assert_eq!(lines(&p), [6]);
    p.graph_mut(id).unwrap().first_layer_mut().connect = scicanvas_core::page::Connect::BSpline;
    assert_eq!(lines(&p), [3 * 16 + 1]);
    p.graph_mut(id).unwrap().first_layer_mut().connect = scicanvas_core::page::Connect::Bezier;
    assert_eq!(lines(&p), [2 * 16 + 1]);
}

#[test]
fn nearest_point_uses_segment_values_and_swapped_axes() {
    use scicanvas_core::coord::{Cartesian, Coord};
    for kind in [PlotType::StackedBar, PlotType::HorizontalBar] {
        let (mut p, id) = project(kind);
        let b = p.book_mut(1).unwrap();
        let mut c = Column::new("C", Role::Y);
        c.data = vec!["3".into(); 4].into();
        b.columns.push(c);
        p.graph_mut(id).unwrap().first_layer_mut().y_keys.push("C".into());
        let sc = scene::layout_page(p.graph(id).unwrap(), &p, &[]);
        let view = &sc.layers[0];
        let layer = p.graph(id).unwrap().first_layer();
        let map = Cartesian {
            x: &view.xa,
            y: &view.ya,
            rect: view.plot,
        };
        // 第二条曲线（C）第 2 行：堆叠后终点为 1 + 3，自身值为 3
        let (_, series) = layer.series(p.book(1).unwrap());
        let pt = series[1].points[1];
        let at = map.project(if kind == PlotType::HorizontalBar {
            [pt[1], pt[0]]
        } else {
            pt
        });
        let hit = scene::nearest_point(layer, view, p.book(1), at[0] + 1.0, at[1], 6.0).unwrap();
        assert_eq!(hit.row, Some(1));
        assert_eq!(hit.y, if kind == PlotType::StackedBar { 3.0 } else { pt[1] });
        assert!(scene::nearest_point(layer, view, None, at[0], at[1], 6.0).is_none());
    }
}

#[test]
fn datetime_x_columns_get_date_tick_labels() {
    let (mut p, id) = project(PlotType::LineSymbol);
    let b = p.book_mut(1).unwrap();
    b.columns[0].data = ["2026-10-18", "2026-10-19", "2026-10-20", "2026-10-21"]
        .map(str::to_string)
        .to_vec()
        .into();
    b.convert_columns(&["A".into()], scicanvas_core::DataKind::DateTime)
        .unwrap();
    let texts: Vec<String> = scene::layout_page(p.graph(id).unwrap(), &p, &[])
        .items
        .iter()
        .filter_map(|it| match &it.prim {
            Prim::Text { text, .. } if it.clip.is_none() => Some(text.clone()),
            _ => None,
        })
        .collect();
    for day in ["2026-10-18", "2026-10-19", "2026-10-20", "2026-10-21"] {
        assert!(texts.iter().any(|t| t == day), "{texts:?}");
    }
    // Y 轴数字格式
    p.graph_mut(id).unwrap().first_layer_mut().y_format = scicanvas_core::format::NumberFormat::Decimal { places: 2 };
    let texts: Vec<String> = scene::layout_page(p.graph(id).unwrap(), &p, &[])
        .items
        .iter()
        .filter_map(|it| match &it.prim {
            Prim::Text { text, .. } if it.clip.is_none() => Some(text.clone()),
            _ => None,
        })
        .collect();
    assert!(
        texts.iter().any(|t| t == "0.20") && texts.iter().any(|t| t == "1.00"),
        "{texts:?}"
    );
}
