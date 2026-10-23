//! 多文档子窗口（Origin 的 MDI 工作区）：标题栏带最小化、最大化 / 还原、关闭按钮，
//! 拖动标题栏移动，双击标题栏最大化 / 还原，拖动右下角或右边、下边改变大小。
//! 最大化铺满工作区，最小化后排在工作区底部。
//! 和 Windows 11 一样贴靠：把标题栏拖到工作区左右边缘占半边，拖到四角占四分之一，
//! 拖到顶边最大化；拖动时显示预览框。贴靠后的窗口拖开时恢复原来的大小，
//! 拖动最大化窗口的标题栏会先还原再跟随鼠标。贴到半边或角落后由界面层显示贴靠助手，
//! 用其余窗口填满剩下的位置（见 `assist_zones`）。

use eframe::egui::{self, pos2, vec2, Color32, CursorIcon, Mesh, Pos2, Rect, Sense, Shadow, Stroke, UiBuilder};

use crate::icons;
use crate::theme::Palette;

pub const TITLE_H: f32 = 24.0;
const MIN_W: f32 = 170.0;
const MIN_SIZE: egui::Vec2 = egui::Vec2::new(220.0, 140.0);
/// 鼠标离工作区边缘多近时触发贴靠。
const SNAP_EDGE: f32 = 8.0;
/// 靠近边缘时，离角落多近算作角落（四分之一）贴靠。
const SNAP_CORNER: f32 = 80.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WinState {
    Normal,
    Max,
    Min,
}

/// 子窗口的位置和状态。rect 相对于工作区左上角，这样侧栏宽度变化时窗口不会乱跑。
#[derive(Clone, Copy, Debug)]
pub struct Geom {
    pub rect: Rect,
    pub state: WinState,
    /// 贴靠前的大小；贴靠后的窗口被拖开时恢复成这个大小。
    pub restore: Option<egui::Vec2>,
}

