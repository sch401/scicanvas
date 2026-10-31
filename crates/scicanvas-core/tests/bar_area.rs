use scicanvas_core::{
    history::History,
    ops::Source,
    plotspec::Transform,
    scene::{self, Prim, Scene, View},
    svg, Column, PlotType, Project, Role,
};

fn project(kind: PlotType, xs: &[&str], ys: &[&[&str]]) -> (Project, u64) {
    let mut p = Project::default();
    p.new_book();
    let b = p.book_mut(1).unwrap();
    b.columns.clear();
    let mut x = Column::new("A", Role::X);
    x.long_name = "位置".into();
    x.units = "s".into();
    x.data = xs.iter().map(|v| v.to_string()).collect();
    b.columns.push(x);
    for values in ys {
        b.push_column(Role::Y);
        let col = b.columns.last_mut().unwrap();
        col.long_name = "测量值".into();
        col.units = "mV".into();
        col.data = values.iter().map(|v| v.to_string()).collect();
    }
    let (id, _) = p.plot_from_selection(1, &[], kind).unwrap();
    (p, id)
}

fn triangles(sc: &Scene) -> Vec<&Vec<[f64; 2]>> {
    sc.items
        .iter()
        .filter_map(|it| match &it.prim {
            Prim::Triangles { pts, .. } => Some(pts),
            _ => None,
        })
        .collect()
}

fn near(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-9, "{a} != {b}");
}

#[test]
fn horizontal_groups_signed_values_titles_and_physical_coordinates() {
    let (p, id) = project(PlotType::HorizontalBar, &["1", "2"], &[&["10", "-5"], &["20", "5"]]);
    let g = p.graph(id).unwrap();
    let sc = scene::layout_page(g, &p, &[]);
    assert!(sc.axes.x.lo <= -5.0 && sc.axes.x.hi >= 20.0);
    assert!(sc.axes.left_y.lo <= 0.65 && sc.axes.left_y.hi >= 2.35);
    assert_eq!(g.first_layer().x_axis_title(p.book(1)), "测量值 (mV)");
    assert_eq!(g.first_layer().y_axis_title(p.book(1)), "位置 (s)");
    let rects: Vec<_> = sc
        .items
        .iter()
        .filter_map(|it| match it.prim {
            Prim::Rect { x, y, w, h, .. } if it.clip.is_some() => Some([x, y, w, h]),
            _ => None,
        })
        .collect();
    assert_eq!(rects.len(), 4);
    let expected = [
        sc.px(0.0),
        sc.py(1.0),
        sc.px(10.0) - sc.px(0.0),
        sc.py(0.65) - sc.py(1.0),
    ];
    for (a, b) in rects[0].iter().zip(expected) {
        near(*a, b);
    }
    near(rects[1][0], sc.px(-5.0));
    near(rects[1][2], sc.px(0.0) - sc.px(-5.0));
    let (value, position) = sc.layers[0].data_at(sc.px(10.0), sc.py(1.0));
    near(value, 10.0);
    near(position, 1.0);
    let output = svg::render_page(g, &p);
    for [x, y, w, h] in rects {
        assert!(output.contains(&format!(
            "<rect x=\"{x:.2}\" y=\"{y:.2}\" width=\"{w:.2}\" height=\"{h:.2}\""
        )));
    }
}

#[test]
fn horizontal_error_directions_and_derived_lines_follow_the_same_rotation() {
    let (mut p, id) = project(
        PlotType::HorizontalBar,
        &["1", "2", "3"],
        &[&["2", "4", "6"], &["3", "6", "9"]],
    );
    let b = p.book_mut(1).unwrap();
    for (role, value) in [(Role::XError, "0.1"), (Role::YError, "0.5")] {
        b.push_column(role);
        b.columns.last_mut().unwrap().data = vec![value.into(); 3].into();
        b.columns.last_mut().unwrap().error_for = Some("B".into());
    }
    p.fit_poly(&Source::Graph(id), 1).unwrap();
    let sc = scene::layout_page(p.graph(id).unwrap(), &p, &[]);
    let segs = sc
        .items
        .iter()
        .find_map(|it| match &it.prim {
            Prim::Segments {
                segs, dashed: false, ..
            } if it.clip.is_some() => Some(segs),
            _ => None,
        })
        .unwrap();
    let expected = [sc.px(1.5), sc.py(0.825), sc.px(2.5), sc.py(0.825)];
    for (a, b) in segs[0].iter().zip(expected) {
        near(*a, b);
    }
    let expected = [sc.px(2.0), sc.py(0.725), sc.px(2.0), sc.py(0.925)];
    for (a, b) in segs[3].iter().zip(expected) {
        near(*a, b);
    }
    let fit = sc
        .items
        .iter()
        .find_map(|it| match &it.prim {
            Prim::Polyline { pts, .. } if pts.len() == 200 => Some(pts),
            _ => None,
        })
        .unwrap();
    near(fit.last().unwrap()[0], sc.px(6.0));
    near(fit.last().unwrap()[1], sc.py(3.0));
}

