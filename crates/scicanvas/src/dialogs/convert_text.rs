use super::*;

impl Dialog {
    pub(super) fn show_convert_text(&mut self, ctx: &egui::Context, pal: &Palette, open: &mut bool) -> Action {
        let Self::ConvertText { book, key, row, value } = self else {
            unreachable!()
        };
        let mut action = Action::None;
        window(ctx, "转换为文本列？").centered().open(open).show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(4.0);
                ui.label(RichText::new(format!("列 {key} 第 {} 行的输入与当前列类型不符", *row + 1)).strong());
                ui.add_space(4.0);
                ui.label(RichText::new("已有内容保留。").color(pal.weak));
            });
            ui.add_space(10.0);
            ui.separator();
            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                // 主按钮文字较长，按实际宽度居中
                let gap = ui.spacing().item_spacing.x;
                let wide = egui::vec2(132.0, BUTTON.y);
                ui.add_space(((ui.available_width() - wide.x - BUTTON.x - gap) / 2.0).max(0.0));
                if ui.add(primary(ui, "转换为文本并写入").min_size(wide)).clicked() {
                    action = Action::ConvertText(*book, key.clone(), *row, value.clone());
                }
                if ui.add(plain("取消")).clicked()
                    || (super::widgets::owns_keyboard(ui)
                        && ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)))
                {
                    action = Action::Close;
                }
            });
        });
        action
    }
}
