//! 曲线访问的过渡入口；数据仍写入原有平行映射表。
use super::Layer;
use crate::plotspec::{default_x, split_key, BookLookup, DataRef, Plot};

impl Layer {
    /// 按现有曲线顺序返回只读描述快照；默认 X 的解析复用原逻辑。
    pub fn plots(&self, books: BookLookup<'_, '_>) -> Vec<Plot> {
        self.plots_in(books)
    }

    /// 按明确来源定位曲线，同名列不会跨表混淆，也不会生成未添加的曲线。
    pub fn plot(&self, source: &DataRef, books: BookLookup<'_, '_>) -> Option<Plot> {
        self.plot_in(self.plot_key(source)?, books)
    }

    /// 在闭包中编辑完整描述，校验通过后一次写回；返回是否实际改变。
    /// 来源和旧键保持不变，失败时原图层不变；不依赖临时对象的析构来提交。
    pub fn plot_mut(
        &mut self,
        source: &DataRef,
        books: BookLookup<'_, '_>,
        edit: impl FnOnce(&mut Plot),
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

    /// 按来源去重并追加，旧键由来源生成；不改绑主表或改变图层默认设置。
    pub fn add_plot(&mut self, mut plot: Plot, books: BookLookup<'_, '_>) -> Result<bool, String> {
        plot.mapping.validate()?;
        if self.plot_key(&plot.mapping.y).is_some() {
            return Ok(false);
        }
        plot.key = plot.mapping.y.key(self.book_id);
        self.store_plot(&plot, None, books)?;
        self.spec.y_keys.push(plot.key);
        Ok(true)
    }

    /// 移除该来源的曲线及六张表中的专属项，同时识别旧的限定键写法。
    pub fn remove_plot(&mut self, source: &DataRef) -> bool {
        let mut removed = false;
        while let Some(key) = self.plot_key(source).map(str::to_owned) {
            self.remove_curve(&key);
            removed = true;
        }
        removed
    }

    /// 按描述筛选，保留原顺序，返回移除数量。未绑定主表的旧裸键原样保留。
    pub fn retain_plots(&mut self, books: BookLookup<'_, '_>, mut keep: impl FnMut(&Plot) -> bool) -> usize {
        let gone: Vec<_> = self
            .plots(books)
            .into_iter()
            .filter(|p| !keep(p))
            .map(|p| p.key)
            .collect();
        let before = self.y_keys.len();
        // ponytail: 旧平行表逐键清理，2.3 换存储后改为单次 retain。
        for key in gone {
            self.remove_curve(&key);
        }
        before - self.y_keys.len()
    }

    /// 删列的过渡入口：在用曲线统一筛选，隐藏曲线的旧项也按原规则清理。
    pub(crate) fn remove_columns(&mut self, book: u64, keys: &[String], books: BookLookup<'_, '_>) {
        if self.book_id != Some(book) {
            self.drop_foreign(book, Some(keys));
            return;
        }
        self.retain_plots(books, |plot| !keys.contains(&plot.key));
        self.spec.line_only.retain(|k| !keys.contains(k));
        self.spec.row_by_y.retain(|k, _| !keys.contains(k));
        self.curves.retain(|k, _| !keys.contains(k));
        self.spec.x_by_y.retain(|y, x| !keys.contains(y) && !keys.contains(x));
        self.spec.base_by_y.retain(|y, _| !keys.contains(y));
        if self.labels.column.as_ref().is_some_and(|k| keys.contains(k)) {
            self.labels.column = None;
        }
        // 保留旧的缺失下端标记，避免把浮动柱静默改成从零开始。
        for base in self.spec.base_by_y.values_mut() {
            if keys.contains(base) {
                base.clear();
            }
        }
        if self.x_key.as_ref().is_some_and(|k| keys.contains(k)) {
            self.spec.x_key = None;
        }
    }

    /// 映射旧存储中的独立范围，包含隐藏曲线；保留显式覆盖与继承的区别。
    pub(crate) fn map_row_ranges(&mut self, book: u64, map: impl Fn(&mut Option<(usize, usize)>)) {
        let main = self.book_id;
        if main == Some(book) {
            map(&mut self.spec.rows);
        }
        for (key, range) in &mut self.spec.row_by_y {
            if DataRef::from_key(main, key).is_some_and(|r| r.book == book) {
                map(range);
            }
        }
    }

    fn plot_key(&self, source: &DataRef) -> Option<&str> {
        self.y_keys
            .iter()
            .find(|key| {
                let (col, book) = split_key(key);
                col == source.col && book.or(self.book_id) == Some(source.book)
            })
            .map(String::as_str)
    }

    fn store_plot(&mut self, plot: &Plot, before: Option<&Plot>, books: BookLookup<'_, '_>) -> Result<(), String> {
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

        // 未修改的项保留原始继承/覆盖关系，不把读出的默认值固化到旧表。
        let key = &plot.key;
        if change_x {
            if let Some(x) = m.x.as_ref().filter(|_| override_x) {
                self.spec.x_by_y.insert(key.clone(), x.key(self.book_id));
            } else {
                self.spec.x_by_y.remove(key);
            }
        }
        if before.is_none_or(|b| m.base != b.mapping.base) {
            if let Some(base) = &m.base {
                self.spec.base_by_y.insert(key.clone(), base.key(self.book_id));
            } else {
                self.spec.base_by_y.remove(key);
            }
        }
        if before.is_none_or(|b| m.rows != b.mapping.rows || m.line_only != b.mapping.line_only) {
            let inherited = if local && !m.line_only { self.rows } else { None };
            if m.rows == inherited {
                self.spec.row_by_y.remove(key);
            } else {
                self.spec.row_by_y.insert(key.clone(), m.rows);
            }
        }
        if before.is_none_or(|b| m.line_only != b.mapping.line_only) {
            self.spec.line_only.retain(|k| k != key);
            if m.line_only {
                self.spec.line_only.push(key.clone());
            }
        }
        if before.is_none_or(|b| plot.style != b.style) {
            if let Some(style) = &plot.style {
                self.curves.insert(key.clone(), style.clone());
            } else {
                self.curves.remove(key);
            }
        }
        Ok(())
    }
}
