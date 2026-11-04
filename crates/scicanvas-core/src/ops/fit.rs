//! 拟合、置信/预测带、残差图与校准。
use super::*;

/// 一条曲线的非线性拟合结果。
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct NlFitReport {
    pub id: u64,
    pub current: bool,
    pub error: Option<String>,
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
                    visible_only: item.visible_only,
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
        let ys = book
            .column(&curve.1)
            .ok_or("拟合曲线的列已被删除，请重新拟合")?
            .values();
        let units = book.column(y_key).map(|c| c.units.clone()).unwrap_or_default();
        let pct = fmt_num(level * 100.0);
        let rows: Vec<(f64, f64, f64)> = xs
            .iter()
            .enumerate()
            .map(|(i, x)| {
                if ys.get(i).is_some_and(|y| y.is_finite()) {
                    band(*x)
                } else {
                    (f64::NAN, f64::NAN, f64::NAN)
                }
            })
            .collect();
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
            c.independent_rows = true;
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
            visible_only: false,
        };
        let keys = self.write_result(&source, columns, ResultLink::None)?.columns;
        let item = Item {
            book_id,
            y: curve.1.clone(),
            x: Some(curve.0.clone()),
            rows: None,
            visible_only: false,
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
        self.book(r.book_id).ok_or("找不到工作簿")?;
        let r = &self.nlfit_report(r).ok_or("拟合结果已删除，请重新打开报告")?.clone();
        if !r.current || r.error.is_some() {
            return Err("请先成功重算拟合，再添加置信带".into());
        }
        let book = self.book(r.book_id).ok_or("找不到工作簿")?;
        if let Some(
            op @ Operation {
                kind: OpKind::Nonlinear(recipe),
                ..
            },
        ) = book.column(&r.curve.1).and_then(|c| c.op.as_ref())
        {
            if recipe.input_digest != Some(recipe.fingerprint(op, book)) {
                return Err("来源已改变，请先重算拟合再添加置信带".into());
            }
        }
        let fit = r.fit.clone();
        let keys = self.add_bands(r.book_id, &r.y_key, &r.curve, &r.name, level, |x| {
            let (c, p) = fit.band(x, level);
            (fit.eval(x), c, p)
        })?;
        if let Some(Operation {
            kind: OpKind::Nonlinear(recipe),
            ..
        }) = &mut self
            .book_mut(r.book_id)
            .unwrap()
            .columns
            .iter_mut()
            .find(|c| c.key == r.curve.1)
            .unwrap()
            .op
        {
            recipe.bands.push(FitBands {
                columns: keys.clone().try_into().unwrap(),
                level,
            });
        }
        Ok(keys)
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
        layer.reference_lines.push(crate::page::ReferenceLine {
            axis: crate::page::ReferenceAxis::YLeft,
            value: 0.0,
            ..Default::default()
        });
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
        self.nlfit_weighted(
            src,
            crate::nlfit::FitFunction::Builtin(model),
            None,
            limits,
            constraints,
            FitWeight::None,
        )
    }

    pub fn nlfit_custom(
        &mut self,
        src: &Source,
        function: crate::nlfit::CustomFitFunction,
        initial: &[f64],
        limits: &[crate::nlfit::ParamLimits],
        constraints: &[crate::nlfit::LinearConstraint],
    ) -> Result<(u64, Vec<NlFitReport>), String> {
        self.nlfit_weighted(
            src,
            crate::nlfit::FitFunction::Custom(function),
            Some(initial),
            limits,
            constraints,
            FitWeight::None,
        )
    }

    /// 非线性拟合通用入口：`weight` 为 `FitWeight::YError` 时每条曲线按关联的 Y 误差列加权。
    pub fn nlfit_weighted(
        &mut self,
        src: &Source,
        function: crate::nlfit::FitFunction,
        initial: Option<&[f64]>,
        limits: &[crate::nlfit::ParamLimits],
        constraints: &[crate::nlfit::LinearConstraint],
        weight: FitWeight,
    ) -> Result<(u64, Vec<NlFitReport>), String> {
        let r = self.resolve(src)?;
        let mut reports = vec![];
        let mut graph = None;
        let mut problems = vec![];
        for item in &r.items {
            let book = self.book(item.book_id).unwrap();
            let (xs, ys, rows) = get_xy(book, item);
            let name = r.name(book, item);
            let sigma = match weight {
                FitWeight::None => Ok(None),
                FitWeight::YError => y_errors(book, &item.y, &rows).map(Some),
            };
            let fit = match sigma.and_then(|sigma| {
                crate::nlfit::fit_function_weighted(
                    function.clone(),
                    &xs,
                    &ys,
                    sigma.as_deref(),
                    initial,
                    limits,
                    constraints,
                )
            }) {
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
            let fx = interpolate::grid(fit.x_min, fit.x_max, 200);
            let mut cx = Column::new("", Role::X);
            cx.long_name = format!("{name} {} 拟合 X", fit.model.name());
            cx.units = x_units;
            cx.comment = fit.model.formula().into();
            cx.data = fx.iter().map(|x| fmt_num(*x)).collect();
            let mut cy = Column::new("", Role::Y);
            cy.long_name = format!("{name} {} 拟合", fit.model.name());
            cy.units = y_units;
            cy.comment = fit_comment(&fit);
            cy.data = fit.sample(&fx).into_iter().map(fmt_num).collect();
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
            let mut operation = Operation {
                kind: OpKind::Nonlinear(Box::new(NonlinearRecipe {
                    function: FitMethod::from_function(&function),
                    initial: initial.map(<[f64]>::to_vec),
                    limits: limits.to_vec(),
                    constraints: constraints
                        .iter()
                        .map(|c| c.text.clone())
                        .collect::<Vec<_>>()
                        .join("\n"),
                    output_x: kx.clone(),
                    bands: Vec::new(),
                    residual: None,
                    weight,
                    input_digest: None,
                    attempted_digest: None,
                })),
                y: item.y.clone(),
                x: item.x.clone(),
                rows: item.rows,
                visible_only: item.visible_only,
                mode: RecalcMode::Auto,
            };
            if let OpKind::Nonlinear(recipe) = &operation.kind {
                let fingerprint = recipe.fingerprint(&operation, self.book(item.book_id).unwrap());
                if let OpKind::Nonlinear(recipe) = &mut operation.kind {
                    recipe.input_digest = Some(fingerprint);
                }
            }
            self.book_mut(item.book_id)
                .unwrap()
                .columns
                .iter_mut()
                .find(|c| c.key == ky)
                .unwrap()
                .op = Some(operation);
            graph = result.graph;
            let report = NlFitReport {
                id: self.alloc_id(),
                current: fit.converged,
                error: None,
                name,
                fit,
                book_id: item.book_id,
                y_key: item.y.clone(),
                curve: (kx, ky.clone()),
            };
            self.book_mut(item.book_id)
                .unwrap()
                .columns
                .iter_mut()
                .find(|c| c.key == ky)
                .unwrap()
                .fit_report = Some(Box::new(report.clone()));
            reports.push(report);
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
        self.calibrate_with_visibility(book_id, x, y, sample, false)
    }

    pub fn calibrate_with_visibility(
        &mut self,
        book_id: u64,
        x: &str,
        y: &str,
        sample: &str,
        visible_only: bool,
    ) -> Result<(crate::stats::Calibration, String, String), String> {
        let book = self.book(book_id).ok_or("找不到工作簿")?;
        let col = |k: &str| book.column(k).ok_or_else(|| format!("找不到列 {k}"));
        let (cx, cy, cs) = (col(x)?, col(y)?, col(sample)?);
        let (xs, ys) = if visible_only {
            (book.visible_values(cx), book.visible_values(cy))
        } else {
            (cx.values(), cy.values())
        };
        let cal = crate::stats::Calibration::fit(&xs, &ys)?;
        let (x_name, x_units, s_name) = (
            cx.display_name().to_owned(),
            cx.units.clone(),
            cs.display_name().to_owned(),
        );
        let inv: Vec<Option<(f64, f64, f64)>> = cs
            .values()
            .iter()
            .enumerate()
            .map(|(row, v)| (v.is_finite() && (!visible_only || book.row_visible(row))).then(|| cal.inverse(*v, 1)))
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
            visible_only: false,
        };
        let result = self.write_result(&item, vec![c0, c1], ResultLink::None)?;
        let [k0, k1]: [String; 2] = result.columns.try_into().unwrap();
        Ok((cal, k0, k1))
    }
}

