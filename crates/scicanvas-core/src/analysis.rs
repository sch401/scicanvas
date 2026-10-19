//! 数值分析算法：多项式拟合、平滑、微分、积分、归一化、描述统计。
//! 只处理数组，不涉及工作簿和图形；把结果写回项目的逻辑在 ops.rs。

use crate::model::fmt_num;

/// 多项式最小二乘拟合的结果。
#[derive(Clone, Debug, PartialEq)]
pub struct PolyFit {
    /// 原始 x 下的系数，coef[k] 对应 x^k。
    pub coef: Vec<f64>,
    /// 各系数的标准误。
    pub se: Vec<f64>,
    pub r2: f64,
    pub adj_r2: f64,
    /// 残差平方和。
    pub rss: f64,
    pub n: usize,
    /// 自由度 n - (阶数 + 1)。
    pub dof: usize,
    pub x_min: f64,
    pub x_max: f64,
    /// 总平方和（对均值）。
    pub tss: f64,
    // 标准化后的系数，用来稳定地求值
    mx: f64,
    sx: f64,
    bt: Vec<f64>,
    /// 标准化系数的协方差 σ²·(TᵀT)⁻¹。
    cov_t: Vec<Vec<f64>>,
}

/// 回归的方差分析表：回归、残差、总计（对均值校正）。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FitAnova {
    pub df_reg: f64,
    pub ss_reg: f64,
    pub df_err: f64,
    pub ss_err: f64,
    pub df_tot: f64,
    pub ss_tot: f64,
    pub f: f64,
    pub p: f64,
}

impl FitAnova {
    pub fn ms_reg(&self) -> f64 {
        self.ss_reg / self.df_reg
    }
    pub fn ms_err(&self) -> f64 {
        self.ss_err / self.df_err
    }
}

/// 置信带与预测带的半宽：t 分位数 × √var(ŷ) 与 × √(σ² + var(ŷ))。
pub fn band_half_widths(var_fit: f64, sigma2: f64, dof: usize, level: f64) -> (f64, f64) {
    let t = crate::stats::t_ppf(1.0 - (1.0 - level) / 2.0, dof as f64);
    (t * var_fit.max(0.0).sqrt(), t * (sigma2 + var_fit.max(0.0)).sqrt())
}

/// 系数的 t 值与双侧 p 值。
pub fn coef_t_tests(values: &[f64], se: &[f64], dof: usize) -> Vec<(f64, f64)> {
    values
        .iter()
        .zip(se)
        .map(|(v, s)| {
            let t = v / s;
            // 双侧尾概率直接取 I_{ν/(ν+t²)}(ν/2, 1/2)，避免 1 − CDF 在极小 p 时相减抵消
            let df = dof as f64;
            (t, crate::stats::beta_inc(df / 2.0, 0.5, df / (df + t * t)))
        })
        .collect()
}

impl PolyFit {
    /// 在 x 处求拟合值。内部用标准化的 x 计算，比直接用原始系数更稳定。
    pub fn eval(&self, x: f64) -> f64 {
        let t = (x - self.mx) / self.sx;
        let mut f = 0.0;
        let mut tp = 1.0;
        for b in &self.bt {
            f += b * tp;
            tp *= t;
        }
        f
    }

    pub fn order(&self) -> usize {
        self.coef.len() - 1
    }

    /// 在 x 处的置信带、预测带半宽（`level` 如 0.95）。
    pub fn band(&self, x: f64, level: f64) -> (f64, f64) {
        let t = (x - self.mx) / self.sx;
        let g: Vec<f64> = (0..self.bt.len()).map(|j| t.powi(j as i32)).collect();
        let var: f64 = (0..g.len())
            .map(|i| (0..g.len()).map(|j| g[i] * self.cov_t[i][j] * g[j]).sum::<f64>())
            .sum();
        band_half_widths(var, self.rss / self.dof as f64, self.dof, level)
    }

    /// 方差分析：回归自由度为阶数，残差自由度 n − 阶数 − 1。
    pub fn anova(&self) -> FitAnova {
        let df_reg = self.order() as f64;
        let df_err = self.dof as f64;
        let ss_reg = self.tss - self.rss;
        let f = (ss_reg / df_reg) / (self.rss / df_err);
        FitAnova {
            df_reg,
            ss_reg,
            df_err,
            ss_err: self.rss,
            df_tot: (self.n - 1) as f64,
            ss_tot: self.tss,
            f,
            p: crate::stats::f_sf(f, df_reg, df_err),
        }
    }

    /// 各系数（原始 x 的幂次）的 t 值和双侧 p 值。
    pub fn t_tests(&self) -> Vec<(f64, f64)> {
        coef_t_tests(&self.coef, &self.se, self.dof)
    }

