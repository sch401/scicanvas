use scicanvas_core::{history::History, PlotType, Project};

#[test]
fn folders_organize_books_graphs_and_promote_contents_without_deleting_data() {
    let mut p = Project::sample();
    let root = p.add_folder(None).unwrap();
    let sub = p.add_folder(Some(root)).unwrap();
    assert_eq!(p.folder_path(sub).as_deref(), Some("文件夹1 / 文件夹1"));
    p.move_project_item(1, Some(sub)).unwrap();
    p.move_project_item(2, Some(root)).unwrap();
    let (plot, _) = p.plot_from_selection(1, &["B".into()], PlotType::Line).unwrap();
    assert_eq!(p.folder_of(plot), Some(sub));
    let books = p.books.clone();
    let graphs = p.graphs.clone();
    p.remove_folder(root).unwrap();
    assert_eq!(p.folder(sub).unwrap().parent, None);
    assert_eq!(p.folder_of(2), None);
    p.remove_folder(sub).unwrap();
    assert!(p.folders.is_empty());
    assert_eq!(p.folder_of(1), None);
    assert_eq!(p.books, books);
    assert_eq!(p.graphs, graphs);
}

#[test]
fn invalid_moves_names_and_depth_are_atomic() {
    let mut p = Project::sample();
    let root = p.add_folder(None).unwrap();
    let child = p.add_folder(Some(root)).unwrap();
    let before = p.clone();
    for (id, to) in [(root, Some(root)), (root, Some(child)), (1, Some(999)), (999, None)] {
        assert!(p.move_project_item(id, to).is_err());
        assert_eq!(p, before);
    }
    assert!(p.rename_project_item(root, "  ").is_err());
    assert_eq!(p, before);
    p.rename_project_item(root, "实验组 α").unwrap();
    assert_eq!(p.folder(root).unwrap().name, "实验组 α");
    let mut parent = child;
    for _ in 2..64 {
        parent = p.add_folder(Some(parent)).unwrap();
    }
    let before = p.clone();
    assert!(p.add_folder(Some(parent)).is_err());
    assert_eq!(p, before);
}

#[test]
fn deleting_objects_cleans_membership_and_folder_history_roundtrips() {
    let mut p = Project::sample();
    let before = p.clone();
    let mut h = History::default();
    h.begin(&p, "workspace");
    let f = p.add_folder(None).unwrap();
    p.move_project_item(1, Some(f)).unwrap();
    p.move_project_item(2, Some(f)).unwrap();
    h.changed();
    h.commit(&p);
    let after = p.clone();
    p = h.undo(&p).unwrap();
    assert_eq!(p, before);
    p = h.redo(&p).unwrap();
    assert_eq!(p, after);
    p.remove(1);
    assert_eq!(p.folder(f).unwrap().items, [2]);
    assert_eq!(p.graph(2).unwrap().first_layer().book_id, None);
    p.validate_folders().unwrap();
}

#[cfg(feature = "serde")]
#[test]
fn v8_saves_folders_and_v7_projects_migrate_to_root() {
    use scicanvas_core::project_file;
    let mut p = Project::sample();
    let f = p.add_folder(None).unwrap();
    p.move_project_item(1, Some(f)).unwrap();
    let text = project_file::to_json(&p).unwrap();
    assert_eq!(project_file::from_json(&text).unwrap(), p);
    let mut v: serde_json::Value = serde_json::from_str(&text).unwrap();
    v["folders"][0]["id"] = 1000.into();
    v["next_id"] = 2.into();
    assert_eq!(project_file::from_json(&v.to_string()).unwrap().next_id, 1001);
    v["version"] = 7.into();
    v.as_object_mut().unwrap().remove("folders");
    let old = project_file::from_json(&v.to_string()).unwrap();
    assert_eq!(old.version, project_file::CURRENT_VERSION);
    assert!(old.folders.is_empty());
    assert_eq!(old.books, p.books);
    assert_eq!(old.graphs, p.graphs);
}

#[test]
fn newly_created_analysis_outputs_follow_the_source_book_folder() {
    let mut p = Project::sample();
    p.graphs.clear();
    let folder = p.add_folder(None).unwrap();
    p.move_project_item(1, Some(folder)).unwrap();
    let source = scicanvas_core::ops::Source::Book {
        id: 1,
        selected: vec!["B".into()],
        rows: None,
    };
    let (graph, _) = p.fit_poly(&source, 1).unwrap();
    assert_eq!(p.folder_of(graph), Some(folder));
    let stats = p.stats_on_columns(&source).unwrap();
    assert_eq!(p.folder_of(stats), Some(folder));
}

#[cfg(feature = "serde")]
#[test]
fn corrupt_folder_ids_cycles_membership_and_missing_parents_are_rejected() {
    use scicanvas_core::project_file;
    let mut p = Project::sample();
    let f = p.add_folder(None).unwrap();
    let child = p.add_folder(Some(f)).unwrap();
    let mut variants = Vec::new();
    let mut bad = p.clone();
    bad.folders[0].id = 1;
    variants.push(bad);
    let mut bad = p.clone();
    bad.folders[0].parent = Some(child);
    variants.push(bad);
    let mut bad = p.clone();
    bad.folders[0].parent = Some(999);
    variants.push(bad);
    let mut bad = p.clone();
    bad.folders[0].items = vec![1];
    bad.folders[1].items = vec![1];
    variants.push(bad);
    let mut bad = p.clone();
    bad.folders[0].items = vec![999];
    variants.push(bad);
    for bad in variants {
        assert!(project_file::to_json(&bad).is_err());
        assert!(project_file::from_json(&serde_json::to_string(&bad).unwrap()).is_err());
    }
}
