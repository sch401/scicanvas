//! 拟合、置信/预测带、残差图与校准。
use super::*;

/// 一条曲线的非线性拟合结果。
#[derive(Clone, Debug, PartialEq)]
pub struct NlFitReport {
    pub name: String,
    pub fit: crate::nlfit::NlFit,
    pub book_id: u64,
    /// 原数据 Y 列与拟合曲线的 (X, Y) 列。
    pub y_key: String,
    pub curve: (String, String),
}

/// 一条拟合的结果，供界面显示报告。
#[derive(Clone, Debug)]
pub struct FitReport {
    pub name: String,
    pub kind: String,
    pub fit: PolyFit,
    /// 残差列所在工作簿、对应的 X 列和残差列（与原数据逐行对齐）。
    pub book_id: u64,
    pub x_key: Option<String>,
    pub residual_key: String,
    /// 原数据 Y 列与拟合曲线的 (X, Y) 列。
    pub y_key: String,
    pub curve: (String, String),
}

impl Project {
    /// 多项式拟合（阶数 1 即线性拟合）。每条数据新增“拟合 X”和拟合值两列（200 个等间距点），
    /// 并把拟合曲线加到图里。返回 (图形, 各条数据的拟合报告)。
    pub fn fit_poly(&mut self, src: &Source, order: usize) -> Result<(u64, Vec<FitReport>), String> {
        let r = self.resolve(src)?;
        let kind = if order == 1 {
            "线性拟合".to_string()
        } else {
            format!("{} 阶多项式拟合", order)
        };
        let mut reports = vec![];
        let mut graph = None;
        let mut skipped = vec![];
        for item in &r.items {
            let book = self.book(item.book_id).unwrap();
            let (xs, ys, rows) = get_xy(book, item);
            let name = r.name(book, item);
            let nrows = book.nrows();
            let Some(fit) = analysis::polyfit(&xs, &ys, order) else {
                skipped.push(name);
                continue;
            };
            let x_units = item
                .x
                .as_deref()
                .and_then(|k| book.column(k))
                .map(|c| c.units.clone())
                .unwrap_or_default();
            let y_units = book.column(&item.y).map(|c| c.units.clone()).unwrap_or_default();
            // 逐点残差 Y − 拟合值，与原数据同一行；参与拟合之外的行留空
            let mut residual = vec![String::new(); nrows];
            for ((x, y), row) in xs.iter().zip(&ys).zip(&rows) {
                residual[*row] = fmt_num(y - fit.eval(*x));
            }
            let mut cr = Column::new("", Role::Y);
            cr.long_name = format!("{} 残差", name);
            cr.units = y_units.clone();
            cr.comment = format!("{kind}：Y − 拟合值");
            while residual.last().is_some_and(String::is_empty) {
                residual.pop();
            }
            cr.data = residual.into_iter().collect();
            let op = |kind: OpKind| {
                Some(Operation {
                    kind,
                    y: item.y.clone(),
                    x: item.x.clone(),
                    rows: item.rows,
                    mode: RecalcMode::Auto,
                })
            };
            cr.op = op(OpKind::PolyResidual { order });
            const N: usize = 200;
            let fx: Vec<f64> = (0..N)
                .map(|i| fit.x_min + (fit.x_max - fit.x_min) * i as f64 / (N - 1) as f64)
                .collect();
            let mut cx = Column::new("", Role::X);
            cx.long_name = format!("{} 拟合 X", name);
            cx.units = x_units;
            cx.comment = kind.clone();
            cx.data = fx.iter().map(|x| fmt_num(*x)).collect();
            cx.op = op(OpKind::PolyFitX { order });
            let mut cy = Column::new("", Role::Y);
            cy.long_name = format!("{} {}", name, kind);
            cy.units = y_units;
            cy.comment = fit.equation();
            cy.data = fx.iter().map(|x| fmt_num(fit.eval(*x))).collect();
            cy.op = op(OpKind::PolyFitY { order });
            let result = self.write_result(
                item,
                vec![cr, cx, cy],
                ResultLink::Auto {
                    resolved: &r,
                    x: 1,
                    y: 2,
                },
            )?;
            let [kr, kx, ky]: [String; 3] = result.columns.try_into().unwrap();
            graph = result.graph;
            reports.push(FitReport {
                name,
                kind: kind.clone(),
                fit,
                book_id: item.book_id,
                x_key: item.x.clone(),
                residual_key: kr,
                y_key: item.y.clone(),
                curve: (kx, ky),
            });
        }
        match graph {
            Some(g) => Ok((g, reports)),
            None => Err(format!(
                "{}：有效数据点太少，{}至少需要 {} 个点",
                skipped.join("、"),
                kind,
                order + 2
            )),
        }
    }

