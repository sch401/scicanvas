//! 读取 Excel 工作簿（.xlsx / .xlsm）：ZIP 容器里的 workbook、关系、共享字符串、样式和工作表 XML。
//! 只取单元格的值（公式取缓存结果），数字整理成工作表的显示格式；按单元格数字格式把日期序列号
//! 写成日期，合并单元格用左上角的值填满。旧版 .xls 交给 [`crate::xls`]。

use std::io::{Cursor, Read};

use roxmltree::{Document, Node};

use crate::model::parse_num;

/// 一张工作表：表名和按行排列的单元格文本（右侧、下方的空单元格已去掉）。
#[derive(Clone, Debug, PartialEq)]
pub struct Sheet {
    pub name: String,
    pub rows: Vec<Vec<String>>,
}

const MAIN_NS: &str = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";
const REL_NS: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
/// 单个 XML 条目解压后的上限，防止损坏或恶意文件耗尽内存。
const MAX_ENTRY: u64 = 1 << 30;

type Archive<'a> = zip::ZipArchive<Cursor<&'a [u8]>>;

fn entry(zip: &mut Archive, name: &str) -> Result<Option<String>, String> {
    let file = match zip.by_name(name) {
        Ok(f) => f,
        Err(zip::result::ZipError::FileNotFound) => return Ok(None),
        Err(e) => return Err(format!("读取 {name} 失败：{e}")),
    };
    if file.size() > MAX_ENTRY {
        return Err(format!("{name} 过大"));
    }
    let mut text = String::new();
    file.take(MAX_ENTRY)
        .read_to_string(&mut text)
        .map_err(|e| format!("读取 {name} 失败：{e}"))?;
    Ok(Some(text))
}

fn is(node: &Node, tag: &str) -> bool {
    node.is_element() && node.tag_name().name() == tag && node.tag_name().namespace() == Some(MAIN_NS)
}

/// `<si>` / `<is>` 里的文字：拼接所有 `<t>`，跳过注音 `<rPh>`。
fn rich_text(node: Node) -> String {
    node.descendants()
        .filter(|n| is(n, "t") && !n.ancestors().any(|a| is(&a, "rPh")))
        .filter_map(|n| n.text())
        .collect()
}

/// “BC12” → (列 54, 行 11)，均从 0 开始。
fn cell_ref(r: &str) -> Option<(usize, usize)> {
    let split = r.find(|c: char| c.is_ascii_digit())?;
    let (letters, digits) = r.split_at(split);
    if letters.is_empty() || letters.len() > 3 || !letters.chars().all(|c| c.is_ascii_uppercase()) {
        return None;
    }
    let col = letters.bytes().try_fold(0usize, |acc, b| {
        acc.checked_mul(26)?.checked_add((b - b'A' + 1) as usize)
    })? - 1;
    let row = digits.parse::<usize>().ok()?.checked_sub(1)?;
    (col < 16_384 && row < 1_048_576).then_some((col, row))
}

/// 导入的数值写成文字：取 15 位有效数字（去掉 0.1 + 0.2 这类二进制尾差，不丢精度），
/// 很大或很小时用科学计数。
pub(crate) fn excel_number(v: f64) -> String {
    let r = crate::model::round_sig(v, 15);
    let a = r.abs();
    if a != 0.0 && !(1e-6..1e15).contains(&a) {
        format!("{r:e}")
    } else {
        format!("{r}")
    }
}

/// 数字格式是否为日期或时间：内置编号 14–22、27–36、45–47、50–58（含中文日期格式），
/// 或自定义格式里（去掉引号文字、转义字符和颜色等方括号后）出现 y、m、d、h、s。
pub(crate) fn is_date_format(id: u16, code: Option<&str>) -> bool {
    if matches!(id, 14..=22 | 27..=36 | 45..=47 | 50..=58) {
        return true;
    }
    let Some(code) = code else { return false };
    let mut plain = String::new();
    let mut chars = code.chars();
    while let Some(c) = chars.next() {
        match c {
            '"' => {
                for d in chars.by_ref() {
                    if d == '"' {
                        break;
                    }
                }
            }
            '\\' | '_' | '*' => {
                chars.next();
            }
            '[' => {
                let inner: String = chars.by_ref().take_while(|&d| d != ']').collect();
                // [h]、[mm]、[ss] 是累计时间
                if inner.chars().all(|d| matches!(d.to_ascii_lowercase(), 'h' | 'm' | 's')) && !inner.is_empty() {
                    plain.push('h');
                }
            }
            _ => plain.push(c.to_ascii_lowercase()),
        }
    }
    plain != "general" && plain.chars().any(|c| matches!(c, 'y' | 'm' | 'd' | 'h' | 's'))
}

