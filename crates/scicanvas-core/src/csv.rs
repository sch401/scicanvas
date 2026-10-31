//! CSV / TXT 导入与剪贴板粘贴。

use crate::model::{looks_numeric, Book, Role};

/// 把文件字节解码成文本：先按 UTF-8（去掉 BOM），失败时按 GBK（需开启 `encoding` 特性）。
pub fn decode(bytes: &[u8]) -> String {
    let bytes = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(bytes);
    match std::str::from_utf8(bytes) {
        Ok(s) => s.to_string(),
        Err(_) => decode_fallback(bytes),
    }
}

#[cfg(feature = "encoding")]
fn decode_fallback(bytes: &[u8]) -> String {
    let (text, _, _) = encoding_rs::GBK.decode(bytes);
    text.into_owned()
}

#[cfg(not(feature = "encoding"))]
fn decode_fallback(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Delim {
    Char(char),
    Whitespace,
}

/// 把文本切成表格。自动识别分隔符：制表符 > 逗号 > 分号 > 空白。
/// 支持双引号包裹的字段（字段里可以有分隔符，"" 表示一个引号）。
pub fn parse_table(text: &str) -> Vec<Vec<String>> {
    let text = text.replace("\r\n", "\n").replace('\r', "\n");
    let lines = table_lines(&text);
    let delim = detect_delim(&lines);
    lines.iter().map(|l| split_line(l, delim)).collect()
}

/// 按行切分，去掉末尾的空行。
fn table_lines(text: &str) -> Vec<&str> {
    let mut lines: Vec<&str> = text.split('\n').collect();
    while lines.last().is_some_and(|l| l.trim().is_empty()) {
        lines.pop();
    }
    lines
}

fn detect_delim(lines: &[&str]) -> Delim {
    if lines.iter().any(|l| l.contains('\t')) {
        Delim::Char('\t')
    } else if lines.iter().any(|l| l.contains(',')) {
        Delim::Char(',')
    } else if lines.iter().any(|l| l.contains(';')) {
        Delim::Char(';')
    } else {
        Delim::Whitespace
    }
}

/// 由文本直接建工作簿（结果与 `book_from_table(parse_table(text))` 相同）：逐行解析后直接写入
/// 类型化列，不生成整张字符串表，百万行导入时峰值内存小得多。
pub fn book_from_text(id: u64, name: &str, text: &str) -> Book {
    let normalized;
    let text = if text.contains('\r') {
        normalized = text.replace("\r\n", "\n").replace('\r', "\n");
        normalized.as_str()
    } else {
        text
    };
    let lines = table_lines(text);
    let delim = detect_delim(&lines);
    let mut book = Book::new(id, name);
    let first = lines.first().map(|l| split_line(l, delim));
    let header = first.filter(|row| lines.len() > 1 && row.iter().any(|s| !s.trim().is_empty() && !looks_numeric(s)));
    let add_column = |book: &mut Book, j: usize, rows_so_far: usize| {
        let key = book.push_column(if j == 0 { Role::X } else { Role::Y });
        let col = book.columns.iter_mut().find(|c| c.key == key).unwrap();
        if let Some(h) = &header {
            let (n, u) = split_name_units(h.get(j).map(|s| s.as_str()).unwrap_or(""));
            col.long_name = n;
            col.units = u;
        }
        for _ in 0..rows_so_far {
            col.data.push(String::new());
        }
    };
    for j in 0..header.as_ref().map_or(0, Vec::len) {
        add_column(&mut book, j, 0);
    }
    let skip = usize::from(header.is_some());
    for (r, line) in lines.iter().skip(skip).enumerate() {
        let cells = split_line(line, delim);
        while book.columns.len() < cells.len() {
            let j = book.columns.len();
            add_column(&mut book, j, r);
        }
        let mut cells = cells.into_iter();
        for col in &mut book.columns {
            col.data.push(cells.next().unwrap_or_default());
        }
    }
    finish_columns(&mut book);
    book
}

fn split_line(line: &str, delim: Delim) -> Vec<String> {
    match delim {
        Delim::Whitespace => line.split_whitespace().map(|s| s.to_string()).collect(),
        Delim::Char(d) => {
            let mut out = Vec::new();
            let mut cur = String::new();
            let mut in_quotes = false;
            let mut chars = line.chars().peekable();
            while let Some(ch) = chars.next() {
                if in_quotes {
                    if ch == '"' {
                        if chars.peek() == Some(&'"') {
                            cur.push('"');
                            chars.next();
                        } else {
                            in_quotes = false;
                        }
                    } else {
                        cur.push(ch);
                    }
                } else if ch == '"' && cur.trim().is_empty() {
                    cur.clear();
                    in_quotes = true;
                } else if ch == d {
                    out.push(cur.trim().to_string());
                    cur.clear();
                } else {
                    cur.push(ch);
                }
            }
            out.push(cur.trim().to_string());
            out
        }
    }
}

/// 表头形如 “电压 (mV)”、“温度[K]”、“压强（kPa）” 时拆成长名称和单位。
pub fn split_name_units(header: &str) -> (String, String) {
    let h = header.trim();
    let pairs = [('(', ')'), ('（', '）'), ('[', ']')];
    for (open, close) in pairs {
        if h.ends_with(close) {
            if let Some(pos) = h.rfind(open) {
                let name = h[..pos].trim();
                let unit = h[pos + open.len_utf8()..h.len() - close.len_utf8()].trim();
                if !unit.is_empty() {
                    return (name.to_string(), unit.to_string());
                }
            }
        }
    }
    (h.to_string(), String::new())
}

/// 第一行里有非数值文字时把它当作表头。
fn has_header(rows: &[Vec<String>]) -> bool {
    rows.len() > 1 && rows[0].iter().any(|s| !s.trim().is_empty() && !looks_numeric(s))
}

/// 由表格创建工作簿：第一列为 X，其余为 Y；有表头时填入长名称和单位。
pub fn book_from_table(id: u64, name: &str, mut rows: Vec<Vec<String>>) -> Book {
    let mut book = Book::new(id, name);
    let header = if has_header(&rows) { Some(rows.remove(0)) } else { None };
    let ncols = rows
        .iter()
        .map(|r| r.len())
        .max()
        .unwrap_or(0)
        .max(header.as_ref().map_or(0, |h| h.len()));
    for j in 0..ncols {
        let key = book.push_column(if j == 0 { Role::X } else { Role::Y });
        let col = book.columns.iter_mut().find(|c| c.key == key).unwrap();
        if let Some(h) = &header {
            let (n, u) = split_name_units(h.get(j).map(|s| s.as_str()).unwrap_or(""));
            col.long_name = n;
            col.units = u;
        }
    }
    // Consume cells into typed columns; avoid cloning the complete row-major table.
    for row in rows {
        let mut cells = row.into_iter();
        for col in &mut book.columns {
            col.data.push(cells.next().unwrap_or_default());
        }
    }
    finish_columns(&mut book);
    book
}

/// 去掉末尾空值；全部是日期写法的文本列（如 Excel 日期、ISO 日期）转为日期时间列。
fn finish_columns(book: &mut Book) {
    for col in &mut book.columns {
        col.data.trim_empty();
        if col.data.kind() == crate::DataKind::Text
            && (0..col.data.len()).any(|r| !col.data.text_at(r).trim().is_empty())
        {
            if let Ok(dates) = col.data.converted(crate::DataKind::DateTime) {
                col.data = dates.into();
            }
        }
    }
}

/// 文本里有制表符或换行时视为多单元格粘贴（从 Excel 复制的一块区域）。
pub fn is_block_paste(text: &str) -> bool {
    text.contains('\t') || text.trim_end().contains('\n')
}

// Excel 剪贴板按制表符分列；单列中的逗号和空格是内容，不能套用文件导入的分隔符推断。
fn paste_table(text: &str) -> Vec<Vec<String>> {
    let text = text.replace("\r\n", "\n").replace('\r', "\n");
    table_lines(&text)
        .iter()
        .map(|line| split_line(line, Delim::Char('\t')))
        .collect()
}

fn paste_has_header(rows: &[Vec<String>]) -> bool {
    has_header(rows)
        && rows[0]
            .iter()
            .all(|v| v.trim().is_empty() || !crate::ColumnData::default().accepts(v))
        && rows[1..].iter().any(|row| row.iter().any(|v| looks_numeric(v)))
}

/// 从 (col, row) 开始粘贴一块数据，列不够时自动补列。
pub fn paste_conflicts(book: &Book, col: usize, row: usize, text: &str) -> Vec<String> {
    let mut rows = paste_table(text);
    if row == 0 && paste_has_header(&rows) {
        rows.remove(0);
    }
    let mut conflicts = std::collections::BTreeSet::new();
    for row in rows {
        for (j, value) in row.iter().enumerate() {
            if let Some(column) = book.columns.get(col + j) {
                if !column.data.accepts_paste(value) {
                    conflicts.insert(column.key.clone());
                }
            }
        }
    }
    conflicts.into_iter().collect()
}

/// 从 (col, row) 开始粘贴一块数据，列不够时自动补列。
/// 从第一行开始粘贴且首行是文字时，首行写入长名称。返回 (行数, 列数)。
pub fn paste_into(book: &mut Book, col: usize, row: usize, text: &str) -> (usize, usize) {
    let mut rows = paste_table(text);
    if row == 0 && paste_has_header(&rows) {
        let header = rows.remove(0);
        book.ensure_columns(col + header.len());
        for (j, h) in header.iter().enumerate() {
            let (n, u) = split_name_units(h);
            let c = &mut book.columns[col + j];
            c.long_name = n;
            if !u.is_empty() {
                c.units = u;
            }
        }
    }
    let ncols = rows.iter().map(|r| r.len()).max().unwrap_or(0);
    book.ensure_columns(col + ncols);
    for (i, r) in rows.iter().enumerate() {
        for (j, v) in r.iter().enumerate() {
            book.columns[col + j].set(row + i, v.clone());
        }
    }
    (rows.len(), ncols)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn streaming_import_matches_the_row_table_import() {
        for text in [
            "时间 (s),电压 (mV)
0,1.5
1,2.5
",
            "1 2 3
4 5 6
",
            "a	b
\"x, y\"	2
3	\"he said \"\"hi\"\"\"
",
            "X,Y
1,2
3,4,5,6
7
",
            "日期,值
2026-10-18,1
2026-10-19,2


",
            "A,B,C,D
1,2
",
            "only
",
            "",
            "1;2
3;x
;5
",
        ] {
            let a = book_from_text(1, "T", text);
            let b = book_from_table(1, "T", parse_table(text));
            assert_eq!(a, b, "{text:?}");
        }
    }

    #[test]
    fn detects_delimiters() {
        assert_eq!(parse_table("1\t2\n3\t4\n"), vec![vec!["1", "2"], vec!["3", "4"]]);
        assert_eq!(parse_table("a,b\r\n1,2"), vec![vec!["a", "b"], vec!["1", "2"]]);
        assert_eq!(parse_table("1  2\n3 4"), vec![vec!["1", "2"], vec!["3", "4"]]);
        assert_eq!(parse_table("1;2"), vec![vec!["1", "2"]]);
    }

    #[test]
    fn quoted_fields() {
        assert_eq!(
            parse_table("\"a, b\",2\n\"say \"\"hi\"\"\",3"),
            vec![vec!["a, b", "2"], vec!["say \"hi\"", "3"]]
        );
    }

    #[test]
    fn units_from_header() {
        assert_eq!(split_name_units("电压 (mV)"), ("电压".into(), "mV".into()));
        assert_eq!(split_name_units("压强（kPa）"), ("压强".into(), "kPa".into()));
        assert_eq!(split_name_units("T[K]"), ("T".into(), "K".into()));
        assert_eq!(split_name_units("时间"), ("时间".into(), "".into()));
    }

    #[test]
    fn import_with_header() {
        let b = book_from_table(5, "data", parse_table("时间 (s),信号\n0,1.5\n1,2.5\n2,"));
        assert_eq!(b.columns.len(), 2);
        assert_eq!(b.columns[0].role, Role::X);
        assert_eq!(b.columns[0].long_name, "时间");
        assert_eq!(b.columns[0].units, "s");
        assert_eq!(b.columns[1].data, vec!["1.5", "2.5"]);
        assert_eq!(b.nrows(), 3);
    }

    #[test]
    fn paste_block_grows_columns() {
        let mut b = Book::new(1, "B");
        b.push_column(Role::X);
        let (r, c) = paste_into(&mut b, 0, 0, "x\ty\tz\n1\t2\t3\n4\t5\t6\n");
        assert_eq!((r, c), (2, 3));
        assert_eq!(b.columns.len(), 3);
        assert_eq!(b.columns[2].long_name, "z");
        assert_eq!(b.columns[1].data, vec!["2", "5"]);
        // 从中间粘贴不吃表头
        paste_into(&mut b, 1, 5, "7\t8");
        assert_eq!(b.columns[1].data.text_at(5), "7");
        assert_eq!(b.columns[2].data.text_at(5), "8");
    }

    #[test]
    fn decode_bom() {
        assert_eq!(decode(&[0xEF, 0xBB, 0xBF, b'a']), "a");
    }

    #[test]
    fn excel_interval_data_keeps_first_row_and_adapts_numeric_columns() {
        let text = "4-10\t73000\t1500\r\n10-40\t76000\t11000\r\n40-100\t81000\t5400\r\n";
        for row in [0, 2] {
            let mut b = Book::new(1, "B");
            b.ensure_columns(3);
            assert!(paste_conflicts(&b, 0, row, text).is_empty());
            assert_eq!(paste_into(&mut b, 0, row, text), (3, 3));
            assert_eq!(b.columns[0].data.text_at(row), "4-10");
            assert_eq!(b.columns[0].data.text_at(row + 2), "40-100");
            assert_eq!(b.columns[1].data.f64_at(row), Some(73000.0));
            assert!(b.columns.iter().all(|c| c.long_name.is_empty()));
            assert_eq!(b.columns[0].data.kind(), crate::DataKind::Text);
            let old_number = b.columns[1].data.f64_at(row + 2);
            assert!(paste_conflicts(&b, 1, row, "wrong\t1\r\nstill wrong\t2").is_empty());
            paste_into(&mut b, 1, row, "wrong\t1\r\nstill wrong\t2");
            assert_eq!(b.columns[1].data.text_at(row), "wrong");
            assert_eq!(b.columns[1].data.f64_at(row + 2), old_number);
        }
    }

    #[test]
    fn excel_single_columns_keep_thousands_separators_and_spaces() {
        for (text, expected) in [
            ("1,234.5\r\n2,345.75\r\n", vec!["1,234.5", "2,345.75"]),
            ("very fine\r\ncoarse grain\r\n", vec!["very fine", "coarse grain"]),
        ] {
            let mut b = Book::new(1, "B");
            b.ensure_columns(1);
            assert!(paste_conflicts(&b, 0, 0, text).is_empty());
            assert_eq!(paste_into(&mut b, 0, 0, text), (2, 1));
            assert_eq!(b.columns.len(), 1);
            for (i, value) in expected.iter().enumerate() {
                assert_eq!(b.columns[0].data.text_at(i), *value);
            }
        }
    }
}
