use super::*;

impl Dialog {
    pub(super) fn show_set_values(
        &mut self,
        ctx: &egui::Context,
        pal: &Palette,
        open: &mut bool,
        _esc: bool,
    ) -> Action {
        let Self::SetValues {
            target,
            formula,
            keep,
            columns,
            error,
        } = self
        else {
            unreachable!()
        };
        let mut action = Action::None;
        window(ctx, "设置列值").open(open).show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(format!("{target} =")).strong());
                let edit = ui.add(
                    egui::TextEdit::singleline(formula)
                        .desired_width(340.0)
                        .font(egui::TextStyle::Monospace)
                        .hint_text("例如 A * 2 + sin(B)"),
                );
                if edit.changed() {
                    *error = None;
                }
                if edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    action = Action::SetValues(formula.clone(), *keep);
                }
            });
            if let Some(e) = error {
                ui.label(RichText::new(e.as_str()).color(ui.visuals().error_fg_color));
            }
            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("插入").small().color(pal.weak));
                for (key, name) in columns.iter() {
                    if ui.small_button(key.as_str()).on_hover_text(name.as_str()).clicked() {
                        formula.push_str(key);
                    }
                }
                // 函数目录：按类别列出，点击插入“函数名(”
                ui.menu_button("函数", |ui| {
                    use scicanvas_core::formula::{CATEGORIES, FUNCTIONS};
                    for category in CATEGORIES {
                        ui.menu_button(category, |ui| {
                            egui::ScrollArea::vertical().max_height(360.0).show(ui, |ui| {
                                for func in FUNCTIONS.iter().filter(|f| f.category == category) {
                                    if ui.button(RichText::new(func.usage).monospace()).clicked() {
                                        formula.push_str(func.name);
                                        formula.push('(');
                                        ui.close_menu();
                                    }
                                }
                            });
                        });
                    }
                });
            });
            ui.label(
                RichText::new(
                    "列用短名（A、B 或 col(A)），i 为行号，常数 pi、e；比较 < > <= >= == != 结果为 1 或 0。\n结果覆盖目标列，无效结果留空，可撤销。",
                )
                .small()
                .color(pal.weak),
            );
            ui.checkbox(keep, "保存公式，引用的列变化时自动重算");
            let (ok, cancel) = buttons(ui, "计算");
            if ok {
                action = Action::SetValues(formula.clone(), *keep);
            } else if cancel {
                action = Action::Close;
            }
        });
        action
    }
}
