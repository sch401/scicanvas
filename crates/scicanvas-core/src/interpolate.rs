//! 插值与外推：线性、自然三次样条（与样条图线同一实现）、Akima（与 SciPy `Akima1DInterpolator` 一致）。
//! 超出数据范围时沿用端点所在一段的多项式外推。

use crate::spline;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InterpMethod {
    Linear,
    Spline,
    Akima,
}

impl InterpMethod {
    pub const ALL: [InterpMethod; 3] = [InterpMethod::Linear, InterpMethod::Spline, InterpMethod::Akima];

    pub fn label(self) -> &'static str {
        match self {
            InterpMethod::Linear => "线性",
            InterpMethod::Spline => "三次样条",
            InterpMethod::Akima => "Akima",
        }
    }
}

/// 去掉非有限值、按 X 排序；X 重复或有效点少于 2 个时报错。
fn prepare(xs: &[f64], ys: &[f64]) -> Result<Vec<[f64; 2]>, String> {
    let mut pts: Vec<[f64; 2]> = xs
        .iter()
        .zip(ys)
        .filter(|(x, y)| x.is_finite() && y.is_finite())
        .map(|(x, y)| [*x, *y])
        .collect();
    pts.sort_by(|a, b| a[0].total_cmp(&b[0]));
    if pts.len() < 2 {
        return Err("插值至少需要 2 个有效数据点".into());
    }
    if pts.windows(2).any(|w| w[1][0] == w[0][0]) {
        return Err("X 有重复值，无法插值；请先合并或去掉重复的 X".into());
    }
    Ok(pts)
}

/// 包含 `x` 的区间序号；超出范围时取端点所在的一段，用于外推。
fn interval(pts: &[[f64; 2]], x: f64) -> usize {
    pts.partition_point(|p| p[0] <= x).clamp(1, pts.len() - 1) - 1
}

/// Akima 各节点处的一阶导数。
fn akima_slopes(pts: &[[f64; 2]]) -> Vec<f64> {
    let n = pts.len();
    // m[k + 2] 是第 k 段的斜率，两端各外延两段
    let mut m = vec![0.0; n + 3];
    for k in 0..n - 1 {
        m[k + 2] = (pts[k + 1][1] - pts[k][1]) / (pts[k + 1][0] - pts[k][0]);
    }
    m[1] = 2.0 * m[2] - m[3];
    m[0] = 2.0 * m[1] - m[2];
    m[n + 1] = 2.0 * m[n] - m[n - 1];
    m[n + 2] = 2.0 * m[n + 1] - m[n];
    let dm: Vec<f64> = m.windows(2).map(|w| (w[1] - w[0]).abs()).collect();
    let f12: Vec<f64> = (0..n).map(|i| dm[i + 2] + dm[i]).collect();
    let big = f12.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    (0..n)
        .map(|i| {
            if f12[i] > 1e-9 * big {
                (dm[i + 2] * m[i + 1] + dm[i] * m[i + 2]) / f12[i]
            } else {
                0.5 * (m[i + 3] + m[i])
            }
        })
        .collect()
}

/// 在 `at` 的各个 X 处求插值；数据可以无序，会先按 X 排序。
pub fn interpolate(xs: &[f64], ys: &[f64], method: InterpMethod, at: &[f64]) -> Result<Vec<f64>, String> {
    let pts = prepare(xs, ys)?;
    // 两个点时三种方法都是直线
    let method = if pts.len() == 2 { InterpMethod::Linear } else { method };
    let out: Vec<f64> = match method {
        InterpMethod::Linear => at
            .iter()
            .map(|&x| {
                let i = interval(&pts, x);
                let [x0, y0] = pts[i];
                let [x1, y1] = pts[i + 1];
                y0 + (y1 - y0) * (x - x0) / (x1 - x0)
            })
            .collect(),
        InterpMethod::Spline => {
            let second = spline::natural_second_derivatives(&pts)?;
            at.iter()
                .map(|&x| {
                    let i = interval(&pts, x);
                    spline::segment(&pts, &second, i, (x - pts[i][0]) / (pts[i + 1][0] - pts[i][0]))
                })
                .collect()
        }
        InterpMethod::Akima => {
            let t = akima_slopes(&pts);
            at.iter()
                .map(|&x| {
                    let i = interval(&pts, x);
                    let [x0, y0] = pts[i];
                    let h = pts[i + 1][0] - x0;
                    let m = (pts[i + 1][1] - y0) / h;
                    let c2 = (3.0 * m - 2.0 * t[i] - t[i + 1]) / h;
                    let c3 = (t[i] + t[i + 1] - 2.0 * m) / (h * h);
                    let d = x - x0;
                    y0 + d * (t[i] + d * (c2 + d * c3))
                })
                .collect()
        }
    };
    if out.iter().any(|v| !v.is_finite()) {
        return Err("插值结果超出可表示范围".into());
    }
    Ok(out)
}

