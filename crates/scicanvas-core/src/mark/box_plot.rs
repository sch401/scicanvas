//! 箱体、中位数、须线、离群点和空心均值符号，共用场景图元。
use super::{Context, Kind};
use crate::{
    page::PreparedPlot,
    scene::{Item, Prim, StyledSymbol},
    Symbol,
};

pub(super) fn draw(ctx: &Context<'_, '_, '_, '_>, plot: &PreparedPlot, items: &mut Vec<Item>) -> bool {
    let style = &plot.style;
    if super::contains(style.kind, Kind::Violin) {
        violin(ctx, plot, items);
        return true;
    }
    if !super::contains(style.kind, Kind::Box) {
        return false;
    }
    let (Some(stats), Some(point)) = (&plot.box_stats, plot.series.points.first()) else {
        return true;
    };
    let x = point[0];
    let half = ctx.dx * 0.3;
    let (left, right) = (ctx.px(x - half), ctx.px(x + half));
    let py = |v: f64| {
        let axis = ctx.y_axis(style.y_axis);
        ctx.py(if axis.log { v.max(axis.lo) } else { v }, style.y_axis)
    };
    let color = style.color.rgb();
    let width = f64::from(style.line_width);
    ctx.push(
        items,
        Prim::Rect {
            x: left,
            y: py(stats.q3),
            w: right - left,
            h: (py(stats.q1) - py(stats.q3)).abs(),
            fill: Some((color, 0.35)),
            stroke: Some((color, width)),
        },
        true,
    );
    let center = ctx.px(x);
    ctx.push(
        items,
        Prim::Segments {
            segs: vec![
                [left, py(stats.median), right, py(stats.median)],
                [center, py(stats.whisker_lo), center, py(stats.q1)],
                [center, py(stats.q3), center, py(stats.whisker_hi)],
                [
                    ctx.px(x - half / 2.0),
                    py(stats.whisker_lo),
                    ctx.px(x + half / 2.0),
                    py(stats.whisker_lo),
                ],
                [
                    ctx.px(x - half / 2.0),
                    py(stats.whisker_hi),
                    ctx.px(x + half / 2.0),
                    py(stats.whisker_hi),
                ],
            ],
            color,
            width,
            dashed: false,
        },
        true,
    );
    let positions: Vec<_> = stats
        .outliers
        .iter()
        .map(|v| [center, ctx.py(*v, style.y_axis)])
        .filter(|p| ctx.inside_x(p[0]) && ctx.inside_y(p[1]))
        .collect();
    if !positions.is_empty() {
        ctx.push(
            items,
            Prim::Symbols {
                sym: style.symbol,
                pts: positions,
                r: f64::from(style.symbol_size) / 2.0,
                color,
            },
            false,
        );
    }
    let at = [center, ctx.py(stats.mean, style.y_axis)];
    if ctx.inside_x(at[0]) && ctx.inside_y(at[1]) {
        ctx.push(
            items,
            Prim::StyledSymbols {
                symbols: vec![StyledSymbol {
                    at,
                    symbol: Symbol::Diamond,
                    radius: f64::from(style.symbol_size) / 2.0,
                    fill: None,
                    stroke: Some((color, width)),
                }],
            },
            false,
        );
    }
    true
}

fn violin(ctx: &Context<'_, '_, '_, '_>, plot: &PreparedPlot, items: &mut Vec<Item>) {
    let (Some(stats), Some(point)) = (&plot.box_stats, plot.series.points.first()) else {
        return;
    };
    let values: Vec<_> = plot.series.points.iter().map(|p| p[1]).collect();
    let lo = values.iter().copied().fold(f64::INFINITY, f64::min);
    let hi = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let at: Vec<_> = (0..100).map(|i| lo + (hi - lo) * i as f64 / 99.).collect();
    let density = crate::stats::gaussian_kde(&values, &at);
    let peak = density.iter().copied().fold(0., f64::max);
    let x = point[0];
    let axis = plot.style.y_axis;
    let color = plot.style.color.rgb();
    if peak > 0. && peak.is_finite() {
        let side = |sign: f64| {
            at.iter()
                .zip(&density)
                .map(|(v, d)| [ctx.px(x + sign * ctx.dx * 0.25 * d / peak), ctx.py(*v, axis)])
                .collect::<Vec<_>>()
        };
        let left = side(-1.);
        let right = side(1.);
        let mut triangles = Vec::new();
        for i in 1..at.len() {
            triangles.extend([left[i - 1], right[i - 1], left[i], left[i], right[i - 1], right[i]]);
        }
        ctx.push(
            items,
            Prim::Triangles {
                pts: triangles,
                color,
                opacity: 0.3,
            },
            true,
        );
        let mut outline = left;
        outline.extend(right.into_iter().rev());
        outline.push(outline[0]);
        ctx.push(
            items,
            Prim::Polyline {
                pts: outline,
                color,
                width: f64::from(plot.style.line_width),
            },
            true,
        );
    }
    let center = ctx.px(x);
    ctx.push(
        items,
        Prim::Segments {
            segs: vec![[center, ctx.py(stats.q1, axis), center, ctx.py(stats.q3, axis)]],
            color,
            width: 3.,
            dashed: false,
        },
        true,
    );
    ctx.push(
        items,
        Prim::Symbols {
            sym: Symbol::Circle,
            pts: vec![[center, ctx.py(stats.median, axis)]],
            r: 2.,
            color: (255, 255, 255),
        },
        true,
    );
}
