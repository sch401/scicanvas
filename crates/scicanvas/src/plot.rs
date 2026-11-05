//! 图形窗口里的绘图区：把核心库排好的图元（scene）画到屏幕上，并处理缩放和平移。
//!
//! 画法和 SVG 导出共用同一份图元，屏幕上看到的就是导出的样子。
//! 操作：滚轮缩放（以鼠标位置为中心），Shift+滚轮只缩放 X，Alt+滚轮只缩放 Y，
//! 左键拖动平移，双击恢复原始范围。

mod shapes;

use std::f32::consts::FRAC_PI_2;

use eframe::egui::{self, pos2, vec2, Color32, Pos2, Rect, Sense, Shape, Stroke};
use scicanvas_core::page::Layer;
pub use scicanvas_core::scene::ObjectTarget;
use scicanvas_core::scene::{self, Anchor, Prim, Rgb, Scene, View};
use scicanvas_core::{Graph, Project};

use crate::fonts::plot_font;

/// 出版画布与辅助提示使用固定颜色，不随界面主题变化。
pub const PAGE_BG: Color32 = Color32::WHITE;
pub const PAGE_INK: Color32 = Color32::BLACK;
pub const PAGE_HINT: Color32 = Color32::from_gray(110);
pub const PAGE_HINT_WEAK: Color32 = Color32::from_gray(140);

/// 缩放平移后的视图，按图形保存在 egui 的临时内存里。
/// 记下当时的坐标轴设置，在“绘图属性”里改了范围或对数轴后自动作废。
#[derive(Clone)]
struct Saved {
    view: View,
    key: [f64; 8],
    offset: Option<scicanvas_core::page::CurveOffset>,
    spec: scicanvas_core::page::LayerSpec,
}

/// 场景缓存：图形设置、视图和所引用的数据都没变时，复用上一帧的排版（百万点的图不必每帧重算）。
#[derive(Clone)]
struct CachedScene {
    graph: Graph,
    views: Vec<Option<View>>,
    data: u64,
    scene: std::sync::Arc<Scene>,
}

/// 图形引用的列（X、Y、浮动柱下端、误差列、标签列）的内容指纹：数值、文字、名称和单位都参与。
fn data_fingerprint(g: &Graph, project: &Project) -> u64 {
    use scicanvas_core::Role;
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    let mut mix = |v: u64| {
        h = (h ^ v).wrapping_mul(0x0000_0100_0000_01b3);
    };
    let text = |t: &str, mix: &mut dyn FnMut(u64)| {
        mix(t.len() as u64);
        for chunk in t.as_bytes().chunks(8) {
            let mut b = [0u8; 8];
            b[..chunk.len()].copy_from_slice(chunk);
            mix(u64::from_le_bytes(b));
        }
    };
    for layer in g.page.layers() {
        let Some(main) = layer.book_id.and_then(|b| project.book(b)) else {
            mix(0);
            continue;
        };
        // 本表和图层里曲线引用到的其他工作簿
        let books = std::iter::once(main).chain(layer.foreign_books().into_iter().filter_map(|b| project.book(b)));
        for book in books {
            mix(book.id);
            if layer.visible_only {
                mix(book.filter_fingerprint());
            }
            text(&book.name, &mut mix);
            let local = book.id == main.id;
            for (index, col) in book.columns.iter().enumerate() {
                if !layer.references_plot_column(book, &col.key)
                    && !matches!(col.role, Role::XError | Role::YError | Role::Label)
                {
                    continue;
                }
                text(&col.key, &mut mix);
                text(&col.long_name, &mut mix);
                text(&col.units, &mut mix);
                mix(col.role as u64);
                mix(col.data.kind() as u64);
                text(book.error_source(index).unwrap_or(""), &mut mix);
                mix(col.format.utc_offset as u64);
                mix(col.data.len() as u64);
                if let Some(levels) = col.data.category_levels() {
                    for l in &levels {
                        text(l, &mut mix);
                    }
                }
                let label_text = col.role == Role::Label || (local && layer.labels.column.as_deref() == Some(&col.key));
                for row in 0..col.data.len() {
                    match col.data.f64_at(row) {
                        Some(v) => {
                            mix(v.to_bits());
                            if label_text {
                                text(&col.data.text_at(row), &mut mix);
                            }
                        }
                        None => text(&col.data.text_at(row), &mut mix),
                    }
                }
            }
        }
    }
    h
}

/// 取场景：与缓存的图形、视图、数据指纹都相同时复用，否则重新排版并更新缓存。
fn cached_layout(
    ui: &egui::Ui,
    id: egui::Id,
    g: &Graph,
    project: &Project,
    views: Vec<Option<View>>,
) -> std::sync::Arc<Scene> {
    let key = id.with("scene-cache");
    let data = data_fingerprint(g, project);
    if let Some(c) = ui.data(|d| d.get_temp::<CachedScene>(key)) {
        if c.data == data && c.views == views && c.graph == *g {
            return c.scene;
        }
    }
    let scene = std::sync::Arc::new(scene::layout_page(g, project, &views));
    ui.data_mut(|d| {
        d.insert_temp(
            key,
            CachedScene {
                graph: g.clone(),
                views,
                data,
                scene: scene.clone(),
            },
        )
    });
    scene
}

/// 图形绘制指令缓存：窗口移动只平移已有矢量图元，设置/数据/视图或显示尺寸变化才重新生成。
#[derive(Clone)]
struct CachedPaint {
    scene: std::sync::Arc<Scene>,
    size: egui::Vec2,
    pixels_per_point: f32,
    shapes: std::sync::Arc<Vec<egui::epaint::ClippedShape>>,
}

fn cached_paint_shapes(
    ui: &mut egui::Ui,
    id: egui::Id,
    rect: Rect,
    scene: &std::sync::Arc<Scene>,
) -> std::sync::Arc<Vec<egui::epaint::ClippedShape>> {
    let key = id.with("paint-cache");
    let pixels_per_point = ui.ctx().pixels_per_point();
    if let Some(cache) = ui.data(|data| data.get_temp::<CachedPaint>(key)) {
        if std::sync::Arc::ptr_eq(&cache.scene, scene)
            && (cache.size - rect.size()).abs().max_elem() <= 0.001
            && cache.pixels_per_point == pixels_per_point
        {
            return cache.shapes;
        }
    }
    let local = Rect::from_min_size(Pos2::ZERO, rect.size());
    let layer = egui::LayerId::new(egui::Order::Background, id.with("paint-source"));
    let source = egui::Painter::new(ui.ctx().clone(), layer, local);
    paint_with_painter(source, local, scene);
    let shapes = ui.ctx().graphics_mut(|graphics| {
        let list = std::mem::take(graphics.entry(layer));
        std::sync::Arc::new(list.all_entries().cloned().collect::<Vec<_>>())
    });
    ui.data_mut(|data| {
        data.insert_temp(
            key,
            CachedPaint {
                scene: scene.clone(),
                size: rect.size(),
                pixels_per_point,
                shapes: shapes.clone(),
            },
        )
    });
    shapes
}

