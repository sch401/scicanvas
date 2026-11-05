//! 坐标轴范围与刻度：自动范围会扩展到整刻度，这和 Origin 的“重新调整刻度”一致。

use crate::model::round_sig;
mod scales;
pub use scales::{axis_scaled, AxisScale};

#[derive(Clone, Debug, PartialEq)]
pub struct Axis {
    pub lo: f64,
    pub hi: f64,
    pub ticks: Vec<f64>,
    pub minor: Vec<f64>,
    pub scale: AxisScale,
}

impl Axis {
    pub fn is_log(&self) -> bool {
        self.scale.is_log()
    }
    pub fn frac(&self, v: f64) -> f64 {
        (self.scale.forward(v) - self.scale.forward(self.lo))
            / (self.scale.forward(self.hi) - self.scale.forward(self.lo))
    }
    pub fn value(&self, f: f64) -> f64 {
        self.scale
            .inverse(self.scale.forward(self.lo) + f * (self.scale.forward(self.hi) - self.scale.forward(self.lo)))
    }
    pub fn clip_domain(&self, v: f64) -> f64 {
        if self.scale == AxisScale::Log10 {
            v.max(self.lo)
        } else if self.scale != AxisScale::Linear {
            v.clamp(self.lo, self.hi)
        } else {
            v
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
            scale: AxisScale::Log10,
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
            scale: AxisScale::Linear,
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
        scale: AxisScale::Linear,
    }
}

/// 日期刻度的时间单位，决定标签写法。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DateUnit {
    Second,
    Minute,
    Hour,
    Day,
    Month,
    Year,
}

const SECOND: f64 = 1000.0;
const MINUTE: f64 = 60.0 * SECOND;
const HOUR: f64 = 60.0 * MINUTE;
const DAY: f64 = 24.0 * HOUR;

