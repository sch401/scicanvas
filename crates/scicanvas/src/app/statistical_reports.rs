use super::*;
impl SciApp {
    pub(crate) fn whole_table_statistics(&mut self) {
        let Some(book) = self.active_book_id() else { return };
        let (keys, rows) = self.picked(book);
        match self.project.whole_table_statistics(book, &keys, rows) {
            Ok(mut rows) => {
                let mut method = vec![String::new(); rows.first().map_or(2, Vec::len)];
                method[0] = "说明".into();
                method[1] = "样本标准差使用 n−1，分位数按线性插值，缺失和非有限值不计入 N。".into();
                rows.push(method);
                self.status = "已汇总所选数值单元格".into();
                self.dialog = Some(Dialog::Table {
                    title: "整表统计".into(),
                    rows,
                    note: "未选区时取整表。".into(),
                });
            }
            Err(error) => self.status = error,
        }
    }
    pub(crate) fn open_statistics_compare(&mut self) {
        let Some(source) = self.analysis_source() else { return };
        match self.project.analysis_columns(&source) {
            Ok(columns) if columns.len() >= 2 => {
                self.dialog = Some(Dialog::StatisticsCompare {
                    source,
                    names: columns.into_iter().map(|c| c.name).collect(),
                    boxes: false,
                })
            }
            Ok(_) => self.status = "并排比较请点选至少两列".into(),
            Err(error) => self.status = error,
        }
    }
    pub(crate) fn graphical_summary(&mut self) {
        let Some(columns) = self.statistical_columns() else {
            return;
        };
        if columns.len() != 1 {
            self.status = "图形摘要请选择一列数据".into();
            return;
        }
        let column = &columns[0];
        self.edit_with(
            "图形摘要",
            |p| p.graphical_summary(column).map(Some),
            |app, (graph, _)| {
                app.show_item(graph);
                app.status = "已生成三幅分布图、统计注释和 Note".into();
                app.dialog = None;
            },
        );
    }
    pub(super) fn apply_statistics_action(&mut self, action: dialogs::StatisticsAction) {
        match action {
            dialogs::StatisticsAction::Compare(source, boxes) => {
                self.edit_with(
                    "并排统计比较",
                    |p| p.compare_statistics(&source, boxes).map(Some),
                    |app, (book, graph)| {
                        app.show_item(book);
                        if let Some(graph) = graph {
                            app.show_item(graph);
                        }
                        app.dialog = None;
                        app.status = "并排统计表已生成".into();
                    },
                );
            }
            dialogs::StatisticsAction::WriteTable(title, rows) => {
                let folder = self.active.and_then(|id| self.project.folder_of(id));
                self.edit_with(
                    "统计写入新表",
                    |p| {
                        let id = p.write_statistics_table(&title, &rows)?;
                        p.move_project_item(id, folder)?;
                        Ok(Some(id))
                    },
                    |app, id| {
                        app.show_item(id);
                        app.status = "统计结果已写入新表".into();
                    },
                );
            }
        }
    }
}