fn paint_cached(ui: &mut egui::Ui, id: egui::Id, rect: Rect, scene: &std::sync::Arc<Scene>) {
    for cached in cached_paint_shapes(ui, id, rect, scene).iter() {
        let mut shape = cached.shape.clone();
        shape.translate(rect.min.to_vec2());
        ui.painter()
            .with_clip_rect(cached.clip_rect.translate(rect.min.to_vec2()))
            .add(shape);
    }
}

fn axis_key(g: &Layer) -> [f64; 8] {
    let b = |v: bool| if v { 1.0 } else { 0.0 };
    [
        b(g.x_auto),
        g.x_min,
        g.x_max,
        g.effective_x_scale() as u8 as f64,
        b(g.y_auto),
        g.y_min,
        g.y_max,
        g.effective_y_scale() as u8 as f64,
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
    /// 鼠标吸附到的数据点。
    pub point: Option<scene::PointHit>,
    pub object: Option<ObjectTarget>,
    pub action: Option<ObjectAction>,
    pub object_dragging: bool,
}

#[derive(Clone, Copy)]
pub struct ObjectUi {
    pub adding_shape: Option<scicanvas_core::page::ShapeKind>,
    pub selected_shape: Option<u64>,
    pub adding: bool,
    pub epoch: u64,
    pub keyboard: bool,
}
impl Default for ObjectUi {
    fn default() -> Self {
        Self {
            adding: false,
            adding_shape: None,
            selected_shape: None,
            epoch: 0,
            keyboard: true,
        }
    }
}
#[derive(Clone, Copy, Debug)]
pub enum ObjectAction {
    AddText([f64; 2]),
    AddShape(scicanvas_core::page::ShapeKind, [f64; 2], [f64; 2]),
    ResizeShape(u64, [f64; 2], [f64; 2]),
    Move(ObjectTarget, [f64; 2]),
    Edit(ObjectTarget),
    Delete(ObjectTarget),
    AutoLegend(usize),
    CancelAdd,
}
#[derive(Clone)]
struct ObjectDrag {
    target: ObjectTarget,
    grab: [f64; 2],
    expected: Option<[f64; 2]>,
    resize: Option<(scicanvas_core::page::ShapeObject, usize)>,
}
struct ObjectInteraction {
    hover: Option<ObjectTarget>,
    selected: Option<ObjectTarget>,
    action: Option<ObjectAction>,
    dragging: bool,
    block_pan: bool,
}
fn object_position(g: &Graph, target: ObjectTarget) -> Option<Option<[f64; 2]>> {
    match target {
        ObjectTarget::Text(id) => g.page.text(id).map(|t| Some(t.position)),
        ObjectTarget::Shape(id) => g.page.shape(id).map(|s| Some(s.origin())),
        ObjectTarget::Legend(i) => g.page.layers().get(i).map(|l| l.legend_position),
        ObjectTarget::ColorScale(i) => g
            .page
            .layers()
            .get(i)
            .map(|l| l.color_scale.as_ref().and_then(|c| c.position)),
    }
}
fn drag_object(d: &mut ObjectDrag, p: Pos2, g: &Graph, sc: &Scene) -> Option<ObjectAction> {
    if let Some((shape, handle)) = &d.resize {
        let (a, b) = shapes::resized(shape, *handle, [p.x as f64 / sc.w, p.y as f64 / sc.h]);
        d.expected = Some([a[0].min(b[0]), a[1].min(b[1])]);
        return Some(ObjectAction::ResizeShape(shape.id, a, b));
    }
    let object = sc.objects.iter().find(|o| o.target == d.target)?;
    let mut position = [
        (p.x as f64 - d.grab[0]).clamp(0., (sc.w - object.bounds[2]).max(0.)) / sc.w,
        (p.y as f64 - d.grab[1]).clamp(0., (sc.h - object.bounds[3]).max(0.)) / sc.h,
    ];
    if let ObjectTarget::Shape(id) = d.target {
        position = g.page.shape(id)?.clamp_origin(position);
    }
    let action =
        (object_position(g, d.target) != Some(Some(position))).then_some(ObjectAction::Move(d.target, position));
    d.expected = Some(position);
    action
}

fn interact_objects(
    ui: &mut egui::Ui,
    response: &egui::Response,
    g: &Graph,
    sc: &Scene,
    transform: egui::emath::TSTransform,
    config: ObjectUi,
) -> ObjectInteraction {
    if config.adding_shape.is_some() {
        return shapes::create(ui, response, g, sc, transform, config);
    }
    let id = view_id(g.id).with(("objects", config.epoch));
    let mut selected = ui.data(|d| d.get_temp::<ObjectTarget>(id));
    let hint = id.with("shape-hint");
    if let Some(shape) = config.selected_shape {
        if ui.data(|d| d.get_temp::<u64>(hint)) != Some(shape) {
            selected = Some(ObjectTarget::Shape(shape));
            ui.data_mut(|d| d.insert_temp(hint, shape));
        }
    } else {
        ui.data_mut(|d| d.remove::<u64>(hint));
    }
    if selected.is_some_and(|t| !sc.objects.iter().any(|o| o.target == t)) {
        selected = None;
    }
    let mut drag = ui.data(|d| d.get_temp::<ObjectDrag>(id.with("drag")));
    if drag
        .as_ref()
        .is_some_and(|d| object_position(g, d.target) != Some(d.expected))
    {
        drag = None;
    }
    let point = ui.input(|i| i.pointer.interact_pos()).map(|p| transform.inverse() * p);
    let handle = if response.contains_pointer() {
        shapes::handle_at(g, selected, point, sc, transform)
    } else {
        None
    };
    let hover = if response.contains_pointer() {
        handle.map(|(shape, _)| ObjectTarget::Shape(shape.id)).or_else(|| {
            point.and_then(|p| sc.object_at_tolerance(p.x as f64, p.y as f64, 4. / f64::from(transform.scaling)))
        })
    } else {
        None
    };
    let mut action = None;
    if response.contains_pointer() && ui.input(|i| i.pointer.primary_pressed() || i.pointer.secondary_pressed()) {
        selected = hover;
        response.request_focus();
        if !config.adding && ui.input(|i| i.pointer.primary_pressed()) {
            if let (Some(target), Some(p)) = (hover, point) {
                let object = sc.objects.iter().find(|o| o.target == target).unwrap();
                drag = Some(ObjectDrag {
                    target,
                    grab: [p.x as f64 - object.bounds[0], p.y as f64 - object.bounds[1]],
                    expected: object_position(g, target).flatten(),
                    resize: handle,
                });
            }
        }
    }
    let mut dragging = false;
    // The page is one egui widget; track clicks per object so rapidly switching objects
    // cannot turn a double-click on the new object into a page-wide triple-click.
    let double_clicked = if !config.adding && response.clicked() {
        if let (Some(target), Some(p)) = (hover.filter(|t| selected == Some(*t)), point) {
            let now = ui.input(|i| i.time);
            let at = transform * p;
            let key = id.with("click");
            let previous = ui.data(|d| d.get_temp::<(ObjectTarget, f64, Pos2)>(key));
            let double = previous
                .is_some_and(|(old, time, position)| old == target && now - time < 0.3 && position.distance(at) <= 6.0);
            ui.data_mut(|d| {
                if double {
                    d.remove::<(ObjectTarget, f64, Pos2)>(key);
                } else {
                    d.insert_temp(key, (target, now, at));
                }
            });
            double
        } else {
            false
        }
    } else {
        false
    };
    let block_pan = config.adding || drag.is_some() || (hover.is_some() && response.double_clicked());
    if config.adding && response.clicked() {
        if let Some(p) = point {
            action = Some(ObjectAction::AddText([
                (p.x as f64 / sc.w).clamp(0.0, 1.0),
                (p.y as f64 / sc.h).clamp(0.0, 1.0),
            ]));
        }
    } else if double_clicked {
        if let Some(target) = hover {
            action = Some(ObjectAction::Edit(target));
        }
    } else if response.dragged_by(egui::PointerButton::Primary)
        || response.drag_stopped_by(egui::PointerButton::Primary)
    {
        if let (Some(d), Some(p)) = (&mut drag, point) {
            action = drag_object(d, p, g, sc);
            dragging = response.dragged_by(egui::PointerButton::Primary);
        }
    }

    if config.keyboard && response.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Delete)) {
        action = selected.map(ObjectAction::Delete);
    }
    if (response.has_focus() || config.adding) && ui.input(|i| i.key_pressed(egui::Key::Escape)) {
        action = config.adding.then_some(ObjectAction::CancelAdd);
        selected = None;
        drag = None;
        dragging = false;
    }
    if !ui.input(|i| i.pointer.primary_down()) {
        drag = None;
    }
    ui.data_mut(|d| {
        d.remove::<ObjectTarget>(id);
        d.remove::<ObjectDrag>(id.with("drag"));
        if let Some(target) = selected {
            d.insert_temp(id, target);
        }
        if let Some(drag) = drag {
            d.insert_temp(id.with("drag"), drag);
        }
    });
    ObjectInteraction {
        hover,
        selected,
        action,
        dragging,
        block_pan,
    }
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

