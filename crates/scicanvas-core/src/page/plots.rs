//! 图层曲线的唯一存储。旧 PlotSpec 只用于作图草稿和早期文件的边界转换。
use super::{CurveStyle, Layer};
use crate::plotspec::{default_x, split_key, BookLookup, CurveMapping, DataRef, Plot as Description, PlotSpec};
use std::collections::BTreeMap;

/// 一条曲线及其暂存设置。book=0 表示跟随图层主表，允许旧空图尚未绑定。
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Plot {
    pub(crate) source: DataRef,
    pub(crate) active: bool,
    pub(crate) x: Option<DataRef>,
    pub(crate) base: Option<DataRef>,
    pub(crate) rows: Option<Option<(usize, usize)>>,
    pub(crate) line_order: Option<usize>,
    pub(crate) style: Option<CurveStyle>,
}

/// 图层公共设置与有序曲线对象，不再持有平行映射表。
#[derive(Clone, Debug, PartialEq)]
pub struct LayerSpec {
    pub book_id: Option<u64>,
    pub x_key: Option<String>,
    pub plot_type: crate::PlotType,
    pub rows: Option<(usize, usize)>,
    pub transform: crate::plotspec::Transform,
    pub color_map: crate::colormap::ColorMap,
    pub(crate) plots: Vec<Plot>,
}

fn raw_ref(key: &str) -> DataRef {
    let (col, book) = split_key(key);
    DataRef::new(book.unwrap_or(0), col)
}
fn raw_key(source: &DataRef) -> String {
    if source.book == 0 {
        source.col.clone()
    } else {
        crate::plotspec::foreign_key(source.book, &source.col)
    }
}
fn resolved(source: &DataRef, main: Option<u64>) -> Option<DataRef> {
    Some(DataRef::new(
        if source.book == 0 { main? } else { source.book },
        &source.col,
    ))
}

