use scicanvas_core::{
    history::History,
    ops::Source,
    plotspec::Transform,
    scene::{self, Prim, Scene},
    svg, Column, PlotType, Project, Role,
};

fn col(key: &str, role: Role, values: &[&str]) -> Column {
    let mut c = Column::new(key, role);
    c.data = values.iter().map(|v| v.to_string()).collect();
    c
}

fn project() -> (Project, u64) {
    let mut p = Project::default();
    let bid = p.new_book();
    p.book_mut(bid).unwrap().columns = vec![
        col("A", Role::X, &["1", "2", "3"]),
        col("B", Role::Y, &["10", "20", "30"]),
        col("C", Role::YError, &["2", "3", "4"]),
        col("D", Role::XError, &["0.2", "0.3", "0.4"]),
    ];
    let (gid, _) = p.plot_from_selection(bid, &[], PlotType::LineSymbol).unwrap();
    (p, gid)
}

// Clipped solid segments distinguish data error bars from the unclipped axis ticks.
fn errors(sc: &Scene) -> Vec<&Vec<[f64; 4]>> {
    sc.items
        .iter()
        .filter_map(|item| match &item.prim {
            Prim::Segments {
                segs, dashed: false, ..
            } if item.clip.is_some() => Some(segs),
            _ => None,
        })
        .collect()
}

#[test]
fn association_supports_both_column_orders_and_shared_x_errors() {
    let (mut p, gid) = project();
    let g = p.graph(gid).unwrap().first_layer();
    assert_eq!(g.draft().y_keys, ["B"]);
    let (_, s) = g.series(p.book(1).unwrap());
    assert_eq!(s[0].errors[0], [Some(0.2), Some(2.0)]);
    let b = p.book_mut(1).unwrap();
    b.columns.swap(2, 3);
    let (_, s) = p.graph(gid).unwrap().first_layer().series(p.book(1).unwrap());
    assert_eq!(s[0].errors[0], [Some(0.2), Some(2.0)]);

    let b = p.book_mut(1).unwrap();
    b.columns = vec![
        col("A", Role::X, &["1"]),
        col("D", Role::XError, &["0.5"]),
        col("B", Role::Y, &["10"]),
        col("C", Role::YError, &["2"]),
        col("E", Role::Y, &["20"]),
    ];
    let (g2, _) = p.plot_from_selection(1, &[], PlotType::Scatter).unwrap();
    let (_, s) = p.graph(g2).unwrap().first_layer().series(p.book(1).unwrap());
    assert_eq!(s.len(), 2);
    assert_eq!(s[0].errors, [[Some(0.5), Some(2.0)]]);
    assert_eq!(s[1].errors, [[Some(0.5), None]]);
}

#[test]
fn errors_keep_original_rows_and_invalid_errors_keep_the_point() {
    let (mut p, gid) = project();
    let b = p.book_mut(1).unwrap();
    b.columns[0].data = ["1", "", "3", "4", "5", "6"].map(str::to_string).to_vec().into();
    b.columns[1].data = ["10", "20", "30", "40", "50", "60"].map(str::to_string).to_vec().into();
    b.columns[2].data = ["2", "99", "-1", "NaN", "inf", "0"].map(str::to_string).to_vec().into();
    b.columns[3].data = ["0.1", "99", "0.3"].map(str::to_string).to_vec().into();
    let layer = p.graph_mut(gid).unwrap().first_layer_mut();
    layer.rows = Some((1, 5));
    let (_, s) = p.graph(gid).unwrap().first_layer().series(p.book(1).unwrap());
    assert_eq!(s[0].points, [[3.0, 30.0], [4.0, 40.0], [5.0, 50.0], [6.0, 60.0]]);
    assert_eq!(
        s[0].errors,
        [[Some(0.3), None], [None, None], [None, None], [None, Some(0.0)]]
    );
}

