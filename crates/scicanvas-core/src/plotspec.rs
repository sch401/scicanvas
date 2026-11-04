//! 声明式作图输入：列映射和统计变换独立于界面与版面。
use crate::model::{Book, Column, PlotType, Series};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// 按编号查工作簿：图层里的曲线可以来自其他工作簿。
pub type BookLookup<'a, 'b> = &'b dyn Fn(u64) -> Option<&'a Book>;

fn normalized(value: f64, lo: f64, hi: f64) -> f64 {
    if hi <= lo {
        return 0.5;
    }
    let span = hi - lo;
    let t = if span.is_finite() {
        (value - lo) / span
    } else {
        let scale = lo.abs().max(hi.abs());
        (value / scale - lo / scale) / (hi / scale - lo / scale)
    };
    if t.is_finite() {
        t.clamp(0.0, 1.0)
    } else {
        0.5
    }
}

struct PointStyleColumns {
    color_values: Option<Vec<f64>>,
    size_values: Option<Vec<f64>>,
    symbol_values: Option<Vec<f64>>,
    color_levels: Option<usize>,
    symbol_levels: Option<usize>,
    color_range: Option<(f64, f64)>,
    size_data_range: Option<(f64, f64)>,
    size_range: [f32; 2],
    color_map: crate::colormap::ColorMap,
}

impl PointStyleColumns {
    fn new(
        style: Option<&crate::page::CurveStyle>,
        book: &Book,
        rows: std::ops::Range<usize>,
        color_map: crate::colormap::ColorMap,
        mask: Option<&[bool]>,
    ) -> Self {
        let column = |key: Option<&String>| key.and_then(|key| book.column(key));
        let color = column(style.and_then(|style| style.color_column.as_ref()));
        let size = column(style.and_then(|style| style.size_column.as_ref()));
        let symbol = column(style.and_then(|style| style.symbol_column.as_ref()));
        let color_values = color.map(|column| column.data.plot_category_positions().unwrap_or_else(|| column.values()));
        let size_values = size.map(|column| column.values());
        let symbol_values = symbol.and_then(|column| column.data.plot_category_positions());
        let color_levels = color
            .and_then(|column| column.data.plot_category_levels())
            .map(|levels| levels.len());
        let symbol_levels = symbol
            .and_then(|column| column.data.plot_category_levels())
            .map(|levels| levels.len());
        let range = |values: &[f64]| {
            values
                .get(rows.clone())
                .unwrap_or_default()
                .iter()
                .enumerate()
                .filter(|(offset, value)| value.is_finite() && crate::filter::visible_at(mask, rows.start + offset))
                .map(|(_, value)| *value)
                .fold(None, |range, value| {
                    Some(range.map_or((value, value), |(lo, hi): (f64, f64)| (lo.min(value), hi.max(value))))
                })
        };
        Self {
            color_range: color_values.as_deref().and_then(range),
            size_data_range: size_values.as_deref().and_then(range),
            size_range: style.and_then(|style| style.size_range).unwrap_or([2.0, 12.0]),
            color_values,
            size_values,
            symbol_values,
            color_levels,
            symbol_levels,
            color_map,
        }
    }

    fn at(&self, row: usize) -> crate::PointStyle {
        let color = self
            .color_values
            .as_ref()
            .and_then(|values| values.get(row).copied())
            .and_then(|value| {
                let (lo, hi) = self.color_range?;
                if !value.is_finite() {
                    return None;
                }
                let t = self.color_levels.map_or_else(
                    || normalized(value, lo, hi),
                    |levels| {
                        if levels <= 1 {
                            0.5
                        } else {
                            normalized(value - 1.0, 0.0, (levels - 1) as f64)
                        }
                    },
                );
                let (r, g, b) = self.color_map.sample(t);
                Some([r, g, b])
            });
        let size = self
            .size_values
            .as_ref()
            .and_then(|values| values.get(row).copied())
            .and_then(|value| {
                let (lo, hi) = self.size_data_range?;
                value.is_finite().then(|| {
                    let t = normalized(value, lo, hi);
                    (self.size_range[0] + (self.size_range[1] - self.size_range[0]) * t as f32).clamp(1.0, 20.0)
                })
            });
        let symbol = self
            .symbol_values
            .as_ref()
            .and_then(|values| values.get(row).copied())
            .filter(|value| value.is_finite())
            .and_then(|value| self.symbol_levels.filter(|levels| *levels > 0).map(|_| value))
            .map(|value| crate::SYMBOLS[(value.round().max(1.0) as usize - 1) % crate::SYMBOLS.len()]);
        crate::PointStyle { color, size, symbol }
    }
}