impl Default for LayerSpec {
    fn default() -> Self {
        PlotSpec::default().into()
    }
}
impl From<PlotSpec> for LayerSpec {
    fn from(spec: PlotSpec) -> Self {
        Self::from_draft(spec, BTreeMap::new())
    }
}
impl LayerSpec {
    /// 缩放缓存沿用旧 PlotSpec 的判定范围，样式及仅存样式的隐藏项不重置视图。
    pub fn same_mapping(&self, other: &Self) -> bool {
        if (
            self.book_id,
            &self.x_key,
            self.plot_type,
            self.rows,
            self.transform,
            self.color_map,
        ) != (
            other.book_id,
            &other.x_key,
            other.plot_type,
            other.rows,
            other.transform,
            other.color_map,
        ) {
            return false;
        }
        let mapping =
            |p: &&Plot| p.active || p.x.is_some() || p.base.is_some() || p.rows.is_some() || p.line_order.is_some();
        let a = self.plots.iter().filter(mapping);
        let b = other.plots.iter().filter(mapping);
        a.clone().count() == b.clone().count()
            && a.zip(b).all(|(a, b)| {
                a.source == b.source
                    && a.active == b.active
                    && a.x == b.x
                    && a.base == b.base
                    && a.rows == b.rows
                    && a.line_order == b.line_order
            })
    }
    fn from_draft(spec: PlotSpec, styles: BTreeMap<String, CurveStyle>) -> Self {
        let mut keys = spec.y_keys.clone();
        for key in spec
            .x_by_y
            .keys()
            .chain(spec.base_by_y.keys())
            .chain(spec.row_by_y.keys())
            .chain(spec.line_only.iter())
            .chain(styles.keys())
        {
            if !keys.contains(key) {
                keys.push(key.clone());
            }
        }
        let plots = keys
            .into_iter()
            .map(|key| Plot {
                source: raw_ref(&key),
                active: spec.y_keys.contains(&key),
                x: spec.x_by_y.get(&key).map(|key| raw_ref(key)),
                base: spec.base_by_y.get(&key).map(|key| raw_ref(key)),
                rows: spec.row_by_y.get(&key).copied(),
                line_order: spec.line_only.iter().position(|k| k == &key),
                style: styles.get(&key).cloned(),
            })
            .collect();
        let mut result = Self {
            book_id: spec.book_id,
            x_key: spec.x_key,
            plot_type: spec.plot_type,
            rows: spec.rows,
            transform: spec.transform,
            color_map: spec.color_map,
            plots,
        };
        result.normalize();
        result
    }
    fn normalize(&mut self) {
        self.plots.retain(|p| {
            p.active
                || p.x.is_some()
                || p.base.is_some()
                || p.rows.is_some()
                || p.line_order.is_some()
                || p.style.is_some()
        });
        self.plots.sort_by(|a, b| match (a.active, b.active) {
            (true, true) => std::cmp::Ordering::Equal,
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => raw_key(&a.source).cmp(&raw_key(&b.source)),
        });
        let mut lines: Vec<_> = self.plots.iter().filter_map(|p| p.line_order).collect();
        lines.sort_unstable();
        lines.dedup();
        for p in &mut self.plots {
            p.line_order = p.line_order.map(|i| lines.binary_search(&i).unwrap());
        }
    }
    fn entry(&self, key: &str) -> Option<&Plot> {
        let source = raw_ref(key);
        self.plots.iter().find(|p| p.source == source)
    }
    fn entry_mut(&mut self, key: &str) -> &mut Plot {
        let source = raw_ref(key);
        let i = self.plots.iter().position(|p| p.source == source).unwrap_or_else(|| {
            self.plots.push(Plot {
                source,
                active: false,
                x: None,
                base: None,
                rows: None,
                line_order: None,
                style: None,
            });
            self.plots.len() - 1
        });
        &mut self.plots[i]
    }
    /// 临时草稿投影，不缓存，也不成为第二份可变存储。
    pub fn draft(&self) -> PlotSpec {
        let mut spec = PlotSpec {
            book_id: self.book_id,
            x_key: self.x_key.clone(),
            plot_type: self.plot_type,
            rows: self.rows,
            transform: self.transform,
            color_map: self.color_map,
            ..Default::default()
        };
        let mut lines = Vec::new();
        for p in &self.plots {
            let key = raw_key(&p.source);
            if p.active {
                spec.y_keys.push(key.clone());
            }
            if let Some(x) = &p.x {
                spec.x_by_y.insert(key.clone(), raw_key(x));
            }
            if let Some(base) = &p.base {
                spec.base_by_y.insert(key.clone(), raw_key(base));
            }
            if let Some(rows) = p.rows {
                spec.row_by_y.insert(key.clone(), rows);
            }
            if let Some(order) = p.line_order {
                lines.push((order, key));
            }
        }
        lines.sort_by_key(|(i, _)| *i);
        spec.line_only = lines.into_iter().map(|(_, key)| key).collect();
        spec
    }
    pub fn edit_styles<T>(&mut self, edit: impl FnOnce(&mut BTreeMap<String, CurveStyle>) -> T) -> T {
        let mut styles = self.styles();
        let result = edit(&mut styles);
        for p in &mut self.plots {
            p.style = None;
        }
        for (key, style) in styles {
            self.entry_mut(&key).style = Some(style);
        }
        self.normalize();
        result
    }
    pub fn styles(&self) -> BTreeMap<String, CurveStyle> {
        self.plots
            .iter()
            .filter_map(|p| Some((raw_key(&p.source), p.style.clone()?)))
            .collect()
    }
    /// 完整替换草稿，保留原本独立于勾选项的样式。
    pub fn replace_draft(&mut self, spec: PlotSpec) {
        *self = Self::from_draft(spec, self.styles());
    }
    pub fn edit_draft<T>(&mut self, edit: impl FnOnce(&mut PlotSpec) -> T) -> T {
        let mut draft = self.draft();
        let result = edit(&mut draft);
        self.replace_draft(draft);
        result
    }
    pub fn rebound(&self, book: u64, defaults: Option<&PlotSpec>) -> Self {
        Self::from_draft(self.draft().rebound(book, defaults), self.styles())
    }
    pub fn set_plot_type(&mut self, kind: crate::PlotType) -> Result<(), String> {
        let mut draft = self.draft();
        draft.set_plot_type(kind)?;
        self.replace_draft(draft);
        Ok(())
    }
    pub fn result_key(&self, book: u64, col: &str) -> String {
        DataRef::new(book, col).key(self.book_id)
    }
    pub fn resolve<'a>(
        &self,
        main: &'a crate::Book,
        others: BookLookup<'a, '_>,
        key: &str,
    ) -> Option<(&'a crate::Book, &'a crate::Column)> {
        let (col, id) = split_key(key);
        let book = match id {
            Some(id) if id != main.id => others(id)?,
            _ => main,
        };
        book.column(col).map(|c| (book, c))
    }
    pub fn curve_label<'a>(&self, main: Option<&'a crate::Book>, others: BookLookup<'a, '_>, key: &str) -> String {
        self.draft().curve_label(main, others, key)
    }
    pub fn foreign_books(&self) -> Vec<u64> {
        self.draft().foreign_books()
    }
    pub fn validate(&self, book: &crate::Book) -> Result<(), String> {
        self.validate_in(book, &|_| None)
    }
    pub fn validate_in<'a>(&self, book: &'a crate::Book, others: BookLookup<'a, '_>) -> Result<(), String> {
        self.draft().validate_in(book, others)
    }
    pub fn add_curve(&mut self, book: &crate::Book, key: &str) -> Result<String, String> {
        let mut draft = self.draft();
        let key = draft.add_curve(book, key)?;
        self.replace_draft(draft);
        Ok(key)
    }
    pub fn remove_curve(&mut self, key: &str) {
        let source = raw_ref(key);
        self.plots.retain(|p| p.source != source);
        self.normalize();
    }
    pub fn series<'a>(&self, book: &'a crate::Book) -> (Option<&'a crate::Column>, Vec<crate::Series>) {
        self.series_in(book, &|_| None)
    }
    pub fn series_in<'a>(
        &self,
        book: &'a crate::Book,
        others: BookLookup<'a, '_>,
    ) -> (Option<&'a crate::Column>, Vec<crate::Series>) {
        let plots = self.plots_in(&|id| if id == book.id { Some(book) } else { others(id) });
        self.series_from_plots(book, others, &plots)
    }
    pub(crate) fn series_from_plots<'a>(
        &self,
        book: &'a crate::Book,
        others: BookLookup<'a, '_>,
        plots: &[Description],
    ) -> (Option<&'a crate::Column>, Vec<crate::Series>) {
        PlotSpec {
            book_id: self.book_id,
            x_key: self.x_key.clone(),
            plot_type: self.plot_type,
            transform: self.transform,
            ..Default::default()
        }
        .series_from_plots(book, others, plots)
    }
    pub fn plot_in(&self, key: &str, books: BookLookup<'_, '_>) -> Option<Description> {
        self.describe(self.entry(key)?, books)
    }
    fn describe(&self, p: &Plot, books: BookLookup<'_, '_>) -> Option<Description> {
        let y = resolved(&p.source, self.book_id)?;
        let local = self.book_id == Some(y.book);
        let x = p.x.as_ref().and_then(|x| resolved(x, self.book_id)).or_else(|| {
            if local {
                self.x_key.as_ref().map(|key| DataRef::new(y.book, key))
            } else {
                books(y.book)
                    .and_then(|b| default_x(b, &y.col))
                    .map(|key| DataRef::new(y.book, key))
            }
        });
        Some(Description {
            key: raw_key(&p.source),
            mapping: CurveMapping {
                y,
                x,
                base: p
                    .base
                    .as_ref()
                    .filter(|b| !b.col.is_empty())
                    .and_then(|b| resolved(b, self.book_id)),
                rows: p.rows.unwrap_or(if local && p.line_order.is_none() {
                    self.rows
                } else {
                    None
                }),
                line_only: p.line_order.is_some(),
            },
            style: p.style.clone(),
            x_override: p.x.is_some(),
        })
    }
    pub fn plots_in(&self, books: BookLookup<'_, '_>) -> Vec<Description> {
        self.plots
            .iter()
            .filter(|p| p.active)
            .filter_map(|p| self.describe(p, books))
            .collect()
    }
}

