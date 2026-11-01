//! XLSX export keeps each workbook in its own sheet and preserves numeric and date cells.
use crate::{DataKind, Project};
use chrono::{Datelike, Timelike};
use rust_xlsxwriter::{ExcelDateTime, Format, Workbook};
use std::collections::HashSet;

fn within_utf16(text: &str, limit: usize) -> String {
    let mut result = String::new();
    let mut used = 0;
    for c in text.chars() {
        if used + c.len_utf16() > limit {
            break;
        }
        used += c.len_utf16();
        result.push(c);
    }
    result
}

fn sheet_name(name: &str, used: &mut HashSet<String>) -> String {
    let sanitized: String = name
        .chars()
        .map(|c| {
            if matches!(c, ':' | '\\' | '/' | '?' | '*' | '[' | ']') {
                '_'
            } else {
                c
            }
        })
        .collect();
    let base = sanitized.trim_matches('\'');
    let base = if base.is_empty() { "Sheet" } else { base };
    let mut number = 1;
    loop {
        let suffix = if number == 1 {
            String::new()
        } else {
            format!(" ({number})")
        };
        let mut name = within_utf16(base, 31 - suffix.len())
            .trim_matches(char::from(39))
            .to_string();
        if name.is_empty() {
            name.push_str("Sheet");
        }
        name.push_str(&suffix);
        if used.insert(name.to_lowercase()) {
            return name;
        }
        number += 1;
    }
}

pub fn write_project(project: &Project, headers: bool) -> Result<Vec<u8>, String> {
    if project.books.is_empty() {
        return Err("项目没有可导出的工作簿".into());
    }
    let mut workbook = Workbook::new();
    let date_format = Format::new().set_num_format("yyyy-mm-dd hh:mm:ss.000");
    let mut names = HashSet::new();
    for book in &project.books {
        let header_rows = if headers { 3 } else { 0 };
        if book.nrows() > 1_048_576 - header_rows || book.columns.len() > 16_384 {
            return Err(format!("工作簿 {} 超出 Excel 的行数或列数限制", book.name));
        }
        let sheet = workbook.add_worksheet();
        sheet
            .set_name(sheet_name(&book.name, &mut names))
            .map_err(|e| e.to_string())?;
        for (index, column) in book.columns.iter().enumerate() {
            let col: u16 = index.try_into().map_err(|_| "工作表的列数超出 Excel 限制")?;
            if headers {
                for (row, text) in [&column.long_name, &column.units, &column.comment].iter().enumerate() {
                    if !text.is_empty() {
                        sheet
                            .write_string(row as u32, col, text.as_str())
                            .map_err(|e| e.to_string())?;
                    }
                }
            }
            for index in 0..column.data.len() {
                let row: u32 = (index + header_rows)
                    .try_into()
                    .map_err(|_| "工作表的行数超出 Excel 限制")?;
                match column.data.kind() {
                    DataKind::Num => {
                        if let Some(value) = column.data.f64_at(index) {
                            if value.is_finite() {
                                sheet.write_number(row, col, value).map_err(|e| e.to_string())?;
                            } else {
                                sheet
                                    .write_string(row, col, column.data.text_at(index).as_ref())
                                    .map_err(|e| e.to_string())?;
                            }
                        }
                    }
                    DataKind::DateTime => {
                        if let Some(value) = column.data.f64_at(index) {
                            let local = (value as i64)
                                .checked_add(i64::from(column.format.utc_offset) * 60_000)
                                .ok_or("日期超出支持范围")?;
                            let date = chrono::DateTime::from_timestamp_millis(local).ok_or("日期超出支持范围")?;
                            let year: u16 = date.year().try_into().map_err(|_| "日期超出 Excel 支持范围")?;
                            let date = ExcelDateTime::from_ymd(year, date.month() as u8, date.day() as u8)
                                .and_then(|v| {
                                    v.and_hms_milli(
                                        date.hour() as u16,
                                        date.minute() as u8,
                                        date.second() as u8,
                                        date.timestamp_subsec_millis() as u16,
                                    )
                                })
                                .map_err(|e| format!("日期超出 Excel 支持范围：{e}"))?;
                            sheet
                                .write_datetime_with_format(row, col, &date, &date_format)
                                .map_err(|e| e.to_string())?;
                        }
                    }
                    DataKind::Text | DataKind::Category => {
                        let text = column.data.text_at(index);
                        if !text.is_empty() {
                            sheet.write_string(row, col, text.as_ref()).map_err(|e| e.to_string())?;
                        }
                    }
                }
            }
        }
    }
    workbook.save_to_buffer().map_err(|e| e.to_string())
}
