//! 公式列、操作记录与按依赖重算。
use super::*;

/// 结果列的重算方式：来源变化时自动重算、只在“重算”命令时重算、或保持不变。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum RecalcMode {
    #[default]
    Auto,
    Manual,
    None,
}

impl RecalcMode {
    pub const ALL: [RecalcMode; 3] = [RecalcMode::Auto, RecalcMode::Manual, RecalcMode::None];

    pub fn label(self) -> &'static str {
        match self {
            RecalcMode::Auto => "自动",
            RecalcMode::Manual => "手动",
            RecalcMode::None => "不重算",
        }
    }
}

/// 可重算的逐列分析。
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum OpKind {
    Smooth {
        method: SmoothMethod,
        window: usize,
    },
    Normalize {
        method: NormalizeMethod,
    },
    Derivative,
    SgDerivative {
        window: usize,
        order: usize,
        deriv: usize,
    },
    Integral,
    Baseline {
        method: crate::baseline::BaselineMethod,
    },
    BaselineSubtracted {
        method: crate::baseline::BaselineMethod,
    },
    FftFilter {
        kind: crate::fft::FilterKind,
    },
    /// 多项式拟合：逐点残差（与原数据同行）、拟合曲线的 X 网格与拟合值（各 200 点）。
    PolyResidual {
        order: usize,
    },
    PolyFitX {
        order: usize,
    },
    PolyFitY {
        order: usize,
    },
}

/// 拟合曲线的网格点数。
const FIT_POINTS: usize = 200;

impl OpKind {
    /// 对一组有效点计算结果（与首次分析用同一实现）。
    pub fn compute(&self, xs: &[f64], ys: &[f64]) -> Vec<f64> {
        match *self {
            OpKind::Smooth { method, window } => analysis::smooth(xs, ys, method, window),
            OpKind::Normalize { method } => analysis::normalize(ys, method),
            OpKind::Derivative => analysis::derivative(xs, ys),
            OpKind::SgDerivative { window, order, deriv } => analysis::sg_derivative(xs, ys, window, order, deriv),
            OpKind::Integral => analysis::cumulative_trapezoid(xs, ys).0,
            OpKind::Baseline { method } => crate::baseline::baseline(xs, ys, method),
            OpKind::BaselineSubtracted { method } => {
                let b = crate::baseline::baseline(xs, ys, method);
                ys.iter().zip(&b).map(|(y, b)| y - b).collect()
            }
            OpKind::FftFilter { kind } => {
                let dt = crate::fft::sample_interval(xs).unwrap_or(1.0);
                crate::fft::filter(ys, dt, kind).unwrap_or_else(|_| vec![f64::NAN; ys.len()])
            }
            OpKind::PolyResidual { order } => match analysis::polyfit(xs, ys, order) {
                Some(fit) => xs.iter().zip(ys).map(|(x, y)| y - fit.eval(*x)).collect(),
                None => vec![f64::NAN; ys.len()],
            },
            OpKind::PolyFitX { order } | OpKind::PolyFitY { order } => match analysis::polyfit(xs, ys, order) {
                Some(fit) => (0..FIT_POINTS)
                    .map(|i| {
                        let x = fit.x_min + (fit.x_max - fit.x_min) * i as f64 / (FIT_POINTS - 1) as f64;
                        if matches!(self, OpKind::PolyFitX { .. }) {
                            x
                        } else {
                            fit.eval(x)
                        }
                    })
                    .collect(),
                None => vec![],
            },
        }
    }

    /// 结果是独立的网格（拟合曲线），不与原数据逐行对齐。
    fn is_grid(&self) -> bool {
        matches!(self, OpKind::PolyFitX { .. } | OpKind::PolyFitY { .. })
    }
}

/// 结果列上记录的分析操作：操作、来源 Y/X 列、行范围与重算方式。
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Operation {
    pub kind: OpKind,
    pub y: String,
    pub x: Option<String>,
    pub rows: Option<(usize, usize)>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub mode: RecalcMode,
}

