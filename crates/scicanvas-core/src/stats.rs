//! 假设检验：单样本、双样本（Student / Welch）和配对 t 检验。
//! t 分布用正则化不完全 Beta 函数计算，结果与 SciPy `ttest_1samp` / `ttest_ind` / `ttest_rel` 一致。
//! 算法引用、数值对照与源码来源核对见 docs/COPYRIGHT_AUDIT.md；第三方声明保留在 THIRD_PARTY_NOTICES.md。

/// ln Γ(x)，Lanczos 近似（g = 7，9 项），x > 0。
pub fn ln_gamma(x: f64) -> f64 {
    const G: [f64; 9] = [
        0.999_999_999_999_809_9,
        676.520_368_121_885_1,
        -1_259.139_216_722_402_8,
        771.323_428_777_653_1,
        -176.615_029_162_140_6,
        12.507_343_278_686_905,
        -0.138_571_095_265_720_12,
        9.984_369_578_019_572e-6,
        1.505_632_735_149_311_6e-7,
    ];
    if x < 0.5 {
        // 反射公式
        return (std::f64::consts::PI / (std::f64::consts::PI * x).sin()).ln() - ln_gamma(1.0 - x);
    }
    let x = x - 1.0;
    let t = x + 7.5;
    let sum = G[1..]
        .iter()
        .enumerate()
        .fold(G[0], |s, (i, g)| s + g / (x + i as f64 + 1.0));
    0.5 * (2.0 * std::f64::consts::PI).ln() + (x + 0.5) * t.ln() - t + sum.ln()
}

/// 不完全 Beta 函数的连分式（Lentz 算法）。
fn beta_cf(a: f64, b: f64, x: f64) -> f64 {
    const TINY: f64 = 1e-300;
    let (qab, qap, qam) = (a + b, a + 1.0, a - 1.0);
    let mut c = 1.0;
    let mut d = 1.0 - qab * x / qap;
    d = if d.abs() < TINY { TINY } else { d };
    d = 1.0 / d;
    let mut h = d;
    for m in 1..=500 {
        let m = m as f64;
        let m2 = 2.0 * m;
        for aa in [
            m * (b - m) * x / ((qam + m2) * (a + m2)),
            -(a + m) * (qab + m) * x / ((a + m2) * (qap + m2)),
        ] {
            d = 1.0 + aa * d;
            d = if d.abs() < TINY { TINY } else { d };
            c = 1.0 + aa / c;
            c = if c.abs() < TINY { TINY } else { c };
            d = 1.0 / d;
            h *= d * c;
        }
        if (d * c - 1.0).abs() < 1e-16 {
            break;
        }
    }
    h
}

/// 正则化不完全 Beta 函数 I_x(a, b)。
pub fn beta_inc(a: f64, b: f64, x: f64) -> f64 {
    if x <= 0.0 {
        return 0.0;
    }
    if x >= 1.0 {
        return 1.0;
    }
    let front = (ln_gamma(a + b) - ln_gamma(a) - ln_gamma(b) + a * x.ln() + b * (1.0 - x).ln()).exp();
    if x < (a + 1.0) / (a + b + 2.0) {
        front * beta_cf(a, b, x) / a
    } else {
        1.0 - front * beta_cf(b, a, 1.0 - x) / b
    }
}

/// 自由度为 `df` 的 t 分布累积分布函数。
pub fn t_cdf(t: f64, df: f64) -> f64 {
    let tail = 0.5 * beta_inc(df / 2.0, 0.5, df / (df + t * t));
    if t > 0.0 {
        1.0 - tail
    } else {
        tail
    }
}