/// 明确的列引用；列名相同的不同工作簿始终是不同的数据源。
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(deny_unknown_fields))]
pub struct DataRef {
    pub book: u64,
    pub col: String,
}

impl DataRef {
    pub fn new(book: u64, col: impl Into<String>) -> Self {
        Self { book, col: col.into() }
    }

    pub fn from_key(main: Option<u64>, key: &str) -> Option<Self> {
        let (col, book) = split_key(key);
        Some(Self::new(book.or(main)?, col))
    }

    /// 现有属性控件使用的键；文件保存和分析使用 DataRef。
    pub fn key(&self, main: Option<u64>) -> String {
        if main == Some(self.book) && !self.col.contains('@') {
            self.col.clone()
        } else {
            foreign_key(self.book, &self.col)
        }
    }
}

/// 绘图、分析和文件共用的一条曲线映射。
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(deny_unknown_fields))]
pub struct CurveMapping {
    pub y: DataRef,
    pub x: Option<DataRef>,
    pub base: Option<DataRef>,
    pub rows: Option<(usize, usize)>,
    pub line_only: bool,
}

impl CurveMapping {
    pub fn validate(&self) -> Result<(), String> {
        for r in std::iter::once(&self.y).chain(self.x.iter()).chain(self.base.iter()) {
            if r.book == 0 || r.book == u64::MAX || r.col.is_empty() {
                return Err("曲线引用的工作簿编号和列标识应有效".into());
            }
            if r.book != self.y.book {
                return Err("一条曲线的 X、Y 和下端列应来自同一工作簿".into());
            }
        }
        if self.rows.is_some_and(|(a, b)| a > b || b == usize::MAX) {
            return Err("曲线行范围无效".into());
        }
        Ok(())
    }
}

/// One resolved curve description shared by rendering, analysis and persistence.
/// The editable PlotSpec is converted at the operation boundary, so there is no second mutable copy.
#[derive(Clone, Debug, PartialEq)]
pub struct Plot {
    pub key: String,
    pub mapping: CurveMapping,
    pub style: Option<crate::page::CurveStyle>,
    pub x_override: bool,
}

/// 其他工作簿的列在曲线键里写成 `列标识@工作簿编号`；不带编号的是图层所绑工作簿的列。
pub fn foreign_key(book_id: u64, col: &str) -> String {
    format!("{col}@{book_id}")
}

/// 拆开曲线键：(列标识, 工作簿编号)。
pub fn split_key(key: &str) -> (&str, Option<u64>) {
    match key
        .rsplit_once('@')
        .and_then(|(col, id)| id.parse().ok().map(|id| (col, id)))
    {
        Some((col, id)) => (col, Some(id)),
        None => (key, None),
    }
}

