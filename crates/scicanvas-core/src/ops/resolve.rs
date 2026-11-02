//! 分析来源、数据提取与值域解析。
use super::*;

/// 统计类操作选中的数据：工作簿编号、列标识和可选的行范围。
pub struct AnalysisColumn {
    pub source: crate::plotspec::DataRef,
    pub name: String,
    pub values: Vec<f64>,
    pub rows: Option<(usize, usize)>,
}

/// 分析的输入来自哪里。
#[derive(Clone, Debug, PartialEq)]
pub enum Source {
    /// Apply the source workbook's filter while keeping original row identities.
    Visible(Box<Source>),
    /// 当前图形里的数据曲线（不含拟合、平滑等结果曲线）。
    Graph(u64),
    /// 指定页面中的图层；Graph 保留为第一图层的兼容入口。
    Layer { graph_id: u64, index: usize },
    /// 图层内明确选中的曲线；空列表表示没有选择，不隐式改为全部。
    Curves {
        graph_id: u64,
        index: usize,
        curves: Vec<crate::plotspec::DataRef>,
    },
    /// 工作簿里选中的列；什么都没选时用全部 Y 列。`rows` 为选区覆盖的行。
    Book {
        id: u64,
        selected: Vec<String>,
        rows: Option<(usize, usize)>,
    },
}

/// 一组要分析的数据：Y 列、对应的 X 列、行范围。
#[derive(Clone, Debug, PartialEq)]
pub struct Item {
    pub book_id: u64,
    pub y: String,
    pub x: Option<String>,
    pub rows: Option<(usize, usize)>,
    pub visible_only: bool,
}

pub struct Resolved {
    pub book_id: u64,
    pub graph_id: Option<u64>,
    pub layer_index: usize,
    pub items: Vec<Item>,
}

impl Resolved {
    pub(super) fn name(&self, book: &Book, item: &Item) -> String {
        let name = name_of(book, &item.y);
        if self.items.iter().any(|i| i.book_id != book.id) {
            format!("{}：{name}", book.name)
        } else {
            name
        }
    }
}

/// 对一组数据取出有限值点：(x, y, 行号)。没有 X 列时用行号（从 1 开始）。
pub fn get_xy(book: &Book, item: &Item) -> (Vec<f64>, Vec<f64>, Vec<usize>) {
    let yv = book.column(&item.y).map(|c| c.values()).unwrap_or_default();
    let xv = item.x.as_deref().and_then(|k| book.column(k)).map(|c| c.values());
    let n = yv.len().max(xv.as_ref().map_or(0, |v| v.len()));
    let (i0, i1) = match item.rows {
        Some((a, b)) => (a, (b + 1).min(n)),
        None => (0, n),
    };
    let mask = if item.visible_only { book.visible_mask() } else { None };
    let (mut xs, mut ys, mut idx) = (vec![], vec![], vec![]);
    for i in i0..i1 {
        if !crate::filter::visible_at(mask.as_deref(), i) {
            continue;
        }
        let x = match &xv {
            Some(v) => v.get(i).copied().unwrap_or(f64::NAN),
            None => (i + 1) as f64,
        };
        let y = yv.get(i).copied().unwrap_or(f64::NAN);
        if x.is_finite() && y.is_finite() {
            xs.push(x);
            ys.push(y);
            idx.push(i);
        }
    }
    (xs, ys, idx)
}

/// Y 列对应的 X 列：先看选中的列里有没有 X，再找左边最近的 X，最后取任意 X 列。
pub(super) fn x_for(book: &Book, y: &Column, among: &[&Column]) -> Option<String> {
    let yi = book.column_index(&y.key).unwrap_or(0);
    among
        .iter()
        .find(|c| c.role == Role::X)
        .map(|c| c.key.clone())
        .or_else(|| {
            book.columns[..yi]
                .iter()
                .rev()
                .find(|c| c.role == Role::X)
                .map(|c| c.key.clone())
        })
        .or_else(|| book.columns.iter().find(|c| c.role == Role::X).map(|c| c.key.clone()))
}

pub(super) fn name_of(book: &Book, key: &str) -> String {
    book.column(key)
        .map(|c| c.display_name().to_string())
        .unwrap_or_else(|| key.to_string())
}