/// t 分布的分位数（`t_cdf` 的反函数），二分到双精度极限。
pub fn t_ppf(p: f64, df: f64) -> f64 {
    let (mut lo, mut hi) = (-1.0, 1.0);
    while t_cdf(lo, df) > p {
        lo *= 2.0;
    }
    while t_cdf(hi, df) < p {
        hi *= 2.0;
    }
    for _ in 0..200 {
        let mid = 0.5 * (lo + hi);
        if mid == lo || mid == hi {
            break;
        }
        if t_cdf(mid, df) < p {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    0.5 * (lo + hi)
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TTestKind {
    /// 与假设均值比较。
    OneSample { mu: f64 },
    /// 两组独立样本；`equal_var` 为假时用 Welch 检验。
    TwoSample { equal_var: bool },
    /// 按行配对的两组样本。
    Paired,
}

impl TTestKind {
    pub fn label(self) -> &'static str {
        match self {
            TTestKind::OneSample { .. } => "单样本 t 检验",
            TTestKind::TwoSample { equal_var: true } => "双样本 t 检验（方差相等）",
            TTestKind::TwoSample { equal_var: false } => "双样本 Welch t 检验",
            TTestKind::Paired => "配对 t 检验",
        }
    }
}

/// t 检验结果。`estimate` 是均值（单样本）、均值差 A−B（双样本）或差值均值（配对），
/// 置信区间针对 `estimate`，p 值为双侧。
#[derive(Clone, Debug, PartialEq)]
pub struct TTest {
    pub kind: TTestKind,
    pub n: Vec<usize>,
    pub mean: Vec<f64>,
    pub sd: Vec<f64>,
    pub estimate: f64,
    pub se: f64,
    pub t: f64,
    pub df: f64,
    pub p: f64,
    pub level: f64,
    pub ci: (f64, f64),
}

fn mean_var(v: &[f64]) -> (f64, f64) {
    let n = v.len() as f64;
    let mean = v.iter().sum::<f64>() / n;
    let var = v.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (n - 1.0);
    (mean, var)
}

fn finite(v: &[f64]) -> Vec<f64> {
    v.iter().copied().filter(|x| x.is_finite()).collect()
}

/// 运行 t 检验。`a`、`b` 中的非有限值视为缺失；配对检验只用两组都有值的行。
pub fn t_test(kind: TTestKind, a: &[f64], b: &[f64], level: f64) -> Result<TTest, String> {
    if level.is_nan() || level <= 0.0 || level >= 1.0 {
        return Err("置信水平应在 0 与 1 之间".into());
    }
    let need = |v: &[f64], what: &str| -> Result<(), String> {
        if v.len() < 2 {
            Err(format!("{what}至少需要 2 个有效数值"))
        } else {
            Ok(())
        }
    };
    let (n, mean, sd, estimate, se, df) = match kind {
        TTestKind::OneSample { .. } => {
            let a = finite(a);
            need(&a, "样本")?;
            let (m, v) = mean_var(&a);
            let n = a.len() as f64;
            (vec![a.len()], vec![m], vec![v.sqrt()], m, (v / n).sqrt(), n - 1.0)
        }
        TTestKind::TwoSample { equal_var } => {
            let (a, b) = (finite(a), finite(b));
            need(&a, "第一组")?;
            need(&b, "第二组")?;
            let ((ma, va), (mb, vb)) = (mean_var(&a), mean_var(&b));
            let (na, nb) = (a.len() as f64, b.len() as f64);
            let (se, df) = if equal_var {
                let pooled = ((na - 1.0) * va + (nb - 1.0) * vb) / (na + nb - 2.0);
                ((pooled * (1.0 / na + 1.0 / nb)).sqrt(), na + nb - 2.0)
            } else {
                let (qa, qb) = (va / na, vb / nb);
                (
                    (qa + qb).sqrt(),
                    (qa + qb).powi(2) / (qa * qa / (na - 1.0) + qb * qb / (nb - 1.0)),
                )
            };
            (
                vec![a.len(), b.len()],
                vec![ma, mb],
                vec![va.sqrt(), vb.sqrt()],
                ma - mb,
                se,
                df,
            )
        }
        TTestKind::Paired => {
            let pairs: Vec<(f64, f64)> = a
                .iter()
                .zip(b)
                .filter(|(x, y)| x.is_finite() && y.is_finite())
                .map(|(x, y)| (*x, *y))
                .collect();
            let d: Vec<f64> = pairs.iter().map(|(x, y)| x - y).collect();
            need(&d, "配对数据")?;
            let (md, vd) = mean_var(&d);
            let n = d.len() as f64;
            let first: Vec<f64> = pairs.iter().map(|p| p.0).collect();
            let second: Vec<f64> = pairs.iter().map(|p| p.1).collect();
            let ((ma, va), (mb, vb)) = (mean_var(&first), mean_var(&second));
            (
                vec![d.len(), d.len()],
                vec![ma, mb],
                vec![va.sqrt(), vb.sqrt()],
                md,
                (vd / n).sqrt(),
                n - 1.0,
            )
        }
    };
    let null = match kind {
        TTestKind::OneSample { mu } => mu,
        _ => 0.0,
    };
    if se.is_nan() || se <= 0.0 || !df.is_finite() {
        return Err("数据没有变化（标准误为 0），无法计算 t 值".into());
    }
    let t = (estimate - null) / se;
    let p = (2.0 * t_cdf(-t.abs(), df)).min(1.0);
    let q = t_ppf(0.5 + level / 2.0, df);
    Ok(TTest {
        kind,
        n,
        mean,
        sd,
        estimate,
        se,
        t,
        df,
        p,
        level,
        ci: (estimate - q * se, estimate + q * se),
    })
}

/// 正则化下不完全 Gamma 函数 P(a, x)；x < a + 1 时直接用级数，避免 1 − Q 的相减误差。
pub fn gamma_p(a: f64, x: f64) -> f64 {
    if x <= 0.0 {
        return 0.0;
    }
    if x < a + 1.0 {
        let front = (-x + a * x.ln() - ln_gamma(a)).exp();
        let (mut term, mut sum, mut ap) = (1.0 / a, 1.0 / a, a);
        for _ in 0..1000 {
            ap += 1.0;
            term *= x / ap;
            sum += term;
            if term.abs() < sum.abs() * 1e-17 {
                break;
            }
        }
        sum * front
    } else {
        1.0 - gamma_q(a, x)
    }
}

/// 正则化上不完全 Gamma 函数 Q(a, x)：x < a + 1 时用级数，否则用连分式。
pub fn gamma_q(a: f64, x: f64) -> f64 {
    if x <= 0.0 {
        return 1.0;
    }
    let front = (-x + a * x.ln() - ln_gamma(a)).exp();
    if x < a + 1.0 {
        let (mut term, mut sum, mut ap) = (1.0 / a, 1.0 / a, a);
        for _ in 0..1000 {
            ap += 1.0;
            term *= x / ap;
            sum += term;
            if term.abs() < sum.abs() * 1e-17 {
                break;
            }
        }
        1.0 - sum * front
    } else {
        const TINY: f64 = 1e-300;
        let mut b = x + 1.0 - a;
        let mut c = 1.0 / TINY;
        let mut d = 1.0 / b;
        let mut h = d;
        for i in 1..1000 {
            let an = -(i as f64) * (i as f64 - a);
            b += 2.0;
            d = an * d + b;
            d = if d.abs() < TINY { TINY } else { d };
            c = b + an / c;
            c = if c.abs() < TINY { TINY } else { c };
            d = 1.0 / d;
            let del = d * c;
            h *= del;
            if (del - 1.0).abs() < 1e-16 {
                break;
            }
        }
        front * h
    }
}

/// 互补误差函数 erfc(x) = Q(1/2, x²)。
pub fn erfc(x: f64) -> f64 {
    if x < 0.0 {
        2.0 - erfc(-x)
    } else {
        gamma_q(0.5, x * x)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CorrMethod {
    Pearson,
    Spearman,
    Kendall,
}

impl CorrMethod {
    pub const ALL: [CorrMethod; 3] = [CorrMethod::Pearson, CorrMethod::Spearman, CorrMethod::Kendall];

    pub fn label(self) -> &'static str {
        match self {
            CorrMethod::Pearson => "Pearson r",
            CorrMethod::Spearman => "Spearman ρ",
            CorrMethod::Kendall => "Kendall τ-b",
        }
    }
}

/// 相关系数与双侧 p 值。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Correlation {
    pub method: CorrMethod,
    pub n: usize,
    pub r: f64,
    pub p: f64,
}

/// 平均秩（并列值取平均名次），从 1 开始。
fn ranks(v: &[f64]) -> Vec<f64> {
    let mut idx: Vec<usize> = (0..v.len()).collect();
    idx.sort_by(|a, b| v[*a].total_cmp(&v[*b]));
    let mut out = vec![0.0; v.len()];
    let mut i = 0;
    while i < idx.len() {
        let mut j = i;
        while j + 1 < idx.len() && v[idx[j + 1]] == v[idx[i]] {
            j += 1;
        }
        let rank = (i + j) as f64 / 2.0 + 1.0;
        for k in &idx[i..=j] {
            out[*k] = rank;
        }
        i = j + 1;
    }
    out
}

fn pearson_r(x: &[f64], y: &[f64]) -> f64 {
    let n = x.len() as f64;
    let (mx, my) = (x.iter().sum::<f64>() / n, y.iter().sum::<f64>() / n);
    let sxy: f64 = x.iter().zip(y).map(|(a, b)| (a - mx) * (b - my)).sum();
    let sxx: f64 = x.iter().map(|a| (a - mx).powi(2)).sum();
    let syy: f64 = y.iter().map(|b| (b - my).powi(2)).sum();
    (sxy / (sxx * syy).sqrt()).clamp(-1.0, 1.0)
}

/// 并列组大小的三个求和：Σt(t−1)、Σt(t−1)(t−2)、Σt(t−1)(2t+5)。
fn tie_sums(v: &[f64]) -> (f64, f64, f64) {
    let mut sorted = v.to_vec();
    sorted.sort_by(|a, b| a.total_cmp(b));
    let (mut a, mut b, mut c) = (0.0, 0.0, 0.0);
    let mut i = 0;
    while i < sorted.len() {
        let mut j = i;
        while j + 1 < sorted.len() && sorted[j + 1] == sorted[i] {
            j += 1;
        }
        let t = (j - i + 1) as f64;
        a += t * (t - 1.0);
        b += t * (t - 1.0) * (t - 2.0);
        c += t * (t - 1.0) * (2.0 * t + 5.0);
        i = j + 1;
    }
    (a, b, c)
}

/// 相关系数。只用两组都有值的行；Pearson、Spearman 的 p 值用 t 分布，
/// Kendall τ-b 用含并列校正的正态近似（与 SciPy `kendalltau(method="asymptotic")` 相同）。
pub fn correlation(x: &[f64], y: &[f64], method: CorrMethod) -> Result<Correlation, String> {
    let (x, y): (Vec<f64>, Vec<f64>) = x
        .iter()
        .zip(y)
        .filter(|(a, b)| a.is_finite() && b.is_finite())
        .map(|(a, b)| (*a, *b))
        .unzip();
    let n = x.len();
    if n < 3 {
        return Err("相关系数至少需要 3 对有效数值".into());
    }
    let constant = |v: &[f64]| v.iter().all(|a| *a == v[0]);
    if constant(&x) || constant(&y) {
        return Err("有一列数值全部相同，相关系数没有定义".into());
    }
    let t_test = |r: f64| {
        let df = (n - 2) as f64;
        if r.abs() >= 1.0 {
            0.0
        } else {
            let t = r * (df / (1.0 - r * r)).sqrt();
            (2.0 * t_cdf(-t.abs(), df)).min(1.0)
        }
    };
    let (r, p) = match method {
        CorrMethod::Pearson => {
            let r = pearson_r(&x, &y);
            (r, t_test(r))
        }
        CorrMethod::Spearman => {
            let r = pearson_r(&ranks(&x), &ranks(&y));
            (r, t_test(r))
        }
        CorrMethod::Kendall => {
            let mut s = 0.0;
            for i in 0..n {
                for j in i + 1..n {
                    s += ((x[i] - x[j]).signum() * f64::from(x[i] != x[j]))
                        * ((y[i] - y[j]).signum() * f64::from(y[i] != y[j]));
                }
            }
            let (xt, x0, x1) = tie_sums(&x);
            let (yt, y0, y1) = tie_sums(&y);
            let nf = n as f64;
            let pairs = nf * (nf - 1.0) / 2.0;
            let tau = s / ((pairs - xt / 2.0) * (pairs - yt / 2.0)).sqrt();
            let m = nf * (nf - 1.0);
            let var = (m * (2.0 * nf + 5.0) - x1 - y1) / 18.0
                + 2.0 * (xt / 2.0) * (yt / 2.0) / m
                + x0 * y0 / (9.0 * m * (nf - 2.0));
            let z = s / var.sqrt();
            (tau, erfc(z.abs() / std::f64::consts::SQRT_2).min(1.0))
        }
    };
    Ok(Correlation { method, n, r, p })
}

/// F 分布的上尾概率 P(F > f)，自由度 (d1, d2)。
pub fn f_sf(f: f64, d1: f64, d2: f64) -> f64 {
    if f <= 0.0 {
        return 1.0;
    }
    beta_inc(d2 / 2.0, d1 / 2.0, d2 / (d2 + d1 * f))
}

/// 单因素方差分析结果：组间、组内的平方和、自由度、均方，F 与 p，以及各组 N、均值、标准差。
#[derive(Clone, Debug, PartialEq)]
pub struct Anova {
    pub n: Vec<usize>,
    pub mean: Vec<f64>,
    pub sd: Vec<f64>,
    pub ss_between: f64,
    pub ss_within: f64,
    pub df_between: f64,
    pub df_within: f64,
    pub f: f64,
    pub p: f64,
}

impl Anova {
    pub fn ms_between(&self) -> f64 {
        self.ss_between / self.df_between
    }
    pub fn ms_within(&self) -> f64 {
        self.ss_within / self.df_within
    }
}

/// 单因素方差分析（与 SciPy `f_oneway` 相同）；每组的非有限值视为缺失。
pub fn one_way_anova(groups: &[Vec<f64>]) -> Result<Anova, String> {
    let groups: Vec<Vec<f64>> = groups.iter().map(|g| finite(g)).collect();
    if groups.len() < 2 {
        return Err("方差分析至少需要 2 组".into());
    }
    if groups.iter().any(|g| g.is_empty()) {
        return Err("有一组没有有效数值".into());
    }
    let total: usize = groups.iter().map(Vec::len).sum();
    let k = groups.len();
    if total <= k {
        return Err("总数据点数必须多于组数".into());
    }
    let grand = groups.iter().flatten().sum::<f64>() / total as f64;
    let mut out = Anova {
        n: vec![],
        mean: vec![],
        sd: vec![],
        ss_between: 0.0,
        ss_within: 0.0,
        df_between: (k - 1) as f64,
        df_within: (total - k) as f64,
        f: 0.0,
        p: 0.0,
    };
    for g in &groups {
        let m = g.iter().sum::<f64>() / g.len() as f64;
        let ss: f64 = g.iter().map(|x| (x - m).powi(2)).sum();
        out.n.push(g.len());
        out.mean.push(m);
        out.sd.push(if g.len() > 1 {
            (ss / (g.len() - 1) as f64).sqrt()
        } else {
            f64::NAN
        });
        out.ss_between += g.len() as f64 * (m - grand).powi(2);
        out.ss_within += ss;
    }
    if out.ss_within <= 0.0 {
        return Err("组内没有变化（组内平方和为 0），无法计算 F 值".into());
    }
    out.f = out.ms_between() / out.ms_within();
    out.p = f_sf(out.f, out.df_between, out.df_within);
    Ok(out)
}

/// 标准正态分布函数 Φ(x)。
pub fn norm_cdf(x: f64) -> f64 {
    0.5 * erfc(-x / std::f64::consts::SQRT_2)
}

/// 学生化极差在无穷自由度下的分布函数 P(W < w)（k 组为 `cc`，`rr` 为 1），
/// Copenhaver–Holland 算法的 Gauss–Legendre 积分（同 R `ptukey` 的 `wprob`）。
fn range_cdf_inf(w: f64, cc: f64) -> f64 {
    const XLEG: [f64; 6] = [
        0.981_560_634_246_719_3,
        0.904_117_256_370_474_9,
        0.769_902_674_194_304_7,
        0.587_317_954_286_617_4,
        0.367_831_498_998_180_2,
        0.125_233_408_511_468_9,
    ];
    const ALEG: [f64; 6] = [
        0.047_175_336_386_511_83,
        0.106_939_325_995_318_43,
        0.160_078_328_543_346_23,
        0.203_167_426_723_065_92,
        0.233_492_536_538_354_8,
        0.249_147_045_813_402_8,
    ];
    let qsqz = w * 0.5;
    if qsqz >= 8.0 {
        return 1.0;
    }
    // 极差落在 [−w/2, w/2] 内的部分
    let mut pr = (2.0 * norm_cdf(qsqz) - 1.0).powf(cc);
    let steps = if w > 3.0 { 2 } else { 3 };
    let binc = (8.0 - qsqz) / steps as f64;
    let (mut lo, mut hi) = (qsqz, qsqz + binc);
    for _ in 0..steps {
        let (a, b) = (0.5 * (hi + lo), 0.5 * (hi - lo));
        let mut sum = 0.0;
        for jj in 0..12 {
            let (xx, wt) = if jj < 6 {
                (-XLEG[jj], ALEG[jj])
            } else {
                (XLEG[11 - jj], ALEG[11 - jj])
            };
            let ac = a + b * xx;
            if ac * ac > 60.0 {
                break;
            }
            let inner = norm_cdf(ac) - norm_cdf(ac - w);
            if inner >= (-30.0 / (cc - 1.0)).exp() {
                sum += wt * (-0.5 * ac * ac).exp() * inner.powf(cc - 1.0);
            }
        }
        pr += sum * 2.0 * b * cc / (2.0 * std::f64::consts::PI).sqrt();
        lo = hi;
        hi += binc;
    }
    pr.min(1.0)
}

/// 学生化极差分布函数 P(Q < q)，k 组、自由度 df（同 R `ptukey`）。
pub fn studentized_range_cdf(q: f64, k: f64, df: f64) -> f64 {
    const XLEGQ: [f64; 8] = [
        0.989_400_934_991_649_9,
        0.944_575_023_073_232_6,
        0.865_631_202_387_831_7,
        0.755_404_408_355_003,
        0.617_876_244_402_643_7,
        0.458_016_777_657_227_4,
        0.281_603_550_779_258_9,
        0.095_012_509_837_637_44,
    ];
    const ALEGQ: [f64; 8] = [
        0.027_152_459_411_754_095,
        0.062_253_523_938_647_89,
        0.095_158_511_682_492_78,
        0.124_628_971_255_533_87,
        0.149_595_988_816_576_73,
        0.169_156_519_395_002_54,
        0.182_603_415_044_923_6,
        0.189_450_610_455_068_5,
    ];
    if q <= 0.0 {
        return 0.0;
    }
    if df > 25000.0 {
        return range_cdf_inf(q, k);
    }
    // 对 s（样本标准差 / σ）的卡方型密度分段做 16 点 Gauss–Legendre 积分
    let f2 = df * 0.5;
    let ulen: f64 = if df <= 100.0 {
        1.0
    } else if df <= 800.0 {
        0.5
    } else if df <= 5000.0 {
        0.25
    } else {
        0.125
    };
    let f2lf = f2 * df.ln() - df * std::f64::consts::LN_2 - ln_gamma(f2) + ulen.ln();
    let (f21, ff4) = (f2 - 1.0, df * 0.25);
    let mut ans = 0.0;
    for i in 1..=50 {
        let mut part = 0.0;
        let twa1 = (2 * i - 1) as f64 * ulen;
        for jj in 0..16 {
            let (u, wt) = if jj < 8 {
                (twa1 - XLEGQ[jj] * ulen, ALEGQ[jj])
            } else {
                (twa1 + XLEGQ[jj - 8] * ulen, ALEGQ[jj - 8])
            };
            let t1 = f2lf + f21 * u.ln() - u * ff4;
            if t1 >= -30.0 {
                part += range_cdf_inf(q * (u * 0.5).sqrt(), k) * wt * t1.exp();
            }
        }
        if i as f64 * ulen >= 1.0 && part <= 1e-14 {
            break;
        }
        ans += part;
    }
    ans.min(1.0)
}

/// 学生化极差分布的分位数（二分法反解 `studentized_range_cdf`）。
pub fn studentized_range_ppf(p: f64, k: f64, df: f64) -> f64 {
    let (mut lo, mut hi) = (0.0, 1.0);
    while studentized_range_cdf(hi, k, df) < p {
        hi *= 2.0;
    }
    for _ in 0..100 {
        let mid = 0.5 * (lo + hi);
        if studentized_range_cdf(mid, k, df) < p {
            lo = mid;
        } else {
            hi = mid;
        }
        if hi - lo < 1e-12 * hi {
            break;
        }
    }
    0.5 * (lo + hi)
}

/// Tukey HSD 两两比较的一行：组 `a` 减组 `b` 的均值差、置信区间与校正后 p 值。
#[derive(Clone, Debug, PartialEq)]
pub struct TukeyPair {
    pub a: usize,
    pub b: usize,
    pub diff: f64,
    pub low: f64,
    pub high: f64,
    pub p: f64,
}

/// Tukey HSD 多重比较（与 SciPy `tukey_hsd` 相同；组大小不等时为 Tukey–Kramer）。`level` 为置信水平。
pub fn tukey_hsd(a: &Anova, level: f64) -> Vec<TukeyPair> {
    let k = a.n.len() as f64;
    let mse = a.ms_within();
    let crit = studentized_range_ppf(level, k, a.df_within);
    let mut out = vec![];
    for i in 0..a.n.len() {
        for j in i + 1..a.n.len() {
            let se = (mse / 2.0 * (1.0 / a.n[i] as f64 + 1.0 / a.n[j] as f64)).sqrt();
            let diff = a.mean[i] - a.mean[j];
            out.push(TukeyPair {
                a: i,
                b: j,
                diff,
                low: diff - crit * se,
                high: diff + crit * se,
                p: (1.0 - studentized_range_cdf(diff.abs() / se, k, a.df_within)).clamp(0.0, 1.0),
            });
        }
    }
    out
}

/// 标准正态分布的分位数（Newton 法反解 Φ）。
pub fn norm_ppf(p: f64) -> f64 {
    if !(0.0..=1.0).contains(&p) || p.is_nan() {
        return f64::NAN;
    }
    if p == 0.0 {
        return f64::NEG_INFINITY;
    }
    if p == 1.0 {
        return f64::INFINITY;
    }
    if p > 0.5 {
        return -norm_ppf(1.0 - p);
    }
    // 左半边 Φ 是凸函数，从 0 出发的 Newton 迭代单调收敛
    let mut x = 0.0_f64;
    for _ in 0..200 {
        let pdf = (-0.5 * x * x).exp() / (2.0 * std::f64::consts::PI).sqrt();
        let step = (norm_cdf(x) - p) / pdf;
        x -= step;
        if step.abs() < 1e-15 * x.abs().max(1.0) {
            break;
        }
    }
    x
}

/// 正态性检验结果。
#[derive(Clone, Debug, PartialEq)]
pub struct Normality {
    pub n: usize,
    pub w: f64,
    pub p: f64,
}

/// Shapiro–Wilk 正态性检验（Royston 1995 的 AS R94 算法，与 SciPy `shapiro` 相同），3 ≤ n ≤ 5000。
pub fn shapiro_wilk(values: &[f64]) -> Result<Normality, String> {
    let poly = |c: &[f64], x: f64| c.iter().rev().fold(0.0, |acc, k| acc * x + k);
    let mut x = finite(values);
    let n = x.len();
    if n < 3 {
        return Err("Shapiro–Wilk 检验至少需要 3 个数值".into());
    }
    if n > 5000 {
        return Err("Shapiro–Wilk 检验最多 5000 个数值".into());
    }
    x.sort_by(f64::total_cmp);
    let range = x[n - 1] - x[0];
    if range < 1e-19 {
        return Err("数值全部相同，无法检验".into());
    }
    let an = n as f64;
    let half = n / 2;
    // 系数 a（只算前一半，按反对称展开）
    let mut a = vec![0.0; half];
    if n == 3 {
        a[0] = std::f64::consts::FRAC_1_SQRT_2;
    } else {
        let m: Vec<f64> = (1..=half).map(|i| norm_ppf((i as f64 - 0.375) / (an + 0.25))).collect();
        let summ2 = 2.0 * m.iter().map(|v| v * v).sum::<f64>();
        let ssumm2 = summ2.sqrt();
        let rsn = 1.0 / an.sqrt();
        let a1 = poly(&[0.0, 0.221157, -0.147981, -2.07119, 4.434685, -2.706056], rsn) - m[0] / ssumm2;
        let (first, fac) = if n > 5 {
            let a2 = -m[1] / ssumm2 + poly(&[0.0, 0.042981, -0.293762, -1.752461, 5.682633, -3.582633], rsn);
            a[1] = a2;
            let fac = ((summ2 - 2.0 * m[0] * m[0] - 2.0 * m[1] * m[1]) / (1.0 - 2.0 * a1 * a1 - 2.0 * a2 * a2)).sqrt();
            (2, fac)
        } else {
            (1, ((summ2 - 2.0 * m[0] * m[0]) / (1.0 - 2.0 * a1 * a1)).sqrt())
        };
        a[0] = a1;
        for i in first..half {
            a[i] = -m[i] / fac;
        }
    }
    // W：系数与排序后数据的相关系数平方
    let coef: Vec<f64> = (0..n)
        .map(|i| {
            if i < half {
                -a[i]
            } else if n - 1 - i < half {
                a[n - 1 - i]
            } else {
                0.0
            }
        })
        .collect();
    let xs: Vec<f64> = x.iter().map(|v| v / range).collect();
    let (ca, cx) = (coef.iter().sum::<f64>() / an, xs.iter().sum::<f64>() / an);
    let (mut ssa, mut ssx, mut sax) = (0.0, 0.0, 0.0);
    for (c, v) in coef.iter().zip(&xs) {
        let (da, dx) = (c - ca, v - cx);
        ssa += da * da;
        ssx += dx * dx;
        sax += da * dx;
    }
    let root = (ssa * ssx).sqrt();
    let w1 = (root - sax) * (root + sax) / (ssa * ssx);
    let w = 1.0 - w1;
    let p = if n == 3 {
        let pi6 = 6.0 / std::f64::consts::PI;
        (pi6 * (w.sqrt().asin() - std::f64::consts::FRAC_PI_3)).max(0.0)
    } else {
        let mut y = w1.ln();
        let (m, sd) = if n <= 11 {
            let gamma = poly(&[-2.273, 0.459], an);
            if y >= gamma {
                return Ok(Normality { n, w, p: 1e-99 });
            }
            y = -(gamma - y).ln();
            (
                poly(&[0.544, -0.39978, 0.025054, -6.714e-4], an),
                poly(&[1.3822, -0.77857, 0.062767, -0.0020322], an).exp(),
            )
        } else {
            let lx = an.ln();
            (
                poly(&[-1.5861, -0.31082, -0.083751, 0.0038915], lx),
                poly(&[-0.4803, -0.082676, 0.0030302], lx).exp(),
            )
        };
        1.0 - norm_cdf((y - m) / sd)
    };
    Ok(Normality { n, w, p })
}

/// Grubbs 异常值检验（双侧）：离均值最远的点是否为异常值。
#[derive(Clone, Debug, PartialEq)]
pub struct Grubbs {
    pub n: usize,
    /// 可疑值在输入中的位置（跳过缺失值之前的序号）与数值。
    pub index: usize,
    pub value: f64,
    pub g: f64,
    /// 显著性水平 `alpha` 下的临界值。
    pub critical: f64,
    /// 双侧 p 值（Bonferroni 上界，最大为 1）。
    pub p: f64,
}

pub fn grubbs(values: &[f64], alpha: f64) -> Result<Grubbs, String> {
    let pts: Vec<(usize, f64)> = values
        .iter()
        .copied()
        .enumerate()
        .filter(|(_, v)| v.is_finite())
        .collect();
    let n = pts.len();
    if n < 3 {
        return Err("Grubbs 检验至少需要 3 个数值".into());
    }
    let nf = n as f64;
    let mean = pts.iter().map(|p| p.1).sum::<f64>() / nf;
    let sd = (pts.iter().map(|p| (p.1 - mean).powi(2)).sum::<f64>() / (nf - 1.0)).sqrt();
    if sd <= 0.0 {
        return Err("数值全部相同，无法检验".into());
    }
    let &(index, value) = pts
        .iter()
        .max_by(|a, b| (a.1 - mean).abs().total_cmp(&(b.1 - mean).abs()))
        .unwrap();
    let g = (value - mean).abs() / sd;
    let df = nf - 2.0;
    let t = t_ppf(alpha / (2.0 * nf), df);
    let critical = (nf - 1.0) / nf.sqrt() * (t * t / (df + t * t)).sqrt();
    let denom = (nf - 1.0).powi(2) - nf * g * g;
    let p = if denom <= 0.0 {
        0.0
    } else {
        let tt = (nf * df * g * g / denom).sqrt();
        (2.0 * nf * t_cdf(-tt, df)).min(1.0)
    };
    Ok(Grubbs {
        n,
        index,
        value,
        g,
        critical,
        p,
    })
}

/// 归一化互相关 r(k) = Σ (x_i − x̄)(y_{i+k} − ȳ) / √(Σ(x − x̄)² Σ(y − ȳ)²)，
/// 滞后 k 从 −(n−1) 到 n−1（与 `numpy.correlate(y − ȳ, x − x̄, "full")` 归一化后相同）。
/// x 与 y 相同时即自相关。返回 (滞后, 相关系数)。
pub fn cross_correlation(x: &[f64], y: &[f64]) -> Result<(Vec<i64>, Vec<f64>), String> {
    if x.len() != y.len() {
        return Err("两列的数据点数不同".into());
    }
    if x.iter().chain(y).any(|v| !v.is_finite()) {
        return Err("数据中有缺失值或非数值，请先补齐".into());
    }
    let n = x.len();
    if n < 2 {
        return Err("至少需要 2 个数据点".into());
    }
    let center = |v: &[f64]| {
        let m = v.iter().sum::<f64>() / n as f64;
        v.iter().map(|a| a - m).collect::<Vec<f64>>()
    };
    let (a, b) = (center(x), center(y));
    let norm = (a.iter().map(|v| v * v).sum::<f64>() * b.iter().map(|v| v * v).sum::<f64>()).sqrt();
    if norm == 0.0 {
        return Err("数值全部相同，相关函数没有定义".into());
    }
    let n = n as i64;
    let lags: Vec<i64> = (-(n - 1)..n).collect();
    let r = lags
        .iter()
        .map(|&k| {
            (0..n)
                .filter(|i| (0..n).contains(&(i + k)))
                .map(|i| a[i as usize] * b[(i + k) as usize])
                .sum::<f64>()
                / norm
        })
        .collect();
    Ok((lags, r))
}

/// 数据序列的类型。
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Sequence {
    /// 等差：首项与公差。
    Arithmetic { start: f64, step: f64 },
    /// 均匀分布随机数 [min, max)。
    Uniform { min: f64, max: f64 },
    /// 正态分布随机数。
    Normal { mean: f64, sd: f64 },
}

/// SplitMix64 伪随机数，给出 [0, 1) 的均匀数。
struct SplitMix(u64);

impl SplitMix {
    fn next(&mut self) -> f64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
        (z >> 11) as f64 / (1u64 << 53) as f64
    }
}

/// 生成 `n` 个数据。随机序列由 `seed` 决定，相同种子给出相同结果。
pub fn sequence(kind: Sequence, n: usize, seed: u64) -> Vec<f64> {
    let mut rng = SplitMix(seed);
    (0..n)
        .map(|i| match kind {
            Sequence::Arithmetic { start, step } => start + step * i as f64,
            Sequence::Uniform { min, max } => min + (max - min) * rng.next(),
            Sequence::Normal { mean, sd } => {
                // Box–Muller；1 − u 避免 ln 0
                let (u, v) = (1.0 - rng.next(), rng.next());
                mean + sd * (-2.0 * u.ln()).sqrt() * (2.0 * std::f64::consts::PI * v).cos()
            }
        })
        .collect()
}

/// 多元线性回归 y = b0 + b1·x1 + … + bk·xk 的结果。系数顺序为截距在前。
#[derive(Clone, Debug, PartialEq)]
pub struct Regression {
    pub n: usize,
    pub coef: Vec<f64>,
    pub se: Vec<f64>,
    pub t: Vec<f64>,
    pub p: Vec<f64>,
    pub r2: f64,
    pub adj_r2: f64,
    pub f: f64,
    pub f_p: f64,
    /// 残差标准差。
    pub s: f64,
}

/// 多元线性回归（最小二乘）。`xs` 是各自变量的列；任一变量缺失的行整行跳过。
pub fn multiple_regression(xs: &[Vec<f64>], y: &[f64]) -> Result<Regression, String> {
    if xs.is_empty() {
        return Err("至少需要一个自变量".into());
    }
    let rows: Vec<usize> = (0..y.len())
        .filter(|&i| y[i].is_finite() && xs.iter().all(|x| x.get(i).is_some_and(|v| v.is_finite())))
        .collect();
    let (n, k) = (rows.len(), xs.len() + 1);
    if n <= k {
        return Err(format!("有效行数（{n}）必须多于参数个数（{k}）"));
    }
    let design = |i: usize, j: usize| if j == 0 { 1.0 } else { xs[j - 1][i] };
    let matrix: Vec<Vec<f64>> = rows.iter().map(|&i| (0..k).map(|j| design(i, j)).collect()).collect();
    let values: Vec<f64> = rows.iter().map(|&i| y[i]).collect();
    let solution =
        crate::linalg::lstsq(&matrix, &values).map_err(|_| "自变量之间线性相关（或某一列为常数），无法回归")?;
    let inv = solution.cov_unscaled;
    let coef = solution.coef;
    let my = rows.iter().map(|&i| y[i]).sum::<f64>() / n as f64;
    let (mut sse, mut sst) = (0.0, 0.0);
    for &i in &rows {
        let fit: f64 = (0..k).map(|a| coef[a] * design(i, a)).sum();
        sse += (y[i] - fit).powi(2);
        sst += (y[i] - my).powi(2);
    }
    let df = (n - k) as f64;
    let s2 = sse / df;
    let se: Vec<f64> = (0..k).map(|a| (s2 * inv[a][a]).sqrt()).collect();
    let t: Vec<f64> = coef.iter().zip(&se).map(|(c, s)| c / s).collect();
    let p = t.iter().map(|t| (2.0 * t_cdf(-t.abs(), df)).min(1.0)).collect();
    let r2 = if sst > 0.0 { 1.0 - sse / sst } else { f64::NAN };
    let f = (sst - sse) / (k - 1) as f64 / s2;
    Ok(Regression {
        n,
        coef,
        se,
        t,
        p,
        r2,
        adj_r2: 1.0 - (1.0 - r2) * (n - 1) as f64 / df,
        f,
        f_p: f_sf(f, (k - 1) as f64, df),
        s: s2.sqrt(),
    })
}

/// 线性标准曲线 y = a + b·x，用于由响应值反查浓度。
#[derive(Clone, Debug, PartialEq)]
pub struct Calibration {
    pub n: usize,
    pub intercept: f64,
    pub slope: f64,
    /// 残差标准差 s(y/x)。
    pub s: f64,
    pub r2: f64,
    mean_y: f64,
    sxx: f64,
}

impl Calibration {
    /// 由标准点拟合；缺失值成对跳过。
    pub fn fit(x: &[f64], y: &[f64]) -> Result<Calibration, String> {
        let (x, y): (Vec<f64>, Vec<f64>) = x
            .iter()
            .zip(y)
            .filter(|(a, b)| a.is_finite() && b.is_finite())
            .map(|(a, b)| (*a, *b))
            .unzip();
        let n = x.len();
        if n < 3 {
            return Err("标准曲线至少需要 3 个标准点".into());
        }
        let nf = n as f64;
        let (mx, my) = (x.iter().sum::<f64>() / nf, y.iter().sum::<f64>() / nf);
        let sxx: f64 = x.iter().map(|v| (v - mx).powi(2)).sum();
        let syy: f64 = y.iter().map(|v| (v - my).powi(2)).sum();
        if sxx <= 0.0 {
            return Err("标准点的浓度（X）全部相同".into());
        }
        let slope = x.iter().zip(&y).map(|(a, b)| (a - mx) * (b - my)).sum::<f64>() / sxx;
        if slope == 0.0 {
            return Err("标准曲线斜率为 0，无法反查".into());
        }
        let intercept = my - slope * mx;
        let sse: f64 = x.iter().zip(&y).map(|(a, b)| (b - intercept - slope * a).powi(2)).sum();
        Ok(Calibration {
            n,
            intercept,
            slope,
            s: (sse / (nf - 2.0)).sqrt(),
            r2: if syy > 0.0 { 1.0 - sse / syy } else { 1.0 },
            mean_y: my,
            sxx,
        })
    }

    /// 由 `m` 次平行测定的平均响应 `y0` 反查 x0，返回 (x0, 标准差 s(x0), 95% 置信区间半宽)。
    /// 公式见 Miller & Miller《分析化学中的统计与化学计量学》。
    pub fn inverse(&self, y0: f64, m: usize) -> (f64, f64, f64) {
        let (b, nf) = (self.slope, self.n as f64);
        let x0 = (y0 - self.intercept) / b;
        let sx0 =
            self.s / b.abs() * (1.0 / m as f64 + 1.0 / nf + (y0 - self.mean_y).powi(2) / (b * b * self.sxx)).sqrt();
        (x0, sx0, t_ppf(0.975, nf - 2.0) * sx0)
    }
}

/// 频数统计的一个区间 `[lo, hi)`；最后一个区间包含右端点。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bin {
    pub lo: f64,
    pub hi: f64,
    pub count: usize,
}

