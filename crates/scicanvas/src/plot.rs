//! 图形窗口里的绘图区：把核心库排好的图元（scene）画到屏幕上，并处理缩放和平移。
//!
//! 画法和 SVG 导出共用同一份图元，屏幕上看到的就是导出的样子。
//! 操作：滚轮缩放（以鼠标位置为中心），Shift+滚轮只缩放 X，Alt+滚轮只缩放 Y，
//! 左键拖动平移，双击恢复原始范围。

use std::f32::consts::FRAC_PI_2;

use eframe::egui::{self, pos2, vec2, Color32, Pos2, Rect, Sense, Shape, Stroke};
use scicanvas_core::page::Layer;
use scicanvas_core::scene::{self, Anchor, Prim, Rgb, Scene, View};
use scicanvas_core::{Graph, Project};

use crate::fonts::plot_font;

/// 缩放平移后的视图，按图形保存在 egui 的临时内存里。
/// 记下当时的坐标轴设置，在“绘图属性”里改了范围或对数轴后自动作废。
#[derive(Clone)]
struct Saved {
    view: View,
    key: [f64; 8],
    spec: scicanvas_core::plotspec::PlotSpec,
}

fn axis_key(g: &Layer) -> [f64; 8] {
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
pub fn reset_view(ctx: &egui::Context, graph: u64, index: usize) {
    ctx.data_mut(|d| {
        if let Some(mut saved) = d.get_temp::<Vec<Option<Saved>>>(view_id(graph)) {
            if let Some(view) = saved.get_mut(index) {
                *view = None;
            }
            d.insert_temp(view_id(graph), saved);
        }
    });
}

pub struct PlotOutput {
    pub response: egui::Response,
    /// 鼠标所在位置的数据坐标。
    pub hover: Option<(f64, f64)>,
    /// 当前是否处于缩放 / 平移后的视图。
    pub zoomed: bool,
    pub selected_layer: Option<usize>,
    pub hover_layer: Option<usize>,
}

fn rgb(c: Rgb) -> Color32 {
    Color32::from_rgb(c.0, c.1, c.2)
}

/// 页面整体缩放并居中，绘制与命中测试使用同一变换。
fn page_transform(rect: Rect, size: [f64; 2]) -> egui::emath::TSTransform {
    let logical = vec2(size[0] as f32, size[1] as f32);
    let scale = (rect.width() / logical.x).min(rect.height() / logical.y);
    egui::emath::TSTransform::new(rect.min.to_vec2() + (rect.size() - logical * scale) * 0.5, scale)
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

/// 画布与页面之间留出的灰边。
const CANVAS_MARGIN: f32 = 14.0;

/// 页面在画布里的位置：按页面宽高比整体缩放到能放下的最大尺寸并居中，
/// 窗口变宽或变高只改变灰边，不改变页面比例。
pub fn page_rect(canvas: Rect, size: [f64; 2]) -> Rect {
    let inner = canvas.shrink(CANVAS_MARGIN);
    let logical = vec2(size[0] as f32, size[1] as f32);
    let scale = (inner.width() / logical.x).min(inner.height() / logical.y).max(0.0);
    Rect::from_center_size(inner.center(), logical * scale)
}

/// Origin 式画布：灰色工作区里放一张带阴影和细边框的白色页面。
fn paint_canvas(ui: &egui::Ui, canvas: Rect, page: Rect) {
    let painter = ui.painter_at(canvas);
    let dark = ui.visuals().dark_mode;
    painter.rect_filled(canvas, 0.0, Color32::from_gray(if dark { 58 } else { 171 }));
    painter.rect_filled(
        page.translate(vec2(3.0, 3.0)),
        0.0,
        Color32::from_black_alpha(if dark { 140 } else { 70 }),
    );
    painter.rect_filled(page, 0.0, Color32::WHITE);
    painter.rect_stroke(
        page,
        0.0,
        Stroke::new(1.0_f32, Color32::from_gray(if dark { 20 } else { 110 })),
    );
}

pub fn show(ui: &mut egui::Ui, g: &Graph, project: &Project, selected: usize) -> PlotOutput {
    let id = view_id(g.id);
    let available = ui.available_size().max(vec2(120.0, 90.0));
    let (canvas, _) = ui.allocate_exact_size(available, Sense::hover());
    let rect = page_rect(canvas, g.page.logical_size());
    paint_canvas(ui, canvas, rect);
    let response = ui.interact(rect, id.with("page"), Sense::click_and_drag());
    let transform = page_transform(rect, g.page.logical_size());
    let local = |p: Pos2| {
        let p = transform.inverse() * p;
        (p.x as f64, p.y as f64)
    };
    // ponytail: 视图按图层序号缓存；增删时清空，支持重排时改用稳定图层编号。
    let mut saved = ui.data(|d| d.get_temp::<Vec<Option<Saved>>>(id)).unwrap_or_default();
    if saved.len() != g.page.layers().len() {
        saved = vec![None; g.page.layers().len()];
    }
    for (state, layer) in saved.iter_mut().zip(g.page.layers()) {
        if state
            .as_ref()
            .is_some_and(|s| s.key != axis_key(layer) || s.spec != layer.spec)
        {
            *state = None;
        }
    }
    let views = |saved: &[Option<Saved>]| saved.iter().map(|s| s.as_ref().map(|s| s.view)).collect::<Vec<_>>();
    let mut sc = scene::layout_page(g, project, &views(&saved));
    let hovered = response.hover_pos().and_then(|p| {
        let (x, y) = local(p);
        sc.layer_at(x, y).filter(|index| sc.layers[*index].contains(x, y))
    });
    let selected_layer = if response.contains_pointer() && ui.input(|i| i.pointer.any_pressed()) {
        response.interact_pointer_pos().and_then(|p| {
            let (x, y) = local(p);
            sc.layer_at(x, y)
        })
    } else {
        None
    };
    // 拖动锁定按下时的图层，越过其他图层也继续操作原图层。
    let dragging = if response.dragged_by(egui::PointerButton::Primary) {
        ui.input(|i| i.pointer.press_origin()).and_then(|p| {
            let (x, y) = local(p);
            sc.layer_at(x, y).filter(|index| sc.layers[*index].contains(x, y))
        })
    } else {
        None
    };
    let mut changed = false;
    if let Some(index) = dragging.or(hovered) {
        let layer = &g.page.layers()[index];
        let layout = &sc.layers[index];
        if response.double_clicked() {
            saved[index] = None;
            changed = true;
        } else {
            let mut view = layout.view();
            if dragging.is_some() {
                let d = response.drag_delta();
                if d != egui::Vec2::ZERO {
                    view.x = adjust(
                        view.x,
                        layer.x_log,
                        -d.x as f64 / f64::from(transform.scaling) / layout.plot[2],
                        1.0,
                        0.5,
                    );
                    view.y = adjust(
                        view.y,
                        layer.y_log,
                        d.y as f64 / f64::from(transform.scaling) / layout.plot[3],
                        1.0,
                        0.5,
                    );
                    changed = true;
                }
            } else if let Some(p) = response.hover_pos() {
                let (scroll, zoom_delta, alt) = ui.input(|i| (i.smooth_scroll_delta, i.zoom_delta(), i.modifiers.alt));
                let (mut zx, mut zy) = (1.0, 1.0);
                if scroll.y != 0.0 {
                    let z = (-scroll.y as f64 * 0.0025).exp();
                    zy = z;
                    if !alt {
                        zx = z;
                    }
                } else if scroll.x != 0.0 {
                    zx = (-scroll.x as f64 * 0.0025).exp();
                }
                if zoom_delta != 1.0 {
                    zx /= zoom_delta as f64;
                    zy /= zoom_delta as f64;
                }
                if zx != 1.0 || zy != 1.0 {
                    let (x, y) = local(p);
                    let fx = ((x - layout.plot[0]) / layout.plot[2]).clamp(0.0, 1.0);
                    let fy = ((layout.plot[1] + layout.plot[3] - y) / layout.plot[3]).clamp(0.0, 1.0);
                    view.x = adjust(view.x, layer.x_log, 0.0, zx, fx);
                    view.y = adjust(view.y, layer.y_log, 0.0, zy, fy);
                    changed = true;
                }
            }
            if changed {
                saved[index] = Some(Saved {
                    view,
                    key: axis_key(layer),
                    spec: layer.spec.clone(),
                });
            }
        }
    }
    if changed {
        sc = scene::layout_page(g, project, &views(&saved));
    }
    paint(ui, rect, &sc);
    if sc.layers.len() > 1 {
        let index = selected_layer.unwrap_or(selected).min(sc.layers.len() - 1);
        let [x, y, w, h] = sc.layers[index].plot;
        let outline = Rect::from_min_size(
            transform * pos2(x as f32, y as f32),
            vec2(w as f32, h as f32) * transform.scaling,
        );
        ui.painter().rect_stroke(
            outline.expand(3.0),
            0.0,
            Stroke::new(1.0_f32, ui.visuals().selection.bg_fill),
        );
    }
    let hover = hovered.and_then(|index| {
        response.hover_pos().map(|p| {
            let (x, y) = local(p);
            sc.layers[index].data_at(x, y)
        })
    });
    if hover.is_some() && !response.dragged() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
    }
    if dragging.is_some() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
    }
    let zoomed = saved.iter().any(Option::is_some);
    ui.data_mut(|d| d.insert_temp(id, saved));
    PlotOutput {
        response,
        hover,
        zoomed,
        selected_layer,
        hover_layer: hovered,
    }
}

