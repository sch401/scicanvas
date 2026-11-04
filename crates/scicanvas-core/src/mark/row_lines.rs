use super::Context;
use crate::{
    page::PreparedPlot,
    scene::{Item, Prim},
};

type RowPoint = (usize, [f64; 2], crate::scene::Rgb);

pub(super) fn draw(ctx: &Context<'_, '_, '_, '_>, plots: &[PreparedPlot], items: &mut Vec<Item>) {
    // ponytail: 有序行表 O(n log n)，极大矩阵可改为多列游标的线性归并。
    let mut rows: std::collections::BTreeMap<usize, Vec<RowPoint>> = std::collections::BTreeMap::new();
    for (column, p) in plots.iter().filter(|p| !p.series.line_only).enumerate() {
        for (index, (&row, point)) in p.series.rows.iter().zip(&p.series.points).enumerate() {
            rows.entry(row).or_default().push((
                column,
                [ctx.px(point[0]), ctx.py(point[1], p.style.y_axis)],
                p.series
                    .point_styles
                    .get(index)
                    .and_then(|s| s.color)
                    .map_or(p.style.color.rgb(), |[r, g, b]| (r, g, b)),
            ));
        }
    }
    let Some(first) = plots.iter().find(|p| !p.series.line_only) else {
        return;
    };
    for points in rows.values() {
        let mut start = 0;
        while start < points.len() {
            let mut end = start + 1;
            while end < points.len() && points[end].0 == points[end - 1].0 + 1 {
                end += 1;
            }
            if end - start > 1 {
                ctx.push(
                    items,
                    Prim::Polyline {
                        pts: points[start..end].iter().map(|p| p.1).collect(),
                        color: points[start].2,
                        width: f64::from(first.style.line_width),
                    },
                    true,
                );
            }
            start = end;
        }
    }
}