impl Geom {
    pub fn new(pos: Pos2, size: egui::Vec2) -> Self {
        Geom {
            rect: Rect::from_min_size(pos, size),
            state: WinState::Normal,
            restore: None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Snap {
    Max,
    Left,
    Right,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

/// 仅在工作区边缘附近贴靠，拖远到工作区外时保持自由位置。
pub fn snap_at(ws: Rect, p: Pos2) -> Option<Snap> {
    if !ws.expand(SNAP_EDGE).contains(p) {
        return None;
    }
    let (left, right) = (p.x <= ws.left() + SNAP_EDGE, p.x >= ws.right() - SNAP_EDGE);
    let (top, bottom) = (p.y <= ws.top() + SNAP_EDGE, p.y >= ws.bottom() - SNAP_EDGE);
    let (near_top, near_bottom) = (p.y <= ws.top() + SNAP_CORNER, p.y >= ws.bottom() - SNAP_CORNER);
    let (near_left, near_right) = (p.x <= ws.left() + SNAP_CORNER, p.x >= ws.right() - SNAP_CORNER);
    Some(if (left && near_top) || (top && near_left) {
        Snap::TopLeft
    } else if (right && near_top) || (top && near_right) {
        Snap::TopRight
    } else if (left && near_bottom) || (bottom && near_left) {
        Snap::BottomLeft
    } else if (right && near_bottom) || (bottom && near_right) {
        Snap::BottomRight
    } else if top {
        Snap::Max
    } else if left {
        Snap::Left
    } else if right {
        Snap::Right
    } else {
        return None;
    })
}

/// 贴靠后的窗口矩形，相对于工作区左上角。
pub fn snap_rect(ws: Rect, snap: Snap) -> Rect {
    let (w, h) = (ws.width(), ws.height());
    let (hw, hh) = ((w / 2.0).round(), (h / 2.0).round());
    let r = |x: f32, y: f32, rw: f32, rh: f32| Rect::from_min_size(pos2(x, y), vec2(rw, rh));
    match snap {
        Snap::Max => r(0.0, 0.0, w, h),
        Snap::Left => r(0.0, 0.0, hw, h),
        Snap::Right => r(hw, 0.0, w - hw, h),
        Snap::TopLeft => r(0.0, 0.0, hw, hh),
        Snap::TopRight => r(hw, 0.0, w - hw, hh),
        Snap::BottomLeft => r(0.0, hh, hw, h - hh),
        Snap::BottomRight => r(hw, hh, w - hw, h - hh),
    }
}

/// 贴靠后留给贴靠助手依次填充的位置：半边贴靠剩另一半，四分之一贴靠剩其余三格。
pub fn assist_zones(snap: Snap) -> Vec<Snap> {
    match snap {
        Snap::Max => Vec::new(),
        Snap::Left => vec![Snap::Right],
        Snap::Right => vec![Snap::Left],
        quarter => [Snap::TopLeft, Snap::TopRight, Snap::BottomLeft, Snap::BottomRight]
            .into_iter()
            .filter(|q| *q != quarter)
            .collect(),
    }
}

/// 让鼠标在标题栏上的相对位置保持不变，把窗口换成 `size` 大小（相对工作区坐标）。
fn follow_pointer(ws: Rect, p: Pos2, from: Rect, size: egui::Vec2) -> Rect {
    let frac = ((p.x - from.left()) / from.width().max(1.0)).clamp(0.0, 1.0);
    Rect::from_min_size(
        pos2(p.x - ws.left() - frac * size.x, p.y - ws.top() - TITLE_H / 2.0),
        size,
    )
}

fn minimized_rect(ws: Rect, slot: usize) -> Rect {
    let columns = (ws.width() / (MIN_W + 4.0)).floor().max(1.0) as usize;
    Rect::from_min_size(
        pos2(
            ws.left() + 4.0 + (slot % columns) as f32 * (MIN_W + 4.0),
            (ws.bottom() - TITLE_H - 4.0 - (slot / columns) as f32 * (TITLE_H + 4.0)).max(ws.top()),
        ),
        vec2(MIN_W, TITLE_H),
    )
}

pub fn layer_id(id: egui::Id) -> egui::LayerId {
    egui::LayerId::new(egui::Order::Middle, id)
}

#[derive(Default)]
pub struct WinResponse {
    pub close: bool,
    pub state: Option<WinState>,
    /// 标题栏被按下（用于激活窗口）。
    pub pressed: bool,
    /// 这一帧松开标题栏时贴到了半边或角落（最大化不算）。
    pub snapped: Option<Snap>,
}

#[derive(Clone, Copy, PartialEq)]
enum Btn {
    Min,
    Max,
    Close,
}

/// 横向渐变填充（当前窗口的标题栏）。
fn gradient(painter: &egui::Painter, r: Rect, left: Color32, right: Color32) {
    let mut m = Mesh::default();
    m.colored_vertex(r.left_top(), left);
    m.colored_vertex(r.right_top(), right);
    m.colored_vertex(r.right_bottom(), right);
    m.colored_vertex(r.left_bottom(), left);
    m.add_triangle(0, 1, 2);
    m.add_triangle(0, 2, 3);
    painter.add(m);
}

fn title_button(ui: &mut egui::Ui, id: egui::Id, r: Rect, kind: Btn, state: WinState, fg: Color32) -> egui::Response {
    let resp = ui.interact(r, id, Sense::click());
    let p = ui.painter();
    if resp.hovered() {
        let bg = if kind == Btn::Close {
            Color32::from_rgb(0xC4, 0x2B, 0x1C)
        } else {
            Color32::from_white_alpha(56)
        };
        p.rect_filled(r, 2.0, bg);
    }
    let c = r.center();
    let s = Stroke::new(
        1.2_f32,
        if resp.hovered() && kind == Btn::Close {
            Color32::WHITE
        } else {
            fg
        },
    );
    match kind {
        Btn::Min => {
            p.line_segment([pos2(c.x - 4.5, c.y + 3.5), pos2(c.x + 4.5, c.y + 3.5)], s);
        }
        Btn::Max if state == WinState::Max => {
            // 还原：两个错开的方框
            p.rect_stroke(Rect::from_min_size(pos2(c.x - 4.5, c.y - 2.0), vec2(6.5, 6.5)), 0.0, s);
            p.line_segment([pos2(c.x - 2.0, c.y - 2.0), pos2(c.x - 2.0, c.y - 4.5)], s);
            p.line_segment([pos2(c.x - 2.0, c.y - 4.5), pos2(c.x + 4.5, c.y - 4.5)], s);
            p.line_segment([pos2(c.x + 4.5, c.y - 4.5), pos2(c.x + 4.5, c.y + 2.0)], s);
            p.line_segment([pos2(c.x + 4.5, c.y + 2.0), pos2(c.x + 2.0, c.y + 2.0)], s);
        }
        Btn::Max if state == WinState::Min => {
            // 最小化状态下这个按钮是“还原”：一个方框
            p.rect_stroke(Rect::from_center_size(c, vec2(8.0, 8.0)), 0.0, s);
        }
        Btn::Max => {
            p.rect_stroke(Rect::from_center_size(c, vec2(9.0, 8.0)), 0.0, s);
            p.line_segment([pos2(c.x - 4.5, c.y - 2.5), pos2(c.x + 4.5, c.y - 2.5)], s);
        }
        Btn::Close => {
            p.line_segment([pos2(c.x - 4.0, c.y - 4.0), pos2(c.x + 4.0, c.y + 4.0)], s);
            p.line_segment([pos2(c.x + 4.0, c.y - 4.0), pos2(c.x - 4.0, c.y + 4.0)], s);
        }
    }
    let tip = match (kind, state) {
        (Btn::Min, _) => "最小化",
        (Btn::Max, WinState::Normal) => "最大化",
        (Btn::Max, _) => "还原",
        (Btn::Close, _) => "隐藏窗口（可在项目管理器中重新打开）",
    };
    resp.on_hover_text(tip)
}

/// 画一个子窗口。`ws` 是工作区矩形，`min_slot` 是最小化时排在底部的第几个。
#[allow(clippy::too_many_arguments)]
pub fn window(
    ctx: &egui::Context,
    id: egui::Id,
    ws: Rect,
    geom: &mut Geom,
    min_slot: usize,
    title: &str,
    icon: &str,
    active: bool,
    pal: &Palette,
    add_body: impl FnOnce(&mut egui::Ui),
) -> WinResponse {
    let mut out = WinResponse::default();
    if geom.state == WinState::Normal {
        geom.rect = geom.rect.translate(vec2(0.0, (-geom.rect.top()).max(0.0)));
    }
    let rect = match geom.state {
        WinState::Max => ws,
        WinState::Min => minimized_rect(ws, min_slot),
        WinState::Normal => geom.rect.translate(ws.min.to_vec2()),
    };

    egui::Area::new(id)
        .order(egui::Order::Middle)
        .fixed_pos(rect.min)
        .constrain(false)
        .show(ctx, |ui| {
            ui.set_clip_rect(ws);
            let (_, _) = ui.allocate_exact_size(rect.size(), Sense::hover());
            let painter = ui.painter().clone();

            // 贴靠预览画在本窗口图层里、窗口本体之前：压在其他窗口之上，被拖动的窗口之下，与 Windows 11 一致。
            if geom.state != WinState::Min && ctx.is_being_dragged(id.with("title")) {
                if let Some(snap) = ctx.input(|i| i.pointer.latest_pos()).and_then(|p| snap_at(ws, p)) {
                    let preview = snap_rect(ws, snap).translate(ws.min.to_vec2()).shrink(4.0);
                    painter.rect_filled(preview, 6.0, pal.title.gamma_multiply(0.22));
                    painter.rect_stroke(preview, 6.0, Stroke::new(2.0_f32, pal.title.gamma_multiply(0.8)));
                }
            }

            // 阴影、边框、底色
            if geom.state != WinState::Max {
                painter.add(
                    Shadow {
                        offset: vec2(2.0, 3.0),
                        blur: 10.0,
                        spread: 0.0,
                        color: Color32::from_black_alpha(if pal.dark { 110 } else { 50 }),
                    }
                    .as_shape(rect, 0.0),
                );
            }
            painter.rect_filled(rect, 0.0, pal.window);

            // 标题栏
            let bar = Rect::from_min_size(rect.min, vec2(rect.width(), TITLE_H));
            if active {
                gradient(&painter, bar, pal.title, pal.title2);
            } else {
                painter.rect_filled(bar, 0.0, pal.title_in);
            }
            let fg = if active { Color32::WHITE } else { pal.ink };
            let bar_resp = ui.interact(bar, id.with("title"), Sense::click_and_drag());
            out.pressed = bar_resp.is_pointer_button_down_on();
            let ic = Rect::from_center_size(pos2(bar.left() + 14.0, bar.center().y), vec2(14.0, 14.0));
            icons::icon(icon, active || pal.dark).paint_at(ui, ic);

            let bw = 22.0;
            let by = bar.center().y;
            let btn_rect = |i: usize| {
                Rect::from_center_size(
                    pos2(bar.right() - 4.0 - bw / 2.0 - i as f32 * (bw + 2.0), by),
                    vec2(bw, 18.0),
                )
            };
            let title_clip = Rect::from_min_max(
                pos2(ic.right() + 6.0, bar.top()),
                pos2(btn_rect(2).left() - 4.0, bar.bottom()),
            );
            let galley = painter.layout_no_wrap(title.to_string(), egui::FontId::proportional(13.0), fg);
            painter
                .with_clip_rect(title_clip)
                .galley(pos2(title_clip.left(), by - galley.size().y / 2.0), galley, fg);

            let close = title_button(ui, id.with("close"), btn_rect(0), Btn::Close, geom.state, fg);
            let max = title_button(ui, id.with("max"), btn_rect(1), Btn::Max, geom.state, fg);
            let min = if geom.state == WinState::Min {
                None
            } else {
                Some(title_button(ui, id.with("min"), btn_rect(2), Btn::Min, geom.state, fg))
            };
            if close.clicked() {
                out.close = true;
            }
            if max.clicked() || bar_resp.double_clicked() {
                out.state = Some(if geom.state == WinState::Normal {
                    WinState::Max
                } else {
                    WinState::Normal
                });
            }
            if min.is_some_and(|m| m.clicked()) {
                out.state = Some(WinState::Min);
            }
            let pointer = bar_resp.interact_pointer_pos();
            let start_id = id.with("drag-start");
            // 这一帧已按鼠标位置重新摆放窗口时，不再叠加拖动位移。
            let mut placed = false;
            if bar_resp.drag_started() {
                if let Some(p) = pointer {
                    if geom.state == WinState::Max {
                        // 拖动最大化窗口：先还原，再跟随鼠标。
                        geom.rect = follow_pointer(ws, p, rect, geom.rect.size());
                        out.state = Some(WinState::Normal);
                        placed = true;
                    } else if let (WinState::Normal, Some(size)) = (geom.state, geom.restore.take()) {
                        geom.rect = follow_pointer(ws, p, rect, size);
                        placed = true;
                    }
                }
                ctx.data_mut(|d| d.insert_temp(start_id, geom.rect));
            }
            if geom.state == WinState::Normal && bar_resp.dragged() && !placed {
                geom.rect = geom.rect.translate(bar_resp.drag_delta());
            }
            if bar_resp.drag_stopped() && geom.state != WinState::Min {
                let start = ctx.data(|d| d.get_temp::<Rect>(start_id)).unwrap_or(geom.rect);
                let released = ctx.input(|i| i.pointer.latest_pos()).or(pointer);
                match released.and_then(|p| snap_at(ws, p)) {
                    Some(Snap::Max) => {
                        geom.rect = start;
                        out.state = Some(WinState::Max);
                    }
                    Some(snap) => {
                        geom.restore.get_or_insert(start.size());
                        geom.rect = snap_rect(ws, snap);
                        out.snapped = Some(snap);
                    }
                    None => {}
                }
            }
            if geom.state == WinState::Min && bar_resp.clicked() {
                out.state = Some(WinState::Normal);
            }
            out.pressed |= close.is_pointer_button_down_on() || max.is_pointer_button_down_on();

            // 内容
            if geom.state != WinState::Min {
                let body = Rect::from_min_max(
                    pos2(rect.left() + 1.0, bar.bottom()),
                    pos2(rect.right() - 1.0, rect.bottom() - 1.0),
                );
                let mut child = ui.new_child(UiBuilder::new().max_rect(body).id_salt("body"));
                child.set_clip_rect(body.intersect(ws));
                add_body(&mut child);
            }

            // 边框
            let line = if active { pal.title } else { pal.line };
            painter.rect_stroke(rect, 0.0, Stroke::new(1.0_f32, line));

            // 改变大小：右下角、右边、下边
            if geom.state == WinState::Normal {
                let grips = [
                    (
                        Rect::from_min_max(pos2(rect.right() - 14.0, rect.bottom() - 14.0), rect.max),
                        CursorIcon::ResizeNwSe,
                        (true, true),
                    ),
                    (
                        Rect::from_min_max(
                            pos2(rect.right() - 4.0, bar.bottom()),
                            pos2(rect.right() + 3.0, rect.bottom() - 14.0),
                        ),
                        CursorIcon::ResizeHorizontal,
                        (true, false),
                    ),
                    (
                        Rect::from_min_max(
                            pos2(rect.left(), rect.bottom() - 4.0),
                            pos2(rect.right() - 14.0, rect.bottom() + 3.0),
                        ),
                        CursorIcon::ResizeVertical,
                        (false, true),
                    ),
                ];
                for (i, (r, cursor, (dx, dy))) in grips.into_iter().enumerate() {
                    let g = ui.interact(r, id.with(("grip", i)), Sense::drag());
                    if g.hovered() || g.dragged() {
                        ctx.set_cursor_icon(cursor);
                    }
                    if g.dragged() {
                        geom.restore = None;
                        let d = g.drag_delta();
                        let mut max = geom.rect.max;
                        if dx {
                            max.x += d.x;
                        }
                        if dy {
                            max.y += d.y;
                        }
                        let size = (max - geom.rect.min).max(MIN_SIZE);
                        geom.rect = Rect::from_min_size(geom.rect.min, size);
                    }
                    if i == 0 {
                        // 右下角的斜纹
                        let s = Stroke::new(1.0_f32, pal.weak);
                        for k in [4.0, 8.0] {
                            painter.line_segment(
                                [
                                    pos2(rect.right() - 2.0 - k, rect.bottom() - 2.0),
                                    pos2(rect.right() - 2.0, rect.bottom() - 2.0 - k),
                                ],
                                s,
                            );
                        }
                    }
                }
            }
        });
    if geom.state == WinState::Normal || out.state == Some(WinState::Normal) {
        geom.rect = geom.rect.translate(vec2(0.0, (-geom.rect.top()).max(0.0)));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    fn frame(ctx: &egui::Context, g: &mut Geom, events: Vec<egui::Event>) -> WinResponse {
        frame_in_workspace(ctx, g, events, Rect::from_min_size(Pos2::ZERO, vec2(800.0, 600.0)))
    }
    fn frame_in_workspace(ctx: &egui::Context, g: &mut Geom, events: Vec<egui::Event>, ws: Rect) -> WinResponse {
        let mut response = WinResponse::default();
        let _ = ctx.run(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1000.0, 700.0))),
                events,
                ..Default::default()
            },
            |ctx| {
                response = window(
                    ctx,
                    egui::Id::new("mdi-test"),
                    ws,
                    g,
                    0,
                    "Window",
                    "book",
                    true,
                    &crate::theme::palette(egui::Theme::Light),
                    |ui| {
                        ui.label("body");
                    },
                );
            },
        );
        if let Some(state) = response.state {
            g.state = state;
        }
        response
    }
    fn button(pos: Pos2, pressed: bool) -> egui::Event {
        egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        }
    }
    fn click(ctx: &egui::Context, g: &mut Geom, pos: Pos2) -> WinResponse {
        frame(ctx, g, vec![egui::Event::PointerMoved(pos), button(pos, true)]);
        frame(ctx, g, vec![button(pos, false)])
    }
    #[test]
    fn actual_title_buttons_drag_and_resize_preserve_normal_geometry() {
        let ctx = egui::Context::default();
        let mut g = Geom::new(pos2(20.0, 20.0), vec2(300.0, 200.0));
        frame(&ctx, &mut g, vec![]);
        frame(&ctx, &mut g, vec![]);
        let start = pos2(100.0, 32.0);
        frame(
            &ctx,
            &mut g,
            vec![egui::Event::PointerMoved(start), button(start, true)],
        );
        let end = start + vec2(30.0, 15.0);
        frame(&ctx, &mut g, vec![egui::Event::PointerMoved(end)]);
        frame(&ctx, &mut g, vec![button(end, false)]);
        assert!((g.rect.left() - 50.0).abs() < 1.0 && (g.rect.top() - 35.0).abs() < 1.0);
        frame(&ctx, &mut g, vec![]);
        let start = g.rect.max - vec2(3.0, 3.0);
        let old = g.rect;
        frame(
            &ctx,
            &mut g,
            vec![egui::Event::PointerMoved(start), button(start, true)],
        );
        let end = start + vec2(40.0, 25.0);
        frame(&ctx, &mut g, vec![egui::Event::PointerMoved(end)]);
        frame(&ctx, &mut g, vec![button(end, false)]);
        assert!(g.rect.width() > old.width() && g.rect.height() > old.height());
        let normal = g.rect;
        let at = pos2(g.rect.right() - 39.0, g.rect.top() + 12.0);
        click(&ctx, &mut g, at);
        assert_eq!(g.state, WinState::Max);
        frame(&ctx, &mut g, vec![]);
        click(&ctx, &mut g, pos2(761.0, 12.0));
        assert_eq!(g.state, WinState::Normal);
        assert_eq!(g.rect, normal);
        frame(&ctx, &mut g, vec![]);
        let at = pos2(g.rect.right() - 63.0, g.rect.top() + 12.0);
        click(&ctx, &mut g, at);
        assert_eq!(g.state, WinState::Min);
        frame(&ctx, &mut g, vec![]);
        click(&ctx, &mut g, pos2(135.0, 584.0));
        assert_eq!(g.state, WinState::Normal);
        assert_eq!(g.rect, normal);
        frame(&ctx, &mut g, vec![]);
        let at = pos2(g.rect.right() - 15.0, g.rect.top() + 12.0);
        assert!(click(&ctx, &mut g, at).close);
    }
    #[test]
    fn minimized_windows_wrap_inside_the_workspace() {
        let ws = Rect::from_min_size(pos2(25.0, 45.0), vec2(600.0, 400.0));
        for slot in 0..12 {
            let r = minimized_rect(ws, slot);
            assert!(ws.contains_rect(r));
        }
        assert!(minimized_rect(ws, 3).top() < minimized_rect(ws, 0).top());
    }

