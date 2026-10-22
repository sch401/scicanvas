//! Existing statistical transforms and valid data runs, shared by plotting and analysis.
use crate::{model::PlotType, plotspec::Transform, Series};

pub(crate) fn apply(series: &mut [Series], transform: Transform) {
    if transform == Transform::Mean {
        for s in series {
            if s.line_only {
                continue;
            }
            let mut indices: Vec<_> = (0..s.points.len()).collect();
            indices.sort_by(|a, b| s.points[*a][0].total_cmp(&s.points[*b][0]));
            let mut grouped = Vec::new();
            let mut bases = Vec::new();
            let mut i = 0;
            while i < s.points.len() {
                let x = s.points[indices[i]][0];
                let mut end = i + 1;
                while end < s.points.len() && s.points[indices[end]][0] == x {
                    end += 1;
                }
                let mean = indices[i..end].iter().map(|j| s.points[*j][1]).sum::<f64>() / (end - i) as f64;
                grouped.push([x, mean]);
                bases.push(indices[i..end].iter().map(|j| s.bases[*j]).sum::<f64>() / (end - i) as f64);
                i = end;
            }
            s.points = grouped;
            s.bases = bases;
            s.rows = vec![0; s.points.len()];
            // 原始行的测量误差不代表分组均值的标准误。
            s.errors = vec![[None; 2]; s.points.len()];
        }
    }
}

pub(crate) fn row_runs(s: &crate::Series) -> Vec<std::ops::Range<usize>> {
    let mut out = Vec::new();
    let mut start = 0;
    while start < s.points.len() {
        let mut end = start + 1;
        while end < s.points.len() && s.rows[end] == s.rows[end - 1] + 1 {
            end += 1;
        }
        out.push(start..end);
        start = end;
    }
    out
}

pub(crate) fn data_runs(s: &crate::Series) -> Vec<Vec<[f64; 2]>> {
    row_runs(s)
        .into_iter()
        .map(|range| {
            let mut run = s.points[range].to_vec();
            run.sort_by(|a, b| a[0].total_cmp(&b[0]));
            run
        })
        .collect()
}

pub(crate) fn spline_paths(s: &crate::Series) -> Vec<Vec<[f64; 2]>> {
    let mut out = Vec::new();
    for run in data_runs(s) {
        let mut start = 0;
        while start < run.len() {
            let mut end = start + 1;
            while end < run.len() && run[end][0] > run[end - 1][0] {
                end += 1;
            }
            // ponytail: 每区间固定 16 段采样；大数据绘图后续改为按页面误差自适应采样。
            out.push(crate::spline::natural(&run[start..end], 16).unwrap_or_else(|_| run[start..end].to_vec()));
            start = end;
        }
    }
    out
}

pub(crate) fn prepare(g: &crate::page::Layer, series: &mut [Series]) {
    for s in series {
        // 均值变换后的点没有原始行，按顺序编号，连线不在组之间断开
        if (g.plot_type == PlotType::Area || g.plot_type.is_spline() || g.plot_type.has_line())
            && g.transform == crate::plotspec::Transform::Mean
            && !s.line_only
        {
            s.rows = (0..s.points.len()).collect();
        }
        let keep: Vec<_> = s
            .points
            .iter()
            .zip(&s.bases)
            .map(|(p, base)| {
                p[1].is_finite()
                    && base.is_finite()
                    && (!g.x_log || p[0] > 0.0)
                    && (!g.y_log || p[1] > 0.0 || (g.plot_type.is_bar() && !s.line_only && *base > 0.0))
            })
            .collect();
        let mut flags = keep.iter();
        s.points.retain(|_| *flags.next().unwrap());
        let mut flags = keep.iter();
        s.errors.retain(|_| *flags.next().unwrap());
        let mut flags = keep.iter();
        s.bases.retain(|_| *flags.next().unwrap());
        let mut flags = keep.iter();
        s.rows.retain(|_| *flags.next().unwrap());
    }
}
