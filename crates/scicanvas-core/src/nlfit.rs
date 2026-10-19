//! 非线性拟合：Levenberg–Marquardt（中心差分雅可比），内置 Origin 同名函数与自动初值。
//! 标准误按 cov = (JᵀJ)⁻¹ · RSS / (n − p)，与 SciPy `curve_fit`（默认 absolute_sigma=False）一致。

use crate::analysis::mat_inv;

/// 内置拟合函数。峰、指数、S 形和幂函数的公式与参数名与 Origin 相同；
/// 吸附与酶动力学函数使用文献通用写法（x 为浓度或时间）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Model {
    Gauss,
    Lorentz,
    Voigt,
    PsdVoigt1,
    LogNormal,
    ExpDec1,
    ExpDec2,
    ExpDec3,
    ExpGro1,
    ExpAssoc,
    Boltzmann,
    Logistic,
    DoseResp,
    Hill,
    Gompertz,
    Allometric1,
    Allometric2,
    Power1,
    Log3P1,
    Langmuir,
    Freundlich,
    PseudoFirstOrder,
    PseudoSecondOrder,
    MichaelisMenten,
}

/// 内置函数的最多参数个数（ExpDec3）。
pub const MAX_PARAMS: usize = 7;

impl Model {
    pub const ALL: [Model; 24] = [
        Model::Gauss,
        Model::Lorentz,
        Model::Voigt,
        Model::PsdVoigt1,
        Model::LogNormal,
        Model::ExpDec1,
        Model::ExpDec2,
        Model::ExpDec3,
        Model::ExpGro1,
        Model::ExpAssoc,
        Model::Boltzmann,
        Model::Logistic,
        Model::DoseResp,
        Model::Hill,
        Model::Gompertz,
        Model::Allometric1,
        Model::Allometric2,
        Model::Power1,
        Model::Log3P1,
        Model::Langmuir,
        Model::Freundlich,
        Model::PseudoFirstOrder,
        Model::PseudoSecondOrder,
        Model::MichaelisMenten,
    ];

    /// 对话框里的类别顺序。
    pub const CATEGORIES: [&'static str; 7] = [
        "峰函数",
        "指数",
        "S 形",
        "幂函数与对数",
        "吸附等温线",
        "吸附动力学",
        "酶动力学",
    ];

