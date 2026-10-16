//! 多文档子窗口（Origin 的 MDI 工作区）：标题栏带最小化、最大化 / 还原、关闭按钮，
//! 拖动标题栏移动，双击标题栏最大化 / 还原，拖动右下角或右边、下边改变大小。
//! 最大化铺满工作区，最小化后排在工作区底部。

use eframe::egui::{self, pos2, vec2, Color32, CursorIcon, Mesh, Pos2, Rect, Sense, Shadow, Stroke, UiBuilder};

use crate::icons;
use crate::theme::Palette;

pub const TITLE_H: f32 = 24.0;
const MIN_W: f32 = 170.0;
const MIN_SIZE: egui::Vec2 = egui::Vec2::new(220.0, 140.0);

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
}

impl Geom {
    pub fn new(pos: Pos2, size: egui::Vec2) -> Self {
        Geom {
            rect: Rect::from_min_size(pos, size),
            state: WinState::Normal,
        }
    }
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
    // 窗口至少有一部分标题栏留在工作区里
    if geom.state == WinState::Normal {
        let r = geom.rect;
        let x = r.left().clamp(-r.width() + 80.0, (ws.width() - 80.0).max(0.0));
        let y = r.top().clamp(0.0, (ws.height() - TITLE_H).max(0.0));
        geom.rect = Rect::from_min_size(pos2(x, y), r.size());
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
            let fg = Color32::WHITE;
            let bar_resp = ui.interact(bar, id.with("title"), Sense::click_and_drag());
            out.pressed = bar_resp.is_pointer_button_down_on();
            let ic = Rect::from_center_size(pos2(bar.left() + 14.0, bar.center().y), vec2(14.0, 14.0));
            icons::icon(icon, true).paint_at(ui, ic);

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
            if geom.state == WinState::Normal && bar_resp.dragged() {
                geom.rect = geom.rect.translate(bar_resp.drag_delta());
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
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    fn frame(ctx: &egui::Context, g: &mut Geom, events: Vec<egui::Event>) -> WinResponse {
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
                    Rect::from_min_size(Pos2::ZERO, vec2(800.0, 600.0)),
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
}