impl Project {
    pub fn nlfit_reports(&self) -> Vec<NlFitReport> {
        self.books
            .iter()
            .flat_map(|b| &b.columns)
            .filter_map(|c| c.fit_report.as_deref().cloned())
            .collect()
    }

    pub fn nlfit_report(&self, reference: &NlFitReport) -> Option<&NlFitReport> {
        self.book(reference.book_id)?
            .column(&reference.curve.1)?
            .fit_report
            .as_deref()
            .filter(|r| r.id == reference.id)
    }

    pub fn nlfit_operation(&self, reference: &NlFitReport) -> Result<&Operation, String> {
        self.nlfit_report(reference).ok_or("拟合结果已删除，请重新打开报告")?;
        let op = self
            .book(reference.book_id)
            .unwrap()
            .column(&reference.curve.1)
            .unwrap()
            .op
            .as_ref()
            .ok_or("来源或输出已删除，当前报告为快照")?;
        if !matches!(op.kind, OpKind::Nonlinear(_)) {
            return Err("该结果已使用其他计算定义".into());
        }
        Ok(op)
    }

    /// 求解及输出准备成功后，一起写入原结果组。
    pub fn edit_nlfit(&mut self, reference: &NlFitReport, recipe: NonlinearRecipe) -> Result<NlFitReport, String> {
        let mut op = self.nlfit_operation(reference)?.clone();
        let OpKind::Nonlinear(old) = &op.kind else {
            unreachable!()
        };
        if recipe.output_x != old.output_x || recipe.bands != old.bands || recipe.residual != old.residual {
            return Err("拟合输出组已改变，请重新打开设置".into());
        }
        op.kind = OpKind::Nonlinear(Box::new(recipe));
        let book = self.book(reference.book_id).unwrap();
        let OpKind::Nonlinear(recipe) = &op.kind else {
            unreachable!()
        };
        let (outputs, fit) = recipe.calculate(&op, book)?;
        self.write_nlfit(reference.book_id, &reference.curve.1, op, outputs, fit)?;
        Ok(self.nlfit_report(reference).unwrap().clone())
    }

