#![cfg(feature = "xlsx")]
use scicanvas_core::{xlsx, xlsx_export, ColumnData, DataKind, Project};
use std::io::{Cursor, Read};

#[test]
fn export_xlsx_preserves_typed_cells_headers_dates_and_sheet_names() {
    let mut project = Project::default();
    let first = project.new_book();
    let book = project.book_mut(first).unwrap();
    book.name = "Data/😀".into();
    book.ensure_columns(4);
    book.columns[0].long_name = "Value".into();
    book.columns[0].units = "kg".into();
    book.columns[0].comment = "measured".into();
    book.columns[0].data = vec!["1.25".into(), "".into(), "-3".into()].into();
    book.columns[1].data = ColumnData::Text(vec!["2026-11-01T08:30:15.123Z".into()]).into();
    book.convert_columns(&["B".into()], DataKind::DateTime).unwrap();
    book.columns[1].format.utc_offset = 60;
    book.columns[2].data = ColumnData::Text(vec!["=not a formula".into(), "<xml&>".into()]).into();
    book.columns[3].data = ColumnData::Text(vec!["red".into(), "blue".into()])
        .converted(DataKind::Category)
        .unwrap()
        .into();
    let second = project.new_book();
    project.book_mut(second).unwrap().name = "data/😀".into();
    let bytes = xlsx_export::write_project(&project, true).unwrap();
    let sheets = xlsx::read(&bytes).unwrap();
    assert_eq!(sheets.len(), 2);
    assert_eq!(sheets[0].name, "Data_😀");
    assert_eq!(sheets[1].name, "data_😀 (2)");
    assert_eq!(sheets[0].rows[0][0], "Value");
    assert_eq!(sheets[0].rows[1][0], "kg");
    assert_eq!(sheets[0].rows[2][0], "measured");
    assert_eq!(sheets[0].rows[3][0], "1.25");
    assert_eq!(sheets[0].rows[3][1], "2026-11-01 09:30:15");
    assert_eq!(sheets[0].rows[3][2], "=not a formula");
    assert_eq!(sheets[0].rows[4][0], "");
    assert_eq!(sheets[0].rows[4][2], "<xml&>");
    assert_eq!(sheets[0].rows[4][3], "blue");
    let mut zip = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut xml = String::new();
    zip.by_name("xl/worksheets/sheet1.xml")
        .unwrap()
        .read_to_string(&mut xml)
        .unwrap();
    let doc = roxmltree::Document::parse(&xml).unwrap();
    let cell = |address| {
        doc.descendants()
            .find(|n| n.has_tag_name("c") && n.attribute("r") == Some(address))
            .unwrap()
    };
    let numeric = cell("A4");
    assert!(numeric.attribute("t").is_none());
    let date = cell("B4");
    assert!(date.attribute("s").is_some());
    let serial: f64 = date
        .children()
        .find(|n| n.has_tag_name("v"))
        .unwrap()
        .text()
        .unwrap()
        .parse()
        .unwrap();
    assert!((serial - 46327.39600836806).abs() < 1e-9);
    assert!(cell("C4").children().all(|n| !n.has_tag_name("f")));
    let without_headers = xlsx_export::write_project(&project, false).unwrap();
    assert_eq!(xlsx::read(&without_headers).unwrap()[0].rows[0][0], "1.25");
}

#[cfg(feature = "serde")]
#[test]
fn excel_limit_does_not_replace_existing_file() {
    let mut project = Project::default();
    let id = project.new_book();
    let rows = 1_048_574;
    project.book_mut(id).unwrap().columns[0].data = ColumnData::Num {
        values: vec![0.0; rows],
        missing: vec![u64::MAX; rows.div_ceil(64)],
        lexemes: Default::default(),
    }
    .into();
    let path = std::env::temp_dir().join(format!("scicanvas-export-limit-{}.xlsx", std::process::id()));
    std::fs::write(&path, b"original").unwrap();
    assert!(scicanvas_core::project_file::save_xlsx_export(&path, &project, true).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), b"original");
    std::fs::remove_file(path).unwrap();
}
