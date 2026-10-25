//! 平滑、插值、归一化与曲线平均。
use super::*;

impl Project {
    /// 在等间距的 `points` 个 X 上插值（`range` 为空时用数据的 X 范围，超出数据时外推），
    /// 结果写成新的 X、Y 列并作为线加到图上。
    pub fn interpolate(
        &mut self,
        src: &Source,
        method: InterpMethod,
        points: usize,
        range: Option<(f64, f64)>,
    ) -> Result<(u64, Vec<String>), String> {
        let r = self.resolve(src)?;
        let kind = format!("{}插值", method.label());
        let mut graph = None;
        let mut made = vec![];
        let mut problems = vec![];
        for item in &r.items {
            let book = self.book(item.book_id).unwrap();
            let (xs, ys, _) = get_xy(book, item);
            let name = r.name(book, item);
            let (lo, hi) = range.unwrap_or_else(|| {
                let finite = || xs.iter().zip(&ys).filter(|(x, y)| x.is_finite() && y.is_finite());
                (
                    finite().map(|(x, _)| *x).fold(f64::INFINITY, f64::min),
                    finite().map(|(x, _)| *x).fold(f64::NEG_INFINITY, f64::max),
                )
            });
            let fx = interpolate::grid(lo, hi, points);
            let fy = match interpolate::interpolate(&xs, &ys, method, &fx) {
                Ok(fy) if lo.is_finite() && hi.is_finite() && lo < hi => fy,
                Ok(_) => {
                    problems.push(format!("{name}：X 范围无效"));
                    continue;
                }
                Err(e) => {
                    problems.push(format!("{name}：{e}"));
                    continue;
                }
            };
            let x_units = item
                .x
                .as_deref()
                .and_then(|k| book.column(k))
                .map(|c| c.units.clone())
                .unwrap_or_default();
            let y_units = book.column(&item.y).map(|c| c.units.clone()).unwrap_or_default();
            let note = format!("{kind}，{} 点，X 从 {} 到 {}", fx.len(), fmt_num(lo), fmt_num(hi));
            let mut cx = Column::new("", Role::X);
            cx.long_name = format!("{name} 插值 X");
            cx.units = x_units;
            cx.comment = note.clone();
            cx.data = fx.iter().map(|x| fmt_num(*x)).collect();
            let mut cy = Column::new("", Role::Y);
            cy.long_name = format!("{name} {kind}");
            cy.units = y_units;
            cy.comment = note;
            cy.data = fy.iter().map(|y| fmt_num(*y)).collect();
            let kx = self.insert_result_column(item.book_id, &item.y, cx).unwrap();
            let ky = self.insert_result_column(item.book_id, &item.y, cy).unwrap();
            let (gid, index) = self.graph_for_result(&r, item)?;
            self.attach_result(gid, index, item, &ky, Some(&kx))?;
            graph = Some(gid);
            made.push(ky);
        }
        match graph {
            Some(g) => Ok((g, made)),
            None => Err(problems.join("；")),
        }
    }

    /// 平滑。来源是图形时，平滑曲线也加到图里。返回新列标识。
    pub fn smooth(&mut self, src: &Source, method: SmoothMethod, window: usize) -> Result<Vec<String>, String> {
        if method == SmoothMethod::FftFilter {
            let r = self.resolve(src)?;
            for item in &r.items {
                let book = self.book(item.book_id).unwrap();
                let (xs, _, _) = get_xy(book, item);
                if item.x.is_some() {
                    crate::fft::sample_interval(&xs).map_err(|e| format!("{}：{e}", name_of(book, &item.y)))?;
                }
            }
        }
        let (r, made) = self.per_item_column(src, OpKind::Smooth { method, window }, |book, item, xs, ys| {
            let y = book.column(&item.y).unwrap();
            (
                analysis::smooth(xs, ys, method, window),
                format!("{} 平滑", y.display_name()),
                y.units.clone(),
                format!("{}，{} 点", method.label(), window.max(3) | 1),
            )
        })?;
        if let Some(gid) = r.graph_id {
            for (item, key) in &made {
                self.attach_result(gid, r.layer_index, item, key, item.x.as_deref())?;
            }
        }
        Ok(made.into_iter().map(|(_, k)| k).collect())
    }

