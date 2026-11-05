use super::*;

#[derive(Clone, Debug)]
pub struct FitComparison {
    pub delta: [f64; 2],
    pub weights: [f64; 2],
    pub f: f64,
    pub p: f64,
}

pub fn compare_fits(a: &NlFitReport, b: &NlFitReport) -> Result<FitComparison, String> {
    if a.book_id != b.book_id || a.y_key != b.y_key || a.fit.n != b.fit.n {
        return Err("拟合必须使用同一工作簿、同一 Y 列和相同样本数".into());
    }
    if a.fit.weighted != b.fit.weighted {
        return Err("拟合比较需要相同加权方式".into());
    }
    let values = [a.fit.aicc(), b.fit.aicc()];
    let minimum = values[0].min(values[1]);
    let delta = values.map(|v| if v == minimum { 0.0 } else { v - minimum });
    let raw = delta.map(|v| (-0.5 * v).exp());
    let weights = raw.map(|v| v / (raw[0] + raw[1]));
    let (simple, full) = if a.fit.dof >= b.fit.dof {
        (&a.fit, &b.fit)
    } else {
        (&b.fit, &a.fit)
    };
    let (f, p) = if simple.dof != full.dof && full.dof > 0 {
        let f = (simple.rss - full.rss) / (simple.dof - full.dof) as f64 / (full.rss / full.dof as f64);
        (
            f,
            crate::stats::f_sf(f, (simple.dof - full.dof) as f64, full.dof as f64),
        )
    } else {
        (f64::NAN, f64::NAN)
    };
    Ok(FitComparison { delta, weights, f, p })
}

impl Project {
    pub fn nlfit_residuals(&mut self, reference: &NlFitReport) -> Result<String, String> {
        let report = self.nlfit_report(reference).ok_or("拟合报告已删除")?.clone();
        if !report.current {
            return Err("请先重算当前拟合".into());
        }
        let operation = self.nlfit_operation(&report)?.clone();
        let OpKind::Nonlinear(recipe) = &operation.kind else {
            unreachable!()
        };
        if let Some(key) = &recipe.residual {
            return Ok(key.clone());
        }
        let book = self.book(report.book_id).unwrap();
        let item = Item {
            book_id: book.id,
            y: operation.y.clone(),
            x: operation.x.clone(),
            rows: operation.rows,
            visible_only: operation.visible_only,
        };
        let (xs, ys, indices) = get_xy(book, &item);
        let mut values = vec![String::new(); book.column(&operation.y).unwrap().data.len()];
        for ((x, y), row) in xs.iter().zip(ys).zip(indices) {
            values[row] = fmt_num(y - report.fit.eval(*x));
        }
        let mut column = Column::new("", Role::Y);
        column.long_name = format!("{} 残差", report.name);
        column.units = book.column(&operation.y).unwrap().units.clone();
        column.comment = "残差 = Y − 拟合值，随原拟合重算".into();
        column.derived_from = Some(operation.y.clone());
        column.data = values.into();
        let key = self.insert_result_column(report.book_id, &operation.y, column).unwrap();
        let owner = self
            .book_mut(report.book_id)
            .unwrap()
            .columns
            .iter_mut()
            .find(|c| c.key == report.curve.1)
            .unwrap();
        let OpKind::Nonlinear(recipe) = &mut owner.op.as_mut().unwrap().kind else {
            unreachable!()
        };
        recipe.residual = Some(key.clone());
        Ok(key)
    }

    pub fn nlfit_residual_plot(&mut self, reports: &[NlFitReport], qq: bool) -> Result<u64, String> {
        let first = reports.first().ok_or("没有拟合结果")?;
        let mut layer = Layer::default();
        layer.spec.book_id = Some(first.book_id);
        layer.spec.plot_type = if qq { PlotType::QQ } else { PlotType::Scatter };
        for report in reports {
            let key = self.nlfit_residuals(report)?;
            let operation = self.nlfit_operation(report)?;
            let book = self.book(report.book_id).unwrap();
            let mut plot = analysis_plot(&layer, book, &key, operation.x.as_deref(), false, None);
            if operation.x.is_none() {
                plot.mapping.x = None;
                plot.x_override = true;
            }
            layer.add_plot(plot, &|id| self.book(id))?;
        }
        if !qq {
            layer.reference_lines.push(crate::page::ReferenceLine {
                axis: crate::page::ReferenceAxis::YLeft,
                value: 0.0,
                ..Default::default()
            });
        }
        self.plot_spec(layer.draft())
    }

    pub fn residual_qq(&mut self, reports: &[FitReport]) -> Result<u64, String> {
        let graph = self.residual_plot(reports)?;
        self.graph_mut(graph).unwrap().first_layer_mut().reference_lines.clear();
        self.graph_mut(graph)
            .unwrap()
            .first_layer_mut()
            .spec
            .set_plot_type(PlotType::QQ)?;
        Ok(graph)
    }