    /// 在拟合曲线的各个 X 上写置信带、预测带（下限、上限各一列，共四列），
    /// 并作为线加到含这条拟合曲线的图层。`band(x)` 返回 (拟合值, 置信半宽, 预测半宽)。返回新列。
    fn add_bands(
        &mut self,
        book_id: u64,
        y_key: &str,
        curve: &(String, String),
        name: &str,
        level: f64,
        band: impl Fn(f64) -> (f64, f64, f64),
    ) -> Result<Vec<String>, String> {
        let book = self.book(book_id).ok_or("找不到工作簿")?;
        let xs = book
            .column(&curve.0)
            .ok_or("拟合曲线的列已被删除，请重新拟合")?
            .values();
        if book.column(&curve.1).is_none() {
            return Err("拟合曲线的列已被删除，请重新拟合".into());
        }
        let units = book.column(y_key).map(|c| c.units.clone()).unwrap_or_default();
        let pct = fmt_num(level * 100.0);
        let rows: Vec<(f64, f64, f64)> = xs.iter().map(|x| band(*x)).collect();
        let mut columns = vec![];
        // 依次为置信下限、置信上限、预测下限、预测上限
        let value = |i: usize, b: &(f64, f64, f64)| match i {
            0 => b.0 - b.1,
            1 => b.0 + b.1,
            2 => b.0 - b.2,
            _ => b.0 + b.2,
        };
        for (i, label) in ["置信下限", "置信上限", "预测下限", "预测上限"].into_iter().enumerate() {
            let mut c = Column::new("", Role::Y);
            c.long_name = format!("{name} {pct}% {label}");
            c.units = units.clone();
            c.comment = "t 分布；置信带为拟合曲线的不确定度，预测带另含残差方差".into();
            c.data = xs
                .iter()
                .zip(&rows)
                .map(|(x, b)| {
                    if x.is_finite() {
                        fmt_num(value(i, b))
                    } else {
                        String::new()
                    }
                })
                .collect();
            columns.push(c);
        }
        let source = Item {
            book_id,
            y: y_key.into(),
            x: None,
            rows: None,
        };
        let keys = self.write_result(&source, columns, ResultLink::None)?.columns;
        let item = Item {
            book_id,
            y: curve.1.clone(),
            x: Some(curve.0.clone()),
            rows: None,
        };
        self.bind_results(&item, &keys, ResultLink::Matching)?;
        Ok(keys)
    }

    /// 线性/多项式拟合的置信带与预测带。
    pub fn poly_bands(&mut self, r: &FitReport, level: f64) -> Result<Vec<String>, String> {
        let fit = r.fit.clone();
        self.add_bands(r.book_id, &r.y_key, &r.curve, &r.name, level, |x| {
            let (c, p) = fit.band(x, level);
            (fit.eval(x), c, p)
        })
    }

    /// 非线性拟合的置信带与预测带（Δ 法）。
    pub fn nlfit_bands(&mut self, r: &NlFitReport, level: f64) -> Result<Vec<String>, String> {
        let fit = r.fit.clone();
        self.add_bands(r.book_id, &r.y_key, &r.curve, &r.name, level, |x| {
            let (c, p) = fit.band(x, level);
            (fit.eval(x), c, p)
        })
    }

    /// 残差图：各条拟合的残差对 X 的散点图，新建在工作簿所在目录。
    pub fn residual_plot(&mut self, reports: &[FitReport]) -> Result<u64, String> {
        let first = reports.first().ok_or("没有拟合结果")?;
        let mut layer = Layer {
            spec: crate::plotspec::PlotSpec {
                book_id: Some(first.book_id),
                x_key: first.x_key.clone(),
                plot_type: PlotType::Scatter,
                ..Default::default()
            }
            .into(),
            ..Default::default()
        };
        for r in reports {
            let book = self.book(r.book_id).ok_or("找不到工作簿")?;
            if book.column(&r.residual_key).is_none() {
                return Err("残差列已被删除或不在同一工作簿".into());
            }
            let plot = analysis_plot(&layer, book, &r.residual_key, r.x_key.as_deref(), false, None);
            layer.add_plot(plot, &|id| self.book(id))?;
        }
        self.plot_spec(layer.draft())
    }

