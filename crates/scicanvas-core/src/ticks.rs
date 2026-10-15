//! 坐标轴范围与刻度：自动范围会扩展到整刻度，这和 Origin 的“重新调整刻度”一致。

use crate::model::round_sig;

#[derive(Clone, Debug, PartialEq)]
pub struct Axis {
    pub lo: f64,
    pub hi: f64,
    pub ticks: Vec<f64>,
    pub minor: Vec<f64>,
    pub log: bool,
}

impl Axis {
    /// 数据值映射到 0..1。
    pub fn frac(&self, v: f64) -> f64 {
        if self.log {
            (v.log10() - self.lo.log10()) / (self.hi.log10() - self.lo.log10())
        } else {
            (v - self.lo) / (self.hi - self.lo)
        }
    }

    /// frac 的反函数：0..1 映射回数据值。
    pub fn value(&self, f: f64) -> f64 {
        if self.log {
            10f64.powf(self.lo.log10() + f * (self.hi.log10() - self.lo.log10()))
        } else {
            self.lo + f * (self.hi - self.lo)
        }
    }
}

/// 选一个“好看”的步长（1、2、5 × 10ⁿ），返回 (下界, 上界, 步长)。
pub fn nice(mut min: f64, mut max: f64, count: usize) -> (f64, f64, f64) {
    if !min.is_finite() || !max.is_finite() {
        min = 0.0;
        max = 1.0;
    }
    if min == max {
        let d = if min.abs() > 0.0 { min.abs() * 0.1 } else { 1.0 };
        min -= d;
        max += d;
    }
    let raw = (max - min) / count.max(1) as f64;
    let mut step = 10f64.powf(raw.log10().floor());
    let e = raw / step;
    if e >= 7.5 {
        step *= 10.0;
    } else if e >= 3.5 {
        step *= 5.0;
    } else if e >= 1.5 {
        step *= 2.0;
    }
    let lo = (min / step + 1e-9).floor() * step;
    let hi = (max / step - 1e-9).ceil() * step;
    (round_sig(lo, 12), round_sig(hi, 12), step)
}

fn tick_list(lo: f64, hi: f64, step: f64) -> Vec<f64> {
    let n = ((hi - lo) / step).round().max(0.0) as usize;
    (0..=n).map(|i| round_sig(lo + i as f64 * step, 12)).collect()
}

fn finite_range(vals: &[f64]) -> Option<(f64, f64)> {
    let mut it = vals.iter().copied().filter(|v| v.is_finite());
    let first = it.next()?;
    Some(it.fold((first, first), |(a, b), v| (a.min(v), b.max(v))))
}

/// 计算坐标轴。`auto` 为假时使用给定的 `min..max`；`force_zero` 用于柱状图（范围包含 0）。
pub fn axis(vals: &[f64], auto: bool, min: f64, max: f64, log: bool, force_zero: bool) -> Axis {
    if log {
        let pos: Vec<f64> = vals.iter().copied().filter(|v| *v > 0.0).collect();
        let (mut a, mut b) = if auto {
            finite_range(&pos).unwrap_or((1.0, 10.0))
        } else {
            (min, max)
        };
        if a.partial_cmp(&0.0) != Some(std::cmp::Ordering::Greater) {
            a = 1.0;
        }
        if b.partial_cmp(&a) != Some(std::cmp::Ordering::Greater) {
            b = a * 10.0;
        }
        let (lo, hi) = if auto {
            (10f64.powf(a.log10().floor()), 10f64.powf(b.log10().ceil()))
        } else {
            (a, b)
        };
        let hi = if hi > lo { hi } else { lo * 10.0 };
        let mut ticks = Vec::new();
        let mut minor = Vec::new();
        let e0 = lo.log10().floor() as i32;
        let e1 = hi.log10().ceil() as i32;
        for e in e0..=e1 {
            let p = 10f64.powi(e);
            if p >= lo * 0.999 && p <= hi * 1.001 {
                ticks.push(p);
            }
            for k in 2..10 {
                let m = k as f64 * p;
                if m > lo && m < hi {
                    minor.push(m);
                }
            }
        }
        return Axis {
            lo,
            hi,
            ticks,
            minor,
            log: true,
        };
    }

    let (mut a, mut b) = finite_range(vals).unwrap_or((0.0, 1.0));
    if force_zero {
        a = a.min(0.0);
        b = b.max(0.0);
    }
    if !auto {
        let (a, b) = if max > min { (min, max) } else { (min, min + 1.0) };
        let (_, _, step) = nice(a, b, 5);
        let first = (a / step - 1e-9).ceil() * step;
        let last = (b / step + 1e-9).floor() * step;
        let ticks = tick_list(first, last, step);
        let mut minor: Vec<f64> = Vec::new();
        if let (Some(t0), Some(t1)) = (ticks.first(), ticks.last()) {
            let mut m = t0 - step / 2.0;
            while m <= t1 + step / 2.0 + 1e-12 {
                if m > a && m < b {
                    minor.push(m);
                }
                m += step;
            }
        }
        return Axis {
            lo: a,
            hi: b,
            ticks,
            minor,
            log: false,
        };
    }
    let (lo, hi, step) = nice(a, b, 5);
    let ticks = tick_list(lo, hi, step);
    let minor = ticks.windows(2).map(|w| (w[0] + w[1]) / 2.0).collect();
    Axis {
        lo,
        hi,
        ticks,
        minor,
        log: false,
    }
}

