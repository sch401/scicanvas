use super::*;

/// A text focus cycle or continuous pointer edit shares one history transaction.
pub(crate) fn edit_group(response: &egui::Response, group: &mut Option<egui::Id>) {
    let text_focus = response.has_focus() && egui::TextEdit::load_state(&response.ctx, response.id).is_some();
    if text_focus || response.is_pointer_button_down_on() || response.dragged() || response.drag_stopped() {
        *group = Some(response.id);
    }
}

/// 对话框按钮的统一尺寸。
pub(crate) const BUTTON: egui::Vec2 = egui::vec2(84.0, 28.0);

/// 让接下来 `n` 个统一尺寸的按钮在这一行里居中。
pub(crate) fn center_buttons(ui: &mut egui::Ui, n: usize) {
    let gap = ui.spacing().item_spacing.x;
    let used = n as f32 * BUTTON.x + n.saturating_sub(1) as f32 * gap;
    ui.add_space(((ui.available_width() - used) / 2.0).max(0.0));
}

/// 主按钮：强调色底、白字。
pub(crate) fn primary(ui: &egui::Ui, text: &str) -> egui::Button<'static> {
    egui::Button::new(RichText::new(text).color(egui::Color32::WHITE).strong())
        .fill(ui.visuals().selection.stroke.color)
        .min_size(BUTTON)
}

/// 普通按钮。
pub(crate) fn plain(text: impl Into<egui::WidgetText>) -> egui::Button<'static> {
    egui::Button::new(text).min_size(BUTTON)
}

pub(super) fn buttons(ui: &mut egui::Ui, ok: &str) -> (bool, bool) {
    let mut res = (false, false);
    ui.add_space(10.0);
    ui.separator();
    ui.add_space(4.0);
    ui.horizontal_wrapped(|ui| {
        center_buttons(ui, 2);
        res.0 = ui.add(primary(ui, ok)).clicked();
        res.1 = ui.add(plain("取消")).clicked();
    });
    res
}
