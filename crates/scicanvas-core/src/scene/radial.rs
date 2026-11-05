//! Pie and polar scenes share the standard page compositor and export primitives.
use super::*;
use crate::{coord::Polar, page::Projection};
use std::f64::consts::{FRAC_PI_2, TAU};

struct Slice {
    row: usize,
    label: String,
    value: f64,
    key: String,
}
fn slices(layer: &Layer, books: BookLookup<'_, '_>) -> (Vec<Slice>, usize) {
    let Some(plot) = layer.plots(books).into_iter().find(|p| !p.mapping.line_only) else {
        return (vec![], 0);
    };
    let Some(book) = books(plot.mapping.y.book) else {
        return (vec![], 0);
    };
    let Some(col) = book.column(&plot.mapping.y.col) else {
        return (vec![], 0);
    };
    let labels = plot.mapping.x.as_ref().and_then(|x| book.column(&x.col));
    let values = col.values();
    let mut result = Vec::new();
    let mut skipped = 0;
    for (row, value) in values.into_iter().enumerate() {
        if plot.mapping.rows.is_some_and(|(a, b)| row < a || row > b) || (layer.visible_only && !book.row_visible(row))
        {
            continue;
        }
        if !value.is_finite() || value <= 0. {
            skipped += 1;
            continue;
        }
        result.push(Slice {
            row,
            value,
            key: plot.key.clone(),
            label: labels
                .map(|c| c.data.text_at(row).into_owned())
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| (row + 1).to_string()),
        });
    }
    (result, skipped)
}
pub(super) fn skipped(layer: &Layer, books: BookLookup<'_, '_>) -> usize {
    slices(layer, books).1
}
fn circle(rect: [f64; 4]) -> ([f64; 2], f64) {
    (
        [rect[0] + rect[2] / 2., rect[1] + rect[3] / 2.],
        rect[2].min(rect[3]) * 0.4,
    )
}
fn polar<'a>(axis: &'a Axis, rect: [f64; 4], degrees: bool) -> Polar<'a> {
    let (center, radius) = circle(rect);
    Polar {
        r: axis,
        center,
        radius,
        degrees,
    }
}
pub(super) fn build(ctx: &LayerContext<'_, '_, '_>) -> Scene {
    let layer = ctx.layer;
    let books = |id| ctx.book.filter(|b| b.id == id).or_else(|| (ctx.others)(id));
    let rect = [24., 24., (ctx.w - 48.).max(20.), (ctx.h - 48.).max(20.)];
    let (_, plots) = ctx.book.map(|b| layer.prepared_plots_in(b, &books)).unwrap_or_default();
    let values: Vec<_> = plots
        .iter()
        .flat_map(|p| p.series.points.iter().map(|p| p[1]))
        .filter(|v| *v >= 0.)
        .collect();
    let mut r = axis_scaled(
        &values,
        layer.y_auto,
        layer.y_min,
        layer.y_max,
        layer.effective_y_scale(),
        true,
    );
    if r.scale == AxisScale::Linear {
        r.lo = 0.;
    }
    let axes = crate::coord::AxisSet {
        x: axis(&[], false, 0., 360., false, false),
        left_y: r,
        right_y: None,
    };
    let mut items = vec![Item {
        prim: Prim::Rect {
            x: 0.,
            y: 0.,
            w: ctx.w,
            h: ctx.h,
            fill: (!layer.transparent).then_some((PAPER, 1.)),
            stroke: None,
        },
        clip: None,
    }];
    let mut objects = Vec::new();
    if layer.plot_type == PlotType::Pie {
        let (slices, _) = slices(layer, &books);
        draw_pie(&slices, rect, &mut items);
        if layer.legend {
            let entries: Vec<_> = slices
                .iter()
                .enumerate()
                .map(|(i, s)| (s.label.clone(), crate::model::SeriesColor::Palette(i).rgb()))
                .collect();
            legend::draw_row_legend(&entries, [ctx.w - 100., 32.], &mut items, &mut objects);
        }
    } else if let Projection::Polar { degrees } = layer.projection {
        draw_polar(&plots, &polar(&axes.left_y, rect, degrees), &mut items);
    }
    if !layer.title.is_empty() {
        items.push(Item {
            prim: text(ctx.w / 2., 10., layer.title.clone(), TITLE_FONT, Anchor::Middle),
            clip: None,
        });
    }
    Scene {
        w: ctx.w,
        h: ctx.h,
        plot: rect,
        axes: axes.clone(),
        items,
        layer_plots: vec![rect],
        layers: vec![LayerView {
            bounds: [0., 0., ctx.w, ctx.h],
            plot: rect,
            axes,
            opaque: !layer.transparent,
        }],
        objects,
    }
}
fn draw_pie(slices: &[Slice], rect: [f64; 4], items: &mut Vec<Item>) {
    let (center, radius) = circle(rect);
    let total = slices.iter().map(|s| s.value).sum::<f64>();
    let mut start = FRAC_PI_2;
    for (i, slice) in slices.iter().enumerate() {
        let angle = TAU * slice.value / total;
        let n = (angle.to_degrees() / 2.).ceil().max(1.) as usize;
        let arc: Vec<_> = (0..=n)
            .map(|j| {
                let a = start - angle * j as f64 / n as f64;
                [center[0] + radius * a.cos(), center[1] - radius * a.sin()]
            })
            .collect();
        let color = crate::model::SeriesColor::Palette(i).rgb();
        let pts = arc.windows(2).flat_map(|p| [center, p[0], p[1]]).collect();
        items.push(Item {
            prim: Prim::Triangles {
                pts,
                color,
                opacity: 1.,
            },
            clip: None,
        });
        let mut outline = vec![center];
        outline.extend(arc);
        outline.push(center);
        items.push(Item {
            prim: Prim::Polyline {
                pts: outline,
                color: PAPER,
                width: 1.,
            },
            clip: None,
        });
        let mid = start - angle / 2.;
        let near_vertical = mid.cos().abs() < 0.2;
        let anchor = if near_vertical {
            Anchor::Middle
        } else if mid.cos() >= 0. {
            Anchor::Start
        } else {
            Anchor::End
        };
        let y_offset = if near_vertical {
            -mid.sin().signum() * TICK_FONT / 2.
        } else {
            0.
        };
        items.push(Item {
            prim: text(
                center[0] + radius * 1.12 * mid.cos(),
                center[1] - radius * 1.12 * mid.sin() + y_offset,
                format!("{:.1}%", 100. * slice.value / total),
                TICK_FONT,
                anchor,
            ),
            clip: None,
        });
        start -= angle;
    }
}
fn draw_polar(plots: &[crate::page::PreparedPlot], map: &Polar<'_>, items: &mut Vec<Item>) {
    for &tick in map.r.ticks.iter().filter(|v| **v >= 0.) {
        let r = map.r.frac(tick) * map.radius;
        let pts = (0..=128)
            .map(|i| {
                let a = TAU * i as f64 / 128.;
                [map.center[0] + r * a.cos(), map.center[1] - r * a.sin()]
            })
            .collect();
        items.push(Item {
            prim: Prim::Polyline {
                pts,
                color: GRID,
                width: 0.6,
            },
            clip: None,
        });
        let a = 22.5f64.to_radians();
        items.push(Item {
            prim: text(
                map.center[0] + r * a.cos(),
                map.center[1] - r * a.sin(),
                crate::fmt_num(tick),
                TICK_FONT,
                Anchor::Start,
            ),
            clip: None,
        });
    }
    for angle in (0..360).step_by(45) {
        let a = (angle as f64).to_radians();
        let end = [
            map.center[0] + map.radius * a.cos(),
            map.center[1] - map.radius * a.sin(),
        ];
        items.push(Item {
            prim: Prim::Segments {
                segs: vec![[map.center[0], map.center[1], end[0], end[1]]],
                color: GRID,
                width: 0.6,
                dashed: false,
            },
            clip: None,
        });
        items.push(Item {
            prim: text(
                map.center[0] + map.radius * 1.1 * a.cos(),
                map.center[1] - map.radius * 1.1 * a.sin(),
                if map.degrees {
                    format!("{angle}°")
                } else {
                    format!("{:.2}", a)
                },
                TICK_FONT,
                Anchor::Middle,
            ),
            clip: None,
        });
    }
    for plot in plots {
        let color = plot.style.color.rgb();
        let points = &plot.series.points;
        if plot.style.kind.has_line() {
            for run in crate::stat::row_runs(&plot.series) {
                let mut pts = Vec::new();
                for pair in points[run].windows(2) {
                    if pair.iter().any(|p| p[1] < 0.) {
                        if pts.len() > 1 {
                            items.push(Item {
                                prim: Prim::Polyline {
                                    pts: std::mem::take(&mut pts),
                                    color,
                                    width: f64::from(plot.style.line_width),
                                },
                                clip: None,
                            });
                        }
                        continue;
                    }
                    pts.extend((0..16).map(|i| {
                        let t = i as f64 / 16.;
                        map.project([
                            pair[0][0] * (1. - t) + pair[1][0] * t,
                            pair[0][1] * (1. - t) + pair[1][1] * t,
                        ])
                    }));
                    pts.push(map.project(pair[1]));
                }
                if pts.len() > 1 {
                    items.push(Item {
                        prim: Prim::Polyline {
                            pts,
                            color,
                            width: f64::from(plot.style.line_width),
                        },
                        clip: None,
                    });
                }
            }
        }
        if plot.style.kind.has_symbol() {
            items.push(Item {
                prim: Prim::Symbols {
                    sym: plot.style.symbol,
                    pts: points.iter().filter(|p| p[1] >= 0.).map(|p| map.project(*p)).collect(),
                    r: f64::from(plot.style.symbol_size) / 2.,
                    color,
                },
                clip: None,
            });
        }
    }
}
pub(super) fn pick_pie(layer: &Layer, view: &LayerView, books: BookLookup<'_, '_>, x: f64, y: f64) -> Option<PointHit> {
    let (center, r) = circle(view.plot);
    if (x - center[0]).hypot(y - center[1]) > r {
        return None;
    }
    let angle = (FRAC_PI_2 - (center[1] - y).atan2(x - center[0])).rem_euclid(TAU);
    let (slices, _) = slices(layer, books);
    let total = slices.iter().map(|s| s.value).sum::<f64>();
    let mut end = 0.;
    for s in slices {
        end += TAU * s.value / total;
        if angle <= end {
            return Some(PointHit {
                series: s.label,
                key: s.key,
                row: Some(s.row),
                x: s.row as f64 + 1.,
                y: s.value,
                at: [x, y],
            });
        }
    }
    None
}
pub(super) fn pick_polar(
    layer: &Layer,
    view: &LayerView,
    books: BookLookup<'_, '_>,
    at: [f64; 2],
    radius: f64,
    degrees: bool,
) -> Option<PointHit> {
    let (_, plots) = layer.prepared_plots_in(books(layer.book_id?)?, books);
    let map = polar(&view.axes.left_y, view.plot, degrees);
    plots
        .iter()
        .filter(|p| !p.series.line_only)
        .flat_map(|plot| {
            plot.series
                .points
                .iter()
                .enumerate()
                .filter(|(_, p)| p[1] >= 0.)
                .map(|(i, p)| {
                    let pixel = map.project(*p);
                    (
                        (pixel[0] - at[0]).hypot(pixel[1] - at[1]),
                        PointHit {
                            series: plot.series.name.clone(),
                            key: plot.plot.key.clone(),
                            row: Some(plot.series.rows[i]),
                            x: p[0],
                            y: p[1],
                            at: pixel,
                        },
                    )
                })
        })
        .filter(|(d, _)| *d <= radius)
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, hit)| hit)
}
