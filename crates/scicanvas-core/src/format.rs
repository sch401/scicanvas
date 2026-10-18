//! 列的显示格式（F032）：只决定工作表里怎么显示，不改变存储的数值和原始写法；
//! 编辑、复制、导出 CSV 仍使用原值。

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// 数值显示方式。小数位 `places` 为小数点后的位数。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(
    feature = "serde",
    derive(Serialize, Deserialize),
    serde(tag = "kind", rename_all = "snake_case")
)]
pub enum NumberFormat {
    /// 按输入的原样显示。
    #[default]
    Auto,
    /// 固定小数位：1234.5 → 1234.50
    Decimal { places: u8 },
    /// 科学计数：1234.5 → 1.23E3
    Scientific { places: u8 },
    /// 工程单位（SI 词头，指数为 3 的倍数）：0.0012 → 1.20m，45600 → 45.60k
    Engineering { places: u8 },
    /// 百分比：0.256 → 25.6%
    Percent { places: u8 },
}

impl NumberFormat {
    pub const KINDS: [&'static str; 5] = ["自动（原样）", "固定小数位", "科学计数", "工程单位", "百分比"];

    pub fn index(self) -> usize {
        match self {
            Self::Auto => 0,
            Self::Decimal { .. } => 1,
            Self::Scientific { .. } => 2,
            Self::Engineering { .. } => 3,
            Self::Percent { .. } => 4,
        }
    }

    pub fn places(self) -> u8 {
        match self {
            Self::Auto => 2,
            Self::Decimal { places }
            | Self::Scientific { places }
            | Self::Engineering { places }
            | Self::Percent { places } => places,
        }
    }

    /// 按序号（与 `KINDS` 对应）和小数位构造。
    pub fn from_index(index: usize, places: u8) -> Self {
        let places = places.min(MAX_PLACES);
        match index {
            1 => Self::Decimal { places },
            2 => Self::Scientific { places },
            3 => Self::Engineering { places },
            4 => Self::Percent { places },
            _ => Self::Auto,
        }
    }

    /// 格式化一个数；`Auto` 返回 None，表示沿用原始写法。
    pub fn apply(self, v: f64) -> Option<String> {
        if !v.is_finite() {
            return match self {
                Self::Auto => None,
                _ => Some(if v.is_nan() {
                    "NaN".into()
                } else if v > 0.0 {
                    "∞".into()
                } else {
                    "-∞".into()
                }),
            };
        }
        let p = usize::from(self.places().min(MAX_PLACES));
        Some(match self {
            Self::Auto => return None,
            Self::Decimal { .. } => clean_zero(format!("{v:.p$}")),
            Self::Percent { .. } => format!("{}%", clean_zero(format!("{:.p$}", v * 100.0))),
            Self::Scientific { .. } => scientific(v, p),
            Self::Engineering { .. } => engineering(v, p),
        })
    }
}

/// 小数位上限，超过 f64 的有效精度没有意义。
pub const MAX_PLACES: u8 = 15;

/// 避免显示 “-0.00”。
fn clean_zero(s: String) -> String {
    if s.starts_with('-') && s[1..].chars().all(|c| c == '0' || c == '.') {
        s[1..].to_string()
    } else {
        s
    }
}

fn scientific(v: f64, p: usize) -> String {
    // Rust 的 {:e} 给出 1.23e3 / 1.23e-3，按 Origin 习惯写成大写 E
    let s = format!("{v:.p$e}");
    let (mantissa, exp) = s.split_once('e').unwrap_or((&s, "0"));
    format!("{}E{}", clean_zero(mantissa.to_string()), exp)
}

fn engineering(v: f64, p: usize) -> String {
    const PREFIXES: [(i32, &str); 17] = [
        (-24, "y"),
        (-21, "z"),
        (-18, "a"),
        (-15, "f"),
        (-12, "p"),
        (-9, "n"),
        (-6, "μ"),
        (-3, "m"),
        (0, ""),
        (3, "k"),
        (6, "M"),
        (9, "G"),
        (12, "T"),
        (15, "P"),
        (18, "E"),
        (21, "Z"),
        (24, "Y"),
    ];
    if v == 0.0 {
        return format!("{:.p$}", 0.0);
    }
    let mut exp = (v.abs().log10() / 3.0).floor() as i32 * 3;
    let mut mantissa = v / 10f64.powi(exp);
    // 四舍五入后进位到 1000 时换下一个词头：999.96 → 1.00k
    if format!("{:.p$}", mantissa.abs()).parse::<f64>().unwrap_or(0.0) >= 1000.0 {
        exp += 3;
        mantissa = v / 10f64.powi(exp);
    }
    match PREFIXES.iter().find(|(e, _)| *e == exp) {
        Some((_, prefix)) => format!("{}{prefix}", clean_zero(format!("{mantissa:.p$}"))),
        None => scientific(v, p),
    }
}

/// 日期时间列的显示方式（UTC）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(rename_all = "snake_case"))]
pub enum DateFormat {
    /// 按输入的原样显示。
    #[default]
    Original,
    /// 2026-10-18
    Date,
    /// 2026-10-18 14:30
    DateMinute,
    /// 2026-10-18 14:30:15
    DateSecond,
    /// 2026年10月18日
    Chinese,
}

impl DateFormat {
    pub const ALL: [Self; 5] = [
        Self::Original,
        Self::Date,
        Self::DateMinute,
        Self::DateSecond,
        Self::Chinese,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Original => "原样",
            Self::Date => "2026-10-18",
            Self::DateMinute => "2026-10-18 14:30",
            Self::DateSecond => "2026-10-18 14:30:15",
            Self::Chinese => "2026年10月18日",
        }
    }

