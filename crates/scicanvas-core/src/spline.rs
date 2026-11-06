//! 自然三次样条：两端二阶导数为零，三对角求解，不修改输入点。
pub fn natural(points: &[[f64; 2]], subdivisions: usize) -> Result<Vec<[f64; 2]>, String> {
    let second = natural_second_derivatives(points)?;
    let n = points.len();
    if n < 2 {
        return Ok(points.to_vec());
    }
    let steps = subdivisions.max(1);
    let mut out = Vec::new();
    for i in 0..n - 1 {
        for j in 0..steps {
            if j == 0 {
                out.push(points[i]);
                continue;
            }
            let b = j as f64 / steps as f64;
            out.push([
                points[i][0] + b * (points[i + 1][0] - points[i][0]),
                segment(points, &second, i, b),
            ]);
        }
    }
    out.push(points[n - 1]);
    if out.iter().flatten().any(|v| !v.is_finite()) {
        return Err("样条数值超出可表示范围".into());
    }
    Ok(out)
}

/// 各节点处的二阶导数（两端为零）。X 必须有限且严格递增。
pub fn natural_second_derivatives(points: &[[f64; 2]]) -> Result<Vec<f64>, String> {
    if points.iter().flatten().any(|v| !v.is_finite()) || points.windows(2).any(|w| w[1][0] <= w[0][0]) {
        return Err("样条需要有限数值和严格递增的 X".into());
    }
    let n = points.len();
    if n < 2 {
        return Ok(vec![0.0; n]);
    }
    let h: Vec<_> = points.windows(2).map(|w| w[1][0] - w[0][0]).collect();
    let mut diagonal = vec![1.0; n];
    let mut rhs = vec![0.0; n];
    for i in 1..n - 1 {
        diagonal[i] = 2.0 * (h[i - 1] + h[i]);
        rhs[i] = 6.0 * ((points[i + 1][1] - points[i][1]) / h[i] - (points[i][1] - points[i - 1][1]) / h[i - 1]);
        if i > 1 {
            let ratio = h[i - 1] / diagonal[i - 1];
            diagonal[i] -= ratio * h[i - 1];
            rhs[i] -= ratio * rhs[i - 1];
        }
    }
    let mut second = vec![0.0; n];
    for i in (1..n - 1).rev() {
        second[i] = (rhs[i] - h[i] * second[i + 1]) / diagonal[i];
    }
    Ok(second)
}

/// 第 `i` 段上位置 `b`（0 为左端点、1 为右端点；超出 0..1 时按该段多项式外推）处的值。
pub fn segment(points: &[[f64; 2]], second: &[f64], i: usize, b: f64) -> f64 {
    let h = points[i + 1][0] - points[i][0];
    let a = 1.0 - b;
    a * points[i][1]
        + b * points[i + 1][1]
        + ((a * a * a - a) * second[i] + (b * b * b - b) * second[i + 1]) * h * h / 6.0
}

/// 两端钳位的均匀三次 B 样条：数据点作控制点，曲线经过首尾两点、逼近中间各点。
/// 节点向量 [0,0,0,0,1,…,n−4,n−3,n−3,n−3,n−3]，每个节点区间取 `subdivisions` 段；不足 4 点时退回折线。
pub fn bspline(points: &[[f64; 2]], subdivisions: usize) -> Vec<[f64; 2]> {
    let n = points.len();
    if n < 4 {
        return points.to_vec();
    }
    let spans = n - 3;
    let knot = |i: usize| -> f64 { i.saturating_sub(3).min(spans) as f64 };
    let sub = subdivisions.max(1);
    let mut out = Vec::with_capacity(spans * sub + 1);
    for step in 0..=spans * sub {
        let u = step as f64 / sub as f64;
        // 所在区间 [knot(k), knot(k+1))，k 从 3 到 n−1；末端取最后一个区间
        let k = ((u.floor() as usize) + 3).min(n - 1);
        // de Boor 递推
        let mut d: Vec<[f64; 2]> = (0..4).map(|j| points[k - 3 + j]).collect();
        for r in 1..=3 {
            for j in (r..=3).rev() {
                let i = k - 3 + j;
                let (a, b) = (knot(i), knot(i + 4 - r));
                let alpha = if b > a { (u - a) / (b - a) } else { 0.0 };
                d[j] = [
                    (1.0 - alpha) * d[j - 1][0] + alpha * d[j][0],
                    (1.0 - alpha) * d[j - 1][1] + alpha * d[j][1],
                ];
            }
        }
        out.push(d[3]);
    }
    out
}

