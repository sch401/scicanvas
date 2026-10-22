use super::Context;
use crate::model::{Series, SeriesStyle};
use crate::scene::{Anchor, Prim};

pub(super) fn errors(
    ctx: &Context<'_, '_, '_, '_>,
    series: &[Series],
    styles: &[SeriesStyle],
    items: &mut Vec<crate::scene::Item>,
) {
    for (series, style) in series.iter().zip(styles) {
        let axis = ctx.y_axis(style.right);
        let mut segs = Vec::new();
        let offset = ctx.offset(style);
        let cap = f64::from(ctx.layer.error_bars.cap) / 2.0;
        for (point, errors) in series.points.iter().zip(&series.errors) {
            let (x, y) = if ctx.horizontal {
                (point[0], point[1] + offset)
            } else {
                (point[0] + offset, point[1])
            };
            if let Some(error) = errors[0] {
                let ends = [x - error, x + error];
                let yy = ctx.py(y, style.right);
                if (!axis.log || y > 0.0) && ends[1] >= ctx.xa.lo && ends[0] <= ctx.xa.hi {
                    segs.push([
                        ctx.px(ends[0].clamp(ctx.xa.lo, ctx.xa.hi)),
                        yy,
                        ctx.px(ends[1].clamp(ctx.xa.lo, ctx.xa.hi)),
                        yy,
                    ]);
                    for end in ends {
                        if (ctx.xa.lo..=ctx.xa.hi).contains(&end) {
                            let xx = ctx.px(end);
                            segs.push([xx, yy - cap, xx, yy + cap]);
                        }
                    }
                }
            }
            if let Some(error) = errors[1] {
                let ends = [y - error, y + error];
                let xx = ctx.px(x);
                if (ctx.xa.lo..=ctx.xa.hi).contains(&x) && ends[1] >= axis.lo && ends[0] <= axis.hi {
                    segs.push([
                        xx,
                        ctx.py(ends[0].clamp(axis.lo, axis.hi), style.right),
                        xx,
                        ctx.py(ends[1].clamp(axis.lo, axis.hi), style.right),
                    ]);
                    for end in ends {
                        if (axis.lo..=axis.hi).contains(&end) {
                            let yy = ctx.py(end, style.right);
                            segs.push([xx - cap, yy, xx + cap, yy]);
                        }
                    }
                }
            }
        }
        if !segs.is_empty() {
            ctx.push(
                items,
                Prim::Segments {
                    segs,
                    color: style.color.rgb(),
                    width: f64::from(ctx.layer.error_bars.width),
                    dashed: false,
                },
                true,
            );
        }
    }
}

pub(super) fn labels(
    ctx: &Context<'_, '_, '_, '_>,
    series: &[Series],
    styles: &[SeriesStyle],
    items: &mut Vec<crate::scene::Item>,
) {
    use crate::page::LabelSource;
    for (series, style) in series.iter().zip(styles).filter(|(series, _)| !series.line_only) {
        let offset = ctx.offset(style);
        for (i, point) in series.points.iter().enumerate() {
            let (x, y) = if ctx.horizontal {
                (point[0], point[1] + offset)
            } else {
                (point[0] + offset, point[1])
            };
            if !ctx.inside_x(ctx.px(x)) || !ctx.inside_y(ctx.py(y, style.right)) {
                continue;
            }
            let value = if ctx.horizontal {
                point[0]
            } else if style.kind.is_stacked() {
                point[1] - series.bases[i]
            } else {
                point[1]
            };
            let value_text = format!(
                "{}{}",
                crate::fmt_num(value),
                if crate::presets::recipe(style.kind).position == crate::position::Position::PercentStack {
                    "%"
                } else {
                    ""
                }
            );
            let label = match ctx.layer.labels.source {
                LabelSource::Value => value_text,
                LabelSource::XY => format!(
                    "({}, {})",
                    crate::fmt_num(if ctx.horizontal { point[1] } else { point[0] }),
                    value_text
                ),
                LabelSource::Row if ctx.layer.transform == crate::plotspec::Transform::Identity => {
                    (series.rows[i] + 1).to_string()
                }
                LabelSource::Column if ctx.layer.transform == crate::plotspec::Transform::Identity => {
                    let own = ctx
                        .book
                        .and_then(|book| ctx.layer.resolve(book, ctx.others, &series.key))
                        .map(|(book, _)| book);
                    let local = own.is_none_or(|book| Some(book.id) == ctx.layer.book_id);
                    ctx.layer
                        .labels
                        .column
                        .as_deref()
                        .filter(|_| local)
                        .and_then(|key| ctx.book.and_then(|book| book.column(key)))
                        .or_else(|| {
                            own.or(ctx.book)
                                .and_then(|book| book.columns.iter().find(|column| column.role == crate::Role::Label))
                        })
                        .and_then(|column| column.data.get(series.rows[i]))
                        .map(std::borrow::Cow::into_owned)
                        .unwrap_or_default()
                }
                _ => String::new(),
            }
            .replace(['\r', '\n'], " ");
            if label.trim().is_empty() {
                continue;
            }
            let positive = if crate::presets::recipe(style.kind).position == crate::position::Position::Interval {
                value >= series.bases[i]
            } else {
                value >= 0.0
            };
            let reach = if ctx.layer.error_bars.visible {
                series.errors[i][usize::from(!ctx.horizontal)].map_or(0.0, |error| {
                    let delta = if ctx.horizontal {
                        ctx.px(if positive { x + error } else { x - error }) - ctx.px(x)
                    } else {
                        ctx.py(if positive { y + error } else { y - error }, style.right) - ctx.py(y, style.right)
                    };
                    if delta.is_finite() {
                        delta.abs()
                    } else {
                        0.0
                    }
                })
            } else {
                0.0
            };
            let delta = f64::from(ctx.layer.labels.offset) + f64::from(ctx.layer.labels.size) / 2.0 + reach;
            let (xx, yy, anchor) = if ctx.horizontal {
                (
                    ctx.px(x) + if positive { delta } else { -delta },
                    ctx.py(y, style.right),
                    if positive { Anchor::Start } else { Anchor::End },
                )
            } else {
                (
                    ctx.px(x),
                    ctx.py(y, style.right) + if positive { -delta } else { delta },
                    Anchor::Middle,
                )
            };
            let width = crate::scene::text_width(&label, f64::from(ctx.layer.labels.size)).min(ctx.rect[2]);
            let xx = match anchor {
                Anchor::Middle => xx.clamp(
                    ctx.rect[0] + width / 2.0,
                    (ctx.rect[0] + ctx.rect[2] - width / 2.0).max(ctx.rect[0] + width / 2.0),
                ),
                Anchor::Start => xx.clamp(ctx.rect[0], ctx.rect[0] + ctx.rect[2] - width),
                Anchor::End => xx.clamp(ctx.rect[0] + width, ctx.rect[0] + ctx.rect[2]),
            };
            let half = (f64::from(ctx.layer.labels.size) / 2.0).min(ctx.rect[3] / 2.0);
            let yy = yy.clamp(ctx.rect[1] + half, ctx.rect[1] + ctx.rect[3] - half);
            ctx.push(
                items,
                Prim::Text {
                    x: xx,
                    y: yy,
                    text: label,
                    size: f64::from(ctx.layer.labels.size),
                    anchor,
                    vertical: false,
                    bold: false,
                    color: style.color.rgb(),
                },
                true,
            );
        }
    }
}