/// 工作簿里某个 Y 列默认配的 X 列：它左侧最近的 X 列，没有时取第一个 X 列。
pub fn default_x(book: &Book, y: &str) -> Option<String> {
    let at = book.column_index(y)?;
    book.columns[..at]
        .iter()
        .rev()
        .chain(book.columns.iter())
        .find(|c| c.role == crate::Role::X)
        .map(|c| c.key.clone())
}

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
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")
    )]
    pub row_by_y: std::collections::BTreeMap<String, Option<(usize, usize)>>,
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "std::ops::Not::not"))]
    pub visible_only: bool,
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
            row_by_y: Default::default(),
            visible_only: false,
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
    pub fn plot_in(&self, key: &str, books: BookLookup<'_, '_>) -> Option<Plot> {
        let source = DataRef::from_key(self.book_id, key)?;
        Some(Plot {
            key: key.to_string(),
            mapping: self.curve_mapping(key, books(source.book))?,
            style: None,
            x_override: self.x_by_y.contains_key(key),
        })
    }
    pub fn plots_in(&self, books: BookLookup<'_, '_>) -> Vec<Plot> {
        self.y_keys.iter().filter_map(|key| self.plot_in(key, books)).collect()
    }

    pub fn curve_mapping(&self, key: &str, book: Option<&Book>) -> Option<CurveMapping> {
        let y = DataRef::from_key(self.book_id, key)?;
        let local = self.book_id == Some(y.book);
        let line_only = self.line_only.iter().any(|k| k == key);
        let x = self
            .x_by_y
            .get(key)
            .and_then(|x| DataRef::from_key(self.book_id, x))
            .or_else(|| {
                if local {
                    self.x_key.as_ref().map(|x| DataRef::new(y.book, x))
                } else {
                    book.and_then(|b| default_x(b, &y.col)).map(|x| DataRef::new(y.book, x))
                }
            });
        Some(CurveMapping {
            base: self
                .base_by_y
                .get(key)
                .filter(|base| !base.is_empty())
                .and_then(|base| DataRef::from_key(self.book_id, base)),
            rows: self
                .row_by_y
                .get(key)
                .copied()
                .unwrap_or(if local && !line_only { self.rows } else { None }),
            y,
            x,
            line_only,
        })
    }

    /// 把分析结果绑定回其来源曲线所在的图层，保留左右轴及来源样式。
    pub fn result_key(&self, book: u64, col: &str) -> String {
        DataRef::new(book, col).key(self.book_id)
    }

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
    /// 曲线键对应的工作簿和列；其他工作簿的列用 `others` 查找。
    pub fn resolve<'a>(&self, main: &'a Book, others: BookLookup<'a, '_>, key: &str) -> Option<(&'a Book, &'a Column)> {
        let (col, id) = split_key(key);
        let book = match id {
            Some(id) if id != main.id => others(id)?,
            _ => main,
        };
        book.column(col).map(|c| (book, c))
    }

    /// 图层里引用到的其他工作簿编号（按首次出现顺序）。
    pub fn foreign_books(&self) -> Vec<u64> {
        let mut out = Vec::new();
        for key in self
            .y_keys
            .iter()
            .chain(self.x_by_y.values())
            .chain(self.base_by_y.values())
        {
            if let (_, Some(id)) = split_key(key) {
                if Some(id) != self.book_id && !out.contains(&id) {
                    out.push(id);
                }
            }
        }
        out
    }

    /// 把另一个工作簿（或本表）的一列加为曲线，X 用它左侧最近的 X 列；已有时不重复添加。
    /// 返回曲线键。
    pub fn add_curve(&mut self, book: &Book, y: &str) -> Result<String, String> {
        let col = book.column(y).ok_or_else(|| format!("找不到列 {y}"))?;
        if matches!(col.role, crate::Role::XError | crate::Role::YError) {
            return Err("误差列会自动关联数据，请选择数据列".into());
        }
        let local = self.book_id == Some(book.id);
        let qualify = |k: &str| if local { k.to_string() } else { foreign_key(book.id, k) };
        let key = qualify(y);
        if self.y_keys.contains(&key) {
            return Ok(key);
        }
        self.y_keys.push(key.clone());
        let x = default_x(book, y).filter(|x| x != y);
        if !local || x != self.x_key {
            match x {
                Some(x) => {
                    self.x_by_y.insert(key.clone(), qualify(&x));
                }
                None if local => {
                    self.x_by_y.remove(&key);
                }
                None => {}
            }
        }
        Ok(key)
    }

    /// 从图层移除一条曲线及其专属映射；曲线样式由图层另行清理。
    pub fn remove_curve(&mut self, key: &str) {
        self.y_keys.retain(|k| k != key);
        self.line_only.retain(|k| k != key);
        self.x_by_y.remove(key);
        self.base_by_y.remove(key);
        self.row_by_y.remove(key);
    }

    /// 曲线在列表里显示的名称：其他工作簿的曲线带上工作簿名。
    pub fn curve_label<'a>(&self, main: Option<&'a Book>, others: BookLookup<'a, '_>, key: &str) -> String {
        let found = match main {
            Some(main) => self.resolve(main, others, key),
            None => {
                let (col, id) = split_key(key);
                id.and_then(others).and_then(|b| b.column(col).map(|c| (b, c)))
            }
        };
        match found {
            Some((b, c)) if Some(b.id) != self.book_id => format!("{}：{}", b.name, c.display_name()),
            Some((_, c)) => c.display_name().to_string(),
            None => key.to_string(),
        }
    }

    pub fn validate(&self, book: &Book) -> Result<(), String> {
        self.validate_in(book, &|_| None)
    }

    /// 校验映射；其他工作簿的曲线在 `others` 里查找。
    pub fn validate_in<'a>(&self, book: &'a Book, others: BookLookup<'a, '_>) -> Result<(), String> {
        if self.book_id != Some(book.id) {
            return Err("工作簿与作图描述不匹配".into());
        }
        if self.y_keys.is_empty() {
            return Err("请至少选择一列 Y 数据".into());
        }
        for key in self.y_keys.iter().chain(self.x_key.iter()).chain(self.x_by_y.values()) {
            if self.resolve(book, others, key).is_none() {
                return Err(format!("找不到列 {key}"));
            }
        }
        if self.y_keys.iter().any(|key| {
            self.resolve(book, others, key)
                .is_some_and(|(_, c)| matches!(c.role, crate::Role::XError | crate::Role::YError))
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
                    || self
                        .resolve(book, others, base)
                        .is_none_or(|(_, c)| matches!(c.role, crate::Role::XError | crate::Role::YError))
                {
                    return Err("浮动柱下端应为另一列有效数据".into());
                }
            }
        }
        Ok(())
    }
    /// 取出 X 列和每条曲线的有效数据点（X、Y 都是有限数值的行）；只看本工作簿的曲线。
    pub fn series<'a>(&self, book: &'a Book) -> (Option<&'a Column>, Vec<Series>) {
        self.series_in(book, &|_| None)
    }

    /// 取出 X 列和每条曲线的有效数据点；其他工作簿的曲线在 `others` 里查找，
    /// 用各自的 X 列（没有映射时用该表里它左侧最近的 X 列），行范围只作用于本表。
    pub fn series_in<'a>(&self, book: &'a Book, others: BookLookup<'a, '_>) -> (Option<&'a Column>, Vec<Series>) {
        let plots = self.plots_in(&|id| if id == book.id { Some(book) } else { others(id) });
        self.series_from_plots(book, others, &plots)
    }

    pub(crate) fn series_from_plots<'a>(
        &self,
        book: &'a Book,
        others: BookLookup<'a, '_>,
        plots: &[Plot],
    ) -> (Option<&'a Column>, Vec<Series>) {
        let xcol = self.x_key.as_deref().and_then(|k| book.column(k));
        use crate::ColumnView;
        let mixed = plots.iter().any(|p| p.mapping.y.book != book.id);
        let mut out = Vec::new();
        for plot in plots {
            let key = &plot.key;
            let mapping = &plot.mapping;
            let Some(cbook) = (if mapping.y.book == book.id {
                Some(book)
            } else {
                others(mapping.y.book)
            }) else {
                continue;
            };
            let Some(c) = cbook.column(&mapping.y.col) else {
                continue;
            };
            if matches!(c.role, crate::Role::XError | crate::Role::YError) {
                continue;
            }
            let foreign = cbook.id != book.id;
            let yv = c;
            let base_values = if self.plot_type == PlotType::FloatingBar && !mapping.line_only {
                let Some(base) = mapping
                    .base
                    .as_ref()
                    .filter(|base| base.book == cbook.id && base.col != mapping.y.col)
                    .and_then(|base| cbook.column(&base.col))
                    .filter(|c| !matches!(c.role, crate::Role::XError | crate::Role::YError))
                else {
                    continue;
                };
                Some(base)
            } else {
                None
            };
            let own_x = mapping
                .x
                .as_ref()
                .and_then(|x| if x.book == cbook.id { cbook.column(&x.col) } else { None });
            // Q-Q 图只使用 Y 样本，X 列的缺失值和类别不参与取样。
            let xv = own_x.filter(|_| self.plot_type != PlotType::QQ || mapping.line_only);
            let n = yv.len().max(xv.as_ref().map_or(0, |v| v.len()));
            let (i0, i1) = match mapping.rows {
                Some((a, b)) => (a, b.saturating_add(1).min(n)),
                None => (0, n),
            };
            let filtered = self.visible_only && !c.independent_rows;
            let mask = if filtered { cbook.visible_mask() } else { None };
            let point_style_columns =
                PointStyleColumns::new(plot.style.as_ref(), cbook, i0..i1, self.color_map, mask.as_deref());
            let mut pts = Vec::with_capacity(i1.saturating_sub(i0));
            let error_values = [crate::Role::XError, crate::Role::YError].map(|role| {
                if mapping.line_only || c.derived_from.is_some() {
                    return None;
                }
                let find = |source: &str| {
                    cbook
                        .columns
                        .iter()
                        .enumerate()
                        .find(|(i, col)| col.role == role && cbook.error_source(*i) == Some(source))
                        .map(|(_, col)| col)
                };
                find(&c.key).or_else(|| {
                    if role == crate::Role::XError {
                        own_x.or(if foreign { None } else { xcol }).and_then(|x| find(&x.key))
                    } else {
                        None
                    }
                })
            });
            let mut errors = Vec::with_capacity(pts.capacity());
            let mut rows = Vec::with_capacity(pts.capacity());
            let mut bases = Vec::with_capacity(pts.capacity());
            let mut point_styles = Vec::with_capacity(i1.saturating_sub(i0));
            // 分类列做 X：按类别显示顺序放在 1、2、3…
            let categories = xv.and_then(|v| v.data.plot_category_positions());
            for i in i0..i1 {
                if !crate::filter::visible_at(mask.as_deref(), i) {
                    continue;
                }
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
                    point_styles.push(point_style_columns.at(i));
                    errors.push(error_values.each_ref().map(|values| {
                        values
                            .as_ref()
                            .and_then(|v| v.f64_at(i))
                            .filter(|v| v.is_finite() && *v >= 0.0)
                    }));
                }
            }
            // 浮动柱表示下端到上端的范围，图例写出两列
            let mut name = match base_values {
                Some(base) => format!("{} – {}", base.display_name(), c.display_name()),
                None => c.display_name().to_string(),
            };
            // 同一图层混合了多个工作簿时，图例带上工作簿名以便区分
            if mixed {
                name = format!("{}：{}", cbook.name, name);
            }
            out.push(Series {
                key: key.clone(),
                name,
                points: pts,
                point_styles,
                errors,
                rows,
                bases,
                line_only: mapping.line_only,
                source: c
                    .derived_from
                    .as_ref()
                    .map(|source| DataRef::new(cbook.id, source).key(self.book_id)),
            });
        }
        if self.plot_type != PlotType::QQ {
            crate::stat::apply(&mut out, self.transform);
        }
        crate::presets::recipe(self.plot_type).position.apply(&mut out);

        (xcol, out)
    }
}