    /// 按住标题栏中间，经过 `via` 拖到 `to` 松开。
    fn drag_title(ctx: &egui::Context, g: &mut Geom, via: Pos2, to: Pos2) -> WinResponse {
        frame(ctx, g, vec![]);
        frame(ctx, g, vec![]);
        let rect = if g.state == WinState::Max {
            Rect::from_min_size(Pos2::ZERO, vec2(800.0, 600.0))
        } else {
            g.rect
        };
        let start = pos2(rect.center().x, rect.top() + TITLE_H / 2.0);
        frame(ctx, g, vec![egui::Event::PointerMoved(start), button(start, true)]);
        frame(ctx, g, vec![egui::Event::PointerMoved(via)]);
        frame(ctx, g, vec![egui::Event::PointerMoved(to)]);
        let released = frame(ctx, g, vec![button(to, false)]);
        frame(ctx, g, vec![]);
        released
    }

    #[test]
    fn windows_can_move_past_workspace_edges_without_being_pulled_back() {
        let ctx = egui::Context::default();
        let mut geom = Geom::new(pos2(80.0, 70.0), vec2(320.0, 220.0));
        let released = drag_title(&ctx, &mut geom, pos2(650.0, 220.0), pos2(950.0, 220.0));
        assert!(released.snapped.is_none());
        assert_eq!(geom.state, WinState::Normal);
        assert!(geom.rect.left() > 720.0);
        assert!(geom.rect.right() > 800.0);
        let moved = geom.rect;
        frame(&ctx, &mut geom, vec![]);
        assert_eq!(geom.rect, moved);
        geom.rect = Rect::from_min_size(pos2(-420.0, -90.0), vec2(320.0, 220.0));
        frame(&ctx, &mut geom, vec![]);
        assert_eq!(geom.rect.min, pos2(-420.0, 0.0));
        assert_eq!(geom.rect.size(), vec2(320.0, 220.0));
        for (via, end) in [
            (pos2(80.0, 220.0), pos2(-200.0, 220.0)),
            (pos2(400.0, 550.0), pos2(400.0, 900.0)),
        ] {
            geom = Geom::new(pos2(80.0, 70.0), vec2(320.0, 220.0));
            let released = drag_title(&ctx, &mut geom, via, end);
            assert!(released.snapped.is_none());
            assert_eq!(geom.state, WinState::Normal);
            if end.x < 0.0 {
                assert!(geom.rect.right() < 0.0);
            } else {
                assert!(geom.rect.top() > 600.0);
            }
            let outside = geom.rect;
            frame(&ctx, &mut geom, vec![]);
            assert_eq!(geom.rect, outside);
        }
    }