#[test]
fn area_mesh_has_the_correct_positive_negative_and_crossing_integrals() {
    for (values, expected) in [
        (&["2", "4", "2"][..], 6.0),
        (&["-2", "-4", "-2"][..], 6.0),
        (&["-2", "2", "-2"][..], 2.0),
    ] {
        let (p, id) = project(PlotType::Area, &["0", "1", "2"], &[values]);
        let sc = scene::layout_page(p.graph(id).unwrap(), &p, &[]);
        assert!(sc.axes.left_y.lo <= 0.0 && sc.axes.left_y.hi >= 0.0);
        let area: f64 = triangles(&sc)
            .iter()
            .flat_map(|pts| pts.as_chunks::<3>().0)
            .map(|t| {
                ((t[1][0] - t[0][0]) * (t[2][1] - t[0][1]) - (t[1][1] - t[0][1]) * (t[2][0] - t[0][0])).abs() / 2.0
            })
            .sum();
        near(area / ((sc.px(1.0) - sc.px(0.0)) * (sc.py(0.0) - sc.py(1.0))), expected);
        assert!(sc
            .items
            .iter()
            .filter(|it| matches!(it.prim, Prim::Triangles { .. }))
            .all(|it| it.clip == Some(sc.plot)));
    }
}

#[test]
fn area_gaps_and_log_filtered_rows_split_both_fill_and_outline() {
    let (mut p, id) = project(PlotType::Area, &["1", "2", "3", "4", "5"], &[&["1", "2", "", "4", "5"]]);
    for log in [false, true] {
        p.graph_mut(id).unwrap().first_layer_mut().y_log = log;
        if log {
            p.book_mut(1).unwrap().columns[1].data.set_text(2, "-3".into());
        }
        let sc = scene::layout_page(p.graph(id).unwrap(), &p, &[]);
        assert_eq!(triangles(&sc).len(), 2);
        let lengths: Vec<_> = sc
            .items
            .iter()
            .filter_map(|it| match &it.prim {
                // 坐标轴本身也是一条黑色折线，这里只数面积边界线
                Prim::Polyline { pts, color, .. } if *color != scene::INK => Some(pts.len()),
                _ => None,
            })
            .collect();
        assert_eq!(lengths, [2, 2]);
        assert!(triangles(&sc)
            .iter()
            .flat_map(|pts| pts.iter().flatten())
            .all(|v| v.is_finite()));
    }
    p.graph_mut(id).unwrap().first_layer_mut().transform = Transform::Mean;
    assert_eq!(triangles(&scene::layout_page(p.graph(id).unwrap(), &p, &[])).len(), 2);
}

#[test]
fn area_sorts_each_run_and_keeps_fitted_curves_unfilled() {
    let (mut p, id) = project(PlotType::Area, &["2", "0", "1", "1"], &[&["2", "2", "4", "3"]]);
    p.fit_poly(&Source::Graph(id), 1).unwrap();
    let sc = scene::layout_page(p.graph(id).unwrap(), &p, &[]);
    assert_eq!(triangles(&sc).len(), 1);
    let data = sc
        .items
        .iter()
        .find_map(|it| match &it.prim {
            Prim::Polyline { pts, .. } if pts.len() == 4 => Some(pts),
            _ => None,
        })
        .unwrap();
    assert!(data.windows(2).all(|w| w[0][0] <= w[1][0]));
    assert!(sc
        .items
        .iter()
        .any(|it| matches!(&it.prim, Prim::Polyline { pts, .. } if pts.len() == 200)));
}

