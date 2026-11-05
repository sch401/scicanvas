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
pub(crate) fn actions(
    ui: &mut egui::Ui,
    items: &[(ActionKind, &str)],
    keys: Keys,
    primary_enabled: Result<(), String>,
) -> Option<usize> {
    let pal = crate::theme::palette_for(ui.ctx());
    let mut selected = None;

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
            let enabled = !matches!(kind, ActionKind::Primary) || primary_enabled.is_ok();
            let response = ui
                .add_enabled_ui(enabled, |ui| ui.add_sized(egui::vec2(width, BUTTON.y), button))
                .inner;
            #[cfg(test)]
            if matches!(kind, ActionKind::Primary) {
                ui.ctx().data_mut(|data| {
                    data.insert_temp(
                        egui::Id::new(("dialog-primary", *label)),
                        (
                            response.rect,
                            egui::WidgetInfo::labeled(egui::WidgetType::Button, response.enabled(), *label),
                        ),
                    )
                });
            }
            if let Err(error) = &primary_enabled {
                if matches!(kind, ActionKind::Primary) {
                    response.clone().on_disabled_hover_text(error);
                }
            }
            if response.clicked()
                && (!matches!(kind, ActionKind::Danger) || response.clicked_by(egui::PointerButton::Primary))
            {
                selected = Some(index);
            }
        }
    });
    if keys == Keys::Modal && selected.is_none() {
        let frame = ui.ctx().cumulative_pass_nr();
        let submit = ui
            .ctx()
            .data(|data| data.get_temp::<(u64, bool, bool)>(egui::Id::new("dialog-enter")))
            .filter(|v| v.0 == frame);
        let (enter, control) = submit.map_or_else(
            || {
                (
                    ui.input(|i| i.key_pressed(egui::Key::Enter)),
                    ui.input(|i| i.modifiers.ctrl),
                )
            },
            |v| (v.1, v.2),
        );
        let multiline = ui
            .ctx()
            .data(|data| data.get_temp::<u64>(egui::Id::new("dialog-multiline")))
            == Some(frame);
        if enter && (control || !multiline) && primary_enabled.is_ok() {
            selected = items.iter().position(|(kind, _)| matches!(kind, ActionKind::Primary));
        }
        if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
            selected = items.iter().rposition(|(kind, _)| matches!(kind, ActionKind::Plain));
        }
    }
    selected
}
pub(super) fn buttons(ui: &mut egui::Ui, ok: &str, primary_enabled: Result<(), String>) -> (bool, bool) {
    if let Some(error) = ui
        .ctx()
        .data(|data| data.get_temp::<Option<String>>(egui::Id::new("dialog-execution-error")))
        .flatten()
    {
        field_error(ui, &error);
    }
    let selected = actions(
        ui,
        &[(ActionKind::Primary, ok), (ActionKind::Plain, "取消")],
        Keys::Modal,
        primary_enabled,
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

pub(crate) struct Form<'a> {
    ui: &'a mut egui::Ui,
    column: usize,
}
impl std::ops::Deref for Form<'_> {
    type Target = egui::Ui;
    fn deref(&self) -> &egui::Ui {
        self.ui
    }
}
impl std::ops::DerefMut for Form<'_> {
    fn deref_mut(&mut self) -> &mut egui::Ui {
        self.ui
    }
}
impl Form<'_> {
    pub(crate) fn label(&mut self, text: impl Into<egui::WidgetText>) -> egui::Response {
        let first = self.column == 0;
        self.column += 1;
        if first {
            self.ui.add_sized(
                egui::vec2(crate::theme::dialog::LABEL_WIDTH, self.ui.spacing().interact_size.y),
                egui::Label::new(text).halign(egui::Align::RIGHT),
            )
        } else {
            self.ui.label(text)
        }
    }
    pub(crate) fn end_row(&mut self) {
        self.column = 0;
        self.ui.end_row();
    }
    pub(crate) fn row<R>(&mut self, label: &str, contents: impl FnOnce(&mut egui::Ui) -> R) -> R {
        self.label(label);
        let result = contents(self.ui);
        self.end_row();
        result
    }
}
pub(crate) fn form<R>(
    ui: &mut egui::Ui,
    id: impl std::hash::Hash,
    contents: impl FnOnce(&mut Form<'_>) -> R,
) -> egui::InnerResponse<R> {
    egui::Grid::new(id)
        .num_columns(2)
        .spacing([crate::theme::space::M, crate::theme::space::S])
        .show(ui, |ui| contents(&mut Form { ui, column: 0 }))
}
pub(crate) fn hint(ui: &mut egui::Ui, text: &str) {
    let pal = crate::theme::palette_for(ui.ctx());
    if text.chars().count() <= 30 {
        ui.add(egui::Label::new(RichText::new(text).small().color(pal.weak)).wrap());
    } else {
        ui.collapsing("详细说明", |ui| {
            ui.add(egui::Label::new(RichText::new(text).small().color(pal.weak)).wrap());
        });
    }
}
pub(crate) fn field_error(ui: &mut egui::Ui, text: &str) -> egui::Response {
    ui.add(
        egui::Label::new(
            RichText::new(text)
                .small()
                .color(crate::theme::palette_for(ui.ctx()).error),
        )
        .wrap(),
    )
}
pub(crate) fn mark_multiline(response: &egui::Response) {
    if response.has_focus() {
        let frame = response.ctx.cumulative_pass_nr();
        response
            .ctx
            .data_mut(|data| data.insert_temp(egui::Id::new("dialog-multiline"), frame));
    }
}
pub(crate) fn multiline(ui: &mut egui::Ui, text: &mut String) -> egui::Response {
    multiline_editor(ui, egui::TextEdit::multiline(text).desired_rows(2))
}
pub(super) fn prepare_submission(ctx: &egui::Context) {
    let control = ctx.input_mut(|input| input.consume_key(egui::Modifiers::CTRL, egui::Key::Enter));
    let enter = control || ctx.input(|input| input.key_pressed(egui::Key::Enter));
    let frame = ctx.cumulative_pass_nr();
    ctx.data_mut(|data| data.insert_temp(egui::Id::new("dialog-enter"), (frame, enter, control)));
}

pub(crate) fn parameter_bounds(limits: &[scicanvas_core::nlfit::ParamLimits]) -> Result<(), String> {
    for (index, limit) in limits.iter().enumerate() {
        if [limit.fixed, limit.lower, limit.upper]
            .into_iter()
            .flatten()
            .any(|v| !v.is_finite())
        {
            return Err(format!("第 {} 个参数的限制应为有限数", index + 1));
        }
        if limit.lower.zip(limit.upper).is_some_and(|(a, b)| a > b) {
            return Err(format!("第 {} 个参数的下限应小于或等于上限", index + 1));
        }
        if limit
            .fixed
            .is_some_and(|v| limit.lower.is_some_and(|a| v < a) || limit.upper.is_some_and(|b| v > b))
        {
            return Err(format!("第 {} 个参数的固定值应在限制范围内", index + 1));
        }
    }
    Ok(())
}

pub(crate) fn multiline_editor(ui: &mut egui::Ui, editor: egui::TextEdit<'_>) -> egui::Response {
    let response = ui.add(editor);
    #[cfg(test)]
    ui.ctx()
        .data_mut(|data| data.insert_temp(egui::Id::new("dialog-multiline-rect"), response.rect));
    mark_multiline(&response);
    response
}