    #[test]
    fn dragging_above_the_workspace_keeps_the_title_below_the_ribbon() {
        let ctx = egui::Context::default();
        let ws = Rect::from_min_size(pos2(40.0, 120.0), vec2(800.0, 500.0));
        let mut geom = Geom::new(pos2(80.0, 70.0), vec2(320.0, 220.0));
        for _ in 0..2 {
            frame_in_workspace(&ctx, &mut geom, vec![], ws);
        }
        let start = ws.min + geom.rect.min.to_vec2() + vec2(160.0, TITLE_H / 2.0);
        frame_in_workspace(&ctx, &mut geom, vec![egui::Event::PointerMoved(start)], ws);
        frame_in_workspace(&ctx, &mut geom, vec![button(start, true)], ws);
        frame_in_workspace(
            &ctx,
            &mut geom,
            vec![egui::Event::PointerMoved(start + vec2(30.0, -20.0))],
            ws,
        );
        let end = pos2(start.x + 30.0, ws.top() - 100.0);
        frame_in_workspace(&ctx, &mut geom, vec![egui::Event::PointerMoved(end)], ws);
        let released = frame_in_workspace(&ctx, &mut geom, vec![button(end, false)], ws);
        frame_in_workspace(&ctx, &mut geom, vec![], ws);
        assert!(released.snapped.is_none());
        assert_eq!(geom.state, WinState::Normal);
        assert_eq!(geom.rect.top(), 0.0);
        assert_eq!(geom.rect.size(), vec2(320.0, 220.0));
        assert_eq!(
            ctx.memory(|m| m.area_rect(egui::Id::new("mdi-test"))).unwrap().top(),
            ws.top()
        );
    }

