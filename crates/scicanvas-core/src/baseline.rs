//! 基线：两端连线，以及非对称最小二乘（ALS，Eilers & Boelens 2005）自动基线。

#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum BaselineMethod {
    /// 首末两点的连线。
    Line,
    /// 非对称最小二乘：平滑度 λ 与非对称系数 p。
    Als { lambda: f64, p: f64 },
}

impl BaselineMethod {
    pub fn label(&self) -> String {
        match self {
            BaselineMethod::Line => "两端连线".into(),
            BaselineMethod::Als { lambda, p } => format!("ALS，λ = {lambda:e}，p = {p}"),
        }
    }
}

/// 计算基线。`xs` 只在两端连线时使用；ALS 按等间距处理（与常见实现相同）。
pub fn baseline(xs: &[f64], ys: &[f64], method: BaselineMethod) -> Vec<f64> {
    let n = ys.len();
    if n == 0 {
        return vec![];
    }
    match method {
        BaselineMethod::Line => {
            let (x0, x1, y0, y1) = (xs[0], xs[n - 1], ys[0], ys[n - 1]);
            xs.iter()
                .map(|x| {
                    if x1 == x0 {
                        y0
                    } else {
                        y0 + (y1 - y0) * (x - x0) / (x1 - x0)
                    }
                })
                .collect()
        }
        BaselineMethod::Als { lambda, p } => als(ys, lambda, p, 10),
    }
}

/// 非对称最小二乘基线：反复求解 (W + λDᵀD) z = W y，D 为二阶差分；
/// 数据高于基线的点权重为 p，其余为 1 − p。
pub fn als(ys: &[f64], lambda: f64, p: f64, iterations: usize) -> Vec<f64> {
    let n = ys.len();
    if n < 3 {
        return ys.to_vec();
    }
    // λDᵀD 的三条带：主对角线、上一条、上两条
    let (mut d0, mut d1, mut d2) = (vec![0.0; n], vec![0.0; n], vec![0.0; n]);
    for r in 0..n - 2 {
        let c = [1.0, -2.0, 1.0];
        for a in 0..3 {
            d0[r + a] += lambda * c[a] * c[a];
            if a < 2 {
                d1[r + a] += lambda * c[a] * c[a + 1];
            }
        }
        d2[r] += lambda * c[0] * c[2];
    }
    let mut w = vec![1.0; n];
    let mut z = ys.to_vec();
    for _ in 0..iterations {
        let diag: Vec<f64> = (0..n).map(|i| d0[i] + w[i]).collect();
        let rhs: Vec<f64> = (0..n).map(|i| w[i] * ys[i]).collect();
        z = solve_penta(&diag, &d1, &d2, &rhs);
        let next: Vec<f64> = (0..n).map(|i| if ys[i] > z[i] { p } else { 1.0 - p }).collect();
        if next == w {
            break;
        }
        w = next;
    }
    z
}

/// 对称正定五对角方程组（主对角线 `a`、上一条 `b`、上两条 `c`）的 LDLᵀ 分解求解。
fn solve_penta(a: &[f64], b: &[f64], c: &[f64], rhs: &[f64]) -> Vec<f64> {
    let n = a.len();
    // L 的两条下带 l1[i] = L[i][i−1]，l2[i] = L[i][i−2]；D 为 dd
    let (mut dd, mut l1, mut l2) = (vec![0.0; n], vec![0.0; n], vec![0.0; n]);
    for i in 0..n {
        if i >= 2 {
            l2[i] = c[i - 2] / dd[i - 2];
        }
        if i >= 1 {
            let mut v = b[i - 1];
            if i >= 2 {
                v -= l2[i] * dd[i - 2] * l1[i - 1];
            }
            l1[i] = v / dd[i - 1];
        }
        let mut v = a[i];
        if i >= 1 {
            v -= l1[i] * l1[i] * dd[i - 1];
        }
        if i >= 2 {
            v -= l2[i] * l2[i] * dd[i - 2];
        }
        dd[i] = v;
    }
    let mut y = vec![0.0; n];
    for i in 0..n {
        let mut v = rhs[i];
        if i >= 1 {
            v -= l1[i] * y[i - 1];
        }
        if i >= 2 {
            v -= l2[i] * y[i - 2];
        }
        y[i] = v;
    }
    let mut x = vec![0.0; n];
    for i in (0..n).rev() {
        let mut v = y[i] / dd[i];
        if i + 1 < n {
            v -= l1[i + 1] * x[i + 1];
        }
        if i + 2 < n {
            v -= l2[i + 2] * x[i + 2];
        }
        x[i] = v;
    }
    x
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn als_matches_dense_numpy_reference() {
        let ys: Vec<f64> = (0..120)
            .map(|i| {
                let i = i as f64;
                0.5 + 0.002 * (i - 30.0).powi(2) / 10.0
                    + 3.0 * (-((i - 40.0) / 4.0).powi(2)).exp()
                    + 2.0 * (-((i - 85.0) / 6.0).powi(2)).exp()
                    + 0.02 * (1.3 * i).sin()
            })
            .collect();
        let xs: Vec<f64> = (0..120).map(|i| i as f64).collect();
        let z = baseline(&xs, &ys, BaselineMethod::Als { lambda: 1e4, p: 0.01 });
        // numpy.linalg.solve 稠密求解 (W + λDᵀD) z = W y，迭代 10 次
        for (k, want) in [
            (0, 0.38882103056535994),
            (20, 0.45578717534922497),
            (40, 0.5411093498021738),
            (60, 0.7002667330345218),
            (85, 1.1293892233508571),
            (119, 1.8516478571066899),
        ] {
            assert!((z[k] - want).abs() < 1e-9 * want, "{k}: {} vs {want}", z[k]);
        }
        // 两端连线
        let line = baseline(&[0.0, 1.0, 3.0], &[1.0, 5.0, 4.0], BaselineMethod::Line);
        assert_eq!(line, [1.0, 2.0, 4.0]);
    }

    #[test]
    fn pentadiagonal_solver_matches_dense_solution() {
        // A = tridiag + 五对角，解 x = [1, 2, 3, 4, 5]
        let a = [6.0, 7.0, 8.0, 7.0, 6.0];
        let b = [-2.0, -1.5, 1.0, -2.5];
        let c = [0.5, 1.0, -0.5];
        let x = [1.0, 2.0, 3.0, 4.0, 5.0];
        let mut rhs = [0.0; 5];
        for i in 0..5 {
            rhs[i] += a[i] * x[i];
            if i + 1 < 5 {
                rhs[i] += b[i] * x[i + 1];
                rhs[i + 1] += b[i] * x[i];
            }
            if i + 2 < 5 {
                rhs[i] += c[i] * x[i + 2];
                rhs[i + 2] += c[i] * x[i];
            }
        }
        for (got, want) in solve_penta(&a, &b, &c, &rhs).iter().zip(x) {
            assert!((got - want).abs() < 1e-12, "{got} vs {want}");
        }
    }
}
