//! Persistent worksheet filter rules; source rows are never removed.
use std::collections::BTreeMap;
use crate::Book;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Filter {
    pub rules: BTreeMap<String, FilterRule>,
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum FilterRule {
    Number { min: Option<f64>, max: Option<f64> },
    Text { contains: String },
    Category { values: Vec<String> },
}

impl Filter {
    pub fn validate(&self, book: &Book) -> Result<(), String> {
        for (key, rule) in &self.rules {
            if book.column(key).is_none() { return Err(format!("筛选列 {key} 不存在")); }
            if let FilterRule::Number { min, max } = rule {
                if min.is_some_and(|v| !v.is_finite()) || max.is_some_and(|v| !v.is_finite()) || min.zip(*max).is_some_and(|(a, b)| a > b) {
                    return Err(format!("筛选列 {key} 的数值范围无效"));
                }
            }
        }
        Ok(())
    }
}

impl Book {
    pub fn row_visible(&self, row: usize) -> bool {
        self.filter.as_ref().is_none_or(|filter| filter.rules.iter().all(|(key, rule)| {
            let Some(column) = self.column(key) else { return false };
            match rule {
                FilterRule::Number { min, max } => column.data.f64_at(row).is_some_and(|value| value.is_finite() && min.is_none_or(|min| value >= min) && max.is_none_or(|max| value <= max)),
                FilterRule::Text { contains } => column.data.text_at(row).contains(contains),
                FilterRule::Category { values } => values.iter().any(|value| value == column.data.text_at(row).as_ref()),
            }
        }))
    }

    /// Fingerprint filter rules and input columns for the plot cache.
    pub fn filter_fingerprint(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut hash = std::collections::hash_map::DefaultHasher::new();
        if let Some(filter) = &self.filter {
            for (key, rule) in &filter.rules {
                key.hash(&mut hash);
                self.column(key).map(|c| c.data.version()).hash(&mut hash);
                match rule {
                    FilterRule::Number { min, max } => { 0_u8.hash(&mut hash); min.map(f64::to_bits).hash(&mut hash); max.map(f64::to_bits).hash(&mut hash); }
                    FilterRule::Text { contains } => { 1_u8.hash(&mut hash); contains.hash(&mut hash); }
                    FilterRule::Category { values } => { 2_u8.hash(&mut hash); values.hash(&mut hash); }
                }
            }
        }
        hash.finish()
    }

    pub fn visible_rows(&self) -> Vec<usize> {
        (0..self.nrows()).filter(|&row| self.row_visible(row)).collect()
    }

    pub fn visible_range_rows(&self, first: usize, last: usize) -> Vec<usize> {
        if self.filter.is_none() { return (first..=last).collect(); }
        if self.nrows() == 0 { return Vec::new(); }
        (first..=last.min(self.nrows().saturating_sub(1))).filter(|&row| self.row_visible(row)).collect()
    }

    pub fn visible_range_tsv(&self, c0: usize, c1: usize, r0: usize, r1: usize) -> String {
        if self.filter.is_none() { return self.range_tsv(c0, c1, r0, r1); }
        self.visible_range_rows(r0, r1).into_iter().map(|row| self.range_tsv(c0, c1, row, row)).collect::<Vec<_>>().join("\n")
    }

    pub fn clear_visible_range(&mut self, c0: usize, c1: usize, r0: usize, r1: usize) -> usize {
        if self.filter.is_none() { self.clear_range(c0, c1, r0, r1); return r1.saturating_sub(r0) + 1; }
        let rows = self.visible_range_rows(r0, r1);
        for &row in &rows { self.clear_range(c0, c1, row, row); }
        rows.len()
    }

    pub fn fill_visible_range(&mut self, c0: usize, c1: usize, r0: usize, r1: usize, value: &str) -> usize {
        if self.filter.is_none() { self.fill_range(c0, c1, r0, r1, value); return r1.saturating_sub(r0) + 1; }
        let rows = self.visible_range_rows(r0, r1);
        for &row in &rows { self.fill_range(c0, c1, row, row, value); }
        rows.len()
    }
}