/// 把图元画到 rect 里。
pub fn paint(ui: &egui::Ui, rect: Rect, sc: &Scene) {
    let transform = page_transform(rect, [sc.w, sc.h]);
    let scale = transform.scaling;
    let at = |x: f64, y: f64| transform * pos2(x as f32, y as f32);
    let free = ui.painter_at(rect);

    for it in &sc.items {
        let p = match it.clip {
            Some([x, y, w, h]) => free.with_clip_rect(
                Rect::from_min_size(at(x, y), vec2(w as f32, h as f32) * scale).intersect(free.clip_rect()),
            ),
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
                let r = Rect::from_min_size(at(*x, *y), vec2(*w as f32, *h as f32) * scale);
                let fill = fill
                    .map(|(c, a)| rgb(c).gamma_multiply(a as f32))
                    .unwrap_or(Color32::TRANSPARENT);
                let stroke = stroke
                    .map(|(c, wd)| Stroke::new(wd as f32 * scale, rgb(c)))
                    .unwrap_or(Stroke::NONE);
                p.rect(r, 0.0, fill, stroke);
            }
            Prim::Segments {
                segs,
                color,
                width,
                dashed,
            } => {
                let stroke = Stroke::new(*width as f32 * scale, rgb(*color));
                for s in segs {
                    let pts = [at(s[0], s[1]), at(s[2], s[3])];
                    if *dashed {
                        p.extend(Shape::dashed_line(&pts, stroke, 3.0 * scale, 3.0 * scale));
                    } else {
                        p.line_segment(pts, stroke);
                    }
                }
            }
            Prim::Polyline { pts, color, width } => {
                let pts: Vec<Pos2> = pts.iter().map(|q| at(q[0], q[1])).collect();
                p.add(Shape::line(pts, Stroke::new(*width as f32 * scale, rgb(*color))));
            }
            Prim::Triangles { pts, color, opacity } => {
                let mut mesh = egui::Mesh::default();
                let color = rgb(*color).gamma_multiply(*opacity as f32);
                for q in pts {
                    mesh.colored_vertex(at(q[0], q[1]), color);
                }
                mesh.indices.extend(0..pts.len() as u32);
                p.add(Shape::mesh(mesh));
            }
            Prim::Symbols { sym, pts, r, color } => {
                let c = rgb(*color);
                for q in pts {
                    match scene::symbol_polygon(*sym, q[0], q[1], *r) {
                        None => {
                            p.circle_filled(at(q[0], q[1]), *r as f32 * scale, c);
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
                let galley = p.layout_no_wrap(text.clone(), plot_font(*size as f32 * scale, *bold), rgb(*color));
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

#[cfg(test)]
mod tests {
    use super::*;

    fn project() -> Project {
        let mut p = Project::sample();
        let mut layer = p.graphs[0].page.layers()[0].clone();
        layer.y_keys = vec!["C".into()];
        p.graphs[0].page.add_layer(layer);
        p.graphs[0].page.columns = 2;
        p
    }

    fn frame(ctx: &egui::Context, p: &Project, events: Vec<egui::Event>) -> PlotOutput {
        let mut output = None;
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1000.0, 700.0))),
            events,
            ..Default::default()
        };
        let _ = ctx.run(input, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                output = Some(show(ui, &p.graphs[0], p, 0));
            });
        });
        output.unwrap()
    }

    fn button(pos: Pos2, pressed: bool) -> egui::Event {
        egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        }
    }

    fn point(rect: Rect, sc: &Scene, index: usize) -> Pos2 {
        let [x, y, w, h] = sc.layers[index].plot;
        page_transform(rect, [sc.w, sc.h]) * pos2((x + w / 2.0) as f32, (y + h / 2.0) as f32)
    }

    #[test]
    fn click_and_wheel_target_second_layer_and_reset_keeps_other_view() {
        let ctx = egui::Context::default();
        crate::fonts::setup(&ctx);
        let p = project();
        let initial = frame(&ctx, &p, vec![]);
        let sc = scene::layout_page(&p.graphs[0], &p, &[]);
        let at = point(initial.response.rect, &sc, 1);
        let output = frame(&ctx, &p, vec![egui::Event::PointerMoved(at), button(at, true)]);
        assert_eq!(output.selected_layer, Some(1));
        frame(&ctx, &p, vec![button(at, false)]);
        let wheel = |at| {
            vec![
                egui::Event::PointerMoved(at),
                egui::Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Point,
                    delta: vec2(0.0, 120.0),
                    modifiers: egui::Modifiers::NONE,
                },
            ]
        };
        let output = frame(&ctx, &p, wheel(at));
        assert_eq!(output.hover_layer, Some(1));
        let states = ctx.data(|d| d.get_temp::<Vec<Option<Saved>>>(view_id(2))).unwrap();
        assert!(states[0].is_none());
        assert!(states[1].is_some());
        let first = point(initial.response.rect, &sc, 0);
        frame(&ctx, &p, wheel(first));
        reset_view(&ctx, 2, 1);
        let states = ctx.data(|d| d.get_temp::<Vec<Option<Saved>>>(view_id(2))).unwrap();
        assert!(states[0].is_some());
        assert!(states[1].is_none());
    }

    #[test]
    fn pan_stays_on_press_layer_when_pointer_crosses_into_next_panel() {
        let ctx = egui::Context::default();
        crate::fonts::setup(&ctx);
        let p = project();
        let initial = frame(&ctx, &p, vec![]);
        let sc = scene::layout_page(&p.graphs[0], &p, &[]);
        let start = point(initial.response.rect, &sc, 0);
        let end = point(initial.response.rect, &sc, 1);
        frame(&ctx, &p, vec![egui::Event::PointerMoved(start), button(start, true)]);
        frame(&ctx, &p, vec![egui::Event::PointerMoved(start + vec2(12.0, 0.0))]);
        frame(&ctx, &p, vec![egui::Event::PointerMoved(end)]);
        let states = ctx.data(|d| d.get_temp::<Vec<Option<Saved>>>(view_id(2))).unwrap();
        assert!(states[0].is_some());
        assert!(states[1].is_none());
    }

    #[test]
    fn area_mesh_scales_vertices_alpha_and_clips_with_the_page() {
        let mut p = Project::sample();
        p.graphs[0]
            .first_layer_mut()
            .set_plot_type(scicanvas_core::PlotType::Area)
            .unwrap();
        p.graphs[0].first_layer_mut().y_keys = vec!["B".into()];
        p.graphs[0].first_layer_mut().area_opacity = 0.5;
        let sc = scene::layout_page(&p.graphs[0], &p, &[]);
        let pts = sc
            .items
            .iter()
            .find_map(|it| match &it.prim {
                Prim::Triangles { pts, .. } => Some(pts),
                _ => None,
            })
            .unwrap();
        let ctx = egui::Context::default();
        crate::fonts::setup(&ctx);
        for scale in [0.5, 1.0, 2.0] {
            let rect = Rect::from_min_size(pos2(20.0, 25.0), vec2(sc.w as f32, sc.h as f32) * scale);
            let output = ctx.run(egui::RawInput::default(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| paint(ui, rect, &sc));
            });
            let (mesh, clip) = output
                .shapes
                .iter()
                .find_map(|s| match &s.shape {
                    Shape::Mesh(m) if m.vertices.len() == pts.len() => Some((m, s.clip_rect)),
                    _ => None,
                })
                .unwrap();
            let tf = page_transform(rect, [sc.w, sc.h]);
            for (vertex, point) in mesh.vertices.iter().zip(pts) {
                assert!(vertex.pos.distance(tf * pos2(point[0] as f32, point[1] as f32)) < 1e-4);
                assert_eq!(vertex.color.a(), 128);
            }
            assert_eq!(mesh.indices.len(), pts.len());
            assert!(rect.contains_rect(clip));
        }
    }

    #[test]
    fn page_keeps_its_aspect_and_stays_centered_as_the_window_resizes() {
        let ctx = egui::Context::default();
        crate::fonts::setup(&ctx);
        let mut p = Project::sample();
        for (width_mm, height_mm) in [(85.0, 63.75), (60.0, 120.0)] {
            p.graphs[0].page.width_mm = width_mm;
            p.graphs[0].page.height_mm = height_mm;
            let aspect = (width_mm / height_mm) as f32;
            for screen in [
                vec2(1400.0, 500.0),
                vec2(500.0, 1200.0),
                vec2(800.0, 800.0),
                vec2(150.0, 120.0),
            ] {
                let mut output = None;
                let mut canvas = Rect::NOTHING;
                let input = egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, screen)),
                    ..Default::default()
                };
                let _ = ctx.run(input, |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        canvas = ui.available_rect_before_wrap();
                        output = Some(show(ui, &p.graphs[0], &p, 0));
                    });
                });
                let page = output.unwrap().response.rect;
                assert!((page.aspect_ratio() - aspect).abs() < 1e-3, "{screen:?}: {page:?}");
                assert!((page.center() - canvas.center()).length() < 0.5, "{screen:?}");
                let inner = canvas.shrink(CANVAS_MARGIN);
                assert!(inner.expand(0.5).contains_rect(page), "{screen:?}");
                // 页面撑满较紧的一边，另一边留灰边。
                let fit = (page.width() - inner.width())
                    .abs()
                    .min((page.height() - inner.height()).abs());
                assert!(fit < 0.5, "{screen:?}");
            }
        }
    }

    #[test]
    fn page_geometry_text_and_strokes_scale_together() {
        let p = Project::sample();
        let sc = scene::layout_page(&p.graphs[0], &p, &[]);
        let ctx = egui::Context::default();
        crate::fonts::setup(&ctx);
        for scale in [1.0_f32, 2.0, 0.5] {
            let rect = Rect::from_min_size(pos2(20.0, 25.0), vec2(sc.w as f32, sc.h as f32) * scale);
            let output = ctx.run(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1500.0, 1200.0))),
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| paint(ui, rect, &sc));
                },
            );
            // 坐标轴是“左轴 + 下轴”的三点折线：先竖直向下，再水平向右。
            let stroke = output
                .shapes
                .iter()
                .find_map(|s| match &s.shape {
                    Shape::Path(p)
                        if p.points.len() == 3
                            && (p.points[0].x - p.points[1].x).abs() < 1e-3
                            && (p.points[1].y - p.points[2].y).abs() < 1e-3 =>
                    {
                        Some(p.stroke.width)
                    }
                    _ => None,
                })
                .unwrap();
            let font = output
                .shapes
                .iter()
                .find_map(|s| match &s.shape {
                    Shape::Text(t) => Some(t.galley.job.sections[0].format.font_id.size),
                    _ => None,
                })
                .unwrap();
            assert!((stroke / scale - scene::AXIS_WIDTH as f32).abs() < 1e-4);
            assert!((font / scale - scene::TICK_FONT as f32).abs() < 1e-4);
            let transform = page_transform(rect, [sc.w, sc.h]);
            let page_point = pos2(sc.px(10.0) as f32, sc.py(0.0) as f32);
            let recovered = transform.inverse() * (transform * page_point);
            assert!(recovered.distance(page_point) < 1e-4);
        }
    }
}
