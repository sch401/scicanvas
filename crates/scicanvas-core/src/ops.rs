//! 在项目上执行分析：确定输入数据、调用 analysis.rs 计算、把结果写成新列并更新图形。
//! 规则与 Origin 一致：结果列紧挨着原始列，拟合曲线加到图里显示为红色实线。

use crate::analysis::{self, Describe, NormalizeMethod, PolyFit, SmoothMethod};
use crate::interpolate::{self, InterpMethod};
use crate::model::{fmt_num, parse_num, Book, Column, Graph, PlotType, Project, Role};

/// 统计类操作选中的数据：工作簿编号、列标识和可选的行范围。
type Picked = (u64, Vec<String>, Option<(usize, usize)>);

/// 分析的输入来自哪里。
#[derive(Clone, Debug, PartialEq)]
pub enum Source {
    /// 当前图形里的数据曲线（不含拟合、平滑等结果曲线）。
    Graph(u64),
    /// 指定页面中的图层；Graph 保留为第一图层的兼容入口。
    Layer { graph_id: u64, index: usize },
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
    pub y: String,
    pub x: Option<String>,
    pub rows: Option<(usize, usize)>,
}

pub struct Resolved {
    pub book_id: u64,
    pub graph_id: Option<u64>,
    pub layer_index: usize,
    pub items: Vec<Item>,
}

/// 一条拟合的结果，供界面显示报告。
#[derive(Clone, Debug)]
pub struct FitReport {
    pub name: String,
    pub kind: String,
    pub fit: PolyFit,
}

