//! 工作表的单元格选区和编辑状态（支持拖动选择一块区域，
//! 双击、F2、回车或直接打字进入编辑）。

use scicanvas_core::{column_letter, Book};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cell {
    pub c: usize,
    pub r: usize,
}

/// 单元格区域。a 是起点（活动单元格），b 是拖动或 Shift 扩展到的另一角。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Range {
    pub a: Cell,
    pub b: Cell,
}

impl Range {
    pub fn single(c: usize, r: usize) -> Self {
        Range {
            a: Cell { c, r },
            b: Cell { c, r },
        }
    }
    pub fn c0(&self) -> usize {
        self.a.c.min(self.b.c)
    }
    pub fn c1(&self) -> usize {
        self.a.c.max(self.b.c)
    }
    pub fn r0(&self) -> usize {
        self.a.r.min(self.b.r)
    }
    pub fn r1(&self) -> usize {
        self.a.r.max(self.b.r)
    }
    pub fn is_multi(&self) -> bool {
        self.a != self.b
    }
    pub fn contains(&self, c: usize, r: usize) -> bool {
        c >= self.c0() && c <= self.c1() && r >= self.r0() && r <= self.r1()
    }
    pub fn rows(&self) -> usize {
        self.r1() - self.r0() + 1
    }
    pub fn cols(&self) -> usize {
        self.c1() - self.c0() + 1
    }
    /// 删列后保证列号有效。
    pub fn clamp(&mut self, ncols: usize) {
        let m = ncols.saturating_sub(1);
        self.a.c = self.a.c.min(m);
        self.b.c = self.b.c.min(m);
    }
    /// 例如 “A1:C5”。
    pub fn label(&self, book: &Book) -> String {
        let name = |c: usize| {
            book.columns
                .get(c)
                .map(|x| x.key.clone())
                .unwrap_or_else(|| column_letter(c))
        };
        if self.is_multi() {
            format!(
                "{}{}:{}{}",
                name(self.c0()),
                self.r0() + 1,
                name(self.c1()),
                self.r1() + 1
            )
        } else {
            format!("{}{}", name(self.a.c), self.a.r + 1)
        }
    }
}

/// 正在拖动选择什么。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Drag {
    Cells,
    Rows,
    /// 拖动列标题，记下起始列。
    Cols(usize),
}

/// 正在编辑的单元格。
#[derive(Clone, Debug)]
pub struct Edit {
    pub cell: Cell,
    pub buf: String,
    /// 是否已经把键盘焦点交给编辑框。
    pub focused: bool,
}

#[derive(Clone, Debug)]
pub struct FilterCache {
    filter: scicanvas_core::filter::Filter,
    versions: Vec<(String, u64)>,
    nrows: usize,
    rows: std::sync::Arc<[usize]>,
}

#[derive(Clone, Debug, Default)]
pub struct SheetState {
    pub range: Option<Range>,
    pub drag: Option<Drag>,
    /// 拖动是否已经移出起始格（用于区分单击和拖动）。
    pub moved: bool,
    pub edit: Option<Edit>,
    /// 下一帧要滚动到的行（键盘移动时保持活动单元格可见）。
    pub scroll_to: Option<usize>,
    /// Frozen data columns are session-only and do not enter the project file.
    pub frozen_columns: usize,
    pub frozen_scroll_y: f32,
    pub viewport_width: f32,
    pub filter_cache: Option<FilterCache>,
}

impl SheetState {
    pub fn filtered_rows(&mut self, book: &scicanvas_core::Book) -> Option<std::sync::Arc<[usize]>> {
        let Some(filter) = &book.filter else {
            self.filter_cache = None;
            return None;
        };
        let versions = filter
            .rules
            .keys()
            .filter_map(|key| book.column(key).map(|column| (key.clone(), column.data.version())))
            .collect::<Vec<_>>();
        let valid = self
            .filter_cache
            .as_ref()
            .is_some_and(|cache| cache.filter == *filter && cache.versions == versions && cache.nrows == book.nrows());
        if !valid {
            self.filter_cache = Some(FilterCache {
                filter: filter.clone(),
                versions,
                nrows: book.nrows(),
                rows: book.visible_rows().into(),
            });
        }
        self.filter_cache.as_ref().map(|cache| cache.rows.clone())
    }

    pub fn cached_rows(&self) -> Option<std::sync::Arc<[usize]>> {
        self.filter_cache.as_ref().map(|cache| cache.rows.clone())
    }

    pub fn step_row(&self, row: usize, delta: i64) -> usize {
        step_visible_row(self.filter_cache.as_ref().map(|cache| cache.rows.as_ref()), row, delta)
    }
}

pub fn step_visible_row(rows: Option<&[usize]>, row: usize, delta: i64) -> usize {
    let Some(rows) = rows else {
        return (row as i64 + delta).max(0) as usize;
    };
    if rows.is_empty() {
        return row;
    }
    let index = rows.partition_point(|&value| value < row).min(rows.len() - 1);
    let next = (index as i64 + delta).clamp(0, rows.len() as i64 - 1) as usize;
    rows[next]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn range_bounds() {
        let r = Range {
            a: Cell { c: 2, r: 5 },
            b: Cell { c: 0, r: 1 },
        };
        assert_eq!((r.c0(), r.c1(), r.r0(), r.r1()), (0, 2, 1, 5));
        assert!(r.contains(1, 3));
        assert!(!r.contains(3, 3));
        assert_eq!((r.rows(), r.cols()), (5, 3));
        assert!(r.is_multi());
        assert!(!Range::single(1, 1).is_multi());
    }
}
