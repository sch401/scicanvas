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
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum OpKind {
    Nonlinear(Box<NonlinearRecipe>),
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

/// 拟合时的函数定义快照，与后续修改的项目函数库独立。
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(deny_unknown_fields)
)]
pub enum FitMethod {
    Builtin(crate::nlfit::Model),
    Custom {
        name: String,
        formula: String,
        parameters: Vec<String>,
    },
}

impl FitMethod {
    pub fn from_function(function: &crate::nlfit::FitFunction) -> Self {
        match function {
            crate::nlfit::FitFunction::Builtin(model) => Self::Builtin(*model),
            _ => Self::Custom {
                name: function.name().into(),
                formula: function.formula().into(),
                parameters: function.params().into_iter().map(str::to_string).collect(),
            },
        }
    }

    pub fn resolve(&self) -> Result<crate::nlfit::FitFunction, String> {
        Ok(match self {
            Self::Builtin(model) => crate::nlfit::FitFunction::Builtin(*model),
            Self::Custom {
                name,
                formula,
                parameters,
            } => crate::nlfit::FitFunction::Custom(crate::nlfit::CustomFitFunction::new_with(
                name,
                formula,
                parameters.clone(),
                &crate::formula::FILE_LIMITS,
            )?),
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(deny_unknown_fields)
)]
pub struct FitBands {
    pub columns: [String; 4],
    pub level: f64,
}

/// 结果 Y 列拥有一次拟合的定义，X 和带由同一次计算一起更新。
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(deny_unknown_fields)
)]
pub struct NonlinearRecipe {
    pub function: FitMethod,
    pub initial: Option<Vec<f64>>,
    pub limits: Vec<crate::nlfit::ParamLimits>,
    pub constraints: String,
    pub output_x: String,
    pub bands: Vec<FitBands>,
    pub input_digest: Option<u64>,
    /// 相同失败来源在自动编辑收尾时不重复求解，手动重算可重试。
    #[cfg_attr(feature = "serde", serde(default))]
    pub attempted_digest: Option<u64>,
}

impl NonlinearRecipe {
    pub fn validate(&self) -> Result<(), String> {
        let function = self.function.resolve()?;
        let count = function.params().len();
        if self
            .initial
            .as_ref()
            .is_some_and(|p| p.len() != count || p.iter().any(|v| !v.is_finite()))
            || self.limits.len() > count
            || self.constraints.len() > crate::formula::FILE_LIMITS.bytes
        {
            return Err("非线性重算参数数量、初值或约束无效".into());
        }
        for limit in &self.limits {
            if [limit.fixed, limit.lower, limit.upper]
                .into_iter()
                .flatten()
                .any(|v| !v.is_finite())
                || matches!((limit.lower,limit.upper),(Some(a),Some(b)) if a>=b)
            {
                return Err("非线性重算参数限制无效".into());
            }
        }
        crate::nlfit::parse_constraints_for_function_with(&function, &self.constraints, &crate::formula::FILE_LIMITS)?;
        if self.output_x.is_empty()
            || self.bands.iter().any(|b| {
                !b.level.is_finite()
                    || !(0.0..1.0).contains(&b.level)
                    || b.level == 0.0
                    || b.columns.iter().any(String::is_empty)
            })
        {
            return Err("非线性重算输出或置信水平无效".into());
        }
        Ok(())
    }

