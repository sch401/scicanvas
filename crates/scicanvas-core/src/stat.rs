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

/// 直方图的派生路径和实际区间宽度。
pub(crate) struct HistogramData {
    pub paths: Vec<Vec<[f64; 2]>>,
    pub bin_width: Option<f64>,
}

/// 各数据曲线使用共同等宽区间，默认按 Sturges 规则分箱。
pub(crate) fn histogram(series: &mut [Series], options: &crate::page::HistogramOptions) -> HistogramData {
    let mut options = *options;
    if !options.is_valid() {
        options.bins = crate::page::HistogramBins::Auto;
        options.range = None;
    }
    let mut result = HistogramData {
        paths: vec![Vec::new(); series.len()],
        bin_width: None,
    };
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
        return result;
    };
    let automatic = [
        pools.iter().flatten().copied().fold(f64::INFINITY, f64::min),
        pools.iter().flatten().copied().fold(f64::NEG_INFINITY, f64::max),
    ];
    let edges = histogram_boundaries(options.range.unwrap_or(automatic), longest, options.bins);
    result.bin_width = edges
        .get(1)
        .map(|edge| edge - edges[0])
        .filter(|width| width.is_finite() && *width > 0.0);
    for (index, (s, values)) in series.iter_mut().zip(&pools).enumerate() {
        if s.line_only {
            continue;
        }
        let selected: Vec<_> = values
            .iter()
            .copied()
            .filter(|v| options.range.is_none_or(|[lo, hi]| *v >= lo && *v <= hi))
            .collect();
        let counts = crate::stats::histogram_edges(&selected, &edges).unwrap_or_default();
        if options.normal_curve {
            result.paths[index] = histogram_normal_curve(values, &counts, options.cumulative);
        }
        let mut cumulative = 0;
        s.points = counts
            .iter()
            .map(|b| {
                cumulative += b.count;
                [
                    (b.lo + b.hi) / 2.0,
                    if options.cumulative { cumulative } else { b.count } as f64,
                ]
            })
            .collect();
        s.bases = vec![0.0; s.points.len()];
        s.rows = (0..s.points.len()).collect();
        s.errors = vec![[None; 2]; s.points.len()];
        s.point_styles = Vec::new();
    }
    result
}

fn histogram_boundaries(mut range: [f64; 2], n: usize, bins: crate::page::HistogramBins) -> Vec<f64> {
    use crate::page::{HistogramBins, HistogramOptions};
    if range[0] == range[1] {
        range = [range[0] - 0.5, range[1] + 0.5];
    }
    let [lo, hi] = range;
    let automatic = crate::stats::sturges_bins(n);
    let count = match bins {
        HistogramBins::Auto => automatic,
        HistogramBins::Count(n) => n,
        HistogramBins::Width(width) => {
            let count = ((hi - lo) / width).ceil();
            if count.is_finite() && (1.0..=HistogramOptions::MAX_BINS as f64).contains(&count) {
                let mut n = count as usize;
                if lo + n as f64 * width < hi {
                    n += 1;
                }
                if n <= HistogramOptions::MAX_BINS && (lo + n as f64 * width).is_finite() {
                    return (0..=n).map(|i| lo + i as f64 * width).collect();
                }
            }
            automatic
        }
    };
    let step = (hi - lo) / count as f64;
    let mut edges: Vec<_> = (0..=count).map(|i| lo + step * i as f64).collect();
    edges[count] = hi;
    edges
}

