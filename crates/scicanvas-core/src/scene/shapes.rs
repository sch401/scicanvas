use super::*;
use crate::page::{ShapeKind, ShapeObject};
fn outline(points: Vec<[f64; 2]>, color: Rgb, width: f64, dashed: bool) -> Prim {
    if !dashed {
        return Prim::Polyline {
            pts: points,
            color,
            width,
        };
    }
    let mut segs = Vec::new();
    let dash = width.max(1.) * 4.;
    let gap = width.max(1.) * 3.;
    let period = dash + gap;
    let mut phase = 0.;
    for pair in points.windows(2) {
        let a = pair[0];
        let b = pair[1];
        let length = (b[0] - a[0]).hypot(b[1] - a[1]);
        let mut along = 0.;
        while along < length {
            let on = phase < dash;
            let boundary = if on { dash } else { period };
            let step = (boundary - phase).min(length - along);
            if step <= 0. {
                phase = 0.;
                continue;
            }
            if on {
                let at = |d: f64| [a[0] + (b[0] - a[0]) * d / length, a[1] + (b[1] - a[1]) * d / length];
                let p = at(along);
                let q = at(along + step);
                segs.push([p[0], p[1], q[0], q[1]]);
            }
            along += step;
            phase = (phase + step).rem_euclid(period);
        }
    }
    Prim::Segments {
        segs,
        color,
        width,
        dashed: false,
    }
}
pub fn shape_primitives(shape: &ShapeObject, w: f64, h: f64) -> Vec<Prim> {
    let a = [shape.a[0] * w, shape.a[1] * h];
    let b = [shape.b[0] * w, shape.b[1] * h];
    let color = (shape.stroke[0], shape.stroke[1], shape.stroke[2]);
    let width = shape.width_pt * 96. / 72.;
    let fill = shape.fill.map(|([r, g, b], alpha)| ((r, g, b), alpha));
    let mut items = Vec::new();
    match shape.kind {
        ShapeKind::Line | ShapeKind::Arrow => {
            let length = (b[0] - a[0]).hypot(b[1] - a[1]);
            if shape.kind == ShapeKind::Arrow && length > 0. && shape.head_pt > 0. {
                let size = (shape.head_pt * 96. / 72.).max(width * 2.).min(length);
                let u = [(b[0] - a[0]) / length, (b[1] - a[1]) / length];
                let base = [b[0] - size * u[0], b[1] - size * u[1]];
                let shaft_end = [b[0] - 2. * size / 3. * u[0], b[1] - 2. * size / 3. * u[1]];
                items.push(outline(vec![a, shaft_end], color, width, shape.dashed));
                items.push(Prim::Triangles {
                    pts: vec![
                        b,
                        [base[0] - size * 0.45 * u[1], base[1] + size * 0.45 * u[0]],
                        [base[0] + size * 0.45 * u[1], base[1] - size * 0.45 * u[0]],
                    ],
                    color,
                    opacity: 1.,
                });
            } else {
                items.push(outline(vec![a, b], color, width, shape.dashed));
            }
        }
        ShapeKind::Rect => {
            let x = a[0].min(b[0]);
            let y = a[1].min(b[1]);
            let w = (b[0] - a[0]).abs();
            let h = (b[1] - a[1]).abs();
            items.push(Prim::Rect {
                x,
                y,
                w,
                h,
                fill,
                stroke: (!shape.dashed).then_some((color, width)),
            });
            if shape.dashed {
                items.push(outline(
                    vec![[x, y], [x + w, y], [x + w, y + h], [x, y + h], [x, y]],
                    color,
                    width,
                    true,
                ));
            }
        }
        ShapeKind::Ellipse => {
            let center = [(a[0] + b[0]) / 2., (a[1] + b[1]) / 2.];
            let radius = [(b[0] - a[0]).abs() / 2., (b[1] - a[1]).abs() / 2.];
            let points: Vec<_> = (0..=96)
                .map(|i| {
                    let angle = std::f64::consts::TAU * i as f64 / 96.;
                    [center[0] + radius[0] * angle.cos(), center[1] + radius[1] * angle.sin()]
                })
                .collect();
            if let Some((color, opacity)) = fill {
                items.push(Prim::Triangles {
                    pts: points.windows(2).flat_map(|p| [center, p[0], p[1]]).collect(),
                    color,
                    opacity,
                });
            }
            items.push(outline(points, color, width, shape.dashed));
        }
    }
    items
}
pub(super) fn append(page: &mut Scene, g: &Graph) {
    for shape in &g.page.shapes {
        let start = page.items.len();
        page.items
            .extend(shape_primitives(shape, page.w, page.h).into_iter().map(|prim| Item {
                prim,
                clip: Some([0., 0., page.w, page.h]),
            }));
        let origin = shape.origin();
        let span = shape.span();
        page.objects.push(ObjectLayout {
            target: ObjectTarget::Shape(shape.id),
            bounds: [
                origin[0] * page.w,
                origin[1] * page.h,
                span[0] * page.w,
                span[1] * page.h,
            ],
            items: start..page.items.len(),
            overlay: true,
            hit_segment: matches!(shape.kind, ShapeKind::Line | ShapeKind::Arrow).then_some([
                [shape.a[0] * page.w, shape.a[1] * page.h],
                [shape.b[0] * page.w, shape.b[1] * page.h],
            ]),
        });
    }
}
