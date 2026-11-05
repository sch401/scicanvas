use super::{baseline, BaselineMethod};
use crate::{
    analysis,
    interpolate::{self, InterpMethod},
};
pub fn with_anchors(xs: &[f64], ys: &[f64], method: BaselineMethod, anchors: &[f64]) -> Result<Vec<f64>, String> {
    match method {
        BaselineMethod::Line | BaselineMethod::Als { .. } => Ok(baseline(xs, ys, method)),
        BaselineMethod::Anchors { count, spline } => {
            let pts = interpolate::prepare(xs, ys)?;
            let (lo, hi) = (pts[0][0], pts[pts.len() - 1][0]);
            let (ax, ay) = if anchors.is_empty() {
                if count < 2 {
                    return Err("自动锚点至少分成 2 段".into());
                }
                let mut selected = vec![pts[0], pts[pts.len() - 1]];
                for bin in 0..count {
                    let a = lo + (hi - lo) * f64::from(bin) / f64::from(count);
                    let b = lo + (hi - lo) * f64::from(bin + 1) / f64::from(count);
                    if let Some(point) = pts
                        .iter()
                        .filter(|p| p[0] >= a && p[0] <= b)
                        .min_by(|a, b| a[1].total_cmp(&b[1]))
                    {
                        selected.push(*point);
                    }
                }
                selected.sort_by(|a, b| a[0].total_cmp(&b[0]));
                selected.dedup_by(|a, b| a[0] == b[0]);
                selected
                    .into_iter()
                    .map(|p| (p[0], p[1]))
                    .unzip::<_, _, Vec<_>, Vec<_>>()
            } else {
                if anchors.iter().any(|x| !x.is_finite()) {
                    return Err("锚点 X 必须是有限数".into());
                }
                let mut ax = anchors.to_vec();
                ax.sort_by(f64::total_cmp);
                ax.dedup();
                let ay = interpolate::interpolate(xs, ys, InterpMethod::Linear, &ax)?;
                (ax, ay)
            };
            interpolate::interpolate(
                &ax,
                &ay,
                if spline {
                    InterpMethod::Spline
                } else {
                    InterpMethod::Linear
                },
                xs,
            )
        }
        BaselineMethod::Polynomial { order, iterations } => {
            if iterations == 0 {
                return Err("多项式基线迭代数至少为 1".into());
            }
            let mut clipped = ys.to_vec();
            let mut result = ys.to_vec();
            for _ in 0..iterations {
                let fit = analysis::polyfit(xs, &clipped, usize::from(order)).ok_or("多项式阶数过高或有效数据不足")?;
                result = xs.iter().map(|x| fit.eval(*x)).collect();
                let next: Vec<_> = clipped.iter().zip(&result).map(|(y, fit)| y.min(*fit)).collect();
                let change = next
                    .iter()
                    .zip(&clipped)
                    .map(|(a, b)| (a - b).powi(2))
                    .sum::<f64>()
                    .sqrt();
                let scale = clipped.iter().map(|y| y * y).sum::<f64>().sqrt();
                clipped = next;
                if change <= 1e-6 * scale.max(f64::MIN_POSITIVE) {
                    break;
                }
            }
            Ok(result)
        }
    }
}