    pub fn name(self) -> &'static str {
        match self {
            Model::Gauss => "Gauss",
            Model::Lorentz => "Lorentz",
            Model::Voigt => "Voigt",
            Model::PsdVoigt1 => "PsdVoigt1",
            Model::LogNormal => "LogNormal",
            Model::ExpDec1 => "ExpDec1",
            Model::ExpDec2 => "ExpDec2",
            Model::ExpDec3 => "ExpDec3",
            Model::ExpGro1 => "ExpGro1",
            Model::ExpAssoc => "ExpAssoc",
            Model::Boltzmann => "Boltzmann",
            Model::Logistic => "Logistic",
            Model::DoseResp => "DoseResp",
            Model::Hill => "Hill",
            Model::Gompertz => "Gompertz",
            Model::Allometric1 => "Allometric1",
            Model::Allometric2 => "Allometric2",
            Model::Power1 => "Power1",
            Model::Log3P1 => "Log3P1",
            Model::Langmuir => "Langmuir",
            Model::Freundlich => "Freundlich",
            Model::PseudoFirstOrder => "准一级动力学",
            Model::PseudoSecondOrder => "准二级动力学",
            Model::MichaelisMenten => "MichaelisMenten",
        }
    }

    pub fn category(self) -> &'static str {
        match self {
            Model::Gauss | Model::Lorentz | Model::Voigt | Model::PsdVoigt1 | Model::LogNormal => "峰函数",
            Model::ExpDec1 | Model::ExpDec2 | Model::ExpDec3 | Model::ExpGro1 | Model::ExpAssoc => "指数",
            Model::Boltzmann | Model::Logistic | Model::DoseResp | Model::Hill | Model::Gompertz => "S 形",
            Model::Allometric1 | Model::Allometric2 | Model::Power1 | Model::Log3P1 => "幂函数与对数",
            Model::Langmuir | Model::Freundlich => "吸附等温线",
            Model::PseudoFirstOrder | Model::PseudoSecondOrder => "吸附动力学",
            Model::MichaelisMenten => "酶动力学",
        }
    }

    pub fn formula(self) -> &'static str {
        match self {
            Model::Gauss => "y = y0 + A/(w·√(π/2))·exp(−2((x − xc)/w)²)",
            Model::Lorentz => "y = y0 + (2A/π)·w/(4(x − xc)² + w²)",
            Model::Voigt => "y = y0 + A·(Gauss wG ⊗ Lorentz wL)，面积 A",
            Model::PsdVoigt1 => "y = y0 + A·[mu·L(w) + (1 − mu)·G(w)]",
            Model::LogNormal => "y = y0 + A/(√(2π)·w·x)·exp(−ln²(x/xc)/(2w²))",
            Model::ExpDec1 => "y = y0 + A1·exp(−x/t1)",
            Model::ExpDec2 => "y = y0 + A1·exp(−x/t1) + A2·exp(−x/t2)",
            Model::ExpDec3 => "y = y0 + Σ Ai·exp(−x/ti)，i = 1…3",
            Model::ExpGro1 => "y = y0 + A1·exp(x/t1)",
            Model::ExpAssoc => "y = y0 + A1(1 − exp(−x/t1)) + A2(1 − exp(−x/t2))",
            Model::Boltzmann => "y = A2 + (A1 − A2)/(1 + exp((x − x0)/dx))",
            Model::Logistic => "y = A2 + (A1 − A2)/(1 + (x/x0)^p)",
            Model::DoseResp => "y = A1 + (A2 − A1)/(1 + 10^((LOGx0 − x)·p))",
            Model::Hill => "y = START + (END − START)·xⁿ/(kⁿ + xⁿ)",
            Model::Gompertz => "y = a·exp(−exp(−k(x − xc)))",
            Model::Allometric1 => "y = a·x^b",
            Model::Allometric2 => "y = a + b·x^c",
            Model::Power1 => "y = A·(x − xc)^p",
            Model::Log3P1 => "y = a − b·ln(x + c)",
            Model::Langmuir => "q = qm·KL·C/(1 + KL·C)",
            Model::Freundlich => "q = KF·C^(1/n)",
            Model::PseudoFirstOrder => "qt = qe·(1 − exp(−k1·t))",
            Model::PseudoSecondOrder => "qt = qe²·k2·t/(1 + qe·k2·t)",
            Model::MichaelisMenten => "v = Vmax·S/(Km + S)",
        }
    }

    pub fn params(self) -> &'static [&'static str] {
        match self {
            Model::Gauss | Model::Lorentz => &["y0", "xc", "w", "A"],
            Model::Voigt => &["y0", "xc", "A", "wG", "wL"],
            Model::PsdVoigt1 => &["y0", "xc", "A", "w", "mu"],
            Model::LogNormal => &["y0", "xc", "w", "A"],
            Model::ExpDec1 => &["y0", "A1", "t1"],
            Model::ExpDec2 => &["y0", "A1", "t1", "A2", "t2"],
            Model::ExpDec3 => &["y0", "A1", "t1", "A2", "t2", "A3", "t3"],
            Model::ExpGro1 => &["y0", "A1", "t1"],
            Model::ExpAssoc => &["y0", "A1", "t1", "A2", "t2"],
            Model::Boltzmann => &["A1", "A2", "x0", "dx"],
            Model::Logistic => &["A1", "A2", "x0", "p"],
            Model::DoseResp => &["A1", "A2", "LOGx0", "p"],
            Model::Hill => &["START", "END", "k", "n"],
            Model::Gompertz => &["a", "xc", "k"],
            Model::Allometric1 => &["a", "b"],
            Model::Allometric2 => &["a", "b", "c"],
            Model::Power1 => &["A", "xc", "p"],
            Model::Log3P1 => &["a", "b", "c"],
            Model::Langmuir => &["qm", "KL"],
            Model::Freundlich => &["KF", "n"],
            Model::PseudoFirstOrder => &["qe", "k1"],
            Model::PseudoSecondOrder => &["qe", "k2"],
            Model::MichaelisMenten => &["Vmax", "Km"],
        }
    }

    pub fn eval(self, x: f64, p: &[f64]) -> f64 {
        match self {
            Model::Gauss => {
                let (y0, xc, w, a) = (p[0], p[1], p[2], p[3]);
                y0 + a / (w * (std::f64::consts::PI / 2.0).sqrt()) * (-2.0 * ((x - xc) / w).powi(2)).exp()
            }
            Model::Lorentz => {
                let (y0, xc, w, a) = (p[0], p[1], p[2], p[3]);
                y0 + 2.0 * a / std::f64::consts::PI * w / (4.0 * (x - xc).powi(2) + w * w)
            }
            Model::Voigt => {
                let (y0, xc, a, wg, wl) = (p[0], p[1], p[2], p[3], p[4]);
                if !(wg > 0.0 && wl >= 0.0) {
                    return f64::NAN;
                }
                y0 + a * voigt_profile(x - xc, wg / (2.0 * (2.0 * LN2).sqrt()), wl / 2.0)
            }
            Model::PsdVoigt1 => {
                let (y0, xc, a, w, mu) = (p[0], p[1], p[2], p[3], p[4]);
                let d = x - xc;
                let lorentz = 2.0 / PI * w / (4.0 * d * d + w * w);
                let gauss = (4.0 * LN2).sqrt() / (PI.sqrt() * w) * (-4.0 * LN2 * d * d / (w * w)).exp();
                y0 + a * (mu * lorentz + (1.0 - mu) * gauss)
            }
            Model::LogNormal => {
                let (y0, xc, w, a) = (p[0], p[1], p[2], p[3]);
                y0 + a / ((2.0 * PI).sqrt() * w * x) * (-(x / xc).ln().powi(2) / (2.0 * w * w)).exp()
            }
            Model::ExpDec1 => p[0] + p[1] * (-x / p[2]).exp(),
            Model::ExpDec2 => p[0] + p[1] * (-x / p[2]).exp() + p[3] * (-x / p[4]).exp(),
            Model::ExpDec3 => p[0] + p[1] * (-x / p[2]).exp() + p[3] * (-x / p[4]).exp() + p[5] * (-x / p[6]).exp(),
            Model::ExpGro1 => p[0] + p[1] * (x / p[2]).exp(),
            Model::ExpAssoc => p[0] + p[1] * (1.0 - (-x / p[2]).exp()) + p[3] * (1.0 - (-x / p[4]).exp()),
            Model::Boltzmann => p[1] + (p[0] - p[1]) / (1.0 + ((x - p[2]) / p[3]).exp()),
            Model::Logistic => p[1] + (p[0] - p[1]) / (1.0 + (x / p[2]).powf(p[3])),
            Model::DoseResp => p[0] + (p[1] - p[0]) / (1.0 + 10f64.powf((p[2] - x) * p[3])),
            Model::Hill => {
                let xn = x.powf(p[3]);
                p[0] + (p[1] - p[0]) * xn / (p[2].powf(p[3]) + xn)
            }
            Model::Gompertz => p[0] * (-(-p[2] * (x - p[1])).exp()).exp(),
            Model::Allometric1 => p[0] * x.powf(p[1]),
            Model::Allometric2 => p[0] + p[1] * x.powf(p[2]),
            Model::Power1 => p[0] * (x - p[1]).powf(p[2]),
            Model::Log3P1 => p[0] - p[1] * (x + p[2]).ln(),
            Model::Langmuir => p[0] * p[1] * x / (1.0 + p[1] * x),
            Model::Freundlich => p[0] * x.powf(1.0 / p[1]),
            Model::PseudoFirstOrder => p[0] * (1.0 - (-p[1] * x).exp()),
            Model::PseudoSecondOrder => p[0] * p[0] * p[1] * x / (1.0 + p[0] * p[1] * x),
            Model::MichaelisMenten => p[0] * x / (p[1] + x),
        }
    }

    /// 由数据估计初值（数据已按 X 排序）。
    fn guess(self, xs: &[f64], ys: &[f64]) -> Vec<f64> {
        let n = xs.len();
        let (lo, hi) = ys
            .iter()
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), y| (a.min(*y), b.max(*y)));
        let mean = |v: &[f64]| v.iter().sum::<f64>() / v.len() as f64;
        let k = (n / 10).max(1);
        // y 穿过 level 的第一个位置
        let crossing = |level: f64| {
            (1..n)
                .find(|&i| (ys[i - 1] - level) * (ys[i] - level) <= 0.0)
                .map_or(xs[n / 2], |i| 0.5 * (xs[i - 1] + xs[i]))
        };
        match self {
            Model::Gauss | Model::Lorentz => {
                let y0 = lo;
                let peak = (0..n).max_by(|a, b| ys[*a].total_cmp(&ys[*b])).unwrap();
                let h = ys[peak] - y0;
                let half = y0 + h / 2.0;
                let left = (0..peak).rev().find(|&i| ys[i] < half).map_or(xs[0], |i| xs[i]);
                let right = (peak..n).find(|&i| ys[i] < half).map_or(xs[n - 1], |i| xs[i]);
                let fwhm = (right - left).max((xs[n - 1] - xs[0]) / n as f64);
                if self == Model::Gauss {
                    let w = fwhm / (2.0 * 2f64.ln()).sqrt();
                    vec![y0, xs[peak], w, h * w * (std::f64::consts::PI / 2.0).sqrt()]
                } else {
                    vec![y0, xs[peak], fwhm, h * std::f64::consts::PI * fwhm / 2.0]
                }
            }
            Model::Voigt | Model::PsdVoigt1 => {
                let g = Model::Gauss.guess(xs, ys);
                let (y0, xc, h) = (g[0], g[1], g[3] / (g[2] * (PI / 2.0).sqrt()));
                let fwhm = g[2] * (2.0 * 2f64.ln()).sqrt();
                if self == Model::Voigt {
                    // 两个宽度相等时 Voigt 半峰宽约为 1.64 倍
                    let w = fwhm / 1.64;
                    vec![y0, xc, h * fwhm * 1.3, w, w]
                } else {
                    vec![y0, xc, h * fwhm * 1.3, fwhm, 0.5]
                }
            }
            Model::LogNormal => {
                let g = Model::Gauss.guess(xs, ys);
                let (y0, xpeak, h) = (g[0], g[1].max(1e-12), g[3] / (g[2] * (PI / 2.0).sqrt()));
                let fwhm = g[2] * (2.0 * 2f64.ln()).sqrt();
                // 半高处 x 的对数宽度 ≈ 2√(2ln2)·w
                let (left, right) = ((xpeak - fwhm / 2.0).max(xpeak * 0.05), xpeak + fwhm / 2.0);
                let w = ((right / left).ln() / (2.0 * (2.0 * LN2).sqrt())).clamp(0.05, 3.0);
                let xc = xpeak * (w * w).exp();
                vec![y0, xc, w, h * (2.0 * PI).sqrt() * w * xc * (-w * w / 2.0).exp()]
            }
            Model::ExpDec2 | Model::ExpDec3 => {
                let e = Model::ExpDec1.guess(xs, ys);
                if self == Model::ExpDec2 {
                    vec![e[0], e[1] / 2.0, e[2] / 2.0, e[1] / 2.0, e[2] * 2.0]
                } else {
                    vec![e[0], e[1] / 3.0, e[2] / 3.0, e[1] / 3.0, e[2], e[1] / 3.0, e[2] * 3.0]
                }
            }
            Model::ExpGro1 => {
                // y0 略低于最小值，ln(y − y0) 对 x 做直线
                let y0 = lo - 0.01 * (hi - lo).max(1e-12);
                let (slope, intercept) = linreg(xs.iter().zip(ys).map(|(x, y)| (*x, (y - y0).ln())));
                let t1 = if slope > 0.0 {
                    1.0 / slope
                } else {
                    (xs[n - 1] - xs[0]) / 3.0
                };
                vec![y0, intercept.exp(), t1]
            }
            Model::ExpAssoc => {
                let y0 = mean(&ys[..k]);
                let total = mean(&ys[n - k..]) - y0;
                let tc = (crossing(y0 + 0.632 * total) - xs[0])
                    .abs()
                    .max((xs[n - 1] - xs[0]) / n as f64);
                vec![y0, total / 2.0, tc / 2.0, total / 2.0, tc * 2.0]
            }
            Model::DoseResp | Model::Hill => {
                let a1 = mean(&ys[..k]);
                let a2 = mean(&ys[n - k..]);
                let mid = crossing(0.5 * (a1 + a2));
                if self == Model::DoseResp {
                    // 中点处斜率 = (A2 − A1)·p·ln10/4
                    let i = (1..n).find(|&i| xs[i] >= mid).unwrap_or(n - 1);
                    let slope = (ys[i] - ys[i - 1]) / (xs[i] - xs[i - 1]).max(1e-12);
                    let p = 4.0 * slope / ((a2 - a1) * std::f64::consts::LN_10);
                    vec![a1, a2, mid, if p.is_finite() && p != 0.0 { p } else { 1.0 }]
                } else {
                    vec![a1, a2, mid.abs().max(1e-12), 2.0]
                }
            }
            Model::Gompertz => {
                // 渐近值 a；y = a/e 时 x = xc；最大斜率 a·k/e 出现在 xc
                let a = if ys[n - 1] >= ys[0] { hi } else { lo };
                let xc = crossing(a / std::f64::consts::E);
                let max_slope = (1..n)
                    .map(|i| (ys[i] - ys[i - 1]) / (xs[i] - xs[i - 1]).max(1e-12))
                    .fold(0.0, |m: f64, s| if s.abs() > m.abs() { s } else { m });
                let k = std::f64::consts::E * max_slope / a;
                vec![a, xc, if k.is_finite() && k != 0.0 { k } else { 1.0 }]
            }
            Model::Allometric2 => {
                // a 取略低于最小值，再对 ln(y − a) ~ ln x 做直线
                let a = lo - 0.05 * (hi - lo).max(1e-12);
                let (c, ln_b) = linreg(
                    xs.iter()
                        .zip(ys)
                        .filter(|(x, _)| **x > 0.0)
                        .map(|(x, y)| (x.ln(), (y - a).ln())),
                );
                if c.is_finite() {
                    vec![a, ln_b.exp(), c]
                } else {
                    vec![a, 1.0, 1.0]
                }
            }
            Model::Power1 => {
                let span = (xs[n - 1] - xs[0]).max(1e-12);
                let xc = xs[0] - 0.05 * span;
                let (p, ln_a) = linreg(
                    xs.iter()
                        .zip(ys)
                        .filter(|(_, y)| **y > 0.0)
                        .map(|(x, y)| ((x - xc).ln(), y.ln())),
                );
                if p.is_finite() {
                    vec![ln_a.exp(), xc, p]
                } else {
                    vec![1.0, xc, 1.0]
                }
            }
            Model::Log3P1 => {
                let span = (xs[n - 1] - xs[0]).max(1e-12);
                let c = if xs[0] > 0.0 { 0.0 } else { 0.05 * span - xs[0] };
                let (slope, intercept) = linreg(xs.iter().zip(ys).map(|(x, y)| ((x + c).ln(), *y)));
                vec![intercept, -slope, c]
            }
            Model::ExpDec1 => {
                // 先取略低于最小值的 y0，再对 ln(y − y0) 做直线拟合
                let y0 = lo - 0.01 * (hi - lo).max(1e-12);
                let pts: Vec<(f64, f64)> = xs.iter().zip(ys).map(|(x, y)| (*x, (y - y0).ln())).collect();
                let (sx, sy) = pts.iter().fold((0.0, 0.0), |(a, b), (x, y)| (a + x, b + y));
                let (mx, my) = (sx / n as f64, sy / n as f64);
                let sxx: f64 = pts.iter().map(|(x, _)| (x - mx).powi(2)).sum();
                let sxy: f64 = pts.iter().map(|(x, y)| (x - mx) * (y - my)).sum();
                let slope = sxy / sxx;
                let t1 = if slope < 0.0 {
                    -1.0 / slope
                } else {
                    (xs[n - 1] - xs[0]) / 3.0
                };
                vec![y0, (my - slope * mx).exp(), t1]
            }
            Model::Boltzmann | Model::Logistic => {
                let a1 = mean(&ys[..k]);
                let a2 = mean(&ys[n - k..]);
                let x0 = crossing(0.5 * (a1 + a2));
                if self == Model::Boltzmann {
                    vec![a1, a2, x0, (xs[n - 1] - xs[0]) / 10.0]
                } else {
                    vec![a1, a2, x0.abs().max(1e-12), 2.0]
                }
            }
            Model::Allometric1 => {
                let pts: Vec<(f64, f64)> = xs
                    .iter()
                    .zip(ys)
                    .filter(|(x, y)| **x > 0.0 && **y > 0.0)
                    .map(|(x, y)| (x.ln(), y.ln()))
                    .collect();
                if pts.len() < 2 {
                    return vec![1.0, 1.0];
                }
                let m = pts.len() as f64;
                let (mx, my) = (
                    pts.iter().map(|p| p.0).sum::<f64>() / m,
                    pts.iter().map(|p| p.1).sum::<f64>() / m,
                );
                let sxx: f64 = pts.iter().map(|p| (p.0 - mx).powi(2)).sum();
                let b = pts.iter().map(|p| (p.0 - mx) * (p.1 - my)).sum::<f64>() / sxx;
                vec![(my - b * mx).exp(), b]
            }
            Model::Langmuir | Model::MichaelisMenten => {
                // 线性化：x/y = 1/(qm·KL) + x/qm（Hanes–Woolf）
                let (slope, intercept) = linreg(
                    xs.iter()
                        .zip(ys)
                        .filter(|(x, y)| **x > 0.0 && **y > 0.0)
                        .map(|(x, y)| (*x, x / y)),
                );
                let (top, half) = if slope > 0.0 && intercept > 0.0 {
                    (1.0 / slope, intercept / slope)
                } else {
                    (hi, crossing(0.5 * hi).abs().max(1e-12))
                };
                if self == Model::Langmuir {
                    vec![top, 1.0 / half]
                } else {
                    vec![top, half]
                }
            }
            Model::Freundlich => {
                // ln q = ln KF + (1/n)·ln C
                let (slope, intercept) = linreg(
                    xs.iter()
                        .zip(ys)
                        .filter(|(x, y)| **x > 0.0 && **y > 0.0)
                        .map(|(x, y)| (x.ln(), y.ln())),
                );
                let n = if slope > 0.0 { 1.0 / slope } else { 2.0 };
                vec![intercept.exp(), n]
            }
            Model::PseudoFirstOrder => {
                let qe = hi * 1.02;
                // ln(qe − qt) = ln qe − k1·t
                let (slope, _) = linreg(
                    xs.iter()
                        .zip(ys)
                        .filter(|(_, y)| **y < qe)
                        .map(|(x, y)| (*x, (qe - y).ln())),
                );
                let k1 = if slope < 0.0 {
                    -slope
                } else {
                    3.0 / (xs[n - 1] - xs[0]).max(1e-12)
                };
                vec![qe, k1]
            }
            Model::PseudoSecondOrder => {
                // t/qt = 1/(k2·qe²) + t/qe
                let (slope, intercept) = linreg(
                    xs.iter()
                        .zip(ys)
                        .filter(|(x, y)| **x > 0.0 && **y > 0.0)
                        .map(|(x, y)| (*x, x / y)),
                );
                if slope > 0.0 && intercept > 0.0 {
                    vec![1.0 / slope, slope * slope / intercept]
                } else {
                    vec![hi, 1.0 / (hi * (xs[n - 1] - xs[0]).max(1e-12))]
                }
            }
        }
    }
}

