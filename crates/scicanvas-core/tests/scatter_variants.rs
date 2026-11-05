use scicanvas_core::{
    scene::{self, ObjectTarget, Prim},
    Book, Column, PlotType, Project, Role,
};
fn project(kind: PlotType) -> Project {
    let mut p = Project::default();
    let id = p.alloc_id();
    let mut b = Book::new(id, "B2");
    for (key, role, values) in [
        ("A", Role::X, ["1", "2", "3", "4"]),
        ("B", Role::Y, ["2", "4", "3", "5"]),
        (
            "C",
            if kind == PlotType::ErrorBand {
                Role::YError
            } else {
                Role::Y
            },
            ["1", "4", "9", "16"],
        ),
    ] {
        let mut c = Column::new(key, role);
        c.data = values.map(str::to_owned).to_vec().into();
        if role == Role::YError {
            c.error_for = Some("B".into());
        }
        b.columns.push(c);
    }
    if kind == PlotType::BubbleColor {
        let mut c = Column::new("D", Role::Y);
        c.data = ["0", "1", "2", "3"].map(str::to_string).to_vec().into();
        b.columns.push(c);
    }
    p.books.push(b);
    p.plot_from_selection(id, &[], kind).unwrap();
    p
}
#[test]
fn b2_area_sizes_color_scale_and_column_jitter() {
    for kind in [PlotType::Bubble, PlotType::ColorScatter, PlotType::BubbleColor] {
        let p = project(kind);
        let g = &p.graphs[0];
        let (_, plots) = g.first_layer().prepared_plots_in(&p.books[0], &|_| None);
        assert_eq!(plots.len(), 1);
        if kind != PlotType::ColorScatter {
            let radii: Vec<_> = plots[0].series.point_styles.iter().map(|p| p.size.unwrap()).collect();
            assert!((radii[3] / radii[0] - 4.).abs() < 1e-6);
        }
        let sc = scene::layout_page(g, &p, &[]);
        let color = sc.objects.iter().find(|o| o.target == ObjectTarget::ColorScale(0));
        assert_eq!(color.is_some(), kind != PlotType::Bubble);
        if let Some(color) = color {
            assert_eq!(
                sc.items[color.items.clone()]
                    .iter()
                    .filter(|i| matches!(i.prim, Prim::Rect { fill: Some(_), .. }))
                    .count(),
                64
            );
        }
    }
    let p = project(PlotType::ColumnScatter);
    let (_, plots) = p.graphs[0].first_layer().prepared_plots_in(&p.books[0], &|_| None);
    assert_eq!(plots.len(), 2);
    assert_eq!(plots[0].series.rows, vec![0, 1, 2, 3]);
    for (j, p) in plots[0].series.points.iter().enumerate() {
        assert!((p[0] - (1. + 0.15 * (2. * (j as f64 * 0.618034).fract() - 1.))).abs() < 1e-14);
    }
    assert_eq!(plots[0].paths[0][0][1], 3.5);
}
#[test]
fn b2_error_band_requires_error_column_and_draws_fill_without_caps() {
    let p = project(PlotType::ErrorBand);
    let sc = scene::layout_page(&p.graphs[0], &p, &[]);
    assert!(sc
        .items
        .iter()
        .any(|i| matches!(&i.prim,Prim::Triangles{pts,..} if !pts.is_empty())));
    let mut bad = p.books[0].clone();
    bad.columns.retain(|c| c.key != "C");
    assert!(
        scicanvas_core::plotspec::PlotSpec::from_selection(&bad, &[], PlotType::ErrorBand)
            .unwrap_err()
            .contains("Y 误差列")
    );
}
#[cfg(feature = "serde")]
#[test]
fn b2_color_scale_position_visibility_and_format_roundtrip() {
    let mut p = project(PlotType::ColorScatter);
    let before = scene::layout_page(&p.graphs[0], &p, &[]);
    p.graphs[0].first_layer_mut().color_scale = Some(scicanvas_core::page::ColorScale {
        position: Some([0.7, 0.2]),
        visible: true,
    });
    let moved = scene::layout_page(&p.graphs[0], &p, &[]);
    let object = moved
        .objects
        .iter()
        .find(|o| o.target == ObjectTarget::ColorScale(0))
        .unwrap();
    assert!((object.bounds[0] - 0.7 * moved.w).abs() < 1e-10);
    let json = scicanvas_core::project_file::to_json(&p).unwrap();
    assert_eq!(p, scicanvas_core::project_file::from_json(&json).unwrap());
    p.graphs[0].first_layer_mut().color_scale.as_mut().unwrap().visible = false;
    let hidden = scene::layout_page(&p.graphs[0], &p, &[]);
    assert!(hidden.plot[2] > before.plot[2]);
    assert!(hidden.objects.iter().all(|o| o.target != ObjectTarget::ColorScale(0)));
}

