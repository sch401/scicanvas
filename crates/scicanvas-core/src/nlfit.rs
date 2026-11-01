//! 非线性拟合：Levenberg–Marquardt（中心差分雅可比），内置 Origin 同名函数与自动初值。
//! 标准误按 cov = (JᵀJ)⁻¹ · RSS / (n − p)，与 SciPy `curve_fit`（默认 absolute_sigma=False）一致。

use crate::analysis::mat_inv;

mod guess;
mod model;
pub use model::{voigt_profile, Model, MAX_PARAMS};

/// 可保存、跨项目交换的函数定义；语法树在使用时重新解析。
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(deny_unknown_fields)
)]
pub struct CustomFitDefinition {
    pub name: String,
    pub formula: String,
    pub parameters: Vec<String>,
    pub initial: Vec<f64>,
    pub constraints: String,
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(deny_unknown_fields)
)]
pub struct SavedFitFunction {
    pub id: u64,
    pub definition: CustomFitDefinition,
}

impl CustomFitDefinition {
    pub fn from_text(
        name: &str,
        formula: &str,
        parameters: &str,
        initial: &str,
        constraints: &str,
    ) -> Result<Self, String> {
        Self::from_text_with(
            name,
            formula,
            parameters,
            initial,
            constraints,
            &crate::formula::INPUT_LIMITS,
        )
    }
    pub fn from_text_with(
        name: &str,
        formula: &str,
        parameters: &str,
        initial: &str,
        constraints: &str,
        limits: &crate::formula::Limits,
    ) -> Result<Self, String> {
        let split = |text: &str| {
            text.split([',', '，', ';', '；'])
                .map(str::trim)
                .filter(|part| !part.is_empty())
                .map(str::to_owned)
                .collect::<Vec<_>>()
        };
        let initial = split(initial)
            .iter()
            .map(|value| value.parse::<f64>().map_err(|_| format!("初始值 {value} 不是有限数")))
            .collect::<Result<Vec<_>, _>>()?;
        let mut definition = Self {
            name: name.trim().into(),
            formula: formula.trim().into(),
            parameters: split(parameters),
            initial,
            constraints: constraints.trim().into(),
        };
        let (function, _) = definition.resolve_with(limits)?;
        definition.name = function.name.clone();
        definition.formula = function.formula.clone();
        Ok(definition)
    }

    pub fn resolve(&self) -> Result<(CustomFitFunction, Vec<LinearConstraint>), String> {
        self.resolve_with(&crate::formula::FILE_LIMITS)
    }
    fn resolve_with(
        &self,
        limits: &crate::formula::Limits,
    ) -> Result<(CustomFitFunction, Vec<LinearConstraint>), String> {
        let function = CustomFitFunction::new_with(&self.name, &self.formula, self.parameters.clone(), limits)?;
        if self.initial.len() != self.parameters.len() || self.initial.iter().any(|value| !value.is_finite()) {
            return Err(format!("请为 {} 个参数各填写一个有限初始值", self.parameters.len()));
        }
        if self.constraints.len() > limits.bytes {
            return Err("参数约束过长".into());
        }
        let constraints =
            parse_constraints_for_function_with(&FitFunction::Custom(function.clone()), &self.constraints, limits)?;
        Ok((function, constraints))
    }
}