    #[test]
    fn snap_zones_follow_windows_11_edges_and_corners() {
        let ws = Rect::from_min_size(pos2(100.0, 50.0), vec2(800.0, 600.0));
        for (p, snap) in [
            (pos2(500.0, 52.0), Some(Snap::Max)),
            (pos2(500.0, 20.0), None),
            (pos2(101.0, 300.0), Some(Snap::Left)),
            (pos2(60.0, 300.0), None),
            (pos2(899.0, 300.0), Some(Snap::Right)),
            (pos2(101.0, 90.0), Some(Snap::TopLeft)),
            (pos2(150.0, 52.0), Some(Snap::TopLeft)),
            (pos2(899.0, 60.0), Some(Snap::TopRight)),
            (pos2(101.0, 640.0), Some(Snap::BottomLeft)),
            (pos2(890.0, 649.0), Some(Snap::BottomRight)),
            (pos2(500.0, 649.0), None),
            (pos2(500.0, 300.0), None),
        ] {
            assert_eq!(snap_at(ws, p), snap, "{p:?}");
        }
        assert_eq!(
            snap_rect(ws, Snap::Left),
            Rect::from_min_size(Pos2::ZERO, vec2(400.0, 600.0))
        );
        assert_eq!(assist_zones(Snap::Left), [Snap::Right]);
        assert_eq!(
            assist_zones(Snap::TopRight),
            [Snap::TopLeft, Snap::BottomLeft, Snap::BottomRight]
        );
        assert!(assist_zones(Snap::Max).is_empty());
        assert_eq!(
            snap_rect(ws, Snap::BottomRight),
            Rect::from_min_size(pos2(400.0, 300.0), vec2(400.0, 300.0))
        );
    }