/// 分段 Bezier：每 4 个点（相邻段共用端点）组成一段三次 Bezier，曲线经过第 1、4、7… 个点；
/// 剩余 3 点用二次、2 点用直线。每段取 `subdivisions` 段。
pub fn bezier(points: &[[f64; 2]], subdivisions: usize) -> Vec<[f64; 2]> {
    let n = points.len();
    if n < 3 {
        return points.to_vec();
    }
    let sub = subdivisions.max(1);
    let mut out = vec![points[0]];
    let mut start = 0;
    while start + 1 < n {
        let end = (start + 3).min(n - 1);
        let ctrl = &points[start..=end];
        for step in 1..=sub {
            let t = step as f64 / sub as f64;
            // de Casteljau
            let mut p = ctrl.to_vec();
            for r in 1..p.len() {
                for j in 0..p.len() - r {
                    p[j] = [
                        (1.0 - t) * p[j][0] + t * p[j + 1][0],
                        (1.0 - t) * p[j][1] + t * p[j + 1][1],
                    ];
                }
            }
            out.push(p[0]);
        }
        start = end;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const P: [[f64; 2]; 6] = [[0.0, 0.0], [1.0, 2.0], [2.0, -1.0], [3.0, 3.0], [4.0, 0.5], [5.0, 2.0]];

    fn near(a: [f64; 2], b: [f64; 2]) {
        assert!(
            (a[0] - b[0]).abs() < 1e-12 && (a[1] - b[1]).abs() < 1e-12,
            "{a:?} vs {b:?}"
        );
    }

    #[test]
    fn clamped_bspline_matches_scipy() {
        // scipy.interpolate.BSpline(t=[0,0,0,0,1,2,3,3,3,3], c=P, k=3) 在 u = 0, 0.25, 1, 1.5, 2.75, 3
        let c = bspline(&P, 4);
        assert_eq!(c.len(), 13);
        for (step, want) in [
            (0, [0.0, 0.0]),
            (1, [0.6627604166666666, 0.9205729166666666]),
            (4, [1.9166666666666665, 0.41666666666666674]),
            (6, [2.5, 1.015625]),
            (11, [4.337239583333334, 1.3274739583333335]),
            (12, [5.0, 2.0]),
        ] {
            near(c[step], want);
        }
        assert_eq!(bspline(&P[..3], 4), P[..3].to_vec());
    }

    #[test]
    fn bezier_passes_every_third_point_and_uses_lower_order_tail() {
        let c = bezier(&P, 4);
        // 三次段 P0..P3（4 个子段）+ 二次段 P3..P5（4 个子段）
        assert_eq!(c.len(), 9);
        near(c[0], P[0]);
        near(c[4], P[3]);
        near(c[8], P[5]);
        near(c[2], [1.5, 0.75]);
        // 二次段 t = 0.25：(1−t)²P3 + 2t(1−t)P4 + t²P5
        let t = 0.25;
        let q = [
            (1.0 - t) * (1.0 - t) * P[3][0] + 2.0 * t * (1.0 - t) * P[4][0] + t * t * P[5][0],
            (1.0 - t) * (1.0 - t) * P[3][1] + 2.0 * t * (1.0 - t) * P[4][1] + t * t * P[5][1],
        ];
        near(c[5], q);
    }
}
