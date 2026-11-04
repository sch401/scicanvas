//! 箱体、中位数、须线、离群点和空心均值符号，共用场景图元。
use super::{Context, Kind};
use crate::{
    page::PreparedPlot,
    scene::{Item, Prim, StyledSymbol},
    Symbol,
};

pub(super) fn draw(ctx: &Context<'_, '_, '_, '_>, plot: &PreparedPlot, items: &mut Vec<Item>) -> bool {
    let style = &plot.style;
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