#[test]
fn svg_contains_the_shared_triangles_opacity_and_independent_layer_clips() {
    let (mut p, id) = project(PlotType::Area, &["0", "1", "2"], &[&["1", "-2", "3"]]);
    let mut other = p.graph(id).unwrap().first_layer().clone();
    other.area_opacity = 0.75;
    p.graph_mut(id).unwrap().page.add_layer(other);
    let sc = scene::layout_page(p.graph(id).unwrap(), &p, &[]);
    let output = svg::render_page(p.graph(id).unwrap(), &p);
    assert_eq!(triangles(&sc).len(), 2);
    for pts in triangles(&sc) {
        for t in pts.as_chunks::<3>().0 {
            assert!(output.contains(&format!(
                "M{:.2},{:.2}L{:.2},{:.2}L{:.2},{:.2}Z",
                t[0][0], t[0][1], t[1][0], t[1][1], t[2][0], t[2][1]
            )));
        }
    }
    assert!(output.contains("fill-opacity=\"0.75\""));
    assert!(output.contains(&format!("clip-path=\"url(#clip{id}-1)\"")));
}

#[test]
fn manual_log_views_use_physical_axes_and_have_finite_geometry() {
    for kind in [PlotType::HorizontalBar, PlotType::Area] {
        let (mut p, id) = project(kind, &["1", "2", "3"], &[&["-1", "10", "100"]]);
        let l = p.graph_mut(id).unwrap().first_layer_mut();
        l.x_log = true;
        l.y_log = true;
        let view = View {
            x: (1.0, 100.0),
            y: (1.0, 100.0),
        };
        let sc = scene::layout_page(p.graph(id).unwrap(), &p, &[Some(view)]);
        assert_eq!(
            (sc.axes.x.lo, sc.axes.x.hi, sc.axes.left_y.lo, sc.axes.left_y.hi),
            (1.0, 100.0, 1.0, 100.0)
        );
        let output = svg::render_page(p.graph(id).unwrap(), &p);
        assert!(!output.contains("NaN") && !output.contains("inf"));
        for pts in triangles(&sc) {
            assert!(pts.iter().flatten().all(|v| v.is_finite()));
        }
    }
}

#[test]
fn types_opacity_and_data_restore_the_exact_svg_through_history() {
    let (mut p, id) = project(PlotType::HorizontalBar, &["1", "2", "3"], &[&["2", "4", "6"]]);
    let before = p.clone();
    let original = svg::render_page(p.graph(id).unwrap(), &p);
    let mut h = History::default();
    h.begin(&p, "area style");
    let l = p.graph_mut(id).unwrap().first_layer_mut();
    l.set_plot_type(PlotType::Area).unwrap();
    l.area_opacity = 0.8;
    p.book_mut(1).unwrap().columns[1].set(1, "5".into());
    h.changed();
    h.commit(&p);
    let after = p.clone();
    p = h.undo(&p).unwrap();
    assert_eq!(p, before);
    assert_eq!(svg::render_page(p.graph(id).unwrap(), &p), original);
    p = h.redo(&p).unwrap();
    assert_eq!(p, after);
}

#[cfg(feature = "serde")]
#[test]
fn v6_roundtrips_new_types_and_migrates_v5_area_defaults() {
    use scicanvas_core::project_file;
    for kind in [PlotType::HorizontalBar, PlotType::Area] {
        let (p, _) = project(kind, &["1", "2"], &[&["2", "4"]]);
        assert_eq!(project_file::from_json(&project_file::to_json(&p).unwrap()).unwrap(), p);
    }
    let (mut p, id) = project(PlotType::Bar, &["1", "2"], &[&["2", "4"]]);
    let mut old: serde_json::Value = serde_json::from_str(&project_file::to_json(&p).unwrap()).unwrap();
    old["version"] = 5.into();
    old["graphs"][0]["page"]["layers"][0]
        .as_object_mut()
        .unwrap()
        .remove("area_opacity");
    let loaded = project_file::from_json(&old.to_string()).unwrap();
    assert_eq!(loaded.version, project_file::CURRENT_VERSION);
    assert_eq!(loaded.graph(id).unwrap().first_layer().area_opacity, 0.35);
    for bad in [-0.1, 1.1, f32::NAN, f32::INFINITY] {
        p.graph_mut(id).unwrap().first_layer_mut().area_opacity = bad;
        assert!(project_file::to_json(&p).is_err());
    }
}