#[test]
fn insertion_fit_deletion_and_key_reuse_preserve_ownership() {
    let (mut p, gid) = project();
    p.fit_poly(&Source::Graph(gid), 1).unwrap();
    let (_, s) = p.graph(gid).unwrap().first_layer().series(p.book(1).unwrap());
    assert_eq!(s[0].errors[2], [Some(0.4), Some(4.0)]);
    assert!(s[1].errors.iter().all(|e| *e == [None, None]));
    assert_eq!(p.book(1).unwrap().column("C").unwrap().error_for.as_deref(), Some("B"));
    p.book_mut(1).unwrap().insert_column(2, Role::Y);
    assert_eq!(
        p.book(1)
            .unwrap()
            .error_source(p.book(1).unwrap().column_index("C").unwrap()),
        Some("B")
    );
    p.delete_columns(1, &["B".into()]);
    let b = p.book_mut(1).unwrap();
    assert_eq!(b.insert_column(1, Role::Y), "B");
    assert_eq!(b.error_source(b.column_index("C").unwrap()), None);
    b.set_roles(&["C".into()], Role::YError);
    // Re-designating the column intentionally associates it with its current neighbour.
    assert!(b.error_source(b.column_index("C").unwrap()).is_some());
    p.delete_columns(1, &["C".into(), "D".into()]);
    let (_, s) = p.graph(gid).unwrap().first_layer().series(p.book(1).unwrap());
    assert!(s.iter().flat_map(|s| &s.errors).all(|e| *e == [None, None]));
}

#[test]
fn autoscale_svg_and_all_basic_chart_types_share_error_segments() {
    let (mut p, gid) = project();
    // 统计图不保留原始测量误差，由对应图型的测试单独核对清除。
    for kind in PlotType::ALL.into_iter().filter(|k| {
        !matches!(
            k,
            PlotType::QQ
                | PlotType::Box
                | PlotType::ErrorBand
                | PlotType::ColumnScatter
                | PlotType::Bubble
                | PlotType::ColorScatter
                | PlotType::BubbleColor
        ) && (!k.is_bar() || *k == PlotType::Bar)
    }) {
        p.graph_mut(gid).unwrap().first_layer_mut().plot_type = kind;
        let g = p.graph(gid).unwrap();
        let sc = scene::layout_page(g, &p, &[]);
        assert!(sc.axes.x.lo <= 0.8 && sc.axes.x.hi >= 3.4);
        assert!(sc.axes.left_y.lo <= 8.0 && sc.axes.left_y.hi >= 34.0);
        let groups = errors(&sc);
        assert_eq!(groups.len(), if kind == PlotType::VerticalDrop { 2 } else { 1 });
        let group = groups.last().unwrap();
        assert_eq!(group.len(), 18);
        let svg = svg::render_page(g, &p);
        for [x1, y1, x2, y2] in *group {
            assert!(svg.contains(&format!("M{x1:.2},{y1:.2}L{x2:.2},{y2:.2}")));
        }
        assert!(svg.contains(&format!("clip-path=\"url(#clip{gid})\"")));
    }
}

/// 版面按页宽整体放大后坐标会有末位舍入差异，按 1e-9 比较线段。
fn has(segs: &[[f64; 4]], want: [f64; 4]) -> bool {
    segs.iter()
        .any(|s| s.iter().zip(want).all(|(a, b)| (a - b).abs() < 1e-9))
}

#[test]
fn logarithmic_filtering_preserves_errors_and_clips_nonpositive_endpoints() {
    let (mut p, gid) = project();
    p.book_mut(1).unwrap().columns[1].data = ["-1", "20", "30"].map(str::to_string).to_vec().into();
    p.book_mut(1).unwrap().columns[2].data = ["999", "25", "4"].map(str::to_string).to_vec().into();
    let l = p.graph_mut(gid).unwrap().first_layer_mut();
    l.y_log = true;
    l.y_auto = false;
    l.y_min = 1.0;
    l.y_max = 100.0;
    let sc = scene::layout_page(p.graph(gid).unwrap(), &p, &[]);
    let groups = errors(&sc);
    assert_eq!(groups[0].len(), 11); // two XY errors; clipped lower Y endpoint has no cap
    assert!(groups[0].iter().flatten().all(|v| v.is_finite()));
    assert!(has(groups[0], [sc.px(2.0), sc.py(1.0), sc.px(2.0), sc.py(45.0)]));
    let svg = svg::render_page(p.graph(gid).unwrap(), &p);
    assert!(!svg.contains("NaN") && !svg.contains("inf"));
}