/// Excel 日期序列号 → 文字（按秒取整）：整天写 `2026-10-18`，整分钟写 `2026-10-18 14:30`，否则带秒。
/// 1900 日期制以 1899-12-30 为 0（与 Excel 自 1900-03-01 起一致），1904 日期制以 1904-01-01 为 0。
pub(crate) fn excel_date_text(serial: f64, date1904: bool) -> Option<String> {
    if !serial.is_finite() || serial < 0.0 || serial > 2_958_466.0 {
        return None;
    }
    let base = if date1904 {
        chrono::NaiveDate::from_ymd_opt(1904, 1, 1)?
    } else {
        chrono::NaiveDate::from_ymd_opt(1899, 12, 30)?
    };
    let secs = (serial * 86_400.0).round() as i64;
    let t = base.and_hms_opt(0, 0, 0)? + chrono::Duration::seconds(secs);
    let pattern = if secs % 86_400 == 0 {
        "%Y-%m-%d"
    } else if secs % 60 == 0 {
        "%Y-%m-%d %H:%M"
    } else {
        "%Y-%m-%d %H:%M:%S"
    };
    Some(t.format(pattern).to_string())
}

/// 合并单元格：每个区域 [首行, 末行, 首列, 末列] 用左上角的值填满（只在已有的表格范围内）。
pub(crate) fn fill_merged(rows: &mut [Vec<String>], ranges: &[[usize; 4]]) {
    for &[r0, r1, c0, c1] in ranges {
        let Some(value) = rows.get(r0).and_then(|r| r.get(c0)).cloned() else {
            continue;
        };
        if value.is_empty() {
            continue;
        }
        for row in rows.iter_mut().take(r1 + 1).skip(r0) {
            for cell in row.iter_mut().take(c1 + 1).skip(c0) {
                *cell = value.clone();
            }
        }
    }
}

/// 读取工作簿里的全部工作表，顺序与 Excel 中的标签相同。旧版 .xls（OLE2 复合文档）交给 `xls::read`。
pub fn read(bytes: &[u8]) -> Result<Vec<Sheet>, String> {
    if bytes.starts_with(&[0xD0, 0xCF, 0x11, 0xE0]) {
        return crate::xls::read(bytes);
    }
    let mut zip = zip::ZipArchive::new(Cursor::new(bytes)).map_err(|e| format!("不是有效的 .xlsx 文件：{e}"))?;
    let workbook = entry(&mut zip, "xl/workbook.xml")?.ok_or("缺少 xl/workbook.xml，不是 Excel 工作簿")?;
    let rels = entry(&mut zip, "xl/_rels/workbook.xml.rels")?.unwrap_or_default();
    let shared: Vec<String> = match entry(&mut zip, "xl/sharedStrings.xml")? {
        Some(text) => {
            let doc = Document::parse(&text).map_err(|e| format!("共享字符串格式错误：{e}"))?;
            doc.root_element()
                .children()
                .filter(|n| is(n, "si"))
                .map(rich_text)
                .collect()
        }
        None => vec![],
    };
    let targets: Vec<(String, String)> = if rels.is_empty() {
        vec![]
    } else {
        let doc = Document::parse(&rels).map_err(|e| format!("工作簿关系格式错误：{e}"))?;
        doc.root_element()
            .children()
            .filter(|n| n.is_element() && n.tag_name().name() == "Relationship")
            .filter_map(|n| Some((n.attribute("Id")?.to_owned(), n.attribute("Target")?.to_owned())))
            .collect()
    };
    // 样式：每个单元格格式（xf）对应的数字格式是否为日期
    let date_xf: Vec<bool> = match entry(&mut zip, "xl/styles.xml")? {
        Some(text) => {
            let doc = Document::parse(&text).map_err(|e| format!("样式格式错误：{e}"))?;
            let custom: std::collections::HashMap<u16, String> = doc
                .descendants()
                .filter(|n| is(n, "numFmt"))
                .filter_map(|n| {
                    Some((
                        n.attribute("numFmtId")?.parse().ok()?,
                        n.attribute("formatCode")?.to_owned(),
                    ))
                })
                .collect();
            doc.descendants()
                .find(|n| is(n, "cellXfs"))
                .map(|xfs| {
                    xfs.children()
                        .filter(|n| is(n, "xf"))
                        .map(|xf| {
                            let id: u16 = xf.attribute("numFmtId").and_then(|v| v.parse().ok()).unwrap_or(0);
                            is_date_format(id, custom.get(&id).map(String::as_str))
                        })
                        .collect()
                })
                .unwrap_or_default()
        }
        None => vec![],
    };
    let doc = Document::parse(&workbook).map_err(|e| format!("工作簿格式错误：{e}"))?;
    let date1904 = doc
        .descendants()
        .find(|n| is(n, "workbookPr"))
        .and_then(|n| n.attribute("date1904"))
        .is_some_and(|v| v == "1" || v == "true");
    let mut sheets = vec![];
    for (i, node) in doc.descendants().filter(|n| is(n, "sheet")).enumerate() {
        let name = node.attribute("name").unwrap_or("Sheet").to_owned();
        let target = node
            .attribute((REL_NS, "id"))
            .and_then(|id| targets.iter().find(|(k, _)| k == id))
            .map(|(_, t)| t.clone())
            .unwrap_or_else(|| format!("worksheets/sheet{}.xml", i + 1));
        let path = match target.strip_prefix('/') {
            Some(abs) => abs.to_owned(),
            None => format!("xl/{target}"),
        };
        let xml = entry(&mut zip, &path)?.ok_or_else(|| format!("缺少工作表 {name}（{path}）"))?;
        sheets.push(Sheet {
            name,
            rows: parse_sheet(&xml, &shared, &date_xf, date1904)?,
        });
    }
    if sheets.is_empty() {
        return Err("工作簿里没有工作表".into());
    }
    Ok(sheets)
}