impl Operation {
    /// 在工作簿当前数据上重算，结果与原数据逐行对齐（无效点留空）。来源列已删除时返回 None。
    fn data(&self, book: &Book) -> Option<crate::ColumnData> {
        book.column(&self.y)?;
        if let Some(x) = &self.x {
            book.column(x)?;
        }
        let item = Item {
            book_id: book.id,
            y: self.y.clone(),
            x: self.x.clone(),
            rows: self.rows,
        };
        let (xs, ys, idx) = get_xy(book, &item);
        let vals = if xs.len() < 3 {
            vec![]
        } else {
            self.kind.compute(&xs, &ys)
        };
        if self.kind.is_grid() {
            return Some(
                vals.iter()
                    .map(|v| if v.is_finite() { fmt_num(*v) } else { String::new() })
                    .collect(),
            );
        }
        let mut data = vec![String::new(); book.nrows()];
        for (k, &row) in idx.iter().enumerate() {
            if vals.get(k).is_some_and(|v| v.is_finite()) {
                data[row] = fmt_num(vals[k]);
            }
        }
        while data.last().is_some_and(|s| s.is_empty()) {
            data.pop();
        }
        Some(data.into())
    }
}

/// 按公式计算目标列的新数据（保持目标列类型）；结果不是有限数的单元格留空，末尾空行去掉。
fn formula_data(book: &Book, target: &str, formula: &str) -> Result<crate::ColumnData, String> {
    let expr = crate::formula::parse(formula)?;
    let mut cols = std::collections::HashMap::new();
    for k in expr.columns() {
        let c = book
            .column(&k)
            .ok_or_else(|| format!("工作簿里没有列 {k}（列用短名引用，如 A、B）"))?;
        cols.insert(k, c.values());
    }
    let rows = if cols.is_empty() {
        book.nrows()
    } else {
        cols.values().map(Vec::len).max().unwrap_or(0)
    };
    if rows == 0 {
        return Err("没有可计算的行：公式不引用列时按工作簿已有的行数计算".into());
    }
    let value = |k: &str, i: usize| cols.get(k).and_then(|v| v.get(i)).copied().unwrap_or(f64::NAN);
    // 列统计、按行序列先对整列算好
    let expr = expr.prepare(rows, &value);
    let mut data: Vec<String> = (0..rows)
        .map(|i| {
            let v = expr.eval(i, &value);
            if v.is_finite() {
                fmt_num(v)
            } else {
                String::new()
            }
        })
        .collect();
    while data.last().is_some_and(|s| s.is_empty()) {
        data.pop();
    }
    book.column(target).ok_or("找不到目标列")?.data.replaced_texts(data)
}

/// 需要重算的列（公式列、自动或 `manual` 时手动的分析结果列）按依赖排序（来源先算）；有循环依赖时报错。
fn computed_order(book: &Book, manual: bool) -> Result<Vec<String>, String> {
    let deps: std::collections::HashMap<String, Vec<String>> = book
        .columns
        .iter()
        .filter_map(|c| {
            if let Some(f) = c.formula.as_deref() {
                return Some((
                    c.key.clone(),
                    crate::formula::parse(f).map(|e| e.columns()).unwrap_or_default(),
                ));
            }
            let op = c.op.as_ref()?;
            let due = op.mode == RecalcMode::Auto || (manual && op.mode == RecalcMode::Manual);
            due.then(|| {
                (
                    c.key.clone(),
                    std::iter::once(op.y.clone()).chain(op.x.clone()).collect(),
                )
            })
        })
        .collect();
    // 0 未访问，1 访问中，2 完成
    let mut state: std::collections::HashMap<&str, u8> = Default::default();
    let mut order = vec![];
    fn visit<'a>(
        key: &'a str,
        deps: &'a std::collections::HashMap<String, Vec<String>>,
        state: &mut std::collections::HashMap<&'a str, u8>,
        order: &mut Vec<String>,
    ) -> Result<(), String> {
        match state.get(key) {
            Some(2) => return Ok(()),
            Some(1) => return Err(format!("列公式或分析结果循环引用（涉及 {key} 列）")),
            _ => {}
        }
        state.insert(key, 1);
        for d in deps.get(key).into_iter().flatten() {
            if deps.contains_key(d) {
                visit(d, deps, state, order)?;
            }
        }
        state.insert(key, 2);
        order.push(key.to_string());
        Ok(())
    }
    let mut keys: Vec<&String> = deps.keys().collect();
    keys.sort();
    for k in keys {
        visit(k, &deps, &mut state, &mut order)?;
    }
    Ok(order)
}