    #[cfg(feature = "serde")]
    pub(crate) fn legacy_fingerprint(&self, op: &Operation, book: &Book) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        format!("{:?}", (&self.function, &self.initial, &self.limits, &self.constraints)).hash(&mut h);
        (&op.y, &op.x, op.rows).hash(&mut h);
        let item = Item {
            book_id: book.id,
            y: op.y.clone(),
            x: op.x.clone(),
            rows: op.rows,
        };
        let (xs, ys, idx) = get_xy(book, &item);
        idx.hash(&mut h);
        for value in xs.iter().chain(&ys) {
            value.to_bits().hash(&mut h);
        }
        h.finish()
    }

    pub(super) fn fit(&self, op: &Operation, book: &Book) -> Result<crate::nlfit::NlFit, String> {
        self.validate()?;
        let item = Item {
            book_id: book.id,
            y: op.y.clone(),
            x: op.x.clone(),
            rows: op.rows,
        };
        book.column(&op.y).ok_or("来源 Y 列已删除")?;
        if let Some(x) = &op.x {
            book.column(x).ok_or("来源 X 列已删除")?;
        }
        let function = self.function.resolve()?;
        let constraints = crate::nlfit::parse_constraints_for_function_with(
            &function,
            &self.constraints,
            &crate::formula::FILE_LIMITS,
        )?;
        let (xs, ys, _) = get_xy(book, &item);
        let fit = crate::nlfit::fit_function(function, &xs, &ys, self.initial.as_deref(), &self.limits, &constraints)?;
        if !fit.converged {
            return Err("非线性拟合尚未收敛，已保留上次结果".into());
        }
        Ok(fit)
    }

    pub(super) fn calculate(
        &self,
        op: &Operation,
        book: &Book,
    ) -> Result<(Vec<(String, crate::ColumnData)>, crate::nlfit::NlFit), String> {
        let fit = self.fit(op, book)?;
        let grid = interpolate::grid(fit.x_min, fit.x_max, FIT_POINTS);
        let values = fit.sample(&grid);
        let data = |values: &[f64]| -> crate::ColumnData { values.iter().map(|v| fmt_num(*v)).collect() };
        let mut result = vec![(self.output_x.clone(), data(&grid))];
        // 首次添加带使用已写入 X 列的显示精度，重算沿用同一精度。
        let rounded: Vec<f64> = grid.iter().map(|x| fmt_num(*x).parse().unwrap_or(f64::NAN)).collect();
        for bands in &self.bands {
            let rows: Vec<_> = rounded
                .iter()
                .zip(&values)
                .map(|(&x, &y)| {
                    let (c, p) = fit.band(x, bands.level);
                    if y.is_finite() {
                        (fit.eval(x), c, p)
                    } else {
                        (f64::NAN, f64::NAN, f64::NAN)
                    }
                })
                .collect();
            for (i, key) in bands.columns.iter().enumerate() {
                let v: Vec<f64> = rows
                    .iter()
                    .map(|&(y, c, p)| match i {
                        0 => y - c,
                        1 => y + c,
                        2 => y - p,
                        _ => y + p,
                    })
                    .collect();
                result.push((key.clone(), data(&v)));
            }
        }
        // Y 的键由拥有此记录的列提供。
        result.push((String::new(), data(&values)));
        Ok((result, fit))
    }

    pub(crate) fn estimated_bytes(&self) -> usize {
        use std::mem::size_of;
        let function = match &self.function {
            FitMethod::Builtin(_) => 0,
            FitMethod::Custom {
                name,
                formula,
                parameters,
            } => {
                name.capacity()
                    + formula.capacity()
                    + parameters.capacity() * size_of::<String>()
                    + parameters.iter().map(String::capacity).sum::<usize>()
            }
        };
        size_of::<Self>()
            + function
            + self.initial.as_ref().map_or(0, |p| p.capacity() * size_of::<f64>())
            + self.limits.capacity() * size_of::<crate::nlfit::ParamLimits>()
            + self.constraints.capacity()
            + self.output_x.capacity()
            + self.bands.capacity() * size_of::<FitBands>()
            + self
                .bands
                .iter()
                .flat_map(|b| &b.columns)
                .map(String::capacity)
                .sum::<usize>()
    }
}

/// 拟合曲线的网格点数。
const FIT_POINTS: usize = 200;