impl Project {
    /// 把分析输入解析成具体的数据组。
    pub fn resolve(&self, src: &Source) -> Result<Resolved, String> {
        match src {
            Source::Visible(source) => {
                let mut resolved = self.resolve(source)?;
                for item in &mut resolved.items {
                    item.visible_only = true;
                }
                Ok(resolved)
            }
            Source::Graph(gid) | Source::Layer { graph_id: gid, .. } | Source::Curves { graph_id: gid, .. } => {
                let layer_index = match src {
                    Source::Layer { index, .. } | Source::Curves { index, .. } => *index,
                    _ => 0,
                };
                let graph = self.graph(*gid).ok_or("找不到图形")?;
                let g = graph.page.layers().get(layer_index).ok_or("找不到图层")?;
                if g.transform != crate::plotspec::Transform::Identity {
                    return Err("统计变换图层的分析请从工作簿选择原始数据执行".into());
                }
                let bid = g.book_id.ok_or("这个图形没有关联数据")?;
                self.book(bid).ok_or("这个图形没有关联数据")?;
                let items: Vec<Item> = g
                    .plots(&|id| self.book(id))
                    .into_iter()
                    .filter_map(|plot| {
                        let curve = plot.mapping;
                        let source_book = self.book(curve.y.book)?;
                        if curve.line_only
                            || source_book
                                .column(&curve.y.col)
                                .is_none_or(|c| matches!(c.role, Role::XError | Role::YError))
                        {
                            return None;
                        }
                        if let Source::Curves { curves, .. } = src {
                            if !curves.contains(&curve.y) {
                                return None;
                            }
                        }
                        Some(Item {
                            book_id: curve.y.book,
                            y: curve.y.col,
                            x: curve.x.filter(|x| x.book == source_book.id).map(|x| x.col),
                            rows: curve.rows,
                            visible_only: false,
                        })
                    })
                    .collect();
                if items.is_empty() {
                    return Err("图形里没有数据曲线".into());
                }
                Ok(Resolved {
                    book_id: bid,
                    graph_id: Some(*gid),
                    layer_index,
                    items,
                })
            }
            Source::Book { id, selected, rows } => {
                let book = self.book(*id).ok_or("找不到工作簿")?;
                let all = selected.is_empty();
                let cols: Vec<&Column> = if all {
                    book.columns.iter().collect()
                } else {
                    book.columns.iter().filter(|c| selected.contains(&c.key)).collect()
                };
                let ys: Vec<&Column> = cols.iter().copied().filter(|c| c.role == Role::Y).collect();
                if ys.is_empty() {
                    return Err(if all {
                        "工作簿里没有 Y 列".into()
                    } else {
                        "选中的列里没有 Y 列，请至少选中一个 Y 列".into()
                    });
                }
                let items = ys
                    .iter()
                    .map(|y| Item {
                        book_id: *id,
                        y: y.key.clone(),
                        x: x_for(book, y, &cols),
                        rows: *rows,
                        visible_only: false,
                    })
                    .collect();
                Ok(Resolved {
                    book_id: *id,
                    graph_id: None,
                    layer_index: 0,
                    items,
                })
            }
        }
    }

    /// 分析输入里全部有效点的 X 范围（插值对话框的默认范围）。
    pub fn x_range(&self, src: &Source) -> Option<(f64, f64)> {
        let r = self.resolve(src).ok()?;
        let xs: Vec<f64> = r
            .items
            .iter()
            .flat_map(|item| {
                let book = self.book(item.book_id).unwrap();
                let (xs, ys, _) = get_xy(book, item);
                xs.into_iter()
                    .zip(ys)
                    .filter(|(x, y)| x.is_finite() && y.is_finite())
                    .map(|(x, _)| x)
                    .collect::<Vec<_>>()
            })
            .collect();
        let lo = xs.iter().copied().fold(f64::INFINITY, f64::min);
        let hi = xs.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        (lo < hi).then_some((lo, hi))
    }

    /// 列统计：结果放进新的工作簿，每行是一列数据的统计量。返回新工作簿。
    /// 来源是工作簿时统计选中的列（任意列属性），没选时统计全部 Y 列。
    /// 统计类操作的输入：工作簿、要统计的列（图层里的曲线或工作簿中选中的列，没选时用全部 Y 列）和行范围。
    fn picked_columns(&self, src: &Source) -> Result<Vec<Item>, String> {
        Ok(match src {
            Source::Visible(source) => {
                let mut items = self.picked_columns(source)?;
                for item in &mut items {
                    item.visible_only = true;
                }
                items
            }
            Source::Graph(_) | Source::Layer { .. } | Source::Curves { .. } => self.resolve(src)?.items,
            Source::Book { id, selected, rows } => {
                let book = self.book(*id).ok_or("找不到工作簿")?;
                let keys = if selected.is_empty() {
                    book.columns
                        .iter()
                        .filter(|c| c.role == Role::Y)
                        .map(|c| c.key.clone())
                        .collect()
                } else {
                    selected.clone()
                };
                keys.into_iter()
                    .map(|y| Item {
                        book_id: *id,
                        y,
                        x: None,
                        rows: *rows,
                        visible_only: false,
                    })
                    .collect()
            }
        })
    }

    pub fn analysis_columns(&self, src: &Source) -> Result<Vec<AnalysisColumn>, String> {
        let items = self.picked_columns(src)?;
        let mixed = items
            .first()
            .is_some_and(|first| items.iter().any(|i| i.book_id != first.book_id));
        items
            .iter()
            .map(|item| {
                let book = self.book(item.book_id).ok_or("找不到工作簿")?;
                let col = book.column(&item.y).ok_or("找不到数据列")?;
                let mut values = col.values();
                if item.visible_only {
                    let mask = book.visible_mask();
                    values = values
                        .into_iter()
                        .enumerate()
                        .filter(|(row, _)| {
                            item.rows.is_none_or(|(a, b)| *row >= a && *row <= b)
                                && crate::filter::visible_at(mask.as_deref(), *row)
                        })
                        .map(|(_, value)| value)
                        .collect();
                } else if let Some((a, b)) = item.rows {
                    values = values.into_iter().skip(a).take(b + 1 - a).collect();
                }
                Ok(AnalysisColumn {
                    source: crate::plotspec::DataRef::new(book.id, &item.y),
                    name: if mixed {
                        format!("{}：{}", book.name, col.display_name())
                    } else {
                        col.display_name().into()
                    },
                    values,
                    rows: item.rows,
                })
            })
            .collect()
    }

    /// 统计类操作选中列的数值范围（频数统计对话框的默认范围）。
    pub fn value_range(&self, src: &Source) -> Option<(f64, f64)> {
        let columns = self.analysis_columns(src).ok()?;
        let mut lo = f64::INFINITY;
        let mut hi = f64::NEG_INFINITY;
        for value in columns.iter().flat_map(|c| c.values.iter()).filter(|v| v.is_finite()) {
            lo = lo.min(*value);
            hi = hi.max(*value);
        }
        (lo < hi).then_some((lo, hi))
    }
}
