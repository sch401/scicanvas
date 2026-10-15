//! 图形窗口里的绘图区：把核心库排好的图元（scene）画到屏幕上，并处理缩放和平移。
//!
//! 画法和 SVG 导出共用同一份图元，屏幕上看到的就是导出的样子。
//! 操作：滚轮缩放（以鼠标位置为中心），Shift+滚轮只缩放 X，Alt+滚轮只缩放 Y，
//! 左键拖动平移，双击恢复原始范围。

use std::f32::consts::FRAC_PI_2;

use eframe::egui::{self, pos2, vec2, Color32, Pos2, Rect, Sense, Shape, Stroke};
use scicanvas_core::scene::{self, Anchor, Prim, Rgb, Scene, View};
use scicanvas_core::{Graph, Project};

use crate::fonts::plot_font;

/// 缩放平移后的视图，按图形保存在 egui 的临时内存里。
/// 记下当时的坐标轴设置，在“绘图属性”里改了范围或对数轴后自动作废。
#[derive(Clone, Copy)]
struct Saved {
    view: View,
    key: [f64; 8],
}

fn axis_key(g: &Graph) -> [f64; 8] {
    let b = |v: bool| if v { 1.0 } else { 0.0 };
    [
        b(g.x_auto),
        g.x_min,
        g.x_max,
        b(g.x_log),
        b(g.y_auto),
        g.y_min,
        g.y_max,
        b(g.y_log),
    ]
}

fn view_id(graph: u64) -> egui::Id {
    egui::Id::new(("scicanvas-plot-view", graph))
}

/// 恢复原始范围（例如改了坐标轴设置之后）。
pub fn reset_view(ctx: &egui::Context, graph: u64) {
    ctx.data_mut(|d| d.remove::<Saved>(view_id(graph)));
}

pub struct PlotOutput {
    pub response: egui::Response,
    /// 鼠标所在位置的数据坐标。
    pub hover: Option<(f64, f64)>,
    /// 当前是否处于缩放 / 平移后的视图。
    pub zoomed: bool,
}

fn rgb(c: Rgb) -> Color32 {
    Color32::from_rgb(c.0, c.1, c.2)
}

fn tf(v: f64, log: bool) -> f64 {
    if log {
        v.log10()
    } else {
        v
    }
}

fn itf(v: f64, log: bool) -> f64 {
    if log {
        10f64.powf(v)
    } else {
        v
    }
}

/// 在变换后的空间里（对数轴取 lg）平移 / 缩放一个范围。
/// shift 是按范围宽度计的平移比例，zoom 是缩放倍数，center 是缩放中心在范围里的位置（0..1）。
fn adjust(r: (f64, f64), log: bool, shift: f64, zoom: f64, center: f64) -> (f64, f64) {
    let (a, b) = (tf(r.0, log), tf(r.1, log));
    let span = b - a;
    let (a, b) = (a + shift * span, b + shift * span);
    let c = a + center * (b - a);
    let (a, b) = (c - (c - a) * zoom, c + (b - c) * zoom);
    let (a, b) = (itf(a, log), itf(b, log));
    if a.is_finite() && b.is_finite() && b > a && (b - a) > 1e-12 * a.abs().max(b.abs()).max(1e-300) {
        (a, b)
    } else {
        r
    }
}

pub fn show(ui: &mut egui::Ui, g: &Graph, project: &Project) -> PlotOutput {
    let id = view_id(g.id);
    let available = ui.available_size().max(vec2(120.0, 90.0));
    let aspect = (g.page.width_mm / g.page.height_mm) as f32;
    let size = if available.x / available.y > aspect {
        vec2(available.y * aspect, available.y)
    } else {
        vec2(available.x, available.x / aspect)
    };
    let (rect, response) = ui.allocate_exact_size(size, Sense::click_and_drag());
    let (w, h) = (rect.width() as f64, rect.height() as f64);

    let mut saved: Option<Saved> = ui.data(|d| d.get_temp::<Saved>(id)).filter(|s| s.key == axis_key(g));
    let mut sc = scene::build_page(g, project, w, h, saved.map(|s| s.view));
    let local = |p: Pos2| ((p.x - rect.left()) as f64, (p.y - rect.top()) as f64);

    // 交互
    let mut changed = false;
    if response.double_clicked() {
        saved = None;
        changed = true;
    } else {
        let mut v = sc.view();
        let pw = sc.plot[2];
        let ph = sc.plot[3];
        if response.dragged_by(egui::PointerButton::Primary)
            && response.interact_pointer_pos().is_some_and(|p| {
                let (x, y) = local(p);
                sc.contains(x, y)
            })
        {
            let d = response.drag_delta();
            if d != egui::Vec2::ZERO {
                v.x = adjust(v.x, g.x_log, -d.x as f64 / pw, 1.0, 0.5);
                v.y = adjust(v.y, g.y_log, d.y as f64 / ph, 1.0, 0.5);
                changed = true;
            }
        }
        if let Some(p) = response.hover_pos().filter(|p| {
            let (x, y) = local(*p);
            sc.contains(x, y)
        }) {
            let (scroll, zoom_delta, alt) = ui.input(|i| (i.smooth_scroll_delta, i.zoom_delta(), i.modifiers.alt));
            // Shift+滚轮在 egui 里会变成水平滚动
            let (mut zx, mut zy) = (1.0, 1.0);
            if scroll.y != 0.0 {
                let z = (-scroll.y as f64 * 0.0025).exp();
                if alt {
                    zy = z;
                } else {
                    zx = z;
                    zy = z;
                }
            } else if scroll.x != 0.0 {
                zx = (-scroll.x as f64 * 0.0025).exp();
            }
            if zoom_delta != 1.0 {
                zx /= zoom_delta as f64;
                zy /= zoom_delta as f64;
            }
            if zx != 1.0 || zy != 1.0 {
                let (lx, ly) = local(p);
                let fx = ((lx - sc.plot[0]) / pw).clamp(0.0, 1.0);
                let fy = ((sc.plot[1] + ph - ly) / ph).clamp(0.0, 1.0);
                v.x = adjust(v.x, g.x_log, 0.0, zx, fx);
                v.y = adjust(v.y, g.y_log, 0.0, zy, fy);
                changed = true;
            }
        }
        if changed {
            saved = Some(Saved {
                view: v,
                key: axis_key(g),
            });
        }
    }
    if changed {
        ui.data_mut(|d| match saved {
            Some(s) => d.insert_temp(id, s),
            None => d.remove::<Saved>(id),
        });
        sc = scene::build_page(g, project, w, h, saved.map(|s| s.view));
    }

    paint(ui, rect, &sc);

    let hover = response
        .hover_pos()
        .map(local)
        .filter(|(x, y)| sc.contains(*x, *y))
        .map(|(x, y)| sc.data_at(x, y));
    if hover.is_some() && !response.dragged() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
    }
    if response.dragged() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
    }
    PlotOutput {
        response,
        hover,
        zoomed: saved.is_some(),
    }
}

