use scicanvas_core::{
    history::History,
    ops::Source,
    plotspec::Transform,
    scene::{self, Prim, Scene},
    svg, Column, PlotType, Project, Role, Series,
};

fn project(kind: PlotType, ys: &[&[&str]]) -> (Project, u64) {
    let mut p = Project::default();
    let bid = p.new_book();
    let b = p.book_mut(bid).unwrap();
    b.columns.clear();
    let mut x = Column::new("A", Role::X);
    x.data = (1..=ys.iter().map(|v| v.len()).max().unwrap())
        .map(|i| i.to_string())
        .collect();
    b.columns.push(x);
    for values in ys {
        let key = b.push_column(Role::Y);
        b.columns.iter_mut().find(|c| c.key == key).unwrap().data = values.iter().map(|v| v.to_string()).collect();
    }
    let (id, _) = p.plot_from_selection(bid, &[], kind).unwrap();
    (p, id)
}

fn series(p: &Project, id: u64) -> Vec<Series> {
    p.graph(id).unwrap().first_layer().series(p.book(1).unwrap()).1
}

fn rectangles(sc: &Scene) -> Vec<[f64; 4]> {
    sc.items
        .iter()
        .filter_map(|item| match item.prim {
            Prim::Rect { x, y, w, h, .. } if item.clip.is_some() => Some([x, y, w, h]),
            _ => None,
        })
        .collect()
}

#[test]
fn signed_stacks_keep_missing_rows_aligned_and_expand_the_axis() {
    let (p, id) = project(
        PlotType::StackedBar,
        &[&["2", "", "-3", "0"], &["5", "7", "-4", "0"], &["-1", "-2", "6", "0"]],
    );
    let s = series(&p, id);
    assert_eq!(s[0].rows, [0, 2, 3]);
    assert_eq!(s[1].bases, [2.0, 0.0, -3.0, 0.0]);
    assert_eq!(s[1].points, [[1.0, 7.0], [2.0, 7.0], [3.0, -7.0], [4.0, 0.0]]);
    assert_eq!(s[2].bases, [0.0; 4]);
    let sc = scene::layout_page(p.graph(id).unwrap(), &p, &[]);
    assert!(sc.axes.left_y.lo <= -7.0 && sc.axes.left_y.hi >= 7.0);
    let rects = rectangles(&sc);
    assert_eq!(rects.len(), 11);
    assert_eq!(rects[0][0], rects[3][0]);
    assert_eq!(rects[0][2], rects[3][2]);
    // 版面按页宽整体放大后坐标会有末位舍入差异
    assert!((rects[3][1] - sc.py(7.0)).abs() < 1e-9);
    assert!((rects[3][3] - (sc.py(2.0) - sc.py(7.0))).abs() < 1e-9);
}

#[test]
fn percentages_normalize_each_sign_and_handle_zero_and_extreme_scales() {
    let (p, id) = project(
        PlotType::PercentStackedBar,
        &[
            &["1", "-1", "0", "1e-320", "1e308"],
            &["3", "-3", "0", "3e-320", "1e308"],
            &["-2", "2", "0", "0", "0"],
        ],
    );
    let s = series(&p, id);
    assert_eq!(s[0].points[0][1], 25.0);
    assert_eq!(s[1].bases[0], 25.0);
    assert_eq!(s[1].points[0][1], 100.0);
    assert_eq!(s[1].points[1][1], -100.0);
    assert_eq!(s[2].points[0][1], -100.0);
    assert_eq!(s[2].points[1][1], 100.0);
    assert!(s.iter().all(|s| s.points[2][1] == 0.0));
    assert!((s[0].points[3][1] - 25.0).abs() < 1e-10);
    assert_eq!(s[1].points[4][1], 100.0);
    let sc = scene::layout_page(p.graph(id).unwrap(), &p, &[]);
    assert_eq!((sc.axes.left_y.lo, sc.axes.left_y.hi), (-100.0, 100.0));
    assert!(svg::render_page(p.graph(id).unwrap(), &p).contains("百分比 (%)"));
}

