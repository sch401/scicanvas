use scicanvas_core::{
    coord::{Coord, Polar},
    page::Projection,
    scene::{self, Prim},
    Book, Column, PlotType, Project, Role,
};
fn project(kind: PlotType) -> Project {
    let mut p = Project::default();
    let id = p.alloc_id();
    let mut b = Book::new(id, "B4");
    for (key, role, values) in [
        ("A", Role::X, ["0", "90", "180", "270", "360"]),
        ("B", Role::Y, ["1", "2", "0", "-1", ""]),
        ("C", Role::Y, ["2", "3", "4", "3", "2"]),
    ] {
        let mut c = Column::new(key, role);
        c.data = values.map(str::to_owned).to_vec().into();
        b.columns.push(c);
    }
    p.books.push(b);
    p.plot_from_selection(id, &[], kind).unwrap();
    p
}
#[test]
fn b4_pie_positive_rows_angles_labels_and_pick() {
    let mut p = project(PlotType::Pie);
    let g = &p.graphs[0];
    let sc = scene::layout_page(g, &p, &[]);
    let layer = g.first_layer();
    assert_eq!(scene::pie_skipped(layer, &|id| p.book(id)), 3);
    assert_eq!(
        sc.items
            .iter()
            .filter(|i| matches!(i.prim, Prim::Triangles { .. }))
            .count(),
        2
    );
    assert!(sc
        .items
        .iter()
        .any(|i| matches!(&i.prim,Prim::Text{text,..} if text=="33.3%")));
    let [l, t, w, h] = sc.plot;
    let at = [l + w / 2. + w.min(h) * 0.15, t + h / 2. - w.min(h) * 0.15];
    let hit = scene::nearest_point(layer, &sc.layers[0], &|id| p.book(id), at[0], at[1], 0.).unwrap();
    assert_eq!((hit.row, hit.y), (Some(0), 1.));
    p.books[0].columns[0].data = ["Alpha", "Beta", "Gamma", "Delta", "Epsilon"]
        .map(str::to_owned)
        .to_vec()
        .into();
    let sc = scene::layout_page(&p.graphs[0], &p, &[]);
    assert!(sc
        .items
        .iter()
        .any(|i| matches!(&i.prim,Prim::Text{text,..} if text=="Alpha")));
}
#[test]
fn b4_polar_projection_inverse_scene_and_original_row_pick() {
    for degrees in [true, false] {
        let mut p = project(PlotType::PolarScatter);
        p.graphs[0].first_layer_mut().projection = Projection::Polar { degrees };
        if !degrees {
            p.books[0].columns[0].data = (0..5)
                .map(|i| (i as f64 * std::f64::consts::FRAC_PI_2).to_string())
                .collect();
        }
        let sc = scene::layout_page(&p.graphs[0], &p, &[]);
        let [l, t, w, h] = sc.plot;
        let map = Polar {
            r: &sc.axes.left_y,
            center: [l + w / 2., t + h / 2.],
            radius: w.min(h) * 0.4,
            degrees,
        };
        let theta = if degrees { 90. } else { std::f64::consts::FRAC_PI_2 };
        let at = map.project([theta, 2.]);
        let back = map.unproject(at);
        assert!((back[0] - theta).abs() < 1e-12);
        assert!((back[1] - 2.).abs() < 1e-12);
        let hit = scene::nearest_point(
            p.graphs[0].first_layer(),
            &sc.layers[0],
            &|id| p.book(id),
            at[0],
            at[1],
            0.1,
        )
        .unwrap();
        assert_eq!((hit.row, hit.y), (Some(1), 2.));
    }
}
#[test]
fn b4_violin_uses_categories_density_and_box_statistics() {
    let p = project(PlotType::Violin);
    let (_, plots) = p.graphs[0].first_layer().prepared_plots_in(&p.books[0], &|_| None);
    assert_eq!(plots[0].box_stats.as_ref().unwrap().median, 0.5);
    assert!(plots[1].series.points.iter().all(|p| p[0] == 2.));
    let sc = scene::layout_page(&p.graphs[0], &p, &[]);
    assert_eq!(
        sc.items
            .iter()
            .filter(|i| matches!(i.prim, Prim::Triangles { opacity: 0.3, .. }))
            .count(),
        2
    );
}
#[cfg(feature = "serde")]
#[test]
fn b4_projection_roundtrip_legacy_default_and_kde_scipy() {
    let p = project(PlotType::PolarLine);
    let json = scicanvas_core::project_file::to_json(&p).unwrap();
    assert_eq!(p, scicanvas_core::project_file::from_json(&json).unwrap());
    let old = Project::sample();
    let json = scicanvas_core::project_file::to_json(&old).unwrap();
    assert!(!json.contains("projection"));
    let doc: serde_json::Value = serde_json::from_str(include_str!("reference/kde.json")).unwrap();
    for case in doc["cases"].as_array().unwrap() {
        let values = |k: &str| {
            case[k]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_f64().unwrap())
                .collect::<Vec<_>>()
        };
        let density = scicanvas_core::stats::gaussian_kde(&values("values"), &values("at"));
        for (a, b) in density.iter().zip(values("density")) {
            assert!((a - b).abs() < 1e-12);
        }
    }
}

#[test]
fn b4_statistical_and_radial_scenes_ignore_cartesian_measurement_errors() {
    for kind in [
        PlotType::Pie,
        PlotType::Violin,
        PlotType::PolarLine,
        PlotType::PolarScatter,
    ] {
        let mut p = project(kind);
        let before = scene::layout_page(&p.graphs[0], &p, &[]);
        let mut errors = Column::new("D", Role::YError);
        errors.error_for = Some("B".into());
        errors.data = vec!["100".to_owned(); 5].into();
        p.books[0].columns.push(errors);
        assert_eq!(before, scene::layout_page(&p.graphs[0], &p, &[]));
    }
}
