//! 自然三次样条：两端二阶导数为零，三对角求解，不修改输入点。
pub fn natural(points: &[[f64; 2]], subdivisions: usize) -> Result<Vec<[f64; 2]>, String> {
    if points.iter().flatten().any(|v| !v.is_finite()) || points.windows(2).any(|w| w[1][0] <= w[0][0]) {
        return Err("样条需要有限数值和严格递增的 X".into());
    }
    let n = points.len();
    if n < 2 {
        return Ok(points.to_vec());
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
    let steps = subdivisions.max(1);
    let mut out = Vec::new();
    for i in 0..n - 1 {
        for j in 0..steps {
            if j == 0 {
                out.push(points[i]);
                continue;
            }
            let b = j as f64 / steps as f64;
            let a = 1.0 - b;
            let y = a * points[i][1]
                + b * points[i + 1][1]
                + ((a * a * a - a) * second[i] + (b * b * b - b) * second[i + 1]) * h[i] * h[i] / 6.0;
            out.push([points[i][0] + b * h[i], y]);
        }
    }
    out.push(points[n - 1]);
    if out.iter().flatten().any(|v| !v.is_finite()) {
        return Err("样条数值超出可表示范围".into());
    }
    Ok(out)
}