    /// 拟合多项式在 x 处的 k 阶导数（k = 0 即拟合值）。
    pub fn derivative_at(&self, x: f64, k: usize) -> f64 {
        let t = (x - self.mx) / self.sx;
        let mut f = 0.0;
        for (j, b) in self.bt.iter().enumerate().skip(k) {
            // d^k/dt^k t^j = j!/(j−k)! · t^(j−k)
            let falling: f64 = ((j - k + 1)..=j).map(|v| v as f64).product();
            f += b * falling * t.powi((j - k) as i32);
        }
        f / self.sx.powi(k as i32)
    }

    /// 线性拟合的 Pearson 相关系数；高阶拟合返回 None。
    pub fn pearson_r(&self) -> Option<f64> {
        (self.coef.len() == 2).then(|| self.coef[1].signum() * self.r2.max(0.0).sqrt())
    }

    /// 方程文字，例如 y = 1.2 + 0.5·x − 0.03·x²
    pub fn equation(&self) -> String {
        let mut s = format!("y = {}", fmt_num(self.coef[0]));
        for (k, c) in self.coef.iter().enumerate().skip(1) {
            let sign = if *c < 0.0 { "−" } else { "+" };
            s.push_str(&format!(" {} {}·{}", sign, fmt_num(c.abs()), power_of_x(k)));
        }
        s
    }
}

fn power_of_x(k: usize) -> String {
    const SUP: [&str; 10] = ["⁰", "¹", "²", "³", "⁴", "⁵", "⁶", "⁷", "⁸", "⁹"];
    if k == 1 {
        "x".to_string()
    } else {
        let digits: String = k.to_string().chars().map(|c| SUP[c as usize - '0' as usize]).collect();
        format!("x{}", digits)
    }
}

/// 高斯-约当求逆（部分主元）。奇异时返回 None。
pub fn mat_inv(a: &[Vec<f64>]) -> Option<Vec<Vec<f64>>> {
    let n = a.len();
    let mut m: Vec<Vec<f64>> = a
        .iter()
        .enumerate()
        .map(|(i, r)| {
            let mut row = r.clone();
            row.extend((0..n).map(|j| if i == j { 1.0 } else { 0.0 }));
            row
        })
        .collect();
    for c in 0..n {
        let p = (c..n).max_by(|&x, &y| m[x][c].abs().partial_cmp(&m[y][c].abs()).unwrap())?;
        if m[p][c].abs() < 1e-14 {
            return None;
        }
        m.swap(c, p);
        let d = m[c][c];
        for v in m[c].iter_mut() {
            *v /= d;
        }
        let (above, rest) = m.split_at_mut(c);
        let (pivot, below) = rest.split_first_mut()?;
        for row in above.iter_mut().chain(below) {
            let f = row[c];
            if f != 0.0 {
                for (v, p) in row.iter_mut().zip(pivot.iter()) {
                    *v -= f * p;
                }
            }
        }
    }
    Some(m.into_iter().map(|r| r[n..].to_vec()).collect())
}

fn binom(n: usize, k: usize) -> f64 {
    (1..=k).fold(1.0, |acc, i| acc * (n - k + i) as f64 / i as f64)
}