/// Sturges 规则的区间数：⌈log₂ n⌉ + 1。
pub fn sturges_bins(n: usize) -> usize {
    ((n.max(1) as f64).log2().ceil() as usize) + 1
}

/// 等宽分箱，规则与 `numpy.histogram` 相同：边界为 `lo + 步长 × i`，最后一个区间闭合，
/// 范围外的值和非有限值不计；`range` 为空时取数据最小、最大值，全部相同时左右各扩 0.5。
pub fn histogram(values: &[f64], bins: usize, range: Option<(f64, f64)>) -> Result<Vec<Bin>, String> {
    let v: Vec<f64> = values.iter().copied().filter(|x| x.is_finite()).collect();
    if bins == 0 {
        return Err("区间数至少为 1".into());
    }
    let (mut lo, mut hi) = match range {
        Some(r) => r,
        None if v.is_empty() => return Err("没有可统计的数值".into()),
        None => (
            v.iter().copied().fold(f64::INFINITY, f64::min),
            v.iter().copied().fold(f64::NEG_INFINITY, f64::max),
        ),
    };
    if !(lo.is_finite() && hi.is_finite()) || lo > hi {
        return Err("统计范围无效：起点应不大于终点".into());
    }
    if lo == hi {
        (lo, hi) = (lo - 0.5, hi + 0.5);
    }
    let step = (hi - lo) / bins as f64;
    let mut edges: Vec<f64> = (0..=bins).map(|i| lo + step * i as f64).collect();
    edges[bins] = hi;
    let mut out: Vec<Bin> = edges
        .windows(2)
        .map(|w| Bin {
            lo: w[0],
            hi: w[1],
            count: 0,
        })
        .collect();
    for x in v {
        if x < lo || x > hi {
            continue;
        }
        let i = edges.partition_point(|e| *e <= x).clamp(1, bins) - 1;
        out[i].count += 1;
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn correlations_match_scipy_with_ties_and_negative_trends() {
        let near = |got: f64, want: f64| assert!((got - want).abs() <= 1e-9 * want.abs().max(1e-12), "{got} vs {want}");
        for (z, want) in [
            (0.1, 0.8875370839817152),
            (1.0, 0.15729920705028516),
            (2.5, 0.00040695201744495886),
            (5.0, 1.5374597944280347e-12),
        ] {
            near(erfc(z), want);
        }
        let x = [1.2, 2.4, 2.4, 3.1, 4.8, 5.0, 5.5, 6.1, 7.3, 7.3, 8.0, 9.6];
        let y = [2.0, 2.9, 3.5, 3.1, 4.2, 6.0, 5.1, 6.6, 6.6, 8.1, 7.7, 9.0];
        let w = [3.0, 1.0, 2.5, 4.0, 2.0, 3.5];
        let v = [1.0, 2.0, 1.5, 0.5, 2.2, 0.9];
        // SciPy 1.17.1：pearsonr、spearmanr、kendalltau(method="asymptotic")
        for (method, a, b) in [
            (
                CorrMethod::Pearson,
                (0.9684776256889684, 2.3248265725276443e-07),
                (-0.9180549521652276, 0.00979735617315806),
            ),
            (
                CorrMethod::Spearman,
                (0.9648521051838397, 3.9823058062772386e-07),
                (-0.942857142857143, 0.004804664723032055),
            ),
            (
                CorrMethod::Kendall,
                (0.8837474839481103, 8.27769245515797e-05),
                (-0.8666666666666666, 0.014595034919316668),
            ),
        ] {
            let c = correlation(&x, &y, method).unwrap();
            near(c.r, a.0);
            near(c.p, a.1);
            let c = correlation(&w, &v, method).unwrap();
            near(c.r, b.0);
            near(c.p, b.1);
        }
        let mut xs = x.to_vec();
        xs.push(f64::NAN);
        let mut ys = y.to_vec();
        ys.push(1.0);
        assert_eq!(correlation(&xs, &ys, CorrMethod::Pearson).unwrap().n, 12);
        assert!(correlation(&[1.0, 2.0], &[1.0, 2.0], CorrMethod::Pearson).is_err());
        assert!(correlation(&[1.0, 1.0, 1.0], &[1.0, 2.0, 3.0], CorrMethod::Spearman).is_err());
    }

    #[test]
    fn one_way_anova_matches_scipy() {
        let near = |got: f64, want: f64| assert!((got - want).abs() <= 1e-9 * want.abs().max(1e-12), "{got} vs {want}");
        // SciPy 1.17.1：f.sf 与 f_oneway
        near(f_sf(3.7, 2.0, 15.0), 0.049411075318250015);
        near(f_sf(0.4, 4.0, 30.0), 0.8070497187155657);
        let groups = vec![
            vec![5.1, 4.9, 6.2, 5.8, 6.0, 5.5, f64::NAN],
            vec![4.6, 5.0, 4.4, 5.2, 4.8],
            vec![6.3, 6.9, 7.1, 6.5, 6.8, 7.4, 6.6],
        ];
        let a = one_way_anova(&groups).unwrap();
        assert_eq!(a.n, [6, 5, 7]);
        near(a.ss_between, 12.254444444444433);
        near(a.ss_within, 2.5483333333333342);
        assert_eq!((a.df_between, a.df_within), (2.0, 15.0));
        near(a.f, 36.06605624591238);
        near(a.p, 1.859292068536071e-06);
        assert!(one_way_anova(&groups[..1]).is_err());
        assert!(one_way_anova(&[vec![1.0, 1.0], vec![2.0, 2.0]])
            .unwrap_err()
            .contains("组内"));
        assert!(one_way_anova(&[vec![1.0], vec![2.0]]).is_err());
    }

    #[test]
    fn tukey_hsd_matches_scipy() {
        let near = |got: f64, want: f64, tol: f64| assert!((got - want).abs() <= tol * want.abs(), "{got} vs {want}");
        // SciPy 1.17.1：studentized_range.sf / ppf 与 tukey_hsd
        for (q, k, df, sf) in [
            (3.5, 3.0, 15.0, 0.06291535464371001),
            (1.2, 4.0, 30.0, 0.8308381575181755),
            (5.0, 2.0, 8.0, 0.007669728020954114),
            (2.8, 6.0, 100.0, 0.3611337924235287),
            (4.4, 3.0, 2000.0, 0.005364251952650667),
            (0.3, 3.0, 10.0, 0.9755580143338938),
        ] {
            near(1.0 - studentized_range_cdf(q, k, df), sf, 1e-6);
        }
        for (k, df, q) in [
            (3.0, 15.0, 3.6733776588970963),
            (4.0, 30.0, 3.845401353042607),
            (2.0, 8.0, 3.2611823228941685),
        ] {
            near(studentized_range_ppf(0.95, k, df), q, 1e-7);
        }
        let groups = vec![
            vec![5.1, 4.9, 6.2, 5.8, 6.0, 5.5],
            vec![4.6, 5.0, 4.4, 5.2, 4.8],
            vec![6.3, 6.9, 7.1, 6.5, 6.8, 7.4, 6.6],
        ];
        let pairs = tukey_hsd(&one_way_anova(&groups).unwrap(), 0.95);
        let want = [
            (
                0,
                1,
                0.7833333333333332,
                0.017494544725510597,
                0.135044134745392,
                1.4316225319212745,
            ),
            (
                0,
                2,
                -1.216666666666666,
                0.0002449843862987189,
                -1.8123015186967866,
                -0.6210318146365452,
            ),
            (
                1,
                2,
                -1.9999999999999991,
                1.5856745170239606e-06,
                -2.626887588262127,
                -1.3731124117378712,
            ),
        ];
        assert_eq!(pairs.len(), 3);
        for (got, (a, b, diff, p, low, high)) in pairs.iter().zip(want) {
            assert_eq!((got.a, got.b), (a, b));
            near(got.diff, diff, 1e-12);
            near(got.p, p, 1e-5);
            near(got.low, low, 1e-7);
            near(got.high, high, 1e-7);
        }
    }

    fn s20() -> Vec<f64> {
        (0..20)
            .map(|i| ((i * 37) % 23) as f64 / 7.0 + (i as f64).sin())
            .collect()
    }

    #[test]
    fn shapiro_wilk_matches_scipy() {
        let near = |got: f64, want: f64, tol: f64| assert!((got - want).abs() <= tol * want.abs(), "{got} vs {want}");
        near(norm_ppf(1.25e-4), -3.6622599308877013, 1e-12);
        near(norm_ppf(0.3), -0.5244005127080409, 1e-12);
        near(norm_ppf(0.7), 0.5244005127080409, 1e-12);
        let s60: Vec<f64> = (0..60)
            .map(|i| (-1.0 + 3.0 * i as f64 / 59.0).exp() + 0.1 * (i as f64 * 2.3).cos())
            .collect();
        // SciPy 1.17.1 shapiro（其内部用单精度，W 只有 6–7 位有效数字）
        for (data, w, p) in [
            (vec![2.1, 3.7, 3.0], 0.9948186528497406, 0.8624063231829059),
            (
                vec![4.2, 4.8, 5.1, 5.0, 4.4, 6.9, 4.9, 5.3],
                0.8285523114638335,
                0.057342968785825094,
            ),
            (s20(), 0.9826675147410115, 0.963716670343387),
            (s60, 0.8642262952339322, 8.22293515304458e-06),
        ] {
            let r = shapiro_wilk(&data).unwrap();
            near(r.w, w, 1e-6);
            near(r.p, p, 1e-4);
        }
        assert!(shapiro_wilk(&[1.0, 2.0]).is_err());
        assert!(shapiro_wilk(&[1.0, 1.0, 1.0]).is_err());
    }

    #[test]
    fn grubbs_matches_t_distribution_formula() {
        let near = |got: f64, want: f64| assert!((got - want).abs() <= 1e-9 * want.abs(), "{got} vs {want}");
        // scipy.stats.t.ppf / t.sf 按 Grubbs 公式计算
        let x = [4.2, 4.8, 5.1, 5.0, f64::NAN, 4.4, 6.9, 4.9, 5.3];
        let r = grubbs(&x, 0.05).unwrap();
        assert_eq!((r.n, r.index, r.value), (8, 6, 6.9));
        near(r.g, 2.223673601093327);
        near(r.critical, 2.1266450871954685);
        near(r.p, 0.01935394175439725);
        let r = grubbs(&s20(), 0.05).unwrap();
        assert_eq!(r.index, 5);
        near(r.g, 1.902394763555888);
        near(r.critical, 2.708245645805754);
        near(r.p, 0.9544522095301384);
        assert!(grubbs(&[1.0, 1.0, 1.0], 0.05).is_err());
    }

    #[test]
    fn cross_and_auto_correlation_match_numpy() {
        let near = |got: f64, want: f64| assert!((got - want).abs() <= 1e-12, "{got} vs {want}");
        let x = [1.0, 3.0, 2.0, 5.0, 4.0, 6.0, 5.5, 7.0];
        let y = [0.5, 1.0, 2.5, 2.0, 4.5, 4.0, 6.0, 5.0];
        let (lags, r) = cross_correlation(&x, &y).unwrap();
        assert_eq!((lags[0], *lags.last().unwrap(), r.len()), (-7, 7, 15));
        // numpy.correlate(y − ȳ, x − x̄, "full") / √(Σx'² Σy'²)
        for (k, want) in [
            (-7, -0.2634433505602296),
            (-2, 0.2121161447921642),
            (0, 0.7569741752531662),
            (1, 0.6544559101514335),
            (7, -0.20136057647471814),
        ] {
            near(r[(k + 7) as usize], want);
        }
        let (_, a) = cross_correlation(&x, &x).unwrap();
        near(a[7], 1.0);
        near(a[8], 0.3398070907194995);
        near(a[10], -0.19330031282586027);
        near(a[6], a[8]);
        assert!(cross_correlation(&x, &y[..7]).is_err());
        assert!(cross_correlation(&[1.0, f64::NAN], &[1.0, 2.0]).is_err());
        assert!(cross_correlation(&[1.0, 1.0], &[1.0, 2.0]).is_err());
    }

    #[test]
    fn sequences_are_reproducible_and_have_the_right_moments() {
        assert_eq!(
            sequence(Sequence::Arithmetic { start: 2.0, step: 0.5 }, 4, 0),
            [2.0, 2.5, 3.0, 3.5]
        );
        let u = sequence(Sequence::Uniform { min: 1.0, max: 3.0 }, 20_000, 7);
        assert!(u.iter().all(|v| (1.0..3.0).contains(v)));
        let mean = u.iter().sum::<f64>() / u.len() as f64;
        assert!((mean - 2.0).abs() < 0.02, "{mean}");
        let g = sequence(Sequence::Normal { mean: 5.0, sd: 2.0 }, 20_000, 11);
        let m = g.iter().sum::<f64>() / g.len() as f64;
        let sd = (g.iter().map(|v| (v - m).powi(2)).sum::<f64>() / (g.len() - 1) as f64).sqrt();
        assert!((m - 5.0).abs() < 0.05 && (sd - 2.0).abs() < 0.05, "{m} {sd}");
        assert_eq!(g[..5], sequence(Sequence::Normal { mean: 5.0, sd: 2.0 }, 5, 11)[..]);
        assert_ne!(g[..5], sequence(Sequence::Normal { mean: 5.0, sd: 2.0 }, 5, 12)[..]);
    }

    #[test]
    fn multiple_regression_matches_numpy_and_scipy() {
        let near = |got: f64, want: f64| assert!((got - want).abs() <= 1e-9 * want.abs(), "{got} vs {want}");
        let x1: Vec<f64> = (1..=10).map(f64::from).collect();
        let mut x2 = vec![2.0, 1.5, 3.2, 2.8, 4.1, 3.9, 5.5, 5.0, 6.2, 6.8];
        let mut y = vec![5.1, 6.0, 8.9, 9.7, 12.6, 13.1, 16.4, 16.2, 19.5, 21.0];
        // numpy.linalg.lstsq，标准误 / t / p 与 scipy.stats.t、f 计算
        let check = |r: &Regression| {
            for (got, want) in r
                .coef
                .iter()
                .zip([1.7134747847478458, 1.0458979089790894, 1.313191881918821])
            {
                near(*got, want);
            }
            for (got, want) in
                r.se.iter()
                    .zip([0.2716168984837354, 0.11457163503596314, 0.19603498815694728])
            {
                near(*got, want);
            }
            for (got, want) in
                r.p.iter()
                    .zip([0.00040090279266631336, 3.888855306495218e-05, 0.0002777537567371148])
            {
                near(*got, want);
            }
            near(r.t[1], 9.128768291128866);
            near(r.r2, 0.9980150489717398);
            near(r.adj_r2, 0.9974479201065225);
            near(r.f, 1759.7676827638034);
            near(r.f_p, 3.4843701764736113e-10);
            near(r.s, 0.2752116962847333);
        };
        check(&multiple_regression(&[x1.clone(), x2.clone()], &y).unwrap());
        // 缺失行整行跳过
        let mut x1m = x1.clone();
        x1m.push(11.0);
        x2.push(f64::NAN);
        y.push(30.0);
        let r = multiple_regression(&[x1m, x2], &y).unwrap();
        assert_eq!(r.n, 10);
        check(&r);
        // 共线
        let x3: Vec<f64> = x1.iter().map(|v| 2.0 * v).collect();
        assert!(multiple_regression(&[x1.clone(), x3], &y[..10])
            .unwrap_err()
            .contains("线性相关"));
        assert!(multiple_regression(&[x1[..2].to_vec()], &y[..2]).is_err());
    }

    #[test]
    fn calibration_inverse_matches_numpy_and_scipy() {
        let near = |got: f64, want: f64| assert!((got - want).abs() <= 1e-9 * want.abs(), "{got} vs {want}");
        let x = [0.0, 1.0, 2.0, 4.0, 6.0, 8.0, 10.0];
        let y = [0.02, 0.215, 0.41, 0.79, 1.22, 1.58, 2.01];
        let c = Calibration::fit(&x, &y).unwrap();
        // numpy.polyfit 与 scipy.stats.t.ppf(0.975, n − 2)
        for (y0, m, x0, sx0, half) in [
            (0.9, 1, 4.468198640158359, 0.0830757705868752, 0.21355306685211103),
            (1.5, 3, 7.494276615887768, 0.05961193918634394, 0.15323736805947238),
        ] {
            let (a, b, h) = c.inverse(y0, m);
            near(a, x0);
            near(b, sx0);
            near(h, half);
        }
        assert!(c.r2 > 0.999);
        assert!(Calibration::fit(&[1.0, 1.0, 1.0], &[1.0, 2.0, 3.0]).is_err());
        assert!(Calibration::fit(&[1.0, 2.0], &[1.0, 2.0]).is_err());
    }

    #[test]
    fn histogram_matches_numpy() {
        let v = [
            2.3,
            4.1,
            3.3,
            5.0,
            1.2,
            3.8,
            4.4,
            2.9,
            3.1,
            5.0,
            1.2,
            4.0,
            3.6,
            f64::NAN,
        ];
        let counts = |b: &[Bin]| b.iter().map(|b| b.count).collect::<Vec<_>>();
        // numpy.histogram(v, bins=4)
        let auto = histogram(&v, 4, None).unwrap();
        assert_eq!(counts(&auto), [2, 2, 5, 4]);
        let edges: Vec<f64> = auto.iter().map(|b| b.lo).chain([auto[3].hi]).collect();
        assert_eq!(edges, [1.2, 2.15, 3.0999999999999996, 4.05, 5.0]);
        // numpy.histogram(v, bins=5, range=(0, 6))
        let ranged = histogram(&v, 5, Some((0.0, 6.0))).unwrap();
        assert_eq!(counts(&ranged), [0, 3, 3, 5, 2]);
        // 范围外的值不计；numpy.histogram(v, bins=2, range=(3, 4))
        assert_eq!(counts(&histogram(&v, 2, Some((3.0, 4.0))).unwrap()), [2, 3]);
        assert_eq!(sturges_bins(13), 5);
        let same = histogram(&[2.0, 2.0], 2, None).unwrap();
        assert_eq!((same[0].lo, same[1].hi, counts(&same)), (1.5, 2.5, vec![0, 2]));
        assert!(histogram(&[], 3, None).is_err());
        assert!(histogram(&v, 3, Some((4.0, 1.0))).is_err());
    }

    const A: [f64; 8] = [5.1, 4.9, 6.2, 5.8, 6.0, 5.5, 5.3, 6.1];
    const B: [f64; 10] = [4.6, 5.0, 4.4, 5.2, 4.8, 4.1, 5.4, 4.7, 4.3, 5.0];

    fn close(got: f64, want: f64) {
        assert!((got - want).abs() <= 1e-9 * want.abs().max(1e-3), "{got} vs {want}");
    }

    /// 参考值来自 SciPy 1.17.1：[t, df, p, CI 下限, CI 上限]。
    fn check(r: &TTest, want: [f64; 5]) {
        for (got, want) in [r.t, r.df, r.p, r.ci.0, r.ci.1].into_iter().zip(want) {
            close(got, want);
        }
    }

    #[test]
    fn distribution_matches_scipy() {
        for (t, want) in [
            (-2.5, 0.020496109292876444),
            (0.3, 0.6135549747989328),
            (1.9, 0.9503969902663183),
        ] {
            close(t_cdf(t, 7.0), want);
        }
        close(t_cdf(4.2, 2.5), 0.9825178456371594);
        for (q, want) in [
            (0.975, 2.1492012622298096),
            (0.995, 2.9868560441933756),
            (0.6, 0.2583202647824722),
        ] {
            close(t_ppf(q, 13.7), want);
        }
        close(ln_gamma(0.5), std::f64::consts::PI.sqrt().ln());
        close(ln_gamma(10.0), 362_880f64.ln());
    }

    #[test]
    fn tests_match_scipy() {
        let one = t_test(TTestKind::OneSample { mu: 5.0 }, &A, &[], 0.95).unwrap();
        check(
            &one,
            [
                3.569625885346298,
                7.0,
                0.009101473008820858,
                5.206762143815647,
                6.018237856184353,
            ],
        );
        let one99 = t_test(TTestKind::OneSample { mu: 5.0 }, &A, &[], 0.99).unwrap();
        close(one99.ci.0, 5.012035528799753);
        close(one99.ci.1, 6.212964471200246);
        let student = t_test(TTestKind::TwoSample { equal_var: true }, &A, &B, 0.95).unwrap();
        check(
            &student,
            [
                4.082643692449276,
                16.0,
                0.0008675734181156577,
                0.4146484463706847,
                1.3103515536293149,
            ],
        );
        let welch = t_test(TTestKind::TwoSample { equal_var: false }, &A, &B, 0.95).unwrap();
        check(
            &welch,
            [
                4.004642282892891,
                13.816471472032072,
                0.001336595933605592,
                0.3999901550351209,
                1.3250098449648786,
            ],
        );
        let paired = t_test(TTestKind::Paired, &A, &B[..8], 0.95).unwrap();
        check(
            &paired,
            [
                3.294576062789573,
                7.0,
                0.01321899964605484,
                0.23639904711074056,
                1.4386009528892592,
            ],
        );
    }

    #[test]
    fn missing_values_and_degenerate_input() {
        // 配对：只用两组都有值的行
        let mut a = A.to_vec();
        let mut b = B[..8].to_vec();
        a.push(f64::NAN);
        b.push(9.0);
        a.push(7.0);
        b.push(f64::NAN);
        let paired = t_test(TTestKind::Paired, &a, &b, 0.95).unwrap();
        assert_eq!(paired.n, [8, 8]);
        close(paired.t, 3.294576062789573);
        assert!(t_test(TTestKind::OneSample { mu: 0.0 }, &[1.0], &[], 0.95).is_err());
        assert!(t_test(TTestKind::OneSample { mu: 0.0 }, &[2.0, 2.0, 2.0], &[], 0.95)
            .unwrap_err()
            .contains("标准误为 0"));
        assert!(t_test(TTestKind::Paired, &A, &B, 1.5).is_err());
    }
}
