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