#[test]
fn floating_columns_pair_by_key_and_skip_incomplete_ranges() {
    let (mut p, id) = project(
        PlotType::FloatingBar,
        &[
            &["10", "", "40", "60"],
            &["20", "30", "35", "bad"],
            &["15", "25", "45", "50"],
            &["25", "35", "55", "70"],
        ],
    );
    let g = p.graph(id).unwrap().first_layer();
    assert_eq!(g.y_keys, ["C", "E"]);
    assert_eq!(g.base_by_y.get("C").map(String::as_str), Some("B"));
    let s = series(&p, id);
    assert_eq!(s[0].points, [[1.0, 20.0], [3.0, 35.0]]);
    assert_eq!(s[0].bases, [10.0, 40.0]);
    let sc = scene::layout_page(p.graph(id).unwrap(), &p, &[]);
    assert!(sc.axes.left_y.lo > 0.0 && sc.axes.left_y.lo <= 10.0 && sc.axes.left_y.hi >= 70.0);
    assert_eq!(rectangles(&sc).len(), 6);
    let before = series(&p, id);
    p.book_mut(1).unwrap().insert_column(2, Role::Y);
    assert_eq!(series(&p, id), before);
    p.delete_columns(1, &["B".into()]);
    assert_eq!(series(&p, id).len(), 1);
    assert_eq!(p.book_mut(1).unwrap().insert_column(1, Role::Y), "B");
    assert_eq!(series(&p, id).len(), 1);
}

#[test]
fn invalid_floating_selection_is_atomic_and_mapping_can_be_adjusted() {
    let (mut p, id) = project(PlotType::Bar, &[&["1"], &["2"], &["3"]]);
    let before = p.clone();
    assert!(p.plot_from_selection(1, &[], PlotType::FloatingBar).is_err());
    assert_eq!(p, before);
    let l = p.graph_mut(id).unwrap().first_layer_mut();
    let before = l.spec.clone();
    assert!(l.set_plot_type(PlotType::FloatingBar).is_err());
    assert_eq!(l.spec, before);
    l.y_keys.pop();
    l.set_plot_type(PlotType::FloatingBar).unwrap();
    l.base_by_y.insert("C".into(), "D".into());
    assert_eq!(series(&p, id)[0].bases, [3.0]);
    p.graph_mut(id)
        .unwrap()
        .first_layer_mut()
        .base_by_y
        .insert("C".into(), "C".into());
    assert!(p.graph(id).unwrap().first_layer().validate(p.book(1).unwrap()).is_err());
    assert!(series(&p, id).is_empty());
}

#[test]
fn means_align_by_x_and_floating_means_use_the_same_complete_rows() {
    let (mut p, id) = project(PlotType::StackedBar, &[&["2", "4", "6"], &["10", "", "30"]]);
    p.book_mut(1).unwrap().columns[0].data = ["2", "1", "2"].map(str::to_string).to_vec().into();
    p.graph_mut(id).unwrap().first_layer_mut().transform = Transform::Mean;
    let s = series(&p, id);
    assert_eq!(s[0].points, [[1.0, 4.0], [2.0, 4.0]]);
    assert_eq!(s[1].bases, [4.0]);
    assert_eq!(s[1].points, [[2.0, 24.0]]);
    p.graph_mut(id)
        .unwrap()
        .first_layer_mut()
        .set_plot_type(PlotType::FloatingBar)
        .unwrap();
    let s = series(&p, id);
    assert_eq!(s[0].points, [[2.0, 20.0]]);
    assert_eq!(s[0].bases, [4.0]);
}

#[test]
fn fitted_curves_keep_raw_values_and_never_join_the_stack_or_pairs() {
    for kind in [PlotType::StackedBar, PlotType::PercentStackedBar, PlotType::FloatingBar] {
        let (mut p, id) = project(kind, &[&["1", "2", "3"], &["2", "4", "6"]]);
        p.fit_poly(&Source::Graph(id), 1).unwrap();
        let s = series(&p, id);
        let derived: Vec<_> = s.iter().filter(|s| s.line_only).collect();
        assert!(!derived.is_empty());
        for s in derived {
            assert_eq!(s.points.len(), 200);
            assert!(s.bases.iter().all(|v| *v == 0.0));
            assert!(s.points.last().unwrap()[1] <= 6.000001);
        }
        assert_eq!(
            rectangles(&scene::layout_page(p.graph(id).unwrap(), &p, &[])).len(),
            if kind == PlotType::FloatingBar { 3 } else { 6 }
        );
    }
}