    /// 非线性拟合：对每条曲线拟合内置函数（自动初值），拟合曲线（200 点）写成新的 X、Y 列并加到图上。
    pub fn nlfit(
        &mut self,
        src: &Source,
        model: crate::nlfit::Model,
        limits: &[crate::nlfit::ParamLimits],
        constraints: &[crate::nlfit::LinearConstraint],
    ) -> Result<(u64, Vec<NlFitReport>), String> {
        let r = self.resolve(src)?;
        let mut reports = vec![];
        let mut graph = None;
        let mut problems = vec![];
        for item in &r.items {
            let book = self.book(item.book_id).unwrap();
            let (xs, ys, _) = get_xy(book, item);
            let name = r.name(book, item);
            let fit = match crate::nlfit::fit_constrained(model, &xs, &ys, None, limits, constraints) {
                Ok(f) => f,
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
            let equation = model
                .params()
                .iter()
                .zip(&fit.params)
                .map(|(n, v)| format!("{n} = {}", fmt_num(round_sig(*v, 6))))
                .collect::<Vec<_>>()
                .join("，");
            let fx = interpolate::grid(fit.x_min, fit.x_max, 200);
            let mut cx = Column::new("", Role::X);
            cx.long_name = format!("{name} {} 拟合 X", model.name());
            cx.units = x_units;
            cx.comment = model.formula().into();
            cx.data = fx.iter().map(|x| fmt_num(*x)).collect();
            let mut cy = Column::new("", Role::Y);
            cy.long_name = format!("{name} {} 拟合", model.name());
            cy.units = y_units;
            cy.comment = format!("{}；{}", model.formula(), equation);
            cy.data = fx.iter().map(|x| fmt_num(fit.eval(*x))).collect();
            let result = self.write_result(
                item,
                vec![cx, cy],
                ResultLink::Auto {
                    resolved: &r,
                    x: 0,
                    y: 1,
                },
            )?;
            let [kx, ky]: [String; 2] = result.columns.try_into().unwrap();
            graph = result.graph;
            reports.push(NlFitReport {
                name,
                fit,
                book_id: item.book_id,
                y_key: item.y.clone(),
                curve: (kx, ky),
            });
        }
        match graph {
            Some(g) => Ok((g, reports)),
            None => Err(problems.join("；")),
        }
    }

    /// 标准曲线反查：用标准点（浓度 `x`、响应 `y`）拟合直线，把样品响应列 `sample` 反查成浓度，
    /// 在样品列右边写入“反查浓度”和“95% 置信区间半宽”两列（单次测定）。返回标准曲线与两个新列标识。
    pub fn calibrate(
        &mut self,
        book_id: u64,
        x: &str,
        y: &str,
        sample: &str,
    ) -> Result<(crate::stats::Calibration, String, String), String> {
        let book = self.book(book_id).ok_or("找不到工作簿")?;
        let col = |k: &str| book.column(k).ok_or_else(|| format!("找不到列 {k}"));
        let (cx, cy, cs) = (col(x)?, col(y)?, col(sample)?);
        let cal = crate::stats::Calibration::fit(&cx.values(), &cy.values())?;
        let (x_name, x_units, s_name) = (
            cx.display_name().to_owned(),
            cx.units.clone(),
            cs.display_name().to_owned(),
        );
        let inv: Vec<Option<(f64, f64, f64)>> = cs
            .values()
            .iter()
            .map(|v| v.is_finite().then(|| cal.inverse(*v, 1)))
            .collect();
        let note = format!(
            "{} = ({} − {}) / {}，R² = {}，n = {}",
            x_name,
            s_name,
            fmt_num(round_sig(cal.intercept, 6)),
            fmt_num(round_sig(cal.slope, 6)),
            fmt_num(round_sig(cal.r2, 6)),
            cal.n
        );
        let make = |long_name: String, pick: fn(&(f64, f64, f64)) -> f64| {
            let mut c = Column::new("", Role::Y);
            c.long_name = long_name;
            c.units = x_units.clone();
            c.comment = note.clone();
            c.data = inv
                .iter()
                .map(|v| v.as_ref().map_or(String::new(), |v| fmt_num(pick(v))))
                .collect();
            while c.data.last().is_some_and(|s| s.is_empty()) {
                c.data.pop();
            }
            c
        };
        let c0 = make(format!("{s_name} 反查{x_name}"), |v| v.0);
        let c1 = make("95% 置信区间 ±".into(), |v| v.2);
        let item = Item {
            book_id,
            y: sample.into(),
            x: None,
            rows: None,
        };
        let result = self.write_result(&item, vec![c0, c1], ResultLink::None)?;
        let [k0, k1]: [String; 2] = result.columns.try_into().unwrap();
        Ok((cal, k0, k1))
    }
}
