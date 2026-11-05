use super::*;
use scicanvas_core::{
    nlfit::{ParamLimits, PeakBaseline, PeakSum},
    ops::{FitMethod, FitWeight, Source},
};
pub(crate) struct PeakFitRequest {
    pub source: Source,
    pub peak: PeakSum,
    pub initial: Vec<f64>,
    pub limits: Vec<ParamLimits>,
    pub constraints: String,
    pub weighted: bool,
}
pub(crate) struct PeakFitEditor {
    pub source: Source,
    pub shape: Model,
    pub count: u8,
    pub baseline: PeakBaseline,
    pub prominence: f64,
    pub initial: String,
    pub limits: Vec<ParamLimits>,
    pub constraints: String,
    pub weighted: bool,
    pub target: Option<Box<FitEditTarget>>,
    pub xs: Vec<f64>,
    pub ys: Vec<f64>,
    pub preview: Option<Vec<[f64; 2]>>,
    pub error: Option<String>,
}
impl PeakFitEditor {
    pub fn new(source: Source, xs: Vec<f64>, ys: Vec<f64>) -> Self {
        Self {
            source,
            shape: Model::Gauss,
            count: 2,
            baseline: PeakBaseline::Constant,
            prominence: 0.05,
            initial: String::new(),
            limits: Vec::new(),
            constraints: String::new(),
            weighted: false,
            target: None,
            xs,
            ys,
            preview: None,
            error: None,
        }
    }
    fn definition(&self) -> Result<PeakSum, String> {
        PeakSum::new(self.shape, self.count, self.baseline)
    }
    fn values(&self) -> Result<Vec<f64>, String> {
        let values = self
            .initial
            .split([',', '，'])
            .map(|v| {
                v.trim()
                    .parse::<f64>()
                    .map_err(|_| "初值应为逗号分隔的有限数".to_string())
            })
            .collect::<Result<Vec<_>, _>>()?;
        if values.len() != self.definition()?.params().len() || values.iter().any(|v| !v.is_finite()) {
            return Err("多峰初始参数数量或数值无效".into());
        }
        Ok(values)
    }
    pub fn preview(&mut self) {
        let result = self.definition().and_then(|peak| {
            let p = if self.initial.trim().is_empty() {
                peak.guess(&self.xs, &self.ys, self.prominence)?
            } else {
                self.values()?
            };
            self.initial = p.iter().map(ToString::to_string).collect::<Vec<_>>().join(", ");
            let lo = self.xs.iter().copied().fold(f64::INFINITY, f64::min);
            let hi = self.xs.iter().copied().fold(f64::NEG_INFINITY, f64::max);
            self.preview = Some(
                (0..300)
                    .map(|i| {
                        let x = lo + (hi - lo) * i as f64 / 299.;
                        [x, peak.eval(x, &p)]
                    })
                    .collect(),
            );
            Ok(())
        });
        self.error = result.err();
    }
    fn model_form(&mut self, ui: &mut egui::Ui) {
        let before = (self.shape, self.count, self.baseline);
        form(ui, "peak-model", |f| {
            f.row("峰形", |ui| {
                ui.add_enabled_ui(self.target.is_none(), |ui| {
                    egui::ComboBox::from_id_salt("peak-shape")
                        .selected_text(self.shape.name())
                        .show_ui(ui, |ui| {
                            for shape in [Model::Gauss, Model::Lorentz, Model::Voigt, Model::PsdVoigt1] {
                                ui.selectable_value(&mut self.shape, shape, shape.name());
                            }
                        });
                });
            });
            f.row("峰数", |ui| {
                ui.add_enabled(
                    self.target.is_none(),
                    egui::DragValue::new(&mut self.count).range(1..=10),
                );
            });
            f.row("基线", |ui| {
                ui.add_enabled_ui(self.target.is_none(), |ui| {
                    egui::ComboBox::from_id_salt("peak-baseline")
                        .selected_text(match self.baseline {
                            PeakBaseline::None => "无基线",
                            PeakBaseline::Constant => "常数基线",
                            PeakBaseline::Linear => "线性基线",
                        })
                        .show_ui(ui, |ui| {
                            for (baseline, name) in [
                                (PeakBaseline::None, "无基线"),
                                (PeakBaseline::Constant, "常数基线"),
                                (PeakBaseline::Linear, "线性基线"),
                            ] {
                                ui.selectable_value(&mut self.baseline, baseline, name);
                            }
                        });
                });
            });
            f.row("最小突出度", |ui| {
                ui.add(egui::DragValue::new(&mut self.prominence).speed(0.01).range(0.0..=1.0))
                    .on_hover_text("占 Y 范围的比例。");
            });
        });
        if before != (self.shape, self.count, self.baseline) {
            self.initial.clear();
            self.preview = None;
            self.limits.clear();
        }
    }
    pub fn show(&mut self, ctx: &egui::Context, _pal: &Palette, open: &mut bool) -> Action {
        let mut action = Action::None;
        window(ctx, "多峰拟合")
            .centered()
            .default_width(crate::theme::dialog::LARGE)
            .open(open)
            .show(ctx, |ui| {
                self.model_form(ui);
                let peak = self.definition().unwrap();
                if self.limits.len() != peak.params().len() {
                    self.limits = peak.limits();
                }
                hint(ui, "先预览初值，再拟合。");
                form(ui, "peak-initial", |f| {
                    f.row("初始值", |ui| {
                        if multiline(ui, &mut self.initial)
                            .on_hover_text("留空则自动寻峰。")
                            .changed()
                        {
                            self.preview = None;
                        }
                    });
                });
                if ui.button("预览初值").clicked() {
                    self.preview();
                }
                if let Some(points) = &self.preview {
                    preview_plot(ui, &self.xs, &self.ys, points);
                }
                let limits_panel = ui.collapsing("参数限制", |ui| {
                    egui::ScrollArea::both()
                        .id_salt("peak-parameters")
                        .max_height(144.)
                        .show(ui, |ui| {
                            parameter_table(ui, "peak-limits", &peak.params(), &mut self.limits);
                        });
                });
                if limits_panel.header_response.clicked() {
                    ctx.data_mut(|d| {
                        d.insert_temp(egui::Id::new(("scicanvas-dialog", "多峰拟合")).with("recenter"), true)
                    });
                    ctx.request_repaint();
                }
                form(ui, "peak-constraints", |f| {
                    f.row("线性约束", |ui| {
                        multiline(ui, &mut self.constraints).on_hover_text(CONSTRAINT_HINT);
                    });
                });
                weighted_checkbox(
                    ui,
                    &mut self.weighted,
                    "按 Y 误差加权（1/σ²）",
                    ctx.data(|d| d.get_temp::<bool>(egui::Id::new("analysis-errors-available")))
                        .unwrap_or(false),
                );
                if let Some(error) = &self.error {
                    field_error(ui, error);
                }
                let valid = if self.preview.is_some() {
                    self.values()
                        .map(|_| ())
                        .and_then(|_| parameter_bounds(&self.limits))
                        .and_then(|_| {
                            scicanvas_core::nlfit::parse_constraints_for_names(&peak.params(), &self.constraints)
                                .map(|_| ())
                        })
                } else {
                    Err("请先预览初值".into())
                };
                if self.error.is_none() {
                    if let Err(error) = &valid {
                        field_error(ui, error);
                    }
                }
                let (ok, cancel) = buttons(ui, "拟合", valid);
                if ok {
                    match self.values() {
                        Ok(initial) if self.preview.is_some() => {
                            if let Some(target) = &self.target {
                                let recipe = target
                                    .recipe(
                                        FitMethod::PeakSum {
                                            shape: self.shape,
                                            count: self.count,
                                            baseline: self.baseline,
                                        },
                                        Some(initial),
                                        effective_limits(&self.limits),
                                        self.constraints.clone(),
                                    )
                                    .map(|mut recipe| {
                                        recipe.weight = FitWeight::from_y_error(self.weighted);
                                        recipe
                                    });
                                action = Action::Refit(
                                    Box::new(FitEditTarget {
                                        report: target.report.clone(),
                                        before: target.before.clone(),
                                    }),
                                    recipe,
                                );
                            } else {
                                action = Action::PeakFit(Box::new(PeakFitRequest {
                                    source: self.source.clone(),
                                    peak,
                                    initial,
                                    limits: effective_limits(&self.limits),
                                    constraints: self.constraints.clone(),
                                    weighted: self.weighted,
                                }));
                            }
                        }
                        Ok(_) => self.error = Some("请先预览初值".into()),
                        Err(error) => self.error = Some(error),
                    }
                } else if cancel {
                    action = Action::Close;
                }
            });
        action
    }
}
fn preview_plot(ui: &mut egui::Ui, xs: &[f64], ys: &[f64], curve: &[[f64; 2]]) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 140.), egui::Sense::hover());
    let points: Vec<_> = xs
        .iter()
        .zip(ys)
        .filter(|(x, y)| x.is_finite() && y.is_finite())
        .map(|(x, y)| [*x, *y])
        .collect();
    let lo: [f64; 2] = std::array::from_fn(|i| points.iter().chain(curve).map(|p| p[i]).fold(f64::INFINITY, f64::min));
    let hi: [f64; 2] = std::array::from_fn(|i| {
        points
            .iter()
            .chain(curve)
            .map(|p| p[i])
            .fold(f64::NEG_INFINITY, f64::max)
    });
    let at = |p: [f64; 2]| {
        egui::pos2(
            rect.left() + ((p[0] - lo[0]) / (hi[0] - lo[0]).max(1e-12)) as f32 * rect.width(),
            rect.bottom() - ((p[1] - lo[1]) / (hi[1] - lo[1]).max(1e-12)) as f32 * rect.height(),
        )
    };
    let painter = ui.painter();
    for point in points {
        painter.circle_filled(at(point), 2., ui.visuals().text_color());
    }
    painter.add(egui::Shape::line(
        curve.iter().map(|p| at(*p)).collect(),
        egui::Stroke::new(1.5_f32, ui.visuals().selection.bg_fill),
    ));
}
