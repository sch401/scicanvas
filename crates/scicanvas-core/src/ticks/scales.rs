//! Axis transformations and nonlegacy tick generation. Linear and Log10 keep their original path.
use super::{axis, finite_range, Axis};
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum AxisScale {
    #[default]
    Linear,
    Log10,
    Log2,
    Ln,
    Reciprocal,
    Probability,
    Logit,
}
impl AxisScale {
    pub const ALL: [Self; 7] = [
        Self::Linear,
        Self::Log10,
        Self::Log2,
        Self::Ln,
        Self::Reciprocal,
        Self::Probability,
        Self::Logit,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Linear => "线性",
            Self::Log10 => "Log10",
            Self::Log2 => "Log2",
            Self::Ln => "Ln",
            Self::Reciprocal => "倒数",
            Self::Probability => "概率（百分数）",
            Self::Logit => "Logit（0–1）",
        }
    }
    pub fn default_value(self) -> f64 {
        match self {
            Self::Linear => 0.,
            Self::Probability => 50.,
            Self::Logit => 0.5,
            _ => 1.,
        }
    }
    pub fn is_log(self) -> bool {
        matches!(self, Self::Log10 | Self::Log2 | Self::Ln)
    }
    pub fn domain_ok(self, v: f64) -> bool {
        v.is_finite()
            && match self {
                Self::Linear => true,
                Self::Log10 | Self::Log2 | Self::Ln => v > 0.,
                Self::Reciprocal => v != 0.,
                Self::Probability => v > 0. && v < 100.,
                Self::Logit => v > 0. && v < 1.,
            }
    }
    pub fn forward(self, v: f64) -> f64 {
        match self {
            Self::Linear => v,
            Self::Log10 => v.log10(),
            Self::Log2 => v.log2(),
            Self::Ln => v.ln(),
            Self::Reciprocal => 1. / v,
            Self::Probability => crate::stats::norm_ppf(v / 100.),
            Self::Logit => v.ln() - (-v).ln_1p(),
        }
    }
    pub fn inverse(self, v: f64) -> f64 {
        match self {
            Self::Linear => v,
            Self::Log10 => 10f64.powf(v),
            Self::Log2 => 2f64.powf(v),
            Self::Ln => v.exp(),
            Self::Reciprocal => 1. / v,
            Self::Probability => 100. * crate::stats::norm_cdf(v),
            Self::Logit => 1. / (1. + (-v).exp()),
        }
    }
    pub fn range_ok(self, lo: f64, hi: f64) -> bool {
        self.domain_ok(lo) && self.domain_ok(hi) && lo < hi && (self != Self::Reciprocal || lo.signum() == hi.signum())
    }
    fn fallback(self) -> (f64, f64) {
        match self {
            Self::Probability => (0.1, 99.9),
            Self::Logit => (0.001, 0.999),
            _ => (1., 10.),
        }
    }
}
pub fn axis_scaled(
    values: &[f64],
    auto: bool,
    min: f64,
    max: f64,
    scale: AxisScale,
    force_zero: bool,
    spacing: &crate::page::TickSpacing,
) -> Axis {
    if matches!(scale, AxisScale::Linear | AxisScale::Log10) {
        return axis(values, auto, min, max, scale == AxisScale::Log10, force_zero, spacing);
    }
    let side = values
        .iter()
        .copied()
        .find(|v| scale.domain_ok(*v))
        .map_or(1., f64::signum);
    let values: Vec<_> = values
        .iter()
        .copied()
        .filter(|v| scale.domain_ok(*v) && (scale != AxisScale::Reciprocal || v.signum() == side))
        .collect();
    let (mut lo, mut hi) = if auto {
        finite_range(&values).unwrap_or_else(|| scale.fallback())
    } else {
        (min, max)
    };
    if !scale.range_ok(lo, hi) {
        if lo == hi && scale.domain_ok(lo) {
            let t = scale.forward(lo);
            let a = scale.inverse(t - 0.5);
            let b = scale.inverse(t + 0.5);
            lo = a.min(b);
            hi = a.max(b);
        } else {
            (lo, hi) = scale.fallback();
        }
    }
    if auto && matches!(scale, AxisScale::Log2 | AxisScale::Ln) {
        lo = scale.inverse((scale.forward(lo) + 1e-12).floor());
        hi = scale.inverse((scale.forward(hi) - 1e-12).ceil());
        if hi <= lo {
            hi = scale.inverse(scale.forward(lo) + 1.);
        }
    }
    let ticks = match scale {
        AxisScale::Log2 | AxisScale::Ln => {
            let first = (scale.forward(lo) - 1e-12).ceil() as i32;
            let last = (scale.forward(hi) + 1e-12).floor() as i32;
            (first..=last).map(|e| scale.inverse(f64::from(e))).collect()
        }
        AxisScale::Probability => [0.1, 1., 5., 10., 20., 30., 50., 70., 80., 90., 95., 99., 99.9]
            .into_iter()
            .filter(|v| *v >= lo && *v <= hi)
            .collect(),
        AxisScale::Logit => [0.001, 0.01, 0.05, 0.1, 0.25, 0.5, 0.75, 0.9, 0.95, 0.99, 0.999]
            .into_iter()
            .filter(|v| *v >= lo && *v <= hi)
            .collect(),
        AxisScale::Reciprocal => {
            let mut candidates = Vec::new();
            let small = lo.abs().min(hi.abs());
            let large = lo.abs().max(hi.abs());
            for exponent in small.log10().floor() as i32..=large.log10().ceil() as i32 {
                for factor in [1., 2., 5.] {
                    let v = factor * 10f64.powi(exponent) * lo.signum();
                    if v >= lo && v <= hi {
                        candidates.push(v);
                    }
                }
            }
            candidates.sort_by(f64::total_cmp);
            let mut kept = Vec::new();
            let mut previous = f64::NEG_INFINITY;
            for v in candidates {
                let f = (scale.forward(v) - scale.forward(lo)) / (scale.forward(hi) - scale.forward(lo));
                if f - previous >= 0.08 {
                    kept.push(v);
                    previous = f;
                }
            }
            kept
        }
        _ => unreachable!(),
    };
    super::with_spacing(
        Axis {
            lo,
            hi,
            ticks,
            minor: Vec::new(),
            scale,
        },
        &values,
        auto,
        force_zero,
        spacing,
    )
}