    #[test]
    fn dragging_title_to_edges_snaps_and_dragging_away_restores_size() {
        let ctx = egui::Context::default();
        let original = Rect::from_min_size(pos2(200.0, 150.0), vec2(300.0, 200.0));
        let mut g = Geom::new(original.min, original.size());

        assert_eq!(
            drag_title(&ctx, &mut g, pos2(150.0, 300.0), pos2(2.0, 300.0)).snapped,
            Some(Snap::Left)
        );
        assert_eq!(
            (g.state, g.rect),
            (WinState::Normal, Rect::from_min_size(Pos2::ZERO, vec2(400.0, 600.0)))
        );
        assert_eq!(g.restore, Some(original.size()));

        drag_title(&ctx, &mut g, pos2(300.0, 200.0), pos2(500.0, 250.0));
        assert_eq!(
            g.rect.size(),
            original.size(),
            "dragging a snapped window away restores its size"
        );
        assert!(g.rect.contains(pos2(500.0, 250.0)) && g.restore.is_none());

        drag_title(&ctx, &mut g, pos2(700.0, 100.0), pos2(798.0, 4.0));
        assert_eq!(g.rect, Rect::from_min_size(pos2(400.0, 0.0), vec2(400.0, 300.0)));
        assert_eq!(g.restore, Some(original.size()));

        // 从四分之一贴靠拖出：恢复大小，鼠标仍在标题栏同一相对位置（原标题栏的 25% 处）。
        drag_title(&ctx, &mut g, pos2(500.0, 200.0), pos2(500.0, 140.0));
        assert_eq!(g.rect.size(), original.size());
        assert!((g.rect.left() + 0.25 * g.rect.width() - 500.0).abs() < 1.0);
        assert!((g.rect.top() + TITLE_H / 2.0 - 140.0).abs() < 1.0);
        let before_max = g.rect;
        assert_eq!(
            drag_title(&ctx, &mut g, pos2(500.0, 60.0), pos2(500.0, 1.0)).snapped,
            None
        );
        assert_eq!(g.state, WinState::Max);
        assert_eq!(g.rect, before_max, "maximizing by drag keeps the normal geometry");

        drag_title(&ctx, &mut g, pos2(300.0, 100.0), pos2(300.0, 300.0));
        assert_eq!(g.state, WinState::Normal);
        assert_eq!(g.rect.size(), original.size());
        assert!(
            g.rect.contains(pos2(300.0, 300.0)),
            "restored window follows the pointer"
        );
    }
}
