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

pub struct FitEditTarget {
    pub report: NlFitReport,
    pub before: scicanvas_core::ops::Operation,
}

impl FitEditTarget {
    pub(super) fn recipe(
        &self,
        function: scicanvas_core::ops::FitMethod,
        initial: Option<Vec<f64>>,
        limits: Vec<scicanvas_core::nlfit::ParamLimits>,
        constraints: String,
    ) -> Result<scicanvas_core::ops::NonlinearRecipe, String> {
        let scicanvas_core::ops::OpKind::Nonlinear(recipe) = &self.before.kind else {
            unreachable!()
        };
        let mut recipe = *recipe.clone();
        recipe.function = match (function, &recipe.function) {
            (
                scicanvas_core::ops::FitMethod::Builtin(Model::SineDamp),
                scicanvas_core::ops::FitMethod::SineDamp { origin },
            ) => scicanvas_core::ops::FitMethod::SineDamp { origin: *origin },
            (function, _) => function,
        };
        recipe.initial = initial;
        recipe.limits = limits;
        recipe.constraints = constraints;
        recipe.validate()?;
        Ok(recipe)
    }
}

pub struct CustomFitEditor {
    loaded_definition: Option<CustomFitDefinition>,
    pub target: Option<Box<FitEditTarget>>,
    pub limits: Vec<scicanvas_core::nlfit::ParamLimits>,
    limit_values: Vec<[f64; 3]>,
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
            target: None,
            loaded_definition: None,
            limits: Vec::new(),
            limit_values: Vec::new(),
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
    fn show_limits(&mut self, ui: &mut egui::Ui) {
        let names: Vec<_> = self
            .parameters
            .split([',', '，'])
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .collect();
        self.limits
            .resize(names.len().min(scicanvas_core::nlfit::MAX_PARAMS), Default::default());
        self.limit_values.resize(self.limits.len(), [0.0; 3]);
        // 设置窗口已提供限高滚动；参数共用同一滚动区，窄窗口也可到达末行。
        for ((name, limit), remembered) in names.iter().zip(&mut self.limits).zip(&mut self.limit_values) {
            ui.strong(*name);
            for ((label, value), remembered) in [
                ("固定", &mut limit.fixed),
                ("下限", &mut limit.lower),
                ("上限", &mut limit.upper),
            ]
            .into_iter()
            .zip(remembered.iter_mut())
            {
                ui.horizontal(|ui| {
                    if let Some(value) = value {
                        *remembered = *value;
                    }
                    let mut on = value.is_some();
                    if ui.checkbox(&mut on, label).changed() {
                        *value = on.then_some(*remembered);
                    }
                    if let Some(value) = value {
                        ui.add(egui::DragValue::new(value).speed(0.01));
                    }
                });
            }
        }
    }
    pub fn from_definition(definition: &CustomFitDefinition) -> Self {
        Self {
            target: None,
            loaded_definition: Some(definition.clone()),
            limits: Vec::new(),
            limit_values: Vec::new(),
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
        let unchanged = self.loaded_definition.as_ref().is_some_and(|d| {
            self.formula == d.formula && self.parameters == d.parameters.join(", ") && self.constraints == d.constraints
        });
        let limits = if unchanged {
            &scicanvas_core::formula::FILE_LIMITS
        } else {
            &scicanvas_core::formula::INPUT_LIMITS
        };
        CustomFitDefinition::from_text_with(
            &self.name,
            &self.formula,
            &self.parameters,
            &self.initial,
            &self.constraints,
            limits,
        )
    }

    pub fn sync_saved(&mut self, library: &[SavedFitFunction]) {
        if self.target.is_some() {
            return;
        }
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
            initial,
            target,
            limits,
            constraints,
            weighted,
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
            if matches!(*model,Model::CompetitiveInhibition | Model::NoncompetitiveInhibition | Model::UncompetitiveInhibition)
                && limits.get(3).is_none_or(|limit|!limit.0) {
                ui.colored_label(pal.warn,"I 为抑制剂浓度，需要固定；请用全局拟合同时拟合多个固定浓度。");
            }
            if let Some(target) = target {
                ui.label(format!("更新 Book {} 的结果列 {}", target.report.book_id, target.report.curve.1));
                ui.label("初始值（留空为自动估计）");
                ui.text_edit_singleline(initial);
            }
            ui.add_space(4.0);
            limits.resize(model.params().len(), (false, 0.0, false, 0.0, false, 1.0));
            egui::ScrollArea::vertical().id_salt("builtin-fit-parameters").max_height(300.0).show(ui, |ui| {
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
            });
            ui.add_space(4.0);
            ui.label(RichText::new("参数约束").color(pal.weak));
            ui.add(
                egui::TextEdit::multiline(constraints)
                    .desired_rows(2)
                    .desired_width(f32::INFINITY)
                    .font(egui::TextStyle::Monospace)
                    .hint_text("每行一条，如 A1 + A2 = 1、t2 >= t1"),
            );
            ui.checkbox(weighted, "按 Y 误差加权（1/σ²）")
                .on_hover_text("使用与 Y 列关联的 Y 误差列：误差为空的点跳过，误差为 0 或负数时报错");
            if target.is_none() && ui.button("自定义函数…").clicked() {
                action = Action::OpenCustomNlFit;
            }
            let (ok, cancel) = buttons(ui, "拟合");
            if ok && !matches!(action, Action::OpenCustomNlFit) {
                let held: Vec<_> = model
                    .params()
                    .iter()
                    .zip(limits.iter())
                    .map(|(_, &(fix, fv, has_lo, lo, has_hi, hi))| scicanvas_core::nlfit::ParamLimits {
                        fixed: fix.then_some(fv),
                        lower: (!fix && has_lo).then_some(lo),
                        upper: (!fix && has_hi).then_some(hi),
                    })
                    .collect();
                action = if let Some(target) = target {
                    let values = if initial.trim().is_empty() { Ok(None) } else {
                        initial.split([',', '，']).map(|v| v.trim().parse::<f64>().map_err(|_| "初始值应为逗号分隔的有限数".to_string())).collect::<Result<Vec<_>, _>>().map(Some)
                    };
                    Action::Refit(Box::new(FitEditTarget { report: target.report.clone(), before: target.before.clone() }), values.and_then(|values| target.recipe(scicanvas_core::ops::FitMethod::Builtin(*model), values, held, constraints.clone())).map(|mut recipe| { recipe.weight = scicanvas_core::ops::FitWeight::from_y_error(*weighted); recipe }))
                } else { Action::NlFit(*model, held, constraints.clone(), *weighted) };
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
            if edit.target.is_none() {
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
            }
            ui.label(
                RichText::new(format!(
                    "使用 x 和参数名组成标量表达式，参数初值以逗号分隔。最多 {} 个参数。",
                    scicanvas_core::nlfit::MAX_PARAMS
                ))
                .color(pal.weak),
            );
            if let Some(target) = &edit.target {
                ui.label(format!(
                    "更新 Book {} 的结果列 {}",
                    target.report.book_id, target.report.curve.1
                ));
            }
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
            if edit.target.is_some() {
                edit.show_limits(ui);
            }
            if edit.target.is_none() {
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
            }
            let (ok, cancel) = buttons(ui, "拟合");
            if ok {
                if let Some(target) = &edit.target {
                    let recipe = edit.definition().and_then(|definition| {
                        target.recipe(
                            scicanvas_core::ops::FitMethod::Custom {
                                name: definition.name,
                                formula: definition.formula,
                                parameters: definition.parameters,
                            },
                            Some(definition.initial),
                            edit.limits.clone(),
                            edit.constraints.clone(),
                        )
                    });
                    action = Action::Refit(
                        Box::new(FitEditTarget {
                            report: target.report.clone(),
                            before: target.before.clone(),
                        }),
                        recipe,
                    );
                } else {
                    action = Action::CustomNlFit {
                        name: edit.name.clone(),
                        formula: edit.formula.clone(),
                        parameters: edit.parameters.clone(),
                        initial: edit.initial.clone(),
                        constraints: edit.constraints.clone(),
                    };
                }
            } else if cancel {
                action = Action::Close;
            }
        });
        action
    }
}
