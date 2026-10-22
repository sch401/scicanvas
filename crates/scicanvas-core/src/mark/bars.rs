use super::{Context, Kind};
use crate::model::{Series, SeriesStyle};
use crate::scene::Prim;

pub(super) fn draw(
    ctx: &Context<'_, '_, '_, '_>,
    series: &Series,
    style: &SeriesStyle,
    items: &mut Vec<crate::scene::Item>,
) -> bool {
    if super::contains(style.kind, Kind::Bar) {
        let position = crate::presets::recipe(style.kind).position;
        let color = style.color.rgb();
        for (point, base) in series.points.iter().zip(&series.bases) {
            let (category, value, category_axis, value_axis) = if ctx.horizontal {
                (point[1], point[0], ctx.ya, ctx.xa)
            } else {
                (point[0], point[1], ctx.xa, ctx.y_axis(style.right))
            };
            let (start, end) = position.span(category, ctx.dx, style.bar_index, style.bar_count);
            let category = |value: f64| {
                if category_axis.log {
                    value.max(category_axis.lo)
                } else {
                    value
                }
            };
            let base = base.clamp(value_axis.lo, value_axis.hi);
            let value = if value_axis.log {
                value.max(value_axis.lo)
            } else {
                value
            };
            let (a, b) = if ctx.horizontal {
                (
                    [ctx.px(base), ctx.py(category(start), style.right)],
                    [ctx.px(value), ctx.py(category(end), style.right)],
                )
            } else {
                (
                    [ctx.px(category(start)), ctx.py(base, style.right)],
                    [ctx.px(category(end)), ctx.py(value, style.right)],
                )
            };
            ctx.push(
                items,
                Prim::Rect {
                    x: a[0].min(b[0]),
                    y: a[1].min(b[1]),
                    w: (b[0] - a[0]).abs(),
                    h: (a[1] - b[1]).abs(),
                    fill: Some((color, 1.0)),
                    stroke: Some(((0, 0, 0), 0.5)),
                },
                true,
            );
        }
        return true;
    }
    if super::contains(style.kind, Kind::Area) {
        let axis = ctx.y_axis(style.right);
        let base = ctx.py(
            if axis.log {
                axis.lo
            } else {
                0.0f64.clamp(axis.lo, axis.hi)
            },
            style.right,
        );
        for run in crate::stat::data_runs(series) {
            let points: Vec<_> = run
                .iter()
                .map(|point| [ctx.px(point[0]), ctx.py(point[1], style.right)])
                .collect();
            let triangles = area_triangles(&points, base);
            if !triangles.is_empty() {
                ctx.push(
                    items,
                    Prim::Triangles {
                        pts: triangles,
                        color: style.color.rgb(),
                        opacity: f64::from(ctx.layer.area_opacity),
                    },
                    true,
                );
            }
            if points.len() > 1 {
                ctx.push(
                    items,
                    Prim::Polyline {
                        pts: points,
                        color: style.color.rgb(),
                        width: f64::from(style.line_width),
                    },
                    true,
                );
            }
        }
        return true;
    }
    false
}

fn area_triangles(points: &[[f64; 2]], base: f64) -> Vec<[f64; 2]> {
    let mut out = Vec::new();
    for pair in points.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        if a[0] == b[0] || (a[1] == base && b[1] == base) {
            continue;
        }
        if (a[1] < base && b[1] > base) || (a[1] > base && b[1] < base) {
            let cross = [a[0] + (b[0] - a[0]) * ((base - a[1]) / (b[1] - a[1])), base];
            out.extend([a, cross, [a[0], base], b, [b[0], base], cross]);
        } else {
            out.extend([a, b, [b[0], base], a, [b[0], base], [a[0], base]]);
        }
    }
    out
}