    pub(super) fn write_nlfit(
        &mut self,
        book_id: u64,
        key: &str,
        mut op: Operation,
        mut outputs: Vec<(String, crate::ColumnData)>,
        fit: crate::nlfit::NlFit,
    ) -> Result<(), String> {
        outputs.last_mut().unwrap().0 = key.into();
        let book = self.book(book_id).ok_or("找不到工作簿")?;
        if outputs.iter().any(|(k, _)| book.column(k).is_none()) {
            return Err("拟合输出已删除，保留原结果".into());
        }
        let OpKind::Nonlinear(recipe) = &op.kind else {
            unreachable!()
        };
        let hash = recipe.fingerprint(&op, book);
        let OpKind::Nonlinear(recipe) = &mut op.kind else {
            unreachable!()
        };
        recipe.input_digest = Some(hash);
        recipe.attempted_digest = None;
        let old_report = book.column(key).and_then(|c| c.fit_report.as_deref());
        let source_name = old_report.map(|r| r.name.clone()).unwrap_or_else(|| {
            format!(
                "{}：{}",
                book.name,
                book.column(&op.y).map_or(op.y.as_str(), |c| c.display_name())
            )
        });
        let update_comment = old_report.is_some_and(|r| book.column(key).unwrap().comment == fit_comment(&r.fit));
        let report_id = book.column(key).and_then(|c| c.fit_report.as_ref()).map(|r| r.id);
        let report_id = report_id.unwrap_or_else(|| self.alloc_id());
        let OpKind::Nonlinear(recipe) = &op.kind else {
            unreachable!()
        };
        let curve = (recipe.output_x.clone(), key.into());
        let y_key = op.y.clone();
        let book = self.book_mut(book_id).unwrap();
        for (k, data) in outputs {
            book.columns.iter_mut().find(|c| c.key == k).unwrap().data = data.into();
        }
        let col = book.columns.iter_mut().find(|c| c.key == key).unwrap();
        if update_comment {
            col.comment = fit_comment(&fit);
        }
        col.op = Some(op);
        col.fit_report = Some(Box::new(NlFitReport {
            id: report_id,
            current: true,
            error: None,
            name: source_name,
            fit,
            book_id,
            y_key,
            curve,
        }));
        Ok(())
    }
}