/// 在变换后的空间里（对数轴取 lg）平移 / 缩放一个范围。
/// shift 是按范围宽度计的平移比例，zoom 是缩放倍数，center 是缩放中心在范围里的位置（0..1）。
fn adjust(r: (f64, f64), scale: scicanvas_core::ticks::AxisScale, shift: f64, zoom: f64, center: f64) -> (f64, f64) {
    let (a, b) = (scale.forward(r.0), scale.forward(r.1));
    let span = b - a;
    let (a, b) = (a + shift * span, b + shift * span);
    let c = a + center * (b - a);
    let (a, b) = (c - (c - a) * zoom, c + (b - c) * zoom);
    let (a, b) = (scale.inverse(a), scale.inverse(b));
    if scale.range_ok(a, b) && (b - a) > 1e-12 * a.abs().max(b.abs()).max(1e-300) {
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
    painter.rect_filled(page, 0.0, PAGE_BG);
    painter.rect_stroke(
        page,
        0.0,
        Stroke::new(1.0_f32, Color32::from_gray(if dark { 20 } else { 110 })),
    );
}

#[derive(Clone, Copy)]
struct ViewInput<'a> {
    response: &'a egui::Response,
    transform: egui::emath::TSTransform,
    hovered: Option<usize>,
    dragging: Option<usize>,
    block_pan: bool,
}

fn update_saved_view(
    ui: &mut egui::Ui,
    id: egui::Id,
    g: &Graph,
    project: &Project,
    mut sc: std::sync::Arc<Scene>,
    saved: &mut [Option<Saved>],
    input: ViewInput<'_>,
) -> std::sync::Arc<Scene> {
    let mut changed = false;
    if let Some(index) = input.dragging.or(input.hovered).filter(|_| !input.block_pan) {
        let layer = &g.page.layers()[index];
        if !layer.projection.is_cartesian() || layer.plot_type == scicanvas_core::PlotType::Pie {
            return sc;
        }
        let layout = &sc.layers[index];
        if input.response.double_clicked() {
            saved[index] = None;
            changed = true;
        } else {
            let mut view = layout.view();
            if input.dragging.is_some() {
                let d = input.response.drag_delta();
                if d != egui::Vec2::ZERO {
                    view.x = adjust(
                        view.x,
                        layer.effective_x_scale(),
                        -d.x as f64 / f64::from(input.transform.scaling) / layout.plot[2],
                        1.0,
                        0.5,
                    );
                    view.y = adjust(
                        view.y,
                        layer.effective_y_scale(),
                        d.y as f64 / f64::from(input.transform.scaling) / layout.plot[3],
                        1.0,
                        0.5,
                    );
                    changed = true;
                }
            } else if let Some(p) = input.response.hover_pos() {
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
                    let (x, y) = {
                        let point = input.transform.inverse() * p;
                        (point.x as f64, point.y as f64)
                    };
                    let fx = ((x - layout.plot[0]) / layout.plot[2]).clamp(0.0, 1.0);
                    let fy = ((layout.plot[1] + layout.plot[3] - y) / layout.plot[3]).clamp(0.0, 1.0);
                    view.x = adjust(view.x, layer.effective_x_scale(), 0.0, zx, fx);
                    view.y = adjust(view.y, layer.effective_y_scale(), 0.0, zy, fy);
                    changed = true;
                }
            }
            if changed {
                saved[index] = Some(Saved {
                    view,
                    key: axis_key(layer),
                    offset: layer.offset,
                    spec: layer.spec.clone(),
                });
            }
        }
    }
    if changed {
        let views = saved.iter().map(|s| s.as_ref().map(|s| s.view)).collect::<Vec<_>>();
        sc = cached_layout(ui, id, g, project, views);
    }
    sc
}

