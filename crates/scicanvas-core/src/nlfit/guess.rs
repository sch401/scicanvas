//! 自动初值按峰型、指数、S型、幂/对数及吸附/动力学计算。
use super::Model;
use std::f64::consts::{LN_2 as LN2, PI};

impl Model {
    /// 由数据估计初值（数据已按 X 排序）。
    pub(super) fn guess(self, xs: &[f64], ys: &[f64]) -> Vec<f64> {
        let (lo, hi) = ys
            .iter()
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), y| (a.min(*y), b.max(*y)));
        match self {
            Model::Gauss | Model::Lorentz | Model::Voigt | Model::PsdVoigt1 | Model::LogNormal => {
                self.guess_peak(xs, ys, lo)
            }
            Model::ExpDec2 | Model::ExpDec3 | Model::ExpGro1 | Model::ExpAssoc | Model::ExpDec1 => {
                self.guess_exponential(xs, ys, lo, hi)
            }
            Model::DoseResp | Model::Hill | Model::Gompertz | Model::Boltzmann | Model::Logistic => {
                self.guess_sigmoid(xs, ys, lo, hi)
            }
            Model::Allometric2 | Model::Power1 | Model::Log3P1 | Model::Allometric1 => self.guess_power(xs, ys, lo, hi),
            Model::Langmuir
            | Model::MichaelisMenten
            | Model::Freundlich
            | Model::PseudoFirstOrder
            | Model::PseudoSecondOrder => self.guess_adsorption(xs, ys, hi),
        }
    }

    fn guess_peak(self, xs: &[f64], ys: &[f64], lo: f64) -> Vec<f64> {
        let n = xs.len();
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
            _ => unreachable!("初值分派与模型类别不一致"),
        }
    }

    fn guess_exponential(self, xs: &[f64], ys: &[f64], lo: f64, hi: f64) -> Vec<f64> {
        let n = xs.len();
        let k = (n / 10).max(1);
        match self {
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
                let tc = (crossing(xs, ys, y0 + 0.632 * total) - xs[0])
                    .abs()
                    .max((xs[n - 1] - xs[0]) / n as f64);
                vec![y0, total / 2.0, tc / 2.0, total / 2.0, tc * 2.0]
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
            _ => unreachable!("初值分派与模型类别不一致"),
        }
    }

    fn guess_sigmoid(self, xs: &[f64], ys: &[f64], lo: f64, hi: f64) -> Vec<f64> {
        let n = xs.len();
        let k = (n / 10).max(1);
        match self {
            Model::DoseResp | Model::Hill => {
                let a1 = mean(&ys[..k]);
                let a2 = mean(&ys[n - k..]);
                let mid = crossing(xs, ys, 0.5 * (a1 + a2));
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
                let xc = crossing(xs, ys, a / std::f64::consts::E);
                let max_slope = (1..n)
                    .map(|i| (ys[i] - ys[i - 1]) / (xs[i] - xs[i - 1]).max(1e-12))
                    .fold(0.0, |m: f64, s| if s.abs() > m.abs() { s } else { m });
                let k = std::f64::consts::E * max_slope / a;
                vec![a, xc, if k.is_finite() && k != 0.0 { k } else { 1.0 }]
            }
            Model::Boltzmann | Model::Logistic => {
                let a1 = mean(&ys[..k]);
                let a2 = mean(&ys[n - k..]);
                let x0 = crossing(xs, ys, 0.5 * (a1 + a2));
                if self == Model::Boltzmann {
                    vec![a1, a2, x0, (xs[n - 1] - xs[0]) / 10.0]
                } else {
                    vec![a1, a2, x0.abs().max(1e-12), 2.0]
                }
            }
            _ => unreachable!("初值分派与模型类别不一致"),
        }
    }

    fn guess_power(self, xs: &[f64], ys: &[f64], lo: f64, hi: f64) -> Vec<f64> {
        let n = xs.len();
        match self {
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
            _ => unreachable!("初值分派与模型类别不一致"),
        }
    }

    fn guess_adsorption(self, xs: &[f64], ys: &[f64], hi: f64) -> Vec<f64> {
        let n = xs.len();
        match self {
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
                    (hi, crossing(xs, ys, 0.5 * hi).abs().max(1e-12))
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
            _ => unreachable!("初值分派与模型类别不一致"),
        }
    }
}

fn mean(v: &[f64]) -> f64 {
    v.iter().sum::<f64>() / v.len() as f64
}