/// 把图元画到 rect 里。
pub fn paint(ui: &egui::Ui, rect: Rect, sc: &Scene) {
    let o = rect.min;
    let at = |x: f64, y: f64| pos2(o.x + x as f32, o.y + y as f32);
    let free = ui.painter_at(rect);

    for it in &sc.items {
        let p = match it.clip {
            Some([x, y, w, h]) => {
                free.with_clip_rect(Rect::from_min_size(at(x, y), vec2(w as f32, h as f32)).intersect(free.clip_rect()))
            }
            None => free.clone(),
        };
        match &it.prim {
            Prim::Rect {
                x,
                y,
                w,
                h,
                fill,
                stroke,
            } => {
                let r = Rect::from_min_size(at(*x, *y), vec2(*w as f32, *h as f32));
                let fill = fill
                    .map(|(c, a)| rgb(c).gamma_multiply(a as f32))
                    .unwrap_or(Color32::TRANSPARENT);
                let stroke = stroke
                    .map(|(c, wd)| Stroke::new(wd as f32, rgb(c)))
                    .unwrap_or(Stroke::NONE);
                p.rect(r, 0.0, fill, stroke);
            }
            Prim::Segments {
                segs,
                color,
                width,
                dashed,
            } => {
                let stroke = Stroke::new(*width as f32, rgb(*color));
                for s in segs {
                    let pts = [at(s[0], s[1]), at(s[2], s[3])];
                    if *dashed {
                        p.extend(Shape::dashed_line(&pts, stroke, 3.0, 3.0));
                    } else {
                        p.line_segment(pts, stroke);
                    }
                }
            }
            Prim::Polyline { pts, color, width } => {
                let pts: Vec<Pos2> = pts.iter().map(|q| at(q[0], q[1])).collect();
                p.add(Shape::line(pts, Stroke::new(*width as f32, rgb(*color))));
            }
            Prim::Symbols { sym, pts, r, color } => {
                let c = rgb(*color);
                for q in pts {
                    match scene::symbol_polygon(*sym, q[0], q[1], *r) {
                        None => {
                            p.circle_filled(at(q[0], q[1]), *r as f32, c);
                        }
                        Some(poly) => {
                            let poly: Vec<Pos2> = poly.iter().map(|v| at(v[0], v[1])).collect();
                            p.add(Shape::convex_polygon(poly, c, Stroke::NONE));
                        }
                    }
                }
            }
            Prim::Text {
                x,
                y,
                text,
                size,
                anchor,
                vertical,
                bold,
                color,
            } => {
                let galley = p.layout_no_wrap(text.clone(), plot_font(*size as f32, *bold), rgb(*color));
                let gs = galley.size();
                let c = at(*x, *y);
                let k = match anchor {
                    Anchor::Start => 0.0,
                    Anchor::Middle => 0.5,
                    Anchor::End => 1.0,
                };
                if *vertical {
                    // 逆时针转 90°：文字沿 -y 方向排开，pos 是旋转前的左上角
                    let pos = pos2(c.x - gs.y / 2.0, c.y + gs.x * k);
                    p.add(egui::epaint::TextShape::new(pos, galley, rgb(*color)).with_angle(-FRAC_PI_2));
                } else {
                    let pos = pos2(c.x - gs.x * k, c.y - gs.y / 2.0);
                    p.galley(pos, galley, rgb(*color));
                }
            }
        }
    }
}
