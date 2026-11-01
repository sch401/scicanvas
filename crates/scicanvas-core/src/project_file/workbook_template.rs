//! Standalone workbook templates store column definitions without worksheet data.
use super::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkbookTemplate {
    pub version: u32,
    pub name: String,
    pub columns: Vec<crate::Column>,
}

pub fn workbook_template_from_book(book: &Book) -> WorkbookTemplate {
    let columns = book
        .columns
        .iter()
        .cloned()
        .map(|mut column| {
            column.data = crate::ColumnData::Text(Vec::new())
                .converted(column.data.kind())
                .unwrap()
                .into();
            column.derived_from = None;
            column.op = None;
            column.fit_report = None;
            column.independent_rows = false;
            column
        })
        .collect();
    WorkbookTemplate {
        version: 1,
        name: book.name.clone(),
        columns,
    }
}

fn validate_workbook_template(template: &WorkbookTemplate) -> Result<(), String> {
    if template.version != 1 {
        return Err(format!("不支持的工作簿模板版本 {}", template.version));
    }
    if template.columns.is_empty() {
        return Err("工作簿模板没有列".into());
    }
    let mut keys = BTreeSet::new();
    for column in &template.columns {
        if column.key.is_empty() || !keys.insert(&column.key) {
            return Err("工作簿模板的列标识无效或重复".into());
        }
        if !column.data.is_empty() || column.op.is_some() || column.fit_report.is_some() {
            return Err("工作簿模板不能包含数据或分析结果".into());
        }
    }
    Ok(())
}

pub fn workbook_template_to_json(template: &WorkbookTemplate) -> Result<String, String> {
    validate_workbook_template(template)?;
    serde_json::to_string_pretty(template).map_err(|e| e.to_string())
}

pub fn workbook_template_from_json(text: &str) -> Result<WorkbookTemplate, String> {
    let template =
        serde_json::from_str(text.trim_start_matches('\u{feff}')).map_err(|e| format!("工作簿模板格式错误：{e}"))?;
    validate_workbook_template(&template)?;
    Ok(template)
}

pub fn save_workbook_template(path: &Path, template: &WorkbookTemplate) -> Result<(), String> {
    let content = workbook_template_to_json(template)?;
    write_atomic(path, content.as_bytes(), "模板文件名为空", "保存工作簿模板失败：")
}

pub fn load_workbook_template(path: &Path) -> Result<WorkbookTemplate, String> {
    let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let mut content = Vec::new();
    file.take(MAX_GRAPH_TEMPLATE_BYTES as u64 + 1)
        .read_to_end(&mut content)
        .map_err(|e| e.to_string())?;
    if content.len() > MAX_GRAPH_TEMPLATE_BYTES {
        return Err("工作簿模板文件过大".into());
    }
    let text = String::from_utf8(content).map_err(|_| "工作簿模板不是 UTF-8 JSON")?;
    workbook_template_from_json(&text)
}

pub fn create_book_from_template(project: &mut Project, template: &WorkbookTemplate) -> Result<u64, String> {
    validate_workbook_template(template)?;
    let id = project.new_book();
    let name = project.unique_name(&template.name);
    let book = project.book_mut(id).unwrap();
    book.name = name;
    book.columns.clone_from(&template.columns);
    Ok(id)
}