// y 穿过 level 的第一个位置，保留原始相邻点中点与回退规则。
fn crossing(xs: &[f64], ys: &[f64], level: f64) -> f64 {
    let n = xs.len();
    (1..n)
        .find(|&i| (ys[i - 1] - level) * (ys[i] - level) <= 0.0)
        .map_or(xs[n / 2], |i| 0.5 * (xs[i - 1] + xs[i]))
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

#[cfg(test)]
mod guess_regression {
    use super::*;

    fn cases() -> Vec<(Vec<f64>, Vec<f64>)> {
        vec![
            (
                (0..21).map(|i| 0.2 + i as f64 * 0.3).collect(),
                (0..21).map(|i| 0.5 + (i as f64 * 0.4).sin().powi(2) * 3.0).collect(),
            ),
            (
                (0..21).map(|i| 0.2 + i as f64 * 0.3).collect(),
                (0..21).map(|i| 4.0 * (-(i as f64) / 5.0).exp() + 0.3).collect(),
            ),
            (
                (0..21).map(|i| i as f64 - 10.0).collect(),
                (0..21).map(|i| -2.0 + 4.0 / (1.0 + (10.0 - i as f64).exp())).collect(),
            ),
            ((0..12).map(|i| i as f64 + 1.0).collect(), vec![2.0; 12]),
            (vec![1.0; 12], (0..12).map(|i| i as f64 - 6.0).collect()),
            (
                (0..12).rev().map(|i| i as f64 - 5.0).collect(),
                (0..12).map(|i| if i % 2 == 0 { -2.0 } else { 2.0 }).collect(),
            ),
        ]
    }

    // 从拆分前实现记录的24模型×6组位模式指纹，NaN仅统一载荷。
    const EXPECTED: [[u64; 24]; 6] = [
        [
            1809068557224687906,
            10409925152216008535,
            12261117266762714721,
            18282536008985730456,
            913143386980589530,
            18032784843914107257,
            11197309254294759162,
            12829785618908910367,
            6651460067417058110,
            14185445370448922111,
            8594267243490471436,
            11093654797996258640,
            15764210961303601595,
            11093654797996258640,
            14854166742777285932,
            16799842720691861928,
            6018548853228321768,
            5683677540843538751,
            13030441022275611115,
            8857706322918082563,
            14088241534204233655,
            466538145028221196,
            11277601043074810599,
            4258791863528811852,
        ],
        [
            16509969598976459433,
            9023606730310521944,
            11185039771899692239,
            12895768327627506594,
            15766956199985958288,
            1447337299075705062,
            6358380547821937628,
            15414562002591486665,
            1599666594062666236,
            10957541914640930208,
            10459775577402485907,
            12391273944603024543,
            4381574988868877317,
            12391273944603024543,
            17770316461631909427,
            9535686659786566370,
            6279483551428782287,
            16492948124306143246,
            12739786302138830095,
            13931387198922414187,
            15716969177565430251,
            6075436816090667402,
            1524251606603222598,
            11270752216326137579,
        ],
        [
            1489743162146276583,
            17991320177217476178,
            5574133014873504639,
            3265615432484847278,
            10125901659326244114,
            9330685429771447506,
            9318086596474403753,
            5008537636933831502,
            384840757028978183,
            12878886718184586350,
            18433592492334967248,
            5379153838541627472,
            5360692716888888886,
            5379153838541627472,
            511394327288833100,
            17751251091089990419,
            3309304565062229709,
            15478209596878474500,
            13590757266980890592,
            15606345770258131043,
            13222410519623463913,
            4562753405557291457,
            9813804341864469837,
            11760566179261376873,
        ],
        [
            5822495908892623311,
            3739492027129614923,
            13197091243521399341,
            7636930772192411202,
            906262003480025258,
            2881387689402023019,
            6595131265655511036,
            4740527049846522379,
            2881387689402023019,
            5133808664016003061,
            464223918907432965,
            606641420352889984,
            713882286989256669,
            606641420352889984,
            6974969666868219988,
            7112722159511595301,
            16472331657491579574,
            17750089421387473644,
            9401421623708268101,
            17601922987473480252,
            7112651790767389797,
            1878392774981710752,
            11046006906617430719,
            7086008424997879145,
        ],
        [
            9615368654820628688,
            9615368654820628688,
            8704199207986619941,
            8673572311588857736,
            14999836294770835551,
            5300957623905311686,
            8824326571257960079,
            5520602235382262790,
            5300957623905311686,
            522073167776207917,
            15402430321141525860,
            15402500689885731364,
            8927500208691731763,
            15402500689885731364,
            10821515197709161258,
            3808917137398389845,
            18001805679470341653,
            590033639614641783,
            17151760326278936821,
            8718559689676112172,
            3694006077645137024,
            16001875066847092527,
            15041376561084774078,
            8718559689676112172,
        ],
        [
            4800482603194968120,
            6459818301297124274,
            4245479121070048432,
            12982465148894018523,
            18186004430971431248,
            11502477317219913316,
            15905932132347462838,
            714103632691292971,
            2342537020451826896,
            13358984729750565413,
            7707582962422887058,
            10143029959041295107,
            16666744592932163466,
            10143029959041295107,
            12014902346599191209,
            7112722159511595301,
            10312886851219346236,
            6100807045443948399,
            6739274193213347607,
            11137908486531653343,
            7112651790767389797,
            4323680852615326264,
            16926092090572682786,
            7095574176161422595,
        ],
    ];

    #[test]
    fn initial_values_match_before_split() {
        for (case, (xs, ys)) in cases().iter().enumerate() {
            for (index, model) in Model::ALL.into_iter().enumerate() {
                let bits: Vec<u64> = model
                    .guess(xs, ys)
                    .into_iter()
                    .map(|v| if v.is_nan() { f64::NAN.to_bits() } else { v.to_bits() })
                    .collect();
                let hash = bits
                    .iter()
                    .flat_map(|v| v.to_le_bytes())
                    .fold(0xcbf29ce484222325u64, |hash, byte| {
                        (hash ^ u64::from(byte)).wrapping_mul(0x100000001b3)
                    });
                assert_eq!(hash, EXPECTED[case][index], "case {case}, {}", model.name());
            }
        }
    }
}
