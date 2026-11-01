//! Worksheet stack, split and row reduction operations.
use super::*;
use crate::{ColumnData, DataKind};

impl Project {
    pub fn stack_columns(&mut self, id: u64, keys: &[String]) -> Result<Vec<String>, String> {
        let book = self.book_mut(id).ok_or("找不到工作簿")?;
        if keys.len() < 2 {
            return Err("请选择至少两列".into());
        }
        let columns = keys
            .iter()
            .map(|key| book.column_index(key).map(|i| &book.columns[i]).ok_or("找不到选择的列"))
            .collect::<Result<Vec<_>, _>>()?;
        let kind = columns.first().map(|column| column.data.kind()).unwrap();
        let kind = if columns.iter().all(|column| column.data.kind() == kind) {
            kind
        } else {
            DataKind::Text
        };
        let mut values = Vec::new();
        let mut sources = Vec::new();
        for column in columns {
            for row in 0..column.data.len() {
                values.push(column.data.text_at(row).into_owned());
                sources.push(column.key.clone());
            }
        }
        let values = ColumnData::Text(values).converted(kind)?;
        let category = ColumnData::Text(sources).converted(DataKind::Category)?;
        let value_key = book.push_column(Role::Y);
        let value = book.columns.last_mut().unwrap();
        value.long_name = "堆叠值".into();
        value.data = values.into();
        value.independent_rows = true;
        let source_key = book.push_column(Role::Label);
        let source = book.columns.last_mut().unwrap();
        source.long_name = "来源列".into();
        source.data = category.into();
        source.independent_rows = true;
        Ok(vec![value_key, source_key])
    }

    pub fn split_column(&mut self, id: u64, value_key: &str, category_key: &str) -> Result<Vec<String>, String> {
        let book = self.book_mut(id).ok_or("找不到工作簿")?;
        let value = &book.columns[book.column_index(value_key).ok_or("找不到值列")?];
        let category = &book.columns[book.column_index(category_key).ok_or("找不到分类列")?];
        if category.data.kind() != DataKind::Category {
            return Err("请选择分类列".into());
        }
        let kind = value.data.kind();
        let mut groups: Vec<(String, Vec<String>)> = Vec::new();
        let mut indexes = std::collections::HashMap::new();
        for row in 0..category.data.len() {
            let name = category.data.text_at(row);
            if name.is_empty() {
                continue;
            }
            let index = *indexes.entry(name.to_string()).or_insert_with(|| {
                groups.push((name.into_owned(), Vec::new()));
                groups.len() - 1
            });
            groups[index].1.push(value.data.text_at(row).into_owned());
        }
        if groups.is_empty() {
            return Err("分类列没有数据".into());
        }
        let groups = groups
            .into_iter()
            .map(|(name, cells)| Ok((name, ColumnData::Text(cells).converted(kind)?)))
            .collect::<Result<Vec<_>, String>>()?;
        let mut result = Vec::new();
        for (name, data) in groups {
            result.push(book.push_column(Role::Y));
            let column = book.columns.last_mut().unwrap();
            column.long_name = name;
            column.data = data.into();
            column.independent_rows = true;
        }
        Ok(result)
    }

    pub fn reduce_column(&mut self, id: u64, key: &str, step: usize, average: bool) -> Result<String, String> {
        if step == 0 {
            return Err("行数间隔必须大于零".into());
        }
        let book = self.book_mut(id).ok_or("找不到工作簿")?;
        let source = &book.columns[book.column_index(key).ok_or("找不到源列")?];
        if average && source.data.kind() != DataKind::Num {
            return Err("求平均需要数值列".into());
        }
        let kind = source.data.kind();
        let mut cells = Vec::new();
        for start in (0..source.data.len()).step_by(step) {
            if average {
                let numbers: Vec<_> = (start..start.saturating_add(step).min(source.data.len()))
                    .filter_map(|row| source.data.f64_at(row))
                    .filter(|value| value.is_finite())
                    .collect();
                cells.push(if numbers.is_empty() {
                    String::new()
                } else {
                    (numbers.iter().sum::<f64>() / numbers.len() as f64).to_string()
                });
            } else {
                cells.push(source.data.text_at(start).into_owned());
            }
        }
        let data = ColumnData::Text(cells).converted(kind)?;
        let output = book.push_column(Role::Y);
        let column = book.columns.last_mut().unwrap();
        column.long_name = if average { "区间平均" } else { "按行抽样" }.into();
        column.data = data.into();
        column.independent_rows = true;
        Ok(output)
    }
}
