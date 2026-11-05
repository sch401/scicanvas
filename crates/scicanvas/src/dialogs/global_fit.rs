use super::*;
use scicanvas_core::{
    nlfit::{global_layout, FitFunction, ParamLimits},
    ops::Source,
};

#[derive(Clone)]
pub(crate) struct GlobalFitEditor {
    pub source: Source,
    pub labels: Vec<String>,
    pub model: Model,
    pub shared: Vec<bool>,
    pub limits: Vec<ParamLimits>,
    pub constraints: String,
    pub weighted: bool,
    errors_available: bool,
    names: Vec<String>,
    error: Option<String>,
}
impl GlobalFitEditor {
    pub fn new(source: Source, labels: Vec<String>, errors_available: bool) -> Self {
        let mut edit = Self {
            source,
            labels,
            model: Model::ExpDec1,
            shared: vec![true; 3],
            limits: Vec::new(),
            constraints: String::new(),
            weighted: false,
            errors_available,
            names: Vec::new(),
            error: None,
        };
        edit.sync();
        edit
    }
    fn sync(&mut self) {
        if self.shared.len() != self.model.params().len() {
            self.shared = vec![true; self.model.params().len()];
            self.constraints.clear();
        }
        match global_layout(&FitFunction::Builtin(self.model), self.labels.len(), &self.shared) {
            Ok(layout) => {
                if layout.names != self.names {
                    self.limits = layout
                        .names
                        .iter()
                        .map(|name| {
                            self.names
                                .iter()
                                .position(|n| n == name)
                                .map_or(ParamLimits::default(), |i| self.limits[i])
                        })
                        .collect();
                    self.names = layout.names;
                }
                self.error = None;
            }
            Err(error) => self.error = Some(error),
        }
    }
    fn model_controls(&mut self, ui: &mut egui::Ui) {
        let before = self.model;
        form(ui, "global-model-form", |f| {
            f.row("模型", |ui| {
                egui::ComboBox::from_id_salt("global-model")
                    .selected_text(self.model.name())
                    .show_ui(ui, |ui| {
                        for model in Model::ALL {
                            ui.selectable_value(&mut self.model, model, model.name());
                        }
                    });
            });
            if before != self.model {
                self.shared = vec![true; self.model.params().len()];
                self.constraints.clear();
                self.sync();
            }
            f.row("表达式", |ui| {
                ui.add(egui::Label::new(self.model.formula()).wrap());
            });
            f.row("共享参数", |ui| {
                ui.horizontal_wrapped(|ui| {
                    for (i, name) in self.model.params().iter().enumerate() {
                        ui.checkbox(&mut self.shared[i], format!("共享 {name}"));
                    }
                });
            });
        });
        self.sync();
    }
    pub fn show(&mut self, ctx: &egui::Context, pal: &Palette, open: &mut bool) -> Action {
        self.sync();
        let mut action = Action::None;
        window(ctx, "全局拟合")
            .centered()
            .default_width(crate::theme::dialog::LARGE)
            .open(open)
            .show(ctx, |ui| {
                ui.label(format!("数据集：{}", self.labels.join("、")));
                self.model_controls(ui);
                egui::ScrollArea::both().max_height(300.0).show(ui, |ui| {
                    egui::Grid::new("global-parameters").striped(true).show(ui, |ui| {
                        for label in ["参数", "固定", "下限", "上限"] {
                            ui.label(label);
                        }
                        ui.end_row();
                        for (name, limit) in self.names.iter().zip(&mut self.limits) {
                            ui.label(name);
                            for field in [&mut limit.fixed, &mut limit.lower, &mut limit.upper] {
                                ui.horizontal(|ui| {
                                    let mut enabled = field.is_some();
                                    let mut value = field.unwrap_or(0.0);
                                    ui.checkbox(&mut enabled, "");
                                    ui.add_enabled(enabled, egui::DragValue::new(&mut value).speed(0.1));
                                    *field = enabled.then_some(value);
                                });
                            }
                            ui.end_row();
                        }
                    });
                });
                if matches!(
                    self.model,
                    Model::CompetitiveInhibition | Model::NoncompetitiveInhibition | Model::UncompetitiveInhibition
                ) && (self.shared[3]
                    || self
                        .names
                        .iter()
                        .enumerate()
                        .filter(|(_, name)| name.starts_with("I_"))
                        .any(|(i, _)| self.limits[i].fixed.is_none()))
                {
                    ui.colored_label(pal.warn, "请把 I 设为独立参数，并逐组固定为该组抑制剂浓度。");
                }
                ui.checkbox(&mut self.weighted, "按各组关联的 Y 误差加权（1/σ²）");
                if self.weighted && !self.errors_available {
                    self.error = Some("全局拟合中各组需同时提供或同时不提供 Y 误差".into());
                }
                form(ui, "global-constraints", |f| {
                    f.row("线性约束", |ui| {
                        multiline(ui, &mut self.constraints).on_hover_text("展开参数名可用于等式或不等式。");
                    });
                });
                hint(ui, "结果不随源数据自动更新。");
                if let Some(error) = &self.error {
                    field_error(ui, error);
                }
                let names = self.names.iter().map(String::as_str).collect::<Vec<_>>();
                let valid = self
                    .error
                    .clone()
                    .map_or(Ok(()), Err)
                    .and_then(|_| parameter_bounds(&self.limits))
                    .and_then(|_| {
                        scicanvas_core::nlfit::parse_constraints_for_names(&names, &self.constraints).map(|_| ())
                    });
                if self.error.is_none() {
                    if let Err(error) = &valid {
                        field_error(ui, error);
                    }
                }
                let (ok, cancel) = buttons(ui, "拟合", valid);
                if ok && self.error.is_none() {
                    action = Action::GlobalFit(Box::new(self.clone()));
                } else if cancel {
                    action = Action::Close;
                }
            });
        action
    }
}
