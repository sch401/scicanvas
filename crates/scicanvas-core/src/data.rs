//! Typed column storage. Missing values use a bitmap, independent of floating NaN.
//! Sparse lexemes preserve legacy spelling without storing a String for each number.
//! ponytail: an entirely noncanonical column still retains per-cell text; compress uniform spelling when T1.2 adds column display formats.
use std::{borrow::Cow, collections::BTreeMap};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DataKind {
    Num,
    Text,
    DateTime,
    Category,
}

impl DataKind {
    pub const ALL: [Self; 4] = [Self::Num, Self::Text, Self::DateTime, Self::Category];
    pub fn label(self) -> &'static str {
        match self {
            Self::Num => "数值",
            Self::Text => "文本",
            Self::DateTime => "日期时间（UTC）",
            Self::Category => "分类",
        }
    }
}

#[derive(Clone, Debug)]
pub enum ColumnData {
    Num {
        values: Vec<f64>,
        missing: Vec<u64>,
        lexemes: BTreeMap<usize, String>,
    },
    Text(Vec<String>),
    DateTime {
        values: Vec<i64>,
        missing: Vec<u64>,
        lexemes: BTreeMap<usize, String>,
    },
    Category {
        codes: Vec<u32>,
        levels: Vec<String>,
        ordered: bool,
    },
}

pub trait ColumnView {
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
    fn f64_at(&self, row: usize) -> Option<f64>;
    fn text_at(&self, row: usize) -> Cow<'_, str>;
}

fn number(text: &str) -> Option<f64> {
    let text = text.trim();
    if text.contains(',') {
        text.replace(',', "").parse().ok()
    } else {
        text.parse().ok()
    }
}

/// 日期时间解析，结果为 UTC 毫秒。
///
/// 接受带时区的 ISO 8601 / RFC 3339（如 `2026-10-18T14:30:00+08:00`）；年在前、用 `-` `/` `.`
/// 分隔或写成 `2026年10月18日` 的日期，可再跟空格或 `T` 和 `时:分[:秒[.小数]]`，按 UTC；
/// 以及单独的四位年份（1000–9999），取该年 1 月 1 日。
///
/// 不接受其他纯整数（避免把数值或 20261018 误当作毫秒时间戳），
/// 也不接受 `18/10/2026` 这类年份不在前、日月顺序不确定的写法。
fn timestamp(text: &str) -> Option<i64> {
    let text = text.trim();
    if let Ok(date) = chrono::DateTime::parse_from_rfc3339(text) {
        return Some(date.timestamp_millis());
    }
    let midnight = |date: chrono::NaiveDate| date.and_hms_opt(0, 0, 0).map(|d| d.and_utc().timestamp_millis());
    if text.len() == 4 && text.bytes().all(|b| b.is_ascii_digit()) {
        let year: i32 = text.parse().ok()?;
        return if year >= 1000 {
            midnight(chrono::NaiveDate::from_ymd_opt(year, 1, 1)?)
        } else {
            None
        };
    }
    // 年份必须在最前：四位数字后紧跟分隔符
    let bytes = text.as_bytes();
    if bytes.len() < 8 || !bytes[..4].iter().all(u8::is_ascii_digit) || bytes[4].is_ascii_digit() {
        return None;
    }
    let (date, time) = match text.find([' ', 'T']) {
        Some(i) => (&text[..i], text[i + 1..].trim()),
        None => (text, ""),
    };
    let date = date.replace(['年', '月', '/', '.'], "-").replace('日', "");
    let date = chrono::NaiveDate::parse_from_str(&date, "%Y-%m-%d").ok()?;
    if time.is_empty() {
        return midnight(date);
    }
    let time = chrono::NaiveTime::parse_from_str(time, "%H:%M:%S%.f")
        .or_else(|_| chrono::NaiveTime::parse_from_str(time, "%H:%M"))
        .ok()?;
    Some(date.and_time(time).and_utc().timestamp_millis())
}

