//! 结果列写入与图层绑定、列运算、填充和排序。
use super::*;

/// 列运算的运算符。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColumnOp {
    Add,
    Sub,
    Mul,
    Div,
}

impl ColumnOp {
    pub const ALL: [ColumnOp; 4] = [ColumnOp::Add, ColumnOp::Sub, ColumnOp::Mul, ColumnOp::Div];

    pub fn symbol(self) -> &'static str {
        match self {
            ColumnOp::Add => "+",
            ColumnOp::Sub => "−",
            ColumnOp::Mul => "×",
            ColumnOp::Div => "÷",
        }
    }

    fn apply(self, a: f64, b: f64) -> f64 {
        match self {
            ColumnOp::Add => a + b,
            ColumnOp::Sub => a - b,
            ColumnOp::Mul => a * b,
            ColumnOp::Div => a / b,
        }
    }

    /// 结果单位：加减保持 A 的单位，乘除组合两边的单位。
    fn units(self, a: &str, b: &str) -> String {
        match (self, a.is_empty(), b.is_empty()) {
            (ColumnOp::Add | ColumnOp::Sub, _, _) | (_, _, true) => a.to_string(),
            (ColumnOp::Mul, true, false) => b.to_string(),
            (ColumnOp::Mul, false, false) => format!("{a}·{b}"),
            (_, true, false) => format!("1/{b}"),
            _ => format!("{a}/{b}"),
        }
    }
}

/// 列运算的第二个操作数。
#[derive(Clone, Debug, PartialEq)]
pub enum Operand {
    Column(String),
    Constant(f64),
}

/// 按原分析流程的默认规则构建描述；来源曲线沿用图层行范围，派生线取完整结果列。
pub(super) fn analysis_plot(
    layer: &Layer,
    book: &Book,
    y: &str,
    x: Option<&str>,
    line_only: bool,
    style: Option<CurveStyle>,
) -> Plot {
    let local = layer.book_id == Some(book.id);
    let y = DataRef::new(book.id, y);
    let inherited_x = if local {
        layer.x_key.clone()
    } else {
        default_x(book, &y.col)
    };
    Plot {
        key: y.key(layer.book_id),
        mapping: CurveMapping {
            y,
            x: x.map(str::to_owned).or(inherited_x).map(|x| DataRef::new(book.id, x)),
            base: None,
            rows: if local && !line_only { layer.rows } else { None },
            line_only,
        },
        style,
        x_override: x.is_some(),
    }
}

// 仅用于内部绑定策略，结果列沿用既有的插入顺序和元数据。
pub(super) enum ResultLink<'a> {
    None,
    Auto { resolved: &'a Resolved, x: usize, y: usize },
    Layer(u64, usize),
    Matching,
}

pub(super) struct ResultKeys {
    pub(super) columns: Vec<String>,
    pub(super) graph: Option<u64>,
}

impl Project {
    pub(super) fn write_result(
        &mut self,
        item: &Item,
        mut columns: Vec<Column>,
        link: ResultLink<'_>,
    ) -> Result<ResultKeys, String> {
        if let ResultLink::Auto { x, y, .. } = &link {
            columns[*x].independent_rows = true;
            columns[*y].independent_rows = true;
        }
        let keys = columns
            .into_iter()
            .map(|col| self.insert_result_column(item.book_id, &item.y, col).unwrap())
            .collect::<Vec<_>>();
        let graph = self.bind_results(item, &keys, link)?;
        Ok(ResultKeys { columns: keys, graph })
    }