use std::f64::consts::{LN_2 as LN2, PI};

/// 归一化 Voigt 线形（面积 1）：σ 为 Gauss 标准差，γ 为 Lorentz 半宽半高；
/// V = Re w(z)/(σ√(2π))，z = (x + iγ)/(σ√2)，与 `scipy.special.voigt_profile` 相同。
pub fn voigt_profile(x: f64, sigma: f64, gamma: f64) -> f64 {
    if gamma <= 0.0 {
        return (-x * x / (2.0 * sigma * sigma)).exp() / (sigma * (2.0 * PI).sqrt());
    }
    let s = sigma * 2f64.sqrt();
    faddeeva_re(x / s, gamma / s) / (sigma * (2.0 * PI).sqrt())
}

/// Faddeeva 函数 w(z) 的实部（Im z > 0），Weideman (1994) 有理展开，N = 64。
fn faddeeva_re(x: f64, y: f64) -> f64 {
    use std::sync::OnceLock;
    const N: usize = 64;
    static COEF: OnceLock<Vec<f64>> = OnceLock::new();
    let a = COEF.get_or_init(|| {
        let m = 2 * N;
        let m2 = 2 * m;
        let l = (N as f64 / 2f64.sqrt()).sqrt();
        // f(k) 对 k = −M+1 … M−1，前面补一个 0，共 2M 个点；fftshift 后做 FFT
        let mut f = vec![0.0; m2];
        for (idx, k) in (-(m as i64) + 1..m as i64).enumerate() {
            let t = l * (k as f64 * PI / m as f64 / 2.0).tan();
            f[idx + 1] = (-t * t).exp() * (l * l + t * t);
        }
        let shifted: Vec<f64> = f[m..].iter().chain(&f[..m]).copied().collect();
        let (re, _) = crate::fft::dft(&shifted, &vec![0.0; m2]);
        (1..=N).map(|j| re[j] / m2 as f64).collect()
    });
    let l = (N as f64 / 2f64.sqrt()).sqrt();
    // Z = (L + iz)/(L − iz)，iz = −y + ix
    let (nr, ni) = (l - y, x);
    let (dr, di) = (l + y, -x);
    let den = dr * dr + di * di;
    let (zr, zi) = ((nr * dr + ni * di) / den, (ni * dr - nr * di) / den);
    // 多项式 p(Z) = Σ a_j Z^j（霍纳法）
    let (mut pr, mut pi) = (0.0, 0.0);
    for c in a.iter().rev() {
        let (r, i) = (pr * zr - pi * zi + c, pr * zi + pi * zr);
        pr = r;
        pi = i;
    }
    // w = 2p/(L − iz)² + 1/(√π (L − iz))
    let (ur, ui) = (dr, di);
    let u2r = ur * ur - ui * ui;
    let u2i = 2.0 * ur * ui;
    let u2 = u2r * u2r + u2i * u2i;
    let term1 = 2.0 * (pr * u2r + pi * u2i) / u2;
    let term2 = ur / (PI.sqrt() * (ur * ur + ui * ui));
    term1 + term2
}

/// 最小二乘直线 (斜率, 截距)；点数不足或 x 不变时返回 (NaN, NaN)。
fn linreg(points: impl Iterator<Item = (f64, f64)>) -> (f64, f64) {
    let pts: Vec<(f64, f64)> = points.filter(|(x, y)| x.is_finite() && y.is_finite()).collect();
    if pts.len() < 2 {
        return (f64::NAN, f64::NAN);
    }
    let m = pts.len() as f64;
    let (mx, my) = (
        pts.iter().map(|p| p.0).sum::<f64>() / m,
        pts.iter().map(|p| p.1).sum::<f64>() / m,
    );
    let sxx: f64 = pts.iter().map(|p| (p.0 - mx).powi(2)).sum();
    if sxx <= 0.0 {
        return (f64::NAN, f64::NAN);
    }
    let slope = pts.iter().map(|p| (p.0 - mx) * (p.1 - my)).sum::<f64>() / sxx;
    (slope, my - slope * mx)
}

/// 拟合结果。
#[derive(Clone, Debug, PartialEq)]
pub struct NlFit {
    pub model: Model,
    pub params: Vec<f64>,
    pub se: Vec<f64>,
    pub rss: f64,
    pub r2: f64,
    pub adj_r2: f64,
    /// RSS / (n − p)
    pub reduced_chi2: f64,
    pub n: usize,
    pub iterations: usize,
    pub converged: bool,
    /// 被固定的参数（标准误记为 0，不计入自由度）。
    pub fixed: Vec<bool>,
    /// 结果停在下限或上限上的参数。
    pub at_bound: Vec<bool>,
    /// 残差自由度 n − 自由参数数。
    pub dof: usize,
    /// 参数协方差（固定参数的行列为 0）。
    pub cov: Vec<Vec<f64>>,
    /// 用到的线性约束（用户写法）。
    pub constraints: Vec<String>,
    pub x_min: f64,
    pub x_max: f64,
}

impl NlFit {
    pub fn eval(&self, x: f64) -> f64 {
        self.model.eval(x, &self.params)
    }

    /// 在 x 处的置信带、预测带半宽（Δ 法：var(ŷ) = gᵀ·Cov·g，g 为中心差分梯度）。
    pub fn band(&self, x: f64, level: f64) -> (f64, f64) {
        let k = self.params.len();
        let g: Vec<f64> = (0..k)
            .map(|j| {
                if self.fixed[j] {
                    return 0.0;
                }
                let h = 1e-6 * self.params[j].abs().max(1e-6);
                let (mut up, mut down) = (self.params.clone(), self.params.clone());
                up[j] += h;
                down[j] -= h;
                (self.model.eval(x, &up) - self.model.eval(x, &down)) / (2.0 * h)
            })
            .collect();
        let var: f64 = (0..k)
            .map(|i| (0..k).map(|j| g[i] * self.cov[i][j] * g[j]).sum::<f64>())
            .sum();
        crate::analysis::band_half_widths(var, self.reduced_chi2, self.dof, level)
    }

    /// 各参数的 t 值和双侧 p 值（固定参数为 NaN）。
    pub fn t_tests(&self) -> Vec<(f64, f64)> {
        crate::analysis::coef_t_tests(&self.params, &self.se, self.dof)
            .into_iter()
            .zip(&self.fixed)
            .map(|(tp, f)| if *f { (f64::NAN, f64::NAN) } else { tp })
            .collect()
    }
}

