//! 读取 Excel 工作簿（.xlsx / .xlsm）：ZIP 容器里的 workbook、关系、共享字符串和工作表 XML。
//! 只取单元格的值（公式取缓存结果），数字整理成工作表的显示格式；不读样式，日期保持序列号。

use std::io::{Cursor, Read};

use roxmltree::{Document, Node};

use crate::model::{fmt_num, parse_num};

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
    if letters.is_empty() || !letters.chars().all(|c| c.is_ascii_uppercase()) {
        return None;
    }
    let col = letters
        .bytes()
        .fold(0usize, |acc, b| acc * 26 + (b - b'A' + 1) as usize)
        - 1;
    let row = digits.parse::<usize>().ok()?.checked_sub(1)?;
    Some((col, row))
}

/// 读取工作簿里的全部工作表，顺序与 Excel 中的标签相同。
pub fn read(bytes: &[u8]) -> Result<Vec<Sheet>, String> {
    if bytes.starts_with(&[0xD0, 0xCF, 0x11, 0xE0]) {
        return Err("这是旧版 .xls 格式，请在 Excel 中另存为 .xlsx 后再导入".into());
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
    let doc = Document::parse(&workbook).map_err(|e| format!("工作簿格式错误：{e}"))?;
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
            rows: parse_sheet(&xml, &shared)?,
        });
    }
    if sheets.is_empty() {
        return Err("工作簿里没有工作表".into());
    }
    Ok(sheets)
}

fn parse_sheet(xml: &str, shared: &[String]) -> Result<Vec<Vec<String>>, String> {
    let doc = Document::parse(xml).map_err(|e| format!("工作表格式错误：{e}"))?;
    let mut cells: Vec<(usize, usize, String)> = vec![];
    let mut next_row = 0;
    for row in doc.descendants().filter(|n| is(n, "row")) {
        let r = row
            .attribute("r")
            .and_then(|v| v.parse::<usize>().ok())
            .and_then(|v| v.checked_sub(1))
            .unwrap_or(next_row);
        next_row = r + 1;
        let mut next_col = 0;
        for c in row.children().filter(|n| is(n, "c")) {
            let (col, row_index) = c.attribute("r").and_then(cell_ref).unwrap_or((next_col, r));
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
                    if x.is_finite() {
                        fmt_num(x)
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
    let mut rows = vec![vec![String::new(); ncols]; nrows];
    for (r, c, text) in cells {
        rows[r][c] = text;
    }
    Ok(rows)
}