#[test]
fn grouped_bars_place_errors_at_each_bar_center() {
    let (mut p, gid) = project();
    let b = p.book_mut(1).unwrap();
    b.columns.push(col("E", Role::Y, &["15", "25", "35"]));
    b.columns.push(col("F", Role::YError, &["1", "2", "3"]));
    let l = p.graph_mut(gid).unwrap().first_layer_mut();
    l.edit_draft(|draft| draft.y_keys.push("E".into()));
    l.plot_type = PlotType::Bar;
    let sc = scene::layout_page(p.graph(gid).unwrap(), &p, &[]);
    let groups = errors(&sc);
    assert_eq!(groups.len(), 2);
    let x = sc.px(1.0 - 0.175);
    assert!(has(groups[0], [x, sc.py(8.0), x, sc.py(12.0)]));
    let x = sc.px(1.0 + 0.175);
    assert!(has(&groups[1][..1], [x, sc.py(14.0), x, sc.py(16.0)]));
}

#[test]
fn error_visibility_style_layers_and_mean_transform_are_independent() {
    let (mut p, gid) = project();
    let mut layer = p.graph(gid).unwrap().first_layer().clone();
    layer.error_bars.visible = false;
    layer.transform = Transform::Mean;
    p.graph_mut(gid).unwrap().page.add_layer(layer);
    let sc = scene::layout_page(p.graph(gid).unwrap(), &p, &[]);
    assert_eq!(errors(&sc).len(), 1);
    let l = p.graph_mut(gid).unwrap().first_layer_mut();
    l.error_bars.visible = false;
    assert!(errors(&scene::layout_page(p.graph(gid).unwrap(), &p, &[])).is_empty());
    let l = p.graph_mut(gid).unwrap().first_layer_mut();
    l.error_bars.visible = true;
    l.transform = Transform::Mean;
    let (_, s) = p.graph(gid).unwrap().first_layer().series(p.book(1).unwrap());
    assert!(s[0].errors.iter().all(|e| *e == [None, None]));
}

#[test]
fn column_roles_data_and_style_undo_restore_the_exact_export() {
    let (mut p, gid) = project();
    let before = p.clone();
    let before_svg = svg::render_page(p.graph(gid).unwrap(), &p);
    let mut history = History::default();
    history.begin(&p, "error bars");
    p.book_mut(1).unwrap().set_roles(&["D".into()], Role::Ignore);
    p.book_mut(1).unwrap().columns[2].set(0, "5".into());
    let style = &mut p.graph_mut(gid).unwrap().first_layer_mut().error_bars;
    style.cap = 12.0;
    style.width = 2.0;
    history.changed();
    history.commit(&p);
    let after = p.clone();
    let after_svg = svg::render_page(p.graph(gid).unwrap(), &p);
    assert_ne!(before_svg, after_svg);
    p = history.undo(&p).unwrap();
    assert_eq!(p, before);
    assert_eq!(svg::render_page(p.graph(gid).unwrap(), &p), before_svg);
    p = history.redo(&p).unwrap();
    assert_eq!(p, after);
    assert_eq!(svg::render_page(p.graph(gid).unwrap(), &p), after_svg);
}

#[cfg(feature = "serde")]
#[test]
fn v4_roundtrip_and_old_projects_default_to_error_style() {
    use scicanvas_core::project_file;
    let (mut p, gid) = project();
    p.book_mut(1)
        .unwrap()
        .set_roles(&["C".into(), "D".into()], Role::YError);
    p.graph_mut(gid).unwrap().first_layer_mut().error_bars.cap = 9.0;
    let text = project_file::to_json(&p).unwrap();
    assert_eq!(project_file::from_json(&text).unwrap(), p);
    let mut old: serde_json::Value = serde_json::from_str(&text).unwrap();
    old["version"] = 3.into();
    for col in old["books"][0]["columns"].as_array_mut().unwrap() {
        col.as_object_mut().unwrap().remove("error_for");
    }
    old["graphs"][0]["page"]["layers"][0]
        .as_object_mut()
        .unwrap()
        .remove("error_bars");
    let migrated = project_file::from_json(&old.to_string()).unwrap();
    assert_eq!(migrated.version, project_file::CURRENT_VERSION);
    assert_eq!(migrated.graph(gid).unwrap().first_layer().error_bars.cap, 6.0);
    for bad in ["-1", "0", "NaN"] {
        let style = &mut p.graph_mut(gid).unwrap().first_layer_mut().error_bars;
        style.width = bad.parse().unwrap();
        assert!(project_file::to_json(&p).is_err());
    }
}
