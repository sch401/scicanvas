use scicanvas_core::{
    scene::{self, Prim},
    PlotType, Project,
};

fn project(kind: PlotType) -> Project {
    let mut p = Project::sample();
    let b = p.book_mut(1).unwrap();
    for (key, values) in [
        ("A", ["1", "2", "3", "4"]),
        ("B", ["2", "-3", "4", "5"]),
        ("C", ["1", "-2", "3", "-1"]),
    ] {
        b.columns.iter_mut().find(|c| c.key == key).unwrap().data = values.map(str::to_owned).to_vec().into();
    }
    p.graphs.clear();
    p.plot_from_selection(1, &["B".into(), "C".into()], kind).unwrap();
    p
}

#[test]
fn b1_horizontal_stacks_keep_bases_and_percent_signs() {
    for kind in [PlotType::StackedHorizontalBar, PlotType::PercentStackedHorizontalBar] {
        let p = project(kind);
        let layer = p.graphs[0].first_layer();
        let (_, plots) = layer.prepared_plots_in(p.book(1).unwrap(), &|_| None);
        let factor = if kind == PlotType::PercentStackedHorizontalBar {
            100.0 / 3.0
        } else {
            1.0
        };
        assert!((plots[1].series.bases[0] - 2.0 * factor).abs() < 1e-12);
        assert!(plots[1].series.points[1][0] < 0.0);
        let scene = scene::layout_page(&p.graphs[0], &p, &[]);
        assert!(scene.layers[0].axes.x.lo <= -5.0);
        assert!(scene.layers[0].axes.x.hi >= 5.0);
        assert_eq!(
            scene
                .items
                .iter()
                .filter(|i| i.clip.is_some() && matches!(i.prim, Prim::Rect { .. }))
                .count(),
            8
        );
    }
}

#[test]
fn b1_fill_pairs_stacked_area_geometry_and_gaps() {
    for kind in [PlotType::StackedArea, PlotType::FillBetween] {
        let mut p = project(kind);
        let layer = p.graphs[0].first_layer();
        let (_, plots) = layer.prepared_plots_in(p.book(1).unwrap(), &|_| None);
        if kind == PlotType::FillBetween {
            assert_eq!(plots[0].plot.mapping.base.as_ref().unwrap().col, "C");
            assert_eq!(plots[1].style.kind, PlotType::Line);
            assert_eq!(plots[0].series.bases, vec![1., -2., 3., -1.]);
        } else {
            assert_eq!(plots[1].series.bases, vec![2., -3., 4., 0.]);
        }
        let scene = scene::layout_page(&p.graphs[0], &p, &[]);
        assert!(scene
            .items
            .iter()
            .any(|i| matches!(&i.prim,Prim::Triangles{pts,..} if !pts.is_empty())));
        p.book_mut(1)
            .unwrap()
            .columns
            .iter_mut()
            .find(|c| c.key == "C")
            .unwrap()
            .data
            .set_text(1, "".into());
        let (_, plots) = p.graphs[0]
            .first_layer()
            .prepared_plots_in(p.book(1).unwrap(), &|_| None);
        assert_eq!(plots[1].series.rows, vec![0, 2, 3]);
    }
}

#[cfg(feature = "serde")]
#[test]
fn b1_v29_roundtrip_and_old_projects_keep_their_plot() {
    for kind in [
        PlotType::StackedHorizontalBar,
        PlotType::PercentStackedHorizontalBar,
        PlotType::StackedArea,
        PlotType::FillBetween,
    ] {
        let p = project(kind);
        let json = scicanvas_core::project_file::to_json(&p).unwrap();
        assert_eq!(p, scicanvas_core::project_file::from_json(&json).unwrap());
    }
}

#[test]
fn b1_odd_fill_column_is_a_line_and_crossing_area_is_correct() {
    let mut p = project(PlotType::FillBetween);
    let mut d = scicanvas_core::Column::new("D", scicanvas_core::Role::Y);
    d.data = ["8", "8", "8", "8"].map(str::to_owned).to_vec().into();
    p.book_mut(1).unwrap().columns.push(d);
    p.graphs[0]
        .first_layer_mut()
        .add_book_curves(p.books.first().unwrap(), &["D".into()], true)
        .unwrap();
    let (_, plots) = p.graphs[0]
        .first_layer()
        .prepared_plots_in(p.book(1).unwrap(), &|_| None);
    assert_eq!(plots.last().unwrap().style.kind, PlotType::Line);
    assert!(plots.last().unwrap().plot.mapping.base.is_none());
    p.books[0].columns.iter_mut().find(|c| c.key == "A").unwrap().data = ["0", "1"].map(str::to_owned).to_vec().into();
    p.books[0].columns.iter_mut().find(|c| c.key == "B").unwrap().data = ["-1", "1"].map(str::to_owned).to_vec().into();
    p.books[0].columns.iter_mut().find(|c| c.key == "C").unwrap().data = ["1", "-1"].map(str::to_owned).to_vec().into();
    let scene = scene::layout_page(&p.graphs[0], &p, &[]);
    let view = &scene.layers[0];
    let area: f64 = scene
        .items
        .iter()
        .filter_map(|i| {
            if let Prim::Triangles { pts, .. } = &i.prim {
                Some(pts)
            } else {
                None
            }
        })
        .flat_map(|pts| pts.as_chunks::<3>().0)
        .map(|[a, b, c]| ((b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])).abs() / 2.0)
        .sum();
    let expected =
        scene.plot[2] / (view.axes.x.hi - view.axes.x.lo) * scene.plot[3] / (view.axes.left_y.hi - view.axes.left_y.lo);
    assert!((area / expected - 1.0).abs() < 1e-12);
}
