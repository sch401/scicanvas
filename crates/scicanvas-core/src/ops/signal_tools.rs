use super::*;
use crate::signal::{self, SignalOperation};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MergeDuplicates {
    Mean,
    KeepAll,
}
impl Project {
    pub fn signal_operation(&mut self, src: &Source, operation: SignalOperation) -> Result<(u64, Vec<String>), String> {
        let resolved = self.resolve(src)?;
        if resolved.items.len() != 2 {
            return Err("请选择信号和卷积核两条 Y 列".into());
        }
        let item = &resolved.items[0];
        let kernel = &resolved.items[1];
        let (_, a, _) = get_xy(self.book(item.book_id).unwrap(), item);
        let (_, b, _) = get_xy(self.book(kernel.book_id).unwrap(), kernel);
        if a.is_empty() || b.is_empty() {
            return Err("信号和卷积核需要有效数据".into());
        }
        let values = match operation {
            SignalOperation::Convolve(mode) => signal::convolve(&a, &b, mode),
            SignalOperation::Deconvolve { epsilon } => {
                if !epsilon.is_finite() || epsilon < 0. {
                    return Err("正则系数 ε 应为有限非负数".into());
                }
                signal::deconvolve(&a, &b, epsilon)
            }
        };
        if values.iter().any(|v| !v.is_finite()) {
            return Err("信号计算含未定义值；反卷积可提高 ε".into());
        }
        let mut x = Column::new("", Role::X);
        x.long_name = "结果样本序号".into();
        x.data = (0..values.len()).map(|i| i.to_string()).collect();
        let mut y = Column::new("", Role::Y);
        y.long_name = match operation {
            SignalOperation::Convolve(mode) => format!("离散卷积（{mode:?}）"),
            SignalOperation::Deconvolve { epsilon } => format!("反卷积，ε={epsilon}"),
        };
        y.units = self.book(item.book_id).unwrap().column(&item.y).unwrap().units.clone();
        y.comment = "离散信号处理，不乘采样间隔 dx；结果为一次性输出".into();
        y.data = values.iter().map(ToString::to_string).collect();
        let result = self.write_result(
            item,
            vec![x, y],
            ResultLink::Auto {
                resolved: &resolved,
                x: 0,
                y: 1,
            },
        )?;
        Ok((result.graph.unwrap(), result.columns))
    }
    pub fn interpolate_at(
        &mut self,
        src: &Source,
        method: InterpMethod,
        target: &DataRef,
    ) -> Result<Vec<String>, String> {
        let at = self
            .book(target.book)
            .and_then(|b| b.column(&target.col))
            .ok_or("找不到目标 X 列")?
            .values();
        let resolved = self.resolve(src)?;
        let mut columns = Vec::new();
        for item in &resolved.items {
            let book = self.book(item.book_id).unwrap();
            let (x, y, _) = get_xy(book, item);
            let finite: Vec<_> = at.iter().copied().filter(|x| x.is_finite()).collect();
            let values = interpolate::interpolate(&x, &y, method, &finite)?;
            let mut column = Column::new("", Role::Y);
            column.long_name = format!(
                "{} {}插值",
                book.column(&item.y).unwrap().display_name(),
                method.label()
            );
            column.units = book.column(&item.y).unwrap().units.clone();
            column.comment = format!("目标 X={}；按原顺序求值，缺失 X 留空", target.col);
            let mut values = values.into_iter();
            column.data = at
                .iter()
                .map(|x| {
                    if x.is_finite() {
                        values.next().unwrap().to_string()
                    } else {
                        String::new()
                    }
                })
                .collect();
            columns.push(column);
        }
        let item = Item {
            book_id: target.book,
            y: target.col.clone(),
            x: Some(target.col.clone()),
            rows: None,
            visible_only: false,
        };
        Ok(self.write_result(&item, columns, ResultLink::None)?.columns)
    }
    pub fn interpolate_value(&self, src: &Source, method: InterpMethod, at: f64) -> Result<Vec<(String, f64)>, String> {
        if !at.is_finite() {
            return Err("查询 X 应为有限数".into());
        }
        let resolved = self.resolve(src)?;
        resolved
            .items
            .iter()
            .map(|item| {
                let book = self.book(item.book_id).unwrap();
                let (x, y, _) = get_xy(book, item);
                Ok((
                    resolved.name(book, item),
                    interpolate::interpolate(&x, &y, method, &[at])?[0],
                ))
            })
            .collect()
    }
    pub fn merge_curves(&mut self, src: &Source, duplicate: MergeDuplicates) -> Result<(u64, u64), String> {
        let resolved = self.resolve(src)?;
        let mut points = Vec::new();
        for item in &resolved.items {
            let (x, y, _) = get_xy(self.book(item.book_id).unwrap(), item);
            points.extend(x.into_iter().zip(y));
        }
        if points.is_empty() {
            return Err("没有有效曲线点".into());
        }
        points.sort_by(|a, b| a.0.total_cmp(&b.0));
        let points = if duplicate == MergeDuplicates::Mean {
            let mut grouped = Vec::new();
            let mut i = 0;
            while i < points.len() {
                let mut end = i + 1;
                while end < points.len() && points[end].0 == points[i].0 {
                    end += 1
                }
                grouped.push((
                    points[i].0,
                    points[i..end].iter().map(|p| p.1).sum::<f64>() / (end - i) as f64,
                ));
                i = end;
            }
            grouped
        } else {
            points
        };
        let first = &resolved.items[0];
        let source = self.book(first.book_id).unwrap();
        let xu = first
            .x
            .as_ref()
            .and_then(|x| source.column(x))
            .map(|c| c.units.clone())
            .unwrap_or_default();
        let yu = source.column(&first.y).unwrap().units.clone();
        let folder = self.folder_of(first.book_id);
        let id = self.alloc_id();
        let mut book = Book::new(id, self.unique_name("曲线合并"));
        for (role, name, units, values) in [
            (Role::X, "X", xu, points.iter().map(|p| p.0).collect::<Vec<_>>()),
            (Role::Y, "合并 Y", yu, points.iter().map(|p| p.1).collect::<Vec<_>>()),
        ] {
            let key = book.push_column(role);
            let col = book.columns.iter_mut().find(|c| c.key == key).unwrap();
            col.long_name = name.into();
            col.units = units;
            col.comment = format!("直接合并，重复 X {:?}", duplicate);
            col.data = values.iter().map(ToString::to_string).collect();
        }
        self.books.push(book);
        self.move_project_item(id, folder)?;
        let (gid, _) = self.plot_from_selection(id, &[], PlotType::Line)?;
        Ok((id, gid))
    }
}