impl Layer {
    pub(crate) fn estimated_plot_bytes(&self) -> usize {
        self.spec.plots.capacity() * std::mem::size_of::<Plot>()
            + self
                .spec
                .plots
                .iter()
                .map(|p| {
                    p.source.col.capacity()
                        + p.x.as_ref().map_or(0, |r| r.col.capacity())
                        + p.base.as_ref().map_or(0, |r| r.col.capacity())
                        + p.style.as_ref().map_or(0, |s| s.legend_label.capacity())
                })
                .sum::<usize>()
    }
    pub fn plot_keys(&self, include_derived: bool) -> Vec<String> {
        self.spec
            .plots
            .iter()
            .filter(|p| p.active && (include_derived || p.line_order.is_none()))
            .map(|p| raw_key(&p.source))
            .collect()
    }
    /// 属性控件以来源标识选择曲线，未绑定的本表项保留 book=0。
    pub fn plot_sources(&self, include_derived: bool) -> Vec<DataRef> {
        self.spec
            .plots
            .iter()
            .filter(|p| p.active && (include_derived || p.line_order.is_none()))
            .map(|p| resolved(&p.source, self.book_id).unwrap_or_else(|| p.source.clone()))
            .collect()
    }
    pub fn source_style(&self, source: &DataRef) -> Option<&CurveStyle> {
        self.spec
            .plots
            .iter()
            .find(|p| resolved(&p.source, self.book_id).as_ref().unwrap_or(&p.source) == source)?
            .style
            .as_ref()
    }
    pub fn set_source_style(&mut self, source: &DataRef, style: Option<CurveStyle>) {
        if self.source_style(source) == style.as_ref() {
            return;
        }
        let key = self
            .spec
            .plots
            .iter()
            .find(|p| resolved(&p.source, self.book_id).as_ref().unwrap_or(&p.source) == source)
            .map(|p| raw_key(&p.source))
            .unwrap_or_else(|| {
                if source.book == 0 {
                    source.col.clone()
                } else {
                    source.key(self.book_id)
                }
            });
        self.set_plot_style(&key, style);
    }
    pub fn set_plot_selected(&mut self, source: &DataRef, on: bool) {
        let key = if source.book == 0 {
            source.col.clone()
        } else {
            source.key(self.book_id)
        };
        if on {
            self.spec.entry_mut(&key).active = true;
        } else {
            let main = self.book_id;
            for p in &mut self.spec.plots {
                if resolved(&p.source, main).as_ref().unwrap_or(&p.source) == source {
                    p.active = false;
                }
            }
        }
        self.spec.normalize();
    }
    pub fn edit_plot_keys(&mut self, edit: impl FnOnce(&mut Vec<String>)) {
        let mut keys = self.plot_keys(true);
        edit(&mut keys);
        let mut old = std::mem::take(&mut self.spec.plots);
        let mut next = Vec::with_capacity(old.len().max(keys.len()));
        for key in keys {
            let source = raw_ref(&key);
            let mut p = old.iter().find(|p| p.source == source).cloned().unwrap_or(Plot {
                source,
                active: true,
                x: None,
                base: None,
                rows: None,
                line_order: None,
                style: None,
            });
            p.active = true;
            next.push(p);
        }
        for mut p in old.drain(..) {
            if !next.iter().any(|n| n.source == p.source) {
                p.active = false;
                next.push(p);
            }
        }
        self.spec.plots = next;
        self.spec.normalize();
    }
    pub fn select_plot(&mut self, source: &DataRef, x: Option<&DataRef>) {
        let key = source.key(self.book_id);
        self.spec.entry_mut(&key).active = true;
        if let Some(x) = x {
            self.spec.entry_mut(&key).x = Some(raw_ref(&x.key(self.book_id)));
        }
        self.spec.normalize();
    }
    pub fn plot_base_key(&self, key: &str) -> Option<String> {
        self.spec.entry(key)?.base.as_ref().map(raw_key)
    }
    pub fn set_plot_base_key(&mut self, key: &str, base: String) {
        self.spec.entry_mut(key).base = Some(raw_ref(&base));
        self.spec.normalize();
    }
    pub fn valid_plot_rows(&self) -> bool {
        !self.rows.is_some_and(|(a, b)| a > b || b == usize::MAX)
            && !self
                .spec
                .plots
                .iter()
                .filter_map(|p| p.rows.flatten())
                .any(|(a, b)| a > b || b == usize::MAX)
    }
    pub fn references_plot_column(&self, book: &crate::Book, key: &str) -> bool {
        let local = self.book_id == Some(book.id);
        let matches = |r: &DataRef| r.col == key && (r.book == book.id || (local && r.book == 0));
        (local && self.x_key.as_deref() == Some(key))
            || (local && self.labels.column.as_deref() == Some(key))
            || self.spec.plots.iter().any(|p| {
                (p.active && matches(&p.source))
                    || p.x.as_ref().is_some_and(matches)
                    || p.base.as_ref().is_some_and(matches)
                    || (!local
                        && p.active
                        && p.source.book == book.id
                        && p.x.is_none()
                        && default_x(book, &p.source.col).as_deref() == Some(key))
            })
    }
    pub fn plot_style(&self, key: &str) -> Option<&CurveStyle> {
        self.spec.entry(key)?.style.as_ref()
    }
    pub fn set_plot_style(&mut self, key: &str, style: Option<CurveStyle>) {
        if self.plot_style(key) == style.as_ref() {
            return;
        }
        let source = raw_ref(key);
        for p in &mut self.spec.plots {
            if p.source == source {
                p.style = style.clone();
            }
        }
        if style.is_some() {
            self.spec.entry_mut(key).style = style;
        }
        self.spec.normalize();
    }
    pub fn has_right_axis_style(&self) -> bool {
        self.spec
            .plots
            .iter()
            .any(|p| p.style.as_ref().is_some_and(|s| s.right_axis))
    }
    pub fn plots(&self, books: BookLookup<'_, '_>) -> Vec<Description> {
        self.spec.plots_in(books)
    }
    pub fn plot(&self, source: &DataRef, books: BookLookup<'_, '_>) -> Option<Description> {
        self.spec.plot_in(&self.plot_key(source)?, books)
    }
    pub fn plot_mut(
        &mut self,
        source: &DataRef,
        books: BookLookup<'_, '_>,
        edit: impl FnOnce(&mut Description),
    ) -> Result<bool, String> {
        let before = self.plot(source, books).ok_or("找不到曲线")?;
        let mut after = before.clone();
        edit(&mut after);
        if after.mapping.y != before.mapping.y || after.key != before.key {
            return Err("曲线来源和旧键不可直接修改，请先移除再添加".into());
        }
        if after == before {
            return Ok(false);
        }
        self.store_plot(&after, Some(&before), books)?;
        Ok(self.plot(source, books).as_ref() != Some(&before))
    }
    pub fn add_plot(&mut self, mut plot: Description, books: BookLookup<'_, '_>) -> Result<bool, String> {
        plot.mapping.validate()?;
        if self.plot_key(&plot.mapping.y).is_some() {
            return Ok(false);
        }
        plot.key = plot.mapping.y.key(self.book_id);
        self.store_plot(&plot, None, books)?;
        self.spec.entry_mut(&plot.key).active = true;
        self.spec.normalize();
        Ok(true)
    }
    pub fn remove_plot(&mut self, source: &DataRef) -> bool {
        let main = self.book_id;
        let before = self.spec.plots.len();
        self.spec
            .plots
            .retain(|p| !p.active || resolved(&p.source, main).as_ref() != Some(source));
        self.spec.normalize();
        before != self.spec.plots.len()
    }
    pub fn retain_plots(&mut self, books: BookLookup<'_, '_>, mut keep: impl FnMut(&Description) -> bool) -> usize {
        let gone: Vec<_> = self
            .plots(books)
            .into_iter()
            .filter(|p| !keep(p))
            .map(|p| raw_ref(&p.key))
            .collect();
        let before = self.plot_keys(true).len();
        self.spec.plots.retain(|p| !gone.contains(&p.source));
        self.spec.normalize();
        before - self.plot_keys(true).len()
    }
    pub(crate) fn remove_columns(&mut self, book: u64, keys: &[String], books: BookLookup<'_, '_>) {
        if self.book_id != Some(book) {
            self.drop_foreign(book, Some(keys));
            return;
        }
        self.retain_plots(books, |p| !keys.contains(&p.key));
        self.spec
            .plots
            .retain(|p| !(p.source.book == 0 && keys.contains(&p.source.col)));
        for p in &mut self.spec.plots {
            if p.x.as_ref().is_some_and(|r| r.book == 0 && keys.contains(&r.col)) {
                p.x = None;
            }
            if p.base.as_ref().is_some_and(|r| r.book == 0 && keys.contains(&r.col)) {
                p.base = Some(raw_ref(""));
            }
        }
        if self.labels.column.as_ref().is_some_and(|k| keys.contains(k)) {
            self.labels.column = None;
        }
        if self.x_key.as_ref().is_some_and(|k| keys.contains(k)) {
            self.x_key = None;
        }
        self.spec.normalize();
    }
    pub(crate) fn map_row_ranges(&mut self, book: u64, map: impl Fn(&mut Option<(usize, usize)>)) {
        let main = self.book_id;
        if main == Some(book) {
            map(&mut self.spec.rows);
        }
        for p in &mut self.spec.plots {
            if resolved(&p.source, main).is_some_and(|r| r.book == book) {
                if let Some(rows) = &mut p.rows {
                    map(rows);
                }
            }
        }
    }
    fn plot_key(&self, source: &DataRef) -> Option<String> {
        self.spec
            .plots
            .iter()
            .find(|p| p.active && resolved(&p.source, self.book_id).as_ref() == Some(source))
            .map(|p| raw_key(&p.source))
    }
    fn store_plot(
        &mut self,
        plot: &Description,
        before: Option<&Description>,
        books: BookLookup<'_, '_>,
    ) -> Result<(), String> {
        let m = &plot.mapping;
        m.validate()?;
        let local = self.book_id == Some(m.y.book);
        let inherited_x = if local {
            self.x_key.as_ref().map(|x| DataRef::new(m.y.book, x))
        } else {
            books(m.y.book)
                .and_then(|b| default_x(b, &m.y.col))
                .map(|x| DataRef::new(m.y.book, x))
        };
        let change_x = before.is_none_or(|b| m.x != b.mapping.x || plot.x_override != b.x_override);
        let override_x = plot.x_override || m.x != inherited_x;
        if change_x && m.x.is_none() && (inherited_x.is_some() || plot.x_override) {
            return Err("旧曲线存储无法把单条曲线的默认 X 覆盖为行号".into());
        }
        let main = self.book_id;
        let inherited_rows = if local && !m.line_only { self.rows } else { None };
        let next_line = self
            .spec
            .plots
            .iter()
            .filter_map(|p| p.line_order)
            .max()
            .map_or(0, |n| n + 1);
        let entry = self.spec.entry_mut(&plot.key);
        if change_x {
            entry.x = m.x.as_ref().filter(|_| override_x).map(|r| raw_ref(&r.key(main)));
        }
        if before.is_none_or(|b| m.base != b.mapping.base) {
            entry.base = m.base.as_ref().map(|r| raw_ref(&r.key(main)));
        }
        if before.is_none_or(|b| m.rows != b.mapping.rows || m.line_only != b.mapping.line_only) {
            entry.rows = if m.rows == inherited_rows { None } else { Some(m.rows) };
        }
        if before.is_none_or(|b| m.line_only != b.mapping.line_only) {
            entry.line_order = m.line_only.then_some(next_line);
        }
        if before.is_none_or(|b| plot.style != b.style) {
            entry.style = plot.style.clone();
        }
        self.spec.normalize();
        Ok(())
    }
}