    /// 一次性标准曲线结果，紧接样品列写入估计值、标准误和 95% 半宽。
    pub fn evaluate_calibration(
        &mut self,
        book_id: u64,
        keys: &[String; 3],
        nonlinear: Option<&NlFitReport>,
        forward: bool,
    ) -> Result<Vec<String>, String> {
        let book = self.book(book_id).ok_or("找不到工作簿")?;
        let sample = book.column(&keys[2]).ok_or("找不到样品列")?;
        let sample_name = sample.display_name().to_owned();
        let visible = book.uses_visible_rows();
        let linear = if nonlinear.is_none() {
            let x = book.column(&keys[0]).ok_or("找不到标准 X 列")?;
            let y = book.column(&keys[1]).ok_or("找不到标准 Y 列")?;
            let values = |c: &Column| if visible { book.visible_values(c) } else { c.values() };
            Some(
                analysis::polyfit(&values(x), &values(y), 1)
                    .filter(|fit| fit.n >= 3)
                    .ok_or("标准曲线至少需要 3 个有效标准点和变化的 X")?,
            )
        } else {
            None
        };
        let inverse_linear = if nonlinear.is_none() && !forward {
            let x = book.column(&keys[0]).unwrap();
            let y = book.column(&keys[1]).unwrap();
            let (xs, ys) = if visible {
                (book.visible_values(x), book.visible_values(y))
            } else {
                (x.values(), y.values())
            };
            Some(crate::stats::Calibration::fit(&xs, &ys)?)
        } else {
            None
        };
        let units = if let Some(r) = nonlinear {
            self.book(r.book_id)
                .and_then(|b| b.column(if forward { &r.y_key } else { &r.curve.0 }))
                .map(|c| c.units.clone())
                .unwrap_or_default()
        } else {
            book.column(&keys[usize::from(forward)]).unwrap().units.clone()
        };
        let mut result = Vec::new();
        let mut notes = vec!["95% 区间；非线性反查用 delta 近似，标准误 se_x = se_ŷ / |f′(x)|。".to_owned()];
        for (row, input) in sample.values().into_iter().enumerate() {
            if !input.is_finite() || (visible && !book.row_visible(row)) {
                result.push(None);
                continue;
            }
            let evaluated = if let Some(report) = nonlinear {
                let fit = &report.fit;
                if !report.current {
                    return Err("请先重算非线性拟合".into());
                }
                if forward {
                    let (_, half) = fit.band(input, 0.95);
                    let t = crate::stats::t_ppf(0.975, fit.dof as f64);
                    if input < fit.x_min || input > fit.x_max {
                        notes.push(format!("第 {} 行：外推", row + 1));
                    }
                    Ok((fit.eval(input), half / t, half))
                } else {
                    fit.invert(input)
                        .map(|x| {
                            let h = f64::EPSILON.cbrt() * x.abs().max(1.0);
                            let derivative = (fit.eval(x + h) - fit.eval(x - h)) / (2.0 * h);
                            let half = fit.band(x, 0.95).0 / derivative.abs();
                            (x, half / crate::stats::t_ppf(0.975, fit.dof as f64), half)
                        })
                        .map_err(|e| e.to_string())
                }
            } else {
                let fit = linear.as_ref().unwrap();
                let t = crate::stats::t_ppf(0.975, fit.n.saturating_sub(2) as f64);
                if forward {
                    let half = fit.band(input, 0.95).1;
                    Ok((fit.eval(input), half / t, half))
                } else {
                    Ok(inverse_linear.as_ref().unwrap().inverse(input, 1))
                }
            };
            match evaluated {
                Ok(value) => result.push(Some(value)),
                Err(e) => {
                    notes.push(format!("第 {} 行：{e}", row + 1));
                    result.push(None);
                }
            }
        }
        let direction = if forward { "由 X 求 Y" } else { "由 Y 求 X" };
        let comment = format!(
            "{direction}；{}\n{}",
            nonlinear.map_or("线性", |r| r.fit.model.name()),
            notes.join("\n")
        );
        let columns: [Column; 3] = std::array::from_fn(|i| {
            let mut c = Column::new("", Role::Y);
            c.long_name = format!("{sample_name} {}", [direction, "标准误", "95% 区间 ±"][i]);
            c.comment = comment.clone();
            c.units = units.clone();
            c.data = result
                .iter()
                .map(|r| r.map_or(String::new(), |(v, se, half)| fmt_num([v, se, half][i])))
                .collect();
            c
        });
        let item = Item {
            book_id,
            y: keys[2].clone(),
            x: None,
            rows: None,
            visible_only: false,
        };
        Ok(self.write_result(&item, columns.to_vec(), ResultLink::None)?.columns)
    }
}
