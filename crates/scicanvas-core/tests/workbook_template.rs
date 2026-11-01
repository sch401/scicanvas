#![cfg(feature = "serde")]
use scicanvas_core::{project_file, DataKind, Project};

#[test]
fn workbook_template_roundtrips_column_definitions_without_data() {
    let mut project = Project::default();
    let id = project.new_book();
    let book = project.book_mut(id).unwrap();
    book.columns[0].long_name = "时间".into();
    book.columns[0].units = "s".into();
    book.columns[0].data = vec!["12345".into(), "67890".into()].into();
    book.columns[1].data = vec!["private-label".into()].into();
    book.convert_columns(&["B".into()], DataKind::Category).unwrap();
    book.columns[1].formula = Some("col(A) * 2".into());
    let template = project_file::workbook_template_from_book(book);
    let json = project_file::workbook_template_to_json(&template).unwrap();
    assert!(!json.contains("12345"));
    assert!(!json.contains("private-label"));
    let path = std::env::temp_dir().join(format!("scicanvas-template-{}.scbt", std::process::id()));
    project_file::save_workbook_template(&path, &template).unwrap();
    let loaded = project_file::load_workbook_template(&path).unwrap();
    std::fs::remove_file(path).unwrap();
    assert_eq!(loaded, template);
    let new = project_file::create_book_from_template(&mut project, &loaded).unwrap();
    let result = project.book(new).unwrap();
    assert_eq!(result.nrows(), 0);
    assert_eq!(result.columns[0].long_name, "时间");
    assert_eq!(result.columns[0].units, "s");
    assert_eq!(result.columns[1].data.kind(), DataKind::Category);
    assert_eq!(result.columns[1].formula.as_deref(), Some("col(A) * 2"));
    let mut bad = loaded;
    bad.columns[0].data = vec!["1".into()].into();
    assert!(project_file::workbook_template_to_json(&bad).is_err());
}
