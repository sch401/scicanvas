use super::guess::linreg;
use super::*;

pub(super) fn data_origin(xs: &[f64], ys: &[f64], sigma: Option<&[f64]>) -> f64 {
    xs.iter()
        .zip(ys)
        .enumerate()
        .filter(|(i, (x, y))| {
            x.is_finite() && y.is_finite() && sigma.is_none_or(|s| s.get(*i).is_some_and(|s| !s.is_nan()))
        })
        .map(|(_, (&x, _))| x)
        .min_by(f64::total_cmp)
        .unwrap_or(0.0)
}
impl FitFunction {
    pub(super) fn for_data(self, xs: &[f64], ys: &[f64], sigma: Option<&[f64]>) -> Self {
        if self == Self::Builtin(Model::SineDamp) {
            Self::SineDamp {
                origin: data_origin(xs, ys, sigma),
            }
        } else {
            self
        }
    }
    pub(super) fn points(&self, xs: &[f64], ys: &[f64], sigma: Option<&[f64]>) -> Result<WeightedPoints, String> {
        if *self != Self::Builtin(Model::BET) {
            return weighted_points(xs, ys, sigma);
        }
        if sigma.is_some_and(|s| s.len() != xs.len() || s.len() != ys.len()) {
            return Err("Y 误差的个数与数据点个数不一致".into());
        }
        let keep: Vec<_> = xs
            .iter()
            .zip(ys)
            .enumerate()
            .filter(|(_, (&x, _))| x > 0.0 && x < 1.0)
            .map(|(i, _)| i)
            .collect();
        let x: Vec<_> = keep.iter().map(|&i| xs[i]).collect();
        let y: Vec<_> = keep.iter().map(|&i| ys[i]).collect();
        let s = sigma.map(|s| keep.iter().map(|&i| s[i]).collect::<Vec<_>>());
        weighted_points(&x, &y, s.as_deref())
    }
}

impl Model {
    pub(super) fn guess_extended(self, xs: &[f64], ys: &[f64]) -> Vec<f64> {
        match self {
            Self::Sine | Self::SquareWave | Self::SineDamp => wave_guess(self, xs, ys),
            Self::Pulse => pulse_guess(xs, ys),
            Self::SubstrateInhibition => {
                let peak = (0..ys.len()).max_by(|a, b| ys[*a].total_cmp(&ys[*b])).unwrap();
                let end = (peak + 1).max(2).min(xs.len());
                let mm = Self::MichaelisMenten.guess(&xs[..end], &ys[..end]);
                vec![
                    mm[0],
                    mm[1],
                    5.0 * xs[peak].abs().max(xs[xs.len() - 1] / xs.len() as f64),
                ]
            }
            Self::CompetitiveInhibition | Self::NoncompetitiveInhibition | Self::UncompetitiveInhibition => {
                let mm = Self::MichaelisMenten.guess(xs, ys);
                vec![mm[0], mm[1], mm[1].abs().max(1.0), 1.0]
            }
            Self::BET => {
                let (s, i) = linreg(
                    xs.iter()
                        .zip(ys)
                        .filter(|(x, y)| **x >= 0.05 && **x <= 0.35 && **y != 0.0)
                        .map(|(x, y)| (*x, x / (y * (1.0 - x)))),
                );
                if i > 0.0 && s + i > 0.0 {
                    vec![1.0 / (s + i), s / i + 1.0]
                } else {
                    vec![ys.iter().copied().fold(0.0, f64::max), 10.0]
                }
            }
            Self::TwoSiteBind | Self::OneSiteTotal => binding_guess(self, xs, ys),
            _ => unreachable!("新模型初值分派"),
        }
    }
}