/// 最小二乘多项式拟合。
///
/// 先把 x 标准化 t = (x − x̄)/s，在 t 上解正规方程（条件数小得多），
/// 再用二项式展开把系数和协方差换算回原始 x。数据点不足或矩阵奇异时返回 None。
pub fn polyfit(xs: &[f64], ys: &[f64], order: usize) -> Option<PolyFit> {
    let n = xs.len().min(ys.len());
    let p = order + 1;
    if n <= p {
        return None;
    }
    let mx = xs[..n].iter().sum::<f64>() / n as f64;
    let sx = {
        let v = xs[..n].iter().map(|x| (x - mx).powi(2)).sum::<f64>() / n as f64;
        if v > 0.0 {
            v.sqrt()
        } else {
            1.0
        }
    };
    let mut a = vec![vec![0.0; p]; p];
    let mut b = vec![0.0; p];
    for i in 0..n {
        let t = (xs[i] - mx) / sx;
        let mut pw = vec![1.0; p];
        for j in 1..p {
            pw[j] = pw[j - 1] * t;
        }
        for r in 0..p {
            b[r] += pw[r] * ys[i];
            for c in 0..p {
                a[r][c] += pw[r] * pw[c];
            }
        }
    }
    let inv = mat_inv(&a)?;
    let bt: Vec<f64> = inv
        .iter()
        .map(|r| r.iter().zip(&b).map(|(v, bj)| v * bj).sum())
        .collect();
    let mut fit = PolyFit {
        coef: vec![],
        se: vec![],
        r2: 0.0,
        adj_r2: 0.0,
        rss: 0.0,
        n,
        dof: n - p,
        x_min: xs[..n].iter().copied().fold(f64::INFINITY, f64::min),
        x_max: xs[..n].iter().copied().fold(f64::NEG_INFINITY, f64::max),
        tss: 0.0,
        mx,
        sx,
        bt,
        cov_t: vec![],
    };
    let ym = ys[..n].iter().sum::<f64>() / n as f64;
    let (mut rss, mut tss) = (0.0, 0.0);
    for i in 0..n {
        rss += (ys[i] - fit.eval(xs[i])).powi(2);
        tss += (ys[i] - ym).powi(2);
    }
    let s2 = rss / fit.dof as f64;
    // m[k][j]：t^j 的系数对 x^k 系数的贡献
    let m: Vec<Vec<f64>> = (0..p)
        .map(|k| {
            (0..p)
                .map(|j| {
                    if j >= k {
                        binom(j, k) * (-mx).powi((j - k) as i32) / sx.powi(j as i32)
                    } else {
                        0.0
                    }
                })
                .collect()
        })
        .collect();
    fit.coef = m
        .iter()
        .map(|r| r.iter().zip(&fit.bt).map(|(v, b)| v * b).sum())
        .collect();
    fit.se = m
        .iter()
        .map(|r| {
            let mut v = 0.0;
            for j in 0..p {
                for l in 0..p {
                    v += r[j] * inv[j][l] * r[l];
                }
            }
            (v * s2).max(0.0).sqrt()
        })
        .collect();
    fit.rss = rss;
    fit.tss = tss;
    fit.cov_t = inv.iter().map(|r| r.iter().map(|v| v * s2).collect()).collect();
    fit.r2 = if tss > 0.0 { 1.0 - rss / tss } else { f64::NAN };
    fit.adj_r2 = 1.0 - (1.0 - fit.r2) * (n - 1) as f64 / fit.dof as f64;
    Some(fit)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SmoothMethod {
    /// 移动平均。
    MovingAverage,
    /// Savitzky-Golay 二阶（窗口内局部二次拟合）。
    SavitzkyGolay,
    /// LOWESS：每点取最近的“窗口点数”个点，按 tricube 权重做局部加权线性回归。
    Lowess,
    /// FFT 低通：去掉首尾连线趋势后，滤掉高于 1/(窗口点数·Δx) 的频率再加回趋势；需要等间距 X。
    FftFilter,
}

impl SmoothMethod {
    pub const ALL: [SmoothMethod; 4] = [
        SmoothMethod::SavitzkyGolay,
        SmoothMethod::MovingAverage,
        SmoothMethod::Lowess,
        SmoothMethod::FftFilter,
    ];

    pub fn label(self) -> &'static str {
        match self {
            SmoothMethod::MovingAverage => "移动平均",
            SmoothMethod::SavitzkyGolay => "Savitzky-Golay（二阶）",
            SmoothMethod::Lowess => "LOWESS",
            SmoothMethod::FftFilter => "FFT 滤波",
        }
    }
}

/// LOWESS 局部加权线性回归（不做稳健迭代）：x 不必有序；每点取最近的 k 个点（k = 窗口点数，不超过总点数），
/// 带宽为其中最远的距离，权重 (1 − (d/h)³)³。
pub fn lowess(xs: &[f64], ys: &[f64], window: usize) -> Vec<f64> {
    let n = ys.len().min(xs.len());
    if n == 0 {
        return vec![];
    }
    let k = window.max(2).min(n);
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by(|&a, &b| xs[a].total_cmp(&xs[b]));
    let sx: Vec<f64> = order.iter().map(|&i| xs[i]).collect();
    let mut out = vec![0.0; n];
    for (pos, &i) in order.iter().enumerate() {
        let x0 = sx[pos];
        // 有序数组里向两侧扩展，每次取更近的一侧，得到最近的 k 个点
        let (mut a, mut b) = (pos, pos);
        while b - a + 1 < k {
            if a == 0 {
                b += 1;
            } else if b == n - 1 || x0 - sx[a - 1] <= sx[b + 1] - x0 {
                a -= 1;
            } else {
                b += 1;
            }
        }
        let h = (x0 - sx[a]).max(sx[b] - x0);
        let (mut sw, mut swx, mut swy, mut swxx, mut swxy) = (0.0, 0.0, 0.0, 0.0, 0.0);
        for j in a..=b {
            let w = if h > 0.0 {
                let u = ((sx[j] - x0).abs() / h).min(1.0);
                (1.0 - u * u * u).powi(3)
            } else {
                1.0
            };
            let (x, y) = (sx[j] - x0, ys[order[j]]);
            sw += w;
            swx += w * x;
            swy += w * y;
            swxx += w * x * x;
            swxy += w * x * y;
        }
        // 以 x0 为原点的加权直线，截距即平滑值；退化时取加权平均
        let det = sw * swxx - swx * swx;
        out[i] = if det.abs() > 1e-12 * sw * swxx.max(f64::MIN_POSITIVE) {
            (swxx * swy - swx * swxy) / det
        } else if sw > 0.0 {
            swy / sw
        } else {
            ys[i]
        };
    }
    out
}

