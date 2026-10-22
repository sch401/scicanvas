use super::{Context, Kind, Path};
use crate::model::{Series, SeriesStyle};
use crate::scene::Prim;

pub(super) fn draw(
    ctx: &Context<'_, '_, '_, '_>,
    series: &Series,
    style: &SeriesStyle,
    splines: &[Vec<[f64; 2]>],
    items: &mut Vec<crate::scene::Item>,
) -> Option<Prim> {
    let recipe = crate::presets::recipe(style.kind);
    let color = style.color.rgb();
    if recipe.marks.contains(&Kind::Drop) {
        let axis = ctx.y_axis(style.y_axis);
        let base = ctx.py(
            if axis.log {
                axis.lo
            } else {
                0.0f64.clamp(axis.lo, axis.hi)
            },
            style.y_axis,
        );
        ctx.push(
            items,
            Prim::Segments {
                segs: series
                    .points
                    .iter()
                    .map(|point| [ctx.px(point[0]), base, ctx.px(point[0]), ctx.py(point[1], style.y_axis)])
                    .collect(),
                color,
                width: f64::from(style.line_width),
                dashed: false,
            },
            true,
        );
    }
    if recipe.path == Path::Natural {
        for path in splines {
            for run in path.split(|point| (ctx.layer.x_log && point[0] <= 0.0) || (ctx.layer.y_log && point[1] <= 0.0))
            {
                if run.len() > 1 {
                    ctx.push(
                        items,
                        Prim::Polyline {
                            pts: run
                                .iter()
                                .map(|point| [ctx.px(point[0]), ctx.py(point[1], style.y_axis)])
                                .collect(),
                            color,
                            width: f64::from(style.line_width),
                        },
                        true,
                    );
                }
            }
        }
    } else if recipe.marks.contains(&Kind::Line) {
        for range in crate::stat::row_runs(series)
            .into_iter()
            .filter(|range| range.len() > 1)
        {
            let run = &series.points[range];
            let points = if matches!(recipe.path, Path::StepHorizontal | Path::StepVertical) {
                let mut points = vec![[ctx.px(run[0][0]), ctx.py(run[0][1], style.y_axis)]];
                for pair in run.windows(2) {
                    let bend = if recipe.path == Path::StepHorizontal {
                        [pair[1][0], pair[0][1]]
                    } else {
                        [pair[0][0], pair[1][1]]
                    };
                    points.push([ctx.px(bend[0]), ctx.py(bend[1], style.y_axis)]);
                    points.push([ctx.px(pair[1][0]), ctx.py(pair[1][1], style.y_axis)]);
                }
                points
            } else {
                let curve = match ctx.layer.connect {
                    crate::page::Connect::BSpline if !series.line_only => crate::spline::bspline(run, 16),
                    crate::page::Connect::Bezier if !series.line_only => crate::spline::bezier(run, 16),
                    _ => run.to_vec(),
                };
                decimate_line(
                    curve
                        .iter()
                        .map(|point| [ctx.px(point[0]), ctx.py(point[1], style.y_axis)])
                        .collect(),
                    ctx.rect[2],
                )
            };
            ctx.push(
                items,
                Prim::Polyline {
                    pts: points,
                    color,
                    width: f64::from(style.line_width),
                },
                true,
            );
        }
    }
    recipe.marks.contains(&Kind::Symbol).then(|| {
        let points = decimate_symbols(
            series
                .points
                .iter()
                .map(|point| [ctx.px(point[0]), ctx.py(point[1], style.y_axis)])
                .filter(|point| ctx.inside_x(point[0]) && ctx.inside_y(point[1]))
                .collect(),
            ctx.rect[2],
        );
        Prim::Symbols {
            sym: style.symbol,
            pts: points,
            r: f64::from(style.symbol_size) / 2.0,
            color,
        }
    })
}

pub(crate) fn decimate_line(pts: Vec<[f64; 2]>, width: f64) -> Vec<[f64; 2]> {
    if (pts.len() as f64) <= 4.0 * width.max(1.0) {
        return pts;
    }
    let mut out = Vec::with_capacity((4.0 * width) as usize + 8);
    let mut start = 0;
    while start < pts.len() {
        let column = pts[start][0].floor();
        let mut end = start + 1;
        while end < pts.len() && pts[end][0].floor() == column {
            end += 1;
        }
        let group = &pts[start..end];
        let (mut lo, mut hi) = (0, 0);
        for (i, point) in group.iter().enumerate() {
            if point[1] < group[lo][1] {
                lo = i;
            }
            if point[1] > group[hi][1] {
                hi = i;
            }
        }
        let mut keep = vec![0, lo, hi, group.len() - 1];
        keep.sort_unstable();
        keep.dedup();
        out.extend(keep.into_iter().map(|i| group[i]));
        start = end;
    }
    out
}

pub(crate) fn decimate_symbols(pts: Vec<[f64; 2]>, width: f64) -> Vec<[f64; 2]> {
    if (pts.len() as f64) <= 4.0 * width.max(1.0) {
        return pts;
    }
    let mut seen = std::collections::HashSet::with_capacity(pts.len().min(1 << 20));
    pts.into_iter()
        .filter(|point| seen.insert(((point[0] * 2.0).round() as i64, (point[1] * 2.0).round() as i64)))
        .collect()
}
