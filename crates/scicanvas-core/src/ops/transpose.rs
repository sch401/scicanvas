//! Transpose a worksheet into a separate workbook.
use super::*;

impl Project {
    pub fn transpose_book(&mut self, source: u64) -> Result<u64, String> {
        let book = self.book(source).ok_or("找不到工作簿")?;
        let rows = book.nrows();
        if book.columns.is_empty() || rows == 0 {
            return Err("工作簿没有可转置的数据".into());
        }
        let mut output = Book::new(0, self.unique_name("Transposed"));
        output.push_column(Role::X);
        output.columns[0].long_name = "原列名".into();
        output.columns[0].data = crate::ColumnData::Text(
            book.columns
                .iter()
                .map(|column| column.display_name().to_owned())
                .collect(),
        )
        .into();
        for row in 0..rows {
            output.push_column(Role::Y);
            let column = output.columns.last_mut().unwrap();
            column.long_name = format!("原第 {} 行", row + 1);
            for source in &book.columns {
                column.data.push(source.data.text_at(row).into_owned());
            }
        }
        let parent = self.folder_of(source);
        output.id = self.alloc_id();
        let id = output.id;
        self.books.push(output);
        self.move_project_item(id, parent)?;
        Ok(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transpose_preserves_values_names_missing_cells_and_folder() {
        let mut project = Project::default();
        let id = project.new_book();
        let folder = project.add_folder(None).unwrap();
        project.move_project_item(id, Some(folder)).unwrap();
        let book = project.book_mut(id).unwrap();
        book.columns[0].long_name = "时间".into();
        book.columns[0].data = ["1", "2", ""].map(str::to_string).to_vec().into();
        book.columns[1].long_name = "标签".into();
        book.columns[1].data = ["甲", "", "丙"].map(str::to_string).to_vec().into();
        let source = book.clone();
        let output = project.transpose_book(id).unwrap();
        assert_eq!(project.book(id).unwrap(), &source);
        assert_eq!(project.folder_of(output), Some(folder));
        let transposed = project.book(output).unwrap();
        assert_eq!(transposed.columns.len(), 4);
        assert_eq!(transposed.nrows(), 2);
        assert_eq!(transposed.columns[0].data.kind(), crate::DataKind::Text);
        assert_eq!(transposed.columns[0].data.text_at(0), "时间");
        assert_eq!(transposed.columns[0].data.text_at(1), "标签");
        for (column, want) in transposed.columns[1..].iter().zip([["1", "甲"], ["2", ""], ["", "丙"]]) {
            assert_eq!(column.data.text_at(0), want[0]);
            assert_eq!(column.data.text_at(1), want[1]);
        }
        let before = project.clone();
        let empty = project.new_book();
        let empty_before = project.clone();
        assert!(project.transpose_book(empty).is_err());
        assert_eq!(project, empty_before);
        assert!(before.book(output).is_some());
    }
}