/// FFT 低通平滑：截止频率 1/(window·Δx)，Δx 取平均采样间隔。先减去首尾连线、滤波后加回，减少端点振铃。
pub fn fft_smooth(xs: &[f64], ys: &[f64], window: usize) -> Vec<f64> {
    let n = ys.len().min(xs.len());
    if n < 3 {
        return ys[..n].to_vec();
    }
    let dt = (xs[n - 1] - xs[0]) / (n - 1) as f64;
    if !(dt.is_finite() && dt > 0.0) {
        return ys[..n].to_vec();
    }
    let trend = |i: usize| ys[0] + (ys[n - 1] - ys[0]) * i as f64 / (n - 1) as f64;
    let detrended: Vec<f64> = (0..n).map(|i| ys[i] - trend(i)).collect();
    let cutoff = 1.0 / (window.max(2) as f64 * dt);
    match crate::fft::filter(&detrended, dt, crate::fft::FilterKind::LowPass(cutoff)) {
        Ok(v) => v.iter().enumerate().map(|(i, y)| y + trend(i)).collect(),
        Err(_) => ys[..n].to_vec(),
    }
}

/// 平滑。窗口点数会被调整为不小于 3 的奇数。
/// 移动平均在两端缩小窗口；Savitzky-Golay 在两端平移窗口保持点数，并按实际 x 间距拟合。
pub fn smooth(xs: &[f64], ys: &[f64], method: SmoothMethod, window: usize) -> Vec<f64> {
    match method {
        SmoothMethod::Lowess => return lowess(xs, ys, window),
        SmoothMethod::FftFilter => return fft_smooth(xs, ys, window),
        _ => {}
    }
    let n = ys.len();
    let w = (window.max(3)) | 1;
    let h = w / 2;
    (0..n)
        .map(|i| match method {
            SmoothMethod::MovingAverage => {
                let a = i.saturating_sub(h);
                let b = (i + h).min(n - 1);
                ys[a..=b].iter().sum::<f64>() / (b - a + 1) as f64
            }
            SmoothMethod::SavitzkyGolay => {
                let (a, b) = sg_window(i, n, h);
                polyfit(&xs[a..=b], &ys[a..=b], 2)
                    .map(|f| f.eval(xs[i]))
                    .unwrap_or(ys[i])
            }
            SmoothMethod::Lowess | SmoothMethod::FftFilter => unreachable!(),
        })
        .collect()
}

/// Savitzky–Golay 窗口：以 i 为中心、半宽 h；靠近两端时平移窗口保持点数。
fn sg_window(i: usize, n: usize, h: usize) -> (usize, usize) {
    let (mut a, mut b) = (i as isize - h as isize, (i + h) as isize);
    if a < 0 {
        b -= a;
        a = 0;
    }
    if b > n as isize - 1 {
        a -= b - (n as isize - 1);
        b = n as isize - 1;
    }
    (a.max(0) as usize, b as usize)
}

/// Savitzky–Golay 平滑求导：每点在窗口内按实际 x 做 `order` 阶最小二乘多项式，
/// 取该多项式在此点的 `deriv` 阶导数。窗口取奇数并不超过数据点数，阶数限制在窗口点数 − 2 以内。
/// 等间距数据与 SciPy `savgol_filter(..., deriv, delta, mode="interp")` 一致。
pub fn sg_derivative(xs: &[f64], ys: &[f64], window: usize, order: usize, deriv: usize) -> Vec<f64> {
    let n = ys.len().min(xs.len());
    if n < 3 {
        return vec![f64::NAN; n];
    }
    let mut w = window.max(3) | 1;
    if w > n {
        w = if n % 2 == 1 { n } else { n - 1 };
    }
    let order = order.clamp(deriv.max(1), w - 2);
    let h = w / 2;
    (0..n)
        .map(|i| {
            let (a, b) = sg_window(i, n, h);
            polyfit(&xs[a..=b], &ys[a..=b], order).map_or(f64::NAN, |f| f.derivative_at(xs[i], deriv))
        })
        .collect()
}

/// 数值微分：中间点用中心差分，两端用单侧差分。
pub fn derivative(xs: &[f64], ys: &[f64]) -> Vec<f64> {
    let n = ys.len();
    (0..n)
        .map(|i| {
            let a = if i == 0 { 0 } else { i - 1 };
            let b = if i + 1 >= n { n - 1 } else { i + 1 };
            (ys[b] - ys[a]) / (xs[b] - xs[a])
        })
        .collect()
}