impl Project {
    /// 设置列值：按公式逐行计算，写入 `target` 列（覆盖原有内容）。公式里的列用短名引用，
    /// 可以引用目标列自身（用的是计算前的值）。结果不是有限数的单元格留空。返回写入的行数。
    pub fn set_column_values(&mut self, book_id: u64, target: &str, formula: &str) -> Result<usize, String> {
        let book = self.book(book_id).ok_or("找不到工作簿")?;
        book.column(target).ok_or("找不到目标列")?;
        let data = formula_data(book, target, formula)?;
        let n = data.len();
        let book = self.book_mut(book_id).unwrap();
        book.freeze_error_sources();
        book.columns.iter_mut().find(|c| c.key == target).unwrap().data = data;
        Ok(n)
    }

    /// 设置列值，并按 `keep` 保存或去掉列公式；保存的公式在引用的列变化时自动重算。
    pub fn set_column_formula(
        &mut self,
        book_id: u64,
        target: &str,
        formula: &str,
        keep: bool,
    ) -> Result<usize, String> {
        if keep {
            let book = self.book(book_id).ok_or("找不到工作簿")?;
            if crate::formula::parse(formula)?.columns().iter().any(|k| k == target) {
                return Err("保存的公式不能引用目标列自身（会循环重算）".into());
            }
            let mut trial = book.clone();
            trial
                .columns
                .iter_mut()
                .find(|c| c.key == target)
                .ok_or("找不到目标列")?
                .formula = Some(formula.into());
            computed_order(&trial, true)?;
        }
        let n = self.set_column_values(book_id, target, formula)?;
        let book = self.book_mut(book_id).unwrap();
        book.columns.iter_mut().find(|c| c.key == target).unwrap().formula = keep.then(|| formula.to_string());
        Ok(n)
    }

    /// 按保存的列公式重算（只算公式列）。
    pub fn recalc_formulas(&mut self) -> Vec<String> {
        self.recalc_computed(false)
    }

    /// 重算公式列和带分析操作的结果列（按依赖顺序）：自动的总是重算，`manual` 时也算手动的；
    /// “不重算”的保持原样，结果不变的列不改动。返回出错的说明。
    pub fn recalc_computed(&mut self, manual: bool) -> Vec<String> {
        let mut problems = vec![];
        for b in 0..self.books.len() {
            if !self.books[b]
                .columns
                .iter()
                .any(|c| c.formula.is_some() || c.op.is_some())
            {
                continue;
            }
            let order = match computed_order(&self.books[b], manual) {
                Ok(order) => order,
                Err(e) => {
                    problems.push(format!("{}：{e}", self.books[b].name));
                    continue;
                }
            };
            for key in order {
                let book = &self.books[b];
                let col = book.column(&key).unwrap();
                if let Some(op) = col.op.as_ref().filter(|_| col.formula.is_none()) {
                    if let Some(data) = op.data(book) {
                        let col = self.books[b].columns.iter_mut().find(|c| c.key == key).unwrap();
                        if col.data != data {
                            col.data = data;
                        }
                    }
                    continue;
                }
                let formula = col.formula.clone().unwrap_or_default();
                match formula_data(book, &key, &formula) {
                    Ok(data) => {
                        let col = self.books[b].columns.iter_mut().find(|c| c.key == key).unwrap();
                        if col.data != data {
                            col.data = data;
                        }
                    }
                    Err(e) => problems.push(format!("{} 列 {key}：{e}", self.books[b].name)),
                }
            }
        }
        problems
    }
}
