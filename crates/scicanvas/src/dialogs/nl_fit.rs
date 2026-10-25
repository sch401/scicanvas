use super::*;

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
    ) -> Action {
        let Self::CustomNlFit {
            name,
            formula,
            parameters,
            initial,
            constraints,
        } = self
        else {
            unreachable!()
        };
        let mut action = Action::None;
        window(ctx, "自定义非线性拟合").open(open).show(ctx, |ui| {
            ui.label(
                RichText::new("使用 x 和参数名组成标量表达式，参数初值以逗号分隔。最多 7 个参数。").color(pal.weak),
            );
            ui.label("函数名");
            ui.text_edit_singleline(name);
            ui.label("表达式");
            ui.add(
                egui::TextEdit::multiline(formula)
                    .desired_rows(2)
                    .desired_width(f32::INFINITY)
                    .font(egui::TextStyle::Monospace)
                    .hint_text("y = a*exp(-b*x)+c"),
            );
            ui.label("参数名");
            ui.text_edit_singleline(parameters);
            ui.label("初始值");
            ui.text_edit_singleline(initial);
            ui.label("参数约束");
            ui.add(
                egui::TextEdit::multiline(constraints)
                    .desired_rows(2)
                    .desired_width(f32::INFINITY)
                    .font(egui::TextStyle::Monospace)
                    .hint_text("每行一条，如 a >= 0"),
            );
            let (ok, cancel) = buttons(ui, "拟合");
            if ok {
                action = Action::CustomNlFit {
                    name: name.clone(),
                    formula: formula.clone(),
                    parameters: parameters.clone(),
                    initial: initial.clone(),
                    constraints: constraints.clone(),
                };
            } else if cancel {
                action = Action::Close;
            }
        });
        action
    }
}