/// 首个 v9 版本（51a5557）的日期列接受 UTC 毫秒整数；读取那时保存的文件时，这类原文仍按毫秒校验。
fn legacy_millis(text: &str) -> Option<i64> {
    text.trim()
        .parse::<i64>()
        .ok()
        .filter(|v| chrono::DateTime::from_timestamp_millis(*v).is_some())
}

fn missing_at(bits: &[u64], row: usize) -> bool {
    bits[row / 64] & (1 << (row % 64)) != 0
}
fn mark(bits: &mut Vec<u64>, row: usize, missing: bool) {
    bits.resize((row + 1).div_ceil(64).max(bits.len()), 0);
    let mask = 1 << (row % 64);
    if missing {
        bits[row / 64] |= mask;
    } else {
        bits[row / 64] &= !mask;
    }
}

impl Default for ColumnData {
    fn default() -> Self {
        Self::Num {
            values: vec![],
            missing: vec![],
            lexemes: BTreeMap::new(),
        }
    }
}

impl ColumnData {
    /// 日期列第 `row` 行按“墙上时间”重新解释：原文不带时区时，按 UTC 偏移 `offset_minutes` 换算成 UTC；
    /// 带时区（ISO 8601 的 Z 或 ±hh:mm）的原文保持原时刻。原文不变。
    pub fn reinterpret_local(&mut self, row: usize, offset_minutes: i32) {
        let Self::DateTime { .. } = self else { return };
        if self.f64_at(row).is_none() {
            return;
        }
        let text = self.text_at(row).into_owned();
        if chrono::DateTime::parse_from_rfc3339(text.trim()).is_ok() {
            return;
        }
        if let (Some(naive), Self::DateTime { values, .. }) = (timestamp(&text), self) {
            values[row] = naive - i64::from(offset_minutes) * 60_000;
        }
    }

    /// 分类列按显示顺序排列的类别：有序字典按字典顺序，否则按文字排序。其他类型返回 None。
    pub fn category_levels(&self) -> Option<Vec<String>> {
        let Self::Category { levels, ordered, .. } = self else {
            return None;
        };
        let mut out = levels.clone();
        if !ordered {
            out.sort();
        }
        Some(out)
    }

    /// 分类列各行在显示顺序中的位置（从 1 开始），缺失为 NaN；用作分类轴的坐标。
    pub fn category_positions(&self) -> Option<Vec<f64>> {
        let Self::Category { codes, levels, .. } = self else {
            return None;
        };
        let order = self.category_levels()?;
        let rank: Vec<f64> = levels
            .iter()
            .map(|l| order.iter().position(|o| o == l).map_or(f64::NAN, |p| p as f64 + 1.0))
            .collect();
        Some(
            codes
                .iter()
                .map(|&c| rank.get(c as usize).copied().unwrap_or(f64::NAN))
                .collect(),
        )
    }

    /// 纯文字 X 按首次出现顺序作为分类标签；含数值的旧文字列沿用数值轴，存储类型保持不变。
    pub fn plot_category_levels(&self) -> Option<Vec<String>> {
        let Self::Text(values) = self else {
            return self.category_levels();
        };
        if values.iter().all(|v| v.trim().is_empty()) || values.iter().any(|v| number(v).is_some()) {
            return None;
        }
        let mut seen = std::collections::HashSet::new();
        Some(
            values
                .iter()
                .filter(|v| !v.trim().is_empty() && seen.insert(v.as_str()))
                .cloned()
                .collect(),
        )
    }

    /// 分类与文字 X 共用分类轴坐标；空标签不参与绘图。
    pub fn plot_category_positions(&self) -> Option<Vec<f64>> {
        let Self::Text(values) = self else {
            return self.category_positions();
        };
        let levels = self.plot_category_levels()?;
        let positions: std::collections::HashMap<_, _> = levels
            .iter()
            .enumerate()
            .map(|(i, value)| (value.as_str(), i as f64 + 1.0))
            .collect();
        Some(
            values
                .iter()
                .map(|v| positions.get(v.as_str()).copied().unwrap_or(f64::NAN))
                .collect(),
        )
    }

