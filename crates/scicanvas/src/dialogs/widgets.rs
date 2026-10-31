use super::*;

/// 文本框连续输入或指针连续编辑共用一个历史事务。
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
        .fill(crate::theme::palette_for(ui.ctx()).accent_fill)
        .min_size(BUTTON)
}

/// 普通按钮。
pub(crate) fn plain(text: impl Into<egui::WidgetText>) -> egui::Button<'static> {
    egui::Button::new(text).min_size(BUTTON)
}

#[derive(Clone, Copy)]
pub(crate) enum ActionKind {
    Primary,
    Plain,
    Danger,
}

/// 统一的确认动作行，文字输入框持有焦点时不接管快捷键。
pub(crate) fn actions(ui: &mut egui::Ui, items: &[(ActionKind, &str)]) -> Option<usize> {
    let pal = crate::theme::palette_for(ui.ctx());
    let mut selected = None;
    let mut ids = Vec::new();
    ui.add_space(10.);
    ui.separator();
    ui.add_space(4.);
    let widths: Vec<_> = items
        .iter()
        .map(|(_, label)| {
            ui.painter()
                .layout_no_wrap((*label).into(), egui::TextStyle::Button.resolve(ui.style()), pal.ink)
                .size()
                .x
                + 2. * ui.spacing().button_padding.x
        })
        .map(|w| w.max(BUTTON.x))
        .collect();
    ui.horizontal_wrapped(|ui| {
        let total = widths.iter().sum::<f32>() + ui.spacing().item_spacing.x * items.len().saturating_sub(1) as f32;
        if total <= ui.available_width() {
            ui.add_space(((ui.available_width() - total) / 2.).max(0.));
        }
        for (index, ((kind, label), width)) in items.iter().zip(widths).enumerate() {
            let button = match kind {
                ActionKind::Primary => primary(ui, label),
                ActionKind::Plain => plain(*label),
                ActionKind::Danger => {
                    egui::Button::new(RichText::new(*label).color(egui::Color32::WHITE).strong()).fill(pal.danger)
                }
            };
            let response = ui.add_sized(egui::vec2(width, BUTTON.y), button);
            ids.push(response.id);
            if response.clicked() {
                selected = Some(index);
            }
        }
    });
    let focus = ui.memory(|m| m.focused());
    if selected.is_none() && (focus.is_none() || focus.is_some_and(|id| ids.contains(&id))) {
        if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter)) {
            selected = items.iter().position(|(k, _)| !matches!(k, ActionKind::Plain));
        }
        if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
            selected = items.iter().rposition(|(k, _)| matches!(k, ActionKind::Plain));
        }
    }
    selected
}
pub(super) fn buttons(ui: &mut egui::Ui, ok: &str) -> (bool, bool) {
    let selected = actions(ui, &[(ActionKind::Primary, ok), (ActionKind::Plain, "取消")]);
    (selected == Some(0), selected == Some(1))
}
