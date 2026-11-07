use super::docking::{dock_panel, dock_target, docks, panel_header_height, store_docks, DockPanel};
use super::*;

pub(super) const PANEL_MARGIN: f32 = 8.0;

pub(crate) struct SettingsWindow<'open> {
    window: egui::Window<'open>,
    id: egui::Id,
    centered: bool,
    native: egui::ViewportBuilder,
    defaults: [Option<f32>; 2],
    open: Option<&'open mut bool>,
}

impl<'open> SettingsWindow<'open> {
    pub(crate) fn centered(mut self) -> Self {
        self.centered = true;
        self
    }

    pub(crate) fn id(mut self, id: egui::Id) -> Self {
        self.id = id;
        self.window = self.window.id(id);
        self
    }
    pub(crate) fn open(mut self, open: &'open mut bool) -> Self {
        self.open = Some(open);
        self
    }
    pub(crate) fn resizable(mut self, value: bool) -> Self {
        self.window = self.window.resizable(value);
        self.native = self.native.with_resizable(value);
        self
    }
    pub(crate) fn default_width(mut self, width: f32) -> Self {
        self.window = self.window.default_width(width);
        self.defaults[0] = Some(width);
        self
    }
    pub(crate) fn default_height(mut self, height: f32) -> Self {
        self.window = self.window.default_height(height);
        self.defaults[1] = Some(height);
        self
    }
    pub(crate) fn show<R>(
        mut self,
        ctx: &egui::Context,
        mut contents: impl FnMut(&mut egui::Ui) -> R,
    ) -> Option<egui::InnerResponse<Option<R>>> {
        let recenter = ctx
            .data_mut(|d| d.remove_temp::<bool>(self.id.with("recenter")))
            .unwrap_or(false);
        let first_open = self.centered && (recenter || ctx.memory(|memory| memory.area_rect(self.id).is_none()));
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
        let float_pos = if panel.rect.is_none() {
            panel.float_pos.take()
        } else {
            None
        };
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
        } else if let Some(pos) = float_pos {
            self.window = self.window.current_pos(pos);
        }
        store_docks(ctx, state);
        let id = self.id;
        let parent_frame = ctx.cumulative_pass_nr();
        let content = |ui: &mut egui::Ui| {
            let ctx = ui.ctx().clone();
            ui.spacing_mut().item_spacing = egui::vec2(crate::theme::space::CONTROL_GAP, crate::theme::space::XS);
            ui.spacing_mut().slider_width = 100.0;
            ui.spacing_mut().text_edit_width = 200.0;
            if let Some(size) = dock_size {
                ui.set_min_size(size);
            }
            let result = egui::ScrollArea::vertical()
                .auto_shrink([false, dock_size.is_none()])
                .show(ui, &mut contents)
                .inner;
            if ctx.data(|d| d.get_temp::<bool>(egui::Id::new("rendering-dialog-window"))) == Some(true)
                && super::widgets::owns_keyboard(ui)
                && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape))
            {
                ctx.data_mut(|d| d.insert_temp(egui::Id::new("dialog-window-escape"), parent_frame));
            }
            result
        };
        if docked.is_none() && !ctx.embed_viewports() {
            return self.show_native(ctx, content, float_pos);
        }
        if let Some(open) = self.open {
            self.window = self.window.open(open);
        }
        let out = self.window.show(ctx, content);
        if let Some(out) = &out {
            ctx.data_mut(|d| {
                let layers = d.get_temp_mut_or_default::<Vec<egui::LayerId>>(egui::Id::new("settings-layers"));
                layers.push(out.response.layer_id);
            });
            Self::window_header(ctx, id, out, docked, dock_size);
            if first_open && docked.is_none() {
                let mut state = docks(ctx);
                if let Some(panel) = state.panels.iter_mut().find(|panel| panel.id == id) {
                    panel.float_pos = Some(ctx.available_rect().center() - out.response.rect.size() / 2.);
                }
                store_docks(ctx, state);
                ctx.request_repaint();
            }
        }
        out
    }
    fn show_native<R>(
        mut self,
        ctx: &egui::Context,
        mut contents: impl FnMut(&mut egui::Ui) -> R,
        float_pos: Option<egui::Pos2>,
    ) -> Option<egui::InnerResponse<Option<R>>> {
        if self.open.as_deref() == Some(&false) {
            return None;
        }
        let id = self.id;
        let key = id.with("native-builder");
        let main = ctx.input(|i| i.viewport().inner_rect).unwrap_or(ctx.screen_rect());
        self.native = native_builder(
            self.native,
            main,
            self.defaults,
            self.centered,
            float_pos,
            ctx.data(|d| d.get_temp(key)),
        );
        ctx.data_mut(|d| d.insert_temp(key, self.native.clone()));
        let mut embedded = Some(self.window);
        ctx.show_viewport_immediate(egui::ViewportId(id), self.native, |child, class| {
            if class == egui::ViewportClass::Embedded {
                let mut window = embedded.take().unwrap();
                if let Some(open) = self.open.as_deref_mut() {
                    window = window.open(open);
                }
                return window.show(child, &mut contents);
            }
            if child.input(|input| input.viewport().close_requested()) {
                if let Some(open) = self.open.as_deref_mut() {
                    *open = false;
                }
                ctx.request_repaint();
                return None;
            }
            super::widgets::begin_keyboard_frame(child);
            super::widgets::prepare_submission(child);
            egui::TopBottomPanel::top(id.with("native-menu")).show(child, |ui| {
                ui.menu_button("面板", |ui| {
                    for (label, side) in [("停靠到左侧", DockSide::Left), ("停靠到右侧", DockSide::Right)] {
                        if ui.button(label).clicked() {
                            dock_panel(ctx, id, Some(side), None);
                            ctx.request_repaint();
                            ui.close_menu();
                        }
                    }
                });
            });
            let out = egui::CentralPanel::default().show(child, &mut contents);
            Some(egui::InnerResponse {
                inner: Some(out.inner),
                response: out.response,
            })
        })
    }

    fn window_header<R>(
        ctx: &egui::Context,
        id: egui::Id,
        out: &egui::InnerResponse<Option<R>>,
        docked: Option<DockSide>,
        dock_size: Option<egui::Vec2>,
    ) {
        // 标题栏原生控件负责折叠。中段用于打开面板菜单，
        // 保留原生折叠/关闭按钮和拖动标题栏移动窗口的行为。
        let mut minimized =
            egui::collapsing_header::CollapsingState::load_with_default_open(ctx, id.with("collapsing"), true);
        let mut button = out.response.clone();
        button.id = id.with("minimize");
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
        title.id = id.with("dock-title-menu");
        title.rect = egui::Rect::from_min_max(
            out.response.rect.min + egui::vec2(32.0, 0.0),
            egui::pos2(controls_left, out.response.rect.top() + panel_header_height(ctx)),
        );
        title.interact_rect = title.rect;
        title.sense = egui::Sense::hover();
        let title = title.interact(egui::Sense::click());
        if title.double_clicked() {
            let mut collapse =
                egui::collapsing_header::CollapsingState::load_with_default_open(ctx, id.with("collapsing"), true);
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
                if ui.radio(docked == side, label).clicked() {
                    dock_panel(ctx, id, side, Some(out.response.rect.min));
                    ui.close_menu();
                }
            }
        });
        if out.response.dragged()
            && !out.response.drag_started()
            && docked.is_some()
            && ctx.input(|input| input.pointer.delta().length_sq() > 0.0)
        {
            dock_panel(ctx, id, None, None);
        }
        if out.response.dragged() || out.response.drag_stopped() {
            if let Some(pos) = ctx.pointer_interact_pos() {
                if let Some((side, rect)) = dock_target(ctx, &docks(ctx), pos) {
                    let layer = egui::LayerId::new(egui::Order::Foreground, id.with("dock-guide"));
                    let painter = ctx.layer_painter(layer);
                    let color = ctx.style().visuals.selection.bg_fill;
                    painter.rect_filled(rect, 0.0, color.gamma_multiply(0.25));
                    painter.rect_stroke(rect, 0.0, egui::Stroke::new(2.0_f32, color));
                    if out.response.drag_stopped() {
                        dock_panel(ctx, id, Some(side), Some(pos));
                    }
                }
            }
        }
        // 折叠后，侧栏预留的行数在下一帧更新。
        let mut state = docks(ctx);
        if let Some(panel) = state.panels.iter_mut().find(|p| p.id == id) {
            let collapsed =
                !egui::collapsing_header::CollapsingState::load_with_default_open(ctx, id.with("collapsing"), true)
                    .is_open();
            if panel.collapsed != collapsed {
                panel.collapsed = collapsed;
                ctx.request_repaint();
            }
            if let Some(size) = dock_size.filter(|_| collapsed == out.inner.is_none()) {
                // 测量原生窗口边框尺寸，计入字体回退和 DPI 取整。
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
}

/// 紧凑的单列设置面板；拖动标题栏即可吸附到侧栏，也可使用面板菜单。
pub(crate) fn window<'open>(ctx: &egui::Context, title: &str) -> SettingsWindow<'open> {
    let id = egui::Id::new(("scicanvas-dialog", title));
    SettingsWindow {
        id,
        centered: false,
        defaults: [None, None],
        open: None,
        native: egui::ViewportBuilder::default()
            .with_title(title)
            .with_icon(crate::window_icon())
            .with_min_inner_size([280., 120.])
            .with_resizable(true)
            .with_decorations(true)
            .with_taskbar(true),
        window: egui::Window::new(RichText::new(title).strong())
            .id(id)
            .constrain(false)
            .collapsible(true)
            .resizable(true)
            .pivot(Align2::LEFT_TOP)
            .default_pos(egui::pos2(
                ctx.screen_rect().center().x - 190.0,
                ctx.available_rect().top().max(ctx.screen_rect().top() + 60.0),
            ))
            .default_width(crate::theme::dialog::SMALL)
            .min_width(280.0)
            .default_height(ctx.screen_rect().height() * 0.75)
            .max_height((ctx.available_rect().height() - 64.0).max(100.0))
            .vscroll(false)
            .frame(egui::Frame::window(&ctx.style()).inner_margin(egui::Margin::same(PANEL_MARGIN))),
    }
}

pub(crate) fn begin_settings(ctx: &egui::Context) {
    ctx.data_mut(|d| d.insert_temp(egui::Id::new("settings-layers"), Vec::<egui::LayerId>::new()));
}

pub(crate) fn raise_settings(ctx: &egui::Context) {
    for layer in ctx
        .data(|d| d.get_temp::<Vec<egui::LayerId>>(egui::Id::new("settings-layers")))
        .unwrap_or_default()
    {
        ctx.move_to_top(layer);
    }
}

fn native_builder(
    mut builder: egui::ViewportBuilder,
    main: egui::Rect,
    defaults: [Option<f32>; 2],
    centered: bool,
    float_pos: Option<egui::Pos2>,
    cached: Option<egui::ViewportBuilder>,
) -> egui::ViewportBuilder {
    if let Some(cached) = cached {
        builder.inner_size = cached.inner_size;
        builder.position = cached.position;
    } else {
        let size = egui::vec2(
            defaults[0].unwrap_or(crate::theme::dialog::SMALL),
            defaults[1].unwrap_or((main.height() * 0.75).max(240.)),
        );
        builder = builder.with_inner_size(size).with_position(if centered {
            main.center() - size / 2.
        } else {
            main.min + egui::vec2(80., 80.)
        });
    }
    if let Some(pos) = float_pos {
        builder.position = Some(main.min + pos.to_vec2());
    }
    builder
}
#[cfg(test)]
mod native_geometry_tests {
    use super::*;
    #[test]
    fn resizing_main_keeps_native_builder_fixed() {
        let main = egui::Rect::from_min_size(egui::pos2(100., 100.), egui::vec2(1200., 800.));
        let first = native_builder(Default::default(), main, [Some(420.), Some(600.)], false, None, None);
        let next = native_builder(
            Default::default(),
            egui::Rect::from_min_size(main.min, egui::vec2(1600., 1200.)),
            [Some(800.), Some(900.)],
            false,
            None,
            Some(first.clone()),
        );
        assert_eq!(first, next);
    }
    #[test]
    fn centered_and_detached_positions_use_main_screen_coordinates() {
        let main = egui::Rect::from_min_size(egui::pos2(-1500., 100.), egui::vec2(1200., 800.));
        let first = native_builder(Default::default(), main, [None, None], true, None, None);
        assert_eq!(first.position.unwrap() + first.inner_size.unwrap() / 2., main.center());
        assert!(main.contains(first.position.unwrap()));
        let detached = native_builder(
            Default::default(),
            main,
            [None, None],
            false,
            Some(egui::pos2(250., 160.)),
            Some(first),
        );
        assert_eq!(detached.position, Some(egui::pos2(-1250., 260.)));
    }
}
