//! Descriptive tables and the three-panel summary reuse existing statistics and plots.
use super::*;
pub fn descriptive_table(columns: &[(&str, &[f64])]) -> Vec<Vec<String>> {
    let descriptions: Vec<_> = columns.iter().map(|(_, v)| analysis::describe(v)).collect();
    let mut rows = vec![std::iter::once("统计量".into())
        .chain(columns.iter().map(|(name, _)| (*name).to_owned()))
        .collect()];
    for (index, label) in [
        "N",
        "均值",
        "标准差",
        "SE",
        "最小",
        "Q1",
        "中位数",
        "Q3",
        "最大",
        "缺失数",
    ]
    .into_iter()
    .enumerate()
    {
        let mut row = vec![label.into()];
        for ((_, values), d) in columns.iter().zip(&descriptions) {
            let n = d.as_ref().map_or(0, |d| d.n);
            let value = if index == 0 {
                n as f64
            } else if index == 9 {
                (values.len() - n) as f64
            } else {
                d.as_ref().map_or(f64::NAN, |d| match index {
                    1 => d.mean,
                    2 => d.sd,
                    3 => d.se,
                    4 => d.min,
                    5 => d.q1,
                    6 => d.median,
                    7 => d.q3,
                    _ => d.max,
                })
            };
            row.push(if value.is_finite() {
                value.to_string()
            } else {
                String::new()
            });
        }
        rows.push(row);
    }
    rows
}
pub fn graphical_summary_text(values: &[f64]) -> Result<String, String> {
    let d = analysis::describe(values).ok_or("图形摘要至少需要一个有效数值")?;
    let sw = crate::stats::shapiro_wilk(values)
        .map(|r| format!("p = {}", fmt_num(r.p)))
        .unwrap_or_else(|e| e);
    let ad = crate::stats::anderson_normal(values)
        .map(|r| format!("p = {}", fmt_num(r.p)))
        .unwrap_or_else(|e| e);
    Ok(format!("描述统计\nN = {}，缺失 = {}\n均值 = {}\nSD = {}，SE = {}\n最小 = {}，最大 = {}\nQ1 = {}，中位数 = {}\nQ3 = {}，总和 = {}\nShapiro–Wilk {}\nAnderson–Darling {}",d.n,values.len()-d.n,fmt_num(d.mean),fmt_num(d.sd),fmt_num(d.se),fmt_num(d.min),fmt_num(d.max),fmt_num(d.q1),fmt_num(d.median),fmt_num(d.q3),fmt_num(d.sum),sw,ad))
}
impl Project {
    pub fn whole_table_statistics(
        &self,
        book_id: u64,
        keys: &[String],
        rows: Option<(usize, usize)>,
    ) -> Result<Vec<Vec<String>>, String> {
        let book = self.book(book_id).ok_or("找不到工作簿")?;
        let columns: Vec<_> = book
            .columns
            .iter()
            .filter(|c| (keys.is_empty() || keys.contains(&c.key)) && c.data.kind() == crate::DataKind::Num)
            .collect();
        if columns.is_empty() {
            return Err("所选范围没有数值列".into());
        }
        let (first, end) = rows.map_or((0, book.nrows()), |(a, b)| {
            (a.min(book.nrows()), b.saturating_add(1).min(book.nrows()))
        });
        let mut values = Vec::new();
        for column in columns {
            let data = column.values();
            values.extend(
                (first..end)
                    .filter(|row| !book.uses_visible_rows() || book.row_visible(*row))
                    .map(|row| data.get(row).copied().unwrap_or(f64::NAN)),
            );
        }
        let mut table = descriptive_table(&[("整表", &values)]);
        table.push(vec![
            "总和".into(),
            values.iter().filter(|v| v.is_finite()).sum::<f64>().to_string(),
        ]);
        Ok(table)
    }
    pub fn write_statistics_table(&mut self, title: &str, rows: &[Vec<String>]) -> Result<u64, String> {
        let headers = rows.first().filter(|r| !r.is_empty()).ok_or("结果表需要表头")?;
        let id = self.alloc_id();
        let mut book = Book::new(id, self.unique_name(title));
        for (i, name) in headers.iter().enumerate() {
            let key = book.push_column(if i == 0 { Role::X } else { Role::Y });
            let col = book.columns.iter_mut().find(|c| c.key == key).unwrap();
            col.long_name = name.clone();
            col.comment = title.into();
            col.data = rows[1..]
                .iter()
                .map(|row| row.get(i).cloned().unwrap_or_default())
                .collect();
        }
        self.books.push(book);
        Ok(id)
    }
    pub fn compare_statistics(&mut self, src: &Source, boxes: bool) -> Result<(u64, Option<u64>), String> {
        let columns = self.analysis_columns(src)?;
        if columns.len() < 2 {
            return Err("并排比较请至少选择两列".into());
        }
        let input: Vec<_> = columns.iter().map(|c| (c.name.as_str(), c.values.as_slice())).collect();
        let rows = descriptive_table(&input);
        let graph = if boxes {
            Some(self.statistics_box_plot(&columns)?)
        } else {
            None
        };
        let book = self.write_statistics_table("并排统计比较", &rows)?;
        self.move_project_item(book, self.folder_of(columns[0].source.book))?;
        Ok((book, graph))
    }
    fn statistics_box_plot(&mut self, columns: &[AnalysisColumn]) -> Result<u64, String> {
        let main = columns[0].source.book;
        let mut spec = crate::plotspec::PlotSpec {
            book_id: Some(main),
            plot_type: PlotType::Box,
            visible_only: columns.iter().any(|c| c.visible_only),
            ..Default::default()
        };
        for column in columns {
            let key = column.source.key(Some(main));
            spec.y_keys.push(key.clone());
            if let Some(rows) = column.rows {
                spec.row_by_y.insert(key, Some(rows));
            }
        }
        self.plot_spec(spec)
    }
    pub fn graphical_summary(&mut self, column: &AnalysisColumn) -> Result<(u64, u64), String> {
        let text = graphical_summary_text(&column.values)?;
        let spec = crate::plotspec::PlotSpec {
            book_id: Some(column.source.book),
            y_keys: vec![column.source.col.clone()],
            rows: column.rows,
            visible_only: column.visible_only,
            plot_type: PlotType::Line,
            ..Default::default()
        };
        let id = self.plot_spec(spec)?;
        let name = self.unique_name("图形摘要");
        self.rename_project_item(id, &name)?;
        let graph = self.graph_mut(id).unwrap();
        let base = graph.first_layer().clone();
        graph.page.add_layer(base.clone());
        graph.page.add_layer(base);
        graph.page.columns = 2;
        (graph.page.width_mm, graph.page.height_mm) = crate::Panels::Grid4.page_mm();
        for (layer, (kind, title)) in graph.page.layers_mut().iter_mut().zip([
            (PlotType::Histogram, "直方图"),
            (PlotType::Box, "箱线图"),
            (PlotType::QQ, "正态 Q-Q"),
        ]) {
            layer.set_plot_type(kind)?;
            layer.legend = false;
            layer.title = title.into();
        }
        let annotation = graph.page.add_text([0.54, 0.54])?;
        let annotation = graph.page.text_mut(annotation).unwrap();
        annotation.text = text.clone();
        annotation.rich = false;
        annotation.size_pt = 6.;
        let note = self.add_note(
            "图形摘要报告",
            format!(
                "# {} 图形摘要\n\n{text}\n\n标准差使用 n−1，分位数使用线性插值。统计量为生成时快照；源数据修改后重新生成本报告。",
                column.name
            ),
        );
        self.move_project_item(note, self.folder_of(column.source.book))?;
        Ok((id, note))
    }
}