/// 梯形法累积积分。返回 (累积积分, 绝对面积)。
/// 绝对面积把 x 轴下方的部分按正值计入；跨零的区间按两段三角形计算。
pub fn cumulative_trapezoid(xs: &[f64], ys: &[f64]) -> (Vec<f64>, f64) {
    let n = ys.len();
    let mut cum = Vec::with_capacity(n);
    let mut abs_area = 0.0;
    if n == 0 {
        return (cum, 0.0);
    }
    cum.push(0.0);
    for i in 1..n {
        let dx = xs[i] - xs[i - 1];
        let (y0, y1) = (ys[i - 1], ys[i]);
        let seg = dx * (y0 + y1) / 2.0;
        cum.push(cum[i - 1] + seg);
        abs_area += if y0 * y1 >= 0.0 {
            seg.abs()
        } else {
            dx.abs() * (y0 * y0 + y1 * y1) / (2.0 * (y0.abs() + y1.abs()))
        };
    }
    (cum, abs_area)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NormalizeMethod {
    /// 缩放到 [0, 1]。
    MinMax,
    /// 除以最大值。
    DivideByMax,
    /// Z 分数：减均值，除以样本标准差。
    ZScore,
}

impl NormalizeMethod {
    pub const ALL: [NormalizeMethod; 3] = [
        NormalizeMethod::MinMax,
        NormalizeMethod::DivideByMax,
        NormalizeMethod::ZScore,
    ];

    pub fn label(self) -> &'static str {
        match self {
            NormalizeMethod::MinMax => "缩放到 [0, 1]",
            NormalizeMethod::DivideByMax => "除以最大值",
            NormalizeMethod::ZScore => "Z 分数（减均值除以标准差）",
        }
    }
}

pub fn normalize(ys: &[f64], method: NormalizeMethod) -> Vec<f64> {
    let n = ys.len() as f64;
    let mn = ys.iter().copied().fold(f64::INFINITY, f64::min);
    let mx = ys.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let mean = ys.iter().sum::<f64>() / n;
    let sd = (ys.iter().map(|y| (y - mean).powi(2)).sum::<f64>() / (n - 1.0)).sqrt();
    ys.iter()
        .map(|y| match method {
            NormalizeMethod::MinMax => (y - mn) / (mx - mn),
            NormalizeMethod::DivideByMax => y / mx,
            NormalizeMethod::ZScore => (y - mean) / sd,
        })
        .collect()
}

/// 描述统计。标准差为样本标准差（n − 1），分位数用线性插值（与 Excel QUARTILE.INC 相同）。
#[derive(Clone, Debug, PartialEq)]
pub struct Describe {
    pub n: usize,
    pub mean: f64,
    pub sd: f64,
    pub se: f64,
    pub min: f64,
    pub q1: f64,
    pub median: f64,
    pub q3: f64,
    pub max: f64,
    pub sum: f64,
}

pub fn quantile(sorted: &[f64], q: f64) -> f64 {
    let pos = (sorted.len() - 1) as f64 * q;
    let (lo, hi) = (pos.floor() as usize, pos.ceil() as usize);
    sorted[lo] + (sorted[hi] - sorted[lo]) * (pos - lo as f64)
}