/// 日期轴（值为 UTC 毫秒）：从秒到年挑一个使刻度间隔不超过 5 个（月、年为 6 个）的整步长；
/// 秒、分、时、日按固定间隔对齐到 UTC，月、年按日历取整。`auto` 时把范围扩到整步长。
pub fn date_axis(lo: f64, hi: f64, auto: bool) -> (Axis, DateUnit) {
    let (lo, hi) = if hi > lo {
        (lo, hi)
    } else {
        (lo - DAY / 2.0, lo + DAY / 2.0)
    };
    let span = hi - lo;
    let fixed: [(f64, DateUnit); 21] = [
        (SECOND, DateUnit::Second),
        (2.0 * SECOND, DateUnit::Second),
        (5.0 * SECOND, DateUnit::Second),
        (10.0 * SECOND, DateUnit::Second),
        (15.0 * SECOND, DateUnit::Second),
        (30.0 * SECOND, DateUnit::Second),
        (MINUTE, DateUnit::Minute),
        (2.0 * MINUTE, DateUnit::Minute),
        (5.0 * MINUTE, DateUnit::Minute),
        (10.0 * MINUTE, DateUnit::Minute),
        (15.0 * MINUTE, DateUnit::Minute),
        (30.0 * MINUTE, DateUnit::Minute),
        (HOUR, DateUnit::Hour),
        (2.0 * HOUR, DateUnit::Hour),
        (3.0 * HOUR, DateUnit::Hour),
        (6.0 * HOUR, DateUnit::Hour),
        (12.0 * HOUR, DateUnit::Hour),
        (DAY, DateUnit::Day),
        (2.0 * DAY, DateUnit::Day),
        (7.0 * DAY, DateUnit::Day),
        (14.0 * DAY, DateUnit::Day),
    ];
    let make = |lo: f64, hi: f64, ticks: Vec<f64>| {
        let minor = ticks
            .windows(2)
            .map(|w| (w[0] + w[1]) / 2.0)
            .filter(|m| *m > lo && *m < hi)
            .collect();
        Axis {
            lo,
            hi,
            ticks,
            minor,
            scale: AxisScale::Linear,
        }
    };
    if let Some(&(step, unit)) = fixed.iter().find(|(step, _)| span / step <= 5.0) {
        let (a, b) = if auto {
            ((lo / step).floor() * step, (hi / step).ceil() * step)
        } else {
            (lo, hi)
        };
        let first = (a / step - 1e-9).ceil() as i64;
        let last = (b / step + 1e-9).floor() as i64;
        return (make(a, b, (first..=last).map(|k| k as f64 * step).collect()), unit);
    }
    // 月、年：在日历上按月数步进
    use chrono::{Datelike, TimeZone, Utc};
    let to_month = |ms: f64| {
        let t = chrono::DateTime::from_timestamp_millis(ms as i64).unwrap_or_default();
        t.year() as i64 * 12 + t.month0() as i64
    };
    let month_ms = |m: i64| {
        Utc.with_ymd_and_hms(m.div_euclid(12) as i32, m.rem_euclid(12) as u32 + 1, 1, 0, 0, 0)
            .single()
            .map_or(f64::NAN, |t| t.timestamp_millis() as f64)
    };
    let months = (to_month(hi) - to_month(lo) + 1).max(1);
    let steps = [1, 2, 3, 6, 12, 24, 60, 120, 240, 600, 1200];
    let step = steps
        .iter()
        .copied()
        .find(|s| months / s <= 6)
        .unwrap_or(12 * ((months / 72) + 1));
    let unit = if step >= 12 { DateUnit::Year } else { DateUnit::Month };
    let m0 = to_month(lo).div_euclid(step) * step;
    let start = if month_ms(m0) < lo && !auto { m0 + step } else { m0 };
    let (a, b) = if auto {
        let end = to_month(hi) + if month_ms(to_month(hi)) < hi { 1 } else { 0 };
        let end = (end + step - 1).div_euclid(step) * step;
        (month_ms(m0), month_ms(end))
    } else {
        (lo, hi)
    };
    let mut ticks = vec![];
    let mut m = start;
    while month_ms(m) <= b + 1e-6 {
        ticks.push(month_ms(m));
        m += step;
    }
    (make(a, b, ticks), unit)
}

/// 日期刻度标签（UTC）。
pub fn format_date_tick(ms: f64, unit: DateUnit) -> String {
    let Some(t) = chrono::DateTime::from_timestamp_millis(ms.round() as i64) else {
        return String::new();
    };
    let pattern = match unit {
        DateUnit::Year => "%Y",
        DateUnit::Month => "%Y-%m",
        DateUnit::Day => "%Y-%m-%d",
        DateUnit::Hour | DateUnit::Minute => "%m-%d %H:%M",
        DateUnit::Second => "%H:%M:%S",
    };
    t.format(pattern).to_string()
}

/// 一个坐标轴的刻度标签写法：数字格式（Auto 为默认写法）、日期或分类（第 k 个类别在 k 处）。
#[derive(Clone, Debug, PartialEq)]
pub enum TickLabel {
    NaturalLog,
    Number(crate::format::NumberFormat),
    /// 日期：时间单位与时区偏移（毫秒）。
    Date(DateUnit, f64),
    Category(Vec<String>),
}

impl TickLabel {
    pub fn text(&self, v: f64) -> String {
        match self {
            TickLabel::NaturalLog => format!("e^{}", v.ln().round() as i32),
            TickLabel::Number(f) => f.apply(v).unwrap_or_else(|| format_tick(v)),
            TickLabel::Date(unit, offset) => format_date_tick(v + offset, *unit),
            TickLabel::Category(names) => {
                let k = v.round();
                if (v - k).abs() < 1e-9 && k >= 1.0 {
                    names.get(k as usize - 1).cloned().unwrap_or_default()
                } else {
                    String::new()
                }
            }
        }
    }
}

