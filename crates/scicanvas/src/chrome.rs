//! 无系统标题栏的窗口外框：顶部栏拖动移窗、右上角窗口按钮、边缘调整大小。
use eframe::egui::{self, vec2, Color32, CursorIcon, Rect, ResizeDirection, Sense, Stroke, ViewportCommand};

use crate::theme::Palette;

/// 边缘可拖动调整大小的宽度（逻辑像素）。
const EDGE: f32 = 5.0;
pub const CAPTION_HEIGHT: f32 = 30.0;

pub fn store_caption(ctx: &egui::Context, rect: Rect) {
    ctx.data_mut(|d| d.insert_temp(egui::Id::new("window-caption"), rect));
}

pub fn caption(ctx: &egui::Context) -> Option<Rect> {
    ctx.data(|d| d.get_temp(egui::Id::new("window-caption")))
}

fn maximized(ctx: &egui::Context) -> bool {
    ctx.input(|i| i.viewport().maximized.unwrap_or(false))
}

/// 窗口边缘：鼠标靠近时显示调整大小的光标，按下后交给系统调整大小；最大化时不处理。
/// 同时画一圈细边线，让无边框窗口在桌面上有清楚的轮廓。
pub fn edges(ctx: &egui::Context, pal: &Palette) {
    if maximized(ctx) {
        return;
    }
    let rect = ctx.screen_rect();
    ctx.layer_painter(egui::LayerId::new(
        egui::Order::Foreground,
        egui::Id::new("window-border"),
    ))
    .rect_stroke(rect.shrink(0.5), 0.0, Stroke::new(1.0_f32, pal.line));
    let Some(pos) = ctx.input(|i| i.pointer.hover_pos()) else {
        return;
    };
    let (left, right) = (pos.x - rect.left() < EDGE, rect.right() - pos.x < EDGE);
    let (top, bottom) = (pos.y - rect.top() < EDGE, rect.bottom() - pos.y < EDGE);
    let direction = match (left, right, top, bottom) {
        (true, _, true, _) => ResizeDirection::NorthWest,
        (_, true, true, _) => ResizeDirection::NorthEast,
        (true, _, _, true) => ResizeDirection::SouthWest,
        (_, true, _, true) => ResizeDirection::SouthEast,
        (true, ..) => ResizeDirection::West,
        (_, true, ..) => ResizeDirection::East,
        (_, _, true, _) => ResizeDirection::North,
        (.., true) => ResizeDirection::South,
        _ => return,
    };
    ctx.set_cursor_icon(match direction {
        ResizeDirection::North | ResizeDirection::South => CursorIcon::ResizeVertical,
        ResizeDirection::East | ResizeDirection::West => CursorIcon::ResizeHorizontal,
        ResizeDirection::NorthWest | ResizeDirection::SouthEast => CursorIcon::ResizeNwSe,
        ResizeDirection::NorthEast | ResizeDirection::SouthWest => CursorIcon::ResizeNeSw,
    });
    if ctx.input(|i| i.pointer.primary_pressed()) {
        ctx.send_viewport_cmd(ViewportCommand::BeginResize(direction));
    }
}

/// 顶部栏空白处：拖动移动窗口，双击最大化 / 还原。在顶部栏的其他控件之前调用，控件优先接收点击。
pub fn drag_area(ui: &mut egui::Ui, rect: Rect) {
    let response = ui.interact(rect, egui::Id::new("title-drag"), Sense::click_and_drag());
    if response.double_clicked() {
        let max = maximized(ui.ctx());
        ui.ctx().send_viewport_cmd(ViewportCommand::Maximized(!max));
    } else if response.drag_started_by(egui::PointerButton::Primary) {
        ui.ctx().send_viewport_cmd(ViewportCommand::StartDrag);
    }
}

/// 右上角的最小化、最大化 / 还原、关闭按钮（在从右往左的布局里调用）。
pub fn buttons(ui: &mut egui::Ui, pal: &Palette) -> bool {
    draw_buttons(ui, pal)
}

/// 模态期间只恢复窗口按钮与拖动，不激活标题栏中的项目命令。
pub fn modal_caption(ui: &mut egui::Ui, caption: Rect, pal: &Palette) {
    ui.spacing_mut().item_spacing = vec2(4., 0.);
    drag_area(ui, caption);
    let buttons = Rect::from_min_max(egui::pos2(caption.right() - 3. * 46. - 8., caption.top()), caption.max);
    ui.painter().rect_filled(buttons, 0., pal.chrome);
    ui.allocate_ui_with_layout(caption.size(), egui::Layout::right_to_left(egui::Align::Center), |ui| {
        if draw_buttons(ui, pal) {
            ui.ctx().send_viewport_cmd(ViewportCommand::Close);
        }
    });
}

fn draw_buttons(ui: &mut egui::Ui, pal: &Palette) -> bool {
    let ctx = ui.ctx().clone();
    let max = maximized(&ctx);
    let size = vec2(46.0, CAPTION_HEIGHT);
    let ink = pal.ink;
    let button = |ui: &mut egui::Ui, hover_fill: Color32, tip: &str| {
        let (rect, response) = ui.allocate_exact_size(size, Sense::click());
        let hovered = response.hovered();
        if hovered {
            ui.painter().rect_filled(rect, 0.0, hover_fill);
        }
        (rect, response.on_hover_text(tip), hovered)
    };
    let stroke = |color| Stroke::new(1.0_f32, color);

    // 关闭：悬停时红底白叉
    let (rect, close, hovered) = button(ui, pal.danger, "关闭");
    let c = rect.center();
    let color = if hovered { Color32::WHITE } else { ink };
    for (a, b) in [(vec2(-5.0, -5.0), vec2(5.0, 5.0)), (vec2(-5.0, 5.0), vec2(5.0, -5.0))] {
        ui.painter().line_segment([c + a, c + b], stroke(color));
    }

    // 最大化 / 还原
    let (rect, toggle, _) = button(ui, pal.hover, if max { "向下还原" } else { "最大化" });
    let c = rect.center();
    if max {
        ui.painter().rect_stroke(
            Rect::from_center_size(c + vec2(-1.5, 1.5), vec2(8.0, 8.0)),
            0.0,
            stroke(ink),
        );
        ui.painter()
            .line_segment([c + vec2(-2.0, -4.5), c + vec2(5.5, -4.5)], stroke(ink));
        ui.painter()
            .line_segment([c + vec2(5.5, -4.5), c + vec2(5.5, 3.0)], stroke(ink));
    } else {
        ui.painter()
            .rect_stroke(Rect::from_center_size(c, vec2(10.0, 10.0)), 0.0, stroke(ink));
    }
    if toggle.clicked() {
        ctx.send_viewport_cmd(ViewportCommand::Maximized(!max));
    }

    // 最小化
    let (rect, minimize, _) = button(ui, pal.hover, "最小化");
    let c = rect.center();
    ui.painter()
        .line_segment([c + vec2(-5.0, 0.0), c + vec2(5.0, 0.0)], stroke(ink));
    if minimize.clicked() {
        ctx.send_viewport_cmd(ViewportCommand::Minimized(true));
    }
    close.clicked()
}
