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
    // 标准化后的系数，用来稳定地求值
    mx: f64,
    sx: f64,
    bt: Vec<f64>,
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
        mx,
        sx,
        bt,
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
}

impl SmoothMethod {
    pub fn label(self) -> &'static str {
        match self {
            SmoothMethod::MovingAverage => "移动平均",
            SmoothMethod::SavitzkyGolay => "Savitzky-Golay（二阶）",
        }
    }
}

/// 平滑。窗口点数会被调整为不小于 3 的奇数。
/// 移动平均在两端缩小窗口；Savitzky-Golay 在两端平移窗口保持点数，并按实际 x 间距拟合。
pub fn smooth(xs: &[f64], ys: &[f64], method: SmoothMethod, window: usize) -> Vec<f64> {
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
                let (mut a, mut b) = (i as isize - h as isize, (i + h) as isize);
                if a < 0 {
                    b -= a;
                    a = 0;
                }
                if b > n as isize - 1 {
                    a -= b - (n as isize - 1);
                    b = n as isize - 1;
                }
                let a = a.max(0) as usize;
                let b = b as usize;
                polyfit(&xs[a..=b], &ys[a..=b], 2)
                    .map(|f| f.eval(xs[i]))
                    .unwrap_or(ys[i])
            }
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
