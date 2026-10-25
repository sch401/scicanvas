use super::*;
use scicanvas_core::nlfit::{CustomFitDefinition, SavedFitFunction};

pub enum CustomFitAction {
    Save {
        before: Option<SavedFitFunction>,
        definition: Result<CustomFitDefinition, String>,
    },
    LoadFile,
    Export(Result<CustomFitDefinition, String>),
    Edit(CustomFitDefinition),
}

pub struct CustomFitEditor {
    pub name: String,
    pub formula: String,
    pub parameters: String,
    pub initial: String,
    pub constraints: String,
    pub before: Option<SavedFitFunction>,
}

impl Default for CustomFitEditor {
    fn default() -> Self {
        Self {
            name: "自定义函数".into(),
            formula: "y = a*exp(-b*x)+c".into(),
            parameters: "a, b, c".into(),
            initial: "2, 0.3, 0".into(),
            constraints: String::new(),
            before: None,
        }
    }
}

impl CustomFitEditor {
    pub fn from_definition(definition: &CustomFitDefinition) -> Self {
        Self {
            name: definition.name.clone(),
            formula: definition.formula.clone(),
            parameters: definition.parameters.join(", "),
            initial: definition
                .initial
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(", "),
            constraints: definition.constraints.clone(),
            before: None,
        }
    }

    pub fn saved(saved: &SavedFitFunction) -> Self {
        let mut editor = Self::from_definition(&saved.definition);
        editor.before = Some(saved.clone());
        editor
    }

    pub fn definition(&self) -> Result<CustomFitDefinition, String> {
        CustomFitDefinition::from_text(
            &self.name,
            &self.formula,
            &self.parameters,
            &self.initial,
            &self.constraints,
        )
    }

    pub fn sync_saved(&mut self, library: &[SavedFitFunction]) {
        let Some(before) = &self.before else { return };
        let clean = Self::from_definition(&before.definition);
        if self.name == clean.name
            && self.formula == clean.formula
            && self.parameters == clean.parameters
            && self.initial == clean.initial
            && self.constraints == clean.constraints
        {
            if let Some(current) = library.iter().find(|entry| entry.id == before.id) {
                if current != before {
                    *self = Self::saved(current);
                }
            }
        }
    }
}