pub fn show(ui: &mut egui::Ui, g: &Graph, project: &Project, selected: usize, objects: ObjectUi) -> PlotOutput {
    let adding = objects.adding || objects.adding_shape.is_some();
    let id = view_id(g.id);
    let available = ui.available_size().max(vec2(120.0, 90.0));
    let (canvas, _) = ui.allocate_exact_size(available, Sense::hover());
    let rect = page_rect(canvas, g.page.logical_size());
    paint_canvas(ui, canvas, rect);
    let response = ui.interact(rect, id.with(("page", objects.epoch)), Sense::click_and_drag());
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
            .is_some_and(|s| s.key != axis_key(layer) || s.offset != layer.offset || !s.spec.same_mapping(&layer.spec))
        {
            *state = None;
        }
    }
    let views = |saved: &[Option<Saved>]| saved.iter().map(|s| s.as_ref().map(|s| s.view)).collect::<Vec<_>>();
    let sc = cached_layout(ui, id, g, project, views(&saved));
    let object = interact_objects(ui, &response, g, &sc, transform, objects);
    let hovered = response.hover_pos().and_then(|p| {
        let (x, y) = local(p);
        sc.layer_at(x, y).filter(|index| sc.layers[*index].contains(x, y))
    });
    let selected_layer =
        if let Some(ObjectTarget::Legend(i)) = object.hover.filter(|_| ui.input(|i| i.pointer.any_pressed())) {
            Some(i)
        } else if object.hover.is_some() || adding {
            None
        } else if response.contains_pointer() && ui.input(|i| i.pointer.any_pressed()) {
            response.interact_pointer_pos().and_then(|p| {
                let (x, y) = local(p);
                sc.layer_at(x, y)
            })
        } else {
            None
        };
    // 拖动锁定按下时的图层，越过其他图层也继续操作原图层。
    let dragging = if !object.block_pan && response.dragged_by(egui::PointerButton::Primary) {
        ui.input(|i| i.pointer.press_origin()).and_then(|p| {
            let (x, y) = local(p);
            sc.layer_at(x, y).filter(|index| sc.layers[*index].contains(x, y))
        })
    } else {
        None
    };
    let input = ViewInput {
        response: &response,
        transform,
        hovered,
        dragging,
        block_pan: object.block_pan,
    };
    let sc = update_saved_view(ui, id, g, project, sc, &mut saved, input);
    paint_cached(ui, id, rect, &sc);
    if let Some(layout) = object.selected.and_then(|t| sc.objects.iter().find(|o| o.target == t)) {
        let [x, y, w, h] = layout.bounds;
        let selection = Rect::from_min_size(
            transform * pos2(x as f32, y as f32),
            vec2(w as f32, h as f32) * transform.scaling,
        );
        ui.painter().rect_stroke(
            selection.expand(2.0),
            0.0,
            Stroke::new(1.0_f32, ui.visuals().selection.bg_fill),
        );
    }
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
    let hover = hovered
        .filter(|_| object.hover.is_none() && !object.dragging && !adding)
        .and_then(|index| {
            response.hover_pos().map(|p| {
                let (x, y) = local(p);
                sc.layers[index].data_at(x, y)
            })
        });
    // 数据点提示：屏幕上 8 像素以内吸附到最近的数据点
    let point = match (hovered, response.hover_pos()) {
        (Some(index), Some(p)) if dragging.is_none() && !object.dragging && object.hover.is_none() && !adding => {
            let (x, y) = local(p);
            let layer = &g.page.layers()[index];
            scene::nearest_point(
                layer,
                &sc.layers[index],
                &|b| project.book(b),
                x,
                y,
                8.0 / f64::from(transform.scaling),
            )
        }
        _ => None,
    };
    if let Some(hit) = &point {
        let at = transform * pos2(hit.at[0] as f32, hit.at[1] as f32);
        ui.painter()
            .circle_stroke(at, 5.0, Stroke::new(1.5_f32, ui.visuals().selection.bg_fill));
        let mut text = hit.series.clone();
        if let Some(row) = hit.row {
            text.push_str(&format!("\n行 {}", row + 1));
        }
        text.push_str(&format!(
            "\nX = {}\nY = {}",
            scicanvas_core::fmt_num(hit.x),
            scicanvas_core::fmt_num(hit.y)
        ));
        egui::show_tooltip_at_pointer(ui.ctx(), ui.layer_id(), id.with("point-tip"), |ui| {
            ui.label(text);
        });
    }
    update_cursor(
        ui,
        hover.is_some(),
        response.dragged(),
        dragging.is_some(),
        object.dragging,
        adding,
        object.hover.is_some(),
    );
    shapes::draw_overlay(ui, g, object.selected, &sc, transform, objects.adding_shape.is_some());
    let zoomed = saved.iter().any(Option::is_some);
    ui.data_mut(|d| d.insert_temp(id, saved));
    PlotOutput {
        object: object.hover,
        action: object.action,
        object_dragging: object.dragging,
        response,
        hover,
        zoomed,
        selected_layer,
        hover_layer: hovered,
        point,
    }
}

fn update_cursor(
    ui: &egui::Ui,
    hovered: bool,
    response_dragged: bool,
    view_dragged: bool,
    object_dragged: bool,
    adding: bool,
    object_hovered: bool,
) {
    if hovered && !response_dragged {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
    }
    if view_dragged || object_dragged {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
    } else if adding {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Text);
    } else if object_hovered {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
    }
}

/// 把图元画到 rect 里。
pub fn paint(ui: &egui::Ui, rect: Rect, sc: &Scene) {
    paint_with_painter(ui.painter_at(rect), rect, sc);
}