impl crate::Project {
    /// 已载入的定义作为编辑基线；重命名保留编号，过期草稿在写入前拒绝。
    pub fn save_fit_function(
        &mut self,
        before: Option<&SavedFitFunction>,
        definition: CustomFitDefinition,
    ) -> Result<u64, String> {
        definition.resolve()?;
        if self
            .fit_functions
            .iter()
            .any(|entry| entry.definition.name == definition.name && Some(entry.id) != before.map(|old| old.id))
        {
            return Err("已存在同名函数，请修改函数名后保存".into());
        }
        if let Some(before) = before {
            let entry = self
                .fit_functions
                .iter_mut()
                .find(|entry| entry.id == before.id)
                .ok_or("函数已移除，请重新载入或新建")?;
            if entry != before {
                return Err("函数已在别处更改，请重新载入后编辑".into());
            }
            entry.definition = definition;
            return Ok(entry.id);
        }
        let id = self.alloc_id();
        self.fit_functions.push(SavedFitFunction { id, definition });
        Ok(id)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CustomFitFunction {
    name: String,
    formula: String,
    parameters: Vec<String>,
    expression: crate::formula::Expr,
}

impl CustomFitFunction {
    pub fn new(name: &str, formula: &str, parameters: Vec<String>) -> Result<Self, String> {
        Self::new_with(name, formula, parameters, &crate::formula::INPUT_LIMITS)
    }
    pub fn new_with(
        name: &str,
        formula: &str,
        parameters: Vec<String>,
        limits: &crate::formula::Limits,
    ) -> Result<Self, String> {
        let name = name.trim();
        let formula = formula.trim();
        if name.is_empty() || name.len() > 64 || formula.is_empty() || formula.len() > limits.bytes {
            return Err("函数名称或表达式为空、过长".into());
        }
        if parameters.is_empty() || parameters.len() > MAX_PARAMS {
            return Err(format!("参数数量必须在 1 到 {MAX_PARAMS} 之间"));
        }
        let valid_identifier = |value: &str| {
            let mut chars = value.chars();
            chars.next().is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
                && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
        };
        let mut unique = std::collections::HashSet::new();
        for parameter in &parameters {
            if !valid_identifier(parameter)
                || matches!(parameter.as_str(), "x" | "y" | "e" | "pi" | "PI")
                || !unique.insert(parameter)
            {
                return Err(format!("参数名 {parameter} 无效或重复"));
            }
        }
        let body = match formula.split_once('=') {
            Some((lhs, rhs)) if matches!(lhs.trim(), "y" | "Y") => rhs.trim(),
            Some(_) => return Err("表达式等号左侧只能写 y".into()),
            None => formula,
        };
        let expression = crate::formula::parse_with(body, limits)?;
        let variables = expression.scalar_variables()?;
        for variable in variables {
            if variable != "x" && !unique.contains(&variable) {
                return Err(format!("表达式中的变量 {variable} 未列在参数表中"));
            }
        }
        Ok(Self {
            name: name.to_owned(),
            formula: format!("y = {body}"),
            parameters,
            expression,
        })
    }

    pub fn parameters(&self) -> &[String] {
        &self.parameters
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    fn eval(&self, x: f64, params: &[f64]) -> f64 {
        self.expression.eval(0, &|name, _| {
            if name == "x" {
                x
            } else {
                self.parameters
                    .iter()
                    .position(|parameter| parameter == name)
                    .and_then(|index| params.get(index).copied())
                    .unwrap_or(f64::NAN)
            }
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(into = "crate::ops::FitMethod", try_from = "crate::ops::FitMethod")
)]
pub enum FitFunction {
    Builtin(Model),
    Custom(CustomFitFunction),
}

impl FitFunction {
    pub fn name(&self) -> &str {
        match self {
            Self::Builtin(model) => model.name(),
            Self::Custom(function) => &function.name,
        }
    }

    pub fn formula(&self) -> &str {
        match self {
            Self::Builtin(model) => model.formula(),
            Self::Custom(function) => &function.formula,
        }
    }

    pub fn params(&self) -> Vec<&str> {
        match self {
            Self::Builtin(model) => model.params().to_vec(),
            Self::Custom(function) => function.parameters.iter().map(String::as_str).collect(),
        }
    }

    fn eval(&self, x: f64, params: &[f64]) -> f64 {
        match self {
            Self::Builtin(model) => model.eval(x, params),
            Self::Custom(function) => function.eval(x, params),
        }
    }

    fn guess(&self, xs: &[f64], ys: &[f64]) -> Option<Vec<f64>> {
        match self {
            Self::Builtin(model) => Some(model.guess(xs, ys)),
            Self::Custom(_) => None,
        }
    }
}

impl From<FitFunction> for crate::ops::FitMethod {
    fn from(function: FitFunction) -> Self {
        Self::from_function(&function)
    }
}
impl TryFrom<crate::ops::FitMethod> for FitFunction {
    type Error = String;
    fn try_from(method: crate::ops::FitMethod) -> Result<Self, String> {
        method.resolve()
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum FitStatus {
    #[default]
    Converged,
    PrecisionLimit,
    Failed,
}

/// 拟合结果。
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct NlFit {
    pub model: FitFunction,
    pub params: Vec<f64>,
    #[cfg_attr(feature = "serde", serde(deserialize_with = "report_errors"))]
    pub se: Vec<f64>,
    #[cfg_attr(feature = "serde", serde(deserialize_with = "report_number"))]
    pub rss: f64,
    #[cfg_attr(feature = "serde", serde(deserialize_with = "report_number"))]
    pub r2: f64,
    #[cfg_attr(feature = "serde", serde(deserialize_with = "report_number"))]
    pub adj_r2: f64,
    /// RSS / (n − p)
    #[cfg_attr(feature = "serde", serde(deserialize_with = "report_number"))]
    pub reduced_chi2: f64,
    pub n: usize,
    pub iterations: usize,
    pub converged: bool,
    #[cfg_attr(feature = "serde", serde(default))]
    pub status: FitStatus,
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Vec::is_empty"))]
    pub warnings: Vec<String>,
    /// 被固定的参数（标准误记为 0，不计入自由度）。
    pub fixed: Vec<bool>,
    /// 结果停在下限或上限上的参数。
    pub at_bound: Vec<bool>,
    /// 残差自由度 n − 自由参数数。
    pub dof: usize,
    /// 参数协方差（固定参数的行列为 0）。
    #[cfg_attr(feature = "serde", serde(deserialize_with = "report_covariance"))]
    pub cov: Vec<Vec<f64>>,
    /// 用到的线性约束（用户写法）。
    pub constraints: Vec<String>,
    #[cfg_attr(feature = "serde", serde(deserialize_with = "report_number"))]
    pub x_min: f64,
    #[cfg_attr(feature = "serde", serde(deserialize_with = "report_number"))]
    pub x_max: f64,
}

impl NlFit {
    pub fn eval(&self, x: f64) -> f64 {
        self.model.eval(x, &self.params)
    }

    /// 在 x 处的置信带、预测带半宽（Δ 法：var(ŷ) = gᵀ·Cov·g，g 复用拟合雅可比梯度）。
    pub fn band(&self, x: f64, level: f64) -> (f64, f64) {
        let k = self.params.len();
        let free: Vec<usize> = (0..k).filter(|&j| !self.fixed[j]).collect();
        let row = jacobian(&self.model, &[x], &self.params, &free).remove(0);
        let mut g = vec![0.0; k];
        for (j, value) in free.into_iter().zip(row) {
            g[j] = value;
        }
        let var: f64 = (0..k)
            .map(|i| (0..k).map(|j| g[i] * self.cov[i][j] * g[j]).sum::<f64>())
            .sum();
        crate::analysis::band_half_widths(var, self.reduced_chi2, self.dof, level)
    }

    /// 写回曲线时在有理函数极点处留缺失值，屏幕和各导出格式共用断线数据。
    pub fn sample(&self, xs: &[f64]) -> Vec<f64> {
        xs.iter()
            .enumerate()
            .map(|(i, &x)| {
                if i > 0 && matches!(&self.model, FitFunction::Builtin(m) if m.crosses_pole(xs[i-1],x,&self.params)) {
                    f64::NAN
                } else {
                    self.eval(x)
                }
            })
            .collect()
    }

    /// 各参数的 t 值和双侧 p 值（固定参数为 NaN）。
    pub fn t_tests(&self) -> Vec<(f64, f64)> {
        crate::analysis::coef_t_tests(&self.params, &self.se, self.dof)
            .into_iter()
            .zip(&self.fixed)
            .map(|(tp, f)| if *f { (f64::NAN, f64::NAN) } else { tp })
            .collect()
    }
}

fn rss(model: &FitFunction, xs: &[f64], ys: &[f64], p: &[f64]) -> f64 {
    let s: f64 = xs.iter().zip(ys).map(|(x, y)| (y - model.eval(*x, p)).powi(2)).sum();
    if s.is_finite() {
        s
    } else {
        f64::INFINITY
    }
}

/// 雅可比 ∂f/∂p（有理模型解析求导，其余中心差分），只对 `free` 中的参数求导，n × free.len()。
fn jacobian(model: &FitFunction, xs: &[f64], p: &[f64], free: &[usize]) -> Vec<Vec<f64>> {
    if let FitFunction::Builtin(model) = model {
        if let Some((n, _)) = model.rational_degrees() {
            // 解析导数避免中心差分的扰动恰好落在有理函数极点上。
            return xs
                .iter()
                .map(|&x| {
                    let d = model.rational_denominator(x, p).0;
                    let y = model.eval(x, p);
                    free.iter()
                        .map(|&k| {
                            if k <= n {
                                x.powi(k as i32) / d
                            } else {
                                -y * x.powi((k - n) as i32) / d
                            }
                        })
                        .collect()
                })
                .collect();
        }
    }
    let mut j = vec![vec![0.0; free.len()]; xs.len()];
    let scale = p.iter().fold(0.0_f64, |max, x| max.max(x.abs()));
    let baseline: Vec<f64> = xs.iter().map(|x| model.eval(*x, p)).collect();
    for (col, &k) in free.iter().enumerate() {
        let mut h = f64::EPSILON.cbrt() * p[k].abs().max(1e-3 * scale).max(1e-8);
        let (mut up, mut down) = (p.to_vec(), p.to_vec());
        up[k] += h;
        down[k] -= h;
        let mut odd = 0.0_f64;
        let mut even = 0.0_f64;
        for (i, x) in xs.iter().enumerate() {
            let hi = model.eval(*x, &up);
            let lo = model.eval(*x, &down);
            odd += (hi - lo).powi(2);
            even += ((hi - baseline[i]) + (lo - baseline[i])).powi(2);
            j[i][col] = (hi - lo) / (2.0 * h);
        }
        // Limit central-difference truncation where the response changes sharply with a parameter.
        let ratio = (even / odd).sqrt();
        if ratio > 1e-5 {
            h *= (1e-5 / ratio).max(1e-3);
            up[k] = p[k] + h;
            down[k] = p[k] - h;
            for (i, x) in xs.iter().enumerate() {
                j[i][col] = (model.eval(*x, &up) - model.eval(*x, &down)) / (2.0 * h);
            }
        }
    }
    j
}

/// 单个参数的限制：固定值（优先）、下限、上限。
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ParamLimits {
    pub fixed: Option<f64>,
    pub lower: Option<f64>,
    pub upper: Option<f64>,
}

impl ParamLimits {
    pub fn fixed(v: f64) -> Self {
        Self {
            fixed: Some(v),
            ..Default::default()
        }
    }

    fn clamp(&self, v: f64) -> f64 {
        let v = self.lower.map_or(v, |lo| v.max(lo));
        self.upper.map_or(v, |hi| v.min(hi))
    }
}

/// 线性约束的关系。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rel {
    Eq,
    Le,
    Ge,
}

/// 参数间的线性约束 Σ coef[k]·p[k]（关系）rhs。
#[derive(Clone, Debug, PartialEq)]
pub struct LinearConstraint {
    pub coef: Vec<f64>,
    pub rel: Rel,
    pub rhs: f64,
    /// 用户写法，用于报告。
    pub text: String,
}

/// 解析参数约束：每行（或用分号隔开）一条，如 `A1 + A2 = 1`、`t2 >= t1 + 3`、`2*w <= 5`。
/// 两边都必须是参数的线性式（参数只能乘、除常数）。
pub fn parse_constraints(model: Model, text: &str) -> Result<Vec<LinearConstraint>, String> {
    if text.len() > crate::formula::INPUT_LIMITS.bytes {
        return Err("参数约束过长".into());
    }
    parse_constraints_for_names(model.params(), text)
}

pub fn parse_constraints_for_function(function: &FitFunction, text: &str) -> Result<Vec<LinearConstraint>, String> {
    parse_constraints_for_function_with(function, text, &crate::formula::INPUT_LIMITS)
}
pub fn parse_constraints_for_function_with(
    function: &FitFunction,
    text: &str,
    limits: &crate::formula::Limits,
) -> Result<Vec<LinearConstraint>, String> {
    if text.len() > limits.bytes {
        return Err("参数约束过长".into());
    }
    parse_constraints_for_names(&function.params(), text)
}

pub fn parse_constraints_for_names(names: &[&str], text: &str) -> Result<Vec<LinearConstraint>, String> {
    let mut out = vec![];
    for line in text.split(['\n', ';', '；']).map(str::trim).filter(|l| !l.is_empty()) {
        let line_norm = line.replace("≤", "<=").replace("≥", ">=").replace("==", "=");
        let (lhs, rel, rhs) = if let Some((a, b)) = line_norm.split_once("<=") {
            (a, Rel::Le, b)
        } else if let Some((a, b)) = line_norm.split_once(">=") {
            (a, Rel::Ge, b)
        } else if let Some((a, b)) = line_norm.split_once('<') {
            (a, Rel::Le, b)
        } else if let Some((a, b)) = line_norm.split_once('>') {
            (a, Rel::Ge, b)
        } else if let Some((a, b)) = line_norm.split_once('=') {
            (a, Rel::Eq, b)
        } else {
            return Err(format!("约束“{line}”缺少 =、<= 或 >="));
        };
        let (cl, kl) = linear_expr(lhs, names).map_err(|e| format!("约束“{line}”：{e}"))?;
        let (cr, kr) = linear_expr(rhs, names).map_err(|e| format!("约束“{line}”：{e}"))?;
        let coef: Vec<f64> = cl.iter().zip(&cr).map(|(a, b)| a - b).collect();
        if coef.iter().all(|c| *c == 0.0) {
            return Err(format!("约束“{line}”里没有参数"));
        }
        out.push(LinearConstraint {
            coef,
            rel,
            rhs: kr - kl,
            text: line.to_string(),
        });
    }
    Ok(out)
}

/// 线性式：返回 (各参数系数, 常数项)。
fn linear_expr(src: &str, names: &[&str]) -> Result<(Vec<f64>, f64), String> {
    // 词法：数字、参数名、+ − * /
    let cs: Vec<char> = src.chars().collect();
    let mut toks: Vec<String> = vec![];
    let mut i = 0;
    while i < cs.len() {
        let c = cs[i];
        if c.is_whitespace() {
            i += 1;
        } else if c.is_ascii_digit() || c == '.' {
            let start = i;
            while i < cs.len()
                && (cs[i].is_ascii_digit() || cs[i] == '.' || ((cs[i] == 'e' || cs[i] == 'E') && i + 1 < cs.len()))
            {
                if (cs[i] == 'e' || cs[i] == 'E') && i + 1 < cs.len() && (cs[i + 1] == '-' || cs[i + 1] == '+') {
                    i += 1;
                }
                i += 1;
            }
            toks.push(cs[start..i].iter().collect());
        } else if c.is_alphabetic() || c == '_' {
            let start = i;
            while i < cs.len() && (cs[i].is_alphanumeric() || cs[i] == '_') {
                i += 1;
            }
            toks.push(cs[start..i].iter().collect());
        } else if "+-*/−".contains(c) {
            toks.push(if c == '−' { "-".into() } else { c.to_string() });
            i += 1;
        } else {
            return Err(format!("无法识别的字符“{c}”"));
        }
    }
    if toks.is_empty() {
        return Err("缺少表达式".into());
    }
    let mut coef = vec![0.0; names.len()];
    let mut constant = 0.0;
    let mut pos = 0;
    let mut sign = 1.0;
    loop {
        // 一项：可带正负号，由数字和至多一个参数相乘除组成
        while pos < toks.len() && (toks[pos] == "+" || toks[pos] == "-") {
            if toks[pos] == "-" {
                sign = -sign;
            }
            pos += 1;
        }
        let mut value = sign;
        let mut param: Option<usize> = None;
        let mut op = "*".to_string();
        loop {
            let t = toks.get(pos).ok_or("表达式不完整")?;
            if let Ok(v) = t.parse::<f64>() {
                if op == "*" {
                    value *= v
                } else {
                    value /= v
                }
            } else if let Some(k) = names.iter().position(|n| n == t) {
                if param.is_some() || op == "/" {
                    return Err("约束必须是参数的线性式".into());
                }
                param = Some(k);
            } else {
                return Err(format!("“{t}”不是参数名，可用 {}", names.join("、")));
            }
            pos += 1;
            match toks.get(pos).map(String::as_str) {
                Some(o @ ("*" | "/")) => {
                    op = o.to_string();
                    pos += 1;
                }
                _ => break,
            }
        }
        match param {
            Some(k) => coef[k] += value,
            None => constant += value,
        }
        sign = 1.0;
        match toks.get(pos).map(String::as_str) {
            None => break,
            Some("+") | Some("-") => {}
            Some(t) => return Err(format!("“{t}”的位置不对")),
        }
    }
    Ok((coef, constant))
}

/// 矩阵行张成空间的正交补（每个基向量长 `dim`）；`rows` 为空时返回单位基。
fn null_space(rows: &[Vec<f64>], dim: usize) -> Vec<Vec<f64>> {
    let dot = |a: &[f64], b: &[f64]| a.iter().zip(b).map(|(x, y)| x * y).sum::<f64>();
    let mut basis: Vec<Vec<f64>> = vec![];
    let orth = |v: &[f64], basis: &mut Vec<Vec<f64>>| {
        let mut w = v.to_vec();
        for _ in 0..2 {
            for b in basis.iter() {
                let d = dot(&w, b);
                w.iter_mut().zip(b).for_each(|(x, y)| *x -= d * y);
            }
        }
        let norm = dot(&w, &w).sqrt();
        let scale = dot(v, v).sqrt().max(1.0);
        if norm > 1e-10 * scale {
            basis.push(w.iter().map(|x| x / norm).collect());
            true
        } else {
            false
        }
    };
    for r in rows {
        orth(r, &mut basis);
    }
    let rank = basis.len();
    let mut out = vec![];
    for j in 0..dim {
        let mut e = vec![0.0; dim];
        e[j] = 1.0;
        if orth(&e, &mut basis) {
            out.push(basis.last().unwrap().clone());
        }
        if out.len() + rank == dim {
            break;
        }
    }
    out
}

/// 用 QR 求逆估计协方差矩阵的无穷范数条件数。
fn covariance_ill_conditioned(cov: &[Vec<f64>], design: impl FnOnce() -> Vec<Vec<f64>>) -> bool {
    let n = cov.len();
    if n == 0 {
        return false;
    }
    let scale = cov.iter().flatten().fold(0.0_f64, |m, v| m.max(v.abs()));
    if scale == 0.0 {
        let design = design();
        return crate::linalg::lstsq(&design, &vec![0.0; design.len()]).is_err();
    }
    // 相关矩阵排除参数自身量级对条件数的影响。
    let diag: Vec<f64> = (0..n).map(|i| cov[i][i]).collect();
    if diag.iter().any(|d| !d.is_finite() || *d < 0.0) {
        return true;
    }
    let keep: Vec<usize> = (0..n).filter(|&i| diag[i] > 0.0).collect();
    if keep.is_empty() {
        return false;
    }
    let matrix: Vec<Vec<f64>> = keep
        .iter()
        .map(|&i| keep.iter().map(|&j| cov[i][j] / (diag[i] * diag[j]).sqrt()).collect())
        .collect();
    let n = keep.len();
    let norm = matrix
        .iter()
        .map(|row| row.iter().map(|v| v.abs()).sum::<f64>())
        .fold(0.0_f64, f64::max);
    let mut inverse_rows = vec![0.0; n];
    for col in 0..n {
        let mut rhs = vec![0.0; n];
        rhs[col] = 1.0;
        let Ok(solution) = crate::linalg::lstsq(&matrix, &rhs) else {
            return true;
        };
        for (sum, value) in inverse_rows.iter_mut().zip(solution.coef) {
            *sum += value.abs();
        }
    }
    norm * inverse_rows.into_iter().fold(0.0_f64, f64::max) > 1e12
}

/// 拟合 `model`；`initial` 为空时自动估计初值。`fixed` 为空或逐个参数给出固定值（`Some`）。
/// 非有限的点会被跳过。
pub fn fit(
    model: Model,
    xs: &[f64],
    ys: &[f64],
    initial: Option<&[f64]>,
    fixed: &[Option<f64>],
) -> Result<NlFit, String> {
    let limits: Vec<ParamLimits> = fixed
        .iter()
        .map(|f| ParamLimits {
            fixed: *f,
            ..Default::default()
        })
        .collect();
    fit_with(model, xs, ys, initial, &limits)
}

/// 带参数限制的拟合：固定参数不参与迭代；上下界用投影法——每步把试探参数截到界内，
/// 初值也先截到界内。标准误按停止点的雅可比计算（与 SciPy `curve_fit` 带 bounds 时相同口径）。
pub fn fit_with(
    model: Model,
    xs: &[f64],
    ys: &[f64],
    initial: Option<&[f64]>,
    limits: &[ParamLimits],
) -> Result<NlFit, String> {
    fit_constrained(model, xs, ys, initial, limits, &[])
}

/// 拟合过程的输入与迭代状态集中保存，独立步骤负责投影、迭代和统计。
struct FitSession<'a> {
    model: FitFunction,
    xs: Vec<f64>,
    ys: Vec<f64>,
    limits: &'a [ParamLimits],
    fixed: Vec<Option<f64>>,
    free: Vec<usize>,
    rows: Vec<(Vec<f64>, Rel, f64)>,
    params: Vec<f64>,
    cost: f64,
    lambda: f64,
    converged: bool,
    status: FitStatus,
    refine: bool,
    iterations: usize,
    constraint_text: Vec<String>,
}

impl<'a> FitSession<'a> {
    fn new(
        model: FitFunction,
        xs: &[f64],
        ys: &[f64],
        initial: Option<&[f64]>,
        limits: &'a [ParamLimits],
        constraints: &[LinearConstraint],
    ) -> Result<Self, String> {
        let names = model.params();
        let all = names.len();
        if initial.is_some_and(|values| values.len() != all) {
            return Err(format!("{} 需要 {all} 个初始参数", model.name()));
        }
        let fixed_values: Vec<Option<f64>> = limits.iter().map(|l| l.fixed).collect();
        let fixed = fixed_values.as_slice();
        let limit = |k: usize| limits.get(k).copied().unwrap_or_default();
        for (k, l) in limits.iter().enumerate() {
            if let (Some(lo), Some(hi)) = (l.lower, l.upper) {
                if lo >= hi || lo.is_nan() || hi.is_nan() {
                    return Err(format!(
                        "参数 {} 的下限必须小于上限",
                        names.get(k).copied().unwrap_or("?")
                    ));
                }
            }
            if l.lower.is_some_and(|v| !v.is_finite()) || l.upper.is_some_and(|v| !v.is_finite()) {
                return Err("上下限必须是有限数".into());
            }
        }
        let mut pts: Vec<(f64, f64)> = xs
            .iter()
            .zip(ys)
            .filter(|(x, y)| x.is_finite() && y.is_finite())
            .map(|(x, y)| (*x, *y))
            .collect();
        pts.sort_by(|a, b| a.0.total_cmp(&b.0));
        let (xs, ys): (Vec<f64>, Vec<f64>) = pts.into_iter().unzip();
        let is_fixed = |k: usize| fixed.get(k).is_some_and(|v| v.is_some());
        let free: Vec<usize> = (0..all).filter(|k| !is_fixed(*k)).collect();
        let m = free.len();
        if m == 0 {
            return Err("至少要有一个参数不固定".into());
        }
        if fixed.iter().flatten().any(|v| !v.is_finite()) {
            return Err("固定参数的值必须是有限数".into());
        }
        let n = xs.len();
        // 约束只作用在自由参数上：固定参数的贡献移到右边
        let rows: Vec<(Vec<f64>, Rel, f64)> = constraints
            .iter()
            .map(|c| {
                let a: Vec<f64> = free.iter().map(|&k| c.coef.get(k).copied().unwrap_or(0.0)).collect();
                let b = c.rhs
                    - (0..all)
                        .filter(|&k| is_fixed(k))
                        .map(|k| c.coef.get(k).copied().unwrap_or(0.0) * fixed[k].unwrap())
                        .sum::<f64>();
                (a, c.rel, b)
            })
            .collect();
        if rows.iter().any(|(a, _, _)| a.iter().all(|v| *v == 0.0)) {
            return Err("约束只涉及固定参数，请去掉或改为固定值".into());
        }
        let n_eq = null_space(
            &rows
                .iter()
                .filter(|r| r.1 == Rel::Eq)
                .map(|r| r.0.clone())
                .collect::<Vec<_>>(),
            m,
        )
        .len();
        if n <= n_eq {
            return Err(format!(
                "{} 有 {} 个参数，至少需要 {} 个有效数据点",
                model.name(),
                n_eq,
                n_eq + 1
            ));
        }
        if matches!(&model, FitFunction::Builtin(m) if m.rational_degrees().is_some())
            && xs.windows(2).filter(|pair| pair[0] != pair[1]).count() < n_eq
        {
            return Err(format!("{} 至少需要 {} 个不同的有效 X 值", model.name(), n_eq + 1));
        }
        let mut params = match initial {
            Some(v) => v.to_vec(),
            None => model
                .guess(&xs, &ys)
                .ok_or_else(|| format!("{} 需要提供每个参数的初始值", model.name()))?,
        };
        if params.iter().any(|value| !value.is_finite()) {
            return Err("初始参数必须为有限数".into());
        }
        for (k, v) in fixed.iter().enumerate() {
            if let Some(v) = v {
                params[k] = *v;
            }
        }
        for &k in &free {
            params[k] = limit(k).clamp(params[k]);
        }
        let lhs = |params: &[f64], a: &[f64]| free.iter().zip(a).map(|(&k, c)| c * params[k]).sum::<f64>();
        let violation = |params: &[f64], (a, rel, b): &(Vec<f64>, Rel, f64)| {
            let d = lhs(params, a) - b;
            match rel {
                Rel::Eq => d,
                Rel::Le => d.max(0.0),
                Rel::Ge => d.min(0.0),
            }
        };
        if !rows.is_empty() {
            // 初值投影到可行域：交替投影到各约束（超平面/半空间）并截到上下界
            let tol = |params: &[f64]| 1e-9 * free.iter().map(|&k| params[k].abs()).fold(1.0, f64::max);
            for _ in 0..500 {
                for row in &rows {
                    let d = violation(&params, row);
                    let norm2: f64 = row.0.iter().map(|c| c * c).sum();
                    if d != 0.0 {
                        for (&k, c) in free.iter().zip(&row.0) {
                            params[k] -= d * c / norm2;
                        }
                    }
                }
                for &k in &free {
                    params[k] = limit(k).clamp(params[k]);
                }
                if rows.iter().all(|r| violation(&params, r).abs() <= tol(&params)) {
                    break;
                }
            }
            if rows
                .iter()
                .any(|r| violation(&params, r).abs() > 1e-6 * free.iter().map(|&k| params[k].abs()).fold(1.0, f64::max))
            {
                return Err("约束之间（或与上下限、固定值）互相矛盾，找不到满足全部约束的参数".into());
            }
        }

        let cost = rss(&model, &xs, &ys, &params);
        if !cost.is_finite() {
            return Err("初值处函数值无效，请检查数据是否适合该函数".into());
        }
        Ok(Self {
            model,
            xs,
            ys,
            limits,
            fixed: fixed_values,
            free,
            rows,
            params,
            cost,
            lambda: 1e-3,
            converged: false,
            status: FitStatus::Converged,
            refine: false,
            iterations: 0,
            constraint_text: constraints.iter().map(|c| c.text.clone()).collect(),
        })
    }

    fn limit(&self, k: usize) -> ParamLimits {
        self.limits.get(k).copied().unwrap_or_default()
    }

    fn lhs(&self, p: &[f64], a: &[f64]) -> f64 {
        self.free.iter().zip(a).map(|(&k, c)| c * p[k]).sum()
    }

    fn active_tol(&self, p: &[f64]) -> f64 {
        1e-8 * self.free.iter().map(|&k| p[k].abs()).fold(1.0, f64::max)
    }

    fn optimize(&mut self) {
        while self.iterations < 500 {
            self.iterations += 1;
            let n = self.xs.len();
            let m = self.free.len();
            let j = jacobian(&self.model, &self.xs, &self.params, &self.free);
            let residuals: Vec<f64> = self
                .xs
                .iter()
                .zip(&self.ys)
                .map(|(x, y)| y - self.model.eval(*x, &self.params))
                .collect();
            let mut a = vec![vec![0.0; m]; m];
            let mut g = vec![0.0; m];
            for i in 0..n {
                for u in 0..m {
                    g[u] += j[i][u] * residuals[i];
                    for v in 0..m {
                        a[u][v] += j[i][u] * j[i][v];
                    }
                }
            }
            let open: Vec<usize> = (0..m)
                .filter(|&u| {
                    let (k, l) = (self.free[u], self.limit(self.free[u]));
                    !(l.lower == Some(self.params[k]) && g[u] < 0.0 || l.upper == Some(self.params[k]) && g[u] > 0.0)
                })
                .collect();
            if self.projected_gradient(&a, &g, &open) <= 1e-10 {
                self.converged = self.identifiable(&a, &g, &open);
                if !self.converged {
                    self.status = FitStatus::Failed;
                }
                break;
            }
            if self.refine {
                self.refine_solution();
                break;
            }
            let stepped = if self.rows.is_empty() {
                self.unconstrained_step(&a, &g, &open)
            } else {
                self.constrained_step(&a, &g, &open)
            };
            if !stepped {
                self.status = self.classify_stall(&a, &g, &open);
                self.converged = self.status != FitStatus::Failed;
                break;
            }
            if self.converged {
                self.converged = self.identifiable(&a, &g, &open);
                if !self.converged {
                    self.status = FitStatus::Failed;
                }
                break;
            }
        }
    }

    fn tangent_basis(&self, g: &[f64], open: &[usize]) -> Option<Vec<Vec<f64>>> {
        let mut active: Vec<(Vec<f64>, bool)> = self
            .rows
            .iter()
            .filter(|r| r.1 == Rel::Eq || (self.lhs(&self.params, &r.0) - r.2).abs() <= self.active_tol(&self.params))
            .map(|r| {
                let sign = if r.1 == Rel::Ge { -1.0 } else { 1.0 };
                (open.iter().map(|&u| sign * r.0[u]).collect(), r.1 == Rel::Eq)
            })
            .collect();
        while !active.is_empty() {
            let gram: Vec<Vec<f64>> = active
                .iter()
                .map(|(a, _)| {
                    active
                        .iter()
                        .map(|(b, _)| a.iter().zip(b).map(|(a, b)| a * b).sum())
                        .collect()
                })
                .collect();
            let inv = mat_inv(&gram)?;
            let rhs: Vec<f64> = active
                .iter()
                .map(|(row, _)| row.iter().zip(open).map(|(a, &u)| a * g[u]).sum())
                .collect();
            let multipliers: Vec<f64> = inv
                .iter()
                .map(|row| row.iter().zip(&rhs).map(|(a, b)| a * b).sum())
                .collect();
            let remove = active
                .iter()
                .enumerate()
                .filter(|(i, (_, equality))| !equality && multipliers[*i] < -1e-12)
                .min_by(|(i, _), (j, _)| multipliers[*i].total_cmp(&multipliers[*j]))
                .map(|(i, _)| i);
            match remove {
                Some(i) => {
                    active.remove(i);
                }
                None => break,
            }
        }
        let active: Vec<Vec<f64>> = active.into_iter().map(|(row, _)| row).collect();
        let basis = null_space(&active, open.len());
        Some(basis)
    }

    fn identifiable(&self, a: &[Vec<f64>], g: &[f64], open: &[usize]) -> bool {
        let Some(basis) = self.tangent_basis(g, open) else {
            return false;
        };
        if basis.is_empty() {
            return true;
        }
        let h: Vec<Vec<f64>> = basis
            .iter()
            .map(|z| {
                basis
                    .iter()
                    .map(|w| {
                        open.iter()
                            .enumerate()
                            .map(|(i, &u)| z[i] * open.iter().enumerate().map(|(j, &v)| a[u][v] * w[j]).sum::<f64>())
                            .sum()
                    })
                    .collect()
            })
            .collect();
        if (0..h.len()).any(|i| h[i][i] <= 0.0 || !h[i][i].is_finite()) {
            return false;
        }
        let scaled: Vec<Vec<f64>> = (0..h.len())
            .map(|i| (0..h.len()).map(|j| h[i][j] / (h[i][i] * h[j][j]).sqrt()).collect())
            .collect();
        mat_inv(&scaled).is_some()
    }

    fn projected_gradient(&self, a: &[Vec<f64>], g: &[f64], open: &[usize]) -> f64 {
        let Some(basis) = self.tangent_basis(g, open) else {
            return f64::INFINITY;
        };
        basis
            .iter()
            .map(|z| {
                let grad: f64 = open.iter().enumerate().map(|(i, &u)| z[i] * g[u]).sum();
                let norm: f64 = open
                    .iter()
                    .enumerate()
                    .map(|(i, &u)| z[i] * open.iter().enumerate().map(|(j, &v)| a[u][v] * z[j]).sum::<f64>())
                    .sum();
                let scale = (norm.max(0.0) * self.cost).sqrt();
                if scale == 0.0 {
                    if grad == 0.0 {
                        0.0
                    } else {
                        f64::INFINITY
                    }
                } else {
                    grad.abs() / scale
                }
            })
            .fold(0.0_f64, f64::max)
    }

    fn refinement_jacobian(&self, params: &[f64]) -> Vec<Vec<f64>> {
        let m = self.free.len();
        let scale = params.iter().fold(0.0_f64, |a, p| a.max(p.abs()));
        let baseline: Vec<f64> = self.xs.iter().map(|&x| self.model.eval(x, params)).collect();
        let mut jac = vec![vec![0.0; m]; self.xs.len()];
        for (u, &k) in self.free.iter().enumerate() {
            let mut h = f64::EPSILON.powf(0.2) * params[k].abs().max(1e-3 * scale).max(1e-8);
            let mut p1 = params.to_vec();
            let mut p2 = params.to_vec();
            for _ in 0..4 {
                p1[k] = params[k] + h;
                p2[k] = params[k] - h;
                let mut odd = 0.0_f64;
                let mut even = 0.0_f64;
                for (i, &x) in self.xs.iter().enumerate() {
                    let hi = self.model.eval(x, &p1);
                    let lo = self.model.eval(x, &p2);
                    odd += (hi - lo).powi(2);
                    even += ((hi - baseline[i]) + (lo - baseline[i])).powi(2);
                }
                let ratio = (even / odd).sqrt();
                if ratio <= 1e-5 || !ratio.is_finite() {
                    break;
                }
                h *= (1e-5 / ratio).max(1e-3);
            }
            p1[k] = params[k] + h;
            p2[k] = params[k] - h;
            let mut p3 = params.to_vec();
            let mut p4 = params.to_vec();
            p3[k] += 2.0 * h;
            p4[k] -= 2.0 * h;
            for (i, &x) in self.xs.iter().enumerate() {
                jac[i][u] = (8.0 * (self.model.eval(x, &p1) - self.model.eval(x, &p2))
                    - (self.model.eval(x, &p3) - self.model.eval(x, &p4)))
                    / (12.0 * h);
            }
        }
        jac
    }

    fn refinement_rss(&self, trial: &[f64]) -> f64 {
        let mut total = 0.0_f64;
        let mut correction = 0.0_f64;
        for (&x, &y) in self.xs.iter().zip(&self.ys) {
            let value = (y - self.model.eval(x, trial)).powi(2);
            let next = total + value;
            correction += if total.abs() >= value.abs() {
                (total - next) + value
            } else {
                (value - next) + total
            };
            total = next;
        }
        total + correction
    }

    fn refine_solution(&mut self) {
        for _ in 0..5 {
            let m = self.free.len();
            let jac = self.refinement_jacobian(&self.params);
            let residual: Vec<f64> = self
                .xs
                .iter()
                .zip(&self.ys)
                .map(|(&x, &y)| y - self.model.eval(x, &self.params))
                .collect();
            let mut a = vec![vec![0.0; m]; m];
            let mut g = vec![0.0; m];
            for (row, r) in jac.iter().zip(&residual) {
                for u in 0..m {
                    g[u] += row[u] * r;
                    for v in 0..m {
                        a[u][v] += row[u] * row[v];
                    }
                }
            }
            let open: Vec<usize> = (0..m)
                .filter(|&u| {
                    let k = self.free[u];
                    let l = self.limit(k);
                    !(l.lower == Some(self.params[k]) && g[u] < 0.0 || l.upper == Some(self.params[k]) && g[u] > 0.0)
                })
                .collect();
            let Some(basis) = self.tangent_basis(&g, &open) else {
                break;
            };
            if basis.is_empty() {
                self.converged = true;
                return;
            }
            let reduced: Vec<Vec<f64>> = jac
                .iter()
                .map(|row| {
                    basis
                        .iter()
                        .map(|z| open.iter().enumerate().map(|(i, &u)| z[i] * row[u]).sum())
                        .collect()
                })
                .collect();
            let Ok(solution) = crate::linalg::lstsq(&reduced, &residual) else {
                break;
            };
            let mut delta = vec![0.0; m];
            for (i, &u) in open.iter().enumerate() {
                delta[u] = basis.iter().zip(&solution.coef).map(|(z, c)| z[i] * c).sum();
            }
            let mut alpha = 1.0_f64;
            for (u, &k) in self.free.iter().enumerate() {
                let l = self.limit(k);
                if delta[u] > 0.0 {
                    if let Some(hi) = l.upper {
                        alpha = alpha.min((hi - self.params[k]).max(0.0) / delta[u]);
                    }
                }
                if delta[u] < 0.0 {
                    if let Some(lo) = l.lower {
                        alpha = alpha.min((lo - self.params[k]).min(0.0) / delta[u]);
                    }
                }
            }
            for (row, rel, rhs) in &self.rows {
                let change: f64 = row.iter().zip(&delta).map(|(a, d)| a * d).sum();
                let slack = rhs - self.lhs(&self.params, row);
                if *rel == Rel::Le && change > 0.0 {
                    alpha = alpha.min(slack.max(0.0) / change);
                }
                if *rel == Rel::Ge && change < 0.0 {
                    alpha = alpha.min(slack.min(0.0) / change);
                }
            }
            if alpha <= 0.0 {
                break;
            }
            let before = self.projected_gradient(&a, &g, &open);
            let mut trial = self.params.clone();
            for (u, &k) in self.free.iter().enumerate() {
                trial[k] += alpha * delta[u];
            }
            let new_cost = self.refinement_rss(&trial);
            if !new_cost.is_finite() || new_cost > self.cost * (1.0 + 4.0 * self.xs.len() as f64 * f64::EPSILON) {
                break;
            }
            let new_j = self.refinement_jacobian(&trial);
            let mut na = vec![vec![0.0; m]; m];
            let mut ng = vec![0.0; m];
            for (i, row) in new_j.iter().enumerate() {
                let r = self.ys[i] - self.model.eval(self.xs[i], &trial);
                for u in 0..m {
                    ng[u] += row[u] * r;
                    for v in 0..m {
                        na[u][v] += row[u] * row[v];
                    }
                }
            }
            let old_params = std::mem::replace(&mut self.params, trial);
            let old_cost = std::mem::replace(&mut self.cost, new_cost);
            let after = self.projected_gradient(&na, &ng, &open);
            if after >= before {
                self.params = old_params;
                self.cost = old_cost;
                break;
            }
            let relative = self
                .free
                .iter()
                .enumerate()
                .map(|(u, &k)| (alpha * delta[u]).abs() / old_params[k].abs().max(1.0))
                .fold(0.0_f64, f64::max);
            if after <= 1e-10 || relative <= 1e-13 {
                self.converged = self.identifiable(&na, &ng, &open);
                if !self.converged {
                    self.status = FitStatus::Failed;
                }
                return;
            }
        }
        let jac = jacobian(&self.model, &self.xs, &self.params, &self.free);
        let m = self.free.len();
        let mut a = vec![vec![0.0; m]; m];
        let mut g = vec![0.0; m];
        for (i, row) in jac.iter().enumerate() {
            let r = self.ys[i] - self.model.eval(self.xs[i], &self.params);
            for u in 0..m {
                g[u] += row[u] * r;
                for v in 0..m {
                    a[u][v] += row[u] * row[v];
                }
            }
        }
        let open: Vec<usize> = (0..m)
            .filter(|&u| {
                let k = self.free[u];
                let l = self.limit(k);
                !(l.lower == Some(self.params[k]) && g[u] < 0.0 || l.upper == Some(self.params[k]) && g[u] > 0.0)
            })
            .collect();
        self.status = self.classify_stall(&a, &g, &open);
        self.converged = self.status != FitStatus::Failed;
    }

    fn constrained_step(&mut self, a: &[Vec<f64>], g: &[f64], open: &[usize]) -> bool {
        let m = self.free.len();
        let mut stepped = false;
        let mut active: Vec<usize> = (0..self.rows.len())
            .filter(|&i| {
                let r = &self.rows[i];
                let ag: f64 = r.0.iter().zip(g).map(|(a, g)| a * g).sum();
                let tight = (self.lhs(&self.params, &r.0) - r.2).abs() <= self.active_tol(&self.params);
                match r.1 {
                    Rel::Eq => true,
                    Rel::Le => tight && ag > 0.0,
                    Rel::Ge => tight && ag < 0.0,
                }
            })
            .collect();
        let mut held: Vec<usize> = vec![];
        'solve: loop {
            let open_now: Vec<usize> = open.iter().copied().filter(|u| !held.contains(u)).collect();
            let c_open: Vec<Vec<f64>> = active
                .iter()
                .map(|&i| open_now.iter().map(|&u| self.rows[i].0[u]).collect())
                .collect();
            let basis = null_space(&c_open, open_now.len());
            if basis.is_empty() {
                break 'solve;
            }
            let r = basis.len();
            // H = Nᵀ A N，b = Nᵀ g
            let an: Vec<Vec<f64>> = open_now
                .iter()
                .map(|&u| {
                    (0..r)
                        .map(|q| open_now.iter().enumerate().map(|(j, &v)| a[u][v] * basis[q][j]).sum())
                        .collect()
                })
                .collect();
            let h0: Vec<Vec<f64>> = (0..r)
                .map(|q1| {
                    (0..r)
                        .map(|q2| (0..open_now.len()).map(|i| basis[q1][i] * an[i][q2]).sum())
                        .collect()
                })
                .collect();
            let nb: Vec<f64> = (0..r)
                .map(|q| open_now.iter().enumerate().map(|(i, &u)| basis[q][i] * g[u]).sum())
                .collect();
            while self.lambda < 1e16 {
                let mut h = h0.clone();
                for (q, row) in h.iter_mut().enumerate() {
                    row[q] += self.lambda * row[q].max(1e-12);
                }
                let Some(inv) = mat_inv(&h) else {
                    self.lambda *= 10.0;
                    continue;
                };
                let z: Vec<f64> = (0..r).map(|q| (0..r).map(|q2| inv[q][q2] * nb[q2]).sum()).collect();
                let mut delta = vec![0.0; m];
                for (i, &u) in open_now.iter().enumerate() {
                    delta[u] = (0..r).map(|q| basis[q][i] * z[q]).sum();
                }
                // 不越过不在活动集里的不等式和上下界；记下最先挡住的那个
                let mut alpha: f64 = 1.0;
                let mut blocker: Option<(bool, usize)> = None;
                for (i, row) in self.rows.iter().enumerate() {
                    if active.contains(&i) {
                        continue;
                    }
                    let ad: f64 = row.0.iter().zip(&delta).map(|(a, d)| a * d).sum();
                    let slack = row.2 - self.lhs(&self.params, &row.0);
                    let limit_here = match row.1 {
                        Rel::Le if ad > 0.0 => slack.max(0.0) / ad,
                        Rel::Ge if ad < 0.0 => slack.min(0.0) / ad,
                        _ => continue,
                    };
                    if limit_here < alpha {
                        alpha = limit_here;
                        blocker = Some((true, i));
                    }
                }
                for (u, &k) in self.free.iter().enumerate() {
                    let l = self.limit(k);
                    let limit_here = match (l.lower, l.upper) {
                        (_, Some(hi)) if delta[u] > 0.0 => (hi - self.params[k]).max(0.0) / delta[u],
                        (Some(lo), _) if delta[u] < 0.0 => (lo - self.params[k]).min(0.0) / delta[u],
                        _ => continue,
                    };
                    if limit_here < alpha {
                        alpha = limit_here;
                        blocker = Some((false, u));
                    }
                }
                if alpha <= 1e-12 {
                    match blocker {
                        Some((true, i)) => active.push(i),
                        Some((false, u)) => held.push(u),
                        None => {}
                    }
                    continue 'solve;
                }
                let mut trial = self.params.clone();
                for (u, &k) in self.free.iter().enumerate() {
                    trial[k] = self.limit(k).clamp(self.params[k] + alpha * delta[u]);
                }
                let new_cost = rss(&self.model, &self.xs, &self.ys, &trial);
                if new_cost < self.cost {
                    let low_damping = self.lambda <= 1e-6;
                    let small_step = self
                        .free
                        .iter()
                        .all(|&k| (trial[k] - self.params[k]).abs() <= 1e-12 * self.params[k].abs().max(1e-12));
                    let small_gain = self.cost - new_cost <= 1e-14 * self.cost.max(1e-300);
                    self.params = trial;
                    self.cost = new_cost;
                    self.lambda = (self.lambda / 10.0).max(1e-12);
                    stepped = true;
                    // 被约束截短的一步不算收敛：下一步会把挡路的约束纳入活动集
                    self.refine = small_gain && alpha >= 1.0;
                    self.converged = small_step && alpha >= 1.0 && low_damping;
                    break 'solve;
                }
                self.lambda *= 10.0;
            }
            break 'solve;
        }
        stepped
    }

    fn unconstrained_step(&mut self, a: &[Vec<f64>], g: &[f64], open: &[usize]) -> bool {
        let m = self.free.len();
        let mut stepped = false;
        while self.lambda < 1e16 && !open.is_empty() {
            let mut damped: Vec<Vec<f64>> = open.iter().map(|&u| open.iter().map(|&v| a[u][v]).collect()).collect();
            for (i, row) in damped.iter_mut().enumerate() {
                row[i] += self.lambda * a[open[i]][open[i]].max(1e-12);
            }
            let Some(inv) = mat_inv(&damped) else {
                self.lambda *= 10.0;
                continue;
            };
            let mut delta = vec![0.0; m];
            for (i, &u) in open.iter().enumerate() {
                delta[u] = open.iter().enumerate().map(|(j, &v)| inv[i][j] * g[v]).sum();
            }
            let mut trial = self.params.clone();
            for (d, &k) in delta.iter().zip(&self.free) {
                trial[k] = self.limit(k).clamp(trial[k] + d);
            }
            let new_cost = rss(&self.model, &self.xs, &self.ys, &trial);
            if new_cost < self.cost {
                let low_damping = self.lambda <= 1e-6;
                let small_step = self
                    .free
                    .iter()
                    .all(|&k| (trial[k] - self.params[k]).abs() <= 1e-12 * self.params[k].abs().max(1e-12));
                let small_gain = self.cost - new_cost <= 1e-14 * self.cost.max(1e-300);
                self.params = trial;
                self.cost = new_cost;
                self.lambda = (self.lambda / 10.0).max(1e-12);
                stepped = true;
                self.refine = small_gain;
                self.converged = small_step && low_damping;
                break;
            }
            self.lambda *= 10.0;
        }
        stepped
    }

    fn classify_stall(&self, a: &[Vec<f64>], g: &[f64], open: &[usize]) -> FitStatus {
        if self.cost <= f64::EPSILON * self.ys.iter().map(|y| y * y).sum::<f64>() {
            return FitStatus::Converged;
        }
        if open.is_empty() {
            return FitStatus::Converged;
        }
        let Some(basis) = self.tangent_basis(g, open) else {
            return FitStatus::Failed;
        };
        if basis.is_empty() {
            return FitStatus::Converged;
        }
        let grad: Vec<f64> = basis
            .iter()
            .map(|z| open.iter().enumerate().map(|(i, &u)| z[i] * g[u]).sum())
            .collect();
        let h: Vec<Vec<f64>> = basis
            .iter()
            .map(|z| {
                basis
                    .iter()
                    .map(|w| {
                        open.iter()
                            .enumerate()
                            .map(|(i, &u)| z[i] * open.iter().enumerate().map(|(j, &v)| a[u][v] * w[j]).sum::<f64>())
                            .sum()
                    })
                    .collect()
            })
            .collect();
        let scaled = grad
            .iter()
            .enumerate()
            .map(|(i, g)| {
                let denom = (h[i][i].max(0.0) * self.cost).sqrt();
                if denom == 0.0 {
                    if *g == 0.0 {
                        0.0
                    } else {
                        f64::INFINITY
                    }
                } else {
                    g.abs() / denom
                }
            })
            .fold(0.0_f64, f64::max);
        if scaled <= 1e-6 {
            return FitStatus::Converged;
        }
        let mut damped = h.clone();
        for (i, row) in damped.iter_mut().enumerate() {
            row[i] += 1e-12 * h[i][i].max(1e-12);
        }
        let Some(inv) = mat_inv(&damped) else {
            return FitStatus::Failed;
        };
        let delta: Vec<f64> = inv
            .iter()
            .map(|row| row.iter().zip(&grad).map(|(a, g)| a * g).sum())
            .collect();
        let predicted: f64 = delta
            .iter()
            .enumerate()
            .map(|(i, d)| d * (grad[i] - 0.5 * h[i].iter().zip(&delta).map(|(a, d)| a * d).sum::<f64>()))
            .sum();
        if predicted >= 0.0 && predicted <= 10.0 * self.xs.len() as f64 * f64::EPSILON * self.cost {
            FitStatus::PrecisionLimit
        } else {
            FitStatus::Failed
        }
    }

    fn finish(self) -> Result<NlFit, String> {
        let model = self.model.clone();
        let xs = &self.xs;
        let free = &self.free;
        let n = xs.len();
        let m = free.len();
        let all = model.params().len();
        let is_fixed = |k: usize| self.fixed.get(k).is_some_and(|v| v.is_some());
        let active_tol = |p: &[f64]| self.active_tol(p);
        let lhs = |p: &[f64], a: &[f64]| self.lhs(p, a);
        let cost = self.cost;
        let j = jacobian(&self.model, &self.xs, &self.params, &self.free);
        let mut a = vec![vec![0.0; m]; m];
        for row in &j {
            for u in 0..m {
                for v in 0..m {
                    a[u][v] += row[u] * row[v];
                }
            }
        }
        // 约束：在停止点的等式与贴边不等式零空间里算协方差，自由度扣除约束个数
        let tight: Vec<Vec<f64>> = self
            .rows
            .iter()
            .filter(|r| r.1 == Rel::Eq || (lhs(&self.params, &r.0) - r.2).abs() <= active_tol(&self.params))
            .map(|r| r.0.clone())
            .collect();
        let basis = null_space(&tight, m);
        let dof_n = n - basis.len();
        let dof = dof_n as f64;
        let reduced = self.cost / dof;
        let mut se = vec![0.0; all];
        let mut cov = vec![vec![0.0; all]; all];
        if !self.rows.is_empty() {
            let r = basis.len();
            let h: Vec<Vec<f64>> = (0..r)
                .map(|q1| {
                    (0..r)
                        .map(|q2| {
                            (0..m)
                                .map(|u| (0..m).map(|v| basis[q1][u] * a[u][v] * basis[q2][v]).sum::<f64>())
                                .sum()
                        })
                        .collect()
                })
                .collect();
            match mat_inv(&h) {
                Some(inv) => {
                    for (u, &k) in self.free.iter().enumerate() {
                        for (v, &l) in self.free.iter().enumerate() {
                            cov[k][l] = (0..r)
                                .map(|q1| (0..r).map(|q2| basis[q1][u] * inv[q1][q2] * basis[q2][v]).sum::<f64>())
                                .sum::<f64>()
                                * reduced;
                        }
                        se[k] = cov[k][k].max(0.0).sqrt();
                    }
                }
                None => {
                    for &k in &self.free {
                        se[k] = f64::NAN;
                    }
                }
            }
        } else {
            match mat_inv(&a) {
                Some(inv) => {
                    for (u, &k) in self.free.iter().enumerate() {
                        se[k] = (inv[u][u] * reduced).max(0.0).sqrt();
                        for (v, &l) in self.free.iter().enumerate() {
                            cov[k][l] = inv[u][v] * reduced;
                        }
                    }
                }
                None => {
                    for &k in &self.free {
                        se[k] = f64::NAN;
                    }
                }
            }
        }
        let at_bound: Vec<bool> = (0..all)
            .map(|k| {
                let l = self.limit(k);
                !is_fixed(k) && (l.lower == Some(self.params[k]) || l.upper == Some(self.params[k]))
            })
            .collect();
        let fixed_flags: Vec<bool> = (0..all).map(is_fixed).collect();
        let constraint_text = self.constraint_text;
        let params = self.params;
        let iterations = self.iterations;
        let converged = self.converged;
        let x_min = self.xs[0];
        let x_max = self.xs[n - 1];
        let mean = self.ys.iter().sum::<f64>() / n as f64;
        let tss: f64 = self.ys.iter().map(|y| (y - mean).powi(2)).sum();
        let r2 = if tss > 0.0 { 1.0 - self.cost / tss } else { f64::NAN };
        let adj_r2 = 1.0 - (1.0 - r2) * (n as f64 - 1.0) / dof;
        let reduced_cov: Vec<Vec<f64>> = basis
            .iter()
            .map(|left| {
                basis
                    .iter()
                    .map(|right| {
                        (0..m)
                            .map(|u| (0..m).map(|v| left[u] * cov[free[u]][free[v]] * right[v]).sum::<f64>())
                            .sum()
                    })
                    .collect()
            })
            .collect();

        let mut warnings = Vec::new();
        if converged {
            let ill = adj_r2 < 0.0
                || covariance_ill_conditioned(&reduced_cov, || {
                    j.iter()
                        .map(|row| {
                            basis
                                .iter()
                                .map(|z| row.iter().zip(z).map(|(v, z)| v * z).sum())
                                .collect()
                        })
                        .collect()
                });
            if ill {
                warnings.push("结果可能停在局部极小或参数不可辨识：请尝试其他初值，或固定部分参数。".to_owned());
            }
            let parameter_names = model.params();
            let names: Vec<&str> = free
                .iter()
                .filter(|&&k| se[k].is_finite() && se[k] > 10.0 * params[k].abs())
                .map(|&k| parameter_names[k])
                .collect();
            if !names.is_empty() {
                warnings.push(format!(
                    "参数 {} 的标准误超过其绝对值的 10 倍，与 0 无显著差异；如物理上应为 0，可考虑固定。",
                    names.join("、")
                ));
            }
        }
        Ok(NlFit {
            model,
            params,
            se,
            rss: cost,
            r2,
            adj_r2,
            reduced_chi2: reduced,
            n,
            iterations,
            converged,
            status: if converged { self.status } else { FitStatus::Failed },
            warnings,
            fixed: fixed_flags,
            at_bound,
            dof: dof_n,
            cov,
            constraints: constraint_text,
            x_min,
            x_max,
        })
    }
}

/// 带参数限制与线性约束的拟合。
pub fn fit_constrained(
    model: Model,
    xs: &[f64],
    ys: &[f64],
    initial: Option<&[f64]>,
    limits: &[ParamLimits],
    constraints: &[LinearConstraint],
) -> Result<NlFit, String> {
    fit_function(FitFunction::Builtin(model), xs, ys, initial, limits, constraints)
}

pub fn fit_function(
    function: FitFunction,
    xs: &[f64],
    ys: &[f64],
    initial: Option<&[f64]>,
    limits: &[ParamLimits],
    constraints: &[LinearConstraint],
) -> Result<NlFit, String> {
    let mut session = FitSession::new(function, xs, ys, initial, limits, constraints)?;
    session.optimize();
    session.finish()
}

pub fn fit_custom(
    function: CustomFitFunction,
    xs: &[f64],
    ys: &[f64],
    initial: &[f64],
    limits: &[ParamLimits],
    constraints: &[LinearConstraint],
) -> Result<NlFit, String> {
    fit_function(
        FitFunction::Custom(function),
        xs,
        ys,
        Some(initial),
        limits,
        constraints,
    )
}
#[cfg(test)]
mod tests {
    use super::*;

    fn noise(i: usize) -> f64 {
        let i = i as f64;
        (1.7 * i).sin() * 0.6 + (0.37 * i * i).cos() * 0.4
    }

    fn data(model: Model, from: f64, to: f64, truth: &[f64], amp: f64) -> (Vec<f64>, Vec<f64>) {
        let xs: Vec<f64> = (0..41).map(|i| from + (to - from) * i as f64 / 40.0).collect();
        let ys = xs
            .iter()
            .enumerate()
            .map(|(i, x)| model.eval(*x, truth) + amp * noise(i))
            .collect();
        (xs, ys)
    }

    /// (函数, X 起点, X 终点, 真值, 噪声幅度, 参考参数, 参考标准误, 参考 RSS)
    type Case<'a> = (Model, f64, f64, &'a [f64], f64, &'a [f64], &'a [f64], f64);

    fn close(got: f64, want: f64, rel: f64) {
        assert!((got - want).abs() <= rel * want.abs().max(1e-9), "{got} vs {want}");
    }

    const AUTO_START_CASES: [Case<'static>; 13] = [
        (
            Model::Voigt,
            0.0,
            10.0,
            &[0.5, 5.0, 8.0, 1.2, 0.8],
            0.03,
            &[
                0.502987389692305,
                4.9994087742597,
                7.98905574485161,
                1.212719629821541,
                0.790032894133966,
            ],
            &[
                0.006207464376162809,
                0.0022433759402392768,
                0.07572102365167718,
                0.025374812557617577,
                0.03208450677549791,
            ],
            0.01072010972019333,
        ),
        (
            Model::PsdVoigt1,
            0.0,
            10.0,
            &[0.3, 4.5, 6.0, 1.5, 0.4],
            0.03,
            &[
                0.3009534141880661,
                4.499172087721348,
                6.021337487717337,
                1.4959212882150772,
                0.4103596138827938,
            ],
            &[
                0.005587425411888454,
                0.0023442627654601854,
                0.06509666834675183,
                0.0071941672597614845,
                0.024730834968163404,
            ],
            0.010758003239766188,
        ),
        (
            Model::LogNormal,
            0.2,
            10.0,
            &[0.2, 3.0, 0.4, 10.0],
            0.03,
            &[
                0.20191298275817007,
                3.000019482867363,
                0.40007606359111575,
                10.005413179919492,
            ],
            &[
                0.004615721739269777,
                0.0036396418783872795,
                0.0012550820665772344,
                0.038148829178791185,
            ],
            0.010908537439269561,
        ),
        (
            Model::ExpDec2,
            0.0,
            10.0,
            &[0.5, 4.0, 0.8, 2.0, 4.0],
            0.02,
            &[
                0.4937960013006856,
                4.01551121614994,
                0.7974110921387975,
                2.002545767816202,
                4.046387326181638,
            ],
            &[
                0.019735941188441294,
                0.06279795003108511,
                0.012293474333771807,
                0.048313578591315055,
                0.1865327482325991,
            ],
            0.00466318079317071,
        ),
        (
            Model::ExpDec3,
            0.0,
            20.0,
            &[0.2, 3.0, 0.5, 2.0, 2.5, 1.5, 9.0],
            0.01,
            &[
                0.1559698425034319,
                3.0833964991005396,
                0.5123124336847743,
                2.1498771149859217,
                2.7657495595984742,
                1.3151952150003738,
                10.713480339384695,
            ],
            &[
                0.04963758730807458,
                0.05390947691399579,
                0.008111631918091625,
                0.16100156150396602,
                0.22126250699496686,
                0.15655332356926485,
                1.9724746003585718,
            ],
            0.001095159831052157,
        ),
        (
            Model::ExpGro1,
            0.0,
            5.0,
            &[1.0, 0.5, 1.5],
            0.05,
            &[1.0036677407365988, 0.5015354665130808, 1.5017889664546158],
            &[0.013547135634189463, 0.005649400152885607, 0.005030990698243008],
            0.02974095858852069,
        ),
        (
            Model::ExpAssoc,
            0.0,
            10.0,
            &[0.5, 3.0, 0.7, 2.0, 4.0],
            0.02,
            &[
                0.5121172136536161,
                2.982998017016836,
                0.7024269163748263,
                1.9988689165175193,
                3.9534873311285605,
            ],
            &[
                0.010467899233794022,
                0.049455331835194315,
                0.012954363639320485,
                0.03733528622375226,
                0.15270653325894512,
            ],
            0.0046612916151662015,
        ),
        (
            Model::DoseResp,
            -3.0,
            3.0,
            &[1.0, 9.0, 0.3, 1.2],
            0.05,
            &[
                1.0074086343720887,
                8.998715240450007,
                0.29953788120668495,
                1.203679123501137,
            ],
            &[
                0.007757719078944026,
                0.009197507525541885,
                0.0025321707989441815,
                0.00744571058800078,
            ],
            0.029938095567038366,
        ),
        (
            Model::Hill,
            0.1,
            10.0,
            &[1.0, 8.0, 2.5, 2.2],
            0.05,
            &[
                1.0189718518346893,
                7.98959662778498,
                2.500333429637884,
                2.217168150144736,
            ],
            &[
                0.01797666302307224,
                0.017193247431952313,
                0.009893466179771038,
                0.020345891202554135,
            ],
            0.02953457394949384,
        ),
        (
            Model::Gompertz,
            0.0,
            10.0,
            &[6.0, 3.0, 0.9],
            0.05,
            &[6.001514782537589, 2.997167483936787, 0.9002960676563785],
            &[0.008223440236067856, 0.005550430110585135, 0.006264636574684849],
            0.03056073914187006,
        ),
        (
            Model::Allometric2,
            0.5,
            10.0,
            &[1.0, 0.8, 1.6],
            0.05,
            &[1.0063233380609855, 0.8003853338039472, 1.5996609617553645],
            &[0.01309187654574319, 0.0038208430944108016, 0.0020292713044496834],
            0.029939208157724446,
        ),
        (
            Model::Power1,
            1.0,
            10.0,
            &[2.0, 0.5, 0.7],
            0.02,
            &[1.9994608903244961, 0.4967582288374258, 0.7000192675871021],
            &[0.0071972334099453935, 0.007057862408701341, 0.0015076629812576963],
            0.004784781759805734,
        ),
        (
            Model::Log3P1,
            0.5,
            10.0,
            &[3.0, 1.2, 0.5],
            0.02,
            &[2.997125235443011, 1.1988014402935538, 0.4915073331562746],
            &[0.020361190029973577, 0.00866411045169326, 0.02210971956403735],
            0.0047744246482707055,
        ),
    ];

    #[test]
    fn added_builtin_functions_converge_to_scipy_curve_fit() {
        // 参考值：scipy.optimize.curve_fit（SciPy 1.17.1，p0 取真值）；这里用自动初值
        let cases = AUTO_START_CASES;
        for (model, from, to, truth, amp, params, se, rss) in cases {
            let (xs, ys) = data(model, from, to, truth, amp);
            let f = fit(model, &xs, &ys, None, &[]).unwrap_or_else(|e| panic!("{}: {e}", model.name()));
            assert!(f.converged, "{}", model.name());
            for (g, w) in f.params.iter().zip(params) {
                close(*g, *w, 1e-5);
            }
            for (g, w) in f.se.iter().zip(se) {
                close(*g, *w, 1e-3);
            }
            close(f.rss, rss, 1e-7);
        }
    }

    #[test]
    fn voigt_profile_matches_scipy() {
        // scipy.special.voigt_profile(x, sigma, gamma)
        for (x, sigma, gamma, want) in [
            (0.0, 1.0, 0.5, 0.27895547038929436),
            (1.3, 0.4, 0.2, 0.05594361959542218),
            (-2.5, 0.7, 1.1, 0.05536634494387373),
            (0.05, 2.0, 0.01, 0.19861602219299082),
            (8.0, 0.5, 0.3, 0.0015077335480456885),
            (0.3, 0.1, 3.0, 0.10494211022325665),
        ] {
            let got = voigt_profile(x, sigma, gamma);
            assert!(
                (got - want).abs() <= 1e-12 * want.abs().max(1e-300) + 1e-15,
                "{x} {sigma} {gamma}: {got} vs {want}"
            );
        }
    }

    #[test]
    fn auto_initial_values_converge_to_scipy_curve_fit() {
        // 参考值：scipy.optimize.curve_fit（SciPy 1.17.1，p0 取真值）
        let cases: [Case; 6] = [
            (
                Model::Gauss,
                0.0,
                10.0,
                &[1.0, 4.2, 1.5, 6.0],
                0.05,
                &[
                    1.0048915270703311,
                    4.200181394465829,
                    1.4950191190305495,
                    5.99169753813478,
                ],
                &[
                    0.005688688212796727,
                    0.0040927900045844,
                    0.009008364506171394,
                    0.03631036450750082,
                ],
                0.030054878704556094,
            ),
            (
                Model::Lorentz,
                0.0,
                10.0,
                &[0.5, 5.5, 1.2, 4.0],
                0.05,
                &[
                    0.5016682833136684,
                    5.504488807447752,
                    1.214493978640595,
                    4.030696352633713,
                ],
                &[
                    0.006520497789558827,
                    0.00587031756445813,
                    0.020346990941884486,
                    0.057702833830455866,
                ],
                0.029429245533392035,
            ),
            (
                Model::ExpDec1,
                0.0,
                8.0,
                &[0.3, 5.0, 1.7],
                0.05,
                &[0.302890515973314, 5.011675158701781, 1.6976676601354435],
                &[0.008905602297414812, 0.017482735005634128, 0.013692057251406079],
                0.02997825443344604,
            ),
            (
                Model::Boltzmann,
                -5.0,
                5.0,
                &[1.0, 9.0, 0.7, 0.8],
                0.1,
                &[
                    1.0155540784588701,
                    8.9923798599876,
                    0.6965204833395424,
                    0.7950285733396781,
                ],
                &[
                    0.017004313951120623,
                    0.022821143265601744,
                    0.010819965534474214,
                    0.009807508460495412,
                ],
                0.11945876884193626,
            ),
            (
                Model::Logistic,
                0.1,
                10.0,
                &[0.5, 8.0, 3.0, 2.5],
                0.1,
                &[
                    0.52815741501568,
                    7.982528548772376,
                    2.997221577226676,
                    2.527521360279785,
                ],
                &[
                    0.030917244118693735,
                    0.035357795335685976,
                    0.018914504733669135,
                    0.041687185127689624,
                ],
                0.11906754992937188,
            ),
            (
                Model::Allometric1,
                0.5,
                10.0,
                &[2.0, 0.75],
                0.05,
                &[2.0059992752066784, 0.7486201278337822],
                &[0.005861355038033662, 0.0014942798002535167],
                0.030083812104675662,
            ),
        ];
        for (model, from, to, truth, amp, params, se, rss) in cases {
            let (xs, ys) = data(model, from, to, truth, amp);
            let f = fit(model, &xs, &ys, None, &[]).unwrap_or_else(|e| panic!("{}: {e}", model.name()));
            assert!(f.converged, "{}", model.name());
            for (g, w) in f.params.iter().zip(params) {
                close(*g, *w, 1e-6);
            }
            for (g, w) in f.se.iter().zip(se) {
                close(*g, *w, 1e-4);
            }
            close(f.rss, rss, 1e-8);
            assert!(f.r2 > 0.99, "{} r2 {}", model.name(), f.r2);
        }
    }

    #[test]
    fn adsorption_and_kinetics_models_match_scipy_from_automatic_starts() {
        // 参考值：scipy.optimize.curve_fit（SciPy 1.17.1，p0 取真值）
        let cases: [Case; 5] = [
            (
                Model::Langmuir,
                0.5,
                200.0,
                &[45.0, 0.03],
                0.5,
                &[44.97634745234103, 0.030183478087933875],
                &[0.1522995827714246, 0.0003879665171647043],
                3.0473473280763974,
            ),
            (
                Model::Freundlich,
                0.5,
                200.0,
                &[6.0, 2.5],
                0.3,
                &[6.032052384932666, 2.506398449728879],
                &[0.03619766608481412, 0.00787496285918195],
                1.0853667639848175,
            ),
            (
                Model::PseudoFirstOrder,
                0.0,
                300.0,
                &[32.0, 0.02],
                0.4,
                &[32.01391575943531, 0.020055996739953872],
                &[0.05556218278874523, 0.00014929094003738433],
                1.9534535473442134,
            ),
            (
                Model::PseudoSecondOrder,
                0.0,
                300.0,
                &[35.0, 0.001],
                0.4,
                &[35.00119576266527, 0.0010050576101159605],
                &[0.08793642812072777, 1.5166045776723856e-05],
                1.9524552085415017,
            ),
            (
                Model::MichaelisMenten,
                0.1,
                20.0,
                &[12.0, 2.5],
                0.1,
                &[11.99732714274374, 2.4887804519721763],
                &[0.025429546861139248, 0.023329314484644446],
                0.12179256523091592,
            ),
        ];
        for (model, from, to, truth, amp, params, se, rss) in cases {
            let (xs, ys) = data(model, from, to, truth, amp);
            let f = fit(model, &xs, &ys, None, &[]).unwrap_or_else(|e| panic!("{}: {e}", model.name()));
            assert!(f.converged, "{}", model.name());
            for (g, w) in f.params.iter().zip(params) {
                close(*g, *w, 1e-6);
            }
            for (g, w) in f.se.iter().zip(se) {
                close(*g, *w, 1e-4);
            }
            close(f.rss, rss, 1e-8);
        }
        assert_eq!(Model::ALL.len(), 28);
        assert!(
            Model::ALL.iter().all(|m| m.params().len() <= MAX_PARAMS),
            "the dialog holds up to MAX_PARAMS parameters"
        );
        assert!(Model::ALL.iter().all(|m| Model::CATEGORIES.contains(&m.category())));
    }

    #[test]
    fn rational1_fits_a_synthetic_curve_from_automatic_initial_values() {
        let xs: Vec<f64> = (0..80).map(|i| i as f64 / 10.0).collect();
        let truth = [2.0, 1.5, 0.2];
        let ys: Vec<f64> = xs.iter().map(|x| Model::Rational1.eval(*x, &truth)).collect();
        let fit = fit(Model::Rational1, &xs, &ys, None, &[]).unwrap();
        assert!(fit.converged);
        for (actual, expected) in fit.params.iter().zip(truth) {
            assert!((actual - expected).abs() < 1e-5, "{actual} != {expected}");
        }
    }

    #[test]
    fn rational_models_match_scipy_parameters_standard_errors_and_rss() {
        // SciPy 1.17.1 curve_fit (LM), truth as p0, ftol/xtol/gtol=1e-12.
        let cases: &[Case<'_>] = &[
            (
                Model::Rational1,
                0.0,
                8.0,
                &[2.0, 1.5, 0.2],
                0.03,
                &[2.0071055689891617, 1.497719878159007, 0.19970195484327713],
                &[0.010078166985916364, 0.01785721057417291, 0.00335576591150021],
                0.010795499502041412,
            ),
            (
                Model::Rational12,
                0.0,
                8.0,
                &[2.0, 1.5, 0.2, 0.08],
                0.01,
                &[
                    2.005222897698752,
                    1.4786863135432924,
                    0.19484277304441625,
                    0.07925964599871921,
                ],
                &[
                    0.004346310246490535,
                    0.021496823321334266,
                    0.005472821024938503,
                    0.0007447918727547438,
                ],
                0.0011701986072127214,
            ),
            (
                Model::Rational22,
                -2.0,
                4.0,
                &[1.2, -0.7, 0.9, 0.3, 0.15],
                0.003,
                &[
                    1.200310753957403,
                    -0.6993557070520809,
                    0.8997781686620413,
                    0.3001524075835758,
                    0.14995924206432937,
                ],
                &[
                    0.0006036432447350066,
                    0.00099203707524957,
                    0.0008214396171027677,
                    0.00037667994231782516,
                    0.00026265222627010556,
                ],
                0.00010672710862900706,
            ),
            (
                Model::Rational33,
                0.0,
                5.0,
                &[1.2, -0.7, 0.9, 0.12, 0.3, 0.15, 0.04],
                0.0001,
                &[
                    1.2000638280357157,
                    -0.700261497061041,
                    0.9004546635278622,
                    0.11973771615739744,
                    0.30007139688404433,
                    0.1499586416629403,
                    0.039948132902852256,
                ],
                &[
                    5.4458357672916986e-05,
                    0.0033117482703237267,
                    0.0023135457218948583,
                    0.002234224011968617,
                    0.0031440950220097638,
                    0.0005399006932944043,
                    0.0005885503164424945,
                ],
                1.1486720784888452e-07,
            ),
        ];
        for &(model, from, to, truth, amp, params, se, rss) in cases {
            let (xs, ys) = data(model, from, to, truth, amp);
            let result = fit(model, &xs, &ys, None, &[]).unwrap();
            assert!(result.converged, "{}", model.name());
            for (actual, expected) in result.params.iter().zip(params) {
                close(*actual, *expected, 2e-4);
            }
            for (actual, expected) in result.se.iter().zip(se) {
                close(*actual, *expected, 2e-3);
            }
            close(result.rss, rss, 1e-7);
        }
    }

    #[test]
    fn rational_models_reject_poles_and_handle_repeated_x_without_panicking() {
        for (model, p) in [
            (Model::Rational1, vec![2.0, 1.0, -1.0]),
            (Model::Rational12, vec![2.0, 1.0, -2.0, 1.0]),
            (Model::Rational22, vec![2.0, 1.0, 0.3, -2.0, 1.0]),
            (Model::Rational33, vec![2.0, 1.0, 0.3, 0.1, -3.0, 3.0, -1.0]),
        ] {
            assert!(model.eval(1.0, &p).is_nan());
            assert!(model.eval(0.0, &p).is_finite());
            assert!(model.eval(1.0 + f64::EPSILON, &p).is_nan());
            let xs = [1.0; 12];
            let ys = [2.0; 12];
            assert!(fit(model, &xs, &ys, Some(&p), &[]).is_err());
            assert!(fit(model, &xs, &ys, None, &[]).is_err());
        }
    }

    #[test]
    fn rational_sampling_breaks_at_simple_double_and_paired_poles() {
        assert!(Model::Rational1.crosses_pole(-1.1, -0.9, &[2., 1., 1.]));
        assert!(!Model::Rational1.crosses_pole(0., 1., &[2., 1., 1.]));
        for b in [[-2., 1.], [-25. / 6., 25. / 6.]] {
            assert!(Model::Rational12.crosses_pole(0., 1., &[2., 1., b[0], b[1]]));
            assert!(Model::Rational12.crosses_pole(1., 0., &[2., 1., b[0], b[1]]));
        }
        let xs = [-2., -1.8, -1.5, -0.5, -0.2, 0., 0.2, 0.4];
        let ys: Vec<_> = xs.iter().map(|x| Model::Rational1.eval(*x, &[2., 1., 1.])).collect();
        let fit = fit(Model::Rational1, &xs, &ys, None, &[]).unwrap();
        let values = fit.sample(&[-1.1, -0.9, 0.]);
        assert!(values[0].is_finite());
        assert!(values[1].is_nan());
        assert!(values[2].is_finite());
    }

    #[test]
    fn constraint_boundary_optimum_is_converged() {
        let function =
            FitFunction::Custom(CustomFitFunction::new("line", "a + b*x", vec!["a".into(), "b".into()]).unwrap());
        let constraints = parse_constraints_for_function(&function, "a >= 0").unwrap();
        let xs = [0.0, 1.0, 2.0, 3.0, 4.0];
        let ys = [-1.0, 1.0, 3.0, 5.0, 7.0];
        let result = fit_function(function, &xs, &ys, Some(&[0.0, 1.0]), &[], &constraints).unwrap();
        assert!(result.converged);
        assert_eq!(result.status, FitStatus::Converged);
        assert!(result.params[0].abs() < 1e-8);
    }

    #[test]
    fn enso_certified_start_difference_step_diagnostic() {
        let data = include_str!("../tests/nist/nls/ENSO.dat");
        let mut params = Vec::new();
        let mut rows = Vec::new();
        let mut in_data = false;
        for line in data.lines() {
            let words: Vec<_> = line.split_whitespace().collect();
            if line.trim_start().starts_with("Data:") && words.get(1) == Some(&"y") {
                in_data = true;
                continue;
            }
            if in_data {
                if !words.is_empty() {
                    rows.push((words[1].parse::<f64>().unwrap(), words[0].parse::<f64>().unwrap()));
                }
            } else if words
                .first()
                .is_some_and(|w| w.starts_with('b') && w[1..].parse::<usize>().is_ok())
            {
                let numbers: Vec<_> = words[1..]
                    .iter()
                    .filter(|w| **w != "=")
                    .map(|w| w.parse::<f64>().unwrap())
                    .collect();
                params.push(numbers[2]);
            }
        }
        let function = CustomFitFunction::new("ENSO", "b1+b2*cos(2*pi*x/12)+b3*sin(2*pi*x/12)+b5*cos(2*pi*x/b4)+b6*sin(2*pi*x/b4)+b8*cos(2*pi*x/b7)+b9*sin(2*pi*x/b7)", (1..=9).map(|i| format!("b{i}")).collect()).unwrap();
        let model = FitFunction::Custom(function);
        let residual: Vec<_> = rows.iter().map(|&(x, y)| y - model.eval(x, &params)).collect();
        let scale = params.iter().fold(0.0_f64, |v, p| v.max(p.abs()));
        for factor in [2.0, 4.0, 8.0, 0.0] {
            let mut j = vec![vec![0.0; params.len()]; rows.len()];
            for k in 0..params.len() {
                let h = if factor == 0.0 {
                    f64::EPSILON.cbrt() * params[k].abs().max(1e-3 * scale).max(1e-8)
                } else {
                    factor * 1e-6 * params[k].abs().max(1e-6)
                };
                let mut up = params.clone();
                let mut down = params.clone();
                up[k] += h;
                down[k] -= h;
                for (i, &(x, _)) in rows.iter().enumerate() {
                    j[i][k] = (model.eval(x, &up) - model.eval(x, &down)) / (2.0 * h);
                }
            }
            let step = crate::linalg::lstsq(&j, &residual).unwrap().coef[7] / params[7];
            println!(
                "{}: b8 relative step {step:.12e}",
                if factor == 0.0 {
                    "new".into()
                } else {
                    format!("{factor}h")
                }
            );
            if factor == 0.0 {
                assert!(step.abs() <= 1e-8);
            }
        }
    }

    #[test]
    fn adaptive_jacobians_match_hahn_rational_derivatives() {
        let p = [
            1.0776351733,
            -0.12269296921,
            0.0040863750610,
            -0.0000014262662514,
            -0.0057609940901,
            0.00024053735503,
            -0.00000012314450199,
        ];
        let xs = [24.41, 100.0, 200.0, 350.0, 500.0, 600.0, 749.27, 851.37];
        let function = CustomFitFunction::new(
            "Hahn",
            "(b1+b2*x+b3*x^2+b4*x^3)/(1+b5*x+b6*x^2+b7*x^3)",
            (1..=7).map(|i| format!("b{i}")).collect(),
        )
        .unwrap();
        let model = FitFunction::Custom(function);
        let ys: Vec<_> = xs.iter().map(|&x| model.eval(x, &p)).collect();
        let session = FitSession::new(model, &xs, &ys, Some(&p), &[], &[]).unwrap();
        let derivatives = jacobian(&session.model, &xs, &p, &session.free);
        let refined = session.refinement_jacobian(&p);
        for (i, &x) in xs.iter().enumerate() {
            let d = 1.0 + p[4] * x + p[5] * x * x + p[6] * x * x * x;
            let f = session.model.eval(x, &p);
            let exact = [
                1.0 / d,
                x / d,
                x * x / d,
                x * x * x / d,
                -f * x / d,
                -f * x * x / d,
                -f * x * x * x / d,
            ];
            for k in 0..7 {
                for value in [derivatives[i][k], refined[i][k]] {
                    assert!(
                        (value - exact[k]).abs() <= 3e-6 * exact[k].abs().max(1e-12),
                        "x={x} parameter={k}: {value} vs {}",
                        exact[k]
                    );
                }
            }
        }
    }

    #[test]
    fn fit_warning_covers_high_relative_error_and_negative_adjusted_r2() {
        let line = || CustomFitFunction::new("line", "a+b*x", vec!["a".into(), "b".into()]).unwrap();
        let x = [-2.0, -1.0, 0.0, 1.0, 2.0];
        let y = [1.4, 0.9, 0.8, 0.9, 1.4];
        let fit = fit_custom(line(), &x, &y, &[1.08, 0.0], &[], &[]).unwrap();
        assert!(fit.converged);
        assert!(fit.se[1] > 10.0 * fit.params[1].abs());
        assert_eq!(fit.warnings.len(), 1);
        assert!(fit.warnings[0].contains("与 0 无显著差异"));
        assert!(!fit.warnings[0].contains("局部极小"));
        let x = [0.0, 1.0, 2.0, 3.0, 4.0, 5.0];
        let y = [-1.0, 0.0, 1.0, 0.0, -1.0, 1.0];
        let limits = [ParamLimits::fixed(10.0), ParamLimits::default()];
        let fit = fit_custom(line(), &x, &y, &[10.0, -2.0], &limits, &[]).unwrap();
        assert!(fit.converged);
        assert!(fit.adj_r2 < 0.0);
        assert!(fit.warnings.iter().any(|warning| warning.contains("局部极小")));
    }

    #[test]
    fn differently_scaled_line_parameters_do_not_warn_about_local_minima() {
        let function = CustomFitFunction::new("line", "a+b*x", vec!["a".into(), "b".into()]).unwrap();
        let x = [1000.0, 2000.0, 3000.0, 4000.0, 5000.0];
        let y = [2.51, 4.48, 6.515, 8.49, 10.505];
        let fit = fit_custom(function, &x, &y, &[0.5, 0.002], &[], &[]).unwrap();
        assert!(fit.converged);
        assert!(!fit.warnings.iter().any(|warning| warning.contains("局部极小")));
    }

    #[test]
    fn covariance_condition_respects_fixed_and_identifiable_directions() {
        let design = || vec![vec![1.0, 0.0], vec![0.0, 1.0], vec![1.0, 1.0]];
        assert!(!covariance_ill_conditioned(&[vec![1.0, 0.0], vec![0.0, 1e-10]], design));
        // 参数量级不同不代表病态。
        assert!(!covariance_ill_conditioned(&[vec![1.0, 0.0], vec![0.0, 1e-13]], design));
        assert!(covariance_ill_conditioned(
            &[vec![1.0, 1.0 - 1e-14], vec![1.0 - 1e-14, 1.0]],
            design
        ));
        assert!(!covariance_ill_conditioned(&[vec![1e10, 0.0], vec![0.0, 1e-10]], design));
        assert!(!covariance_ill_conditioned(&[vec![0.0, 0.0], vec![0.0, 0.0]], design));
        assert!(covariance_ill_conditioned(&[vec![0.0, 0.0], vec![0.0, 0.0]], || vec![
            vec![1.0, 1.0],
            vec![2.0, 2.0]
        ]));
    }

    #[test]
    fn unidentifiable_parameters_are_not_reported_as_converged() {
        let function = CustomFitFunction::new("sum", "a+b", vec!["a".into(), "b".into()]).unwrap();
        let result = fit_custom(
            function,
            &[0.0, 1.0, 2.0, 3.0],
            &[3.0, 3.0, 3.0, 3.0],
            &[1.0, 1.0],
            &[],
            &[],
        )
        .unwrap();
        assert!(!result.converged);
        assert_eq!(result.status, FitStatus::Failed);
    }

    #[test]
    fn constrained_damping_exhaustion_is_not_convergence() {
        let function =
            FitFunction::Custom(CustomFitFunction::new("line", "a + b*x", vec!["a".into(), "b".into()]).unwrap());
        let constraints = parse_constraints_for_function(&function, "a >= 0").unwrap();
        let mut session = FitSession::new(
            function,
            &[0.0, 1.0, 2.0, 3.0],
            &[2.0, 4.0, 6.0, 8.0],
            Some(&[1.0, 0.1]),
            &[],
            &constraints,
        )
        .unwrap();
        session.lambda = 1e16;
        session.optimize();
        let result = session.finish().unwrap();
        assert!(!result.converged);
        assert_eq!(result.status, FitStatus::Failed);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn precision_status_roundtrips_and_legacy_report_defaults() {
        let x = [0.0, 1.0, 2.0, 3.0, 4.0];
        let y: Vec<f64> = x.iter().map(|x| Model::Gauss.eval(*x, &[0.1, 1.0, 1.2, 2.0])).collect();
        let mut result = fit(Model::Gauss, &x, &y, Some(&[0.1, 1.0, 1.2, 2.0]), &[]).unwrap();
        result.status = FitStatus::PrecisionLimit;
        let mut value = serde_json::to_value(&result).unwrap();
        let restored: NlFit = serde_json::from_value(value.clone()).unwrap();
        assert_eq!(restored.status, FitStatus::PrecisionLimit);
        value.as_object_mut().unwrap().remove("status");
        let legacy: NlFit = serde_json::from_value(value).unwrap();
        assert_eq!(legacy.status, FitStatus::Converged);
    }

    #[test]
    fn rational_derivatives_near_poles_and_exhausted_damping_are_reported() {
        let p = [2., 1., -1. / (1. + 1e-6)];
        let j = jacobian(&FitFunction::Builtin(Model::Rational1), &[1.], &p, &[0, 1, 2]);
        assert!(j.iter().flatten().all(|v| v.is_finite()));
        let (xs, ys) = data(Model::Rational12, 0., 8., &[2., 1.5, 0.2, 0.08], 0.01);
        let mut session = FitSession::new(
            FitFunction::Builtin(Model::Rational12),
            &xs,
            &ys,
            Some(&[1., 1., 0., 0.]),
            &[],
            &[],
        )
        .unwrap();
        session.lambda = 1e16;
        session.optimize();
        let result = session.finish().unwrap();
        assert!(!result.converged);
        assert_eq!(result.iterations, 1);
        assert!(result.rss.is_finite());
    }

    #[test]
    fn rational_models_reuse_fixed_parameters_bounds_and_linear_constraints() {
        for (model, truth) in [
            (Model::Rational12, vec![2., 1.5, 0.2, 0.08]),
            (Model::Rational22, vec![1.2, -0.7, 0.9, 0.3, 0.15]),
            (Model::Rational33, vec![1.2, -0.7, 0.9, 0.12, 0.3, 0.15, 0.04]),
        ] {
            let (xs, ys) = data(model, 0., 5., &truth, 0.0);
            let mut limits = vec![ParamLimits::default(); truth.len()];
            limits[0] = ParamLimits::fixed(truth[0]);
            limits.last_mut().unwrap().lower = Some(0.);
            limits.last_mut().unwrap().upper = Some(0.5);
            let constraints = parse_constraints(model, "b1 >= 0").unwrap();
            let result = fit_constrained(model, &xs, &ys, None, &limits, &constraints).unwrap();
            assert!(
                result.converged,
                "model={} rss={} iterations={}",
                model.name(),
                result.rss,
                result.iterations
            );
            assert_eq!(result.params[0], truth[0]);
            assert!(result.rss < 1e-12);
            assert!(result.params.last().is_some_and(|p| (0.0..=0.5).contains(p)));
            assert!(result.constraints.iter().any(|s| s == "b1 >= 0"));
        }
    }

    #[test]
    fn rational33_matches_nist_thurber_certified_values() {
        // NIST/ITL StRD Thurber, 37 observations, cubic/cubic rational model.
        // https://www.itl.nist.gov/div898/strd/nls/data/LINKS/DATA/Thurber.dat
        let points: &[(f64, f64)] = &[
            (-3.067, 80.574),
            (-2.981, 84.248),
            (-2.921, 87.264),
            (-2.912, 87.195),
            (-2.84, 89.076),
            (-2.797, 89.608),
            (-2.702, 89.868),
            (-2.699, 90.101),
            (-2.633, 92.405),
            (-2.481, 95.854),
            (-2.363, 100.696),
            (-2.322, 101.06),
            (-1.501, 401.672),
            (-1.46, 390.724),
            (-1.274, 567.534),
            (-1.212, 635.316),
            (-1.1, 733.054),
            (-1.046, 759.087),
            (-0.915, 894.206),
            (-0.714, 990.785),
            (-0.566, 1090.109),
            (-0.545, 1080.914),
            (-0.4, 1122.643),
            (-0.309, 1178.351),
            (-0.109, 1260.531),
            (-0.103, 1273.514),
            (0.01, 1288.339),
            (0.119, 1327.543),
            (0.377, 1353.863),
            (0.79, 1414.509),
            (0.963, 1425.208),
            (1.006, 1421.384),
            (1.115, 1442.962),
            (1.572, 1464.35),
            (1.841, 1468.705),
            (2.047, 1447.894),
            (2.2, 1457.628),
        ];
        let (xs, ys): (Vec<_>, Vec<_>) = points.iter().copied().unzip();
        let expected = [
            1288.1396800,
            1491.0792535,
            583.23836877,
            75.416644291,
            0.96629502864,
            0.39797285797,
            0.049727297349,
        ];
        let se = [
            4.6647963344,
            39.571156086,
            28.698696102,
            5.5675370270,
            0.031333340687,
            0.014984928198,
            0.0065842344623,
        ];
        for initial in [
            None,
            Some(vec![1000., 1000., 400., 40., 0.7, 0.3, 0.03]),
            Some(vec![1300., 1500., 500., 75., 1., 0.4, 0.05]),
        ] {
            let result = fit(Model::Rational33, &xs, &ys, initial.as_deref(), &[]).unwrap();
            assert!(result.converged);
            close(result.rss, 5642.7082397, 1e-8);
            for (actual, want) in result.params.iter().zip(expected) {
                close(*actual, want, 1e-4);
            }
            for (actual, want) in result.se.iter().zip(se) {
                close(*actual, want, 2e-4);
            }
        }
    }

    #[test]
    fn custom_formula_uses_validated_scalar_variables_and_the_shared_lm_solver() {
        let function = CustomFitFunction::new(
            "Decay",
            "y = a * exp(-b*x) + c",
            vec!["a".into(), "b".into(), "c".into()],
        )
        .unwrap();
        let truth = [2.2, 0.4, 0.3];
        let xs: Vec<f64> = (0..60).map(|i| i as f64 / 10.0).collect();
        let ys: Vec<f64> = xs.iter().map(|x| function.eval(*x, &truth)).collect();
        let fit = fit_custom(
            function.clone(),
            &xs,
            &ys,
            &[2.0, 0.3, 0.2],
            &[ParamLimits::default(); 3],
            &[],
        )
        .unwrap();
        assert!(fit.converged);
        assert_eq!(fit.model.name(), "Decay");
        assert_eq!(fit.model.params(), vec!["a", "b", "c"]);
        assert!((fit.eval(1.5) - function.eval(1.5, &truth)).abs() < 1e-7);
        assert!(CustomFitFunction::new("bad", "a*x + missing", vec!["a".into()]).is_err());
        assert!(CustomFitFunction::new("bad", "mean(x)", vec!["a".into()]).is_err());
    }

    #[test]
    fn nonlinear_bands_and_t_tests_match_delta_method() {
        // 参考：scipy curve_fit 的 pcov 与解析梯度的 Δ 法，t 分布 95%（SciPy 1.17.1）
        let (xs, ys) = data(Model::ExpDec1, 0.0, 8.0, &[0.3, 5.0, 1.7], 0.05);
        let f = fit(Model::ExpDec1, &xs, &ys, None, &[]).unwrap();
        let at = [0.0, 1.0, 2.5, 8.0];
        let conf = [
            0.036930037359520045,
            0.016145292629534957,
            0.014471897661722667,
            0.015446935846694449,
        ];
        let pred = [
            0.06780030389556413,
            0.05910773234568062,
            0.058672731067223025,
            0.05892080596875095,
        ];
        for i in 0..4 {
            let (c, p) = f.band(at[i], 0.95);
            close(c, conf[i], 1e-5);
            close(p, pred[i], 1e-6);
        }
        let t = [34.01123351318792, 286.66425189541997, 123.98923371068953];
        let pv = [4.606615447919501e-30, 5.510180515572003e-65, 3.602688045999483e-51];
        for (k, (tv, p)) in f.t_tests().into_iter().enumerate() {
            close(tv, t[k], 1e-4);
            close(p, pv[k], 1e-2);
        }
        // 固定参数没有 t 检验
        let held = fit(Model::ExpDec1, &xs, &ys, None, &[None, None, Some(1.7)]).unwrap();
        assert!(held.t_tests()[2].0.is_nan());
    }

    #[test]
    fn linear_constraints_match_scipy_substitution() {
        // 参考值：SciPy 1.17.1 curve_fit 对代入约束后的模型（等式消元；生效的不等式按等式代入）
        let (xs, ys) = data(Model::ExpDec2, 0.0, 10.0, &[0.5, 4.0, 0.8, 2.0, 4.0], 0.02);
        let cases: [(&str, &[f64], &[f64], f64); 3] = [
            (
                "A1 + A2 = 6.5",
                &[
                    -0.008869506971709452,
                    4.599838952542216,
                    0.9059992722022512,
                    1.9001610474577841,
                    9.03039219176094,
                ],
                &[
                    0.020674970458570655,
                    0.031508185530406595,
                    0.013377031324750327,
                    0.031508185530406595,
                    0.333684070913209,
                ],
                0.020842198807211432,
            ),
            (
                "t2 >= t1 + 3.5",
                &[
                    0.46666306034400296,
                    4.094307934602258,
                    0.8113114174798253,
                    1.9476663075416343,
                    4.311311417479825,
                ],
                &[
                    0.004800016887384544,
                    0.016949920857287096,
                    0.0063353188402990365,
                    0.020885120441479183,
                    0.0063353188402990365,
                ],
                0.0048867135867287775,
            ),
            (
                "t2 - t1 >= 1",
                &[
                    0.4937960013006856,
                    4.01551121614994,
                    0.7974110921387975,
                    2.002545767816202,
                    4.046387326181638,
                ],
                &[
                    0.019735941188441294,
                    0.06279795003108511,
                    0.012293474333771807,
                    0.048313578591315055,
                    0.1865327482325991,
                ],
                0.00466318079317071,
            ),
        ];
        for (text, params, se, rss) in cases {
            let constraints = parse_constraints(Model::ExpDec2, text).unwrap();
            let f = fit_constrained(Model::ExpDec2, &xs, &ys, None, &[], &constraints).unwrap();
            assert!(f.converged, "{text}");
            for (g, w) in f.params.iter().zip(params) {
                close(*g, *w, 1e-5);
            }
            for (g, w) in f.se.iter().zip(se) {
                close(*g, *w, 1e-3);
            }
            close(f.rss, rss, 1e-7);
            assert_eq!(f.constraints, [text]);
        }
    }

    #[test]
    fn constraint_parsing_and_errors() {
        let c = parse_constraints(Model::Gauss, "2*w - xc/2 <= 3 + y0；A = 1").unwrap();
        assert_eq!(c.len(), 2);
        assert_eq!(
            (c[0].coef.clone(), c[0].rel, c[0].rhs),
            (vec![-1.0, -0.5, 2.0, 0.0], Rel::Le, 3.0)
        );
        assert_eq!(
            (c[1].coef.clone(), c[1].rel, c[1].rhs),
            (vec![0.0, 0.0, 0.0, 1.0], Rel::Eq, 1.0)
        );
        for (text, msg) in [
            ("w * xc = 1", "线性"),
            ("q = 1", "不是参数名"),
            ("w 1", "缺少"),
            ("3 = 3", "没有参数"),
        ] {
            let e = parse_constraints(Model::Gauss, text).unwrap_err();
            assert!(e.contains(msg), "{text}: {e}");
        }
        // 互相矛盾的约束
        let (xs, ys) = data(Model::Gauss, 0.0, 10.0, &[1.0, 4.2, 1.5, 6.0], 0.05);
        let bad = parse_constraints(Model::Gauss, "w >= 3; w <= 2").unwrap();
        assert!(fit_constrained(Model::Gauss, &xs, &ys, None, &[], &bad)
            .unwrap_err()
            .contains("矛盾"));
    }

    #[test]
    fn bounded_parameters_match_scipy_trf() {
        // 参考值：scipy curve_fit(bounds=…, method="trf")（SciPy 1.17.1）
        let lim = |lower: Option<f64>, upper: Option<f64>| ParamLimits {
            fixed: None,
            lower,
            upper,
        };
        let free = ParamLimits::default();
        type Bounded<'a> = (
            Model,
            f64,
            f64,
            &'a [f64],
            &'a [ParamLimits],
            &'a [f64],
            &'a [f64],
            f64,
            &'a [bool],
        );
        let cases: [Bounded; 3] = [
            (
                Model::Gauss,
                0.0,
                10.0,
                &[1.0, 4.2, 1.5, 6.0],
                &[lim(Some(1.02), None), free, lim(None, Some(1.3)), free],
                &[
                    1.0591747191658383,
                    4.200262070736825,
                    1.2999999999999998,
                    5.43529480038129,
                ],
                &[
                    0.021308377368655526,
                    0.014258322592542791,
                    0.0308399632343038,
                    0.12682856689340216,
                ],
                0.4565305024320042,
                &[false, false, true, false],
            ),
            (
                Model::ExpDec1,
                0.0,
                8.0,
                &[0.3, 5.0, 1.7],
                &[free, free, lim(None, Some(1.5))],
                &[0.40247361429382816, 5.096088116350162, 1.4999999999999998],
                &[0.021649870222356992, 0.049727736755135005, 0.031631938817463304],
                0.22313286158621473,
                &[false, false, true],
            ),
            (
                // 界不生效时与无约束结果相同
                Model::ExpDec1,
                0.0,
                8.0,
                &[0.3, 5.0, 1.7],
                &[
                    lim(Some(0.0), Some(1.0)),
                    lim(Some(0.0), Some(10.0)),
                    lim(Some(0.5), Some(5.0)),
                ],
                &[0.3028905161777613, 5.011675158968168, 1.6976676595997724],
                &[0.008905602142936982, 0.017482735121955775, 0.013692056871435564],
                0.029978254433445914,
                &[false, false, false],
            ),
        ];
        for (model, from, to, truth, limits, params, se, rss, at_bound) in cases {
            let amp = 0.05;
            let (xs, ys) = data(model, from, to, truth, amp);
            let f = fit_with(model, &xs, &ys, None, limits).unwrap();
            assert!(f.converged);
            for (g, w) in f.params.iter().zip(params) {
                close(*g, *w, 1e-6);
            }
            for (g, w) in f.se.iter().zip(se) {
                close(*g, *w, 1e-4);
            }
            close(f.rss, rss, 1e-8);
            assert_eq!(f.at_bound, at_bound);
        }
        assert!(fit_with(Model::Gauss, &[0.0; 10], &[0.0; 10], None, &[lim(Some(2.0), Some(1.0))]).is_err());
    }

    #[test]
    fn fixed_parameters_match_scipy_with_the_parameter_held() {
        // 参考值：curve_fit 对去掉固定参数后的函数拟合（SciPy 1.17.1）
        let (xs, ys) = data(Model::Gauss, 0.0, 10.0, &[1.0, 4.2, 1.5, 6.0], 0.05);
        let f = fit(Model::Gauss, &xs, &ys, None, &[Some(1.0), None, None, None]).unwrap();
        assert_eq!(f.fixed, [true, false, false, false]);
        assert_eq!((f.params[0], f.se[0]), (1.0, 0.0));
        let want = [4.200180465074677, 1.4982909982647807, 6.0112335219205635];
        let se = [0.004078720331065471, 0.008157440762355455, 0.02834340061698604];
        for k in 0..3 {
            close(f.params[k + 1], want[k], 1e-6);
            close(f.se[k + 1], se[k], 1e-4);
        }
        let (xs, ys) = data(Model::ExpDec1, 0.0, 8.0, &[0.3, 5.0, 1.7], 0.05);
        let f = fit(Model::ExpDec1, &xs, &ys, None, &[Some(0.3)]).unwrap();
        close(f.params[1], 5.01261358915415, 1e-6);
        close(f.params[2], 1.7010939158349043, 1e-6);
        close(f.se[2], 0.008672307488815372, 1e-4);
        assert!(fit(Model::Allometric1, &xs, &ys, None, &[Some(1.0), Some(1.0)]).is_err());
        assert!(fit(Model::ExpDec1, &xs, &ys, None, &[Some(f64::NAN)]).is_err());
    }

    #[test]
    fn too_few_points_and_bad_initial_values_are_reported() {
        assert!(fit(Model::Gauss, &[1.0, 2.0, 3.0], &[1.0, 2.0, 1.0], None, &[])
            .unwrap_err()
            .contains("至少需要 5 个"));
        let (xs, ys) = data(Model::ExpDec1, 0.0, 8.0, &[0.3, 5.0, 1.7], 0.05);
        assert!(fit(Model::ExpDec1, &xs, &ys, Some(&[0.0, 1.0, 0.0]), &[]).is_err());
        // 无序输入先按 X 排序
        let mut rev = xs.clone();
        rev.reverse();
        let mut rys = ys.clone();
        rys.reverse();
        let a = fit(Model::ExpDec1, &xs, &ys, None, &[]).unwrap();
        let b = fit(Model::ExpDec1, &rev, &rys, None, &[]).unwrap();
        close(a.params[2], b.params[2], 1e-9);
        close(
            a.eval(2.0),
            a.params[0] + a.params[1] * (-2.0 / a.params[2]).exp(),
            1e-12,
        );
    }
}

#[cfg(test)]
mod function_library_tests {
    use super::*;
    use crate::{history::History, Project};

    #[test]
    fn custom_fit_library_edits_are_atomic_stable_and_undoable() {
        let mut project = Project::default();
        let definition =
            CustomFitDefinition::from_text("Decay", "a*exp(-b*x)+c", "a,b,c", "2,0.3,0", "b >= 0").unwrap();
        let before = project.clone();
        let mut history = History::default();
        history.begin(&project, "save function");
        let id = project.save_fit_function(None, definition.clone()).unwrap();
        history.changed();
        history.commit(&project);
        let saved = project.clone();
        project = history.undo(&project).unwrap();
        assert_eq!(project, before);
        project = history.redo(&project).unwrap();
        assert_eq!(project, saved);
        let original = project.fit_functions[0].clone();
        let mut edited = definition.clone();
        edited.name = "Renamed".into();
        edited.initial = vec![2.2, 0.4, 0.3];
        history.begin(&project, "edit function");
        assert_eq!(project.save_fit_function(Some(&original), edited.clone()).unwrap(), id);
        history.changed();
        history.commit(&project);
        let changed = project.clone();
        assert_eq!(project.fit_functions[0].id, id);
        assert_eq!(project.fit_functions[0].definition, edited);
        assert!(project.save_fit_function(Some(&original), definition.clone()).is_err());
        assert_eq!(
            project, changed,
            "stale edits preserve the current definition and allocator"
        );
        assert!(project.save_fit_function(None, edited.clone()).is_err());
        assert_eq!(project, changed);
        let current = project.fit_functions[0].clone();
        project.save_fit_function(Some(&current), edited).unwrap();
        assert_eq!(project, changed, "saving an unchanged function is a no-op");
        project = history.undo(&project).unwrap();
        assert_eq!(project, saved);
        project = history.redo(&project).unwrap();
        assert_eq!(project, changed);
        for bad in [
            CustomFitDefinition {
                formula: "a*x+missing".into(),
                ..definition.clone()
            },
            CustomFitDefinition {
                initial: vec![f64::NAN, 0.3, 0.0],
                ..definition.clone()
            },
            CustomFitDefinition {
                initial: vec![1.0],
                ..definition.clone()
            },
            CustomFitDefinition {
                constraints: "unknown = 0".into(),
                ..definition.clone()
            },
        ] {
            assert!(project.save_fit_function(None, bad).is_err());
            assert_eq!(project, changed);
        }
    }
}

#[cfg(feature = "serde")]
fn report_number<'de, D: serde::Deserializer<'de>>(d: D) -> Result<f64, D::Error> {
    use serde::Deserialize;
    Ok(Option::<f64>::deserialize(d)?.unwrap_or(f64::NAN))
}
#[cfg(feature = "serde")]
fn report_errors<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Vec<f64>, D::Error> {
    use serde::Deserialize;
    Ok(Vec::<Option<f64>>::deserialize(d)?
        .into_iter()
        .map(|x| x.unwrap_or(f64::NAN))
        .collect())
}
#[cfg(feature = "serde")]
fn report_covariance<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Vec<Vec<f64>>, D::Error> {
    use serde::Deserialize;
    Ok(Vec::<Vec<Option<f64>>>::deserialize(d)?
        .into_iter()
        .map(|row| row.into_iter().map(|x| x.unwrap_or(f64::NAN)).collect())
        .collect())
}

impl PartialEq for NlFit {
    fn eq(&self, other: &Self) -> bool {
        let number = |a: f64, b: f64| a == b || (a.is_nan() && b.is_nan());
        let values = |a: &[f64], b: &[f64]| a.len() == b.len() && a.iter().zip(b).all(|(&a, &b)| number(a, b));
        self.model == other.model
            && values(&self.params, &other.params)
            && values(&self.se, &other.se)
            && values(
                &[
                    self.rss,
                    self.r2,
                    self.adj_r2,
                    self.reduced_chi2,
                    self.x_min,
                    self.x_max,
                ],
                &[
                    other.rss,
                    other.r2,
                    other.adj_r2,
                    other.reduced_chi2,
                    other.x_min,
                    other.x_max,
                ],
            )
            && self.n == other.n
            && self.iterations == other.iterations
            && self.converged == other.converged
            && self.status == other.status
            && self.warnings == other.warnings
            && self.fixed == other.fixed
            && self.at_bound == other.at_bound
            && self.dof == other.dof
            && self.cov.len() == other.cov.len()
            && self.cov.iter().zip(&other.cov).all(|(a, b)| values(a, b))
            && self.constraints == other.constraints
    }
}
