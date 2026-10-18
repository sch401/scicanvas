use scicanvas_core::{
    ops::Source,
    plotspec::PlotSpec,
    scene::{self, Prim},
    PlotType, Project,
};

fn project(kind: PlotType) -> (Project, u64) {
    let mut p = Project::sample();
    let book = p.book_mut(1).unwrap();
    book.columns[0].data = vec!["0".into(), "1".into(), "2".into()].into();
    book.columns[1].data = vec!["10".into(), "20".into(), "15".into()].into();
    let spec = PlotSpec::from_selection(book, &["B".into()], kind).unwrap();
    let id = p.plot_spec(spec).unwrap();
    (p, id)
}

#[test]
fn horizontal_and_vertical_steps_preserve_the_sample_values() {
    for (kind, expected) in [
        (
            PlotType::StepHorizontal,
            vec![[0.0, 10.0], [1.0, 10.0], [1.0, 20.0], [2.0, 20.0], [2.0, 15.0]],
        ),
        (
            PlotType::StepVertical,
            vec![[0.0, 10.0], [0.0, 20.0], [1.0, 20.0], [1.0, 15.0], [2.0, 15.0]],
        ),
    ] {
        let (p, id) = project(kind);
        let graph = p.graph(id).unwrap();
        let sc = scene::layout_page(graph, &p, &[]);
        let actual = sc
            .items
            .iter()
            .find_map(|item| match &item.prim {
                Prim::Polyline { pts, .. } => Some(pts),
                _ => None,
            })
            .unwrap();
        let expected: Vec<_> = expected.into_iter().map(|p| [sc.px(p[0]), sc.py(p[1])]).collect();
        // 版面按页宽整体放大后坐标有末位舍入差异
        assert_eq!(actual.len(), expected.len());
        assert!(
            actual
                .iter()
                .zip(&expected)
                .all(|(a, b)| (a[0] - b[0]).abs() < 1e-9 && (a[1] - b[1]).abs() < 1e-9),
            "{actual:?} vs {expected:?}"
        );
    }
}

#[test]
fn fitted_curve_stays_smooth_when_data_is_stepped() {
    let (mut p, id) = project(PlotType::StepHorizontal);
    p.fit_poly(&Source::Graph(id), 1).unwrap();
    let sc = scene::layout_page(p.graph(id).unwrap(), &p, &[]);
    let lengths: Vec<_> = sc
        .items
        .iter()
        .filter_map(|item| match &item.prim {
            Prim::Polyline { pts, .. } => Some(pts.len()),
            _ => None,
        })
        .collect();
    assert!(lengths.contains(&5));
    assert!(lengths.contains(&200));
}

#[cfg(feature = "serde")]
#[test]
fn step_chart_roundtrips_through_the_current_project_format() {
    let (p, _) = project(PlotType::StepVertical);
    let json = scicanvas_core::project_file::to_json(&p).unwrap();
    assert_eq!(scicanvas_core::project_file::from_json(&json).unwrap(), p);
}
