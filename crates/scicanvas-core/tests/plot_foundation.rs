use scicanvas_core::{
    colormap::ColorMap,
    coord::{Cartesian, Coord},
    page::Layer,
    plotspec::{PlotSpec, Transform},
    scene, svg, Book, Column, Graph, PlotType, Project, Role,
};

#[test]
fn presets_have_example_data_and_use_the_shared_scene() {
    let book = scicanvas_core::presets::example();
    let mut project = Project::default();
    project.books.push(book.clone());
    let presets = scicanvas_core::presets::all();
    assert_eq!(presets.len(), 4);
    let mut ids = std::collections::BTreeSet::new();
    for preset in presets {
        assert!(ids.insert(preset.id));
        let spec = PlotSpec::from_selection(&book, &[], preset.kind).unwrap();
        let mut graph = Graph::new(20, preset.name);
        graph.spec = spec;
        let sc = scene::build_page(&graph, &project, 800.0, 600.0, None);
        assert!(!sc.items.is_empty());
        assert_eq!(sc.layer_plots.len(), 1);
        assert!(svg::render_page(&graph, &project, 800.0, 600.0).contains("85mm"));
    }
}

#[test]
fn invalid_plot_does_not_mutate_project_and_selection_rules_are_shared() {
    let mut project = Project::sample();
    let before = project.clone();
    let spec = PlotSpec {
        book_id: Some(1),
        y_keys: vec!["missing".into()],
        ..Default::default()
    };
    assert!(project.plot_spec(spec).is_err());
    assert_eq!(project, before);
    let spec = PlotSpec::from_selection(project.book(1).unwrap(), &["B".into()], PlotType::Line).unwrap();
    assert_eq!(spec.x_key.as_deref(), Some("A"));
    let id = project.plot_spec(spec).unwrap();
    assert_eq!(project.graph(id).unwrap().y_keys, vec!["B"]);
}

#[test]
fn mean_transform_handles_unsorted_groups_blanks_and_derived_curves() {
    let mut book = Book::new(1, "重复测量");
    for (key, role, data) in [
        ("A", Role::X, vec!["2", "1", "2", "1", "3", "4"]),
        ("B", Role::Y, vec!["4", "2", "8", "6", "", "bad"]),
        ("C", Role::Y, vec!["4", "2", "8", "6", "", "bad"]),
    ] {
        let mut c = Column::new(key, role);
        c.data = data.into_iter().map(String::from).collect();
        book.columns.push(c);
    }
    let mut spec = PlotSpec::from_selection(&book, &[], PlotType::Scatter).unwrap();
    spec.transform = Transform::Mean;
    spec.line_only.push("C".into());
    let (_, curves) = spec.series(&book);
    assert_eq!(curves[0].points, vec![[1.0, 4.0], [2.0, 6.0]]);
    assert_eq!(curves[1].points, vec![[2.0, 4.0], [1.0, 2.0], [2.0, 8.0], [1.0, 6.0]]);
}

#[test]
fn multiple_books_and_layers_have_independent_layout_and_clipping() {
    let mut project = Project::sample();
    let mut second = project.books[0].clone();
    second.id = project.alloc_id();
    second.name = "第二本".into();
    second.columns[1].long_name = "第二图层曲线".into();
    let layer = Layer {
        spec: PlotSpec::from_selection(&second, &["B".into()], PlotType::Bar).unwrap(),
        title: "第二图层标题".into(),
        ..Default::default()
    };
    project.books.push(second);
    project.graphs[0].page.add_layer(layer);
    project.graphs[0].page.columns = 2;
    let graph = &project.graphs[0];
    let sc = scene::build_page(graph, &project, 1000.0, 450.0, None);
    assert_eq!(sc.layer_plots.len(), 2);
    assert!(sc.layer_plots[0][0] < 500.0 && sc.layer_plots[1][0] > 500.0);
    let clipped: Vec<_> = sc.items.iter().filter_map(|item| item.clip).collect();
    assert!(clipped.contains(&sc.layer_plots[0]));
    assert!(clipped.contains(&sc.layer_plots[1]));
    let image = svg::render_page(graph, &project, 1000.0, 450.0);
    assert!(image.contains("clip2-1"));
    assert!(image.contains("第二图层标题"));
    assert!(image.contains("第二图层曲线"));

    let key = "B".to_string();
    project.delete_columns(1, &[key]);
    assert!(!project.graphs[0].page.layers()[0].y_keys.contains(&"B".into()));
    assert!(project.graphs[0].page.layers()[1].y_keys.contains(&"B".into()));
    project.remove(1);
    assert_eq!(project.graphs[0].page.layers()[0].book_id, None);
    assert!(project.graphs[0].page.layers()[1].book_id.is_some());
}

#[test]
fn coordinate_inverse_and_palette_boundaries() {
    for log in [false, true] {
        let x = scicanvas_core::ticks::axis(&[], false, 1.0, 100.0, log, false);
        let y = scicanvas_core::ticks::axis(&[], false, 1.0, 1000.0, log, false);
        let coord = Cartesian {
            x: &x,
            y: &y,
            rect: [30.0, 20.0, 700.0, 500.0],
        };
        for point in [[1.0, 1.0], [10.0, 100.0], [100.0, 1000.0]] {
            let recovered = coord.unproject(coord.project(point));
            assert!((point[0] - recovered[0]).abs() < 1e-9);
            assert!((point[1] - recovered[1]).abs() < 1e-9);
        }
    }
    assert_eq!(ColorMap::Viridis.sample(0.0), (68, 1, 84));
    assert_eq!(ColorMap::Viridis.sample(1.0), (253, 231, 37));
    assert_eq!(ColorMap::Viridis.sample(f64::NAN), ColorMap::Viridis.sample(0.0));
    assert_eq!(ColorMap::Colorblind.sample(-1.0), ColorMap::Colorblind.sample(0.0));
    let mut p = Project::sample();
    let original = svg::render(&p.graphs[0], p.book(1), 800.0, 600.0);
    p.graphs[0].color_map = ColorMap::Viridis;
    let colored = svg::render(&p.graphs[0], p.book(1), 800.0, 600.0);
    assert_ne!(original, colored);
    assert!(colored.contains("#440154"));
}