#[test]
fn color_scale_stays_inside_page() {
    for kind in [PlotType::ColorScatter, PlotType::BubbleColor] {
        for narrow in [false, true] {
            let mut p = project(kind);
            if narrow {
                p.graphs[0].first_layer_mut().placement = Some(scicanvas_core::page::LayerRect {
                    x: 0.75,
                    y: 0.,
                    width: 0.25,
                    height: 1.,
                });
            }
            let sc = scene::layout_page(&p.graphs[0], &p, &[]);
            let object = sc
                .objects
                .iter()
                .find(|o| o.target == ObjectTarget::ColorScale(0))
                .unwrap();
            let [x, y, w, h] = object.bounds;
            assert!(
                x >= -1e-9 && y >= -1e-9 && x + w <= sc.w + 1e-9 && y + h <= sc.h + 1e-9,
                "{kind:?} narrow={narrow}: {:?} page={}x{}",
                object.bounds,
                sc.w,
                sc.h
            );
            let mut blocks = Vec::new();
            for item in &sc.items[object.items.clone()] {
                match &item.prim {
                    Prim::Text { x, y, text, size, .. } => {
                        let layout = scicanvas_core::text::layout(text, *size, false);
                        assert!(
                            *x >= 0.
                                && *x + layout.width <= sc.w + 1e-9
                                && *y - layout.height / 2. >= -1e-9
                                && *y + layout.height / 2. <= sc.h + 1e-9,
                            "{text}: {x},{y}"
                        );
                    }
                    Prim::Rect {
                        y, h, fill: Some(_), ..
                    } => blocks.push((*y, *h)),
                    _ => {}
                }
            }
            assert_eq!(blocks.len(), 64);
            blocks.sort_by(|a, b| a.0.total_cmp(&b.0));
            assert!(blocks.windows(2).all(|p| p[0].0 + p[0].1 > p[1].0));
        }
    }
}

#[test]
fn review_new_colormaps_are_continuous_and_manual_mapping_is_retained() {
    use scicanvas_core::colormap::ColorMap;
    for kind in [PlotType::ColorScatter, PlotType::BubbleColor] {
        let mut p = project(kind);
        assert_eq!(p.graphs[0].first_layer().color_map, ColorMap::Viridis);
        let mut draft = scicanvas_core::plotspec::PlotSpec::from_selection(&p.books[0], &[], kind).unwrap();
        draft.color_map = ColorMap::Colorblind;
        let explicit: scicanvas_core::page::LayerSpec = draft.into();
        assert_eq!(explicit.color_map, ColorMap::Colorblind);
        p.graphs[0].first_layer_mut().color_map = ColorMap::Origin;
        p.graphs[0].first_layer_mut().edit_draft(|d| d.rows = Some((0, 2)));
        assert_eq!(p.graphs[0].first_layer().color_map, ColorMap::Origin);
        #[cfg(feature = "serde")]
        {
            let json = scicanvas_core::project_file::to_json(&p).unwrap();
            assert_eq!(p, scicanvas_core::project_file::from_json(&json).unwrap());
        }
    }
}

#[test]
fn review_dataset_color_size_mapping() {
    use scicanvas_core::{colormap::ColorMap, page::CurveStyle};
    let mut p = project(PlotType::Scatter);
    let layer = p.graphs[0].first_layer_mut();
    layer.edit_draft(|d| d.y_keys = vec!["B".into()]);
    layer.color_map = ColorMap::Viridis;
    layer.set_plot_style(
        "B",
        Some(CurveStyle {
            color_column: Some("C".into()),
            size_column: Some("C".into()),
            size_range: Some([2., 10.]),
            ..Default::default()
        }),
    );
    let (_, plots) = p.graphs[0].first_layer().prepared_plots_in(&p.books[0], &|_| None);
    let points = &plots[0].series;
    assert_eq!(points.points, vec![[1., 2.], [2., 4.], [3., 3.], [4., 5.]]);
    assert_eq!(points.point_styles.first().unwrap().size, Some(2.));
    assert_eq!(points.point_styles.last().unwrap().size, Some(10.));
    assert_eq!(points.point_styles.first().unwrap().color, Some([68, 1, 84]));
    assert_eq!(points.point_styles.last().unwrap().color, Some([253, 231, 37]));
}

#[test]
fn supplement_declarative_origin_is_not_replaced_by_ui_defaults() {
    use scicanvas_core::{colormap::ColorMap, plotspec::PlotSpec};
    for kind in [PlotType::ColorScatter, PlotType::BubbleColor] {
        let mut p = project(kind);
        let spec = PlotSpec {
            book_id: Some(p.books[0].id),
            x_key: Some("A".into()),
            y_keys: if kind == PlotType::BubbleColor {
                vec!["B".into(), "C".into(), "D".into()]
            } else {
                vec!["B".into(), "C".into()]
            },
            plot_type: kind,
            color_map: ColorMap::Origin,
            ..Default::default()
        };
        let id = p.plot_spec(spec).unwrap();
        assert_eq!(p.graph(id).unwrap().first_layer().color_map, ColorMap::Origin);
    }
}
