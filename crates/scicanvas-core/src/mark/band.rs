use super::Context;
use crate::{
    coord::YAxis,
    scene::{Item, Prim},
};

/// 同一组 X 上的两条边界，先投影再剖分；缺失值断开，交点两侧分别填充。
pub(crate) fn fill_between(
    ctx: &Context<'_, '_, '_, '_>,
    upper: &[[f64; 2]],
    lower: &[[f64; 2]],
    axis: YAxis,
    color: (u8, u8, u8),
    opacity: f64,
) -> Item {
    let points: Vec<_> = upper
        .iter()
        .zip(lower)
        .map(|(u, l)| ([ctx.px(u[0]), ctx.py(u[1], axis)], [ctx.px(l[0]), ctx.py(l[1], axis)]))
        .collect();
    let mut pts = Vec::new();
    for pair in points.windows(2) {
        let ((a, c), (b, d)) = (pair[0], pair[1]);
        if ![a, b, c, d].iter().flatten().all(|v| v.is_finite()) {
            continue;
        }
        let (da, db) = (a[1] - c[1], b[1] - d[1]);
        if da * db < 0.0 {
            let t = da / (da - db);
            let cross = [a[0] + t * (b[0] - a[0]), a[1] + t * (b[1] - a[1])];
            pts.extend([a, c, cross, b, cross, d]);
        } else {
            pts.extend([a, b, d, a, d, c]);
        }
    }
    Item {
        prim: Prim::Triangles { pts, color, opacity },
        clip: Some(ctx.rect),
    }
}

pub(super) fn draw(
    ctx: &Context<'_, '_, '_, '_>,
    series: &crate::Series,
    style: &crate::SeriesStyle,
    items: &mut Vec<Item>,
) {
    for range in crate::stat::row_runs(series) {
        let mut indices: Vec<_> = range.collect();
        indices.sort_by(|&a, &b| series.points[a][0].total_cmp(&series.points[b][0]));
        let axis = ctx.y_axis(style.y_axis);
        let upper: Vec<_> = indices.iter().map(|&i| series.points[i]).collect();
        let lower: Vec<_> = indices
            .iter()
            .map(|&i| {
                [
                    series.points[i][0],
                    if axis.log {
                        series.bases[i].max(axis.lo)
                    } else {
                        series.bases[i]
                    },
                ]
            })
            .collect();
        items.push(fill_between(
            ctx,
            &upper,
            &lower,
            style.y_axis,
            style.color.rgb(),
            f64::from(ctx.layer.area_opacity),
        ));
        if upper.len() > 1 {
            ctx.push(
                items,
                Prim::Polyline {
                    pts: upper
                        .iter()
                        .map(|p| [ctx.px(p[0]), ctx.py(p[1], style.y_axis)])
                        .collect(),
                    color: style.color.rgb(),
                    width: f64::from(style.line_width),
                },
                true,
            );
        }
    }
}
