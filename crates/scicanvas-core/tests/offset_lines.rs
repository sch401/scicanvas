use scicanvas_core::{
    coord::{Coord, YAxis},
    ops::Source,
    page::CurveOffset,
    scene::{self, Prim},
    Book, Column, PlotType, Project, Role,
};
fn project(kind: PlotType) -> Project {
    let mut p = Project::default();
    let id = p.alloc_id();
    let mut b = Book::new(id, "B3");
    for (key, role, values) in [
        ("A", Role::X, ["0", "1", "2", "3"]),
        ("B", Role::Y, ["0", "2", "4", "3"]),
        ("C", Role::Y, ["1", "3", "8", "5"]),
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
fn b3_offsets_paths_and_pick_return_original_values() {
    for kind in [PlotType::OffsetLine, PlotType::Waterfall] {
        let mut p = project(kind);
        let layer = p.graphs[0].first_layer();
        let (_, plots) = layer.prepared_plots_in(&p.books[0], &|_| None);
        let dx = if kind == PlotType::Waterfall { 0.15 } else { 0. };
        assert_eq!(plots[0].offset, [0., 0.]);
        assert!((plots[1].offset[0] - dx).abs() < 1e-12);
        assert_eq!(plots[1].offset[1], 7.);
        let id = p.graphs[0].id;
        p.fit_poly(&Source::Graph(id), 1).unwrap();
        let (_, plots) = p.graphs[0].first_layer().prepared_plots_in(&p.books[0], &|_| None);
        let fit = plots
            .iter()
            .find(|p| p.series.line_only && p.series.source.as_deref() == Some("C"))
            .unwrap();
        assert_eq!(
            fit.offset,
            plots
                .iter()
                .find(|p| p.series.key == "C" && !p.series.line_only)
                .unwrap()
                .offset
        );
        p.graphs[0].first_layer_mut().offset = Some(CurveOffset {
            dx: 2.,
            dy: 3.,
            auto: false,
        });
        let sc = scene::layout_page(&p.graphs[0], &p, &[]);
        let map = sc.layers[0].axes.cartesian(sc.plot, YAxis::Left);
        let at = map.project([5., 8.]);
        let hit = scene::nearest_point(
            p.graphs[0].first_layer(),
            &sc.layers[0],
            &|id| p.book(id),
            at[0],
            at[1],
            1.,
        )
        .unwrap();
        assert_eq!((hit.key, hit.row, hit.x, hit.y), ("C".into(), Some(3), 3., 5.));
    }
}
#[test]
fn b3_line_series_links_original_rows_without_hidden_book() {
    let mut p = project(PlotType::LineSeries);
    p.books[0]
        .columns
        .iter_mut()
        .find(|c| c.key == "A")
        .unwrap()
        .data
        .set_text(0, "".into());
    p.books[0]
        .columns
        .iter_mut()
        .find(|c| c.key == "B")
        .unwrap()
        .data
        .set_text(1, "".into());
    let (_, plots) = p.graphs[0].first_layer().prepared_plots_in(&p.books[0], &|_| None);
    assert_eq!(plots[0].series.rows, vec![0, 2, 3]);
    assert!(plots[0].series.points.iter().all(|p| p[0] == 1.));
    let sc = scene::layout_page(&p.graphs[0], &p, &[]);
    assert_eq!(
        sc.items
            .iter()
            .filter(|i| i.clip == Some(sc.plot) && matches!(&i.prim,Prim::Polyline{pts,..} if pts.len()==2))
            .count(),
        3
    );
    assert_eq!(p.books.len(), 1);
    assert_eq!(p.books[0].columns.len(), 3);
    let map = sc.layers[0].axes.cartesian(sc.plot, YAxis::Left);
    let at = map.project([2., 8.]);
    let hit = scene::nearest_point(
        p.graphs[0].first_layer(),
        &sc.layers[0],
        &|id| p.book(id),
        at[0],
        at[1],
        1.,
    )
    .unwrap();
    assert_eq!((hit.key, hit.row), ("C".into(), Some(2)));
}
#[cfg(feature = "serde")]
#[test]
fn b3_offset_roundtrips_and_old_project_has_no_offset() {
    let p = project(PlotType::Waterfall);
    let json = scicanvas_core::project_file::to_json(&p).unwrap();
    assert_eq!(p, scicanvas_core::project_file::from_json(&json).unwrap());
    let p = Project::sample();
    let mut legacy: serde_json::Value =
        serde_json::from_str(&scicanvas_core::project_file::to_json(&p).unwrap()).unwrap();
    legacy["version"] = 31.into();
    assert!(scicanvas_core::project_file::from_json(&legacy.to_string())
        .unwrap()
        .graphs[0]
        .first_layer()
        .offset
        .is_none());
}
