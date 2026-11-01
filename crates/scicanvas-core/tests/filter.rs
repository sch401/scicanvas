use scicanvas_core::{filter::{Filter, FilterRule}, ColumnData, DataKind, Project};

#[test]
fn filters_combine_without_removing_rows_and_follow_column_deletion() {
    let mut project = Project::sample();
    let book = project.book_mut(1).unwrap();
    book.columns[0].data = vec!["1".into(), "2".into(), "3".into(), "4".into()].into();
    book.columns[1].data = ColumnData::Text(["apple", "pear", "apricot", "apple"].map(str::to_string).to_vec()).into();
    book.columns[2].data = ColumnData::Text(["red", "red", "blue", "red"].map(str::to_string).to_vec()).converted(DataKind::Category).unwrap().into();
    let original = book.columns.clone();
    let mut filter = Filter::default();
    filter.rules.insert("A".into(), FilterRule::Number { min: Some(2.0), max: Some(4.0) });
    filter.rules.insert("B".into(), FilterRule::Text { contains: "ap".into() });
    filter.rules.insert("C".into(), FilterRule::Category { values: vec!["red".into()] });
    filter.validate(book).unwrap();
    book.filter = Some(filter);
    assert_eq!(book.visible_rows(), vec![3]);
    assert_eq!(book.columns, original);
    project.delete_columns(1, &["B".into()]);
    assert_eq!(project.book(1).unwrap().visible_rows(), vec![1, 3]);
    project.delete_columns(1, &["A".into(), "C".into()]);
    assert!(project.book(1).unwrap().filter.is_none());
}

#[cfg(feature = "serde")]
#[test]
fn filter_project_format_v23_roundtrips_and_reads_v22() {
    use scicanvas_core::project_file;
    let mut project = Project::sample();
    let mut old: serde_json::Value = serde_json::from_str(&project_file::to_json(&project).unwrap()).unwrap();
    old["version"] = 22.into();
    let loaded = project_file::from_json(&old.to_string()).unwrap();
    assert_eq!(loaded.version, 23);
    assert!(loaded.books[0].filter.is_none());
    let mut filter = Filter::default();
    filter.rules.insert("A".into(), FilterRule::Number { min: Some(1.0), max: Some(3.0) });
    project.books[0].filter = Some(filter);
    let loaded = project_file::from_scv(&project_file::to_scv(&project).unwrap()).unwrap();
    assert_eq!(loaded, project);
    old["version"] = 24.into();
    assert!(project_file::from_json(&old.to_string()).unwrap_err().contains("高于当前支持"));
}

#[test]
fn copy_paste_and_clear_filtered_ranges_leave_hidden_rows_untouched() {
    let mut project = Project::sample();
    let book = project.book_mut(1).unwrap();
    book.columns[0].data = (1..=6).map(|v| v.to_string()).collect::<Vec<_>>().into();
    book.columns[1].data = ColumnData::Text(["keep", "hide", "keep", "hide", "keep", "hide"].map(str::to_string).to_vec()).converted(DataKind::Category).unwrap().into();
    let mut filter = Filter::default();
    filter.rules.insert("B".into(), FilterRule::Category { values: vec!["keep".into()] });
    book.filter = Some(filter);
    let visible = book.visible_rows();
    assert_eq!(visible, vec![0, 2, 4]);
    assert_eq!(book.visible_range_tsv(0, 0, 0, 5), "1\n3\n5");
    assert_eq!(scicanvas_core::csv::paste_into_visible(book, 0, &visible, 0, "10\n20\n30").unwrap(), (3, 1, 4));
    assert_eq!((book.columns[0].data.text_at(0), book.columns[0].data.text_at(1), book.columns[0].data.text_at(2), book.columns[0].data.text_at(3), book.columns[0].data.text_at(4), book.columns[0].data.text_at(5)), ("10".into(), "2".into(), "20".into(), "4".into(), "30".into(), "6".into()));
    let before = book.clone();
    assert!(scicanvas_core::csv::paste_into_visible(book, 0, &visible, 0, "1\n2\n3\n4").is_err());
    assert_eq!(*book, before);
    assert_eq!(book.clear_visible_range(0, 0, 0, 5), 3);
    assert_eq!((book.columns[0].data.text_at(1), book.columns[0].data.text_at(3), book.columns[0].data.text_at(5)), ("2".into(), "4".into(), "6".into()));
}

#[cfg(feature = "serde")]
#[test]
fn visible_only_analysis_plot_and_recalculation_keep_original_rows() {
    use scicanvas_core::{analysis::SmoothMethod, ops::{get_xy, Source}, project_file, PlotType};
    let mut project = Project::sample();
    let book = project.book_mut(1).unwrap();
    book.columns[0].data = (1..=5).map(|v| v.to_string()).collect::<Vec<_>>().into();
    book.columns[1].data = (1..=5).map(|v| (v * 2).to_string()).collect::<Vec<_>>().into();
    let mut filter = Filter::default();
    filter.rules.insert("A".into(), FilterRule::Number { min: Some(2.0), max: Some(4.0) });
    book.filter = Some(filter);
    let source = Source::Visible(Box::new(Source::Book { id: 1, selected: vec!["B".into()], rows: None }));
    let resolved = project.resolve(&source).unwrap();
    let (xs, ys, rows) = get_xy(project.book(1).unwrap(), &resolved.items[0]);
    assert_eq!((xs, ys, rows), (vec![2.0, 3.0, 4.0], vec![4.0, 6.0, 8.0], vec![1, 2, 3]));
    assert_eq!(project.analysis_columns(&source).unwrap()[0].values, vec![4.0, 6.0, 8.0]);
    let (graph, _) = project.plot_from_selection(1, &["B".into()], PlotType::Line).unwrap();
    project.graph_mut(graph).unwrap().first_layer_mut().visible_only = true;
    let (_, series) = project.graph(graph).unwrap().first_layer().series(project.book(1).unwrap());
    assert_eq!(series[0].rows, vec![1, 2, 3]);
    let output = project.smooth(&source, SmoothMethod::MovingAverage, 3).unwrap().remove(0);
    let result = project.book(1).unwrap().column(&output).unwrap();
    assert!(result.op.as_ref().unwrap().visible_only);
    assert!(result.data.f64_at(0).is_none());
    assert!(result.data.f64_at(1).is_some());
    let loaded = project_file::from_scv(&project_file::to_scv(&project).unwrap()).unwrap();
    assert!(loaded.graph(graph).unwrap().first_layer().visible_only);
    assert!(loaded.book(1).unwrap().column(&output).unwrap().op.as_ref().unwrap().visible_only);
    project.book_mut(1).unwrap().filter.as_mut().unwrap().rules.insert("A".into(), FilterRule::Number { min: Some(3.0), max: Some(5.0) });
    project.recalc_computed(false);
    let result = project.book(1).unwrap().column(&output).unwrap();
    assert!(result.data.f64_at(1).is_none());
    assert!(result.data.f64_at(4).is_some());
    let (_, series) = project.graph(graph).unwrap().first_layer().series(project.book(1).unwrap());
    assert_eq!(series[0].rows, vec![2, 3, 4]);
}
