use super::docking::{dock_panel, dock_target, docks, panel_header_height, store_docks, DockPanel};
use super::*;

pub(super) const PANEL_MARGIN: f32 = 8.0;

pub(crate) struct SettingsWindow<'open> {
    window: egui::Window<'open>,
    id: egui::Id,
}

impl<'open> SettingsWindow<'open> {
    pub(crate) fn id(mut self, id: egui::Id) -> Self {
        self.id = id;
        self.window = self.window.id(id);
        self
    }
    pub(crate) fn open(mut self, open: &'open mut bool) -> Self {
        self.window = self.window.open(open);
        self
    }
    pub(crate) fn resizable(mut self, value: bool) -> Self {
        self.window = self.window.resizable(value);
        self
    }
    pub(crate) fn default_width(mut self, width: f32) -> Self {
        self.window = self.window.default_width(width);
        self
    }
    pub(crate) fn default_height(mut self, height: f32) -> Self {
        self.window = self.window.default_height(height);
        self
    }
    pub(crate) fn show<R>(
        mut self,
        ctx: &egui::Context,
        contents: impl FnOnce(&mut egui::Ui) -> R,
    ) -> Option<egui::InnerResponse<Option<R>>> {
        let mut state = docks(ctx);
        if !state.panels.iter().any(|p| p.id == self.id) {
            state.panels.push(DockPanel {
                id: self.id,
                side: None,
                visible: true,
                seen: true,
                rect: None,
                share: 1.0,
                float_pos: None,
                collapsed: false,
                chrome_height: None,
                header_height: None,
            });
        }
        let panel = state.panels.iter_mut().find(|p| p.id == self.id).unwrap();
        panel.seen = true;
        panel.visible = true;
        let docked = panel.side;
        let mut dock_size = None;
        if let Some(rect) = panel.rect {
            let size = (rect.size()
                - egui::vec2(
                    2.0 * PANEL_MARGIN,
                    panel
                        .chrome_height
                        .unwrap_or_else(|| panel_header_height(ctx) + 2.0 * PANEL_MARGIN),
                ))
            .max(egui::vec2(1.0, 1.0));
            dock_size = Some(size);
            self.window = self.window.current_pos(rect.min).fixed_size(size);
        } else if let Some(pos) = panel.float_pos.take() {
            self.window = self.window.current_pos(pos);
        }
        store_docks(ctx, state);
        let out = self.window.show(ctx, |ui| {
            ui.spacing_mut().item_spacing = egui::vec2(6.0, 4.0);
            ui.spacing_mut().slider_width = 100.0;
            ui.spacing_mut().text_edit_width = 200.0;
            if let Some(size) = dock_size {
                ui.set_min_size(size);
            }
            egui::ScrollArea::vertical()
                .auto_shrink([false, dock_size.is_none()])
                .show(ui, contents)
                .inner
        });
        if let Some(out) = &out {
            // The native title consumes clicks for collapsing. Give its middle portion a panel menu,
            // while keeping the native collapse/close buttons and background dragging intact.
            let mut minimized =
                egui::collapsing_header::CollapsingState::load_with_default_open(ctx, self.id.with("collapsing"), true);
            let mut button = out.response.clone();
            button.id = self.id.with("minimize");
            let style = ctx.style();
            let header_height = panel_header_height(ctx);
            let button_size = egui::Vec2::splat(style.spacing.icon_width);
            let close_center = egui::pos2(
                out.response.rect.right() - header_height / 2.0,
                out.response.rect.top() + header_height / 2.0,
            );
            button.rect = egui::Rect::from_center_size(
                close_center - egui::vec2(button_size.x + style.spacing.item_spacing.x, 0.0),
                button_size,
            );
            button.interact_rect = button.rect;
            button.sense = egui::Sense::hover();
            let button = button.interact(egui::Sense::click());
            let painter = ctx.layer_painter(out.response.layer_id);
            let visuals = style.interact(&button);
            let icon_rect = button.rect.shrink(2.0).expand(visuals.expansion);
            let center = icon_rect.center();
            painter.line_segment([icon_rect.left_center(), icon_rect.right_center()], visuals.fg_stroke);
            if !minimized.is_open() {
                painter.line_segment(
                    [
                        egui::pos2(center.x, icon_rect.top()),
                        egui::pos2(center.x, icon_rect.bottom()),
                    ],
                    visuals.fg_stroke,
                );
            }
            let controls_left = button.rect.left() - style.spacing.item_spacing.x;
            if button.clicked() {
                minimized.set_open(!minimized.is_open());
                minimized.store(ctx);
                ctx.request_repaint();
            }
            button.widget_info(|| {
                egui::WidgetInfo::labeled(
                    egui::WidgetType::Button,
                    button.enabled(),
                    if minimized.is_open() {
                        "最小化面板"
                    } else {
                        "还原面板"
                    },
                )
            });
            button.on_hover_text(if minimized.is_open() {
                "最小化面板"
            } else {
                "还原面板"
            });
            let mut title = out.response.clone();
            title.id = self.id.with("dock-title-menu");
            title.rect = egui::Rect::from_min_max(
                out.response.rect.min + egui::vec2(32.0, 0.0),
                egui::pos2(controls_left, out.response.rect.top() + panel_header_height(ctx)),
            );
            title.interact_rect = title.rect;
            title.sense = egui::Sense::hover();
            let title = title.interact(egui::Sense::click());
            if title.double_clicked() {
                let mut collapse = egui::collapsing_header::CollapsingState::load_with_default_open(
                    ctx,
                    self.id.with("collapsing"),
                    true,
                );
                collapse.set_open(!collapse.is_open());
                collapse.store(ctx);
                ctx.request_repaint();
            }
            title.context_menu(|ui| {
                for (label, side) in [
                    ("吸附到左侧", Some(DockSide::Left)),
                    ("吸附到右侧", Some(DockSide::Right)),
                    ("浮动", None),
                ] {
                    if ui.button(label).clicked() {
                        dock_panel(ctx, self.id, side, Some(out.response.rect.min));
                        ui.close_menu();
                    }
                }
            });
            if out.response.dragged()
                && !out.response.drag_started()
                && docked.is_some()
                && ctx.input(|input| input.pointer.delta().length_sq() > 0.0)
            {
                dock_panel(ctx, self.id, None, None);
            }
            if out.response.dragged() || out.response.drag_stopped() {
                if let Some(pos) = ctx.pointer_interact_pos() {
                    if let Some((side, rect)) = dock_target(ctx, &docks(ctx), pos) {
                        let layer = egui::LayerId::new(egui::Order::Foreground, self.id.with("dock-guide"));
                        let painter = ctx.layer_painter(layer);
                        let color = ctx.style().visuals.selection.bg_fill;
                        painter.rect_filled(rect, 0.0, color.gamma_multiply(0.25));
                        painter.rect_stroke(rect, 0.0, egui::Stroke::new(2.0_f32, color));
                        if out.response.drag_stopped() {
                            dock_panel(ctx, self.id, Some(side), Some(pos));
                        }
                    }
                }
            }
            // Collapsing changes the rows reserved by the sidebar on the next frame.
            let mut state = docks(ctx);
            if let Some(panel) = state.panels.iter_mut().find(|p| p.id == self.id) {
                let collapsed = !egui::collapsing_header::CollapsingState::load_with_default_open(
                    ctx,
                    self.id.with("collapsing"),
                    true,
                )
                .is_open();
                if panel.collapsed != collapsed {
                    panel.collapsed = collapsed;
                    ctx.request_repaint();
                }
                if let Some(size) = dock_size.filter(|_| collapsed == out.inner.is_none()) {
                    // Measure native window chrome, including font fallback and DPI rounding.
                    let measured = if collapsed {
                        out.response.rect.height()
                    } else {
                        out.response.rect.height() - size.y
                    };
                    let saved = if collapsed {
                        &mut panel.header_height
                    } else {
                        &mut panel.chrome_height
                    };
                    if saved.is_none_or(|height| (height - measured).abs() > 0.5) {
                        *saved = Some(measured);
                        ctx.request_repaint();
                    }
                }
            }
            store_docks(ctx, state);
        }
        out
    }
}

