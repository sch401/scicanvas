//! Composite peaks reuse the built-in profiles and LM solver.
use super::*;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum PeakBaseline {
    None,
    Constant,
    Linear,
}
#[derive(Clone, Debug, PartialEq)]
pub struct PeakSum {
    pub shape: Model,
    pub count: u8,
    pub baseline: PeakBaseline,
    names: Vec<String>,
    pub(super) name: String,
}
impl PeakSum {
    pub fn new(shape: Model, count: u8, baseline: PeakBaseline) -> Result<Self, String> {
        if !matches!(shape, Model::Gauss | Model::Lorentz | Model::Voigt | Model::PsdVoigt1)
            || !(1..=10).contains(&count)
        {
            return Err("峰形须为 Gauss、Lorentz、Voigt 或 PsdVoigt1，峰数为 1–10".into());
        }
        let mut names = Vec::new();
        for i in 1..=count {
            names.extend(shape.params()[1..].iter().map(|p| format!("{p}_{i}")));
        }
        if baseline != PeakBaseline::None {
            names.push("y0".into());
        }
        if baseline == PeakBaseline::Linear {
            names.push("B".into());
        }
        Ok(Self {
            shape,
            count,
            baseline,
            names,
            name: format!("{} × {}", shape.name(), count),
        })
    }
    pub fn params(&self) -> Vec<&str> {
        self.names.iter().map(String::as_str).collect()
    }
    fn stride(&self) -> usize {
        self.shape.params().len() - 1
    }
    fn peak_params(&self, p: &[f64], index: usize) -> Vec<f64> {
        let n = self.stride();
        std::iter::once(0.)
            .chain(p[index * n..(index + 1) * n].iter().copied())
            .collect()
    }
    pub fn component(&self, x: f64, p: &[f64], index: usize) -> f64 {
        self.shape.eval(x, &self.peak_params(p, index))
    }
    pub fn eval(&self, x: f64, p: &[f64]) -> f64 {
        let n = self.stride() * usize::from(self.count);
        let base = match self.baseline {
            PeakBaseline::None => 0.,
            PeakBaseline::Constant => p[n],
            PeakBaseline::Linear => p[n] + p[n + 1] * x,
        };
        base + (0..usize::from(self.count))
            .map(|i| self.component(x, p, i))
            .sum::<f64>()
    }
    pub fn limits(&self) -> Vec<ParamLimits> {
        self.names
            .iter()
            .map(|name| {
                if name.starts_with('w') {
                    ParamLimits {
                        lower: Some(1e-12),
                        ..Default::default()
                    }
                } else if name.starts_with("A_") {
                    ParamLimits {
                        lower: Some(0.),
                        ..Default::default()
                    }
                } else if name.starts_with("mu_") {
                    ParamLimits {
                        lower: Some(0.),
                        upper: Some(1.),
                        ..Default::default()
                    }
                } else {
                    ParamLimits::default()
                }
            })
            .collect()
    }
    pub fn guess(&self, xs: &[f64], ys: &[f64], prominence: f64) -> Result<Vec<f64>, String> {
        let (xs, ys, _) = weighted_points(xs, ys, None)?;
        if xs.len() < 3 {
            return Err("多峰寻峰至少需要 3 个有效点".into());
        }
        let edge = (ys.len() / 20).max(1);
        let baseline = ys[..edge]
            .iter()
            .chain(&ys[ys.len() - edge..])
            .copied()
            .fold(f64::INFINITY, f64::min);
        let baseline = if self.baseline == PeakBaseline::None {
            0.
        } else {
            baseline
        };
        let corrected: Vec<_> = ys.iter().map(|y| y - baseline).collect();
        let span =
            ys.iter().copied().fold(f64::NEG_INFINITY, f64::max) - ys.iter().copied().fold(f64::INFINITY, f64::min);
        let mut peaks = crate::peaks::find_peaks(&xs, &corrected, prominence * span);
        peaks.sort_by(|a, b| b.prominence.total_cmp(&a.prominence));
        if peaks.len() < usize::from(self.count) {
            return Err(format!("检测到 {} 个峰，请降低峰数或最小突出度", peaks.len()));
        }
        peaks.truncate(usize::from(self.count));
        peaks.sort_by(|a, b| a.x.total_cmp(&b.x));
        let mut p = Vec::new();
        for peak in peaks {
            let width = peak.fwhm.max((xs[xs.len() - 1] - xs[0]) / xs.len() as f64);
            let mut one = match self.shape {
                Model::Gauss => vec![0., peak.x, width / (4f64.ln()).sqrt(), 1.],
                Model::Lorentz => vec![0., peak.x, width, 1.],
                Model::Voigt => vec![0., peak.x, 1., width / 2., width / 2.],
                Model::PsdVoigt1 => vec![0., peak.x, 1., width, 0.5],
                _ => unreachable!(),
            };
            let area = self.shape.params().iter().position(|p| *p == "A").unwrap();
            one[area] = peak.height / self.shape.eval(peak.x, &one);
            p.extend_from_slice(&one[1..]);
        }
        if self.baseline != PeakBaseline::None {
            p.push(baseline);
        }
        if self.baseline == PeakBaseline::Linear {
            p.push(0.);
        }
        Ok(p)
    }
    /// Centre, height, full area and FWHM, in that order.
    pub fn metrics(&self, p: &[f64], index: usize) -> [f64; 4] {
        let one = self.peak_params(p, index);
        let names = self.shape.params();
        let get = |name| one[names.iter().position(|n| *n == name).unwrap()];
        let center = get("xc");
        let height = self.shape.eval(center, &one);
        let width = match self.shape {
            Model::Gauss => get("w") * (4f64.ln()).sqrt(),
            Model::Lorentz | Model::PsdVoigt1 => get("w"),
            Model::Voigt => {
                let mut lo = 0.;
                let mut hi = get("wG") + get("wL");
                while self.shape.eval(center + hi, &one) > height / 2. {
                    hi *= 2.;
                }
                for _ in 0..60 {
                    let mid = (lo + hi) / 2.;
                    if self.shape.eval(center + mid, &one) > height / 2. {
                        lo = mid
                    } else {
                        hi = mid
                    }
                }
                lo + hi
            }
            _ => unreachable!(),
        };
        [center, height, get("A"), width]
    }
    pub fn metric_errors(&self, fit: &NlFit, index: usize) -> [f64; 4] {
        let n = fit.params.len();
        let mut gradients = vec![[0.; 4]; n];
        for (i, g) in gradients.iter_mut().enumerate() {
            let h = 1e-5 * fit.params[i].abs().max(1.);
            let mut p = fit.params.clone();
            p[i] += h;
            let a = self.metrics(&p, index);
            p[i] -= 2. * h;
            let b = self.metrics(&p, index);
            for j in 0..4 {
                g[j] = (a[j] - b[j]) / (2. * h);
            }
        }
        std::array::from_fn(|j| {
            let mut variance = 0.;
            for a in 0..n {
                for b in 0..n {
                    variance += gradients[a][j] * fit.cov[a][b] * gradients[b][j];
                }
            }
            variance.max(0.).sqrt()
        })
    }
}
pub(super) fn fit(
    peak: PeakSum,
    xs: &[f64],
    ys: &[f64],
    sigma: Option<&[f64]>,
    initial: Option<&[f64]>,
    limits: &[ParamLimits],
    constraints: &[LinearConstraint],
) -> Result<NlFit, String> {
    if initial.is_some_and(|p| p.len() != peak.params().len()) {
        return Err("多峰初始参数数量不符".into());
    }
    let mut held = peak.limits();
    for (i, limit) in limits.iter().enumerate() {
        if i >= held.len() {
            return Err("多峰参数限制数量过多".into());
        }
        held[i] = *limit;
    }
    if peak.count == 1 && peak.baseline == PeakBaseline::Constant {
        let n = peak.params().len();
        let order: Vec<_> = std::iter::once(n - 1).chain(0..n - 1).collect();
        let start = initial.map(|p| order.iter().map(|i| p[*i]).collect::<Vec<_>>());
        let held: Vec<_> = order.iter().map(|i| held[*i]).collect();
        let constraints: Vec<_> = constraints
            .iter()
            .map(|c| LinearConstraint {
                coef: order.iter().map(|i| c.coef.get(*i).copied().unwrap_or(0.)).collect(),
                ..c.clone()
            })
            .collect();
        let mut result = fit_function_weighted(
            FitFunction::Builtin(peak.shape),
            xs,
            ys,
            sigma,
            start.as_deref(),
            &held,
            &constraints,
        )?;
        let inverse: Vec<_> = (1..n).chain(std::iter::once(0)).collect();
        result.params = inverse.iter().map(|i| result.params[*i]).collect();
        result.se = inverse.iter().map(|i| result.se[*i]).collect();
        result.fixed = inverse.iter().map(|i| result.fixed[*i]).collect();
        result.at_bound = inverse.iter().map(|i| result.at_bound[*i]).collect();
        result.cov = inverse
            .iter()
            .map(|i| inverse.iter().map(|j| result.cov[*i][*j]).collect())
            .collect();
        result.model = FitFunction::PeakSum(peak);
        return Ok(result);
    }
    let auto = if initial.is_none() {
        let (x, y, _) = weighted_points(xs, ys, sigma)?;
        Some(peak.guess(&x, &y, 0.05)?)
    } else {
        None
    };
    let mut session = FitSession::new(
        FitFunction::PeakSum(peak),
        xs,
        ys,
        sigma,
        initial.or(auto.as_deref()),
        &held,
        constraints,
    )?;
    session.optimize();
    session.finish()
}
