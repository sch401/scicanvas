use super::*;
impl Dialog {
    pub(super) fn show_baseline(&mut self, ctx: &egui::Context, _pal: &Palette, open: &mut bool) -> Action {
        let Self::Baseline {
            advanced,
            count,
            spline,
            order,
            iterations,
            anchors,
            als,
            log_lambda,
            p,
        } = self
        else {
            unreachable!()
        };
        let mut action = Action::None;
        window(ctx, "扣除基线")
            .centered()
            .default_width(crate::theme::dialog::MEDIUM)
            .open(open)
            .show(ctx, |ui| {
                let mut parsed = Ok(Vec::new());
                form(ui, "baseline-form", |f| {
                    f.row("方法", |ui| {
                        ui.horizontal_wrapped(|ui| {
                            ui.radio_value(advanced, 0, "基础方法");
                            ui.radio_value(advanced, 1, "锚点法");
                            ui.radio_value(advanced, 2, "多项式基线");
                        });
                    });
                    match *advanced {
                        0 => {
                            f.row("基线", |ui| {
                                ui.horizontal_wrapped(|ui| {
                                    ui.radio_value(als, false, "两端连线");
                                    ui.radio_value(als, true, "自动基线（ALS）");
                                });
                            });
                            if *als {
                                f.row("log₁₀ λ", |ui| {
                                    ui.add(egui::DragValue::new(log_lambda).range(0.0..=12.0).speed(0.05));
                                });
                                f.row("p", |ui| {
                                    ui.add(egui::DragValue::new(p).range(0.0001..=0.5).speed(0.001));
                                });
                            }
                        }
                        1 => {
                            f.row("自动分段数", |ui| {
                                ui.add(egui::DragValue::new(count).range(2..=1000));
                            });
                            f.row("插值", |ui| {
                                ui.checkbox(spline, "自然三次样条");
                            });
                            f.row("锚点 X", |ui| {
                                ui.vertical(|ui| {
                                    multiline(ui, anchors).on_hover_text("逗号分隔，留空则自动选择。");
                                    parsed = anchor_values(anchors);
                                    if let Err(error) = &parsed {
                                        field_error(ui, error);
                                    }
                                });
                            });
                        }
                        _ => {
                            f.row("阶数", |ui| {
                                ui.add(egui::DragValue::new(order).range(0..=10));
                            });
                            f.row("最多迭代", |ui| {
                                ui.add(egui::DragValue::new(iterations).range(1..=1000));
                            });
                        }
                    }
                });
                hint(ui, "结果写入基线和扣除后两列，随源数据重算。");
                let valid = if *advanced == 0 && *als && (!log_lambda.is_finite() || !p.is_finite()) {
                    Err("基线参数应为有限数".into())
                } else {
                    parsed.as_ref().map(|_| ()).map_err(Clone::clone)
                };
                if *advanced == 0 {
                    if let Err(error) = &valid {
                        field_error(ui, error);
                    }
                }
                let (ok, cancel) = buttons(ui, "扣除", valid);
                if ok {
                    action = match *advanced {
                        1 => Action::SignalTools(Box::new(SignalAction::Baseline(
                            BaselineMethod::Anchors {
                                count: *count,
                                spline: *spline,
                            },
                            parsed.unwrap(),
                        ))),
                        2 => Action::SignalTools(Box::new(SignalAction::Baseline(
                            BaselineMethod::Polynomial {
                                order: *order,
                                iterations: *iterations,
                            },
                            Vec::new(),
                        ))),
                        _ => Action::Baseline(if *als {
                            BaselineMethod::Als {
                                lambda: 10f64.powf(*log_lambda),
                                p: *p,
                            }
                        } else {
                            BaselineMethod::Line
                        }),
                    };
                } else if cancel {
                    action = Action::Close;
                }
            });
        action
    }
}
fn anchor_values(text: &str) -> Result<Vec<f64>, String> {
    text.split([',', '，', ' ', '\n', ';', '；'])
        .filter(|s| !s.trim().is_empty())
        .map(|s| {
            s.trim()
                .parse::<f64>()
                .ok()
                .filter(|v| v.is_finite())
                .ok_or_else(|| "锚点 X 应为逗号分隔的有限数".into())
        })
        .collect()
}