    // 独立绑定入口保留多曲线操作“全部写列后再附图”的顺序。
    pub(super) fn bind_results(
        &mut self,
        item: &Item,
        keys: &[String],
        link: ResultLink<'_>,
    ) -> Result<Option<u64>, String> {
        let (targets, keys, x) = match link {
            ResultLink::None => return Ok(None),
            ResultLink::Auto { resolved, x, y } => (
                vec![self.graph_for_result(resolved, item)?],
                std::slice::from_ref(&keys[y]),
                Some(keys[x].as_str()),
            ),
            ResultLink::Layer(gid, index) => (vec![(gid, index)], keys, item.x.as_deref()),
            ResultLink::Matching => {
                let mut targets = Vec::new();
                for graph in &self.graphs {
                    for (index, layer) in graph.page.layers().iter().enumerate() {
                        if layer
                            .plot(&DataRef::new(item.book_id, &item.y), &|id| self.book(id))
                            .is_some()
                        {
                            targets.push((graph.id, index));
                        }
                    }
                }
                (targets, keys, item.x.as_deref())
            }
        };
        for &(gid, index) in &targets {
            for key in keys {
                self.attach_result(gid, index, item, key, x)?;
            }
        }
        Ok(targets.last().map(|&(gid, _)| gid))
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
    pub(super) fn graph_for_result(&mut self, r: &Resolved, item: &Item) -> Result<(u64, usize), String> {
        if let Some(g) = r.graph_id {
            return Ok((g, r.layer_index));
        }
        for graph in &self.graphs {
            if let Some(index) = graph.page.layers().iter().position(|g| {
                g.plot(&DataRef::new(item.book_id, &item.y), &|id| self.book(id))
                    .is_some()
                    && g.transform == crate::plotspec::Transform::Identity
            }) {
                return Ok((graph.id, index));
            }
        }
        let id = self.alloc_id();
        let mut g = Graph::new(id, self.unique_name("Graph"));
        g.first_layer_mut().book_id = Some(item.book_id);
        g.first_layer_mut().x_key = item.x.clone();
        g.first_layer_mut().plot_type = PlotType::Scatter;
        g.first_layer_mut().rows = item.rows;
        let book = self.book(item.book_id).unwrap();
        let layer = g.first_layer_mut();
        let plot = analysis_plot(layer, book, &item.y, None, false, None);
        layer.add_plot(plot, &|id| self.book(id))?;
        self.graphs.push(g);
        self.move_project_item(id, self.folder_of(item.book_id)).unwrap();
        Ok((id, 0))
    }

    /// 追加新生成的分析结果；逐曲线只继承右轴，其他样式继续使用原派生线规则。
    pub(super) fn attach_result(
        &mut self,
        gid: u64,
        index: usize,
        item: &Item,
        result: &str,
        x: Option<&str>,
    ) -> Result<(), String> {
        let book = self.books.iter().find(|b| b.id == item.book_id).unwrap();
        let layer = &mut self.graphs.iter_mut().find(|g| g.id == gid).unwrap().page.layers_mut()[index];
        let lookup = |id| (id == book.id).then_some(book);
        let source = DataRef::new(item.book_id, &item.y);
        if layer.plot(&source, &lookup).is_none() {
            let plot = analysis_plot(layer, book, &item.y, None, false, None);
            layer.add_plot(plot, &lookup)?;
        }
        let right_axis = layer
            .plot(&source, &lookup)
            .and_then(|p| p.style)
            .is_some_and(|s| s.right_axis);
        let style = right_axis.then(|| CurveStyle {
            right_axis: true,
            ..Default::default()
        });
        let plot = analysis_plot(layer, book, result, x, true, style);
        layer.add_plot(plot, &lookup)?;
        Ok(())
    }

    /// 对每组数据计算一列结果并写在原列右边。`f` 返回 (结果值, 长名称, 单位, 注释)。
    pub(super) fn per_item_column(
        &mut self,
        src: &Source,
        op: OpKind,
        mut f: impl FnMut(&Book, &Item, &[f64], &[f64]) -> (Vec<f64>, String, String, String),
    ) -> Result<(Resolved, Vec<(Item, String)>), String> {
        let r = self.resolve(src)?;
        let mut made = vec![];
        for item in &r.items {
            let book = self.book(item.book_id).unwrap();
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
            c.data = data.into();
            c.op = Some(Operation {
                kind: op.clone(),
                y: item.y.clone(),
                x: item.x.clone(),
                rows: item.rows,
                mode: RecalcMode::Auto,
            });
            let key = self
                .write_result(item, vec![c], ResultLink::None)?
                .columns
                .pop()
                .unwrap();
            made.push((item.clone(), key));
        }
        if made.is_empty() {
            return Err("有效数据点太少（至少需要 3 个）".into());
        }
        Ok((r, made))
    }

    /// 按公式之外的方式填充列：等差序列或随机数，覆盖目标列，`n` 行。
    pub fn fill_sequence(
        &mut self,
        book_id: u64,
        target: &str,
        kind: crate::stats::Sequence,
        n: usize,
        seed: u64,
    ) -> Result<(), String> {
        if n == 0 {
            return Err("行数必须大于 0".into());
        }
        let book = self.book_mut(book_id).ok_or("找不到工作簿")?;
        book.column(target).ok_or("找不到目标列")?;
        let data: Vec<String> = crate::stats::sequence(kind, n, seed)
            .iter()
            .map(|v| fmt_num(*v))
            .collect();
        let data = book.column(target).unwrap().data.replaced_texts(data)?;
        book.freeze_error_sources();
        book.columns.iter_mut().find(|c| c.key == target).unwrap().data = data.into();
        Ok(())
    }

    /// 列运算：A op B，结果写成新列放在 A 右边。B 与 A 对应同一 X 列时逐行计算；
    /// 对应不同 X 列时（如原始曲线减拟合曲线）先把 B 线性插值到 A 的 X 上，超出 B 的 X 范围处留空。
    /// 结果不是有限数（除以 0、缺失值）的单元格留空。
    pub fn column_math(&mut self, book_id: u64, a: &str, op: ColumnOp, b: &Operand) -> Result<String, String> {
        let book = self.book(book_id).ok_or("找不到工作簿")?;
        let ca = book.column(a).ok_or("找不到列 A")?;
        let av = ca.values();
        let (bv, b_name, b_units, note) = match b {
            Operand::Constant(c) => {
                if !c.is_finite() {
                    return Err("常数必须是有限数".into());
                }
                (vec![*c; av.len()], fmt_num(*c), String::new(), String::new())
            }
            Operand::Column(k) => {
                let cb = book.column(k).ok_or("找不到列 B")?;
                let (xa, xb) = (x_for(book, ca, &[]), x_for(book, cb, &[]));
                let values = if xa == xb {
                    let mut v = cb.values();
                    v.resize(av.len(), f64::NAN);
                    v
                } else {
                    let item = Item {
                        book_id: book.id,
                        y: k.clone(),
                        x: xb.clone(),
                        rows: None,
                    };
                    let (xs, ys, _) = get_xy(book, &item);
                    let lo = xs.iter().copied().fold(f64::INFINITY, f64::min);
                    let hi = xs.iter().copied().fold(f64::NEG_INFINITY, f64::max);
                    let ax: Vec<f64> = match xa.as_deref().and_then(|x| book.column(x)) {
                        Some(c) => {
                            let mut v = c.values();
                            v.resize(av.len(), f64::NAN);
                            v
                        }
                        None => (1..=av.len()).map(|i| i as f64).collect(),
                    };
                    let inside: Vec<f64> = ax.iter().map(|x| if *x >= lo && *x <= hi { *x } else { 0.0 }).collect();
                    let at = interpolate::interpolate(&xs, &ys, InterpMethod::Linear, &inside)
                        .map_err(|e| format!("列 B 无法插值到 A 的 X 上：{e}"))?;
                    ax.iter()
                        .zip(at)
                        .map(|(x, y)| if *x >= lo && *x <= hi { y } else { f64::NAN })
                        .collect()
                };
                let note = if xa == xb {
                    String::new()
                } else {
                    format!("；{} 按线性插值到 {} 的 X 上", cb.display_name(), ca.display_name())
                };
                (values, cb.display_name().to_owned(), cb.units.clone(), note)
            }
        };
        let mut data: Vec<String> = av
            .iter()
            .zip(&bv)
            .map(|(x, y)| {
                let r = op.apply(*x, *y);
                if r.is_finite() {
                    fmt_num(r)
                } else {
                    String::new()
                }
            })
            .collect();
        while data.last().is_some_and(|s| s.is_empty()) {
            data.pop();
        }
        if data.is_empty() {
            return Err("运算结果没有有效数值".into());
        }
        let name = format!("{} {} {}", ca.display_name(), op.symbol(), b_name);
        let mut col = Column::new("", if ca.role == Role::X { Role::Y } else { ca.role });
        col.units = op.units(&ca.units, &b_units);
        col.comment = format!("列运算 {name}{note}");
        col.long_name = name;
        col.data = data.into();
        let item = Item {
            book_id,
            y: a.to_string(),
            x: None,
            rows: None,
        };
        Ok(self
            .write_result(&item, vec![col], ResultLink::None)?
            .columns
            .pop()
            .unwrap())
    }

    /// 按某一列排序整张工作表，各行数据一起移动。数值排在文本前面，空单元格排在最后。
    pub fn sort_book(&mut self, book_id: u64, col: usize, ascending: bool) -> Result<(), String> {
        let key = self
            .book(book_id)
            .ok_or("找不到工作簿")?
            .columns
            .get(col)
            .ok_or("没有这一列")?
            .key
            .clone();
        self.sort_rows(book_id, &[(key, ascending)], None)
    }

    /// 多关键字排序：先按第一个关键字，相同时按下一个，再相同保持原顺序（稳定）。
    /// `within` 为 None 时整张表各行一起移动；否则只重排这些列（关键字列须在其中），其余列不动。
    pub fn sort_rows(
        &mut self,
        book_id: u64,
        keys: &[(String, bool)],
        within: Option<&[String]>,
    ) -> Result<(), String> {
        let book = self.book_mut(book_id).ok_or("找不到工作簿")?;
        if keys.is_empty() {
            return Err("请至少选择一个排序关键字".into());
        }
        let moving: Vec<usize> = match within {
            None => (0..book.columns.len())
                .filter(|&i| !book.columns[i].independent_rows)
                .collect(),
            Some(cols) => cols.iter().filter_map(|k| book.column_index(k)).collect(),
        };
        let mut key_cols = vec![];
        for (k, asc) in keys {
            let i = book.column_index(k).ok_or_else(|| format!("找不到列 {k}"))?;
            if !moving.contains(&i) {
                return Err(format!("排序关键字 {k} 不在要排序的列里"));
            }
            key_cols.push((i, *asc));
        }
        let n = moving.iter().map(|&i| book.columns[i].data.len()).max().unwrap_or(0);
        let mut idx: Vec<usize> = (0..n).collect();
        idx.sort_by(|&i, &j| {
            key_cols
                .iter()
                .map(|&(c, asc)| book.columns[c].data.compare_rows(i, j, asc))
                .find(|o| o.is_ne())
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(i.cmp(&j))
        });
        for &c in &moving {
            let c = &mut book.columns[c];
            c.data = c.data.reordered(&idx).into();
            while c.data.last().is_some_and(|s| s.trim().is_empty()) {
                c.data.pop();
            }
        }
        Ok(())
    }
}