#[test]
fn mapped_area_keeps_source_colors_through_sorting_gaps_and_zero_crossings() {
    use scicanvas_core::page::CurveStyle;
    let (mut p, id) = project(
        PlotType::Area,
        &["2", "0", "1", "3", "4", "5"],
        &[&["2", "-2", "4", "", "2", "4"]],
    );
    let key = p.book_mut(1).unwrap().push_column(Role::Ignore);
    p.book_mut(1).unwrap().columns.last_mut().unwrap().data =
        ["1", "0", "", "0", "0", "1"].into_iter().map(str::to_owned).collect();
    let fallback = (201, 17, 131);
    let layer = p.graph_mut(id).unwrap().first_layer_mut();
    layer.area_opacity = 0.4;
    layer.legend = false;
    layer.set_plot_style(
        "B",
        Some(CurveStyle {
            color: Some(fallback.into()),
            right_axis: true,
            ..Default::default()
        }),
    );
    let plain = scene::layout_page(p.graph(id).unwrap(), &p, &[]);
    let mut style = p.graph(id).unwrap().first_layer().plot_style("B").cloned().unwrap();
    style.color_column = Some(key);
    p.graph_mut(id)
        .unwrap()
        .first_layer_mut()
        .set_plot_style("B", Some(style));
    let sc = scene::layout_page(p.graph(id).unwrap(), &p, &[]);
    let segments: Vec<_> = sc
        .items
        .iter()
        .filter(|it| it.clip.is_some())
        .flat_map(|it| match &it.prim {
            Prim::ColoredSegments { segments, .. } => segments.clone(),
            _ => Vec::new(),
        })
        .collect();
    let low = scicanvas_core::PALETTE[0];
    let high = *scicanvas_core::PALETTE.last().unwrap();
    assert_eq!(
        segments.iter().map(|s| s.color).collect::<Vec<_>>(),
        [low, fallback, fallback, high, low, high]
    );
    for (segment, (a, b)) in
        segments
            .iter()
            .zip([(0.0, 0.5), (0.5, 1.0), (1.0, 1.5), (1.5, 2.0), (4.0, 4.5), (4.5, 5.0)])
    {
        near(segment.points[0], sc.px(a));
        near(segment.points[2], sc.px(b));
    }
    let area = |scene: &Scene| {
        triangles(scene)
            .into_iter()
            .flat_map(|pts| pts.as_chunks::<3>().0)
            .map(|t| {
                ((t[1][0] - t[0][0]) * (t[2][1] - t[0][1]) - (t[1][1] - t[0][1]) * (t[2][0] - t[0][0])).abs() / 2.0
            })
            .sum::<f64>()
    };
    near(area(&sc), area(&plain));
    for item in sc
        .items
        .iter()
        .filter(|it| matches!(it.prim, Prim::Triangles { .. } | Prim::ColoredSegments { .. }))
    {
        assert_eq!(item.clip, Some(sc.plot));
        if let Prim::Triangles { opacity, .. } = item.prim {
            near(opacity, f64::from(0.4f32));
        }
    }
    let output = svg::render_page(p.graph(id).unwrap(), &p);
    for color in [low, fallback, high] {
        let hex = format!("#{:02x}{:02x}{:02x}", color.0, color.1, color.2);
        assert!(output.contains(&format!("fill=\"{hex}\"")));
        assert!(output.contains(&format!("stroke=\"{hex}\"")));
    }
    #[cfg(feature = "serde")]
    {
        let saved = scicanvas_core::project_file::to_json(&p).unwrap();
        let loaded = scicanvas_core::project_file::from_json(&saved).unwrap();
        assert_eq!(loaded, p);
        assert_eq!(svg::render_page(loaded.graph(id).unwrap(), &loaded), output);
        assert_eq!(loaded.version, scicanvas_core::project_file::CURRENT_VERSION);
    }
}

