use scicanvas_core::{DataKind, Project};

#[test]
fn stack_split_and_reduce_keep_values_types_and_missing_cells() {
    let mut project = Project::default();
    let id = project.new_book();
    let book = project.book_mut(id).unwrap();
    book.columns[0].data = ["1", "", "3", "4", "5"].map(str::to_string).to_vec().into();
    book.columns[1].data = ["7", "8", ""].map(str::to_string).to_vec().into();
    let a = book.columns[0].key.clone();
    let b = book.columns[1].key.clone();
    let stacked = project.stack_columns(id, &[a.clone(), b.clone()]).unwrap();
    let book = project.book(id).unwrap();
    let value = &book.columns[book.column_index(&stacked[0]).unwrap()];
    let category = &book.columns[book.column_index(&stacked[1]).unwrap()];
    assert_eq!(value.data.kind(), DataKind::Num);
    assert_eq!(category.data.kind(), DataKind::Category);
    assert_eq!(value.data.text_at(1), "");
    assert_eq!(category.data.text_at(5), b);
    let split = project.split_column(id, &stacked[0], &stacked[1]).unwrap();
    let book = project.book(id).unwrap();
    assert_eq!(book.columns[book.column_index(&split[0]).unwrap()].data.text_at(1), "");
    assert_eq!(book.columns[book.column_index(&split[1]).unwrap()].data.text_at(1), "8");
    let mean = project.reduce_column(id, &a, 2, true).unwrap();
    let sample = project.reduce_column(id, &a, 2, false).unwrap();
    let book = project.book(id).unwrap();
    let mean = &book.columns[book.column_index(&mean).unwrap()];
    assert_eq!(
        (mean.data.f64_at(0), mean.data.f64_at(1), mean.data.f64_at(2)),
        (Some(1.0), Some(3.5), Some(5.0))
    );
    let sample = &book.columns[book.column_index(&sample).unwrap()];
    assert_eq!(
        (
            sample.data.text_at(0).as_ref(),
            sample.data.text_at(1).as_ref(),
            sample.data.text_at(2).as_ref()
        ),
        ("1", "3", "5")
    );
    let before = project.clone();
    assert!(project.reduce_column(id, &a, 0, false).is_err());
    assert!(project.split_column(id, &a, &b).is_err());
    assert_eq!(project, before);
}
