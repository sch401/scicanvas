use super::*;
use scicanvas_core::page::{ShapeKind, ShapeObject};
fn positions(shape: &ShapeObject) -> Vec<[f64; 2]> {
    if matches!(shape.kind, ShapeKind::Line | ShapeKind::Arrow) {
        vec![shape.a, shape.b]
    } else {
        let o = shape.origin();
        let s = shape.span();
        vec![o, [o[0] + s[0], o[1]], [o[0] + s[0], o[1] + s[1]], [o[0], o[1] + s[1]]]
    }
}
pub(super) fn handle_at(
    g: &Graph,
    selected: Option<ObjectTarget>,
    point: Option<Pos2>,
    sc: &Scene,
    transform: egui::emath::TSTransform,
) -> Option<(ShapeObject, usize)> {
    let ObjectTarget::Shape(id) = selected? else {
        return None;
    };
    let shape = *g.page.shape(id)?;
    let point = point?;
    let radius = 6. / transform.scaling;
    positions(&shape)
        .iter()
        .position(|p| pos2((p[0] * sc.w) as f32, (p[1] * sc.h) as f32).distance(point) <= radius)
        .map(|i| (shape, i))
}
pub(super) fn resized(shape: &ShapeObject, handle: usize, point: [f64; 2]) -> ([f64; 2], [f64; 2]) {
    let point = point.map(|v| v.clamp(0., 1.));
    if matches!(shape.kind, ShapeKind::Line | ShapeKind::Arrow) {
        if handle == 0 {
            (point, shape.b)
        } else {
            (shape.a, point)
        }
    } else {
        (point, positions(shape)[(handle + 2) % 4])
    }
}
pub(super) fn draw_overlay(
    ui: &egui::Ui,
    g: &Graph,
    selected: Option<ObjectTarget>,
    sc: &Scene,
    transform: egui::emath::TSTransform,
    adding: bool,
) {
    if adding {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
    }
    let Some(ObjectTarget::Shape(id)) = selected else {
        return;
    };
    let Some(shape) = g.page.shape(id) else { return };
    for p in positions(shape) {
        let at = transform * pos2((p[0] * sc.w) as f32, (p[1] * sc.h) as f32);
        let rect = Rect::from_center_size(at, vec2(7., 7.));
        ui.painter()
            .rect_filled(rect, 0., ui.visuals().widgets.inactive.bg_fill);
        ui.painter()
            .rect_stroke(rect, 0., Stroke::new(1.0_f32, ui.visuals().selection.bg_fill));
    }
}
pub(super) fn create(
    ui: &mut egui::Ui,
    response: &egui::Response,
    g: &Graph,
    sc: &Scene,
    transform: egui::emath::TSTransform,
    config: ObjectUi,
) -> ObjectInteraction {
    let kind = config.adding_shape.unwrap();
    let id = view_id(g.id).with(("objects", config.epoch)).with("shape-add");
    let mut start = ui.data(|d| d.get_temp::<[f64; 2]>(id));
    let point = ui.input(|i| i.pointer.interact_pos()).map(|p| transform.inverse() * p);
    let mut action = None;
    if response.contains_pointer() && ui.input(|i| i.pointer.primary_pressed()) {
        if let Some(p) = point.filter(|p| p.x >= 0. && p.y >= 0. && p.x <= sc.w as f32 && p.y <= sc.h as f32) {
            start = Some([p.x as f64 / sc.w, p.y as f64 / sc.h]);
            response.request_focus();
        }
    }
    if let (Some(a), Some(p)) = (start, point) {
        let b = [(p.x as f64 / sc.w).clamp(0., 1.), (p.y as f64 / sc.h).clamp(0., 1.)];
        let mut preview = ShapeObject::new(kind, a, b);
        let color = ui.visuals().selection.bg_fill;
        preview.stroke = [color.r(), color.g(), color.b()];
        if matches!(kind, ShapeKind::Rect | ShapeKind::Ellipse) {
            preview.fill = Some((preview.stroke, 0.1));
        }
        let items: Vec<_> = scene::shape_primitives(&preview, sc.w, sc.h)
            .into_iter()
            .map(|prim| scene::Item {
                prim,
                clip: Some([0., 0., sc.w, sc.h]),
            })
            .collect();
        paint_items(ui.painter().clone(), transform, &items);
        if response.drag_stopped_by(egui::PointerButton::Primary) {
            if ((a[0] - b[0]) * sc.w).hypot((a[1] - b[1]) * sc.h) * f64::from(transform.scaling) >= 2. {
                action = Some(ObjectAction::AddShape(kind, a, b));
            }
            start = None;
        }
    }
    if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
        action = Some(ObjectAction::CancelAdd);
        start = None;
    }
    let dragging = start.is_some() && ui.input(|i| i.pointer.primary_down());
    if !ui.input(|i| i.pointer.primary_down()) {
        start = None;
    }
    ui.data_mut(|d| {
        d.remove::<[f64; 2]>(id);
        if let Some(start) = start {
            d.insert_temp(id, start);
        }
    });
    ObjectInteraction {
        hover: None,
        selected: None,
        action,
        dragging,
        block_pan: true,
    }
}