    /// 多条曲线平均：把各曲线线性插值到公共 X 区间（各曲线 X 范围的交集）上的等间距点，
    /// 点数取最长曲线的点数；结果工作簿含 X、平均 Y、标准差（Y 误差）与 N，并画成线图。
    pub fn average_curves(&mut self, src: &Source) -> Result<(u64, u64), String> {
        let r = self.resolve(src)?;
        if r.items.len() < 2 {
            return Err("至少需要两条曲线".into());
        }
        let mut curves = vec![];
        for item in &r.items {
            let book = self.book(item.book_id).unwrap();
            let (xs, ys, _) = get_xy(book, item);
            if xs.len() < 2 {
                return Err(format!("{}：数据点太少", name_of(book, &item.y)));
            }
            curves.push((xs, ys));
        }
        let lo = curves
            .iter()
            .map(|(x, _)| x.iter().copied().fold(f64::INFINITY, f64::min))
            .fold(f64::NEG_INFINITY, f64::max);
        let hi = curves
            .iter()
            .map(|(x, _)| x.iter().copied().fold(f64::NEG_INFINITY, f64::max))
            .fold(f64::INFINITY, f64::min);
        if hi.is_nan() || lo.is_nan() || hi <= lo {
            return Err("各曲线的 X 范围没有重叠".into());
        }
        let n = curves.iter().map(|(x, _)| x.len()).max().unwrap();
        let grid = interpolate::grid(lo, hi, n);
        let mut ys = vec![];
        for (x, y) in &curves {
            ys.push(interpolate::interpolate(x, y, InterpMethod::Linear, &grid)?);
        }
        let k = ys.len() as f64;
        let mean: Vec<f64> = (0..n).map(|i| ys.iter().map(|v| v[i]).sum::<f64>() / k).collect();
        let sd: Vec<f64> = (0..n)
            .map(|i| (ys.iter().map(|v| (v[i] - mean[i]).powi(2)).sum::<f64>() / (k - 1.0)).sqrt())
            .collect();
        let first = &r.items[0];
        let book = self.book(first.book_id).unwrap();
        let x_units = first
            .x
            .as_deref()
            .and_then(|k| book.column(k))
            .map(|c| c.units.clone())
            .unwrap_or_default();
        let y_units = book.column(&first.y).map(|c| c.units.clone()).unwrap_or_default();
        let note = format!(
            "{} 条曲线在 X ∈ [{}, {}] 上线性插值后平均",
            r.items.len(),
            fmt_num(lo),
            fmt_num(hi)
        );
        let folder = self.folder_of(first.book_id);
        let id = self.alloc_id();
        let mut nb = Book::new(id, self.unique_name("曲线平均"));
        let mut mean_key = String::new();
        for (role, name, units, values) in [
            (Role::X, "X", &x_units, &grid),
            (Role::Y, "平均", &y_units, &mean),
            (Role::YError, "标准差", &y_units, &sd),
        ] {
            let key = nb.push_column(role);
            let c = nb.columns.iter_mut().find(|c| c.key == key).unwrap();
            c.long_name = name.into();
            c.units = units.clone();
            c.comment = note.clone();
            c.data = values.iter().map(|v| fmt_num(*v)).collect();
            if name == "平均" {
                mean_key = key;
            }
        }
        self.books.push(nb);
        self.move_project_item(id, folder)?;
        let (gid, _) = self.plot_from_selection(id, &[mean_key], PlotType::Line)?;
        Ok((id, gid))
    }

    /// 归一化，结果写成新列。
    pub fn normalize(&mut self, src: &Source, method: NormalizeMethod) -> Result<Vec<String>, String> {
        let (_, made) = self.per_item_column(src, OpKind::Normalize { method }, |book, item, _xs, ys| {
            let y = book.column(&item.y).unwrap();
            (
                analysis::normalize(ys, method),
                format!("{} 归一化", y.display_name()),
                String::new(),
                method.label().into(),
            )
        })?;
        Ok(made.into_iter().map(|(_, k)| k).collect())
    }
}
