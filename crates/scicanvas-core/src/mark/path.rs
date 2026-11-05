use super::{Context, Kind, Path};
use crate::model::{Series, SeriesStyle};
use crate::page::{CurveStyle, SymbolFill};
use crate::scene::{ColoredSegment, Prim, StyledSymbol};

pub(super) fn draw(
    ctx: &Context<'_, '_, '_, '_>,
    series: &Series,
    style: &SeriesStyle,
    curve_style: Option<&CurveStyle>,
    splines: &[Vec<[f64; 2]>],
    items: &mut Vec<crate::scene::Item>,
) -> Option<Prim> {
    let recipe = crate::presets::recipe(style.kind);
    let color = style.color.rgb();
    let mapped_color = series.point_styles.iter().any(|point| point.color.is_some());
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
        if mapped_color {
            ctx.push(
                items,
                Prim::ColoredSegments {
                    segments: series
                        .points
                        .iter()
                        .enumerate()
                        .map(|(index, point)| ColoredSegment {
                            points: [ctx.px(point[0]), base, ctx.px(point[0]), ctx.py(point[1], style.y_axis)],
                            color: point_color(series, index, color),
                        })
                        .collect(),
                    width: f64::from(style.line_width),
                },
                true,
            );
        } else {
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
    }
    if recipe.path == Path::Natural || matches!(style.kind, crate::PlotType::QQ | crate::PlotType::ColumnScatter) {
        if mapped_color {
            let segments = colored_spline_segments(ctx, series, style, splines, color);
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
        } else {
            draw_paths(ctx, style, splines, items);
        }
    } else if recipe.marks.contains(&Kind::Line) && style.kind != crate::PlotType::LineSeries {
        if mapped_color {
            let segments = colored_line_segments(ctx, series, style, recipe.path, color);
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
        } else {
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
    }
    draw_symbols(ctx, series, style, curve_style, color)
}

/// 样条、Q-Q 参考线和直方图分布曲线复用的路径绘制。
pub(super) fn draw_paths(
    ctx: &Context<'_, '_, '_, '_>,
    style: &SeriesStyle,
    paths: &[Vec<[f64; 2]>],
    items: &mut Vec<crate::scene::Item>,
) {
    for path in paths {
        for run in path.split(|p| (ctx.layer.x_log && p[0] <= 0.0) || (ctx.layer.y_log && p[1] <= 0.0)) {
            if run.len() > 1 {
                ctx.push(
                    items,
                    Prim::Polyline {
                        pts: run.iter().map(|p| [ctx.px(p[0]), ctx.py(p[1], style.y_axis)]).collect(),
                        color: style.color.rgb(),
                        width: f64::from(style.line_width),
                    },
                    true,
                );
            }
        }
    }
}

fn draw_symbols(
    ctx: &Context<'_, '_, '_, '_>,
    series: &Series,
    style: &SeriesStyle,
    curve_style: Option<&CurveStyle>,
    color: crate::scene::Rgb,
) -> Option<Prim> {
    if !crate::presets::recipe(style.kind).marks.contains(&Kind::Symbol) {
        return None;
    }
    let styled = curve_style.is_some_and(|style| {
        style.symbol_fill.is_some() || style.symbol_border_color.is_some() || style.symbol_border_width.is_some()
    }) || series
        .point_styles
        .iter()
        .any(|point| point.color.is_some() || point.size.is_some() || point.symbol.is_some());
    if styled {
        let visible: Vec<_> = series
            .points
            .iter()
            .enumerate()
            .map(|(index, point)| (index, [ctx.px(point[0]), ctx.py(point[1], style.y_axis)]))
            .filter(|(_, point)| ctx.inside_x(point[0]) && ctx.inside_y(point[1]))
            .collect();
        let positions: Vec<_> = visible.iter().map(|(_, point)| *point).collect();
        let indices = decimate_symbol_indices(&positions, ctx.rect[2]);
        let fill_style = curve_style
            .and_then(|style| style.symbol_fill)
            .unwrap_or(SymbolFill::MatchCurve);
        let border_color = curve_style.and_then(|style| style.symbol_border_color);
        let border_width = curve_style.and_then(|style| style.symbol_border_width);
        Some(Prim::StyledSymbols {
            symbols: indices
                .into_iter()
                .map(|position_index| {
                    let (index, at) = visible[position_index];
                    let point = series.point_styles.get(index).copied().unwrap_or_default();
                    let fill = match fill_style {
                        SymbolFill::MatchCurve => Some(point.color.map_or(color, |[r, g, b]| (r, g, b))),
                        SymbolFill::Hollow => None,
                        SymbolFill::Color([r, g, b]) => Some((r, g, b)),
                    };
                    let stroke = if border_color.is_some() || border_width.is_some() {
                        Some((
                            border_color.map_or((0, 0, 0), |[r, g, b]| (r, g, b)),
                            f64::from(border_width.unwrap_or(0.6)),
                        ))
                    } else {
                        None
                    };
                    StyledSymbol {
                        at,
                        symbol: point.symbol.unwrap_or(style.symbol),
                        radius: f64::from(point.size.unwrap_or(style.symbol_size)) / 2.0,
                        fill,
                        stroke,
                    }
                })
                .collect(),
        })
    } else {
        let points = decimate_symbols(
            series
                .points
                .iter()
                .map(|point| [ctx.px(point[0]), ctx.py(point[1], style.y_axis)])
                .filter(|point| ctx.inside_x(point[0]) && ctx.inside_y(point[1]))
                .collect(),
            ctx.rect[2],
        );
        Some(Prim::Symbols {
            sym: style.symbol,
            pts: points,
            r: f64::from(style.symbol_size) / 2.0,
            color,
        })
    }
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

fn point_color(series: &Series, index: usize, default: crate::scene::Rgb) -> crate::scene::Rgb {
    series
        .point_styles
        .get(index)
        .and_then(|style| style.color)
        .map_or(default, |[r, g, b]| (r, g, b))
}

fn nearest_source(series: &Series, indices: &[usize], x: f64) -> usize {
    let at = indices.partition_point(|index| series.points[*index][0] < x);
    match (indices.get(at.saturating_sub(1)), indices.get(at)) {
        (Some(left), Some(right)) => {
            if (series.points[*left][0] - x).abs() <= (series.points[*right][0] - x).abs() {
                *left
            } else {
                *right
            }
        }
        (Some(index), None) | (None, Some(index)) => *index,
        (None, None) => 0,
    }
}

fn decimate_line_indices(points: &[[f64; 2]], width: f64) -> Vec<usize> {
    if points.len() as f64 <= 4.0 * width.max(1.0) {
        return (0..points.len()).collect();
    }
    let mut out = Vec::with_capacity((4.0 * width) as usize + 8);
    let mut start = 0;
    while start < points.len() {
        let column = points[start][0].floor();
        let mut end = start + 1;
        while end < points.len() && points[end][0].floor() == column {
            end += 1;
        }
        let group = &points[start..end];
        let (mut lo, mut hi) = (0, 0);
        for (index, point) in group.iter().enumerate() {
            if point[1] < group[lo][1] {
                lo = index;
            }
            if point[1] > group[hi][1] {
                hi = index;
            }
        }
        let mut keep = vec![start, start + lo, start + hi, end - 1];
        keep.sort_unstable();
        keep.dedup();
        out.extend(keep);
        start = end;
    }
    out
}

fn decimate_symbol_indices(points: &[[f64; 2]], width: f64) -> Vec<usize> {
    if points.len() as f64 <= 4.0 * width.max(1.0) {
        return (0..points.len()).collect();
    }
    let mut seen = std::collections::HashSet::with_capacity(points.len().min(1 << 20));
    points
        .iter()
        .enumerate()
        .filter_map(|(index, point)| {
            seen.insert(((point[0] * 2.0).round() as i64, (point[1] * 2.0).round() as i64))
                .then_some(index)
        })
        .collect()
}

fn colored_spline_segments(
    ctx: &Context<'_, '_, '_, '_>,
    series: &Series,
    style: &SeriesStyle,
    splines: &[Vec<[f64; 2]>],
    default: crate::scene::Rgb,
) -> Vec<ColoredSegment> {
    let mut source: Vec<_> = (0..series.points.len()).collect();
    source.sort_by(|a, b| series.points[*a][0].total_cmp(&series.points[*b][0]));
    let mut segments = Vec::new();
    for path in splines {
        for run in path.split(|point| (ctx.layer.x_log && point[0] <= 0.0) || (ctx.layer.y_log && point[1] <= 0.0)) {
            let pixels: Vec<_> = run
                .iter()
                .map(|point| [ctx.px(point[0]), ctx.py(point[1], style.y_axis)])
                .collect();
            let kept = decimate_line_indices(&pixels, ctx.rect[2]);
            for pair in kept.windows(2) {
                let a = run[pair[0]];
                let b = run[pair[1]];
                let index = nearest_source(series, &source, (a[0] + b[0]) / 2.0);
                segments.push(ColoredSegment {
                    points: [
                        pixels[pair[0]][0],
                        pixels[pair[0]][1],
                        pixels[pair[1]][0],
                        pixels[pair[1]][1],
                    ],
                    color: point_color(series, index, default),
                });
            }
        }
    }
    segments
}

fn colored_line_segments(
    ctx: &Context<'_, '_, '_, '_>,
    series: &Series,
    style: &SeriesStyle,
    path: Path,
    default: crate::scene::Rgb,
) -> Vec<ColoredSegment> {
    let mut segments = Vec::new();
    for range in crate::stat::row_runs(series)
        .into_iter()
        .filter(|range| range.len() > 1)
    {
        let run = &series.points[range.clone()];
        if matches!(path, Path::StepHorizontal | Path::StepVertical) {
            for (offset, pair) in run.windows(2).enumerate() {
                let bend = if path == Path::StepHorizontal {
                    [pair[1][0], pair[0][1]]
                } else {
                    [pair[0][0], pair[1][1]]
                };
                let a = [ctx.px(pair[0][0]), ctx.py(pair[0][1], style.y_axis)];
                let b = [ctx.px(bend[0]), ctx.py(bend[1], style.y_axis)];
                let c = [ctx.px(pair[1][0]), ctx.py(pair[1][1], style.y_axis)];
                segments.push(ColoredSegment {
                    points: [a[0], a[1], b[0], b[1]],
                    color: point_color(series, range.start + offset, default),
                });
                segments.push(ColoredSegment {
                    points: [b[0], b[1], c[0], c[1]],
                    color: point_color(series, range.start + offset + 1, default),
                });
            }
            continue;
        }

        let curve = match ctx.layer.connect {
            crate::page::Connect::BSpline if !series.line_only => crate::spline::bspline(run, 16),
            crate::page::Connect::Bezier if !series.line_only => crate::spline::bezier(run, 16),
            _ => run.to_vec(),
        };
        let pixels: Vec<_> = curve
            .iter()
            .map(|point| [ctx.px(point[0]), ctx.py(point[1], style.y_axis)])
            .collect();
        let kept = decimate_line_indices(&pixels, ctx.rect[2]);
        let mut source: Vec<_> = range.collect();
        source.sort_by(|a, b| series.points[*a][0].total_cmp(&series.points[*b][0]));
        for pair in kept.windows(2) {
            let a = curve[pair[0]];
            let b = curve[pair[1]];
            let index = nearest_source(series, &source, (a[0] + b[0]) / 2.0);
            segments.push(ColoredSegment {
                points: [
                    pixels[pair[0]][0],
                    pixels[pair[0]][1],
                    pixels[pair[1]][0],
                    pixels[pair[1]][1],
                ],
                color: point_color(series, index, default),
            });
        }
    }
    segments
}
