//! Series positions: grouping, stacking, percentages and supplied interval bases.
use crate::Series;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Position {
    Identity,
    Dodge,
    Stack,
    PercentStack,
    Interval,
    /// 直方图：柱子占满整个区间，各曲线在同一区间重叠。
    Bin,
}

impl Position {
    pub fn is_stacked(self) -> bool {
        matches!(self, Self::Stack | Self::PercentStack)
    }
    pub(crate) fn apply(self, series: &mut [Series]) {
        if self.is_stacked() {
            stack_bars(series, self == Self::PercentStack);
        }
    }
    pub(crate) fn offset(self, spacing: f64, index: usize, count: usize) -> f64 {
        if matches!(self, Self::Dodge | Self::Interval) {
            spacing * 0.7 * ((index as f64 + 0.5) / count.max(1) as f64 - 0.5)
        } else {
            0.0
        }
    }
    pub(crate) fn span(self, category: f64, spacing: f64, index: usize, count: usize) -> (f64, f64) {
        if self == Self::Bin {
            return (category - spacing / 2.0, category + spacing / 2.0);
        }
        let gw = spacing * 0.7;
        let count = if self.is_stacked() { 1 } else { count.max(1) };
        let index = if self.is_stacked() { 0 } else { index };
        let bw = gw / count as f64;
        (
            category - gw / 2.0 + index as f64 * bw,
            category - gw / 2.0 + (index as f64 + 1.0) * bw,
        )
    }
}

fn stack_bars(series: &mut [Series], percent: bool) {
    use std::collections::BTreeMap;
    let key = |row: usize, x: f64| (row, if x == 0.0 { 0 } else { x.to_bits() });
    let mut totals: BTreeMap<_, [f64; 2]> = BTreeMap::new();
    let mut scales: BTreeMap<_, [f64; 2]> = BTreeMap::new();
    if percent {
        for s in series.iter().filter(|s| !s.line_only) {
            for (p, row) in s.points.iter().zip(&s.rows) {
                let scale = &mut scales.entry(key(*row, p[0])).or_default()[usize::from(p[1] < 0.0)];
                *scale = scale.max(p[1].abs());
            }
        }
        for s in series.iter().filter(|s| !s.line_only) {
            for (p, row) in s.points.iter().zip(&s.rows) {
                let key = key(*row, p[0]);
                let sign = usize::from(p[1] < 0.0);
                let scale = scales[&key][sign];
                totals.entry(key).or_default()[sign] += if scale > 0.0 { p[1].abs() / scale } else { 0.0 };
            }
        }
    }
    let mut sums: BTreeMap<_, [f64; 2]> = BTreeMap::new();
    for s in series.iter_mut().filter(|s| !s.line_only) {
        for (((p, base), row), errors) in s.points.iter_mut().zip(&mut s.bases).zip(&s.rows).zip(&mut s.errors) {
            let key = key(*row, p[0]);
            let sign = usize::from(p[1] < 0.0);
            if percent {
                let total = totals[&key][sign];
                let (scale, factor) = if total > 0.0 {
                    (scales[&key][sign], 100.0 / total)
                } else {
                    (1.0, 0.0)
                };
                p[1] = p[1] / scale * factor;
                errors[1] = errors[1].map(|e| e / scale * factor).filter(|e| e.is_finite());
            }
            let sum = &mut sums.entry(key).or_default()[sign];
            *base = *sum;
            *sum += p[1];
            p[1] = *sum;
        }
    }
}