#[test]
fn error_bars_follow_stack_endpoints_and_percentage_scaling() {
    let (mut p, id) = project(PlotType::PercentStackedBar, &[&["1"], &["3"]]);
    let b = p.book_mut(1).unwrap();
    let key = b.push_column(Role::YError);
    b.columns.iter_mut().find(|c| c.key == key).unwrap().data = vec!["0.3".into()].into();
    let s = series(&p, id);
    assert_eq!(s[1].errors[0][0], None);
    assert!((s[1].errors[0][1].unwrap() - 7.5).abs() < 1e-12);
    let sc = scene::layout_page(p.graph(id).unwrap(), &p, &[]);
    let segs = sc
        .items
        .iter()
        .find_map(|item| match &item.prim {
            Prim::Segments {
                segs, dashed: false, ..
            } if item.clip.is_some() => Some(segs),
            _ => None,
        })
        .unwrap();
    let expected = [sc.px(1.0), sc.py(92.5), sc.px(1.0), sc.py(107.5)];
    assert!(segs[0].iter().zip(expected).all(|(a, b)| (*a - b).abs() < 1e-10));
    p.graph_mut(id)
        .unwrap()
        .first_layer_mut()
        .set_plot_type(PlotType::StackedBar)
        .unwrap();
    assert_eq!(series(&p, id)[1].errors, [[None, Some(0.3)]]);
}

#[test]
fn logarithmic_views_clip_ranges_and_svg_reuses_rectangle_geometry() {
    for kind in [PlotType::StackedBar, PlotType::PercentStackedBar, PlotType::FloatingBar] {
        let (mut p, id) = project(kind, &[&["-2", "10", "-4"], &["3", "-2", "-5"]]);
        let l = p.graph_mut(id).unwrap().first_layer_mut();
        l.y_log = true;
        l.x_log = true;
        l.y_auto = false;
        l.y_min = 1.0;
        l.y_max = 100.0;
        let sc = scene::layout_page(p.graph(id).unwrap(), &p, &[]);
        let output = svg::render_page(p.graph(id).unwrap(), &p);
        let rects = rectangles(&sc);
        assert!(!rects.is_empty());
        for [x, y, w, h] in rects {
            assert!([x, y, w, h].iter().all(|v| v.is_finite()));
            assert!(output.contains(&format!(
                "<rect x=\"{x:.2}\" y=\"{y:.2}\" width=\"{w:.2}\" height=\"{h:.2}\""
            )));
        }
        assert!(!output.contains("NaN") && !output.contains("inf"));
        assert!(output.contains(&format!("clip-path=\"url(#clip{id})\"")));
    }
}

#[test]
fn types_pairing_and_deleted_baselines_restore_through_history() {
    let (mut p, id) = project(PlotType::Bar, &[&["10", "20"], &["15", "30"]]);
    let mut h = History::default();
    let before = p.clone();
    h.begin(&p, "floating");
    p.graph_mut(id)
        .unwrap()
        .first_layer_mut()
        .set_plot_type(PlotType::FloatingBar)
        .unwrap();
    h.changed();
    h.commit(&p);
    let floating = p.clone();
    let output = svg::render_page(p.graph(id).unwrap(), &p);
    h.begin(&p, "delete lower");
    p.delete_columns(1, &["B".into()]);
    h.changed();
    h.commit(&p);
    assert!(series(&p, id).is_empty());
    p = h.undo(&p).unwrap();
    assert_eq!(p, floating);
    assert_eq!(svg::render_page(p.graph(id).unwrap(), &p), output);
    p = h.undo(&p).unwrap();
    assert_eq!(p, before);
    p = h.redo(&p).unwrap();
    assert_eq!(p, floating);
}

#[cfg(feature = "serde")]
#[test]
fn v5_persists_all_bar_types_and_v4_defaults_to_no_baseline_mapping() {
    use scicanvas_core::{plotspec::PlotSpec, project_file};
    for kind in [PlotType::StackedBar, PlotType::PercentStackedBar, PlotType::FloatingBar] {
        let (p, _) = project(kind, &[&["10"], &["15"]]);
        assert_eq!(project_file::from_json(&project_file::to_json(&p).unwrap()).unwrap(), p);
    }
    let (p, id) = project(PlotType::Bar, &[&["10"], &["15"]]);
    let mut v: serde_json::Value = serde_json::from_str(&project_file::to_json(&p).unwrap()).unwrap();
    v["version"] = 4.into();
    v["graphs"][0]["page"]["layers"][0]
        .as_object_mut()
        .unwrap()
        .remove("base_by_y");
    let loaded = project_file::from_json(&v.to_string()).unwrap();
    assert_eq!(loaded.version, project_file::CURRENT_VERSION);
    assert!(loaded.graph(id).unwrap().first_layer().base_by_y.is_empty());
    assert_eq!(series(&loaded, id), series(&p, id));
    let spec = PlotSpec::from_selection(p.book(1).unwrap(), &[], PlotType::FloatingBar).unwrap();
    spec.validate(p.book(1).unwrap()).unwrap();
}