fn rss(model: Model, xs: &[f64], ys: &[f64], p: &[f64]) -> f64 {
    let s: f64 = xs.iter().zip(ys).map(|(x, y)| (y - model.eval(*x, p)).powi(2)).sum();
    if s.is_finite() {
        s
    } else {
        f64::INFINITY
    }
}

/// 中心差分雅可比 ∂f/∂p，只对 `free` 中的参数求导，n × free.len()。
fn jacobian(model: Model, xs: &[f64], p: &[f64], free: &[usize]) -> Vec<Vec<f64>> {
    let mut j = vec![vec![0.0; free.len()]; xs.len()];
    for (col, &k) in free.iter().enumerate() {
        let h = 1e-6 * p[k].abs().max(1e-6);
        let (mut up, mut down) = (p.to_vec(), p.to_vec());
        up[k] += h;
        down[k] -= h;
        for (i, x) in xs.iter().enumerate() {
            j[i][col] = (model.eval(*x, &up) - model.eval(*x, &down)) / (2.0 * h);
        }
    }
    j
}

/// 单个参数的限制：固定值（优先）、下限、上限。
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ParamLimits {
    pub fixed: Option<f64>,
    pub lower: Option<f64>,
    pub upper: Option<f64>,
}

impl ParamLimits {
    pub fn fixed(v: f64) -> Self {
        Self {
            fixed: Some(v),
            ..Default::default()
        }
    }

    fn clamp(&self, v: f64) -> f64 {
        let v = self.lower.map_or(v, |lo| v.max(lo));
        self.upper.map_or(v, |hi| v.min(hi))
    }
}

/// 线性约束的关系。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rel {
    Eq,
    Le,
    Ge,
}

/// 参数间的线性约束 Σ coef[k]·p[k]（关系）rhs。
#[derive(Clone, Debug, PartialEq)]
pub struct LinearConstraint {
    pub coef: Vec<f64>,
    pub rel: Rel,
    pub rhs: f64,
    /// 用户写法，用于报告。
    pub text: String,
}

/// 解析参数约束：每行（或用分号隔开）一条，如 `A1 + A2 = 1`、`t2 >= t1 + 3`、`2*w <= 5`。
/// 两边都必须是参数的线性式（参数只能乘、除常数）。
pub fn parse_constraints(model: Model, text: &str) -> Result<Vec<LinearConstraint>, String> {
    let names = model.params();
    let mut out = vec![];
    for line in text.split(['\n', ';', '；']).map(str::trim).filter(|l| !l.is_empty()) {
        let line_norm = line.replace("≤", "<=").replace("≥", ">=").replace("==", "=");
        let (lhs, rel, rhs) = if let Some((a, b)) = line_norm.split_once("<=") {
            (a, Rel::Le, b)
        } else if let Some((a, b)) = line_norm.split_once(">=") {
            (a, Rel::Ge, b)
        } else if let Some((a, b)) = line_norm.split_once('<') {
            (a, Rel::Le, b)
        } else if let Some((a, b)) = line_norm.split_once('>') {
            (a, Rel::Ge, b)
        } else if let Some((a, b)) = line_norm.split_once('=') {
            (a, Rel::Eq, b)
        } else {
            return Err(format!("约束“{line}”缺少 =、<= 或 >="));
        };
        let (cl, kl) = linear_expr(lhs, names).map_err(|e| format!("约束“{line}”：{e}"))?;
        let (cr, kr) = linear_expr(rhs, names).map_err(|e| format!("约束“{line}”：{e}"))?;
        let coef: Vec<f64> = cl.iter().zip(&cr).map(|(a, b)| a - b).collect();
        if coef.iter().all(|c| *c == 0.0) {
            return Err(format!("约束“{line}”里没有参数"));
        }
        out.push(LinearConstraint {
            coef,
            rel,
            rhs: kr - kl,
            text: line.to_string(),
        });
    }
    Ok(out)
}

/// 线性式：返回 (各参数系数, 常数项)。
fn linear_expr(src: &str, names: &[&str]) -> Result<(Vec<f64>, f64), String> {
    // 词法：数字、参数名、+ − * /
    let cs: Vec<char> = src.chars().collect();
    let mut toks: Vec<String> = vec![];
    let mut i = 0;
    while i < cs.len() {
        let c = cs[i];
        if c.is_whitespace() {
            i += 1;
        } else if c.is_ascii_digit() || c == '.' {
            let start = i;
            while i < cs.len()
                && (cs[i].is_ascii_digit() || cs[i] == '.' || ((cs[i] == 'e' || cs[i] == 'E') && i + 1 < cs.len()))
            {
                if (cs[i] == 'e' || cs[i] == 'E') && i + 1 < cs.len() && (cs[i + 1] == '-' || cs[i + 1] == '+') {
                    i += 1;
                }
                i += 1;
            }
            toks.push(cs[start..i].iter().collect());
        } else if c.is_alphabetic() || c == '_' {
            let start = i;
            while i < cs.len() && (cs[i].is_alphanumeric() || cs[i] == '_') {
                i += 1;
            }
            toks.push(cs[start..i].iter().collect());
        } else if "+-*/−".contains(c) {
            toks.push(if c == '−' { "-".into() } else { c.to_string() });
            i += 1;
        } else {
            return Err(format!("无法识别的字符“{c}”"));
        }
    }
    if toks.is_empty() {
        return Err("缺少表达式".into());
    }
    let mut coef = vec![0.0; names.len()];
    let mut constant = 0.0;
    let mut pos = 0;
    let mut sign = 1.0;
    loop {
        // 一项：可带正负号，由数字和至多一个参数相乘除组成
        while pos < toks.len() && (toks[pos] == "+" || toks[pos] == "-") {
            if toks[pos] == "-" {
                sign = -sign;
            }
            pos += 1;
        }
        let mut value = sign;
        let mut param: Option<usize> = None;
        let mut op = "*".to_string();
        loop {
            let t = toks.get(pos).ok_or("表达式不完整")?;
            if let Ok(v) = t.parse::<f64>() {
                if op == "*" {
                    value *= v
                } else {
                    value /= v
                }
            } else if let Some(k) = names.iter().position(|n| n == t) {
                if param.is_some() || op == "/" {
                    return Err("约束必须是参数的线性式".into());
                }
                param = Some(k);
            } else {
                return Err(format!("“{t}”不是参数名，可用 {}", names.join("、")));
            }
            pos += 1;
            match toks.get(pos).map(String::as_str) {
                Some(o @ ("*" | "/")) => {
                    op = o.to_string();
                    pos += 1;
                }
                _ => break,
            }
        }
        match param {
            Some(k) => coef[k] += value,
            None => constant += value,
        }
        sign = 1.0;
        match toks.get(pos).map(String::as_str) {
            None => break,
            Some("+") | Some("-") => {}
            Some(t) => return Err(format!("“{t}”的位置不对")),
        }
    }
    Ok((coef, constant))
}

/// 矩阵行张成空间的正交补（每个基向量长 `dim`）；`rows` 为空时返回单位基。
fn null_space(rows: &[Vec<f64>], dim: usize) -> Vec<Vec<f64>> {
    let dot = |a: &[f64], b: &[f64]| a.iter().zip(b).map(|(x, y)| x * y).sum::<f64>();
    let mut basis: Vec<Vec<f64>> = vec![];
    let orth = |v: &[f64], basis: &mut Vec<Vec<f64>>| {
        let mut w = v.to_vec();
        for _ in 0..2 {
            for b in basis.iter() {
                let d = dot(&w, b);
                w.iter_mut().zip(b).for_each(|(x, y)| *x -= d * y);
            }
        }
        let norm = dot(&w, &w).sqrt();
        let scale = dot(v, v).sqrt().max(1.0);
        if norm > 1e-10 * scale {
            basis.push(w.iter().map(|x| x / norm).collect());
            true
        } else {
            false
        }
    };
    for r in rows {
        orth(r, &mut basis);
    }
    let rank = basis.len();
    let mut out = vec![];
    for j in 0..dim {
        let mut e = vec![0.0; dim];
        e[j] = 1.0;
        if orth(&e, &mut basis) {
            out.push(basis.last().unwrap().clone());
        }
        if out.len() + rank == dim {
            break;
        }
    }
    out
}

/// 拟合 `model`；`initial` 为空时自动估计初值。`fixed` 为空或逐个参数给出固定值（`Some`）。
/// 非有限的点会被跳过。
pub fn fit(
    model: Model,
    xs: &[f64],
    ys: &[f64],
    initial: Option<&[f64]>,
    fixed: &[Option<f64>],
) -> Result<NlFit, String> {
    let limits: Vec<ParamLimits> = fixed
        .iter()
        .map(|f| ParamLimits {
            fixed: *f,
            ..Default::default()
        })
        .collect();
    fit_with(model, xs, ys, initial, &limits)
}

/// 带参数限制的拟合：固定参数不参与迭代；上下界用投影法——每步把试探参数截到界内，
/// 初值也先截到界内。标准误按停止点的雅可比计算（与 SciPy `curve_fit` 带 bounds 时相同口径）。
pub fn fit_with(
    model: Model,
    xs: &[f64],
    ys: &[f64],
    initial: Option<&[f64]>,
    limits: &[ParamLimits],
) -> Result<NlFit, String> {
    fit_constrained(model, xs, ys, initial, limits, &[])
}

