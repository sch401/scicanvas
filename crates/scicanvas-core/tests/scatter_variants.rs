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
