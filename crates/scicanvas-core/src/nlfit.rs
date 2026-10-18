//! 非线性拟合：Levenberg–Marquardt（中心差分雅可比），内置 Origin 同名函数与自动初值。
//! 标准误按 cov = (JᵀJ)⁻¹ · RSS / (n − p)，与 SciPy `curve_fit`（默认 absolute_sigma=False）一致。

use crate::analysis::mat_inv;

/// 内置拟合函数，公式与参数名与 Origin 相同。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Model {
    Gauss,
    Lorentz,
    ExpDec1,
    Boltzmann,
    Logistic,
    Allometric1,
}

impl Model {
    pub const ALL: [Model; 6] = [
        Model::Gauss,
        Model::Lorentz,
        Model::ExpDec1,
        Model::Boltzmann,
        Model::Logistic,
        Model::Allometric1,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Model::Gauss => "Gauss",
            Model::Lorentz => "Lorentz",
            Model::ExpDec1 => "ExpDec1",
            Model::Boltzmann => "Boltzmann",
            Model::Logistic => "Logistic",
            Model::Allometric1 => "Allometric1",
        }
    }

    pub fn category(self) -> &'static str {
        match self {
            Model::Gauss | Model::Lorentz => "峰函数",
            Model::ExpDec1 => "指数",
            Model::Boltzmann | Model::Logistic => "S 形",
            Model::Allometric1 => "幂函数",
        }
    }

    pub fn formula(self) -> &'static str {
        match self {
            Model::Gauss => "y = y0 + A/(w·√(π/2))·exp(−2((x − xc)/w)²)",
            Model::Lorentz => "y = y0 + (2A/π)·w/(4(x − xc)² + w²)",
            Model::ExpDec1 => "y = y0 + A1·exp(−x/t1)",
            Model::Boltzmann => "y = A2 + (A1 − A2)/(1 + exp((x − x0)/dx))",
            Model::Logistic => "y = A2 + (A1 − A2)/(1 + (x/x0)^p)",
            Model::Allometric1 => "y = a·x^b",
        }
    }

    pub fn params(self) -> &'static [&'static str] {
        match self {
            Model::Gauss | Model::Lorentz => &["y0", "xc", "w", "A"],
            Model::ExpDec1 => &["y0", "A1", "t1"],
            Model::Boltzmann => &["A1", "A2", "x0", "dx"],
            Model::Logistic => &["A1", "A2", "x0", "p"],
            Model::Allometric1 => &["a", "b"],
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
            Model::ExpDec1 => p[0] + p[1] * (-x / p[2]).exp(),
            Model::Boltzmann => p[1] + (p[0] - p[1]) / (1.0 + ((x - p[2]) / p[3]).exp()),
            Model::Logistic => p[1] + (p[0] - p[1]) / (1.0 + (x / p[2]).powf(p[3])),
            Model::Allometric1 => p[0] * x.powf(p[1]),
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
        }
    }
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
    pub x_min: f64,
    pub x_max: f64,
}

impl NlFit {
    pub fn eval(&self, x: f64) -> f64 {
        self.model.eval(x, &self.params)
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

/// 拟合 `model`；`initial` 为空时自动估计初值。`fixed` 为空或逐个参数给出固定值（`Some`）。
/// 非有限的点会被跳过。
pub fn fit(
    model: Model,
    xs: &[f64],
    ys: &[f64],
    initial: Option<&[f64]>,
    fixed: &[Option<f64>],
) -> Result<NlFit, String> {
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
    if n <= m {
        return Err(format!(
            "{} 有 {} 个参数，至少需要 {} 个有效数据点",
            model.name(),
            m,
            m + 1
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
        let mut stepped = false;
        while lambda < 1e16 {
            let mut damped = a.clone();
            for (u, row) in damped.iter_mut().enumerate() {
                row[u] += lambda * a[u][u].max(1e-12);
            }
            let Some(inv) = mat_inv(&damped) else {
                lambda *= 10.0;
                continue;
            };
            let delta: Vec<f64> = (0..m).map(|u| (0..m).map(|v| inv[u][v] * g[v]).sum()).collect();
            let mut trial = p.clone();
            for (d, &k) in delta.iter().zip(&free) {
                trial[k] += d;
            }
            let new_cost = rss(model, &xs, &ys, &trial);
            if new_cost < cost {
                let small_step = delta
                    .iter()
                    .zip(&free)
                    .all(|(d, &k)| d.abs() <= 1e-10 * p[k].abs().max(1e-10));
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
    let dof = (n - m) as f64;
    let reduced = cost / dof;
    let mut se = vec![0.0; all];
    match mat_inv(&a) {
        Some(inv) => {
            for (u, &k) in free.iter().enumerate() {
                se[k] = (inv[u][u] * reduced).max(0.0).sqrt();
            }
        }
        None => {
            for &k in &free {
                se[k] = f64::NAN;
            }
        }
    }
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
