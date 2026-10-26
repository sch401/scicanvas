//! 内置模型定义、公式与 Voigt 线形。
use std::f64::consts::{LN_2 as LN2, PI};

/// 内置拟合函数。峰、指数、S 形和幂函数的公式与参数名与 Origin 相同；
/// 吸附与酶动力学函数使用文献通用写法（x 为浓度或时间）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
    Rational1,
    Rational12,
    Rational22,
    Rational33,
}

/// 内置函数的最多参数个数（ExpDec3）。
pub const MAX_PARAMS: usize = 7;

impl Model {
    pub const ALL: [Model; 28] = [
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
        Model::Rational1,
        Model::Rational12,
        Model::Rational22,
        Model::Rational33,
    ];

    /// 对话框里的类别顺序。
    pub const CATEGORIES: [&'static str; 8] = [
        "峰函数",
        "指数",
        "S 形",
        "幂函数与对数",
        "吸附等温线",
        "吸附动力学",
        "酶动力学",
        "有理函数",
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
            Model::Rational1 => "Rational1",
            Model::Rational12 => "Rational12",
            Model::Rational22 => "Rational22",
            Model::Rational33 => "Rational33",
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
            Model::Rational1 | Model::Rational12 | Model::Rational22 | Model::Rational33 => "有理函数",
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
            Model::Rational1 => "y = (a0 + a1·x)/(1 + b1·x)",
            Model::Rational12 => "y = (a0 + a1·x)/(1 + b1·x + b2·x²)",
            Model::Rational22 => "y = (a0 + a1·x + a2·x²)/(1 + b1·x + b2·x²)",
            Model::Rational33 => "y = (a0 + a1·x + a2·x² + a3·x³)/(1 + b1·x + b2·x² + b3·x³)",
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
            Model::Rational1 => &["a0", "a1", "b1"],
            Model::Rational12 => &["a0", "a1", "b1", "b2"],
            Model::Rational22 => &["a0", "a1", "a2", "b1", "b2"],
            Model::Rational33 => &["a0", "a1", "a2", "a3", "b1", "b2", "b3"],
        }
    }

    // Rational12/22/33 的数字明确表示分子/分母阶数，Rational1 保留既有名称。
    pub(super) fn rational_degrees(self) -> Option<(usize, usize)> {
        match self {
            Self::Rational1 => Some((1, 1)),
            Self::Rational12 => Some((1, 2)),
            Self::Rational22 => Some((2, 2)),
            Self::Rational33 => Some((3, 3)),
            _ => None,
        }
    }

    pub(super) fn rational_denominator(self, x: f64, p: &[f64]) -> (f64, f64) {
        let (n, _) = self.rational_degrees().unwrap();
        let (d, magnitude) = p[n + 1..]
            .iter()
            .rev()
            .fold((0.0, 0.0), |(d, s), b| (d * x + b, s * x.abs() + b.abs()));
        (d * x + 1.0, magnitude * x.abs() + 1.0)
    }

    /// 在分母驻点处分段检查，识别一次穿越、偶数个极点及偶重根。
    pub(super) fn crosses_pole(self, from: f64, to: f64, p: &[f64]) -> bool {
        let Some((n, d)) = self.rational_degrees() else {
            return false;
        };
        let (lo, hi) = (from.min(to), from.max(to));
        let mut probes = vec![lo, hi];
        let b = &p[n + 1..];
        let (a, c) = (if d == 3 { 3.0 * b[2] } else { 0.0 }, b[0]);
        let middle = if d >= 2 { 2.0 * b[1] } else { 0.0 };
        if a == 0.0 {
            if middle != 0.0 {
                probes.push(-c / middle);
            }
        } else {
            let discriminant = middle * middle - 4.0 * a * c;
            if discriminant >= 0.0 {
                // 避免根公式在相消时丢失较小的驻点。
                let q = -0.5 * (middle + discriminant.sqrt().copysign(middle));
                probes.push(q / a);
                if q != 0.0 {
                    probes.push(c / q);
                }
            }
        }
        let sign = self.rational_denominator(lo, p).0.is_sign_positive();
        probes.into_iter().filter(|x| *x >= lo && *x <= hi).any(|x| {
            let (value, magnitude) = self.rational_denominator(x, p);
            !value.is_finite() || value.abs() <= 32.0 * f64::EPSILON * magnitude || value.is_sign_positive() != sign
        })
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
            Model::Rational1 | Model::Rational12 | Model::Rational22 | Model::Rational33 => {
                let (n, _) = self.rational_degrees().unwrap();
                let numerator = p[..=n].iter().rev().fold(0.0, |sum, a| sum * x + a);
                let (denominator, magnitude) = self.rational_denominator(x, p);
                if !denominator.is_finite() || denominator.abs() <= 32.0 * f64::EPSILON * magnitude {
                    f64::NAN
                } else {
                    numerator / denominator
                }
            }
        }
    }
}

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
