//! 假设检验：单样本、双样本（Student / Welch）和配对 t 检验。
//! t 分布用正则化不完全 Beta 函数计算，结果与 SciPy `ttest_1samp` / `ttest_ind` / `ttest_rel` 一致。

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
