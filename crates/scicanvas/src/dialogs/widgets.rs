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
    let pal = crate::theme::palette_for(ui.ctx());
    egui::Button::new(RichText::new(text).color(pal.on_accent).strong())
        .fill(pal.accent_fill)
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

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Keys {
    None,
    Modal,
}

/// 普通面板不接管键盘；模态 Enter 只触发主按钮，危险按钮需要点击或先获得焦点。
pub(crate) fn actions(ui: &mut egui::Ui, items: &[(ActionKind, &str)], keys: Keys) -> Option<usize> {
    let pal = crate::theme::palette_for(ui.ctx());
    let mut selected = None;
    let mut ids = Vec::new();
    ui.add_space(crate::theme::space::M);
    ui.separator();
    ui.add_space(crate::theme::space::XS);
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
                    egui::Button::new(RichText::new(*label).color(pal.on_accent).strong()).fill(pal.danger)
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
    if keys == Keys::Modal && selected.is_none() && (focus.is_none() || focus.is_some_and(|id| ids.contains(&id))) {
        if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter)) {
            selected = items.iter().position(|(k, _)| matches!(k, ActionKind::Primary));
        }
        if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
            selected = items.iter().rposition(|(k, _)| matches!(k, ActionKind::Plain));
        }
    }
    selected
}
pub(super) fn buttons(ui: &mut egui::Ui, ok: &str) -> (bool, bool) {
    let selected = actions(
        ui,
        &[(ActionKind::Primary, ok), (ActionKind::Plain, "取消")],
        Keys::None,
    );
    (selected == Some(0), selected == Some(1))
}

pub(super) fn data_hint(ui: &mut egui::Ui) {
    let caption = ui
        .ctx()
        .data(|data| data.get_temp::<Option<String>>(egui::Id::new("analysis-data-caption")))
        .flatten();
    if let Some(caption) = caption {
        ui.weak(caption);
    }
}

pub(crate) fn axis_scale_options(ui: &mut egui::Ui, scale: &mut scicanvas_core::ticks::AxisScale) {
    for kind in scicanvas_core::ticks::AxisScale::ALL {
        ui.selectable_value(scale, kind, kind.label());
    }
}
