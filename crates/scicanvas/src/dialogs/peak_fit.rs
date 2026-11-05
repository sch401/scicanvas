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
    pub fn show(&mut self, ctx: &egui::Context, pal: &Palette, open: &mut bool) -> Action {
        let mut action = Action::None;
        window(ctx, "多峰拟合").default_width(660.).open(open).show(ctx, |ui| {
            let before = (self.shape, self.count, self.baseline);
            ui.add_enabled_ui(self.target.is_none(), |ui| {
                ui.horizontal(|ui| {
                    egui::ComboBox::from_id_salt("peak-shape")
                        .selected_text(self.shape.name())
                        .show_ui(ui, |ui| {
                            for shape in [Model::Gauss, Model::Lorentz, Model::Voigt, Model::PsdVoigt1] {
                                ui.selectable_value(&mut self.shape, shape, shape.name());
                            }
                        });
                    ui.label("峰数");
                    ui.add(egui::DragValue::new(&mut self.count).range(1..=10));
                    egui::ComboBox::from_id_salt("peak-baseline")
                        .selected_text(format!("{:?}", self.baseline))
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
            if before != (self.shape, self.count, self.baseline) {
                self.initial.clear();
                self.preview = None;
                self.limits.clear();
            }
            ui.horizontal(|ui| {
                ui.label("最小突出度（Y 范围比例）");
                ui.add(egui::DragValue::new(&mut self.prominence).speed(0.01).range(0.0..=1.0));
            });
            let peak = self.definition().unwrap();
            if self.limits.len() != peak.params().len() {
                self.limits = peak.limits();
            }
            ui.label("先预览自动初值，再运行拟合。支持最多 10 个峰，Voigt 和 PsdVoigt1 最多 42 个参数。");
            ui.label("初始值（留空为自动寻峰）");
            if ui.text_edit_multiline(&mut self.initial).changed() {
                self.preview = None;
            }
            if ui.button("预览初值").clicked() {
                self.preview();
            }
            if let Some(points) = &self.preview {
                preview_plot(ui, &self.xs, &self.ys, points);
            }
            egui::ScrollArea::both()
                .id_salt("peak-parameters")
                .max_height(200.)
                .show(ui, |ui| {
                    egui::Grid::new("peak-limits").show(ui, |ui| {
                        for label in ["参数", "固定", "下限", "上限"] {
                            ui.label(label);
                        }
                        ui.end_row();
                        for (name, limit) in peak.params().iter().zip(&mut self.limits) {
                            ui.label(*name);
                            for field in [&mut limit.fixed, &mut limit.lower, &mut limit.upper] {
                                ui.horizontal(|ui| {
                                    let mut enabled = field.is_some();
                                    let mut value = field.unwrap_or(0.);
                                    ui.checkbox(&mut enabled, "");
                                    ui.add_enabled(enabled, egui::DragValue::new(&mut value).speed(0.01));
                                    *field = enabled.then_some(value);
                                });
                            }
                            ui.end_row();
                        }
                    });
                });
            ui.label("线性参数约束");
            ui.text_edit_multiline(&mut self.constraints);
            ui.checkbox(&mut self.weighted, "按 Y 误差加权（1/σ²）");
            if let Some(error) = &self.error {
                ui.colored_label(pal.error, error);
            }
            let (ok, cancel) = buttons(ui, "拟合");
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
                                    self.limits.clone(),
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
                                limits: self.limits.clone(),
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