impl OpKind {
    /// 对一组有效点计算结果（与首次分析用同一实现）。
    pub fn compute(&self, xs: &[f64], ys: &[f64]) -> Vec<f64> {
        let kind = match self {
            Self::Nonlinear(_) => return Vec::new(), // 多列结果由事务一起写入。
            kind => kind.clone(),
        };
        match kind {
            OpKind::Nonlinear(_) => unreachable!(),
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
fn formula_data(book: &Book, target: &str, expr: &crate::formula::Expr) -> Result<crate::ColumnData, String> {
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
fn computed_order(
    book: &Book,
    manual: bool,
    formulas: &mut std::collections::HashMap<String, Result<crate::formula::Expr, String>>,
) -> Result<Vec<String>, String> {
    let mut deps: std::collections::HashMap<String, Vec<String>> = book
        .columns
        .iter()
        .filter_map(|c| {
            if let Some(f) = c.formula.as_deref() {
                let expr = crate::formula::parse_file(f);
                let deps = expr.as_ref().map(|e| e.columns()).unwrap_or_default();
                formulas.insert(c.key.clone(), expr);
                return Some((c.key.clone(), deps));
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
    for col in &book.columns {
        if let Some(Operation {
            kind: OpKind::Nonlinear(recipe),
            ..
        }) = &col.op
        {
            if col.formula.is_some() {
                return Err(format!("拟合结果列 {} 同时有其他计算定义", col.key));
            }
            for key in std::iter::once(&recipe.output_x).chain(recipe.bands.iter().flat_map(|b| &b.columns)) {
                if deps.contains_key(key) {
                    return Err(format!("非线性输出列 {key} 同时有其他计算定义"));
                }
                if deps.contains_key(&col.key) {
                    deps.insert(key.clone(), vec![col.key.clone()]);
                }
            }
        }
    }
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
        let limits = if book
            .column(target)
            .is_some_and(|c| c.formula.as_deref() == Some(formula))
        {
            &crate::formula::FILE_LIMITS
        } else {
            &crate::formula::INPUT_LIMITS
        };
        let expr = crate::formula::parse_with(formula, limits)?;
        let data = formula_data(book, target, &expr)?;
        let n = data.len();
        let book = self.book_mut(book_id).unwrap();
        book.freeze_error_sources();
        book.columns.iter_mut().find(|c| c.key == target).unwrap().data = data.into();
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
            if let Some(owner) = book.nonlinear_output_owner(target) {
                return Err(format!(
                    "工作簿 {} 列 {target} 属于拟合结果 {owner}，请先解除拟合再保存公式",
                    book.name
                ));
            }
            let limits = if book
                .column(target)
                .is_some_and(|c| c.formula.as_deref() == Some(formula))
            {
                &crate::formula::FILE_LIMITS
            } else {
                &crate::formula::INPUT_LIMITS
            };
            if crate::formula::parse_with(formula, limits)?
                .columns()
                .iter()
                .any(|k| k == target)
            {
                return Err("保存的公式不能引用目标列自身（会循环重算）".into());
            }
            let mut trial = book.clone();
            trial
                .columns
                .iter_mut()
                .find(|c| c.key == target)
                .ok_or("找不到目标列")?
                .formula = Some(formula.into());
            computed_order(&trial, true, &mut Default::default())?;
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
            let mut formulas = std::collections::HashMap::new();
            let order = match computed_order(&self.books[b], manual, &mut formulas) {
                Ok(order) => order,
                Err(e) => {
                    problems.push(format!("{}：{e}", self.books[b].name));
                    continue;
                }
            };
            for key in order {
                let book = &self.books[b];
                let Some(col) = book.column(&key) else {
                    continue;
                };
                if let Some(op) = col.op.as_ref().filter(|_| col.formula.is_none()) {
                    if let OpKind::Nonlinear(recipe) = &op.kind {
                        let fingerprint = recipe.fingerprint(op, book);
                        if recipe.input_digest == Some(fingerprint) && !manual {
                            if let Some(Operation {
                                kind: OpKind::Nonlinear(recipe),
                                ..
                            }) = &mut self.books[b].columns.iter_mut().find(|c| c.key == key).unwrap().op
                            {
                                recipe.attempted_digest = None;
                            }
                            if let Some(r) = &mut self.books[b]
                                .columns
                                .iter_mut()
                                .find(|c| c.key == key)
                                .unwrap()
                                .fit_report
                            {
                                r.current = r.fit.converged;
                                r.error = None;
                            }
                            continue;
                        }
                        if recipe.attempted_digest == Some(fingerprint) && !manual {
                            continue;
                        }
                        let op = op.clone();
                        let computed = recipe.calculate(&op, book);
                        let result = computed
                            .and_then(|(outputs, fit)| self.write_nlfit(self.books[b].id, &key, op, outputs, fit));
                        if let Err(e) = result {
                            if let Some(Operation {
                                kind: OpKind::Nonlinear(recipe),
                                ..
                            }) = &mut self.books[b].columns.iter_mut().find(|c| c.key == key).unwrap().op
                            {
                                recipe.attempted_digest = Some(fingerprint);
                            }
                            problems.push(format!("{} 列 {key}：{e}", self.books[b].name));
                            if let Some(r) = &mut self.books[b]
                                .columns
                                .iter_mut()
                                .find(|c| c.key == key)
                                .unwrap()
                                .fit_report
                            {
                                r.current = false;
                                r.error = Some(e);
                            }
                        }
                        continue;
                    }
                    if let Some(data) = op.data(book) {
                        let col = self.books[b].columns.iter_mut().find(|c| c.key == key).unwrap();
                        if col.data != data {
                            col.data = data.into();
                        }
                    }
                    continue;
                }
                let Some(expr) = formulas.remove(&key) else {
                    continue;
                };
                match expr.and_then(|expr| formula_data(book, &key, &expr)) {
                    Ok(data) => {
                        let col = self.books[b].columns.iter_mut().find(|c| c.key == key).unwrap();
                        if col.data != data {
                            col.data = data.into();
                        }
                    }
                    Err(e) => problems.push(format!("{} 列 {key}：{e}", self.books[b].name)),
                }
            }
            let book = &self.books[b];
            let pending: Vec<_> = book
                .columns
                .iter()
                .filter_map(|c| {
                    let op = c.op.as_ref()?;
                    let OpKind::Nonlinear(recipe) = &op.kind else {
                        return None;
                    };
                    (op.mode == RecalcMode::None || (op.mode == RecalcMode::Manual && !manual))
                        .then(|| (c.key.clone(), recipe.input_digest == Some(recipe.fingerprint(op, book))))
                })
                .collect();
            for (key, current) in pending {
                if let Some(r) = &mut self.books[b]
                    .columns
                    .iter_mut()
                    .find(|c| c.key == key)
                    .unwrap()
                    .fit_report
                {
                    r.current = current && r.fit.converged;
                    if r.current {
                        r.error = None;
                    }
                }
            }
        }
        problems
    }
}

impl Operation {
    pub fn same_settings(&self, other: &Self) -> bool {
        if self.y != other.y || self.x != other.x || self.rows != other.rows || self.mode != other.mode {
            return false;
        }
        match (&self.kind, &other.kind) {
            (OpKind::Nonlinear(a), OpKind::Nonlinear(b)) => {
                a.function == b.function
                    && a.initial == b.initial
                    && a.limits == b.limits
                    && a.constraints == b.constraints
                    && a.output_x == b.output_x
                    && a.bands == b.bands
            }
            _ => self.kind == other.kind,
        }
    }
}

/// v21 使用固定 FNV-1a；各字段采用带长度字符串和小端整数编码。
struct Digest(u64);
impl Digest {
    fn bytes(&mut self, bytes: &[u8]) {
        for b in bytes {
            self.0 = (self.0 ^ u64::from(*b)).wrapping_mul(0x100000001b3);
        }
    }
    fn integer(&mut self, value: u64) {
        self.bytes(&value.to_le_bytes());
    }
    fn text(&mut self, text: &str) {
        self.integer(text.len() as u64);
        self.bytes(text.as_bytes());
    }
    fn number(&mut self, value: Option<f64>) {
        self.integer(u64::from(value.is_some()));
        if let Some(v) = value {
            self.integer(v.to_bits());
        }
    }
}
impl NonlinearRecipe {
    pub(crate) fn fingerprint(&self, op: &Operation, book: &Book) -> u64 {
        let mut h = Digest(0xcbf29ce484222325);
        h.text("SciCanvas-fit-v21");
        match &self.function {
            FitMethod::Builtin(model) => {
                h.integer(0);
                h.text(model.name());
            }
            FitMethod::Custom {
                name,
                formula,
                parameters,
            } => {
                h.integer(1);
                h.text(name);
                h.text(formula);
                h.integer(parameters.len() as u64);
                for p in parameters {
                    h.text(p);
                }
            }
        }
        h.integer(u64::from(self.initial.is_some()));
        if let Some(values) = &self.initial {
            h.integer(values.len() as u64);
            for value in values {
                h.integer(value.to_bits());
            }
        }
        h.integer(self.limits.len() as u64);
        for limit in &self.limits {
            h.number(limit.fixed);
            h.number(limit.lower);
            h.number(limit.upper);
        }
        h.text(&self.constraints);
        h.text(&op.y);
        h.integer(u64::from(op.x.is_some()));
        if let Some(x) = &op.x {
            h.text(x);
        }
        h.integer(u64::from(op.rows.is_some()));
        if let Some((a, b)) = op.rows {
            h.integer(a as u64);
            h.integer(b as u64);
        }
        let (xs, ys, rows) = get_xy(
            book,
            &Item {
                book_id: book.id,
                y: op.y.clone(),
                x: op.x.clone(),
                rows: op.rows,
            },
        );
        h.integer(rows.len() as u64);
        for ((x, y), row) in xs.iter().zip(ys).zip(rows) {
            h.integer(row as u64);
            h.integer(x.to_bits());
            h.integer(y.to_bits());
        }
        h.0
    }
}

#[cfg(test)]
mod round_three_tests {
    use super::*;
    #[test]
    fn legacy_formulas_recalculate_in_dependency_order_and_keep_errors_local() {
        let mut p = Project::sample();
        let book = p.book_mut(1).unwrap();
        book.columns.iter_mut().find(|c| c.key == "B").unwrap().formula =
            Some(std::iter::repeat_n("A", 150).collect::<Vec<_>>().join("+"));
        book.columns.iter_mut().find(|c| c.key == "C").unwrap().formula = Some("B+1".into());
        assert!(p.recalc_computed(false).is_empty());
        let book = p.book(1).unwrap();
        for (a, c) in book
            .column("A")
            .unwrap()
            .values()
            .iter()
            .zip(book.column("C").unwrap().values())
        {
            assert!((c - (a * 150. + 1.)).abs() < 1e-8);
        }
        let book = p.book_mut(1).unwrap();
        let old = book.column("B").unwrap().data.clone();
        book.columns.iter_mut().find(|c| c.key == "B").unwrap().formula = Some("sin(".into());
        assert_eq!(p.recalc_computed(false).len(), 1);
        assert_eq!(p.book(1).unwrap().column("B").unwrap().data, old);
    }
}