fn paint_with_painter(free: egui::Painter, rect: Rect, sc: &Scene) {
    let transform = page_transform(rect, [sc.w, sc.h]);
    paint_items(free, transform, &sc.items);
}
fn paint_items(free: egui::Painter, transform: egui::emath::TSTransform, items: &[scene::Item]) {
    let scale = transform.scaling;
    let at = |x: f64, y: f64| transform * pos2(x as f32, y as f32);
    for it in items {
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
            Prim::StyledSymbols { symbols } => {
                for symbol in symbols {
                    let fill = symbol.fill.map_or(egui::Color32::TRANSPARENT, rgb);
                    let stroke = symbol.stroke.map_or(Stroke::NONE, |(color, width)| {
                        Stroke::new(width as f32 * scale, rgb(color))
                    });
                    let center = at(symbol.at[0], symbol.at[1]);
                    match scene::symbol_polygon(symbol.symbol, symbol.at[0], symbol.at[1], symbol.radius) {
                        None => {
                            p.circle(center, symbol.radius as f32 * scale, fill, stroke);
                        }
                        Some(polygon) => {
                            let polygon: Vec<Pos2> = polygon.iter().map(|point| at(point[0], point[1])).collect();
                            p.add(Shape::convex_polygon(polygon, fill, stroke));
                        }
                    }
                }
            }
            Prim::ColoredSegments { segments, width } => {
                for segment in segments {
                    p.line_segment(
                        [
                            at(segment.points[0], segment.points[1]),
                            at(segment.points[2], segment.points[3]),
                        ],
                        Stroke::new(*width as f32 * scale, rgb(segment.color)),
                    );
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
                rich,
                color,
            } => {
                let layout = scicanvas_core::text::layout_formatted(text, *size, *bold, *rich);
                let factor = match anchor {
                    Anchor::Start => 0.0,
                    Anchor::Middle => 0.5,
                    Anchor::End => 1.0,
                };
                let origin = at(*x, *y);
                for run in &layout.runs {
                    let raw = p.layout_no_wrap(
                        run.text.clone(),
                        plot_font(run.size as f32 * scale, run.font),
                        rgb(*color),
                    );
                    let baseline = raw
                        .rows
                        .first()
                        .and_then(|row| row.glyphs.first())
                        .map_or(0.0, |g| g.pos.y);
                    // Preserve native glyph proportions; pixel rounding must not stretch the glyph mesh.
                    let dx = (run.x - layout.widths[run.line] * factor) as f32 * scale;
                    let dy = (-layout.height / 2.0 + layout.ascent + run.line as f64 * layout.line_height + run.dy)
                        as f32
                        * scale
                        - baseline;
                    let pos = if *vertical {
                        origin + vec2(dy, -dx)
                    } else {
                        origin + vec2(dx, dy)
                    };
                    let shape = egui::epaint::TextShape::new(pos, raw, rgb(*color));
                    p.add(if *vertical { shape.with_angle(-FRAC_PI_2) } else { shape });
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn b6_view_changes_use_each_transform_space() {
        use scicanvas_core::ticks::AxisScale as S;
        for (scale, range) in [
            (S::Linear, (1., 10.)),
            (S::Log10, (1., 100.)),
            (S::Log2, (1., 16.)),
            (S::Ln, (1., 4f64.exp())),
            (S::Reciprocal, (1., 10.)),
            (S::Reciprocal, (-10., -1.)),
            (S::Probability, (1., 99.)),
            (S::Logit, (0.01, 0.99)),
        ] {
            let center = scale.inverse((scale.forward(range.0) + scale.forward(range.1)) / 2.);
            let zoom = super::adjust(range, scale, 0., 0.8, 0.5);
            assert!(scale.range_ok(zoom.0, zoom.1));
            let fraction =
                (scale.forward(center) - scale.forward(zoom.0)) / (scale.forward(zoom.1) - scale.forward(zoom.0));
            assert!((fraction - 0.5).abs() < 1e-10);
            let pan = super::adjust(range, scale, 0.1, 1., 0.5);
            let shift =
                (scale.forward(pan.0) - scale.forward(range.0)) / (scale.forward(range.1) - scale.forward(range.0));
            assert!((shift - 0.1).abs() < 1e-10);
        }
    }

    use super::*;

    fn project() -> Project {
        let mut p = Project::sample();
        let mut layer = p.graphs[0].page.layers()[0].clone();
        layer.edit_draft(|draft| draft.y_keys = vec!["C".into()]);
        p.graphs[0].page.add_layer(layer);
        p.graphs[0].page.columns = 2;
        p
    }

    fn frame(ctx: &egui::Context, p: &Project, events: Vec<egui::Event>) -> PlotOutput {
        let mut harness = crate::test_ui::Harness::new(ctx.clone(), Some(vec2(1000.0, 700.0)), ());
        harness
            .run(events, |ctx| {
                egui::CentralPanel::default()
                    .show(ctx, |ui| show(ui, &p.graphs[0], p, 0, ObjectUi::default()))
                    .inner
            })
            .1
    }

    fn button(pos: Pos2, pressed: bool) -> egui::Event {
        crate::test_ui::button(pos, pressed)
    }

    fn point(rect: Rect, sc: &Scene, index: usize) -> Pos2 {
        let [x, y, w, h] = sc.layers[index].plot;
        page_transform(rect, [sc.w, sc.h]) * pos2((x + w / 2.0) as f32, (y + h / 2.0) as f32)
    }

    #[test]
    fn objects_capture_drags_and_double_clicks_without_panning_the_axes() {
        let ctx = egui::Context::default();
        crate::fonts::setup(&ctx);
        let mut p = Project::sample();
        let id = p.graphs[0].page.add_text([0.25, 0.25]).unwrap();
        p.graphs[0].page.text_mut(id).unwrap().text = "Text".into();
        let initial = frame(&ctx, &p, vec![]);
        let object_point = |sc: &Scene, target: ObjectTarget, rect| {
            let o = sc.objects.iter().find(|o| o.target == target).unwrap();
            page_transform(rect, [sc.w, sc.h]) * pos2((o.bounds[0] + 4.0) as f32, (o.bounds[1] + 4.0) as f32)
        };
        for target in [ObjectTarget::Text(id), ObjectTarget::Legend(0)] {
            let sc = scene::layout_page(&p.graphs[0], &p, &[]);
            let at = object_point(&sc, target, initial.response.rect);
            frame(&ctx, &p, vec![egui::Event::PointerMoved(at), button(at, true)]);
            let end = at + vec2(70.0, -30.0);
            let moved = frame(&ctx, &p, vec![egui::Event::PointerMoved(end)]);
            assert!(moved.object_dragging);
            let Some(ObjectAction::Move(actual, position)) = moved.action else {
                panic!("object was not dragged")
            };
            assert_eq!(actual, target);
            match target {
                ObjectTarget::Text(id) => p.graphs[0].page.text_mut(id).unwrap().position = position,
                ObjectTarget::Legend(i) => p.graphs[0].page.layers_mut()[i].legend_position = Some(position),
                ObjectTarget::ColorScale(_) | ObjectTarget::Shape(_) => unreachable!(),
            }
            frame(&ctx, &p, vec![button(end, false)]);
            assert!(!frame(&ctx, &p, vec![]).zoomed);
            let after = scene::layout_page(&p.graphs[0], &p, &[]);
            let at = object_point(&after, target, initial.response.rect);
            let mut edits = Vec::new();
            let mut info = Vec::new();
            for _ in 0..2 {
                frame(&ctx, &p, vec![egui::Event::PointerMoved(at), button(at, true)]);
                let release = frame(&ctx, &p, vec![button(at, false)]);
                info.push((
                    release.object,
                    release.response.clicked(),
                    release.response.double_clicked(),
                    release.response.drag_stopped(),
                ));
                edits.push(release.action);
            }
            assert!(
                matches!(edits.last(), Some(Some(ObjectAction::Edit(t))) if *t == target),
                "{target:?}: {edits:?}, {info:?}, {at:?}"
            );
            assert!(!frame(&ctx, &p, vec![]).zoomed);
        }
        let sc = scene::layout_page(&p.graphs[0], &p, &[]);
        let at = object_point(&sc, ObjectTarget::Text(id), initial.response.rect);
        frame(&ctx, &p, vec![egui::Event::PointerMoved(at), button(at, true)]);
        frame(&ctx, &p, vec![button(at, false)]);
        let deleted = frame(
            &ctx,
            &p,
            vec![egui::Event::Key {
                key: egui::Key::Delete,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
        );
        assert!(matches!(deleted.action, Some(ObjectAction::Delete(ObjectTarget::Text(t))) if t == id));
        assert_eq!(sc.plot, scene::layout_page(&p.graphs[0], &p, &[]).plot);
    }

    #[test]
    fn cached_paint_reuses_moves_and_matches_direct_vector_output() {
        use std::sync::Arc;
        let ctx = egui::Context::default();
        crate::fonts::setup(&ctx);
        let p = Project::sample();
        let scene = Arc::new(scene::layout_page(&p.graphs[0], &p, &[]));
        let id = egui::Id::new("paint-regression");
        let render = |rect: Rect, cached: bool, scene: &Arc<Scene>| {
            let out = ctx.run(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1000.0, 800.0))),
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        if cached {
                            paint_cached(ui, id, rect, scene);
                        } else {
                            paint(ui, rect, scene);
                        }
                    });
                },
            );
            ctx.tessellate(out.shapes, out.pixels_per_point)
        };
        let first = Rect::from_min_size(pos2(20.0, 25.0), vec2(400.0, 300.0));
        render(first, false, &scene);
        render(first, true, &scene);
        let initial = ctx.data(|d| d.get_temp::<CachedPaint>(id.with("paint-cache"))).unwrap();
        for origin in [pos2(120.25, 90.5), pos2(-100.0, 75.0)] {
            let rect = Rect::from_min_size(origin, first.size());
            let cached = render(rect, true, &scene);
            let cache = ctx.data(|d| d.get_temp::<CachedPaint>(id.with("paint-cache"))).unwrap();
            assert!(
                Arc::ptr_eq(&initial.shapes, &cache.shapes),
                "moving only reuses vector instructions"
            );
            let direct = render(rect, false, &scene);
            assert_eq!(cached.len(), direct.len());
            for (a, b) in cached.iter().zip(&direct) {
                assert!((a.clip_rect.min - b.clip_rect.min).length() < 0.001);
                assert!((a.clip_rect.max - b.clip_rect.max).length() < 0.001);
                let (egui::epaint::Primitive::Mesh(a), egui::epaint::Primitive::Mesh(b)) = (&a.primitive, &b.primitive)
                else {
                    panic!("graph uses vectors")
                };
                assert_eq!(a.texture_id, b.texture_id);
                assert_eq!(a.indices, b.indices);
                assert_eq!(a.vertices.len(), b.vertices.len());
                for (a, b) in a.vertices.iter().zip(&b.vertices) {
                    assert!(a.pos.distance(b.pos) < 0.001);
                    assert_eq!(a.color, b.color);
                    assert_eq!(a.uv, b.uv);
                }
            }
        }
        let resized = Rect::from_min_size(first.min, vec2(480.0, 360.0));
        render(resized, true, &scene);
        let cache = ctx.data(|d| d.get_temp::<CachedPaint>(id.with("paint-cache"))).unwrap();
        assert!(!Arc::ptr_eq(&initial.shapes, &cache.shapes));
        ctx.set_pixels_per_point(1.5);
        render(resized, true, &scene);
        let dpi = ctx.data(|d| d.get_temp::<CachedPaint>(id.with("paint-cache"))).unwrap();
        assert!(!Arc::ptr_eq(&cache.shapes, &dpi.shapes));
        let changed = Arc::new((*scene).clone());
        render(resized, true, &changed);
        let cache = ctx.data(|d| d.get_temp::<CachedPaint>(id.with("paint-cache"))).unwrap();
        assert!(
            !Arc::ptr_eq(&cache.shapes, &dpi.shapes),
            "updated content rebuilds paint"
        );
    }

    #[test]
    fn cached_plot_tracks_color_size_and_shape_mapping_data_changes() {
        use scicanvas_core::{page::CurveStyle, ColumnData, Role};
        let ctx = egui::Context::default();
        crate::fonts::setup(&ctx);
        let mut p = Project::sample();
        let book = &mut p.books[0];
        let rows = book.nrows();
        let mut keys = Vec::new();
        for _ in 0..3 {
            keys.push(book.push_column(Role::Ignore));
            book.columns.last_mut().unwrap().data = (0..rows).map(|row| row.to_string()).collect();
        }
        book.columns.last_mut().unwrap().data = (ColumnData::Category {
            codes: (0..rows).map(|row| (row % 2) as u32).collect(),
            levels: vec!["first".into(), "second".into()],
            ordered: true,
        })
        .into();
        p.graphs[0].first_layer_mut().set_plot_style(
            "B",
            Some(CurveStyle {
                color_column: Some(keys[0].clone()),
                size_column: Some(keys[1].clone()),
                symbol_column: Some(keys[2].clone()),
                ..Default::default()
            }),
        );
        let render = |p: &Project| {
            let mut result = None;
            let _ = ctx.run(egui::RawInput::default(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    result = Some(cached_layout(ui, egui::Id::new("mapped-data"), &p.graphs[0], p, vec![]));
                });
            });
            result.unwrap()
        };
        let mut before = render(&p);
        for (key, value) in keys.iter().zip(["200", "100", "second"]) {
            p.books[0]
                .columns
                .iter_mut()
                .find(|col| col.key == *key)
                .unwrap()
                .set(0, value.into());
            let actual = render(&p);
            assert!(!std::sync::Arc::ptr_eq(&before, &actual));
            assert_eq!(*actual, scene::layout_page(&p.graphs[0], &p, &[]));
            before = actual;
        }
    }

    #[test]
    fn cached_plot_tracks_workbook_names_error_links_label_spelling_and_column_types() {
        use scicanvas_core::{Column, DataKind, Role};
        let ctx = egui::Context::default();
        crate::fonts::setup(&ctx);
        let mut p = Project::sample();
        let graph = p.graphs[0].id;
        let mut other = p.books[0].clone();
        other.id = 42;
        other.name = "Other".into();
        p.books.push(other);
        p.add_curves(graph, 0, 42, &["B".into()]).unwrap();
        let mut label = Column::new("D", Role::Label);
        label.data = vec!["1.0".to_string()].into();
        p.books[0].columns.push(label);
        let mut error = Column::new("E", Role::YError);
        error.data = vec!["0.5".to_string()].into();
        error.error_for = Some("B".into());
        p.books[0].columns.push(error);
        let layer = p.graphs[0].first_layer_mut();
        layer.labels.visible = true;
        layer.labels.source = scicanvas_core::page::LabelSource::Column;
        layer.labels.column = Some("D".into());
        let render = |p: &Project| {
            let mut sc = None;
            let _ = ctx.run(egui::RawInput::default(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    sc = Some(cached_layout(
                        ui,
                        egui::Id::new("cache-regression"),
                        &p.graphs[0],
                        p,
                        vec![],
                    ));
                });
            });
            sc.unwrap()
        };
        let mut before = render(&p);
        assert!(
            std::sync::Arc::ptr_eq(&before, &render(&p)),
            "unchanged plots reuse the cache"
        );
        let verify = |p: &Project, before: &Scene| {
            let expected = scene::layout_page(&p.graphs[0], p, &[]);
            assert_ne!(before, &expected, "the edit affects the actual rendered scene");
            let actual = render(p);
            assert_eq!(*actual, expected, "screen cache matches a fresh/export layout");
            actual
        };
        p.books[1].name = "Renamed".into();
        before = verify(&p, &before);
        p.books[0].columns[4].error_for = Some("C".into());
        before = verify(&p, &before);
        p.books[0].columns[3].data.set_text(0, "1.00".into());
        verify(&p, &before);
        // An unused Z column changes automatic error association without becoming plotted data.
        p.books[0].columns[4].error_for = None;
        let error = p.books[0].columns.pop().unwrap();
        p.books[0].columns.insert(3, error);
        before = render(&p);
        p.books[0].columns.insert(3, Column::new("F", Role::Z));
        verify(&p, &before);
        p.books[0].columns[0].data = vec!["0".to_string(), "86400000".into()].into();
        before = render(&p);
        p.books[0].columns[0].data =
            (scicanvas_core::ColumnData::from(vec!["1970-01-01".to_string(), "1970-01-02".into()])
                .converted(DataKind::DateTime)
                .unwrap())
            .into();
        verify(&p, &before);
    }

    #[test]
    fn text_preserves_native_glyph_geometry_at_different_zoom_and_dpi() {
        let mut p = Project::sample();
        p.graphs[0].first_layer_mut().title = r"WWW iii μ ± 中文 CO_{2} R^{2} \b{normal} \i{italic}".into();
        p.graphs[0].first_layer_mut().rich_text = true;
        let annotation = p.graphs[0].page.add_text([0.2, 0.45]).unwrap();
        p.graphs[0].page.text_mut(annotation).unwrap().text = "Note_{1} α\n\\i{second}".into();
        p.graphs[0].first_layer_mut().legend_position = Some([0.6, 0.55]);
        p.graphs[0].first_layer_mut().y_title = r"\i{c} / mol L^{−1}".into();
        let sc = scene::layout_page(&p.graphs[0], &p, &[]);
        let svg = scicanvas_core::svg::render_page(&p.graphs[0], &p);
        use resvg::usvg::{Group, Node};
        let tree = crate::export::svg_tree(&svg).unwrap();
        let mut svg_runs = Vec::new();
        fn collect(group: &Group, runs: &mut Vec<(String, Pos2, f32)>) {
            for node in group.children() {
                match node {
                    Node::Group(g) => collect(g, runs),
                    Node::Text(t) => {
                        for chunk in t.chunks() {
                            if !chunk.text().trim().is_empty() {
                                let mut point =
                                    resvg::tiny_skia::Point::from_xy(chunk.x().unwrap(), chunk.y().unwrap());
                                t.abs_transform().map_point(&mut point);
                                runs.push((
                                    chunk.text().to_owned(),
                                    pos2(point.x, point.y),
                                    chunk.spans()[0].font_size().get(),
                                ));
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        collect(tree.root(), &mut svg_runs);
        let expected: Vec<_> = sc
            .items
            .iter()
            .flat_map(|item| {
                if let Prim::Text {
                    text, size, bold, rich, ..
                } = &item.prim
                {
                    scicanvas_core::text::layout_formatted(text, *size, *bold, *rich)
                        .runs
                        .into_iter()
                        .filter(|run| !run.text.trim().is_empty())
                        .map(|run| (run.font, run.text, run.width, run.size))
                        .collect::<Vec<_>>()
                } else {
                    Vec::new()
                }
            })
            .collect();
        for (scale, dpi) in [(0.5_f32, 1.0_f32), (0.85, 1.25), (1.1, 1.5), (2.0, 2.0)] {
            let ctx = egui::Context::default();
            crate::fonts::setup(&ctx);
            ctx.set_pixels_per_point(dpi);
            let rect = Rect::from_min_size(Pos2::ZERO, vec2(sc.w as f32, sc.h as f32) * scale);
            let out = ctx.run(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(2400.0, 1800.0))),
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| paint(ui, rect, &sc));
                },
            );
            let mut checked = 0;
            for clipped in out.shapes {
                if let Shape::Text(shape) = clipped.shape {
                    if shape.galley.job.text.trim().is_empty() {
                        continue;
                    }
                    let id = &shape.galley.job.sections[0].format.font_id;
                    if let egui::FontFamily::Name(name) = &id.family {
                        if let Some(i) = name.strip_prefix("plot-").and_then(|s| s.parse::<usize>().ok()) {
                            let em_size = id.size / scale;
                            let _ = expected
                                .iter()
                                .find(|(face, text, _, size)| {
                                    *face == i
                                        && text == &shape.galley.job.text
                                        && (*size as f32 - em_size).abs() < 1e-3
                                })
                                .expect("screen uses a shared page run");
                            let native = ctx.fonts(|fonts| fonts.layout_job((*shape.galley.job).clone()));
                            assert_eq!(shape.galley.rect, native.rect);
                            assert_eq!(shape.galley.rows.len(), native.rows.len());
                            for (actual, original) in shape.galley.rows.iter().zip(&native.rows) {
                                assert_eq!(actual.visuals.mesh.vertices, original.visuals.mesh.vertices);
                                assert_eq!(actual.visuals.mesh.indices, original.visuals.mesh.indices);
                            }
                            let (svg_text, svg_at, svg_size) = &svg_runs[checked];
                            assert_eq!(&shape.galley.job.text, svg_text);
                            assert!((id.size / scale - svg_size).abs() < 1e-3);
                            let baseline = shape.galley.rows[0].glyphs[0].pos.y;
                            let (sin, cos) = shape.angle.sin_cos();
                            let baseline_at = (shape.pos.to_vec2() + vec2(-sin * baseline, cos * baseline)) / scale;
                            assert!(
                                (baseline_at - svg_at.to_vec2()).length() < 0.03,
                                "screen/SVG baseline: {svg_text}"
                            );
                            checked += 1;
                        }
                    }
                }
            }
            assert_eq!(checked, expected.len());
            assert_eq!(checked, svg_runs.len());
        }
    }

    #[test]
    fn pointer_near_a_data_point_snaps_and_reports_its_row() {
        let ctx = egui::Context::default();
        crate::fonts::setup(&ctx);
        let p = project();
        let initial = frame(&ctx, &p, vec![]);
        let sc = scene::layout_page(&p.graphs[0], &p, &[]);
        // 第二个图层画 C 列；取第 6 行（下标 5）的数据点，鼠标偏开 3 像素
        let layer = &p.graphs[0].page.layers()[1];
        let (_, series) = layer.series(p.book(1).unwrap());
        let s = &series[0];
        let view = &sc.layers[1];
        let map = scicanvas_core::coord::Cartesian {
            x: &view.axes.x,
            y: &view.axes.left_y,
            rect: view.plot,
        };
        use scicanvas_core::coord::Coord;
        let at = map.project(s.points[5]);
        let screen = page_transform(initial.response.rect, [sc.w, sc.h]) * pos2(at[0] as f32, at[1] as f32);
        let output = frame(&ctx, &p, vec![egui::Event::PointerMoved(screen + vec2(3.0, -2.0))]);
        let hit = output.point.expect("snapped point");
        assert_eq!(hit.row, Some(s.rows[5]));
        assert_eq!((hit.x, hit.y), (s.points[5][0], s.points[5][1]));
        // 远离数据点时不吸附
        let [x, y, w, _] = view.plot;
        let empty = page_transform(initial.response.rect, [sc.w, sc.h]) * pos2((x + w * 0.5) as f32, (y + 2.0) as f32);
        let far = frame(&ctx, &p, vec![egui::Event::PointerMoved(empty)]);
        assert!(far.point.is_none() || far.point.unwrap().at[1] > y + 2.0);
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
    fn curve_style_edits_preserve_saved_zoom_but_mapping_edits_reset_it() {
        let ctx = egui::Context::default();
        crate::fonts::setup(&ctx);
        let mut p = Project::sample();
        frame(&ctx, &p, vec![]);
        let layer = p.graphs[0].first_layer();
        let view = View {
            x: (2.0, 8.0),
            y: (-2.0, 4.0),
        };
        let state = Saved {
            view,
            offset: None,
            key: axis_key(layer),
            spec: layer.spec.clone(),
        };
        ctx.data_mut(|d| d.insert_temp(view_id(2), vec![Some(state)]));
        for key in ["B", "style-only"] {
            p.graphs[0].first_layer_mut().set_plot_style(
                key,
                Some(scicanvas_core::page::CurveStyle {
                    color: Some([90, 20, 30]),
                    right_axis: true,
                    ..Default::default()
                }),
            );
            assert!(frame(&ctx, &p, vec![]).zoomed);
            let state = ctx.data(|d| d.get_temp::<Vec<Option<Saved>>>(view_id(2))).unwrap();
            assert_eq!(state[0].as_ref().unwrap().view, view);
        }
        p.graphs[0].first_layer_mut().x_key = None;
        assert!(!frame(&ctx, &p, vec![]).zoomed);
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
        p.graphs[0]
            .first_layer_mut()
            .edit_draft(|draft| draft.y_keys = vec!["B".into()]);
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
                        output = Some(show(ui, &p.graphs[0], &p, 0, ObjectUi::default()));
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
            // 页面版面按页宽整体放大 k 倍，再按窗口缩放 scale 倍
            let k = p.graphs[0].page.scale() as f32;
            assert!((stroke / scale - scene::AXIS_WIDTH as f32 * k).abs() < 1e-4);
            assert!((font / scale - scene::TICK_FONT as f32 * k).abs() < 1e-4);
            let transform = page_transform(rect, [sc.w, sc.h]);
            let page_point = pos2(sc.px(10.0) as f32, sc.py(0.0) as f32);
            let recovered = transform.inverse() * (transform * page_point);
            assert!(recovered.distance(page_point) < 1e-4);
        }
    }
}