impl LayerSpec {
    pub(crate) fn has_curve_kind(&self) -> bool {
        self.plots
            .iter()
            .any(|p| p.style.as_ref().is_some_and(|s| s.kind.is_some()))
    }
    pub(crate) fn clear_plots(&mut self) {
        self.plots.clear();
    }
    pub(crate) fn clear_plot_x(&mut self, key: &str) {
        if let Some(p) = self.plots.iter_mut().find(|p| p.source == raw_ref(key)) {
            p.x = None;
        }
        self.normalize();
    }
    pub(crate) fn explicit_plot_x(&self, key: &str) -> Option<String> {
        self.entry(key)?.x.as_ref().map(raw_key)
    }
    pub(crate) fn import_mapping(&mut self, key: String, m: CurveMapping, style: Option<CurveStyle>, missing: bool) {
        let main = self.book_id;
        let order = self
            .plots
            .iter()
            .filter_map(|p| p.line_order)
            .max()
            .map_or(0, |i| i + 1);
        self.plots.push(Plot {
            source: raw_ref(&key),
            active: true,
            x: m.x.map(|r| raw_ref(&r.key(main))),
            base: m
                .base
                .map(|r| raw_ref(&r.key(main)))
                .or_else(|| missing.then(|| raw_ref(""))),
            rows: m.rows.map(Some),
            line_order: m.line_only.then_some(order),
            style,
        });
        self.normalize();
    }
    pub(crate) fn add_derived(&mut self, source: &str, key: &str, x: Option<String>) {
        self.entry_mut(source).active = true;
        let order = self
            .plots
            .iter()
            .filter_map(|p| p.line_order)
            .max()
            .map_or(0, |i| i + 1);
        let entry = self.entry_mut(key);
        entry.active = true;
        if entry.line_order.is_none() {
            entry.line_order = Some(order);
        }
        if let Some(x) = x {
            entry.x = Some(raw_ref(&x));
        }
        self.normalize();
    }
    pub(crate) fn drop_foreign(&mut self, book: u64, keys: Option<&[String]>) {
        let hit = |r: &DataRef| r.book == book && keys.is_none_or(|keys| keys.contains(&r.col));
        // 沿用既有删除语义：隐藏的编辑设置仍保留，下端缺失留下明确标记。
        self.plots
            .retain(|p| !p.active || !(hit(&p.source) || p.x.as_ref().is_some_and(hit)));
        for p in &mut self.plots {
            if p.base.as_ref().is_some_and(hit) {
                p.base = Some(raw_ref(""));
            }
        }
        self.normalize();
    }
}