impl NlFitReport {
    pub fn validate(&self) -> Result<(), String> {
        let f = &self.fit;
        let n = f.model.params().len();
        if self.id == 0
            || self.curve.0.is_empty()
            || self.curve.1.is_empty()
            || self.curve.0 == self.curve.1
            || self.y_key.is_empty()
            || f.params.len() != n
            || f.se.len() != n
            || f.fixed.len() != n
            || f.at_bound.len() != n
            || f.cov.len() != n
            || f.cov.iter().any(|row| row.len() != n)
            || f.params.iter().any(|v| !v.is_finite())
            || !f.x_min.is_finite()
            || !f.x_max.is_finite()
            || f.x_min > f.x_max
            || !f.rss.is_finite()
            || f.rss < 0.0
            || f.se.iter().any(|v| v.is_infinite() || *v < 0.0)
            || f.cov.iter().flatten().any(|v| v.is_infinite())
            || [f.r2, f.adj_r2, f.reduced_chi2].iter().any(|v| v.is_infinite())
            || f.n == 0
            || f.dof > f.n
            || (f.status == crate::nlfit::FitStatus::PrecisionLimit && !f.converged)
            || (f.status == crate::nlfit::FitStatus::Failed && f.converged)
            || (self.current && (!f.converged || self.error.is_some()))
            || f.warnings.iter().map(String::len).sum::<usize>() > crate::formula::FILE_LIMITS.bytes
            || f.constraints.iter().map(String::len).sum::<usize>() > crate::formula::FILE_LIMITS.bytes
        {
            return Err("拟合报告的参数、统计量或状态无效".into());
        }
        crate::nlfit::parse_constraints_for_function_with(
            &f.model,
            &f.constraints.join("\n"),
            &crate::formula::FILE_LIMITS,
        )?;
        Ok(())
    }

    pub(crate) fn estimated_bytes(&self) -> usize {
        use std::mem::size_of;
        let f = &self.fit;
        size_of::<Self>()
            + [&self.name, &self.y_key, &self.curve.0, &self.curve.1]
                .iter()
                .map(|s| s.capacity())
                .sum::<usize>()
            + self.error.as_ref().map_or(0, String::capacity)
            + f.model.name().len()
            + f.model.formula().len()
            + f.model.params().iter().map(|s| s.len()).sum::<usize>()
            + (f.params.capacity() + f.se.capacity()) * size_of::<f64>()
            + f.fixed.capacity()
            + f.at_bound.capacity()
            + f.cov.capacity() * size_of::<Vec<f64>>()
            + f.cov.iter().map(|r| r.capacity() * size_of::<f64>()).sum::<usize>()
            + f.warnings.capacity() * size_of::<String>()
            + f.warnings.iter().map(String::capacity).sum::<usize>()
            + f.constraints.capacity() * size_of::<String>()
            + f.constraints.iter().map(String::capacity).sum::<usize>()
    }
}

fn fit_comment(fit: &crate::nlfit::NlFit) -> String {
    let equation = fit
        .model
        .params()
        .iter()
        .zip(&fit.params)
        .map(|(name, value)| format!("{name} = {}", fmt_num(round_sig(*value, 6))))
        .collect::<Vec<_>>()
        .join("，");
    format!("{}；{}", fit.model.formula(), equation)
}