fn superscript(n: i32) -> String {
    n.to_string()
        .chars()
        .map(|c| match c {
            '-' => '⁻',
            '0' => '⁰',
            '1' => '¹',
            '2' => '²',
            '3' => '³',
            '4' => '⁴',
            '5' => '⁵',
            '6' => '⁶',
            '7' => '⁷',
            '8' => '⁸',
            _ => '⁹',
        })
        .collect()
}

/// 刻度标签：一般数值直接显示，很大或很小时显示为 a×10ⁿ。
pub fn format_tick(v: f64) -> String {
    if v == 0.0 {
        return "0".to_string();
    }
    let a = v.abs();
    if a >= 1e5 || a < 1e-3 {
        let e = a.log10().floor() as i32;
        let m = round_sig(v / 10f64.powi(e), 3);
        let mant = if m == 1.0 {
            String::new()
        } else if m == -1.0 {
            "-".to_string()
        } else {
            format!("{}×", m)
        };
        return format!("{}10{}", mant, superscript(e));
    }
    format!("{}", round_sig(v, 10))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nice_steps() {
        assert_eq!(nice(0.0, 20.0, 5), (0.0, 20.0, 5.0));
        assert_eq!(nice(-8.3, 9.7, 5), (-10.0, 10.0, 5.0));
        assert_eq!(nice(0.13, 1.9, 5), (0.0, 2.0, 0.5));
        assert_eq!(nice(0.12, 0.87, 5), (0.1, 0.9, 0.1));
    }

    #[test]
    fn auto_axis_extends_to_ticks() {
        let ax = axis(&[0.4, 19.6, f64::NAN], true, 0.0, 0.0, false, false);
        assert_eq!(ax.lo, 0.0);
        assert_eq!(ax.hi, 20.0);
        assert_eq!(ax.ticks, vec![0.0, 5.0, 10.0, 15.0, 20.0]);
        assert_eq!(ax.minor.len(), 4);
    }

    #[test]
    fn bar_axis_includes_zero() {
        let ax = axis(&[3.0, 7.0], true, 0.0, 0.0, false, true);
        assert_eq!(ax.lo, 0.0);
    }

    #[test]
    fn manual_axis() {
        let ax = axis(&[], false, 1.0, 9.0, false, false);
        assert_eq!((ax.lo, ax.hi), (1.0, 9.0));
        assert_eq!(ax.ticks, vec![2.0, 4.0, 6.0, 8.0]);
    }

    #[test]
    fn log_axis() {
        let ax = axis(&[3.0, 450.0, -1.0], true, 0.0, 0.0, true, false);
        assert_eq!((ax.lo, ax.hi), (1.0, 1000.0));
        assert_eq!(ax.ticks, vec![1.0, 10.0, 100.0, 1000.0]);
        assert_eq!(ax.minor.len(), 24);
        assert!((ax.frac(10.0) - 1.0 / 3.0).abs() < 1e-12);
    }

    #[test]
    fn tick_labels() {
        assert_eq!(format_tick(0.0), "0");
        assert_eq!(format_tick(2.5), "2.5");
        assert_eq!(format_tick(0.1 + 0.2), "0.3");
        assert_eq!(format_tick(100000.0), "10⁵");
        assert_eq!(format_tick(2e-4), "2×10⁻⁴");
    }
}