pub fn describe(values: &[f64]) -> Option<Describe> {
    let v: Vec<f64> = values.iter().copied().filter(|x| x.is_finite()).collect();
    let n = v.len();
    if n == 0 {
        return None;
    }
    let mut s = v.clone();
    s.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let sum: f64 = v.iter().sum();
    let mean = sum / n as f64;
    let sd = if n > 1 {
        (v.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (n - 1) as f64).sqrt()
    } else {
        f64::NAN
    };
    Some(Describe {
        n,
        mean,
        sd,
        se: sd / (n as f64).sqrt(),
        min: s[0],
        q1: quantile(&s, 0.25),
        median: quantile(&s, 0.5),
        q3: quantile(&s, 0.75),
        max: s[n - 1],
        sum,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() <= tol * (1.0 + b.abs())
    }

    #[test]
    fn linear_fit_textbook() {
        // x = 1..5, y = 2,4,5,4,5：斜率 0.6，截距 2.2，R² = 0.6
        let x = [1.0, 2.0, 3.0, 4.0, 5.0];
        let y = [2.0, 4.0, 5.0, 4.0, 5.0];
        let f = polyfit(&x, &y, 1).unwrap();
        assert!(close(f.coef[0], 2.2, 1e-12));
        assert!(close(f.coef[1], 0.6, 1e-12));
        assert!(close(f.r2, 0.6, 1e-12));
        assert!(close(f.rss, 2.4, 1e-12));
        // s² = 2.4/3 = 0.8；Sxx = 10 → se(斜率) = sqrt(0.08)；se(截距) = sqrt(0.8·(1/5 + 9/10))
        assert!(close(f.se[1], 0.08f64.sqrt(), 1e-12));
        assert!(close(f.se[0], (0.8f64 * 1.1).sqrt(), 1e-12));
        assert!(close(f.pearson_r().unwrap(), 0.6f64.sqrt(), 1e-12));
        assert_eq!(f.equation(), "y = 2.2 + 0.6·x");
    }

    #[test]
    fn quadratic_recovered_exactly_with_offset_x() {
        // x 远离 0 时直接解正规方程会严重失真，这里检验标准化后的换算
        let x: Vec<f64> = (0..30).map(|i| 1000.0 + i as f64 * 0.5).collect();
        let y: Vec<f64> = x.iter().map(|x| 3.0 - 0.002 * x + 1.5e-6 * x * x).collect();
        let f = polyfit(&x, &y, 2).unwrap();
        assert!(close(f.coef[0], 3.0, 1e-6), "{:?}", f.coef);
        assert!(close(f.coef[1], -0.002, 1e-6));
        assert!(close(f.coef[2], 1.5e-6, 1e-6));
        assert!(f.rss < 1e-18);
        assert!(close(
            f.eval(1007.0),
            3.0 - 0.002 * 1007.0 + 1.5e-6 * 1007.0 * 1007.0,
            1e-12
        ));
    }

    #[test]
    fn too_few_points() {
        assert!(polyfit(&[1.0, 2.0], &[1.0, 2.0], 1).is_none());
        assert!(polyfit(&[1.0, 1.0, 1.0], &[1.0, 2.0, 3.0], 1).is_none());
    }

    #[test]
    fn smoothing() {
        let x: Vec<f64> = (0..9).map(|i| i as f64).collect();
        let y = vec![1.0, 1.0, 1.0, 10.0, 1.0, 1.0, 1.0, 1.0, 1.0];
        let ma = smooth(&x, &y, SmoothMethod::MovingAverage, 3);
        assert!(close(ma[3], 4.0, 1e-12));
        assert!(close(ma[0], 1.0, 1e-12));
        // 二次多项式经 Savitzky-Golay 二阶平滑后不变
        let q: Vec<f64> = x.iter().map(|x| x * x - 3.0 * x).collect();
        let sg = smooth(&x, &q, SmoothMethod::SavitzkyGolay, 5);
        for (a, b) in sg.iter().zip(&q) {
            assert!(close(*a, *b, 1e-9));
        }
        // 偶数窗口按奇数处理
        assert_eq!(
            smooth(&x, &y, SmoothMethod::MovingAverage, 4),
            smooth(&x, &y, SmoothMethod::MovingAverage, 5)
        );
    }

    #[test]
    fn savitzky_golay_derivatives_match_scipy() {
        // 参考值：SciPy 1.17.1 savgol_filter(y, w, order, deriv, delta=0.5, mode="interp")
        let xs: Vec<f64> = (0..21).map(|i| i as f64 * 0.5).collect();
        let ys: Vec<f64> = xs
            .iter()
            .map(|x| x.sin() + 0.1 * (3.0 * x).cos() + 0.05 * x * x)
            .collect();
        let cases: [((usize, usize, usize), [f64; 21]); 3] = [
            (
                (7, 2, 1),
                [
                    1.2288647913282356,
                    0.878913110890015,
                    0.5289614304517944,
                    0.1790097500135727,
                    -0.11305126965050136,
                    -0.3178199022619514,
                    -0.4193068628723384,
                    -0.360385292862031,
                    -0.0940177391507212,
                    0.3137475274346442,
                    0.724245698629218,
                    1.055062901983875,
                    1.2893441204270788,
                    1.3834645877258036,
                    1.2749345673293544,
                    0.993761574815048,
                    0.671469969649942,
                    0.4151520871158408,
                    0.110712648234077,
                    -0.19372679064769227,
                    -0.4981662295294613,
                ],
            ),
            (
                (9, 3, 2),
                [
                    -1.1030624516612693,
                    -0.9521215782816457,
                    -0.8011807049020221,
                    -0.6502398315223986,
                    -0.4992989581427748,
                    -0.3053723773511236,
                    -0.033628165392129816,
                    0.3271659709805541,
                    0.6617433097968165,
                    0.8066127449432817,
                    0.7416961594678995,
                    0.5666397310787632,
                    0.320761216897252,
                    -0.01999320626558554,
                    -0.3805725173477872,
                    -0.5896260748950561,
                    -0.5758406998981406,
                    -0.5487916922003695,
                    -0.5217426845025978,
                    -0.494693676804826,
                    -0.4676446691070542,
                ],
            ),
            (
                (5, 3, 1),
                [
                    0.7745739500717682,
                    0.7257137891496365,
                    0.6023357879382183,
                    0.47601755760440767,
                    -0.1422955154852812,
                    -0.7946179103007259,
                    -0.7956748693480048,
                    -0.3547031538711767,
                    -0.11211816648978162,
                    0.029605896452785496,
                    0.6131718146745665,
                    1.4432223953654515,
                    1.754457565412268,
                    1.4663884094134998,
                    1.2337640411396378,
                    1.2232304430875134,
                    0.8914169928073257,
                    0.15538546948160104,
                    -0.2591836769420599,
                    -0.06169312502709734,
                    0.7042938573576871,
                ],
            ),
        ];
        for ((w, order, deriv), want) in cases {
            let got = sg_derivative(&xs, &ys, w, order, deriv);
            for (g, e) in got.iter().zip(want) {
                assert!(
                    (g - e).abs() < 1e-9 * (1.0 + e.abs()),
                    "w={w} order={order} deriv={deriv}: {g} vs {e}"
                );
            }
        }
        // 二次函数的二阶导数处处为常数；窗口大于数据点数时自动缩小
        let q: Vec<f64> = xs.iter().map(|x| 3.0 * x * x - x + 2.0).collect();
        assert!(sg_derivative(&xs, &q, 51, 2, 2).iter().all(|d| (d - 6.0).abs() < 1e-9));
        assert!(sg_derivative(&xs[..2], &q[..2], 5, 2, 1).iter().all(|d| d.is_nan()));
    }

    #[test]
    fn lowess_and_fft_smoothing_match_numpy_references() {
        // LOWESS 参考：NumPy 逐点取最近 k 点、tricube 权重加权线性最小二乘（输入 x 无序、不等间距）
        let xp = [
            2.359807621135331,
            0.0,
            12.419160170741758,
            5.826012958872607,
            0.75,
            10.47846096908265,
            4.0590169943749475,
            7.65,
            1.5414213562373094,
            11.443608329051592,
            4.934846922834953,
            8.581138830084189,
            3.1999999999999997,
            9.52414363469547,
            6.731370849898476,
        ];
        let yp = [
            1.2393594152648446,
            0.3,
            0.9397088056528399,
            -0.1587369139173762,
            0.6250147004584157,
            -0.12135780705562005,
            -0.3990030405357141,
            1.6745176613308415,
            0.9967635735845345,
            0.5036763647294626,
            -0.2639511451346272,
            1.4903286340883781,
            0.04936229817811538,
            1.1037479321589518,
            1.3407074170470343,
        ];
        for (k, want) in [
            (
                5,
                [
                    0.826048407995424,
                    0.3074982986326627,
                    0.8181299180348789,
                    0.24031276460305978,
                    0.6263820870553979,
                    0.41252336187186567,
                    -0.22880148376920403,
                    1.524568480510214,
                    0.9560102423787585,
                    0.5542156948059328,
                    -0.27294086351347413,
                    1.4329321951511074,
                    0.26854767808896873,
                    0.8648323762458509,
                    1.0007572160338059,
                ],
            ),
            (
                9,
                [
                    0.43884185987169244,
                    0.7824099153254878,
                    0.4624738639160305,
                    0.44560105972254715,
                    0.6901662289415013,
                    0.782390633499318,
                    0.14366653566629586,
                    1.0424922975400932,
                    0.5702685475123925,
                    0.6214830336031489,
                    0.1812709093135082,
                    1.0624786705017357,
                    0.3025465641716403,
                    0.9192846532158123,
                    0.7992514043812813,
                ],
            ),
        ] {
            let got = smooth(&xp, &yp, SmoothMethod::Lowess, k);
            for (g, w) in got.iter().zip(want) {
                assert!((g - w).abs() < 1e-9, "k={k}: {g} vs {w}");
            }
        }
        // 直线不变
        let line: Vec<f64> = xp.iter().map(|x| 2.0 * x - 1.0).collect();
        assert!(lowess(&xp, &line, 5)
            .iter()
            .zip(&line)
            .all(|(a, b)| (a - b).abs() < 1e-9));
        // FFT 平滑参考：numpy.fft.rfft 去首尾趋势、频率 > 1/(5·0.1) 置零、irfft 后加回趋势
        let t: Vec<f64> = (0..32).map(|i| i as f64 * 0.1).collect();
        let s: Vec<f64> = t
            .iter()
            .map(|t| {
                let tau = 2.0 * std::f64::consts::PI;
                (tau * 0.8 * t).sin() + 0.25 * (tau * 4.0 * t).sin() + 0.5 * t
            })
            .collect();
        let want = [
            0.09023167491837975,
            0.4803506441091951,
            0.923918654360487,
            1.1756623432721311,
            1.1244989072477463,
            0.8238938517285221,
            0.40491277963974576,
            -0.015327456136765671,
            -0.35212032417634176,
            -0.5261824557321666,
            -0.4644858113706162,
            -0.1467352511104726,
            0.35766069828793345,
            0.9135268792082749,
            1.3861355697679818,
            1.687261598598019,
            1.7733249019269877,
            1.6297979476553839,
            1.2825163237032107,
            0.822669805404732,
            0.39549121175389823,
            0.13814642664674448,
            0.11710981980833912,
            0.32228619407436376,
            0.7091457467732173,
            1.2241424069950442,
            1.7797380040387454,
            2.229453934181274,
            2.41861869394293,
            2.3034414995185717,
            2.027537549636495,
            1.8523774107559259,
        ];
        for (g, w) in smooth(&t, &s, SmoothMethod::FftFilter, 5).iter().zip(want) {
            assert!((g - w).abs() < 1e-9, "{g} vs {w}");
        }
    }

    #[test]
    fn polynomial_bands_anova_and_t_tests_match_numpy_scipy() {
        // 参考：NumPy 原始范德蒙德矩阵最小二乘 + scipy.stats（t、F 分布），二阶多项式，95%
        let noise = |i: usize| (1.7 * i as f64).sin() * 0.6 + (0.37 * (i * i) as f64).cos() * 0.4;
        let xs: Vec<f64> = (0..41).map(|i| i as f64 * 0.25).collect();
        let ys: Vec<f64> = xs
            .iter()
            .enumerate()
            .map(|(i, x)| 1.0 + 0.5 * x - 0.03 * x * x + 0.2 * noise(i))
            .collect();
        let fit = polyfit(&xs, &ys, 2).unwrap();
        let close = |g: f64, w: f64, rel: f64| assert!((g - w).abs() <= rel * w.abs(), "{g} vs {w}");
        let at = [0.0, 3.3, 7.7, 10.0, 12.0];
        let conf = [
            0.10150171944461821,
            0.0488263171620758,
            0.04851656672767118,
            0.10150171944461794,
            0.20421951132107755,
        ];
        let pred = [
            0.24893031711680438,
            0.2324816400864172,
            0.2324167829066984,
            0.24893031711680427,
            0.30556392544585254,
        ];
        for i in 0..5 {
            let (c, p) = fit.band(at[i], 0.95);
            close(c, conf[i], 1e-9);
            close(p, pred[i], 1e-9);
        }
        let a = fit.anova();
        close(a.f, 639.0783635586567, 1e-9);
        close(a.p, 5.611045444989869e-30, 1e-6);
        close(a.ss_reg, 16.113114531602644, 1e-9);
        close(a.ss_err, 0.4790479439730725, 1e-9);
        close(a.ss_tot, 16.592162475575716, 1e-9);
        assert_eq!((a.df_reg, a.df_err, a.df_tot), (2.0, 38.0, 40.0));
        let t = [20.512546418687485, 21.589277997879766, -13.58972948708151];
        let pv = [3.7382843618811248e-22, 6.215137765756575e-23, 3.610205994049847e-16];
        for (k, (tv, p)) in fit.t_tests().into_iter().enumerate() {
            close(tv, t[k], 1e-9);
            close(p, pv[k], 1e-6);
        }
    }

    #[test]
    fn derivative_and_integral() {
        let x: Vec<f64> = (0..=10).map(|i| i as f64 * 0.1).collect();
        let y: Vec<f64> = x.iter().map(|x| x * x).collect();
        let d = derivative(&x, &y);
        // 中心差分对二次函数是精确的
        for i in 1..10 {
            assert!(close(d[i], 2.0 * x[i], 1e-9));
        }
        let lin: Vec<f64> = x.clone();
        let (cum, abs) = cumulative_trapezoid(&x, &lin);
        assert!(close(*cum.last().unwrap(), 0.5, 1e-12));
        assert!(close(abs, 0.5, 1e-12));
        // y = x − 0.5 在 [0,1] 上：面积 0，绝对面积 0.25
        let shifted: Vec<f64> = x.iter().map(|x| x - 0.5).collect();
        let (cum, abs) = cumulative_trapezoid(&x, &shifted);
        assert!(cum.last().unwrap().abs() < 1e-12);
        assert!(close(abs, 0.25, 1e-12));
    }

    #[test]
    fn normalization() {
        let y = [2.0, 4.0, 6.0];
        assert_eq!(normalize(&y, NormalizeMethod::MinMax), vec![0.0, 0.5, 1.0]);
        assert_eq!(normalize(&y, NormalizeMethod::DivideByMax)[0], 2.0 / 6.0);
        assert_eq!(normalize(&y, NormalizeMethod::ZScore), vec![-1.0, 0.0, 1.0]);
    }

    #[test]
    fn descriptive_stats() {
        let d = describe(&[1.0, 2.0, 3.0, 4.0, f64::NAN, 10.0]).unwrap();
        assert_eq!(d.n, 5);
        assert_eq!(d.mean, 4.0);
        assert_eq!(d.median, 3.0);
        assert_eq!(d.q1, 2.0);
        assert_eq!(d.q3, 4.0);
        assert_eq!((d.min, d.max, d.sum), (1.0, 10.0, 20.0));
        assert!(close(d.sd, (50.0f64 / 4.0).sqrt(), 1e-12));
        assert!(describe(&[f64::NAN]).is_none());
    }

    #[test]
    fn equation_text() {
        let x = [0.0, 1.0, 2.0, 3.0];
        let y: Vec<f64> = x.iter().map(|x| 1.0 - 2.0 * x + 0.5 * x * x).collect();
        let f = polyfit(&x, &y, 2).unwrap();
        assert_eq!(f.order(), 2);
        assert_eq!(f.equation(), "y = 1 − 2·x + 0.5·x²");
    }
}
