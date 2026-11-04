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
            let mut point_styles = Vec::new();
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
                let styles = indices[i..end]
                    .iter()
                    .filter_map(|j| s.point_styles.get(*j))
                    .collect::<Vec<_>>();
                let color = (0..3)
                    .map(|channel| {
                        let values = styles
                            .iter()
                            .filter_map(|style| style.color.map(|color| f64::from(color[channel])));
                        let (sum, count) = values.fold((0.0, 0), |(sum, count), value| (sum + value, count + 1));
                        (count > 0).then(|| (sum / count as f64).round() as u8)
                    })
                    .collect::<Option<Vec<_>>>()
                    .and_then(|channels| channels.try_into().ok());
                let size = styles
                    .iter()
                    .filter_map(|style| style.size)
                    .fold((0.0, 0), |(sum, count), value| (sum + f64::from(value), count + 1));
                point_styles.push(crate::PointStyle {
                    color,
                    size: (size.1 > 0).then(|| (size.0 / size.1 as f64) as f32),
                    symbol: styles.iter().find_map(|style| style.symbol),
                });
                i = end;
            }
            s.points = grouped;
            s.point_styles = point_styles;
            s.bases = bases;
            s.rows = vec![0; s.points.len()];
            // 原始行的测量误差不代表分组均值的标准误。
            s.errors = vec![[None; 2]; s.points.len()];
        }
    }
}

/// 直方图：把各数据曲线的 Y 值按共同的等宽区间计数，点改为（区间中点，计数），柱底为 0。
/// 区间范围取全部曲线有效值的最小、最大值，区间数按点数最多的曲线用 Sturges 规则；分箱与 numpy.histogram 相同。
pub(crate) fn histogram(series: &mut [Series]) {
    let pools: Vec<Vec<f64>> = series
        .iter()
        .map(|s| {
            if s.line_only {
                Vec::new()
            } else {
                s.points.iter().map(|p| p[1]).filter(|v| v.is_finite()).collect()
            }
        })
        .collect();
    let Some(longest) = pools.iter().map(Vec::len).max().filter(|n| *n > 0) else {
        return;
    };
    let lo = pools.iter().flatten().copied().fold(f64::INFINITY, f64::min);
    let hi = pools.iter().flatten().copied().fold(f64::NEG_INFINITY, f64::max);
    let bins = crate::stats::sturges_bins(longest);
    for (s, values) in series.iter_mut().zip(&pools) {
        if s.line_only {
            continue;
        }
        let Ok(counts) = crate::stats::histogram(values, bins, Some((lo, hi))) else {
            continue;
        };
        s.points = counts.iter().map(|b| [(b.lo + b.hi) / 2.0, b.count as f64]).collect();
        s.bases = vec![0.0; s.points.len()];
        s.rows = (0..s.points.len()).collect();
        s.errors = vec![[None; 2]; s.points.len()];
        s.point_styles = Vec::new();
    }
}

/// 正态 Q-Q 图：Filliben 排序位置，与 SciPy probplot(dist="norm") 一致。
pub(crate) fn qq(series: &mut [Series]) {
    for s in series.iter_mut().filter(|s| !s.line_only) {
        let mut values: Vec<f64> = s.points.iter().map(|p| p[1]).filter(|v| v.is_finite()).collect();
        values.sort_by(f64::total_cmp);
        let n = values.len();
        s.points = if n < 3 {
            Vec::new()
        } else {
            let end = 0.5_f64.powf(1.0 / n as f64);
            values
                .into_iter()
                .enumerate()
                .map(|(i, value)| {
                    let position = if i == 0 {
                        1.0 - end
                    } else if i == n - 1 {
                        end
                    } else {
                        (i as f64 + 1.0 - 0.3175) / (n as f64 + 0.365)
                    };
                    [crate::stats::norm_ppf(position), value]
                })
                .collect()
        };
        s.bases = vec![0.0; s.points.len()];
        s.rows = (0..s.points.len()).collect();
        s.errors = vec![[None; 2]; s.points.len()];
        s.point_styles.clear();
    }
}

/// 在理论分位数两端绘制最小二乘参考线，复用已有线性拟合。
pub(crate) fn qq_reference(s: &Series) -> Option<Vec<[f64; 2]>> {
    if s.line_only || s.points.len() < 3 {
        return None;
    }
    let xs: Vec<_> = s.points.iter().map(|p| p[0]).collect();
    let ys: Vec<_> = s.points.iter().map(|p| p[1]).collect();
    let fit = crate::analysis::polyfit(&xs, &ys, 1)?;
    let first = s.points.first()?[0];
    let last = s.points.last()?[0];
    Some(vec![[first, fit.eval(first)], [last, fit.eval(last)]])
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
        s.point_styles.retain(|_| *flags.next().unwrap());
        let mut flags = keep.iter();
        s.bases.retain(|_| *flags.next().unwrap());
        let mut flags = keep.iter();
        s.rows.retain(|_| *flags.next().unwrap());
    }
}