fn parse_sheet(xml: &str, shared: &[String], date_xf: &[bool], date1904: bool) -> Result<Vec<Vec<String>>, String> {
    let doc = Document::parse(xml).map_err(|e| format!("工作表格式错误：{e}"))?;
    let mut cells: Vec<(usize, usize, String)> = vec![];
    let mut next_row = 0;
    for row in doc.descendants().filter(|n| is(n, "row")) {
        let r = match row.attribute("r") {
            None => next_row,
            Some(text) => text
                .parse::<usize>()
                .ok()
                .and_then(|v| v.checked_sub(1))
                .ok_or("工作表行号无效")?,
        };
        if r >= 1_048_576 {
            return Err("工作表行号超出 Excel 支持范围".into());
        }
        next_row = r + 1;
        let mut next_col = 0;
        for c in row.children().filter(|n| is(n, "c")) {
            let (col, row_index) = match c.attribute("r") {
                Some(reference) => {
                    cell_ref(reference).ok_or_else(|| format!("单元格坐标 {reference} 无效或超出 Excel 范围"))?
                }
                None => (next_col, r),
            };
            if col >= 16_384 {
                return Err("工作表列号超出 Excel 支持范围".into());
            }
            next_col = col + 1;
            let v = || c.children().find(|n| is(n, "v")).and_then(|n| n.text()).unwrap_or("");
            let text = match c.attribute("t").unwrap_or("n") {
                "s" => v()
                    .trim()
                    .parse::<usize>()
                    .ok()
                    .and_then(|i| shared.get(i).cloned())
                    .unwrap_or_default(),
                "inlineStr" => c.children().find(|n| is(n, "is")).map(rich_text).unwrap_or_default(),
                "b" => if v().trim() == "1" { "TRUE" } else { "FALSE" }.to_owned(),
                "str" | "e" => v().to_owned(),
                _ => {
                    let raw = v().trim();
                    let x = parse_num(raw);
                    let is_date = c
                        .attribute("s")
                        .and_then(|s| s.parse::<usize>().ok())
                        .and_then(|s| date_xf.get(s))
                        .copied()
                        .unwrap_or(false);
                    if x.is_finite() && is_date && !raw.is_empty() {
                        excel_date_text(x, date1904).unwrap_or_else(|| excel_number(x))
                    } else if x.is_finite() {
                        excel_number(x)
                    } else {
                        raw.to_owned()
                    }
                }
            };
            if !text.is_empty() {
                cells.push((row_index, col, text));
            }
        }
    }
    let nrows = cells.iter().map(|c| c.0 + 1).max().unwrap_or(0);
    let ncols = cells.iter().map(|c| c.1 + 1).max().unwrap_or(0);
    check_sheet_size(nrows, ncols)?;
    let mut rows = vec![vec![String::new(); ncols]; nrows];
    for (r, c, text) in cells {
        rows[r][c] = text;
    }
    let merged: Vec<[usize; 4]> = doc
        .descendants()
        .filter(|n| is(n, "mergeCell"))
        .filter_map(|n| {
            let (a, b) = n.attribute("ref")?.split_once(':')?;
            let ((c0, r0), (c1, r1)) = (cell_ref(a)?, cell_ref(b)?);
            Some([r0, r1, c0, c1])
        })
        .collect();
    fill_merged(&mut rows, &merged);
    Ok(rows)
}

pub(crate) fn check_sheet_size(rows: usize, columns: usize) -> Result<(), String> {
    // 每格 String 至少 24 字节，将稠密导入限制在约 240 MB 的空格开销内。
    if rows.saturating_mul(columns) > 10_000_000 {
        return Err(format!(
            "工作表范围过大（{rows} 行 × {columns} 列），请删除远处单元格或拆分数据后导入"
        ));
    }
    Ok(())
}

#[cfg(test)]
mod review_limits_tests {
    use super::*;
    #[test]
    fn sparse_far_cells_and_overflowed_coordinates_are_rejected() {
        assert!(cell_ref("AAAAAAAAAAAAAAAAAAAA1").is_none());
        assert!(cell_ref("XFE1").is_none());
        assert!(cell_ref("A1048577").is_none());
        assert_eq!(cell_ref("XFD1048576"), Some((16383, 1048575)));
        let xml = r#"<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData><row r="1"><c r="A1"><v>1</v></c></row><row r="1048576"><c r="XFD1048576"><v>2</v></c></row></sheetData></worksheet>"#;
        assert!(parse_sheet(xml, &[], &[], false).unwrap_err().contains("范围过大"));
        let invalid = r#"<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData><row r="18446744073709551615"><c><v>1</v></c></row></sheetData></worksheet>"#;
        assert!(parse_sheet(invalid, &[], &[], false).is_err());
    }
}