impl Dialog {
    pub(super) fn show_nl_fit(&mut self, ctx: &egui::Context, pal: &Palette, open: &mut bool, _esc: bool) -> Action {
        let Self::NlFit {
            model,
            limits,
            constraints,
        } = self
        else {
            unreachable!()
        };
        let mut action = Action::None;
        window(ctx, "非线性拟合").open(open).show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label("类别");
                let mut category = model.category();
                egui::ComboBox::from_id_salt("nlfit-category")
                    .selected_text(category)
                    .show_ui(ui, |ui| {
                        for c in Model::CATEGORIES {
                            ui.selectable_value(&mut category, c, c);
                        }
                    });
                if category != model.category() {
                    if let Some(m) = Model::ALL.into_iter().find(|m| m.category() == category) {
                        *model = m;
                    }
                }
            });
            egui::Grid::new("nlfit-models")
                .num_columns(2)
                .max_col_width((ui.available_width() - 120.0).max(100.0))
                .spacing([12.0, 6.0])
                .show(ui, |ui| {
                    let category = model.category();
                    for m in Model::ALL.into_iter().filter(|m| m.category() == category) {
                        ui.radio_value(model, m, m.name());
                        ui.add(egui::Label::new(RichText::new(m.formula()).monospace()).wrap());
                        ui.end_row();
                    }
                });
            ui.label(
                RichText::new(format!(
                    "参数：{}。初值由数据自动估计，Levenberg–Marquardt 迭代；\n对当前图形的数据曲线，或工作簿里选中的 Y 列拟合，拟合曲线加到工作簿和图中。",
                    model.params().join("、")
                ))
                .small()
                .color(pal.weak),
            );
            ui.add_space(4.0);
            for (name, (fix, fv, has_lo, lo, has_hi, hi)) in model.params().iter().zip(limits.iter_mut()) {
                ui.strong(*name);
                let free = !*fix;
                egui::Grid::new(("nlfit-limits", name)).num_columns(2).spacing([8.0, 4.0]).show(ui, |ui| {
                    for (label, on, value, enabled) in [("固定", fix, fv, true), ("下限", has_lo, lo, free), ("上限", has_hi, hi, free)] {
                        ui.add_enabled_ui(enabled, |ui| { ui.checkbox(on, label); });
                        ui.add_enabled(enabled && *on, egui::DragValue::new(value).speed(0.01));
                        ui.end_row();
                    }
                });
            }
            ui.add_space(4.0);
            ui.label(RichText::new("参数约束").color(pal.weak));
            ui.add(
                egui::TextEdit::multiline(constraints)
                    .desired_rows(2)
                    .desired_width(f32::INFINITY)
                    .font(egui::TextStyle::Monospace)
                    .hint_text("每行一条，如 A1 + A2 = 1、t2 >= t1"),
            );
            if ui.button("自定义函数…").clicked() {
                action = Action::OpenCustomNlFit;
            }
            let (ok, cancel) = buttons(ui, "拟合");
            if ok && !matches!(action, Action::OpenCustomNlFit) {
                let held = model
                    .params()
                    .iter()
                    .zip(limits.iter())
                    .map(|(_, &(fix, fv, has_lo, lo, has_hi, hi))| scicanvas_core::nlfit::ParamLimits {
                        fixed: fix.then_some(fv),
                        lower: (!fix && has_lo).then_some(lo),
                        upper: (!fix && has_hi).then_some(hi),
                    })
                    .collect();
                action = Action::NlFit(*model, held, constraints.clone());
            } else if cancel && !matches!(action, Action::OpenCustomNlFit) {
                action = Action::Close;
            }
        });
        action
    }

    pub(super) fn show_custom_nl_fit(
        &mut self,
        ctx: &egui::Context,
        pal: &Palette,
        open: &mut bool,
        _esc: bool,
        project: &scicanvas_core::Project,
    ) -> Action {
        let Self::CustomNlFit(edit) = self else { unreachable!() };
        edit.sync_saved(&project.fit_functions);
        let mut action = Action::None;
        window(ctx, "自定义非线性拟合").open(open).show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label("项目函数");
                let before = edit.before.as_ref().map(|entry| entry.id);
                let mut picked = before;
                let mut load = None;
                let selected = project
                    .fit_functions
                    .iter()
                    .find(|entry| Some(entry.id) == before)
                    .map_or("新函数", |entry| entry.definition.name.as_str());
                egui::ComboBox::from_id_salt("custom-fit-library")
                    .selected_text(selected)
                    .show_ui(ui, |ui| {
                        for entry in &project.fit_functions {
                            if ui
                                .selectable_value(&mut picked, Some(entry.id), &entry.definition.name)
                                .clicked()
                            {
                                load = Some(entry.clone());
                            }
                        }
                    });
                if let Some(entry) = load {
                    *edit = CustomFitEditor::saved(&entry);
                }
                if ui.button("新建").clicked() {
                    *edit = CustomFitEditor::default();
                }
            });
            ui.label(
                RichText::new("使用 x 和参数名组成标量表达式，参数初值以逗号分隔。最多 7 个参数。").color(pal.weak),
            );
            ui.label("函数名");
            ui.text_edit_singleline(&mut edit.name);
            ui.label("表达式");
            ui.add(
                egui::TextEdit::multiline(&mut edit.formula)
                    .desired_rows(2)
                    .desired_width(f32::INFINITY)
                    .font(egui::TextStyle::Monospace)
                    .hint_text("y = a*exp(-b*x)+c"),
            );
            ui.label("参数名");
            ui.text_edit_singleline(&mut edit.parameters);
            ui.label("初始值");
            ui.text_edit_singleline(&mut edit.initial);
            ui.label("参数约束");
            ui.add(
                egui::TextEdit::multiline(&mut edit.constraints)
                    .desired_rows(2)
                    .desired_width(f32::INFINITY)
                    .font(egui::TextStyle::Monospace)
                    .hint_text("每行一条，如 a >= 0"),
            );
            ui.horizontal_wrapped(|ui| {
                if ui
                    .button("保存函数")
                    .on_hover_text("保存到当前项目函数库，随项目文件保存")
                    .clicked()
                {
                    action = Action::CustomFitLibrary(Box::new(CustomFitAction::Save {
                        before: edit.before.clone(),
                        definition: edit.definition(),
                    }));
                }
                if ui.button("载入文件…").clicked() {
                    action = Action::CustomFitLibrary(Box::new(CustomFitAction::LoadFile));
                }
                if ui.button("导出函数…").clicked() {
                    action = Action::CustomFitLibrary(Box::new(CustomFitAction::Export(edit.definition())));
                }
            });
            let (ok, cancel) = buttons(ui, "拟合");
            if ok {
                action = Action::CustomNlFit {
                    name: edit.name.clone(),
                    formula: edit.formula.clone(),
                    parameters: edit.parameters.clone(),
                    initial: edit.initial.clone(),
                    constraints: edit.constraints.clone(),
                };
            } else if cancel {
                action = Action::Close;
            }
        });
        action
    }
}