    /// 设置类别顺序：`order` 必须正好是现有类别的一种排列。之后按此顺序显示、排序和作图。
    pub fn set_category_order(&mut self, order: &[String]) -> Result<(), String> {
        let Self::Category { codes, levels, ordered } = self else {
            return Err("只有分类列可以设置类别顺序".into());
        };
        let mut a = levels.clone();
        let mut b = order.to_vec();
        a.sort();
        b.sort();
        if a != b {
            return Err("类别顺序必须包含全部类别且不重复".into());
        }
        let remap: Vec<u32> = levels
            .iter()
            .map(|l| order.iter().position(|o| o == l).unwrap() as u32)
            .collect();
        for c in codes.iter_mut() {
            if let Some(&n) = remap.get(*c as usize) {
                *c = n;
            }
        }
        *levels = order.to_vec();
        *ordered = true;
        Ok(())
    }

    pub fn kind(&self) -> DataKind {
        match self {
            Self::Num { .. } => DataKind::Num,
            Self::Text(_) => DataKind::Text,
            Self::DateTime { .. } => DataKind::DateTime,
            Self::Category { .. } => DataKind::Category,
        }
    }
    pub fn len(&self) -> usize {
        match self {
            Self::Num { values, .. } => values.len(),
            Self::Text(v) => v.len(),
            Self::DateTime { values, .. } => values.len(),
            Self::Category { codes, .. } => codes.len(),
        }
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    pub fn f64_at(&self, row: usize) -> Option<f64> {
        if row >= self.len() {
            return None;
        }
        match self {
            Self::Num { values, missing, .. } => (!missing_at(missing, row)).then_some(values[row]),
            Self::Text(v) => (!v[row].trim().is_empty()).then(|| crate::parse_num(&v[row])),
            Self::DateTime { values, missing, .. } => (!missing_at(missing, row)).then_some(values[row] as f64),
            Self::Category { .. } => None,
        }
    }
    pub fn text_at(&self, row: usize) -> Cow<'_, str> {
        if row >= self.len() {
            return Cow::Borrowed("");
        }
        match self {
            Self::Text(v) => Cow::Borrowed(&v[row]),
            Self::Num {
                values,
                missing,
                lexemes,
            } => lexemes.get(&row).map(|s| Cow::Borrowed(s.as_str())).unwrap_or_else(|| {
                if missing_at(missing, row) {
                    Cow::Borrowed("")
                } else {
                    Cow::Owned(values[row].to_string())
                }
            }),
            Self::DateTime {
                values,
                missing,
                lexemes,
            } => lexemes.get(&row).map(|s| Cow::Borrowed(s.as_str())).unwrap_or_else(|| {
                if missing_at(missing, row) {
                    Cow::Borrowed("")
                } else {
                    Cow::Owned(
                        chrono::DateTime::from_timestamp_millis(values[row])
                            .unwrap()
                            .to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
                    )
                }
            }),
            Self::Category { codes, levels, .. } => levels
                .get(codes[row] as usize)
                .map(|s| Cow::Borrowed(s.as_str()))
                .unwrap_or(Cow::Borrowed("")),
        }
    }
    pub fn iter(&self) -> impl DoubleEndedIterator<Item = Cow<'_, str>> + ExactSizeIterator {
        (0..self.len()).map(|i| self.text_at(i))
    }
    pub fn get(&self, row: usize) -> Option<Cow<'_, str>> {
        (row < self.len()).then(|| self.text_at(row))
    }
    pub fn last(&self) -> Option<Cow<'_, str>> {
        self.len().checked_sub(1).map(|r| self.text_at(r))
    }
    pub fn accepts(&self, text: &str) -> bool {
        text.trim().is_empty()
            || match self.kind() {
                DataKind::Num => number(text).is_some(),
                DataKind::DateTime => timestamp(text).is_some(),
                _ => true,
            }
    }
    /// 新建空数值列在粘贴时按内容推断；已有数据和明确的日期/分类列沿用原校验。
    pub fn accepts_paste(&self, text: &str) -> bool {
        self.accepts(text) || (self.kind() == DataKind::Num && self.is_empty())
    }

    pub fn converted(&self, kind: DataKind) -> Result<Self, String> {
        if self.kind() == kind {
            return Ok(self.clone());
        }
        if kind == DataKind::Category {
            let mut codes = Vec::with_capacity(self.len());
            let mut levels = Vec::new();
            let mut index = std::collections::HashMap::new();
            for text in self.iter() {
                if text.is_empty() {
                    codes.push(u32::MAX);
                    continue;
                }
                let code = *index.entry(text.into_owned()).or_insert_with_key(|label| {
                    let code = u32::try_from(levels.len()).expect("category count fits memory");
                    levels.push(label.clone());
                    code
                });
                codes.push(code);
            }
            return Ok(Self::Category {
                codes,
                levels,
                ordered: false,
            });
        }
        let mut out = match kind {
            DataKind::Num => Self::default(),
            DataKind::Text => Self::Text(vec![]),
            DataKind::DateTime => Self::DateTime {
                values: vec![],
                missing: vec![],
                lexemes: BTreeMap::new(),
            },
            DataKind::Category => Self::Category {
                codes: vec![],
                levels: vec![],
                ordered: false,
            },
        };
        for row in 0..self.len() {
            let text = self.text_at(row);
            if !out.accepts(&text) {
                return Err(format!("第 {} 行无法转换为{}：{}", row + 1, kind.label(), text));
            }
            out.push(text.into_owned());
        }
        Ok(out)
    }

    /// Whole-column calculations keep the target type. Validate before replacing it.
    pub fn replaced_texts(&self, cells: Vec<String>) -> Result<Self, String> {
        match self {
            Self::Text(_) => Ok(Self::Text(cells)),
            Self::Category { levels, ordered, .. } => {
                let mut levels = levels.clone();
                let mut index: std::collections::HashMap<String, u32> =
                    levels.iter().enumerate().map(|(i, s)| (s.clone(), i as u32)).collect();
                let codes = cells
                    .into_iter()
                    .map(|text| {
                        if text.is_empty() {
                            return u32::MAX;
                        }
                        *index.entry(text).or_insert_with_key(|label| {
                            let code = u32::try_from(levels.len()).expect("category count fits memory");
                            levels.push(label.clone());
                            code
                        })
                    })
                    .collect();
                Ok(Self::Category {
                    codes,
                    levels,
                    ordered: *ordered,
                })
            }
            _ => {
                let data = Self::from(cells);
                if data.kind() == self.kind() {
                    Ok(data)
                } else {
                    data.converted(self.kind())
                }
            }
        }
    }
    /// Adaptive bulk write. Interactive edits confirm conversion before calling this.
    pub fn set_text(&mut self, row: usize, text: String) {
        if !self.accepts(&text) {
            *self = self.converted(DataKind::Text).unwrap();
        }
        while self.len() <= row {
            self.push_empty();
        }
        match self {
            Self::Text(v) => v[row] = text,
            Self::Num {
                values,
                missing,
                lexemes,
            } => {
                let value = number(&text);
                values[row] = value.unwrap_or(0.0);
                mark(missing, row, value.is_none());
                // Plain small integers already have canonical spelling, avoiding a formatting allocation.
                let digits = text.strip_prefix('-').unwrap_or(&text);
                let integer = !digits.is_empty()
                    && digits.len() <= 15
                    && digits.bytes().all(|b| b.is_ascii_digit())
                    && (digits == "0" || !digits.starts_with('0'))
                    && text != "-0";
                let canonical = (value.is_none() && text.is_empty())
                    || (value.is_some() && (integer || text == values[row].to_string()));
                if canonical {
                    lexemes.remove(&row);
                } else {
                    lexemes.insert(row, text);
                }
            }
            Self::DateTime {
                values,
                missing,
                lexemes,
            } => {
                let value = timestamp(&text);
                values[row] = value.unwrap_or(0);
                mark(missing, row, value.is_none());
                if text.is_empty() {
                    lexemes.remove(&row);
                } else {
                    lexemes.insert(row, text);
                }
            }
            Self::Category { codes, levels, .. } => {
                // ponytail: a single-cell lookup is linear in levels; cache an index if high-cardinality editing becomes common.
                if text.is_empty() {
                    codes[row] = u32::MAX;
                } else {
                    let code = levels.iter().position(|s| s == &text).unwrap_or_else(|| {
                        levels.push(text);
                        levels.len() - 1
                    });
                    codes[row] = u32::try_from(code).expect("category count fits memory");
                }
            }
        }
    }
    fn push_empty(&mut self) {
        let row = self.len();
        match self {
            Self::Num { values, missing, .. } => {
                values.push(0.0);
                mark(missing, row, true);
            }
            Self::DateTime { values, missing, .. } => {
                values.push(0);
                mark(missing, row, true);
            }
            Self::Text(v) => v.push(String::new()),
            Self::Category { codes, .. } => codes.push(u32::MAX),
        }
    }
    pub fn push(&mut self, text: String) {
        self.set_text(self.len(), text);
    }
    pub fn clear(&mut self) {
        self.truncate(0);
    }
    pub fn pop(&mut self) {
        if !self.is_empty() {
            self.truncate(self.len() - 1);
        }
    }
    pub fn truncate(&mut self, len: usize) {
        match self {
            Self::Num {
                values,
                missing,
                lexemes,
            } => {
                values.truncate(len);
                trim_aux(missing, lexemes, values.len());
            }
            Self::DateTime {
                values,
                missing,
                lexemes,
            } => {
                values.truncate(len);
                trim_aux(missing, lexemes, values.len());
            }
            Self::Text(v) => v.truncate(len),
            Self::Category { codes, .. } => codes.truncate(len),
        }
    }
    pub fn trim_empty(&mut self) {
        while self.last().is_some_and(|s| s.trim().is_empty()) {
            self.pop();
        }
    }
    /// Reordering preserves storage type, category dictionary, missing mask and spelling.
    pub fn reordered(&self, order: &[usize]) -> Self {
        match self {
            Self::Num {
                values,
                missing,
                lexemes,
            } => {
                let (missing, lexemes) = reordered_aux(missing, lexemes, values.len(), order);
                Self::Num {
                    values: order.iter().map(|i| values.get(*i).copied().unwrap_or(0.0)).collect(),
                    missing,
                    lexemes,
                }
            }
            Self::DateTime {
                values,
                missing,
                lexemes,
            } => {
                let (missing, lexemes) = reordered_aux(missing, lexemes, values.len(), order);
                Self::DateTime {
                    values: order.iter().map(|i| values.get(*i).copied().unwrap_or(0)).collect(),
                    missing,
                    lexemes,
                }
            }
            Self::Text(values) => Self::Text(
                order
                    .iter()
                    .map(|i| values.get(*i).cloned().unwrap_or_default())
                    .collect(),
            ),
            Self::Category { codes, levels, ordered } => Self::Category {
                codes: order
                    .iter()
                    .map(|i| codes.get(*i).copied().unwrap_or(u32::MAX))
                    .collect(),
                levels: levels.clone(),
                ordered: *ordered,
            },
        }
    }
    pub fn insert_empty(&mut self, at: usize, count: usize) {
        if at >= self.len() {
            return;
        }
        let mut order: Vec<usize> = (0..at).collect();
        order.extend(std::iter::repeat_n(usize::MAX, count));
        order.extend(at..self.len());
        *self = self.reordered(&order);
    }
    pub fn delete_rows(&mut self, first: usize, last: usize) {
        *self = self.reordered(&(0..self.len()).filter(|i| *i < first || *i > last).collect::<Vec<_>>());
        self.trim_empty();
    }
    pub fn estimated_bytes(&self) -> usize {
        let strings = |v: &Vec<String>| {
            v.capacity() * std::mem::size_of::<String>() + v.iter().map(String::capacity).sum::<usize>()
        };
        match self {
            Self::Text(v) => strings(v),
            Self::Num {
                values,
                missing,
                lexemes,
            } => {
                values.capacity() * 8
                    + missing.capacity() * 8
                    + lexemes.values().map(|s| s.capacity() + 64).sum::<usize>()
            }
            Self::DateTime {
                values,
                missing,
                lexemes,
            } => {
                values.capacity() * 8
                    + missing.capacity() * 8
                    + lexemes.values().map(|s| s.capacity() + 64).sum::<usize>()
            }
            Self::Category { codes, levels, .. } => codes.capacity() * 4 + strings(levels),
        }
    }
    pub fn compare_rows(&self, a: usize, b: usize, ascending: bool) -> std::cmp::Ordering {
        use std::cmp::Ordering::*;
        let (ta, tb) = if matches!(self, Self::Text(_) | Self::Category { .. }) {
            (self.text_at(a), self.text_at(b))
        } else {
            (Cow::Borrowed(""), Cow::Borrowed(""))
        };
        let blank = |row: usize, text: &str| match self {
            Self::Num { .. } | Self::DateTime { .. } => self.f64_at(row).is_none(),
            _ => text.trim().is_empty(),
        };
        match (blank(a, &ta), blank(b, &tb)) {
            (true, true) => return Equal,
            (true, false) => return Greater,
            (false, true) => return Less,
            _ => {}
        }
        let order = if let Self::Category {
            codes, ordered: true, ..
        } = self
        {
            codes[a].cmp(&codes[b])
        } else if matches!(self, Self::Category { .. }) {
            ta.cmp(&tb)
        } else {
            let (na, nb) = (self.f64_at(a).unwrap_or(f64::NAN), self.f64_at(b).unwrap_or(f64::NAN));
            match (na.is_finite(), nb.is_finite()) {
                (true, true) => na.partial_cmp(&nb).unwrap_or(Equal),
                (true, false) => return Less,
                (false, true) => return Greater,
                _ => self.text_at(a).trim().cmp(self.text_at(b).trim()),
            }
        };
        if ascending {
            order
        } else {
            order.reverse()
        }
    }
    pub fn validate(&self) -> Result<(), String> {
        let n = self.len();
        let check = |bits: &[u64], lexemes: &BTreeMap<usize, String>| {
            if bits.len() != n.div_ceil(64)
                || (!n.is_multiple_of(64) && bits.last().is_some_and(|v| *v >> (n % 64) != 0))
                || lexemes.keys().any(|r| *r >= n)
            {
                Err("列的缺失位图或原始文本索引无效".to_string())
            } else {
                Ok(())
            }
        };
        match self {
            Self::Num {
                values,
                missing,
                lexemes,
            } => {
                check(missing, lexemes)?;
                for (&row, text) in lexemes {
                    if (!missing_at(missing, row) && number(text).is_none_or(|v| v.to_bits() != values[row].to_bits()))
                        || (missing_at(missing, row) && !text.trim().is_empty())
                    {
                        return Err("数值与原始文本不一致".into());
                    }
                }
            }
            Self::DateTime {
                values,
                missing,
                lexemes,
            } => {
                check(missing, lexemes)?;
                if values
                    .iter()
                    .enumerate()
                    .any(|(i, v)| !missing_at(missing, i) && chrono::DateTime::from_timestamp_millis(*v).is_none())
                {
                    return Err("日期时间超出支持范围".into());
                }
                // 不带时区的原文按列时区存储：与 UTC 解析结果相差同一个整分钟偏移（±14 小时内）
                let mut shift: Option<i64> = None;
                for (&row, text) in lexemes {
                    let missing_here = missing_at(missing, row);
                    if missing_here {
                        if !text.trim().is_empty() {
                            return Err("日期时间与原始文本不一致".into());
                        }
                        continue;
                    }
                    let v = values[row];
                    let zoned = chrono::DateTime::parse_from_rfc3339(text.trim()).is_ok();
                    let ok = match timestamp(text) {
                        Some(t) if !zoned => {
                            let d = t - v;
                            let same = *shift.get_or_insert(d) == d;
                            same && d % 60_000 == 0 && d.abs() <= 14 * 3_600_000
                        }
                        Some(t) => t == v,
                        None => false,
                    } || legacy_millis(text) == Some(v);
                    if !ok {
                        return Err("日期时间与原始文本不一致".into());
                    }
                }
            }
            Self::Category { codes, levels, .. } => {
                if levels.len() >= u32::MAX as usize
                    || levels.iter().any(String::is_empty)
                    || levels.iter().collect::<std::collections::BTreeSet<_>>().len() != levels.len()
                    || codes.iter().any(|c| *c != u32::MAX && *c as usize >= levels.len())
                {
                    return Err("分类字典或类别编码无效".into());
                }
            }
            Self::Text(_) => {}
        }
        Ok(())
    }
}