/// `from..=to` 上等间距的 `n` 个点（至少 2 个）。
pub fn grid(from: f64, to: f64, n: usize) -> Vec<f64> {
    let n = n.max(2);
    (0..n).map(|i| from + (to - from) * i as f64 / (n - 1) as f64).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    // 数据故意打乱顺序；参考值来自 SciPy 1.17.1：interp1d(fill_value="extrapolate")、
    // CubicSpline(bc_type="natural")、Akima1DInterpolator(extrapolate=True)。
    const X: [f64; 6] = [3.0, 0.0, 6.0, 1.0, 4.5, 2.5];
    const Y: [f64; 6] = [1.9, 1.0, 2.4, 2.2, 3.1, 0.7];
    const AT: [f64; 7] = [-0.5, 0.3, 1.7, 2.75, 4.0, 5.2, 6.4];

    fn close(got: &[f64], want: &[f64]) {
        for (g, w) in got.iter().zip(want) {
            assert!((g - w).abs() <= 1e-12 * w.abs().max(1.0), "{got:?} vs {want:?}");
        }
    }

    #[test]
    fn matches_scipy_inside_and_when_extrapolating() {
        let cases = [
            (
                InterpMethod::Linear,
                [0.4, 1.36, 1.5, 1.3, 2.7, 2.7733333333333334, 2.213333333333333],
            ),
            (
                InterpMethod::Spline,
                [
                    0.0980683506686475,
                    1.5798062407132243,
                    1.1916539100764956,
                    1.2348192174343735,
                    3.1151340047328153,
                    2.842571041769853,
                    2.1683933300313685,
                ],
            ),
            (
                InterpMethod::Akima,
                [
                    -0.5133928571428573,
                    1.57615,
                    1.4230128654970757,
                    1.2833834586466166,
                    2.905359911406423,
                    2.995114556416882,
                    1.8824944013781217,
                ],
            ),
        ];
        for (method, want) in cases {
            close(&interpolate(&X, &Y, method, &AT).unwrap(), &want);
        }
        let three = interpolate(
            &[0.0, 1.0, 2.0],
            &[0.0, 1.0, 0.5],
            InterpMethod::Akima,
            &[0.5, 1.5, 2.5],
        )
        .unwrap();
        close(&three, &[0.6875, 0.9375, -0.3125]);
    }

    #[test]
    fn nodes_are_reproduced_and_two_points_give_a_line() {
        for method in InterpMethod::ALL {
            let got = interpolate(&X, &Y, method, &X).unwrap();
            close(&got, &Y);
            let line = interpolate(&[0.0, 2.0], &[1.0, 5.0], method, &[-1.0, 1.0, 3.0]).unwrap();
            close(&line, &[-1.0, 3.0, 7.0]);
        }
    }

    #[test]
    fn invalid_input_is_reported() {
        assert!(interpolate(&[1.0], &[1.0], InterpMethod::Linear, &[1.0]).is_err());
        assert!(
            interpolate(&[1.0, 1.0, 2.0], &[1.0, 2.0, 3.0], InterpMethod::Spline, &[1.5])
                .unwrap_err()
                .contains("重复")
        );
        // 非有限值被跳过，剩下的点照常插值
        let got = interpolate(&[0.0, f64::NAN, 2.0], &[0.0, 5.0, 2.0], InterpMethod::Linear, &[1.0]).unwrap();
        close(&got, &[1.0]);
        assert_eq!(grid(0.0, 1.0, 1), [0.0, 1.0]);
        assert_eq!(grid(-1.0, 1.0, 5), [-1.0, -0.5, 0.0, 0.5, 1.0]);
    }
}