fn histogram_normal_curve(values: &[f64], bins: &[crate::stats::Bin], cumulative: bool) -> Vec<[f64; 2]> {
    let (Some(first), Some(last)) = (bins.first(), bins.last()) else {
        return Vec::new();
    };
    if values.len() < 2 {
        return Vec::new();
    }
    let n = values.len() as f64;
    let mean = values.iter().sum::<f64>() / n;
    let sd = (values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / (n - 1.0)).sqrt();
    if !sd.is_finite() || sd == 0.0 {
        return Vec::new();
    }
    (0..100)
        .map(|i| {
            let x = if i == 99 {
                last.hi
            } else {
                first.lo + (last.hi - first.lo) * i as f64 / 99.0
            };
            let z = (x - mean) / sd;
            let y = if cumulative {
                n * crate::stats::norm_cdf(z)
            } else {
                n * (first.hi - first.lo) / sd * (-0.5 * z * z).exp() / (2.0 * std::f64::consts::PI).sqrt()
            };
            [x, y]
        })
        .collect()
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

/// 各 Y 列占一个类别位置，统计量独立于 Series 的原始样本。
pub(crate) fn boxes(plots: &mut [crate::page::PreparedPlot]) {
    for (index, p) in plots.iter_mut().filter(|p| !p.series.line_only).enumerate() {
        let values: Vec<_> = p.series.points.iter().map(|p| p[1]).collect();
        p.box_stats = crate::stats::box_stats(&values);
        for point in &mut p.series.points {
            point[0] = index as f64 + 1.0;
        }
        p.series.errors.fill([None, None]);
        p.series.point_styles.clear();
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
    let scales = [g.effective_x_scale(), g.effective_y_scale()];
    let sides = std::array::from_fn::<_, 3, _>(|axis| {
        let (dim, auto, min) = match axis {
            0 => (0, g.x_auto, g.x_min),
            1 => (1, g.y_auto, g.y_min),
            _ => (1, g.y2_auto, g.y2_min),
        };
        if auto {
            series
                .iter()
                .filter(|s| axis == 0 || g.plot_style(&s.key).is_some_and(|s| s.right_axis) == (axis == 2))
                .flat_map(|s| s.points.iter().map(|p| p[dim]))
                .find(|v| scales[dim].domain_ok(*v))
                .map_or(1., f64::signum)
        } else {
            min.signum()
        }
    });
    let horizontal = crate::presets::recipe(g.plot_type).orientation == crate::coord::Orientation::Horizontal;
    for s in series {
        let y_side = if g.plot_style(&s.key).is_some_and(|s| s.right_axis) {
            sides[2]
        } else {
            sides[1]
        };
        let domain = |dim: usize, v: f64| {
            scales[dim].domain_ok(v)
                && (scales[dim] != crate::ticks::AxisScale::Reciprocal
                    || v.signum() == if dim == 0 { sides[0] } else { y_side })
        };
        // 均值变换后的点没有原始行，按顺序编号，连线不在组之间断开
        if (g.plot_type == PlotType::Area || g.plot_type.is_spline() || g.plot_type.has_line())
            && g.plot_type != PlotType::LineSeries
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
                    && (domain(0, p[0]) || (horizontal && g.plot_type.is_bar() && !s.line_only && domain(0, *base)))
                    && (domain(1, p[1]) || (!horizontal && g.plot_type.is_bar() && !s.line_only && domain(1, *base)))
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

/// 偏移不改变列数据或源标识；派生拟合线跟随对应原曲线，不占序号。
pub(crate) fn offset(layer: &crate::page::Layer, plots: &mut [crate::page::PreparedPlot]) {
    let options = layer.offset.or_else(|| {
        matches!(layer.plot_type, PlotType::OffsetLine | PlotType::Waterfall).then_some(Default::default())
    });
    let Some(options) = options else {
        return;
    };
    let span = |values: Vec<f64>| {
        values
            .iter()
            .copied()
            .fold(None, |range, v| {
                Some(range.map_or((v, v), |(lo, hi): (f64, f64)| (lo.min(v), hi.max(v))))
            })
            .map_or(0.0, |(lo, hi)| hi - lo)
    };
    let (dx, dy) = if options.auto {
        let dy = plots
            .iter()
            .filter(|p| !p.series.line_only)
            .map(|p| span(p.series.points.iter().map(|v| v[1]).collect()))
            .fold(0.0, f64::max);
        let dx = if layer.plot_type == PlotType::Waterfall {
            0.05 * span(
                plots
                    .iter()
                    .filter(|p| !p.series.line_only)
                    .flat_map(|p| p.series.points.iter().map(|v| v[0]))
                    .collect(),
            )
        } else {
            0.0
        };
        (dx, dy)
    } else {
        (options.dx, options.dy)
    };
    let shifts: std::collections::BTreeMap<_, _> = plots
        .iter()
        .filter(|p| !p.series.line_only)
        .enumerate()
        .map(|(i, p)| (p.series.key.clone(), [i as f64 * dx, i as f64 * dy]))
        .collect();
    for plot in plots {
        let shift = if plot.series.line_only {
            plot.series
                .source
                .as_ref()
                .and_then(|k| shifts.get(k))
                .copied()
                .unwrap_or([0.; 2])
        } else {
            shifts[&plot.series.key]
        };
        for point in &mut plot.series.points {
            point[0] += shift[0];
            point[1] += shift[1];
        }
        for point in plot.paths.iter_mut().flatten() {
            point[0] += shift[0];
            point[1] += shift[1];
        }
        plot.offset = shift;
    }
}

pub(crate) fn columns(layer: &crate::page::Layer, prepared: &mut [crate::page::PreparedPlot]) {
    if layer.plot_type == crate::PlotType::LineSeries {
        for (i, p) in prepared.iter_mut().filter(|p| !p.series.line_only).enumerate() {
            for point in &mut p.series.points {
                point[0] = (i + 1) as f64;
            }
            for e in &mut p.series.errors {
                e[0] = None;
            }
            if p.plot.style.as_ref().is_none_or(|s| s.color.is_none()) {
                p.style.color = if layer.color_map == crate::colormap::ColorMap::Origin {
                    crate::SeriesColor::Palette(0)
                } else {
                    crate::SeriesColor::Rgb(layer.color_map.sample(0.0))
                };
            }
        }
    }
    if layer.plot_type == crate::PlotType::ColumnScatter {
        for (i, p) in prepared.iter_mut().filter(|p| !p.series.line_only).enumerate() {
            let center = (i + 1) as f64;
            for error in &mut p.series.errors {
                error[0] = None;
            }
            let mean = p.series.points.iter().map(|p| p[1]).sum::<f64>() / p.series.points.len() as f64;
            for (j, point) in p.series.points.iter_mut().enumerate() {
                point[0] = center + 0.15 * (2.0 * (j as f64 * 0.618034).fract() - 1.0);
            }
            if mean.is_finite() {
                p.paths.push(vec![[center - 0.2, mean], [center + 0.2, mean]]);
            }
        }
    }
}