fn wave_guess(model: Model, xs: &[f64], ys: &[f64]) -> Vec<f64> {
    let n = xs.len();
    let mean = ys.iter().sum::<f64>() / n as f64;
    let lo = ys.iter().copied().fold(f64::INFINITY, f64::min);
    let hi = ys.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let span = xs[n - 1] - xs[0];
    let mut crossings = Vec::new();
    for i in 1..n {
        if ys[i - 1] < mean && ys[i] >= mean {
            crossings.push(xs[i - 1] + (xs[i] - xs[i - 1]) * (mean - ys[i - 1]) / (ys[i] - ys[i - 1]));
        }
    }
    let fallback = if crossings.len() > 1 {
        (crossings[crossings.len() - 1] - crossings[0]) / (crossings.len() - 1) as f64
    } else if span > 0.0 {
        span
    } else {
        1.0
    };
    let period = if n >= 8 && span > 0.0 {
        let grid = crate::interpolate::grid(xs[0], xs[n - 1], n);
        crate::interpolate::interpolate(xs, ys, crate::interpolate::InterpMethod::Linear, &grid)
            .ok()
            .and_then(|values| {
                crate::fft::spectrum(
                    &values.iter().map(|v| v - mean).collect::<Vec<_>>(),
                    span / (n - 1) as f64,
                    crate::fft::Window::Hann,
                )
                .ok()
            })
            .and_then(|s| {
                (1..s.freq.len())
                    .max_by(|&a, &b| s.amplitude[a].total_cmp(&s.amplitude[b]))
                    .map(|i| 1.0 / s.freq[i])
            })
            .unwrap_or(fallback)
    } else {
        fallback
    };
    let phase = crossings.first().copied().unwrap_or(xs[0]);
    let amplitude = (hi - lo) / 2.0;
    if matches!(model, Model::Sine | Model::SquareWave) {
        return vec![mean, amplitude, phase, period];
    }
    let peaks: Vec<_> = (1..n - 1)
        .filter(|&i| {
            (ys[i] - mean).abs() >= (ys[i - 1] - mean).abs() && (ys[i] - mean).abs() > (ys[i + 1] - mean).abs()
        })
        .collect();
    let (slope, _) = linreg(
        peaks
            .iter()
            .filter(|&&i| (ys[i] - mean).abs() > 0.0)
            .map(|&i| (xs[i], (ys[i] - mean).abs().ln())),
    );
    let decay = if slope < 0.0 { -1.0 / slope } else { span.max(period) };
    vec![mean, amplitude, phase, period, decay]
}

fn pulse_guess(xs: &[f64], ys: &[f64]) -> Vec<f64> {
    let n = ys.len();
    let baseline = ys[..(n / 10).max(1)].iter().sum::<f64>() / (n / 10).max(1) as f64;
    let peak = (0..n).max_by(|a, b| ys[*a].total_cmp(&ys[*b])).unwrap();
    let height = ys[peak] - baseline;
    let start = (0..=peak).find(|&i| ys[i] >= baseline + 0.1 * height).unwrap_or(0);
    let x0 = xs[start];
    let step = (xs[n - 1] - xs[0]) / n as f64;
    let t1 = ((xs[peak] - x0) / 3.0).max(step);
    let end = (peak + 1..n)
        .find(|&i| ys[i] <= baseline + height / std::f64::consts::E)
        .unwrap_or(n - 1);
    let t2 = (xs[end] - xs[peak]).max(step);
    let delta = xs[peak] - x0;
    let amplitude = height / ((1.0 - (-delta / t1).exp()) * (-delta / t2).exp());
    vec![baseline, amplitude, x0, t1, t2, 1.0]
}

fn binding_guess(model: Model, xs: &[f64], ys: &[f64]) -> Vec<f64> {
    let tail = (xs.len() * 3 / 4).min(xs.len().saturating_sub(2));
    let (slope, _) = linreg(xs[tail..].iter().zip(&ys[tail..]).map(|(x, y)| (*x, *y)));
    let background = ys[0];
    let slope = if model == Model::OneSiteTotal { slope } else { 0.0 };
    let corrected: Vec<_> = xs
        .iter()
        .zip(ys)
        .map(|(x, y)| y - slope * x - if model == Model::OneSiteTotal { background } else { 0.0 })
        .collect();
    let fit = fit_function_weighted(
        FitFunction::Builtin(Model::Langmuir),
        xs,
        &corrected,
        None,
        None,
        &[],
        &[],
    )
    .ok();
    let guess = fit.map_or_else(|| Model::Langmuir.guess(xs, &corrected), |f| f.params);
    let kd = 1.0 / guess[1];
    if model == Model::TwoSiteBind {
        vec![guess[0] / 2.0, kd * 0.1, guess[0] / 2.0, kd * 10.0]
    } else {
        vec![guess[0], kd, slope, background]
    }
}
