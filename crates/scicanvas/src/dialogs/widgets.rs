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
    Modal,
}

/// Enter / Esc 只作用于帧开始时拥有焦点的窗口；无焦点时只作用于指针所在窗口。危险按钮仍需点击。
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
    if keys == Keys::Modal && selected.is_none() && owns_keyboard(ui) {
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
    value: Option<egui::Ui>,
    column: usize,
    row: usize,
    left: f32,
    width: f32,
    top: f32,
    label_bottom: f32,
    background: Option<egui::layers::ShapeIdx>,
}
impl std::ops::Deref for Form<'_> {
    type Target = egui::Ui;
    fn deref(&self) -> &egui::Ui {
        self.value.as_ref().unwrap_or(self.ui)
    }
}
impl std::ops::DerefMut for Form<'_> {
    fn deref_mut(&mut self) -> &mut egui::Ui {
        if self.column == 0 {
            self.ui
        } else {
            self.value_ui()
        }
    }
}
impl Form<'_> {
    fn value_ui(&mut self) -> &mut egui::Ui {
        if self.value.is_none() {
            let x = self.left + crate::theme::dialog::LABEL_WIDTH + crate::theme::space::M;
            let width = (self.left + self.width - x).max(1.0);
            let mut value = self.ui.new_child(
                egui::UiBuilder::new()
                    .id_salt(("form-value", self.row))
                    .max_rect(egui::Rect::from_min_size(
                        egui::pos2(x, self.top),
                        egui::vec2(width, f32::INFINITY),
                    ))
                    .layout(egui::Layout::top_down(egui::Align::Min)),
            );
            value.set_min_width(width);
            value.spacing_mut().item_spacing.y = crate::theme::space::XS;
            self.value = Some(value);
        }
        self.value.as_mut().unwrap()
    }
    pub(crate) fn label(&mut self, text: impl Into<egui::WidgetText>) -> egui::Response {
        if self.column != 0 {
            return self.value_ui().label(text);
        }
        self.column = 1;
        self.top = self.ui.cursor().top();
        self.background = Some(self.ui.painter().add(egui::Shape::Noop));
        let mut label = self.ui.new_child(
            egui::UiBuilder::new()
                .id_salt(("form-label", self.row))
                .max_rect(egui::Rect::from_min_size(
                    egui::pos2(self.left, self.top),
                    egui::vec2(crate::theme::dialog::LABEL_WIDTH, self.ui.spacing().interact_size.y),
                ))
                .layout(egui::Layout::top_down(egui::Align::RIGHT)),
        );
        let response = label.add_sized(
            egui::vec2(crate::theme::dialog::LABEL_WIDTH, self.ui.spacing().interact_size.y),
            egui::Label::new(text).wrap().halign(egui::Align::RIGHT),
        );
        self.label_bottom = label.min_rect().bottom();
        response
    }
    pub(crate) fn end_row(&mut self) {
        if self.column == 0 {
            return;
        }
        let bottom = self
            .value
            .take()
            .map_or(self.label_bottom, |value| value.min_rect().bottom())
            .max(self.label_bottom)
            .max(self.top + self.ui.spacing().interact_size.y);
        let rect = egui::Rect::from_min_max(
            egui::pos2(self.left, self.top),
            egui::pos2(self.left + self.width, bottom),
        );
        if self.row % 2 == 1 && self.ui.visuals().striped {
            if let Some(background) = self.background.take() {
                self.ui.painter().set(
                    background,
                    egui::Shape::rect_filled(
                        rect.expand2(egui::vec2(0., crate::theme::space::S / 2.)),
                        2.,
                        self.ui.visuals().faint_bg_color,
                    ),
                );
            }
        }
        #[cfg(test)]
        self.ui
            .data_mut(|data| data.insert_temp(egui::Id::new(("responsive-form-row", self.ui.id(), self.row)), rect));
        self.ui.advance_cursor_after_rect(rect);
        self.ui.add_space(crate::theme::space::S);
        self.column = 0;
        self.row += 1;
    }
    pub(crate) fn row<R>(&mut self, label: &str, contents: impl FnOnce(&mut egui::Ui) -> R) -> R {
        self.label(label);
        let result = contents(self.value_ui());
        self.end_row();
        result
    }
}
pub(crate) fn form<R>(
    ui: &mut egui::Ui,
    id: impl std::hash::Hash,
    contents: impl FnOnce(&mut Form<'_>) -> R,
) -> egui::InnerResponse<R> {
    ui.scope_builder(
        egui::UiBuilder::new()
            .id_salt(id)
            .layout(egui::Layout::top_down(egui::Align::Min)),
        |ui| {
            let left = ui.cursor().left();
            let width = ui.available_width();
            ui.spacing_mut().item_spacing.y = 0.;
            let mut form = Form {
                ui,
                value: None,
                column: 0,
                row: 0,
                left,
                width,
                top: 0.,
                label_bottom: 0.,
                background: None,
            };
            let result = contents(&mut form);
            form.end_row();
            result
        },
    )
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
pub(crate) fn begin_keyboard_frame(ctx: &egui::Context) {
    let frame = ctx.cumulative_pass_nr();
    let focus = ctx.memory(|memory| memory.focused());
    ctx.data_mut(|data| data.insert_temp(egui::Id::new("dialog-frame-focus"), (frame, focus)));
}
pub(super) fn owns_keyboard(ui: &egui::Ui) -> bool {
    if ui
        .ctx()
        .data(|d| d.get_temp::<(u64, egui::LayerId)>(egui::Id::new("keyboard-modal-layer")))
        == Some((ui.ctx().cumulative_pass_nr(), ui.layer_id()))
    {
        return true;
    }
    let focus = ui
        .ctx()
        .data(|data| data.get_temp::<(u64, Option<egui::Id>)>(egui::Id::new("dialog-frame-focus")))
        .and_then(|(_, focus)| focus);
    match focus {
        Some(id) => ui.ctx().read_response(id).is_some_and(|r| r.layer_id == ui.layer_id()),
        None => ui
            .ctx()
            .pointer_hover_pos()
            .is_some_and(|p| ui.ctx().layer_id_at(p) == Some(ui.layer_id())),
    }
}
pub(super) fn prepare_submission(ctx: &egui::Context) {
    let frame = ctx.cumulative_pass_nr();
    if ctx
        .data(|data| data.get_temp::<(u64, Option<egui::Id>)>(egui::Id::new("dialog-frame-focus")))
        .is_none_or(|v| v.0 != frame)
    {
        begin_keyboard_frame(ctx);
    }
    let control = ctx.input_mut(|input| input.consume_key(egui::Modifiers::CTRL, egui::Key::Enter));
    let enter = control || ctx.input(|input| input.key_pressed(egui::Key::Enter));
    let frame = ctx.cumulative_pass_nr();
    ctx.data_mut(|data| data.insert_temp(egui::Id::new("dialog-enter"), (frame, enter, control)));
}

pub(crate) fn effective_limits(
    limits: &[scicanvas_core::nlfit::ParamLimits],
) -> Vec<scicanvas_core::nlfit::ParamLimits> {
    limits
        .iter()
        .map(|limit| {
            if limit.fixed.is_some() {
                scicanvas_core::nlfit::ParamLimits {
                    lower: None,
                    upper: None,
                    ..*limit
                }
            } else {
                *limit
            }
        })
        .collect()
}

pub(crate) fn parameter_bounds(limits: &[scicanvas_core::nlfit::ParamLimits]) -> Result<(), String> {
    for (index, limit) in effective_limits(limits).iter().enumerate() {
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

pub(crate) fn odd_window(ui: &mut egui::Ui, value: &mut usize) -> egui::Response {
    let response = ui.add(egui::DragValue::new(value).range(3..=101).speed(2.));
    let notice = response.id.with("odd-window-adjusted");
    if response.changed() {
        let adjusted = (*value).is_multiple_of(2);
        if adjusted {
            *value = (*value + 1).min(101);
        }
        ui.data_mut(|d| d.insert_temp(notice, adjusted));
    }
    if ui.data(|d| d.get_temp::<bool>(notice)) == Some(true) {
        hint(ui, "窗口点数已调整为奇数。");
    }
    response
}
pub(crate) fn live_hint(ui: &mut egui::Ui) {
    hint(ui, "修改即时生效，可撤销（Ctrl+Z）");
}

pub(crate) fn numeric_step(value: f64) -> f64 {
    if value.is_finite() && value != 0. {
        10_f64.powf(value.abs().log10().floor()) / 100.
    } else {
        0.01
    }
}
pub(crate) fn parameter_table(
    ui: &mut egui::Ui,
    id: &str,
    names: &[&str],
    limits: &mut [scicanvas_core::nlfit::ParamLimits],
) {
    egui::Grid::new(id).striped(true).num_columns(4).show(ui, |ui| {
        for label in ["参数", "固定", "下限", "上限"] {
            ui.label(label);
        }
        ui.end_row();
        for (name, limit) in names.iter().zip(limits) {
            ui.label(*name);
            let mut fixed = limit.fixed.is_some();
            for (column, field) in [&mut limit.fixed, &mut limit.lower, &mut limit.upper]
                .into_iter()
                .enumerate()
            {
                ui.add_enabled_ui(column == 0 || !fixed, |ui| {
                    let key = ui.id().with((id, name, column));
                    let mut value = field.unwrap_or_else(|| ui.data(|d| d.get_temp::<f64>(key)).unwrap_or(0.));
                    let mut on = field.is_some();
                    let response = ui.checkbox(&mut on, "");
                    #[cfg(test)]
                    ui.ctx().data_mut(|d| {
                        d.insert_temp(egui::Id::new(("parameter-toggle", id, *name, column)), response.rect)
                    });
                    let _ = response;
                    let step = numeric_step(value);
                    ui.add_enabled(
                        on,
                        egui::DragValue::new(&mut value)
                            .speed(step)
                            .custom_formatter(|value, _| {
                                if value != 0. && (value.abs() < 0.001 || value.abs() >= 1e6) {
                                    format!("{value:.3e}")
                                } else {
                                    scicanvas_core::fmt_num(value)
                                }
                            }),
                    );
                    ui.data_mut(|d| d.insert_temp(key, value));
                    if column == 0 {
                        fixed = on;
                    }
                    *field = (on && (column == 0 || !fixed)).then_some(value);
                });
            }
            ui.end_row();
        }
    });
}
pub(crate) fn weighted_checkbox(ui: &mut egui::Ui, weighted: &mut bool, label: &str, available: bool) {
    if !available {
        *weighted = false;
    }
    let response = ui.add_enabled(available, egui::Checkbox::new(weighted, label));
    #[cfg(test)]
    ui.ctx().data_mut(|d| {
        d.insert_temp(
            egui::Id::new(("weighted-control", label)),
            (response.rect, response.enabled()),
        )
    });
    response
        .on_hover_text("误差为空的点跳过，误差为 0 或负数时报错")
        .on_disabled_hover_text("数据未关联 Y 误差列");
}

pub(crate) const CONSTRAINT_HINT: &str = "每行一条，如 a >= 0 或 a + b = 1";

#[cfg(test)]
mod review_tests {
    use super::*;
    use scicanvas_core::nlfit::ParamLimits;

    #[test]
    fn form_rows_fill_width_and_grow_when_controls_wrap() {
        let ctx = egui::Context::default();
        crate::theme::install(&ctx);
        crate::fonts::setup(&ctx);
        let mut heights = Vec::new();
        for width in [330., 680., 330.] {
            let mut rows = Vec::new();
            let _ = ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(width, 500.))),
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        form(ui, "responsive-test", |f| {
                            let id = f.ui.id();
                            let expected = f.width;
                            f.row("Margins", |ui| {
                                ui.horizontal_wrapped(|ui| {
                                    for name in ["Left", "Top", "Right", "Bottom"] {
                                        ui.add_sized([90., 28.], egui::Button::new(name));
                                    }
                                });
                            });
                            f.row("Background", |ui| {
                                let _ = ui.button("Color");
                            });
                            for row in 0_usize..2 {
                                let rect = ctx
                                    .data(|d| d.get_temp::<egui::Rect>(egui::Id::new(("responsive-form-row", id, row))))
                                    .unwrap();
                                assert!((rect.width() - expected).abs() < 0.1);
                                rows.push(rect);
                            }
                        });
                    });
                },
            );
            assert!(rows[1].top() > rows[0].bottom());
            heights.push(rows[0].height());
        }
        assert!(heights[0] > heights[1]);
        assert_eq!(heights[0], heights[2]);
    }

    #[test]
    fn p11_fixed_parameters_disable_and_ignore_bounds() {
        let ctx = egui::Context::default();
        crate::fonts::setup(&ctx);
        let mut limits = vec![ParamLimits {
            fixed: Some(2.),
            lower: Some(9.),
            upper: Some(1.),
        }];
        assert!(parameter_bounds(&limits).is_ok());
        let effective = effective_limits(&limits);
        assert_eq!(effective[0].fixed, Some(2.));
        assert_eq!(effective[0].lower, None);
        assert_eq!(effective[0].upper, None);
        let mut h = crate::test_ui::Harness::new(
            ctx.clone(),
            Some(egui::vec2(600., 300.)),
            |limits: &mut Vec<ParamLimits>, ctx: &egui::Context| {
                egui::CentralPanel::default().show(ctx, |ui| parameter_table(ui, "review", &["a"], limits));
            },
        );
        h.frame(&mut limits, vec![]);
        let lower = ctx
            .data(|d| d.get_temp::<egui::Rect>(egui::Id::new(("parameter-toggle", "review", "a", 1_usize))))
            .unwrap();
        h.click(&mut limits, lower.center());
        assert_eq!(limits[0].lower, None);
        assert_eq!(limits[0].upper, None);
    }
}
