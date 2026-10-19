//! 声明式作图输入：列映射和统计变换独立于界面与版面。
use crate::model::{Book, Column, PlotType, Series};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct PlotSpec {
    pub book_id: Option<u64>,
    pub x_key: Option<String>,
    pub y_keys: Vec<String>,
    pub plot_type: PlotType,
    #[cfg_attr(feature = "serde", serde(default))]
    pub x_by_y: std::collections::BTreeMap<String, String>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub base_by_y: std::collections::BTreeMap<String, String>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub line_only: Vec<String>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub rows: Option<(usize, usize)>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub transform: Transform,
    #[cfg_attr(feature = "serde", serde(default))]
    pub color_map: crate::colormap::ColorMap,
}
impl Default for PlotSpec {
    fn default() -> Self {
        Self {
            book_id: None,
            x_key: None,
            y_keys: vec![],
            plot_type: PlotType::LineSymbol,
            x_by_y: Default::default(),
            base_by_y: Default::default(),
            line_only: vec![],
            rows: None,
            transform: Transform::Identity,
            color_map: Default::default(),
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Transform {
    #[default]
    Identity,
    /// 按相同 X 分组，输出各组 Y 的均值。
    Mean,
}
impl PlotSpec {
    /// 换用另一张表作图：`defaults` 是该表的默认映射（`from_selection` 选中全部列的结果，没有 Y 列时为 None）。
    /// 保留图类型（新表不适用时用默认类型）、统计变换和配色；没有 Y 列时只关联表，列留空待选。
    pub fn rebound(&self, book_id: u64, defaults: Option<&PlotSpec>) -> PlotSpec {
        let mut next = defaults.cloned().unwrap_or_else(|| PlotSpec {
            book_id: Some(book_id),
            ..Default::default()
        });
        next.book_id = Some(book_id);
        if next.y_keys.is_empty() {
            next.plot_type = self.plot_type;
        } else {
            let mut typed = next.clone();
            if typed.set_plot_type(self.plot_type).is_ok() {
                next = typed;
            }
        }
        next.transform = self.transform;
        next.color_map = self.color_map;
        next
    }

    pub fn from_selection(book: &Book, selected: &[String], plot_type: PlotType) -> Result<Self, String> {
        let cols: Vec<&Column> = book
            .columns
            .iter()
            .filter(|c| selected.is_empty() || selected.contains(&c.key))
            .collect();
        let y_keys: Vec<String> = cols
            .iter()
            .filter(|c| c.role == crate::Role::Y)
            .map(|c| c.key.clone())
            .collect();
        let Some(first_y) = y_keys.first().and_then(|k| book.column_index(k)) else {
            return Err(if selected.is_empty() {
                "工作簿里没有 Y 列，请先把列设为 Y".into()
            } else {
                "选中的列里没有 Y 列，请至少选中一个 Y 列".into()
            });
        };
        let x_key = cols
            .iter()
            .find(|c| c.role == crate::Role::X)
            .map(|c| c.key.clone())
            .or_else(|| {
                book.columns[..first_y]
                    .iter()
                    .rev()
                    .find(|c| c.role == crate::Role::X)
                    .map(|c| c.key.clone())
            })
            .or_else(|| {
                book.columns
                    .iter()
                    .find(|c| c.role == crate::Role::X)
                    .map(|c| c.key.clone())
            });
        let mut spec = Self {
            book_id: Some(book.id),
            x_key,
            y_keys,
            ..Default::default()
        };
        spec.set_plot_type(plot_type)?;
        Ok(spec)
    }

    /// 初次选择浮动柱时，按 Y 列顺序配成下端/上端；以后使用保存的映射。
    pub fn set_plot_type(&mut self, kind: PlotType) -> Result<(), String> {
        if self.plot_type == kind {
            return Ok(());
        }
        if kind == PlotType::FloatingBar {
            let keys: Vec<_> = self
                .y_keys
                .iter()
                .filter(|k| !self.line_only.contains(k))
                .cloned()
                .collect();
            if !keys.iter().all(|k| self.base_by_y.contains_key(k)) || keys.is_empty() {
                if keys.is_empty() || keys.len() % 2 != 0 {
                    return Err("浮动柱请按下端、上端的顺序选择成对的 Y 列".into());
                }
                let mut uppers = Vec::new();
                for pair in keys.as_chunks::<2>().0 {
                    self.base_by_y.insert(pair[1].clone(), pair[0].clone());
                    uppers.push(pair[1].clone());
                }
                uppers.extend(self.line_only.iter().filter(|k| self.y_keys.contains(k)).cloned());
                self.y_keys = uppers;
            }
        }
        self.plot_type = kind;
        Ok(())
    }
    pub fn validate(&self, book: &Book) -> Result<(), String> {
        if self.book_id != Some(book.id) {
            return Err("工作簿与作图描述不匹配".into());
        }
        if self.y_keys.is_empty() {
            return Err("请至少选择一列 Y 数据".into());
        }
        for key in self.y_keys.iter().chain(self.x_key.iter()).chain(self.x_by_y.values()) {
            if book.column(key).is_none() {
                return Err(format!("找不到列 {key}"));
            }
        }
        if self.y_keys.iter().any(|key| {
            book.column(key)
                .is_some_and(|c| matches!(c.role, crate::Role::XError | crate::Role::YError))
        }) {
            return Err("误差列会自动关联数据，请从 Y 列列表中移除误差列".into());
        }
        if self.rows.is_some_and(|(a, b)| a > b || b == usize::MAX) {
            return Err("行范围无效".into());
        }
        if self.plot_type == PlotType::FloatingBar {
            for key in self.y_keys.iter().filter(|k| !self.line_only.contains(k)) {
                let base = self.base_by_y.get(key).ok_or("请为浮动柱选择下端列")?;
                if base == key
                    || book
                        .column(base)
                        .is_none_or(|c| matches!(c.role, crate::Role::XError | crate::Role::YError))
                {
                    return Err("浮动柱下端应为另一列有效数据".into());
                }
            }
        }
        Ok(())
    }
    /// 取出 X 列和每条曲线的有效数据点（X、Y 都是有限数值的行）。
    pub fn series<'a>(&self, book: &'a Book) -> (Option<&'a Column>, Vec<Series>) {
        let xcol = self.x_key.as_deref().and_then(|k| book.column(k));
        use crate::ColumnView;
        let main_xv = xcol;
        let mut out = Vec::new();
        for key in &self.y_keys {
            let Some(c) = book.column(key) else { continue };
            if matches!(c.role, crate::Role::XError | crate::Role::YError) {
                continue;
            }
            let yv = c;
            let base_values = if self.plot_type == PlotType::FloatingBar && !self.line_only.contains(key) {
                let Some(base) = self
                    .base_by_y
                    .get(key)
                    .filter(|k| *k != key)
                    .and_then(|k| book.column(k))
                    .filter(|c| !matches!(c.role, crate::Role::XError | crate::Role::YError))
                else {
                    continue;
                };
                Some(base)
            } else {
                None
            };
            let own_x = self.x_by_y.get(key).and_then(|k| book.column(k));
            let own_xv = own_x;
            let xv = if own_x.is_some() { &own_xv } else { &main_xv };
            let n = yv.len().max(xv.as_ref().map_or(0, |v| v.len()));
            let (i0, i1) = match (self.rows, own_x) {
                (Some((a, b)), None) => (a, (b + 1).min(n)),
                _ => (0, n),
            };
            let mut pts = Vec::with_capacity(i1.saturating_sub(i0));
            let error_values = [crate::Role::XError, crate::Role::YError].map(|role| {
                if self.line_only.contains(key) || c.derived_from.is_some() {
                    return None;
                }
                let find = |source: &str| {
                    book.columns
                        .iter()
                        .enumerate()
                        .find(|(i, col)| col.role == role && book.error_source(*i) == Some(source))
                        .map(|(_, col)| col)
                };
                find(key).or_else(|| {
                    if role == crate::Role::XError {
                        own_x.or(xcol).and_then(|x| find(&x.key))
                    } else {
                        None
                    }
                })
            });
            let mut errors = Vec::with_capacity(pts.capacity());
            let mut rows = Vec::with_capacity(pts.capacity());
            let mut bases = Vec::with_capacity(pts.capacity());
            // 分类列做 X：按类别显示顺序放在 1、2、3…
            let categories = xv.and_then(|v| v.data.category_positions());
            for i in i0..i1 {
                let x = match (&categories, xv) {
                    (Some(c), _) => c.get(i).copied().unwrap_or(f64::NAN),
                    (None, Some(v)) => v.f64_at(i).unwrap_or(f64::NAN),
                    (None, None) => (i + 1) as f64,
                };
                let y = yv.f64_at(i).unwrap_or(f64::NAN);
                let base = base_values.as_ref().map_or(0.0, |v| v.f64_at(i).unwrap_or(f64::NAN));
                if x.is_finite() && y.is_finite() && base.is_finite() {
                    pts.push([x, y]);
                    rows.push(i);
                    bases.push(base);
                    errors.push(error_values.each_ref().map(|values| {
                        values
                            .as_ref()
                            .and_then(|v| v.f64_at(i))
                            .filter(|v| v.is_finite() && *v >= 0.0)
                    }));
                }
            }
            // 浮动柱表示下端到上端的范围，图例写出两列
            let name = match base_values {
                Some(base) => format!("{} – {}", base.display_name(), c.display_name()),
                None => c.display_name().to_string(),
            };
            out.push(Series {
                key: c.key.clone(),
                name,
                points: pts,
                errors,
                rows,
                bases,
                line_only: self.line_only.contains(key),
                source: c.derived_from.clone(),
            });
        }
        if self.transform == Transform::Mean {
            for s in &mut out {
                if s.line_only {
                    continue;
                }
                let mut indices: Vec<_> = (0..s.points.len()).collect();
                indices.sort_by(|a, b| s.points[*a][0].total_cmp(&s.points[*b][0]));
                let mut grouped = Vec::new();
                let mut bases = Vec::new();
                let mut i = 0;
                while i < s.points.len() {
                    let x = s.points[indices[i]][0];
                    let mut end = i + 1;
                    while end < s.points.len() && s.points[indices[end]][0] == x {
                        end += 1;
                    }
                    let mean = indices[i..end].iter().map(|j| s.points[*j][1]).sum::<f64>() / (end - i) as f64;
                    grouped.push([x, mean]);
                    bases.push(indices[i..end].iter().map(|j| s.bases[*j]).sum::<f64>() / (end - i) as f64);
                    i = end;
                }
                s.points = grouped;
                s.bases = bases;
                s.rows = vec![0; s.points.len()];
                // 原始行的测量误差不代表分组均值的标准误。
                s.errors = vec![[None; 2]; s.points.len()];
            }
        }
        if self.plot_type.is_stacked() {
            stack_bars(&mut out, self.plot_type == PlotType::PercentStackedBar);
        }
        (xcol, out)
    }
}

fn stack_bars(series: &mut [Series], percent: bool) {
    use std::collections::BTreeMap;
    let key = |row: usize, x: f64| (row, if x == 0.0 { 0 } else { x.to_bits() });
    let mut totals: BTreeMap<_, [f64; 2]> = BTreeMap::new();
    let mut scales: BTreeMap<_, [f64; 2]> = BTreeMap::new();
    if percent {
        for s in series.iter().filter(|s| !s.line_only) {
            for (p, row) in s.points.iter().zip(&s.rows) {
                let scale = &mut scales.entry(key(*row, p[0])).or_default()[usize::from(p[1] < 0.0)];
                *scale = scale.max(p[1].abs());
            }
        }
        for s in series.iter().filter(|s| !s.line_only) {
            for (p, row) in s.points.iter().zip(&s.rows) {
                let key = key(*row, p[0]);
                let sign = usize::from(p[1] < 0.0);
                let scale = scales[&key][sign];
                totals.entry(key).or_default()[sign] += if scale > 0.0 { p[1].abs() / scale } else { 0.0 };
            }
        }
    }
    let mut sums: BTreeMap<_, [f64; 2]> = BTreeMap::new();
    for s in series.iter_mut().filter(|s| !s.line_only) {
        for (((p, base), row), errors) in s.points.iter_mut().zip(&mut s.bases).zip(&s.rows).zip(&mut s.errors) {
            let key = key(*row, p[0]);
            let sign = usize::from(p[1] < 0.0);
            if percent {
                let total = totals[&key][sign];
                let (scale, factor) = if total > 0.0 {
                    (scales[&key][sign], 100.0 / total)
                } else {
                    (1.0, 0.0)
                };
                p[1] = p[1] / scale * factor;
                errors[1] = errors[1].map(|e| e / scale * factor).filter(|e| e.is_finite());
            }
            let sum = &mut sums.entry(key).or_default()[sign];
            *base = *sum;
            *sum += p[1];
            p[1] = *sum;
        }
    }
}