fn reordered_aux(
    bits: &[u64],
    lexemes: &BTreeMap<usize, String>,
    n: usize,
    order: &[usize],
) -> (Vec<u64>, BTreeMap<usize, String>) {
    let mut missing = vec![0; order.len().div_ceil(64)];
    let mut text = BTreeMap::new();
    for (new, &old) in order.iter().enumerate() {
        mark(&mut missing, new, old >= n || missing_at(bits, old));
        if let Some(s) = lexemes.get(&old) {
            text.insert(new, s.clone());
        }
    }
    (missing, text)
}

fn trim_aux(bits: &mut Vec<u64>, lexemes: &mut BTreeMap<usize, String>, n: usize) {
    bits.truncate(n.div_ceil(64));
    if !n.is_multiple_of(64) {
        if let Some(last) = bits.last_mut() {
            *last &= (1 << (n % 64)) - 1;
        }
    }
    lexemes.retain(|row, _| *row < n);
}

impl ColumnView for ColumnData {
    fn len(&self) -> usize {
        self.len()
    }
    fn f64_at(&self, row: usize) -> Option<f64> {
        self.f64_at(row)
    }
    fn text_at(&self, row: usize) -> Cow<'_, str> {
        self.text_at(row)
    }
}
impl FromIterator<String> for ColumnData {
    fn from_iter<T: IntoIterator<Item = String>>(iter: T) -> Self {
        let mut out = Self::default();
        for text in iter {
            out.push(text);
        }
        out
    }
}
impl From<Vec<String>> for ColumnData {
    fn from(v: Vec<String>) -> Self {
        v.into_iter().collect()
    }
}
impl<S: AsRef<str>> PartialEq<Vec<S>> for ColumnData {
    fn eq(&self, other: &Vec<S>) -> bool {
        self.len() == other.len() && self.iter().zip(other).all(|(a, b)| a == b.as_ref())
    }
}
impl<S: AsRef<str>, const N: usize> PartialEq<[S; N]> for ColumnData {
    fn eq(&self, other: &[S; N]) -> bool {
        self.len() == N && self.iter().zip(other).all(|(a, b)| a == b.as_ref())
    }
}
impl PartialEq for ColumnData {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (
                Self::Num {
                    values: a,
                    missing: am,
                    lexemes: al,
                },
                Self::Num {
                    values: b,
                    missing: bm,
                    lexemes: bl,
                },
            ) => am == bm && al == bl && a.len() == b.len() && a.iter().zip(b).all(|(a, b)| a.to_bits() == b.to_bits()),
            (
                Self::DateTime {
                    values: a,
                    missing: am,
                    lexemes: al,
                },
                Self::DateTime {
                    values: b,
                    missing: bm,
                    lexemes: bl,
                },
            ) => a == b && am == bm && al == bl,
            (Self::Text(a), Self::Text(b)) => a == b,
            (
                Self::Category {
                    codes: a,
                    levels: al,
                    ordered: ao,
                },
                Self::Category {
                    codes: b,
                    levels: bl,
                    ordered: bo,
                },
            ) => a == b && al == bl && ao == bo,
            _ => false,
        }
    }
}