/// 带参数限制与线性约束的拟合。等式约束和“贴边且下降方向朝外”的不等式在零空间里求步长，
/// 其余不等式与上下界按步长截断，保持每一步可行；标准误按停止点的约束零空间计算，
/// 自由度扣除约束个数（与代入消元后的拟合相同）。
pub fn fit_constrained(
    model: Model,
    xs: &[f64],
    ys: &[f64],
    initial: Option<&[f64]>,
    limits: &[ParamLimits],
    constraints: &[LinearConstraint],
) -> Result<NlFit, String> {
    let fixed: Vec<Option<f64>> = limits.iter().map(|l| l.fixed).collect();
    let fixed = fixed.as_slice();
    let limit = |k: usize| limits.get(k).copied().unwrap_or_default();
    for (k, l) in limits.iter().enumerate() {
        if let (Some(lo), Some(hi)) = (l.lower, l.upper) {
            if lo >= hi || lo.is_nan() || hi.is_nan() {
                return Err(format!(
                    "参数 {} 的下限必须小于上限",
                    model.params().get(k).unwrap_or(&"?")
                ));
            }
        }
        if l.lower.is_some_and(|v| !v.is_finite()) || l.upper.is_some_and(|v| !v.is_finite()) {
            return Err("上下限必须是有限数".into());
        }
    }
    let mut pts: Vec<(f64, f64)> = xs
        .iter()
        .zip(ys)
        .filter(|(x, y)| x.is_finite() && y.is_finite())
        .map(|(x, y)| (*x, *y))
        .collect();
    pts.sort_by(|a, b| a.0.total_cmp(&b.0));
    let (xs, ys): (Vec<f64>, Vec<f64>) = pts.into_iter().unzip();
    let all = model.params().len();
    let is_fixed = |k: usize| fixed.get(k).is_some_and(|v| v.is_some());
    let free: Vec<usize> = (0..all).filter(|k| !is_fixed(*k)).collect();
    let m = free.len();
    if m == 0 {
        return Err("至少要有一个参数不固定".into());
    }
    if fixed.iter().flatten().any(|v| !v.is_finite()) {
        return Err("固定参数的值必须是有限数".into());
    }
    let n = xs.len();
    // 约束只作用在自由参数上：固定参数的贡献移到右边
    let rows: Vec<(Vec<f64>, Rel, f64)> = constraints
        .iter()
        .map(|c| {
            let a: Vec<f64> = free.iter().map(|&k| c.coef.get(k).copied().unwrap_or(0.0)).collect();
            let b = c.rhs
                - (0..all)
                    .filter(|&k| is_fixed(k))
                    .map(|k| c.coef.get(k).copied().unwrap_or(0.0) * fixed[k].unwrap())
                    .sum::<f64>();
            (a, c.rel, b)
        })
        .collect();
    if rows.iter().any(|(a, _, _)| a.iter().all(|v| *v == 0.0)) {
        return Err("约束只涉及固定参数，请去掉或改为固定值".into());
    }
    let n_eq = null_space(
        &rows
            .iter()
            .filter(|r| r.1 == Rel::Eq)
            .map(|r| r.0.clone())
            .collect::<Vec<_>>(),
        m,
    )
    .len();
    if n <= n_eq {
        return Err(format!(
            "{} 有 {} 个参数，至少需要 {} 个有效数据点",
            model.name(),
            n_eq,
            n_eq + 1
        ));
    }
    let mut p = match initial {
        Some(v) => v.to_vec(),
        None => model.guess(&xs, &ys),
    };
    for (k, v) in fixed.iter().enumerate() {
        if let Some(v) = v {
            p[k] = *v;
        }
    }
    for &k in &free {
        p[k] = limit(k).clamp(p[k]);
    }
    let lhs = |p: &[f64], a: &[f64]| free.iter().zip(a).map(|(&k, c)| c * p[k]).sum::<f64>();
    let violation = |p: &[f64], (a, rel, b): &(Vec<f64>, Rel, f64)| {
        let d = lhs(p, a) - b;
        match rel {
            Rel::Eq => d,
            Rel::Le => d.max(0.0),
            Rel::Ge => d.min(0.0),
        }
    };
    if !rows.is_empty() {
        // 初值投影到可行域：交替投影到各约束（超平面/半空间）并截到上下界
        let tol = |p: &[f64]| 1e-9 * free.iter().map(|&k| p[k].abs()).fold(1.0, f64::max);
        for _ in 0..500 {
            for row in &rows {
                let d = violation(&p, row);
                let norm2: f64 = row.0.iter().map(|c| c * c).sum();
                if d != 0.0 {
                    for (&k, c) in free.iter().zip(&row.0) {
                        p[k] -= d * c / norm2;
                    }
                }
            }
            for &k in &free {
                p[k] = limit(k).clamp(p[k]);
            }
            if rows.iter().all(|r| violation(&p, r).abs() <= tol(&p)) {
                break;
            }
        }
        if rows
            .iter()
            .any(|r| violation(&p, r).abs() > 1e-6 * free.iter().map(|&k| p[k].abs()).fold(1.0, f64::max))
        {
            return Err("约束之间（或与上下限、固定值）互相矛盾，找不到满足全部约束的参数".into());
        }
    }
    let active_tol = |p: &[f64]| 1e-8 * free.iter().map(|&k| p[k].abs()).fold(1.0, f64::max);
    let mut cost = rss(model, &xs, &ys, &p);
    if !cost.is_finite() {
        return Err("初值处函数值无效，请检查数据是否适合该函数".into());
    }
    let mut lambda = 1e-3;
    let mut converged = false;
    let mut iterations = 0;
    while iterations < 500 {
        iterations += 1;
        let j = jacobian(model, &xs, &p, &free);
        let r: Vec<f64> = xs.iter().zip(&ys).map(|(x, y)| y - model.eval(*x, &p)).collect();
        let mut a = vec![vec![0.0; m]; m];
        let mut g = vec![0.0; m];
        for i in 0..n {
            for u in 0..m {
                g[u] += j[i][u] * r[i];
                for v in 0..m {
                    a[u][v] += j[i][u] * j[i][v];
                }
            }
        }
        // 活动集：贴着边界、且下降方向还往界外推的参数，本步不动，只解其余参数
        let open: Vec<usize> = (0..m)
            .filter(|&u| {
                let (k, l) = (free[u], limit(free[u]));
                !(l.lower == Some(p[k]) && g[u] < 0.0 || l.upper == Some(p[k]) && g[u] > 0.0)
            })
            .collect();
        let mut stepped = false;
        if !rows.is_empty() {
            // 约束：等式与“贴边且朝外”的不等式进入零空间，其余不等式与上下界截断步长。
            // 截断到 0 的那条约束（或上下界）说明它在挡路：加入活动集后重新求步。
            let mut active: Vec<usize> = (0..rows.len())
                .filter(|&i| {
                    let r = &rows[i];
                    let ag: f64 = r.0.iter().zip(&g).map(|(a, g)| a * g).sum();
                    let tight = (lhs(&p, &r.0) - r.2).abs() <= active_tol(&p);
                    match r.1 {
                        Rel::Eq => true,
                        Rel::Le => tight && ag > 0.0,
                        Rel::Ge => tight && ag < 0.0,
                    }
                })
                .collect();
            let mut held: Vec<usize> = vec![];
            'solve: loop {
                let open_now: Vec<usize> = open.iter().copied().filter(|u| !held.contains(u)).collect();
                let c_open: Vec<Vec<f64>> = active
                    .iter()
                    .map(|&i| open_now.iter().map(|&u| rows[i].0[u]).collect())
                    .collect();
                let basis = null_space(&c_open, open_now.len());
                if basis.is_empty() {
                    break 'solve;
                }
                let r = basis.len();
                // H = Nᵀ A N，b = Nᵀ g
                let an: Vec<Vec<f64>> = open_now
                    .iter()
                    .map(|&u| {
                        (0..r)
                            .map(|q| open_now.iter().enumerate().map(|(j, &v)| a[u][v] * basis[q][j]).sum())
                            .collect()
                    })
                    .collect();
                let h0: Vec<Vec<f64>> = (0..r)
                    .map(|q1| {
                        (0..r)
                            .map(|q2| (0..open_now.len()).map(|i| basis[q1][i] * an[i][q2]).sum())
                            .collect()
                    })
                    .collect();
                let nb: Vec<f64> = (0..r)
                    .map(|q| open_now.iter().enumerate().map(|(i, &u)| basis[q][i] * g[u]).sum())
                    .collect();
                while lambda < 1e16 {
                    let mut h = h0.clone();
                    for (q, row) in h.iter_mut().enumerate() {
                        row[q] += lambda * row[q].max(1e-12);
                    }
                    let Some(inv) = mat_inv(&h) else {
                        lambda *= 10.0;
                        continue;
                    };
                    let z: Vec<f64> = (0..r).map(|q| (0..r).map(|q2| inv[q][q2] * nb[q2]).sum()).collect();
                    let mut delta = vec![0.0; m];
                    for (i, &u) in open_now.iter().enumerate() {
                        delta[u] = (0..r).map(|q| basis[q][i] * z[q]).sum();
                    }
                    // 不越过不在活动集里的不等式和上下界；记下最先挡住的那个
                    let mut alpha: f64 = 1.0;
                    let mut blocker: Option<(bool, usize)> = None;
                    for (i, row) in rows.iter().enumerate() {
                        if active.contains(&i) {
                            continue;
                        }
                        let ad: f64 = row.0.iter().zip(&delta).map(|(a, d)| a * d).sum();
                        let slack = row.2 - lhs(&p, &row.0);
                        let limit_here = match row.1 {
                            Rel::Le if ad > 0.0 => slack.max(0.0) / ad,
                            Rel::Ge if ad < 0.0 => slack.min(0.0) / ad,
                            _ => continue,
                        };
                        if limit_here < alpha {
                            alpha = limit_here;
                            blocker = Some((true, i));
                        }
                    }
                    for (u, &k) in free.iter().enumerate() {
                        let l = limit(k);
                        let limit_here = match (l.lower, l.upper) {
                            (_, Some(hi)) if delta[u] > 0.0 => (hi - p[k]).max(0.0) / delta[u],
                            (Some(lo), _) if delta[u] < 0.0 => (lo - p[k]).min(0.0) / delta[u],
                            _ => continue,
                        };
                        if limit_here < alpha {
                            alpha = limit_here;
                            blocker = Some((false, u));
                        }
                    }
                    if alpha <= 1e-12 {
                        match blocker {
                            Some((true, i)) => active.push(i),
                            Some((false, u)) => held.push(u),
                            None => {}
                        }
                        continue 'solve;
                    }
                    let mut trial = p.clone();
                    for (u, &k) in free.iter().enumerate() {
                        trial[k] = limit(k).clamp(p[k] + alpha * delta[u]);
                    }
                    let new_cost = rss(model, &xs, &ys, &trial);
                    if new_cost < cost {
                        let small_step = free
                            .iter()
                            .all(|&k| (trial[k] - p[k]).abs() <= 1e-10 * p[k].abs().max(1e-10));
                        let small_gain = cost - new_cost <= 1e-14 * cost.max(1e-300);
                        p = trial;
                        cost = new_cost;
                        lambda = (lambda / 10.0).max(1e-12);
                        stepped = true;
                        // 被约束截短的一步不算收敛：下一步会把挡路的约束纳入活动集
                        converged = small_gain || (small_step && alpha >= 1.0);
                        break 'solve;
                    }
                    lambda *= 10.0;
                }
                break 'solve;
            }
            if !stepped {
                converged = true;
            }
            if converged {
                break;
            }
            continue;
        }
        while lambda < 1e16 && !open.is_empty() {
            let mut damped: Vec<Vec<f64>> = open.iter().map(|&u| open.iter().map(|&v| a[u][v]).collect()).collect();
            for (i, row) in damped.iter_mut().enumerate() {
                row[i] += lambda * a[open[i]][open[i]].max(1e-12);
            }
            let Some(inv) = mat_inv(&damped) else {
                lambda *= 10.0;
                continue;
            };
            let mut delta = vec![0.0; m];
            for (i, &u) in open.iter().enumerate() {
                delta[u] = open.iter().enumerate().map(|(j, &v)| inv[i][j] * g[v]).sum();
            }
            let mut trial = p.clone();
            for (d, &k) in delta.iter().zip(&free) {
                trial[k] = limit(k).clamp(trial[k] + d);
            }
            let new_cost = rss(model, &xs, &ys, &trial);
            if new_cost < cost {
                let small_step = free
                    .iter()
                    .all(|&k| (trial[k] - p[k]).abs() <= 1e-10 * p[k].abs().max(1e-10));
                let small_gain = cost - new_cost <= 1e-14 * cost.max(1e-300);
                p = trial;
                cost = new_cost;
                lambda = (lambda / 10.0).max(1e-12);
                stepped = true;
                converged = small_step || small_gain;
                break;
            }
            lambda *= 10.0;
        }
        if !stepped {
            // 任何阻尼都无法再降低残差：已到极小点
            converged = true;
        }
        if converged {
            break;
        }
    }
    let j = jacobian(model, &xs, &p, &free);
    let mut a = vec![vec![0.0; m]; m];
    for row in &j {
        for u in 0..m {
            for v in 0..m {
                a[u][v] += row[u] * row[v];
            }
        }
    }
    // 约束：在停止点的等式与贴边不等式零空间里算协方差，自由度扣除约束个数
    let tight: Vec<Vec<f64>> = rows
        .iter()
        .filter(|r| r.1 == Rel::Eq || (lhs(&p, &r.0) - r.2).abs() <= active_tol(&p))
        .map(|r| r.0.clone())
        .collect();
    let basis = null_space(&tight, m);
    let dof_n = n - basis.len();
    let dof = dof_n as f64;
    let reduced = cost / dof;
    let mut se = vec![0.0; all];
    let mut cov = vec![vec![0.0; all]; all];
    if !rows.is_empty() {
        let r = basis.len();
        let h: Vec<Vec<f64>> = (0..r)
            .map(|q1| {
                (0..r)
                    .map(|q2| {
                        (0..m)
                            .map(|u| (0..m).map(|v| basis[q1][u] * a[u][v] * basis[q2][v]).sum::<f64>())
                            .sum()
                    })
                    .collect()
            })
            .collect();
        match mat_inv(&h) {
            Some(inv) => {
                for (u, &k) in free.iter().enumerate() {
                    for (v, &l) in free.iter().enumerate() {
                        cov[k][l] = (0..r)
                            .map(|q1| (0..r).map(|q2| basis[q1][u] * inv[q1][q2] * basis[q2][v]).sum::<f64>())
                            .sum::<f64>()
                            * reduced;
                    }
                    se[k] = cov[k][k].max(0.0).sqrt();
                }
            }
            None => {
                for &k in &free {
                    se[k] = f64::NAN;
                }
            }
        }
    } else {
        match mat_inv(&a) {
            Some(inv) => {
                for (u, &k) in free.iter().enumerate() {
                    se[k] = (inv[u][u] * reduced).max(0.0).sqrt();
                    for (v, &l) in free.iter().enumerate() {
                        cov[k][l] = inv[u][v] * reduced;
                    }
                }
            }
            None => {
                for &k in &free {
                    se[k] = f64::NAN;
                }
            }
        }
    }
    let at_bound: Vec<bool> = (0..all)
        .map(|k| {
            let l = limit(k);
            !is_fixed(k) && (l.lower == Some(p[k]) || l.upper == Some(p[k]))
        })
        .collect();
    let mean = ys.iter().sum::<f64>() / n as f64;
    let tss: f64 = ys.iter().map(|y| (y - mean).powi(2)).sum();
    let r2 = if tss > 0.0 { 1.0 - cost / tss } else { f64::NAN };
    Ok(NlFit {
        model,
        params: p,
        se,
        rss: cost,
        r2,
        adj_r2: 1.0 - (1.0 - r2) * (n as f64 - 1.0) / dof,
        reduced_chi2: reduced,
        n,
        iterations,
        converged,
        fixed: (0..all).map(is_fixed).collect(),
        at_bound,
        dof: dof_n,
        cov,
        constraints: constraints.iter().map(|c| c.text.clone()).collect(),
        x_min: xs[0],
        x_max: xs[n - 1],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn noise(i: usize) -> f64 {
        let i = i as f64;
        (1.7 * i).sin() * 0.6 + (0.37 * i * i).cos() * 0.4
    }

    fn data(model: Model, from: f64, to: f64, truth: &[f64], amp: f64) -> (Vec<f64>, Vec<f64>) {
        let xs: Vec<f64> = (0..41).map(|i| from + (to - from) * i as f64 / 40.0).collect();
        let ys = xs
            .iter()
            .enumerate()
            .map(|(i, x)| model.eval(*x, truth) + amp * noise(i))
            .collect();
        (xs, ys)
    }

    /// (函数, X 起点, X 终点, 真值, 噪声幅度, 参考参数, 参考标准误, 参考 RSS)
    type Case<'a> = (Model, f64, f64, &'a [f64], f64, &'a [f64], &'a [f64], f64);

    fn close(got: f64, want: f64, rel: f64) {
        assert!((got - want).abs() <= rel * want.abs().max(1e-9), "{got} vs {want}");
    }

    #[test]
    fn added_builtin_functions_converge_to_scipy_curve_fit() {
        // 参考值：scipy.optimize.curve_fit（SciPy 1.17.1，p0 取真值）；这里用自动初值
        let cases: [Case; 13] = [
            (
                Model::Voigt,
                0.0,
                10.0,
                &[0.5, 5.0, 8.0, 1.2, 0.8],
                0.03,
                &[
                    0.502987389692305,
                    4.9994087742597,
                    7.98905574485161,
                    1.212719629821541,
                    0.790032894133966,
                ],
                &[
                    0.006207464376162809,
                    0.0022433759402392768,
                    0.07572102365167718,
                    0.025374812557617577,
                    0.03208450677549791,
                ],
                0.01072010972019333,
            ),
            (
                Model::PsdVoigt1,
                0.0,
                10.0,
                &[0.3, 4.5, 6.0, 1.5, 0.4],
                0.03,
                &[
                    0.3009534141880661,
                    4.499172087721348,
                    6.021337487717337,
                    1.4959212882150772,
                    0.4103596138827938,
                ],
                &[
                    0.005587425411888454,
                    0.0023442627654601854,
                    0.06509666834675183,
                    0.0071941672597614845,
                    0.024730834968163404,
                ],
                0.010758003239766188,
            ),
            (
                Model::LogNormal,
                0.2,
                10.0,
                &[0.2, 3.0, 0.4, 10.0],
                0.03,
                &[
                    0.20191298275817007,
                    3.000019482867363,
                    0.40007606359111575,
                    10.005413179919492,
                ],
                &[
                    0.004615721739269777,
                    0.0036396418783872795,
                    0.0012550820665772344,
                    0.038148829178791185,
                ],
                0.010908537439269561,
            ),
            (
                Model::ExpDec2,
                0.0,
                10.0,
                &[0.5, 4.0, 0.8, 2.0, 4.0],
                0.02,
                &[
                    0.4937960013006856,
                    4.01551121614994,
                    0.7974110921387975,
                    2.002545767816202,
                    4.046387326181638,
                ],
                &[
                    0.019735941188441294,
                    0.06279795003108511,
                    0.012293474333771807,
                    0.048313578591315055,
                    0.1865327482325991,
                ],
                0.00466318079317071,
            ),
            (
                Model::ExpDec3,
                0.0,
                20.0,
                &[0.2, 3.0, 0.5, 2.0, 2.5, 1.5, 9.0],
                0.01,
                &[
                    0.1559698425034319,
                    3.0833964991005396,
                    0.5123124336847743,
                    2.1498771149859217,
                    2.7657495595984742,
                    1.3151952150003738,
                    10.713480339384695,
                ],
                &[
                    0.04963758730807458,
                    0.05390947691399579,
                    0.008111631918091625,
                    0.16100156150396602,
                    0.22126250699496686,
                    0.15655332356926485,
                    1.9724746003585718,
                ],
                0.001095159831052157,
            ),
            (
                Model::ExpGro1,
                0.0,
                5.0,
                &[1.0, 0.5, 1.5],
                0.05,
                &[1.0036677407365988, 0.5015354665130808, 1.5017889664546158],
                &[0.013547135634189463, 0.005649400152885607, 0.005030990698243008],
                0.02974095858852069,
            ),
            (
                Model::ExpAssoc,
                0.0,
                10.0,
                &[0.5, 3.0, 0.7, 2.0, 4.0],
                0.02,
                &[
                    0.5121172136536161,
                    2.982998017016836,
                    0.7024269163748263,
                    1.9988689165175193,
                    3.9534873311285605,
                ],
                &[
                    0.010467899233794022,
                    0.049455331835194315,
                    0.012954363639320485,
                    0.03733528622375226,
                    0.15270653325894512,
                ],
                0.0046612916151662015,
            ),
            (
                Model::DoseResp,
                -3.0,
                3.0,
                &[1.0, 9.0, 0.3, 1.2],
                0.05,
                &[
                    1.0074086343720887,
                    8.998715240450007,
                    0.29953788120668495,
                    1.203679123501137,
                ],
                &[
                    0.007757719078944026,
                    0.009197507525541885,
                    0.0025321707989441815,
                    0.00744571058800078,
                ],
                0.029938095567038366,
            ),
            (
                Model::Hill,
                0.1,
                10.0,
                &[1.0, 8.0, 2.5, 2.2],
                0.05,
                &[
                    1.0189718518346893,
                    7.98959662778498,
                    2.500333429637884,
                    2.217168150144736,
                ],
                &[
                    0.01797666302307224,
                    0.017193247431952313,
                    0.009893466179771038,
                    0.020345891202554135,
                ],
                0.02953457394949384,
            ),
            (
                Model::Gompertz,
                0.0,
                10.0,
                &[6.0, 3.0, 0.9],
                0.05,
                &[6.001514782537589, 2.997167483936787, 0.9002960676563785],
                &[0.008223440236067856, 0.005550430110585135, 0.006264636574684849],
                0.03056073914187006,
            ),
            (
                Model::Allometric2,
                0.5,
                10.0,
                &[1.0, 0.8, 1.6],
                0.05,
                &[1.0063233380609855, 0.8003853338039472, 1.5996609617553645],
                &[0.01309187654574319, 0.0038208430944108016, 0.0020292713044496834],
                0.029939208157724446,
            ),
            (
                Model::Power1,
                1.0,
                10.0,
                &[2.0, 0.5, 0.7],
                0.02,
                &[1.9994608903244961, 0.4967582288374258, 0.7000192675871021],
                &[0.0071972334099453935, 0.007057862408701341, 0.0015076629812576963],
                0.004784781759805734,
            ),
            (
                Model::Log3P1,
                0.5,
                10.0,
                &[3.0, 1.2, 0.5],
                0.02,
                &[2.997125235443011, 1.1988014402935538, 0.4915073331562746],
                &[0.020361190029973577, 0.00866411045169326, 0.02210971956403735],
                0.0047744246482707055,
            ),
        ];
        for (model, from, to, truth, amp, params, se, rss) in cases {
            let (xs, ys) = data(model, from, to, truth, amp);
            let f = fit(model, &xs, &ys, None, &[]).unwrap_or_else(|e| panic!("{}: {e}", model.name()));
            assert!(f.converged, "{}", model.name());
            for (g, w) in f.params.iter().zip(params) {
                close(*g, *w, 1e-5);
            }
            for (g, w) in f.se.iter().zip(se) {
                close(*g, *w, 1e-3);
            }
            close(f.rss, rss, 1e-7);
        }
    }

    #[test]
    fn voigt_profile_matches_scipy() {
        // scipy.special.voigt_profile(x, sigma, gamma)
        for (x, sigma, gamma, want) in [
            (0.0, 1.0, 0.5, 0.27895547038929436),
            (1.3, 0.4, 0.2, 0.05594361959542218),
            (-2.5, 0.7, 1.1, 0.05536634494387373),
            (0.05, 2.0, 0.01, 0.19861602219299082),
            (8.0, 0.5, 0.3, 0.0015077335480456885),
            (0.3, 0.1, 3.0, 0.10494211022325665),
        ] {
            let got = voigt_profile(x, sigma, gamma);
            assert!(
                (got - want).abs() <= 1e-12 * want.abs().max(1e-300) + 1e-15,
                "{x} {sigma} {gamma}: {got} vs {want}"
            );
        }
    }

    #[test]
    fn auto_initial_values_converge_to_scipy_curve_fit() {
        // 参考值：scipy.optimize.curve_fit（SciPy 1.17.1，p0 取真值）
        let cases: [Case; 6] = [
            (
                Model::Gauss,
                0.0,
                10.0,
                &[1.0, 4.2, 1.5, 6.0],
                0.05,
                &[
                    1.0048915270703311,
                    4.200181394465829,
                    1.4950191190305495,
                    5.99169753813478,
                ],
                &[
                    0.005688688212796727,
                    0.0040927900045844,
                    0.009008364506171394,
                    0.03631036450750082,
                ],
                0.030054878704556094,
            ),
            (
                Model::Lorentz,
                0.0,
                10.0,
                &[0.5, 5.5, 1.2, 4.0],
                0.05,
                &[
                    0.5016682833136684,
                    5.504488807447752,
                    1.214493978640595,
                    4.030696352633713,
                ],
                &[
                    0.006520497789558827,
                    0.00587031756445813,
                    0.020346990941884486,
                    0.057702833830455866,
                ],
                0.029429245533392035,
            ),
            (
                Model::ExpDec1,
                0.0,
                8.0,
                &[0.3, 5.0, 1.7],
                0.05,
                &[0.302890515973314, 5.011675158701781, 1.6976676601354435],
                &[0.008905602297414812, 0.017482735005634128, 0.013692057251406079],
                0.02997825443344604,
            ),
            (
                Model::Boltzmann,
                -5.0,
                5.0,
                &[1.0, 9.0, 0.7, 0.8],
                0.1,
                &[
                    1.0155540784588701,
                    8.9923798599876,
                    0.6965204833395424,
                    0.7950285733396781,
                ],
                &[
                    0.017004313951120623,
                    0.022821143265601744,
                    0.010819965534474214,
                    0.009807508460495412,
                ],
                0.11945876884193626,
            ),
            (
                Model::Logistic,
                0.1,
                10.0,
                &[0.5, 8.0, 3.0, 2.5],
                0.1,
                &[
                    0.52815741501568,
                    7.982528548772376,
                    2.997221577226676,
                    2.527521360279785,
                ],
                &[
                    0.030917244118693735,
                    0.035357795335685976,
                    0.018914504733669135,
                    0.041687185127689624,
                ],
                0.11906754992937188,
            ),
            (
                Model::Allometric1,
                0.5,
                10.0,
                &[2.0, 0.75],
                0.05,
                &[2.0059992752066784, 0.7486201278337822],
                &[0.005861355038033662, 0.0014942798002535167],
                0.030083812104675662,
            ),
        ];
        for (model, from, to, truth, amp, params, se, rss) in cases {
            let (xs, ys) = data(model, from, to, truth, amp);
            let f = fit(model, &xs, &ys, None, &[]).unwrap_or_else(|e| panic!("{}: {e}", model.name()));
            assert!(f.converged, "{}", model.name());
            for (g, w) in f.params.iter().zip(params) {
                close(*g, *w, 1e-6);
            }
            for (g, w) in f.se.iter().zip(se) {
                close(*g, *w, 1e-4);
            }
            close(f.rss, rss, 1e-8);
            assert!(f.r2 > 0.99, "{} r2 {}", model.name(), f.r2);
        }
    }

    #[test]
    fn adsorption_and_kinetics_models_match_scipy_from_automatic_starts() {
        // 参考值：scipy.optimize.curve_fit（SciPy 1.17.1，p0 取真值）
        let cases: [Case; 5] = [
            (
                Model::Langmuir,
                0.5,
                200.0,
                &[45.0, 0.03],
                0.5,
                &[44.97634745234103, 0.030183478087933875],
                &[0.1522995827714246, 0.0003879665171647043],
                3.0473473280763974,
            ),
            (
                Model::Freundlich,
                0.5,
                200.0,
                &[6.0, 2.5],
                0.3,
                &[6.032052384932666, 2.506398449728879],
                &[0.03619766608481412, 0.00787496285918195],
                1.0853667639848175,
            ),
            (
                Model::PseudoFirstOrder,
                0.0,
                300.0,
                &[32.0, 0.02],
                0.4,
                &[32.01391575943531, 0.020055996739953872],
                &[0.05556218278874523, 0.00014929094003738433],
                1.9534535473442134,
            ),
            (
                Model::PseudoSecondOrder,
                0.0,
                300.0,
                &[35.0, 0.001],
                0.4,
                &[35.00119576266527, 0.0010050576101159605],
                &[0.08793642812072777, 1.5166045776723856e-05],
                1.9524552085415017,
            ),
            (
                Model::MichaelisMenten,
                0.1,
                20.0,
                &[12.0, 2.5],
                0.1,
                &[11.99732714274374, 2.4887804519721763],
                &[0.025429546861139248, 0.023329314484644446],
                0.12179256523091592,
            ),
        ];
        for (model, from, to, truth, amp, params, se, rss) in cases {
            let (xs, ys) = data(model, from, to, truth, amp);
            let f = fit(model, &xs, &ys, None, &[]).unwrap_or_else(|e| panic!("{}: {e}", model.name()));
            assert!(f.converged, "{}", model.name());
            for (g, w) in f.params.iter().zip(params) {
                close(*g, *w, 1e-6);
            }
            for (g, w) in f.se.iter().zip(se) {
                close(*g, *w, 1e-4);
            }
            close(f.rss, rss, 1e-8);
        }
        assert_eq!(Model::ALL.len(), 24);
        assert!(
            Model::ALL.iter().all(|m| m.params().len() <= MAX_PARAMS),
            "the dialog holds up to MAX_PARAMS parameters"
        );
        assert!(Model::ALL.iter().all(|m| Model::CATEGORIES.contains(&m.category())));
    }

    #[test]
    fn nonlinear_bands_and_t_tests_match_delta_method() {
        // 参考：scipy curve_fit 的 pcov 与解析梯度的 Δ 法，t 分布 95%（SciPy 1.17.1）
        let (xs, ys) = data(Model::ExpDec1, 0.0, 8.0, &[0.3, 5.0, 1.7], 0.05);
        let f = fit(Model::ExpDec1, &xs, &ys, None, &[]).unwrap();
        let at = [0.0, 1.0, 2.5, 8.0];
        let conf = [
            0.036930037359520045,
            0.016145292629534957,
            0.014471897661722667,
            0.015446935846694449,
        ];
        let pred = [
            0.06780030389556413,
            0.05910773234568062,
            0.058672731067223025,
            0.05892080596875095,
        ];
        for i in 0..4 {
            let (c, p) = f.band(at[i], 0.95);
            close(c, conf[i], 1e-5);
            close(p, pred[i], 1e-6);
        }
        let t = [34.01123351318792, 286.66425189541997, 123.98923371068953];
        let pv = [4.606615447919501e-30, 5.510180515572003e-65, 3.602688045999483e-51];
        for (k, (tv, p)) in f.t_tests().into_iter().enumerate() {
            close(tv, t[k], 1e-4);
            close(p, pv[k], 1e-2);
        }
        // 固定参数没有 t 检验
        let held = fit(Model::ExpDec1, &xs, &ys, None, &[None, None, Some(1.7)]).unwrap();
        assert!(held.t_tests()[2].0.is_nan());
    }

    #[test]
    fn linear_constraints_match_scipy_substitution() {
        // 参考值：SciPy 1.17.1 curve_fit 对代入约束后的模型（等式消元；生效的不等式按等式代入）
        let (xs, ys) = data(Model::ExpDec2, 0.0, 10.0, &[0.5, 4.0, 0.8, 2.0, 4.0], 0.02);
        let cases: [(&str, &[f64], &[f64], f64); 3] = [
            (
                "A1 + A2 = 6.5",
                &[
                    -0.008869506971709452,
                    4.599838952542216,
                    0.9059992722022512,
                    1.9001610474577841,
                    9.03039219176094,
                ],
                &[
                    0.020674970458570655,
                    0.031508185530406595,
                    0.013377031324750327,
                    0.031508185530406595,
                    0.333684070913209,
                ],
                0.020842198807211432,
            ),
            (
                "t2 >= t1 + 3.5",
                &[
                    0.46666306034400296,
                    4.094307934602258,
                    0.8113114174798253,
                    1.9476663075416343,
                    4.311311417479825,
                ],
                &[
                    0.004800016887384544,
                    0.016949920857287096,
                    0.0063353188402990365,
                    0.020885120441479183,
                    0.0063353188402990365,
                ],
                0.0048867135867287775,
            ),
            (
                "t2 - t1 >= 1",
                &[
                    0.4937960013006856,
                    4.01551121614994,
                    0.7974110921387975,
                    2.002545767816202,
                    4.046387326181638,
                ],
                &[
                    0.019735941188441294,
                    0.06279795003108511,
                    0.012293474333771807,
                    0.048313578591315055,
                    0.1865327482325991,
                ],
                0.00466318079317071,
            ),
        ];
        for (text, params, se, rss) in cases {
            let constraints = parse_constraints(Model::ExpDec2, text).unwrap();
            let f = fit_constrained(Model::ExpDec2, &xs, &ys, None, &[], &constraints).unwrap();
            assert!(f.converged, "{text}");
            for (g, w) in f.params.iter().zip(params) {
                close(*g, *w, 1e-5);
            }
            for (g, w) in f.se.iter().zip(se) {
                close(*g, *w, 1e-3);
            }
            close(f.rss, rss, 1e-7);
            assert_eq!(f.constraints, [text]);
        }
    }

    #[test]
    fn constraint_parsing_and_errors() {
        let c = parse_constraints(Model::Gauss, "2*w - xc/2 <= 3 + y0；A = 1").unwrap();
        assert_eq!(c.len(), 2);
        assert_eq!(
            (c[0].coef.clone(), c[0].rel, c[0].rhs),
            (vec![-1.0, -0.5, 2.0, 0.0], Rel::Le, 3.0)
        );
        assert_eq!(
            (c[1].coef.clone(), c[1].rel, c[1].rhs),
            (vec![0.0, 0.0, 0.0, 1.0], Rel::Eq, 1.0)
        );
        for (text, msg) in [
            ("w * xc = 1", "线性"),
            ("q = 1", "不是参数名"),
            ("w 1", "缺少"),
            ("3 = 3", "没有参数"),
        ] {
            let e = parse_constraints(Model::Gauss, text).unwrap_err();
            assert!(e.contains(msg), "{text}: {e}");
        }
        // 互相矛盾的约束
        let (xs, ys) = data(Model::Gauss, 0.0, 10.0, &[1.0, 4.2, 1.5, 6.0], 0.05);
        let bad = parse_constraints(Model::Gauss, "w >= 3; w <= 2").unwrap();
        assert!(fit_constrained(Model::Gauss, &xs, &ys, None, &[], &bad)
            .unwrap_err()
            .contains("矛盾"));
    }

    #[test]
    fn bounded_parameters_match_scipy_trf() {
        // 参考值：scipy curve_fit(bounds=…, method="trf")（SciPy 1.17.1）
        let lim = |lower: Option<f64>, upper: Option<f64>| ParamLimits {
            fixed: None,
            lower,
            upper,
        };
        let free = ParamLimits::default();
        type Bounded<'a> = (
            Model,
            f64,
            f64,
            &'a [f64],
            &'a [ParamLimits],
            &'a [f64],
            &'a [f64],
            f64,
            &'a [bool],
        );
        let cases: [Bounded; 3] = [
            (
                Model::Gauss,
                0.0,
                10.0,
                &[1.0, 4.2, 1.5, 6.0],
                &[lim(Some(1.02), None), free, lim(None, Some(1.3)), free],
                &[
                    1.0591747191658383,
                    4.200262070736825,
                    1.2999999999999998,
                    5.43529480038129,
                ],
                &[
                    0.021308377368655526,
                    0.014258322592542791,
                    0.0308399632343038,
                    0.12682856689340216,
                ],
                0.4565305024320042,
                &[false, false, true, false],
            ),
            (
                Model::ExpDec1,
                0.0,
                8.0,
                &[0.3, 5.0, 1.7],
                &[free, free, lim(None, Some(1.5))],
                &[0.40247361429382816, 5.096088116350162, 1.4999999999999998],
                &[0.021649870222356992, 0.049727736755135005, 0.031631938817463304],
                0.22313286158621473,
                &[false, false, true],
            ),
            (
                // 界不生效时与无约束结果相同
                Model::ExpDec1,
                0.0,
                8.0,
                &[0.3, 5.0, 1.7],
                &[
                    lim(Some(0.0), Some(1.0)),
                    lim(Some(0.0), Some(10.0)),
                    lim(Some(0.5), Some(5.0)),
                ],
                &[0.3028905161777613, 5.011675158968168, 1.6976676595997724],
                &[0.008905602142936982, 0.017482735121955775, 0.013692056871435564],
                0.029978254433445914,
                &[false, false, false],
            ),
        ];
        for (model, from, to, truth, limits, params, se, rss, at_bound) in cases {
            let amp = 0.05;
            let (xs, ys) = data(model, from, to, truth, amp);
            let f = fit_with(model, &xs, &ys, None, limits).unwrap();
            assert!(f.converged);
            for (g, w) in f.params.iter().zip(params) {
                close(*g, *w, 1e-6);
            }
            for (g, w) in f.se.iter().zip(se) {
                close(*g, *w, 1e-4);
            }
            close(f.rss, rss, 1e-8);
            assert_eq!(f.at_bound, at_bound);
        }
        assert!(fit_with(Model::Gauss, &[0.0; 10], &[0.0; 10], None, &[lim(Some(2.0), Some(1.0))]).is_err());
    }

    #[test]
    fn fixed_parameters_match_scipy_with_the_parameter_held() {
        // 参考值：curve_fit 对去掉固定参数后的函数拟合（SciPy 1.17.1）
        let (xs, ys) = data(Model::Gauss, 0.0, 10.0, &[1.0, 4.2, 1.5, 6.0], 0.05);
        let f = fit(Model::Gauss, &xs, &ys, None, &[Some(1.0), None, None, None]).unwrap();
        assert_eq!(f.fixed, [true, false, false, false]);
        assert_eq!((f.params[0], f.se[0]), (1.0, 0.0));
        let want = [4.200180465074677, 1.4982909982647807, 6.0112335219205635];
        let se = [0.004078720331065471, 0.008157440762355455, 0.02834340061698604];
        for k in 0..3 {
            close(f.params[k + 1], want[k], 1e-6);
            close(f.se[k + 1], se[k], 1e-4);
        }
        let (xs, ys) = data(Model::ExpDec1, 0.0, 8.0, &[0.3, 5.0, 1.7], 0.05);
        let f = fit(Model::ExpDec1, &xs, &ys, None, &[Some(0.3)]).unwrap();
        close(f.params[1], 5.01261358915415, 1e-6);
        close(f.params[2], 1.7010939158349043, 1e-6);
        close(f.se[2], 0.008672307488815372, 1e-4);
        assert!(fit(Model::Allometric1, &xs, &ys, None, &[Some(1.0), Some(1.0)]).is_err());
        assert!(fit(Model::ExpDec1, &xs, &ys, None, &[Some(f64::NAN)]).is_err());
    }

    #[test]
    fn too_few_points_and_bad_initial_values_are_reported() {
        assert!(fit(Model::Gauss, &[1.0, 2.0, 3.0], &[1.0, 2.0, 1.0], None, &[])
            .unwrap_err()
            .contains("至少需要 5 个"));
        let (xs, ys) = data(Model::ExpDec1, 0.0, 8.0, &[0.3, 5.0, 1.7], 0.05);
        assert!(fit(Model::ExpDec1, &xs, &ys, Some(&[0.0, 1.0, 0.0]), &[]).is_err());
        // 无序输入先按 X 排序
        let mut rev = xs.clone();
        rev.reverse();
        let mut rys = ys.clone();
        rys.reverse();
        let a = fit(Model::ExpDec1, &xs, &ys, None, &[]).unwrap();
        let b = fit(Model::ExpDec1, &rev, &rys, None, &[]).unwrap();
        close(a.params[2], b.params[2], 1e-9);
        close(
            a.eval(2.0),
            a.params[0] + a.params[1] * (-2.0 / a.params[2]).exp(),
            1e-12,
        );
    }
}
