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
        for (index, (point, base)) in series.points.iter().zip(&series.bases).enumerate() {
            let color = series
                .point_styles
                .get(index)
                .and_then(|style| style.color)
                .map_or(color, |[r, g, b]| (r, g, b));
            let (category, value, category_axis, value_axis) = if ctx.horizontal {
                (point[1], point[0], &ctx.axes.left_y, &ctx.axes.x)
            } else {
                (point[0], point[1], &ctx.axes.x, ctx.y_axis(style.y_axis))
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
                    [ctx.px(base), ctx.py(category(start), style.y_axis)],
                    [ctx.px(value), ctx.py(category(end), style.y_axis)],
                )
            } else {
                (
                    [ctx.px(category(start)), ctx.py(base, style.y_axis)],
                    [ctx.px(category(end)), ctx.py(value, style.y_axis)],
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
        let axis = ctx.y_axis(style.y_axis);
        let base = ctx.py(
            if axis.log {
                axis.lo
            } else {
                0.0f64.clamp(axis.lo, axis.hi)
            },
            style.y_axis,
        );
        if series.point_styles.iter().any(|point| point.color.is_some()) {
            mapped_area(ctx, series, style, base, items);
            return true;
        }
        for run in crate::stat::data_runs(series) {
            let points: Vec<_> = run
                .iter()
                .map(|point| [ctx.px(point[0]), ctx.py(point[1], style.y_axis)])
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

/// 每个点的颜色覆盖相邻边的半段；分段与原面积图使用相同的投影和三角剖分。
fn mapped_area(
    ctx: &Context<'_, '_, '_, '_>,
    series: &Series,
    style: &SeriesStyle,
    base: f64,
    items: &mut Vec<crate::scene::Item>,
) {
    for range in crate::stat::row_runs(series) {
        let mut indices: Vec<_> = range.collect();
        indices.sort_by(|a, b| series.points[*a][0].total_cmp(&series.points[*b][0]));
        // 同一段按 X 排序后填充互不重叠，按颜色合并图元；各缺失行分段仍按原顺序叠放。
        let mut fills: std::collections::BTreeMap<_, Vec<_>> = std::collections::BTreeMap::new();
        let mut segments = Vec::new();
        for pair in indices.windows(2) {
            let project = |index: usize| {
                let point = series.points[index];
                [ctx.px(point[0]), ctx.py(point[1], style.y_axis)]
            };
            let (a, b) = (project(pair[0]), project(pair[1]));
            let mid = [a[0] / 2.0 + b[0] / 2.0, a[1] / 2.0 + b[1] / 2.0];
            for (index, from, to) in [(pair[0], a, mid), (pair[1], mid, b)] {
                let color = series.point_styles[index]
                    .color
                    .map_or(style.color.rgb(), |[r, g, b]| (r, g, b));
                fills
                    .entry(color)
                    .or_default()
                    .extend(area_triangles(&[from, to], base));
                segments.push(crate::scene::ColoredSegment {
                    points: [from[0], from[1], to[0], to[1]],
                    color,
                });
            }
        }
        for (color, pts) in fills.into_iter().filter(|(_, pts)| !pts.is_empty()) {
            ctx.push(
                items,
                Prim::Triangles {
                    pts,
                    color,
                    opacity: f64::from(ctx.layer.area_opacity),
                },
                true,
            );
        }
        if !segments.is_empty() {
            ctx.push(
                items,
                Prim::ColoredSegments {
                    segments,
                    width: f64::from(style.line_width),
                },
                true,
            );
        }
    }
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
