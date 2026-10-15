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
        Ok(Self {
            book_id: Some(book.id),
            x_key,
            y_keys,
            plot_type,
            ..Default::default()
        })
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
        if self.rows.is_some_and(|(a, b)| a > b || b == usize::MAX) {
            return Err("行范围无效".into());
        }
        Ok(())
    }
    /// 取出 X 列和每条曲线的有效数据点（X、Y 都是有限数值的行）。
    pub fn series<'a>(&self, book: &'a Book) -> (Option<&'a Column>, Vec<Series>) {
        let xcol = self.x_key.as_deref().and_then(|k| book.column(k));
        let main_xv = xcol.map(|c| c.values());
        let mut out = Vec::new();
        for key in &self.y_keys {
            let Some(c) = book.column(key) else { continue };
            let yv = c.values();
            let own_x = self.x_by_y.get(key).and_then(|k| book.column(k));
            let own_xv = own_x.map(|c| c.values());
            let xv = if own_x.is_some() { &own_xv } else { &main_xv };
            let n = yv.len().max(xv.as_ref().map_or(0, |v| v.len()));
            let (i0, i1) = match (self.rows, own_x) {
                (Some((a, b)), None) => (a, (b + 1).min(n)),
                _ => (0, n),
            };
            let mut pts = Vec::with_capacity(i1.saturating_sub(i0));
            for i in i0..i1 {
                let x = match xv {
                    Some(v) => v.get(i).copied().unwrap_or(f64::NAN),
                    None => (i + 1) as f64,
                };
                let y = yv.get(i).copied().unwrap_or(f64::NAN);
                if x.is_finite() && y.is_finite() {
                    pts.push([x, y]);
                }
            }
            out.push(Series {
                key: c.key.clone(),
                name: c.display_name().to_string(),
                points: pts,
                line_only: self.line_only.contains(key),
                source: c.derived_from.clone(),
            });
        }
        if self.transform == Transform::Mean {
            for s in &mut out {
                if s.line_only {
                    continue;
                }
                s.points.sort_by(|a, b| a[0].total_cmp(&b[0]));
                let mut grouped = Vec::new();
                let mut i = 0;
                while i < s.points.len() {
                    let x = s.points[i][0];
                    let mut end = i + 1;
                    while end < s.points.len() && s.points[end][0] == x {
                        end += 1;
                    }
                    let mean = s.points[i..end].iter().map(|p| p[1]).sum::<f64>() / (end - i) as f64;
                    grouped.push([x, mean]);
                    i = end;
                }
                s.points = grouped;
            }
        }
        (xcol, out)
    }
}
