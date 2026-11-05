#![cfg(feature = "serde")]
use scicanvas_core::{
    ops::{descriptive_table, Source},
    project_file, Book, Column, PlotType, Project, Role,
};
use serde_json::Value;
fn reference() -> Value {
    serde_json::from_str(include_str!("reference/statistical_reports.json")).unwrap()
}
fn groups(doc: &Value) -> &Vec<Value> {
    doc["cases"][0]["groups"].as_array().unwrap()
}
fn dataset() -> Project {
    let doc = reference();
    let mut p = Project::default();
    let id = p.alloc_id();
    let mut book = Book::new(id, "统计输入");
    let mut x = Column::new("A", Role::X);
    x.data = (0..5).map(|i| i.to_string()).collect();
    book.columns.push(x);
    for (i, group) in groups(&doc).iter().enumerate() {
        let mut c = Column::new(scicanvas_core::column_letter(i + 1), Role::Y);
        c.long_name = group["name"].as_str().unwrap().into();
        c.data = group["values"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_f64().map_or_else(String::new, |v| v.to_string()))
            .collect();
        book.columns.push(c);
    }
    let mut text = Column::new("F", Role::Label);
    text.data = ["a", "b", "c", "d", "e"].map(str::to_owned).to_vec().into();
    book.columns.push(text);
    p.books.push(book);
    p
}
fn check(table: &[Vec<String>], column: usize, expected: &Value) {
    for (i, key) in ["n", "mean", "sd", "se", "min", "q1", "median", "q3", "max", "missing"]
        .into_iter()
        .enumerate()
    {
        let actual = &table[i + 1][column];
        if let Some(value) = expected[key].as_f64() {
            assert!((actual.parse::<f64>().unwrap() - value).abs() < 1e-12, "{key}");
        } else {
            assert!(actual.is_empty(), "{key}");
        }
    }
}
#[test]
fn a7_whole_and_side_by_side_statistics_match_numpy() {
    let doc = reference();
    let p = dataset();
    let before = p.clone();
    let whole = p.whole_table_statistics(1, &[], None).unwrap();
    check(&whole, 1, &doc["cases"][1]["stats"]);
    let selected = p
        .whole_table_statistics(1, &["B".into(), "C".into()], Some((1, 3)))
        .unwrap();
    check(&selected, 1, &doc["cases"][2]["stats"]);
    assert_eq!(p, before);
    let columns = p
        .analysis_columns(&Source::Book {
            id: 1,
            selected: vec!["B".into(), "C".into(), "D".into(), "E".into()],
            rows: None,
        })
        .unwrap();
    let input: Vec<_> = columns.iter().map(|c| (c.name.as_str(), c.values.as_slice())).collect();
    let table = descriptive_table(&input);
    for (i, group) in groups(&doc).iter().enumerate() {
        check(&table, i + 1, &group["stats"]);
    }
}
#[test]
fn a7_comparison_table_box_and_graphical_summary_save_together() {
    let mut p = dataset();
    let before = p.clone();
    let (book, graph) = p
        .compare_statistics(
            &Source::Book {
                id: 1,
                selected: vec!["B".into(), "C".into()],
                rows: None,
            },
            true,
        )
        .unwrap();
    assert_eq!(p.books.len(), before.books.len() + 1);
    assert_eq!(p.graph(graph.unwrap()).unwrap().first_layer().plot_type, PlotType::Box);
    assert_eq!(p.book(book).unwrap().columns.len(), 3);
    let column = p
        .analysis_columns(&Source::Book {
            id: 1,
            selected: vec!["B".into()],
            rows: Some((1, 3)),
        })
        .unwrap()
        .remove(0);
    let count = p.notes.len();
    let (id, note) = p.graphical_summary(&column).unwrap();
    let graph = p.graph(id).unwrap();
    assert_eq!(
        graph.page.layers().iter().map(|l| l.plot_type).collect::<Vec<_>>(),
        vec![PlotType::Histogram, PlotType::Box, PlotType::QQ]
    );
    assert!(graph.page.layers().iter().all(|l| l.rows == Some((1, 3))));
    assert_eq!(graph.page.annotations.len(), 1);
    assert!(graph.page.annotations[0].text.contains("N = 3"));
    assert!(graph.page.annotations[0].text.contains("p ="));
    assert_eq!(p.notes.len(), count + 1);
    assert!(p.note(note).unwrap().text.contains("Shapiro"));
    assert_eq!(p.book(1).unwrap(), before.book(1).unwrap());
    let json = project_file::to_json(&p).unwrap();
    assert_eq!(p, project_file::from_json(&json).unwrap());
}