#[test]
fn mapped_area_foreign_categories_survive_mean_and_log_projection() {
    use scicanvas_core::{colormap::ColorMap, page::CurveStyle, plotspec::foreign_key};
    let (mut p, id) = project(
        PlotType::Area,
        &["1", "1", "10", "10", "100", "100"],
        &[&["1", "3", "10", "30", "100", "300"]],
    );
    let mut foreign = p.book(1).unwrap().clone();
    foreign.id = p.alloc_id();
    let bid = foreign.id;
    let key = foreign.push_column(Role::Ignore);
    foreign.columns.last_mut().unwrap().data = (scicanvas_core::ColumnData::Category {
        codes: vec![0, 0, 1, 1, 2, 2],
        levels: vec!["Z".into(), "A".into(), "B".into()],
        ordered: true,
    })
    .into();
    p.books.push(foreign);
    p.add_curves(id, 0, bid, &["B".into()]).unwrap();
    let layer = p.graph_mut(id).unwrap().first_layer_mut();
    layer.transform = Transform::Mean;
    layer.x_log = true;
    layer.y_log = true;
    layer.color_map = ColorMap::Colorblind;
    layer.set_plot_style(
        &foreign_key(bid, "B"),
        Some(CurveStyle {
            color_column: Some(key),
            right_axis: true,
            ..Default::default()
        }),
    );
    let (_, prepared) = p
        .graph(id)
        .unwrap()
        .first_layer()
        .prepared_plots_in(p.book(1).unwrap(), &|bid| p.book(bid));
    let data = &prepared
        .iter()
        .find(|plot| plot.plot.mapping.y.book == bid)
        .unwrap()
        .series;
    assert_eq!(data.points, [[1.0, 2.0], [10.0, 20.0], [100.0, 200.0]]);
    let expected: Vec<_> = [0.0, 0.5, 1.0]
        .into_iter()
        .map(|t| ColorMap::Colorblind.sample(t))
        .collect();
    assert_eq!(
        data.point_styles
            .iter()
            .map(|s| s.color.unwrap().into())
            .collect::<Vec<(u8, u8, u8)>>(),
        expected
    );
    let sc = scene::layout_page(p.graph(id).unwrap(), &p, &[]);
    let mapped: Vec<_> = sc
        .items
        .iter()
        .filter_map(|it| match &it.prim {
            Prim::ColoredSegments { segments, .. } if it.clip.is_some() => Some(segments),
            _ => None,
        })
        .flatten()
        .collect();
    assert_eq!(mapped.len(), 4);
    assert_eq!(
        mapped.iter().map(|segment| segment.color).collect::<Vec<_>>(),
        [expected[0], expected[1], expected[1], expected[2]]
    );
    assert!(mapped.iter().flat_map(|s| s.points).all(f64::is_finite));
    assert!(!triangles(&sc).is_empty());
}

#[test]
fn mapped_area_constant_color_legend_matches_fill_and_outline() {
    use scicanvas_core::{colormap::ColorMap, page::CurveStyle};
    let (mut p, id) = project(PlotType::Area, &["0", "1", "1", "2"], &[&["2", "4", "3", "2"]]);
    p.graph_mut(id).unwrap().page.width_mm = 89.0;
    let key = p.book_mut(1).unwrap().push_column(Role::Ignore);
    p.book_mut(1).unwrap().columns.last_mut().unwrap().data =
        ["5", "5", "5", "5"].into_iter().map(str::to_owned).collect();
    let layer = p.graph_mut(id).unwrap().first_layer_mut();
    layer.color_map = ColorMap::Viridis;
    layer.area_opacity = 0.5;
    layer.set_plot_style(
        "B",
        Some(CurveStyle {
            color_column: Some(key),
            line_width: Some(2.0),
            ..Default::default()
        }),
    );
    let sc = scene::layout_page(p.graph(id).unwrap(), &p, &[]);
    let expected = ColorMap::Viridis.sample(0.5);
    assert!(sc.items.iter().any(|it| matches!(it.prim,
        Prim::Rect { fill: Some((color, 0.5)), stroke: Some((border, 2.0)), .. }
            if it.clip.is_none() && color == expected && border == expected
    )));
    assert!(sc.items.iter().any(|it| matches!(it.prim,
        Prim::Triangles { color, opacity: 0.5, .. } if it.clip.is_some() && color == expected
    )));
}
