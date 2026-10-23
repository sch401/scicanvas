use super::*;

#[test]
fn markdown_tables_escape_cells_and_pad_short_rows() {
    let rows = vec![
        vec!["列 A".to_string(), "p".to_string()],
        vec!["a|b".to_string(), "0.01".to_string()],
        vec!["两\n行".to_string()],
    ];
    assert_eq!(
        md_table(&rows),
        "| 列 A | p |\n| --- | --- |\n| a\\|b | 0.01 |\n| 两 行 |  |\n"
    );
    assert_eq!(md_table(&[]), "");
    assert_eq!(
        md_pairs(["项目", "值"], &[("t".into(), "2".into())]),
        "| 项目 | 值 |\n| --- | --- |\n| t | 2 |\n"
    );
}