    /// 格式化 UTC 毫秒；`Original` 返回 None，表示沿用原始写法。
    pub fn apply(self, millis: i64) -> Option<String> {
        let t = chrono::DateTime::from_timestamp_millis(millis)?;
        let pattern = match self {
            Self::Original => return None,
            Self::Date => "%Y-%m-%d",
            Self::DateMinute => "%Y-%m-%d %H:%M",
            Self::DateSecond => "%Y-%m-%d %H:%M:%S",
            Self::Chinese => "%Y年%m月%d日",
        };
        Some(t.format(pattern).to_string())
    }
}

/// 一列的显示格式。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(default))]
pub struct DisplayFormat {
    pub number: NumberFormat,
    pub date: DateFormat,
}

impl DisplayFormat {
    /// 全部为“原样”时不写入项目文件，旧文件与新文件保持一致。
    pub fn is_auto(&self) -> bool {
        *self == Self::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_format_like_origin_display_options() {
        let f = |n: NumberFormat, v: f64| n.apply(v).unwrap();
        assert_eq!(NumberFormat::Auto.apply(1.5), None);
        assert_eq!(f(NumberFormat::Decimal { places: 2 }, 1234.5), "1234.50");
        assert_eq!(f(NumberFormat::Decimal { places: 0 }, 2.5), "2");
        assert_eq!(f(NumberFormat::Decimal { places: 2 }, -0.001), "0.00");
        assert_eq!(f(NumberFormat::Scientific { places: 2 }, 1234.5), "1.23E3");
        assert_eq!(f(NumberFormat::Scientific { places: 3 }, -0.000123), "-1.230E-4");
        assert_eq!(f(NumberFormat::Scientific { places: 1 }, 0.0), "0.0E0");
        assert_eq!(f(NumberFormat::Engineering { places: 2 }, 0.0012), "1.20m");
        assert_eq!(f(NumberFormat::Engineering { places: 2 }, 45600.0), "45.60k");
        assert_eq!(f(NumberFormat::Engineering { places: 1 }, -3.3e-7), "-330.0n");
        assert_eq!(f(NumberFormat::Engineering { places: 2 }, 999.996), "1.00k");
        assert_eq!(f(NumberFormat::Engineering { places: 2 }, 5.0), "5.00");
        assert_eq!(f(NumberFormat::Engineering { places: 2 }, 1e30), "1.00E30");
        assert_eq!(f(NumberFormat::Percent { places: 1 }, 0.256), "25.6%");
        assert_eq!(f(NumberFormat::Decimal { places: 2 }, f64::NAN), "NaN");
        assert_eq!(f(NumberFormat::Decimal { places: 2 }, f64::NEG_INFINITY), "-∞");
        assert_eq!(
            NumberFormat::from_index(1, 40),
            NumberFormat::Decimal { places: MAX_PLACES }
        );
    }

    #[test]
    fn dates_format_in_utc() {
        // 2026-10-18T14:30:15Z
        let ms = 1_792_333_815_000;
        assert_eq!(DateFormat::Original.apply(ms), None);
        assert_eq!(DateFormat::Date.apply(ms).unwrap(), "2026-10-18");
        assert_eq!(DateFormat::DateMinute.apply(ms).unwrap(), "2026-10-18 14:30");
        assert_eq!(DateFormat::DateSecond.apply(ms).unwrap(), "2026-10-18 14:30:15");
        assert_eq!(DateFormat::Chinese.apply(ms).unwrap(), "2026年10月18日");
    }
}