/// 分类轴：k 个类别放在 1…k，范围 0.5…k + 0.5，每个类别一个刻度、没有次刻度。
pub fn category_axis(count: usize) -> Axis {
    let k = count.max(1);
    Axis {
        lo: 0.5,
        hi: k as f64 + 0.5,
        ticks: (1..=k).map(|i| i as f64).collect(),
        minor: vec![],
        scale: AxisScale::Linear,
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
    if !v.is_finite() {
        return v.to_string();
    }
    if v == 0.0 {
        return "0".to_string();
    }
    let a = v.abs();
    if !(1e-3..1e5).contains(&a) {
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
        assert_eq!(format_tick(1e-3), "0.001");
        assert_eq!(format_tick(-1e5), "-10⁵");
        assert_eq!(format_tick(f64::NAN), "NaN");
        assert_eq!(format_tick(f64::INFINITY), "inf");
        assert_eq!(format_tick(f64::NEG_INFINITY), "-inf");
    }

    fn ms(text: &str) -> f64 {
        chrono::NaiveDateTime::parse_from_str(text, "%Y-%m-%d %H:%M:%S")
            .unwrap()
            .and_utc()
            .timestamp_millis() as f64
    }

    #[test]
    fn date_axes_pick_calendar_steps_and_labels() {
        let labels = |a: &Axis, u: DateUnit| a.ticks.iter().map(|t| format_date_tick(*t, u)).collect::<Vec<_>>();
        // 3 天：按日，对齐到 UTC 零点
        let (a, u) = date_axis(ms("2026-10-18 06:00:00"), ms("2026-10-20 20:00:00"), true);
        assert_eq!(u, DateUnit::Day);
        assert_eq!(labels(&a, u), ["2026-10-18", "2026-10-19", "2026-10-20", "2026-10-21"]);
        assert_eq!((a.lo, a.hi), (ms("2026-10-18 00:00:00"), ms("2026-10-21 00:00:00")));
        // 10 小时：每 2 小时
        let (a, u) = date_axis(ms("2026-10-18 01:10:00"), ms("2026-10-18 11:00:00"), true);
        assert_eq!(u, DateUnit::Hour);
        assert_eq!(labels(&a, u).first().unwrap(), "10-18 00:00");
        assert!(a.ticks.windows(2).all(|w| w[1] - w[0] == 2.0 * HOUR));
        // 一年多：每 3 个月，落在 1/4/7/10 月 1 日
        let (a, u) = date_axis(ms("2025-08-15 00:00:00"), ms("2026-10-01 00:00:00"), true);
        assert_eq!(u, DateUnit::Month);
        assert_eq!(
            labels(&a, u),
            ["2025-07", "2025-10", "2026-01", "2026-04", "2026-07", "2026-10"]
        );
        // 三十年：每 5 年
        let (a, u) = date_axis(ms("1996-03-01 00:00:00"), ms("2026-10-01 00:00:00"), true);
        assert_eq!(u, DateUnit::Year);
        assert_eq!(
            labels(&a, u),
            ["1995", "2000", "2005", "2010", "2015", "2020", "2025", "2030"]
        );
        // 手动范围：不扩展，只取范围内的整刻度
        let (a, u) = date_axis(ms("2026-10-18 06:00:00"), ms("2026-10-20 20:00:00"), false);
        assert_eq!((a.lo, labels(&a, u).len()), (ms("2026-10-18 06:00:00"), 2));
    }

    #[test]
    fn tick_labels_follow_number_formats() {
        use crate::format::NumberFormat;
        assert_eq!(TickLabel::Number(NumberFormat::Auto).text(2e-4), "2×10⁻⁴");
        assert_eq!(TickLabel::Number(NumberFormat::Decimal { places: 2 }).text(1.5), "1.50");
        assert_eq!(TickLabel::Number(NumberFormat::Percent { places: 0 }).text(0.25), "25%");
        assert_eq!(
            TickLabel::Number(NumberFormat::Engineering { places: 1 }).text(0.0012),
            "1.2m"
        );
    }
}