#[derive(Clone, Debug)]
pub struct AreaReport {
    pub name: String,
    pub area: f64,
    pub abs_area: f64,
    pub n: usize,
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
    let (mut xs, mut ys, mut idx) = (vec![], vec![], vec![]);
    for i in i0..i1 {
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
fn x_for(book: &Book, y: &Column, among: &[&Column]) -> Option<String> {
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

fn name_of(book: &Book, key: &str) -> String {
    book.column(key)
        .map(|c| c.display_name().to_string())
        .unwrap_or_else(|| key.to_string())
}

impl Project {
    /// 把分析输入解析成具体的数据组。
    pub fn resolve(&self, src: &Source) -> Result<Resolved, String> {
        match src {
            Source::Graph(gid) | Source::Layer { graph_id: gid, .. } => {
                let layer_index = match src {
                    Source::Layer { index, .. } => *index,
                    _ => 0,
                };
                let graph = self.graph(*gid).ok_or("找不到图形")?;
                let g = graph.page.layers().get(layer_index).ok_or("找不到图层")?;
                if g.transform != crate::plotspec::Transform::Identity {
                    return Err("统计变换图层的分析请从工作簿选择原始数据执行".into());
                }
                let bid = g.book_id.ok_or("这个图形没有关联数据")?;
                let book = self.book(bid).ok_or("这个图形没有关联数据")?;
                let items: Vec<Item> = g
                    .y_keys
                    .iter()
                    .filter(|k| {
                        !g.line_only.contains(k)
                            && book
                                .column(k)
                                .is_some_and(|c| !matches!(c.role, Role::XError | Role::YError))
                    })
                    .map(|k| Item {
                        y: k.clone(),
                        x: g.x_by_y.get(k).cloned().or_else(|| g.x_key.clone()),
                        rows: g.rows,
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
                        y: y.key.clone(),
                        x: x_for(book, y, &cols),
                        rows: *rows,
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

    /// 在 after_key 列右边插入结果列（排在同一来源已有结果列之后），返回新列标识。
    pub fn insert_result_column(&mut self, book_id: u64, after_key: &str, mut col: Column) -> Option<String> {
        let book = self.book_mut(book_id)?;
        book.freeze_error_sources();
        let key = book.next_key();
        col.key = key.clone();
        col.derived_from = Some(after_key.to_string());
        let mut at = book
            .column_index(after_key)
            .map(|i| i + 1)
            .unwrap_or(book.columns.len());
        while at < book.columns.len() && book.columns[at].derived_from.as_deref() == Some(after_key) {
            at += 1;
        }
        book.columns.insert(at, col);
        Some(key)
    }

    /// 结果曲线要加到哪张图：分析的来源图；否则已打开的、含这条 Y 的图；都没有就新建散点图。
    fn graph_for_result(&mut self, r: &Resolved, item: &Item) -> (u64, usize) {
        if let Some(g) = r.graph_id {
            return (g, r.layer_index);
        }
        for graph in &self.graphs {
            if let Some(index) = graph.page.layers().iter().position(|g| {
                g.book_id == Some(r.book_id)
                    && g.y_keys.contains(&item.y)
                    && g.transform == crate::plotspec::Transform::Identity
            }) {
                return (graph.id, index);
            }
        }
        let id = self.alloc_id();
        let mut g = Graph::new(id, self.unique_name("Graph"));
        g.first_layer_mut().book_id = Some(r.book_id);
        g.first_layer_mut().x_key = item.x.clone();
        g.first_layer_mut().y_keys = vec![item.y.clone()];
        g.first_layer_mut().plot_type = PlotType::Scatter;
        g.first_layer_mut().rows = item.rows;
        self.graphs.push(g);
        self.move_project_item(id, self.folder_of(r.book_id)).unwrap();
        (id, 0)
    }

    /// 多项式拟合（阶数 1 即线性拟合）。每条数据新增“拟合 X”和拟合值两列（200 个等间距点），
    /// 并把拟合曲线加到图里。返回 (图形, 各条数据的拟合报告)。
    pub fn fit_poly(&mut self, src: &Source, order: usize) -> Result<(u64, Vec<FitReport>), String> {
        let r = self.resolve(src)?;
        let kind = if order == 1 {
            "线性拟合".to_string()
        } else {
            format!("{} 阶多项式拟合", order)
        };
        let mut reports = vec![];
        let mut graph = None;
        let mut skipped = vec![];
        for item in &r.items {
            let book = self.book(r.book_id).unwrap();
            let (xs, ys, _) = get_xy(book, item);
            let name = name_of(book, &item.y);
            let Some(fit) = analysis::polyfit(&xs, &ys, order) else {
                skipped.push(name);
                continue;
            };
            let x_units = item
                .x
                .as_deref()
                .and_then(|k| book.column(k))
                .map(|c| c.units.clone())
                .unwrap_or_default();
            let y_units = book.column(&item.y).map(|c| c.units.clone()).unwrap_or_default();
            const N: usize = 200;
            let fx: Vec<f64> = (0..N)
                .map(|i| fit.x_min + (fit.x_max - fit.x_min) * i as f64 / (N - 1) as f64)
                .collect();
            let mut cx = Column::new("", Role::X);
            cx.long_name = format!("{} 拟合 X", name);
            cx.units = x_units;
            cx.comment = kind.clone();
            cx.data = fx.iter().map(|x| fmt_num(*x)).collect();
            let mut cy = Column::new("", Role::Y);
            cy.long_name = format!("{} {}", name, kind);
            cy.units = y_units;
            cy.comment = fit.equation();
            cy.data = fx.iter().map(|x| fmt_num(fit.eval(*x))).collect();
            let kx = self.insert_result_column(r.book_id, &item.y, cx).unwrap();
            let ky = self.insert_result_column(r.book_id, &item.y, cy).unwrap();
            let (gid, index) = self.graph_for_result(&r, item);
            let g = &mut self.graph_mut(gid).unwrap().page.layers_mut()[index];
            if !g.y_keys.contains(&item.y) {
                g.y_keys.push(item.y.clone());
            }
            g.y_keys.push(ky.clone());
            g.x_by_y.insert(ky.clone(), kx);
            g.line_only.push(ky);
            graph = Some(gid);
            reports.push(FitReport {
                name,
                kind: kind.clone(),
                fit,
            });
        }
        match graph {
            Some(g) => Ok((g, reports)),
            None => Err(format!(
                "{}：有效数据点太少，{}至少需要 {} 个点",
                skipped.join("、"),
                kind,
                order + 2
            )),
        }
    }

    /// 分析输入里全部有效点的 X 范围（插值对话框的默认范围）。
    pub fn x_range(&self, src: &Source) -> Option<(f64, f64)> {
        let r = self.resolve(src).ok()?;
        let book = self.book(r.book_id)?;
        let xs: Vec<f64> = r
            .items
            .iter()
            .flat_map(|item| {
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

    /// 在等间距的 `points` 个 X 上插值（`range` 为空时用数据的 X 范围，超出数据时外推），
    /// 结果写成新的 X、Y 列并作为线加到图上。
    pub fn interpolate(
        &mut self,
        src: &Source,
        method: InterpMethod,
        points: usize,
        range: Option<(f64, f64)>,
    ) -> Result<(u64, Vec<String>), String> {
        let r = self.resolve(src)?;
        let kind = format!("{}插值", method.label());
        let mut graph = None;
        let mut made = vec![];
        let mut problems = vec![];
        for item in &r.items {
            let book = self.book(r.book_id).unwrap();
            let (xs, ys, _) = get_xy(book, item);
            let name = name_of(book, &item.y);
            let (lo, hi) = range.unwrap_or_else(|| {
                let finite = || xs.iter().zip(&ys).filter(|(x, y)| x.is_finite() && y.is_finite());
                (
                    finite().map(|(x, _)| *x).fold(f64::INFINITY, f64::min),
                    finite().map(|(x, _)| *x).fold(f64::NEG_INFINITY, f64::max),
                )
            });
            let fx = interpolate::grid(lo, hi, points);
            let fy = match interpolate::interpolate(&xs, &ys, method, &fx) {
                Ok(fy) if lo.is_finite() && hi.is_finite() && lo < hi => fy,
                Ok(_) => {
                    problems.push(format!("{name}：X 范围无效"));
                    continue;
                }
                Err(e) => {
                    problems.push(format!("{name}：{e}"));
                    continue;
                }
            };
            let x_units = item
                .x
                .as_deref()
                .and_then(|k| book.column(k))
                .map(|c| c.units.clone())
                .unwrap_or_default();
            let y_units = book.column(&item.y).map(|c| c.units.clone()).unwrap_or_default();
            let note = format!("{kind}，{} 点，X 从 {} 到 {}", fx.len(), fmt_num(lo), fmt_num(hi));
            let mut cx = Column::new("", Role::X);
            cx.long_name = format!("{name} 插值 X");
            cx.units = x_units;
            cx.comment = note.clone();
            cx.data = fx.iter().map(|x| fmt_num(*x)).collect();
            let mut cy = Column::new("", Role::Y);
            cy.long_name = format!("{name} {kind}");
            cy.units = y_units;
            cy.comment = note;
            cy.data = fy.iter().map(|y| fmt_num(*y)).collect();
            let kx = self.insert_result_column(r.book_id, &item.y, cx).unwrap();
            let ky = self.insert_result_column(r.book_id, &item.y, cy).unwrap();
            let (gid, index) = self.graph_for_result(&r, item);
            let g = &mut self.graph_mut(gid).unwrap().page.layers_mut()[index];
            if !g.y_keys.contains(&item.y) {
                g.y_keys.push(item.y.clone());
            }
            g.y_keys.push(ky.clone());
            g.x_by_y.insert(ky.clone(), kx);
            g.line_only.push(ky.clone());
            graph = Some(gid);
            made.push(ky);
        }
        match graph {
            Some(g) => Ok((g, made)),
            None => Err(problems.join("；")),
        }
    }

    /// 对每组数据计算一列结果并写在原列右边。`f` 返回 (结果值, 长名称, 单位, 注释)。
    fn per_item_column(
        &mut self,
        src: &Source,
        mut f: impl FnMut(&Book, &Item, &[f64], &[f64]) -> (Vec<f64>, String, String, String),
    ) -> Result<(Resolved, Vec<(Item, String)>), String> {
        let r = self.resolve(src)?;
        let mut made = vec![];
        for item in &r.items {
            let book = self.book(r.book_id).unwrap();
            let (xs, ys, idx) = get_xy(book, item);
            if xs.len() < 3 {
                continue;
            }
            let (vals, long_name, units, comment) = f(book, item, &xs, &ys);
            let nrows = book.nrows();
            let mut data = vec![String::new(); nrows];
            for (k, &row) in idx.iter().enumerate() {
                if vals[k].is_finite() {
                    data[row] = fmt_num(vals[k]);
                }
            }
            while data.last().is_some_and(|s| s.is_empty()) {
                data.pop();
            }
            let mut c = Column::new("", Role::Y);
            c.long_name = long_name;
            c.units = units;
            c.comment = comment;
            c.data = data;
            let key = self.insert_result_column(r.book_id, &item.y, c).unwrap();
            made.push((item.clone(), key));
        }
        if made.is_empty() {
            return Err("有效数据点太少（至少需要 3 个）".into());
        }
        Ok((r, made))
    }

    /// 平滑。来源是图形时，平滑曲线也加到图里。返回新列标识。
    pub fn smooth(&mut self, src: &Source, method: SmoothMethod, window: usize) -> Result<Vec<String>, String> {
        let (r, made) = self.per_item_column(src, |book, item, xs, ys| {
            let y = book.column(&item.y).unwrap();
            (
                analysis::smooth(xs, ys, method, window),
                format!("{} 平滑", y.display_name()),
                y.units.clone(),
                format!("{}，{} 点", method.label(), window.max(3) | 1),
            )
        })?;
        if let Some(gid) = r.graph_id {
            let g = &mut self.graph_mut(gid).unwrap().page.layers_mut()[r.layer_index];
            for (item, key) in &made {
                g.y_keys.push(key.clone());
                g.line_only.push(key.clone());
                if item.x != g.x_key {
                    if let Some(x) = &item.x {
                        g.x_by_y.insert(key.clone(), x.clone());
                    }
                }
            }
        }
        Ok(made.into_iter().map(|(_, k)| k).collect())
    }

    /// 数值微分，结果写成新列 d(Y)/dX。
    pub fn differentiate(&mut self, src: &Source) -> Result<Vec<String>, String> {
        let (_, made) = self.per_item_column(src, |book, item, xs, ys| {
            let y = book.column(&item.y).unwrap();
            let xu = item
                .x
                .as_deref()
                .and_then(|k| book.column(k))
                .map(|c| c.units.clone())
                .unwrap_or_default();
            let units = if !y.units.is_empty() && !xu.is_empty() {
                format!("{}/{}", y.units, xu)
            } else {
                String::new()
            };
            (
                analysis::derivative(xs, ys),
                format!("d({})/dX", y.display_name()),
                units,
                "中心差分".into(),
            )
        })?;
        Ok(made.into_iter().map(|(_, k)| k).collect())
    }

    /// 梯形法累积积分，结果写成新列，并返回每组数据的总面积。
    pub fn integrate(&mut self, src: &Source) -> Result<(Vec<String>, Vec<AreaReport>), String> {
        let mut areas = vec![];
        let (_, made) = self.per_item_column(src, |book, item, xs, ys| {
            let y = book.column(&item.y).unwrap();
            let (cum, abs) = analysis::cumulative_trapezoid(xs, ys);
            let total = *cum.last().unwrap_or(&0.0);
            areas.push(AreaReport {
                name: y.display_name().to_string(),
                area: total,
                abs_area: abs,
                n: xs.len(),
            });
            let xu = item
                .x
                .as_deref()
                .and_then(|k| book.column(k))
                .map(|c| c.units.clone())
                .unwrap_or_default();
            let units = if !y.units.is_empty() && !xu.is_empty() {
                format!("{}·{}", y.units, xu)
            } else {
                String::new()
            };
            (
                cum,
                format!("∫{} dX", y.display_name()),
                units,
                format!("梯形法累积积分，总面积 {}", fmt_num(total)),
            )
        })?;
        Ok((made.into_iter().map(|(_, k)| k).collect(), areas))
    }

    /// 归一化，结果写成新列。
    pub fn normalize(&mut self, src: &Source, method: NormalizeMethod) -> Result<Vec<String>, String> {
        let (_, made) = self.per_item_column(src, |book, item, _xs, ys| {
            let y = book.column(&item.y).unwrap();
            (
                analysis::normalize(ys, method),
                format!("{} 归一化", y.display_name()),
                String::new(),
                method.label().into(),
            )
        })?;
        Ok(made.into_iter().map(|(_, k)| k).collect())
    }

    /// 列统计：结果放进新的工作簿，每行是一列数据的统计量。返回新工作簿。
    /// 来源是工作簿时统计选中的列（任意列属性），没选时统计全部 Y 列。
    /// 统计类操作的输入：工作簿、要统计的列（图层里的曲线或工作簿中选中的列，没选时用全部 Y 列）和行范围。
    fn picked_columns(&self, src: &Source) -> Result<Picked, String> {
        Ok(match src {
            Source::Graph(_) | Source::Layer { .. } => {
                let r = self.resolve(src)?;
                (
                    r.book_id,
                    r.items.iter().map(|i| i.y.clone()).collect(),
                    r.items[0].rows,
                )
            }
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
                (*id, keys, *rows)
            }
        })
    }

    pub fn stats_on_columns(&mut self, src: &Source) -> Result<u64, String> {
        let (book_id, keys, rows) = self.picked_columns(src)?;
        let book = self.book(book_id).ok_or("找不到工作簿")?;
        let src_name = book.name.clone();
        let stats: Vec<(String, Describe)> = keys
            .iter()
            .filter_map(|k| {
                let c = book.column(k)?;
                let mut v = c.values();
                if let Some((a, b)) = rows {
                    v = v.into_iter().skip(a).take(b + 1 - a).collect();
                }
                analysis::describe(&v).map(|d| (format!("{} · {}", c.key, c.display_name()), d))
            })
            .collect();
        if stats.is_empty() {
            return Err("选中的数据里没有数值".into());
        }
        let id = self.alloc_id();
        let mut nb = Book::new(id, self.unique_name("列统计"));
        let note = match rows {
            Some((a, b)) => format!("来自 {} 第 {}–{} 行", src_name, a + 1, b + 1),
            None => format!("来自 {}", src_name),
        };
        type Getter = fn(&Describe) -> f64;
        let defs: [(&str, Getter); 10] = [
            ("N", |d| d.n as f64),
            ("均值", |d| d.mean),
            ("标准差", |d| d.sd),
            ("标准误", |d| d.se),
            ("最小值", |d| d.min),
            ("下四分位数", |d| d.q1),
            ("中位数", |d| d.median),
            ("上四分位数", |d| d.q3),
            ("最大值", |d| d.max),
            ("总和", |d| d.sum),
        ];
        let k = nb.push_column(Role::Ignore);
        let c = nb.columns.iter_mut().find(|c| c.key == k).unwrap();
        c.long_name = "数据列".into();
        c.comment = note.clone();
        c.data = stats.iter().map(|(n, _)| n.clone()).collect();
        for (label, get) in defs {
            let k = nb.push_column(Role::Y);
            let c = nb.columns.iter_mut().find(|c| c.key == k).unwrap();
            c.long_name = label.into();
            c.comment = note.clone();
            c.data = stats.iter().map(|(_, d)| fmt_num(get(d))).collect();
        }
        self.books.push(nb);
        self.move_project_item(id, self.folder_of(book_id))?;
        Ok(id)
    }

    /// 统计类操作选中列的数值范围（频数统计对话框的默认范围）。
    pub fn value_range(&self, src: &Source) -> Option<(f64, f64)> {
        let (book_id, keys, rows) = self.picked_columns(src).ok()?;
        let book = self.book(book_id)?;
        let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
        for key in &keys {
            let v = book.column(key)?.values();
            let v = match rows {
                Some((a, b)) => v.into_iter().skip(a).take(b + 1 - a).collect(),
                None => v,
            };
            for x in v.into_iter().filter(|x| x.is_finite()) {
                (lo, hi) = (lo.min(x), hi.max(x));
            }
        }
        (lo < hi).then_some((lo, hi))
    }

    /// 频数统计：每个选中的列生成一个结果工作簿，列出区间下限、中点、上限、频数、累积频数、
    /// 相对频率和累积相对频率；`bins` 为空时按 Sturges 规则，`range` 为空时用数据范围。
    pub fn frequency_counts(
        &mut self,
        src: &Source,
        bins: Option<usize>,
        range: Option<(f64, f64)>,
    ) -> Result<Vec<u64>, String> {
        let (book_id, keys, rows) = self.picked_columns(src)?;
        let book = self.book(book_id).ok_or("找不到工作簿")?;
        let src_name = book.name.clone();
        let mut results = vec![];
        let mut problems = vec![];
        for key in &keys {
            let Some(c) = book.column(key) else { continue };
            let mut v = c.values();
            if let Some((a, b)) = rows {
                v = v.into_iter().skip(a).take(b + 1 - a).collect();
            }
            let n = v.iter().filter(|x| x.is_finite()).count();
            let k = bins.unwrap_or_else(|| crate::stats::sturges_bins(n));
            match crate::stats::histogram(&v, k, range) {
                Ok(b) => results.push((c.display_name().to_owned(), c.units.clone(), b)),
                Err(e) => problems.push(format!("{}：{e}", c.display_name())),
            }
        }
        if results.is_empty() {
            return Err(if problems.is_empty() {
                "选中的数据里没有数值".into()
            } else {
                problems.join("；")
            });
        }
        let folder = self.folder_of(book_id);
        let mut ids = vec![];
        for (name, units, bins) in results {
            let id = self.alloc_id();
            let mut nb = Book::new(id, self.unique_name(&format!("{name} 频数")));
            let total: usize = bins.iter().map(|b| b.count).sum();
            let note = match rows {
                Some((a, b)) => format!("来自 {src_name} 的 {name}，第 {}–{} 行", a + 1, b + 1),
                None => format!("来自 {src_name} 的 {name}"),
            };
            let mut cumulative = 0;
            let cum: Vec<usize> = bins
                .iter()
                .map(|b| {
                    cumulative += b.count;
                    cumulative
                })
                .collect();
            let share = |c: usize| if total == 0 { f64::NAN } else { c as f64 / total as f64 };
            let columns: [(Role, &str, &str, Vec<f64>); 7] = [
                (Role::Ignore, "区间下限", &units, bins.iter().map(|b| b.lo).collect()),
                (
                    Role::X,
                    "区间中点",
                    &units,
                    bins.iter().map(|b| 0.5 * (b.lo + b.hi)).collect(),
                ),
                (Role::Ignore, "区间上限", &units, bins.iter().map(|b| b.hi).collect()),
                (Role::Y, "频数", "", bins.iter().map(|b| b.count as f64).collect()),
                (Role::Y, "累积频数", "", cum.iter().map(|c| *c as f64).collect()),
                (Role::Y, "相对频率", "", bins.iter().map(|b| share(b.count)).collect()),
                (Role::Y, "累积相对频率", "", cum.iter().map(|c| share(*c)).collect()),
            ];
            for (role, long_name, units, values) in columns {
                let k = nb.push_column(role);
                let c = nb.columns.iter_mut().find(|c| c.key == k).unwrap();
                c.long_name = long_name.into();
                c.units = units.into();
                c.comment = note.clone();
                c.data = values.into_iter().map(fmt_num).collect();
            }
            self.books.push(nb);
            self.move_project_item(id, folder)?;
            ids.push(id);
        }
        Ok(ids)
    }

    /// 按某一列排序整张工作表，各行数据一起移动。数值排在文本前面，空单元格排在最后。
    pub fn sort_book(&mut self, book_id: u64, col: usize, ascending: bool) -> Result<(), String> {
        let book = self.book_mut(book_id).ok_or("找不到工作簿")?;
        if col >= book.columns.len() {
            return Err("没有这一列".into());
        }
        let n = book.nrows();
        let key: Vec<String> = (0..n)
            .map(|i| book.columns[col].data.get(i).cloned().unwrap_or_default())
            .collect();
        let mut idx: Vec<usize> = (0..n).collect();
        idx.sort_by(|&i, &j| {
            use std::cmp::Ordering::*;
            let (a, b) = (key[i].trim(), key[j].trim());
            let ord = match (a.is_empty(), b.is_empty()) {
                (true, true) => Equal,
                (true, false) => return Greater,
                (false, true) => return Less,
                _ => {
                    let (na, nb) = (parse_num(a), parse_num(b));
                    match (na.is_finite(), nb.is_finite()) {
                        (true, true) => na.partial_cmp(&nb).unwrap_or(Equal),
                        (true, false) => return Less,
                        (false, true) => return Greater,
                        _ => a.cmp(b),
                    }
                }
            };
            let ord = if ascending { ord } else { ord.reverse() };
            ord.then(i.cmp(&j))
        });
        for c in &mut book.columns {
            let old = std::mem::take(&mut c.data);
            c.data = idx.iter().map(|&i| old.get(i).cloned().unwrap_or_default()).collect();
            while c.data.last().is_some_and(|s| s.trim().is_empty()) {
                c.data.pop();
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn book_src(sel: &[&str]) -> Source {
        Source::Book {
            id: 1,
            selected: sel.iter().map(|s| s.to_string()).collect(),
            rows: None,
        }
    }

    #[test]
    fn linear_fit_adds_columns_and_graph_curve() {
        let mut p = Project::sample();
        let (gid, reps) = p.fit_poly(&Source::Graph(2), 1).unwrap();
        assert_eq!(gid, 2);
        assert_eq!(reps.len(), 2);
        let book = p.book(1).unwrap();
        // 原来 A B C，现在 B 后面跟着 B 的拟合 X 和拟合值，C 后面同理
        let keys: Vec<&str> = book.columns.iter().map(|c| c.key.as_str()).collect();
        assert_eq!(keys, vec!["A", "B", "D", "E", "C", "F", "G"]);
        assert_eq!(book.column("D").unwrap().role, Role::X);
        assert_eq!(book.column("E").unwrap().data.len(), 200);
        let g = p.graph(2).unwrap().first_layer();
        assert_eq!(g.y_keys, vec!["B", "C", "E", "G"]);
        assert_eq!(g.x_by_y.get("E").map(|s| s.as_str()), Some("D"));
        assert!(g.line_only.contains(&"G".to_string()));
        // 拟合曲线不再参与下一次分析
        assert_eq!(p.resolve(&Source::Graph(2)).unwrap().items.len(), 2);
        // 图形曲线：结果曲线标记为只画线，并记录来源
        let (_, s) = g.series(p.book(1).unwrap());
        assert!(s[2].line_only && s[2].source.as_deref() == Some("B"));
        assert_eq!(s[2].points.len(), 200);
    }

    #[test]
    fn fit_from_book_creates_scatter_graph() {
        let mut p = Project::sample();
        p.graphs.clear();
        let (gid, reps) = p.fit_poly(&book_src(&["C"]), 3).unwrap();
        assert_eq!(reps[0].kind, "3 阶多项式拟合");
        let g = p.graph(gid).unwrap().first_layer();
        assert_eq!(g.plot_type, PlotType::Scatter);
        assert_eq!(g.x_key.as_deref(), Some("A"));
        assert!(reps[0].fit.r2 > 0.99);
    }

    #[test]
    fn fit_errors() {
        let mut p = Project::sample();
        assert!(p.fit_poly(&book_src(&["A"]), 1).is_err());
        let id = p.new_book();
        p.book_mut(id).unwrap().columns[1].data = vec!["1".into()];
        let src = Source::Book {
            id,
            selected: vec![],
            rows: None,
        };
        assert!(p.fit_poly(&src, 1).unwrap_err().contains("至少需要 3 个点"));
    }

    #[test]
    fn frequency_counts_build_one_table_per_column_next_to_the_source() {
        let mut p = Project::sample();
        let folder = p.add_folder(None).unwrap();
        p.move_project_item(1, Some(folder)).unwrap();
        let ids = p.frequency_counts(&book_src(&["B", "C"]), Some(4), None).unwrap();
        assert_eq!(ids.len(), 2);
        let book = p.book(ids[0]).unwrap();
        assert_eq!(book.name, "电压 A 频数1");
        assert_eq!(p.folder_of(ids[0]), Some(folder), "results sit beside the source book");
        let names: Vec<_> = book.columns.iter().map(|c| c.long_name.as_str()).collect();
        assert_eq!(
            names,
            [
                "区间下限",
                "区间中点",
                "区间上限",
                "频数",
                "累积频数",
                "相对频率",
                "累积相对频率"
            ]
        );
        assert_eq!(book.columns[1].role, Role::X);
        assert_eq!(book.columns[0].units, "mV");
        let col = |i: usize| book.columns[i].values();
        assert_eq!(col(3).len(), 4);
        assert_eq!(col(3).iter().sum::<f64>(), 41.0, "every sample value is counted once");
        assert_eq!(col(4)[3], 41.0);
        assert_eq!(col(6)[3], 1.0);
        // 单元格按 8 位有效数字保存，相加只需接近 1
        assert!((col(5).iter().sum::<f64>() - 1.0).abs() < 1e-6);

        // 默认按 Sturges 规则；自定义范围外的值不计
        let auto = p.frequency_counts(&book_src(&["B"]), None, None).unwrap();
        assert_eq!(p.book(auto[0]).unwrap().nrows(), crate::stats::sturges_bins(41));
        let narrow = p
            .frequency_counts(&book_src(&["A"]), Some(2), Some((0.0, 4.75)))
            .unwrap();
        assert_eq!(p.book(narrow[0]).unwrap().columns[3].values(), [5.0, 5.0]);
        assert!(p
            .frequency_counts(&book_src(&["B"]), Some(3), Some((5.0, 1.0)))
            .is_err());
        assert_eq!(p.value_range(&book_src(&["A"])), Some((0.0, 20.0)));
    }

    #[test]
    fn interpolation_writes_xy_columns_and_adds_lines_to_the_graph() {
        let mut p = Project::sample();
        let (gid, keys) = p
            .interpolate(&Source::Graph(2), InterpMethod::Spline, 41, None)
            .unwrap();
        assert_eq!(gid, 2);
        assert_eq!(keys.len(), 2, "one result per data curve");
        let book = p.book(1).unwrap();
        let layer = p.graph(2).unwrap().first_layer();
        for key in &keys {
            assert!(layer.line_only.contains(key) && layer.y_keys.contains(key));
            let x = book.column(&layer.x_by_y[key]).unwrap();
            let y = book.column(key).unwrap();
            assert_eq!((x.data.len(), y.data.len()), (41, 41));
            assert_eq!((x.data[0].as_str(), x.data[40].as_str()), ("0", "20"));
            assert!(y.long_name.ends_with("三次样条插值") && y.units == "mV");
        }
        // 网格落在原始 X 上（步长 0.5）时，插值结果就是原始数据
        let y = book.column(&keys[0]).unwrap();
        assert_eq!(y.data[..5], book.column("B").unwrap().data[..5]);

        let (_, wide) = p
            .interpolate(&Source::Graph(2), InterpMethod::Linear, 3, Some((-10.0, 30.0)))
            .unwrap();
        let x = p
            .book(1)
            .unwrap()
            .column(&p.graph(2).unwrap().first_layer().x_by_y[&wide[0]])
            .unwrap();
        assert_eq!(x.data, ["-10", "10", "30"], "custom ranges extrapolate");

        let id = p.new_book();
        p.book_mut(id).unwrap().columns[0].data = vec!["1".into(), "1".into()];
        p.book_mut(id).unwrap().columns[1].data = vec!["2".into(), "3".into()];
        let src = Source::Book {
            id,
            selected: vec![],
            rows: None,
        };
        assert!(p
            .interpolate(&src, InterpMethod::Akima, 10, None)
            .unwrap_err()
            .contains("重复"));
    }

    #[test]
    fn math_columns_align_with_rows() {
        let mut p = Project::sample();
        p.book_mut(1).unwrap().columns[1].data[5] = "".into(); // 中间空一格
        let keys = p.differentiate(&book_src(&["B"])).unwrap();
        let col = p.book(1).unwrap().column(&keys[0]).unwrap().clone();
        assert_eq!(col.data[5], "");
        assert!(!col.data[4].is_empty());
        assert_eq!(col.long_name, "d(电压 A)/dX");
        assert_eq!(col.units, "mV/s");
        let (_, areas) = p.integrate(&book_src(&["C"])).unwrap();
        assert!(areas[0].area > 100.0 && (areas[0].area - areas[0].abs_area).abs() < 1e-9);
        let keys = p.normalize(&book_src(&["C"]), NormalizeMethod::MinMax).unwrap();
        let v = p.book(1).unwrap().column(&keys[0]).unwrap().values();
        assert_eq!(v.iter().copied().fold(f64::NEG_INFINITY, f64::max), 1.0);
    }

    #[test]
    fn smoothing_from_graph_adds_lines() {
        let mut p = Project::sample();
        let keys = p.smooth(&Source::Graph(2), SmoothMethod::SavitzkyGolay, 7).unwrap();
        assert_eq!(keys.len(), 2);
        let g = p.graph(2).unwrap().first_layer();
        assert!(keys.iter().all(|k| g.line_only.contains(k) && g.y_keys.contains(k)));
    }

    #[test]
    fn stats_book() {
        let mut p = Project::sample();
        let id = p.stats_on_columns(&book_src(&[])).unwrap();
        let b = p.book(id).unwrap();
        assert_eq!(b.name, "列统计1");
        assert_eq!(b.columns.len(), 11);
        assert_eq!(b.nrows(), 2);
        assert_eq!(b.columns[1].data, vec!["41", "41"]);
        assert_eq!(b.columns[0].data[0], "B · 电压 A");
    }

    #[test]
    fn sorting_keeps_rows_together() {
        let mut p = Project::sample();
        let before: Vec<(String, String)> = (0..41)
            .map(|i| {
                (
                    p.book(1).unwrap().columns[0].data[i].clone(),
                    p.book(1).unwrap().columns[1].data[i].clone(),
                )
            })
            .collect();
        p.sort_book(1, 1, false).unwrap();
        let b = p.book(1).unwrap();
        let v = b.columns[1].values();
        assert!(v.windows(2).all(|w| w[0] >= w[1]));
        for i in 0..41 {
            let pair = (b.columns[0].data[i].clone(), b.columns[1].data[i].clone());
            assert!(before.contains(&pair));
        }
        // 文本和空单元格排在数值后面
        let id = p.new_book();
        p.book_mut(id).unwrap().columns[0].data = vec!["b".into(), "3".into(), "".into(), "1".into()];
        p.book_mut(id).unwrap().columns[1].data = vec!["x".into(), "y".into(), "z".into(), "w".into()];
        p.sort_book(id, 0, true).unwrap();
        assert_eq!(p.book(id).unwrap().columns[0].data, vec!["1", "3", "b"]);
        assert_eq!(p.book(id).unwrap().columns[1].data, vec!["w", "y", "x", "z"]);
    }
}