#[cfg(feature = "serde")]
mod serialization {
    use super::*;
    use base64::{engine::general_purpose::STANDARD, Engine};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    #[derive(Serialize, Deserialize)]
    #[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
    enum Wire {
        Num {
            values_le: String,
            missing: Vec<u64>,
            #[serde(default, skip_serializing_if = "Vec::is_empty")]
            lexemes: Vec<(usize, String)>,
        },
        Text {
            values: Vec<String>,
        },
        DateTime {
            values_le: String,
            missing: Vec<u64>,
            #[serde(default, skip_serializing_if = "Vec::is_empty")]
            lexemes: Vec<(usize, String)>,
        },
        Category {
            codes: Vec<u32>,
            levels: Vec<String>,
            ordered: bool,
        },
    }
    impl Serialize for ColumnData {
        fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            self.validate().map_err(serde::ser::Error::custom)?;
            let wire = match self {
                Self::Num {
                    values,
                    missing,
                    lexemes,
                } => Wire::Num {
                    values_le: STANDARD.encode(values.iter().flat_map(|v| v.to_le_bytes()).collect::<Vec<_>>()),
                    missing: missing.clone(),
                    lexemes: lexemes.iter().map(|(&r, s)| (r, s.clone())).collect(),
                },
                Self::Text(values) => Wire::Text { values: values.clone() },
                Self::DateTime {
                    values,
                    missing,
                    lexemes,
                } => Wire::DateTime {
                    values_le: STANDARD.encode(values.iter().flat_map(|v| v.to_le_bytes()).collect::<Vec<_>>()),
                    missing: missing.clone(),
                    lexemes: lexemes.iter().map(|(&r, s)| (r, s.clone())).collect(),
                },
                Self::Category { codes, levels, ordered } => Wire::Category {
                    codes: codes.clone(),
                    levels: levels.clone(),
                    ordered: *ordered,
                },
            };
            wire.serialize(serializer)
        }
    }
    impl<'de> Deserialize<'de> for ColumnData {
        fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
            let raw = |entries: Vec<(usize, String)>| -> Result<BTreeMap<usize, String>, D::Error> {
                let mut map = BTreeMap::new();
                for (row, text) in entries {
                    if map.insert(row, text).is_some() {
                        return Err(serde::de::Error::custom("原始文本包含重复行索引"));
                    }
                }
                Ok(map)
            };
            let decode = |text: &str| -> Result<Vec<[u8; 8]>, String> {
                let bytes = STANDARD.decode(text).map_err(|e| e.to_string())?;
                if !bytes.len().is_multiple_of(8) {
                    return Err("数值字节长度必须为 8 的倍数".into());
                }
                Ok(bytes.as_chunks::<8>().0.to_vec())
            };
            let data = match Wire::deserialize(deserializer)? {
                Wire::Num {
                    values_le,
                    missing,
                    lexemes,
                } => Self::Num {
                    values: decode(&values_le)
                        .map_err(serde::de::Error::custom)?
                        .into_iter()
                        .map(f64::from_le_bytes)
                        .collect(),
                    missing,
                    lexemes: raw(lexemes)?,
                },
                Wire::Text { values } => Self::Text(values),
                Wire::DateTime {
                    values_le,
                    missing,
                    lexemes,
                } => Self::DateTime {
                    values: decode(&values_le)
                        .map_err(serde::de::Error::custom)?
                        .into_iter()
                        .map(i64::from_le_bytes)
                        .collect(),
                    missing,
                    lexemes: raw(lexemes)?,
                },
                Wire::Category { codes, levels, ordered } => Self::Category { codes, levels, ordered },
            };
            data.validate().map_err(serde::de::Error::custom)?;
            Ok(data)
        }
    }
}