/// 项目 v11–v14 的原有曲线对象，字段与省略规则保持。
#[cfg(feature = "serde")]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct StoredCurve {
    pub mapping: CurveMapping,
    pub style: Option<CurveStyle>,
    #[serde(default)]
    pub x_override: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub base_missing: bool,
}
#[cfg(feature = "serde")]
impl serde::Serialize for LayerSpec {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::Error;
        // 先只写公共图层设置；旧空图保留原有未绑定草稿的字段。
        let mut value = serde_json::json!({
            "book_id":self.book_id,"x_key":self.x_key,"plot_type":self.plot_type,
            "rows":self.rows,"transform":self.transform,"color_map":self.color_map,
        });
        if self.book_id.is_none() {
            value = serde_json::to_value(self.draft()).map_err(S::Error::custom)?;
            let styles = self.styles();
            if !styles.is_empty() {
                value["curves"] = serde_json::to_value(styles).map_err(S::Error::custom)?;
            }
        } else {
            let plots: Vec<_> = self
                .plots_in(&|_| None)
                .into_iter()
                .map(|p| StoredCurve {
                    base_missing: self
                        .entry(&p.key)
                        .and_then(|p| p.base.as_ref())
                        .is_some_and(|r| r.col.is_empty()),
                    mapping: p.mapping,
                    style: p.style,
                    x_override: p.x_override,
                })
                .collect();
            value["plots"] = serde_json::to_value(plots).map_err(S::Error::custom)?;
        }
        value.serialize(serializer)
    }
}
#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for LayerSpec {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        use serde::de::Error;
        let mut value = serde_json::Value::deserialize(deserializer)?;
        let plots = value.get("plots").cloned();
        if plots.is_some() {
            // 与旧读取规则一致：plots 是权威数据，忽略同时存在的旧平行字段。
            for key in ["y_keys", "x_by_y", "base_by_y", "row_by_y", "line_only", "curves"] {
                value.as_object_mut().unwrap().remove(key);
            }
            value["y_keys"] = serde_json::json!([]);
        }
        let draft: PlotSpec = serde_json::from_value(value.clone()).map_err(D::Error::custom)?;
        if let Some(plots) = plots {
            let plots: Vec<StoredCurve> = serde_json::from_value(plots).map_err(D::Error::custom)?;
            let mut result = Self::from(PlotSpec {
                book_id: draft.book_id,
                x_key: draft.x_key,
                plot_type: draft.plot_type,
                rows: draft.rows,
                transform: draft.transform,
                color_map: draft.color_map,
                ..Default::default()
            });
            let mut seen = std::collections::BTreeSet::new();
            for (order, p) in plots.into_iter().enumerate() {
                p.mapping.validate().map_err(D::Error::custom)?;
                if p.base_missing && p.mapping.base.is_some() {
                    return Err(D::Error::custom("曲线下端不能同时为缺失和已指定"));
                }
                if !seen.insert(p.mapping.y.clone()) {
                    return Err(D::Error::custom("曲线行范围无效或来源重复"));
                }
                let main = result.book_id;
                let m = p.mapping;
                let inherited_rows = if Some(m.y.book) == main && !m.line_only {
                    result.rows
                } else {
                    None
                };
                let inherited_x = if Some(m.y.book) == main {
                    result.x_key.as_ref().map(|x| DataRef::new(m.y.book, x))
                } else {
                    None
                };
                result.plots.push(Plot {
                    source: raw_ref(&m.y.key(main)),
                    active: true,
                    x: m.x
                        .filter(|x| p.x_override || Some(x) != inherited_x.as_ref())
                        .map(|r| raw_ref(&r.key(main))),
                    base: m
                        .base
                        .map(|r| raw_ref(&r.key(main)))
                        .or_else(|| p.base_missing.then(|| raw_ref(""))),
                    rows: if m.rows == inherited_rows { None } else { Some(m.rows) },
                    line_order: m.line_only.then_some(order),
                    style: p.style,
                });
            }
            result.normalize();
            Ok(result)
        } else {
            let styles = value
                .get("curves")
                .cloned()
                .map(serde_json::from_value)
                .transpose()
                .map_err(D::Error::custom)?
                .unwrap_or_default();
            Ok(Self::from_draft(draft, styles))
        }
    }
}