/// Keep floating titles below the ribbon while leaving the other three sides unrestricted.
pub(crate) fn floating_bounds(ctx: &egui::Context) -> egui::Rect {
    let mut bounds = egui::Rect::EVERYTHING;
    bounds.min.y = ctx.available_rect().top();
    bounds
}

/// Compact single-column settings; drag the title into a sidebar or use the panel context menu.
pub(crate) fn window<'open>(ctx: &egui::Context, title: &str) -> SettingsWindow<'open> {
    let id = egui::Id::new(("scicanvas-dialog", title));
    SettingsWindow {
        id,
        window: egui::Window::new(RichText::new(title).strong())
            .id(id)
            .constrain_to(floating_bounds(ctx))
            .collapsible(true)
            .resizable(true)
            .pivot(Align2::LEFT_TOP)
            .default_pos(egui::pos2(
                ctx.screen_rect().center().x - 190.0,
                ctx.screen_rect().top() + 60.0,
            ))
            .default_width(360.0)
            .min_width(280.0)
            .default_height(ctx.screen_rect().height() * 0.75)
            .max_height((ctx.available_rect().height() - 64.0).max(100.0))
            .vscroll(false)
            .frame(egui::Frame::window(&ctx.style()).inner_margin(egui::Margin::same(PANEL_MARGIN))),
    }
}
