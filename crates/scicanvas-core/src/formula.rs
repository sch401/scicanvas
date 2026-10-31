//! 列公式：设置列值用的表达式。
//!
//! 语法：数字、列（短名 `A`、`B` 或 `col(A)`）、行号 `i`（从 1 开始）、常数 `pi`、`e`，
//! 运算符 `+ - * / ^`（`^` 右结合，优先级高于负号：`-2^2 = -4`）、括号，
//! 比较 `< > <= >= == !=`（结果为 1 或 0，优先级最低），以及 [`FUNCTIONS`] 中的函数。
//! 列统计（`mean(A)`）与按行序列（`cumsum(A)`）先用 [`Expr::prepare`] 对整列算好，再逐行 [`Expr::eval`]。

use std::sync::Arc;

#[derive(Clone, Debug, PartialEq)]
pub enum Expr {
    Num(f64),
    Col(String),
    Row,
    Neg(Box<Expr>),
    /// 运算符：`+ - * / ^`，比较用 `<` `>` `≤` `≥` `=` `≠`。
    Bin(char, Box<Expr>, Box<Expr>),
    Call(String, Vec<Expr>),
    /// `prepare` 算好的逐行结果。
    Vector(Arc<Vec<f64>>),
}

/// 函数种类：逐行标量、整列统计（结果为常数）、按行序列（需要整列数据）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Scalar,
    Aggregate,
    Series,
}

/// 函数目录的一项。
#[derive(Clone, Copy, Debug)]
pub struct Func {
    pub name: &'static str,
    /// 参数个数的下限、上限。
    pub args: (usize, usize),
    pub kind: Kind,
    pub category: &'static str,
    /// 调用写法与说明。
    pub usage: &'static str,
}

const fn f(name: &'static str, lo: usize, hi: usize, kind: Kind, category: &'static str, usage: &'static str) -> Func {
    Func {
        name,
        args: (lo, hi),
        kind,
        category,
        usage,
    }
}

use Kind::{Aggregate as Agg, Scalar as Sc, Series as Ser};

const MATH: &str = "数学";
const TRIG: &str = "三角与双曲";
const SPECIAL: &str = "特殊函数";
const LOGIC: &str = "逻辑与比较";
const DIST: &str = "概率分布";
const STAT: &str = "列统计";
const SEQ: &str = "按行序列";
const DATE: &str = "日期时间";

/// 目录中的类别顺序。
pub const CATEGORIES: [&str; 8] = [MATH, TRIG, SPECIAL, LOGIC, DIST, STAT, SEQ, DATE];

/// 全部函数。`min`、`max` 一个参数时是列统计，两个参数时逐行比较。
pub const FUNCTIONS: &[Func] = &[
    f("abs", 1, 1, Sc, MATH, "abs(x) 绝对值"),
    f("sign", 1, 1, Sc, MATH, "sign(x) 符号：−1、0、1"),
    f("sqrt", 1, 1, Sc, MATH, "sqrt(x) 平方根"),
    f("cbrt", 1, 1, Sc, MATH, "cbrt(x) 立方根"),
    f("sqr", 1, 1, Sc, MATH, "sqr(x) 平方"),
    f("exp", 1, 1, Sc, MATH, "exp(x) eˣ"),
    f("exp2", 1, 1, Sc, MATH, "exp2(x) 2ˣ"),
    f("exp10", 1, 1, Sc, MATH, "exp10(x) 10ˣ"),
    f("ln", 1, 1, Sc, MATH, "ln(x) 自然对数"),
    f("log", 1, 1, Sc, MATH, "log(x) 常用对数（同 log10）"),
    f("log10", 1, 1, Sc, MATH, "log10(x) 常用对数"),
    f("log2", 1, 1, Sc, MATH, "log2(x) 以 2 为底的对数"),
    f("logb", 2, 2, Sc, MATH, "logb(x, b) 以 b 为底的对数"),
    f("pow", 2, 2, Sc, MATH, "pow(x, y) xʸ"),
    f("floor", 1, 1, Sc, MATH, "floor(x) 向下取整"),
    f("ceil", 1, 1, Sc, MATH, "ceil(x) 向上取整"),
    f("trunc", 1, 1, Sc, MATH, "trunc(x) 截去小数部分"),
    f("frac", 1, 1, Sc, MATH, "frac(x) 小数部分 x − trunc(x)"),
    f("round", 1, 2, Sc, MATH, "round(x[, n]) 四舍五入到 n 位小数"),
    f("recip", 1, 1, Sc, MATH, "recip(x) 倒数 1/x"),
    f("mod", 2, 2, Sc, MATH, "mod(x, y) 取模（结果与 y 同号的非负余数）"),
    f("rem", 2, 2, Sc, MATH, "rem(x, y) 余数（与 x 同号）"),
    f("hypot", 2, 2, Sc, MATH, "hypot(x, y) √(x² + y²)"),
    f("fact", 1, 1, Sc, MATH, "fact(n) 阶乘 n!"),
    f("ncr", 2, 2, Sc, MATH, "ncr(n, k) 组合数"),
    f("npr", 2, 2, Sc, MATH, "npr(n, k) 排列数"),
    f("gcd", 2, 2, Sc, MATH, "gcd(a, b) 最大公约数"),
    f("lcm", 2, 2, Sc, MATH, "lcm(a, b) 最小公倍数"),
    f("clamp", 3, 3, Sc, MATH, "clamp(x, lo, hi) 限制到 [lo, hi]"),
    f("isnan", 1, 1, Sc, MATH, "isnan(x) 缺失或非数时为 1"),
    f("isinf", 1, 1, Sc, MATH, "isinf(x) 无穷大时为 1"),
    f("sin", 1, 1, Sc, TRIG, "sin(x) 正弦（弧度）"),
    f("cos", 1, 1, Sc, TRIG, "cos(x) 余弦"),
    f("tan", 1, 1, Sc, TRIG, "tan(x) 正切"),
    f("sec", 1, 1, Sc, TRIG, "sec(x) 正割"),
    f("csc", 1, 1, Sc, TRIG, "csc(x) 余割"),
    f("cot", 1, 1, Sc, TRIG, "cot(x) 余切"),
    f("asin", 1, 1, Sc, TRIG, "asin(x) 反正弦"),
    f("acos", 1, 1, Sc, TRIG, "acos(x) 反余弦"),
    f("atan", 1, 1, Sc, TRIG, "atan(x) 反正切"),
    f("acot", 1, 1, Sc, TRIG, "acot(x) 反余切"),
    f("atan2", 2, 2, Sc, TRIG, "atan2(y, x) 四象限反正切"),
    f("sinh", 1, 1, Sc, TRIG, "sinh(x) 双曲正弦"),
    f("cosh", 1, 1, Sc, TRIG, "cosh(x) 双曲余弦"),
    f("tanh", 1, 1, Sc, TRIG, "tanh(x) 双曲正切"),
    f("sech", 1, 1, Sc, TRIG, "sech(x) 双曲正割"),
    f("csch", 1, 1, Sc, TRIG, "csch(x) 双曲余割"),
    f("coth", 1, 1, Sc, TRIG, "coth(x) 双曲余切"),
    f("asinh", 1, 1, Sc, TRIG, "asinh(x) 反双曲正弦"),
    f("acosh", 1, 1, Sc, TRIG, "acosh(x) 反双曲余弦"),
    f("atanh", 1, 1, Sc, TRIG, "atanh(x) 反双曲正切"),
    f("deg", 1, 1, Sc, TRIG, "deg(x) 弧度转角度"),
    f("rad", 1, 1, Sc, TRIG, "rad(x) 角度转弧度"),
    f("sinc", 1, 1, Sc, TRIG, "sinc(x) sin(πx)/(πx)"),
    f("gamma", 1, 1, Sc, SPECIAL, "gamma(x) Γ 函数"),
    f("lngamma", 1, 1, Sc, SPECIAL, "lngamma(x) ln|Γ(x)|"),
    f("beta", 2, 2, Sc, SPECIAL, "beta(a, b) B 函数"),
    f("lnbeta", 2, 2, Sc, SPECIAL, "lnbeta(a, b) ln B(a, b)"),
    f("erf", 1, 1, Sc, SPECIAL, "erf(x) 误差函数"),
    f("erfc", 1, 1, Sc, SPECIAL, "erfc(x) 互补误差函数"),
    f("erfinv", 1, 1, Sc, SPECIAL, "erfinv(y) 误差函数的反函数"),
    f("betainc", 3, 3, Sc, SPECIAL, "betainc(x, a, b) 正则化不完全 B 函数"),
    f(
        "gammainc",
        2,
        2,
        Sc,
        SPECIAL,
        "gammainc(x, a) 正则化下不完全 Γ 函数 P(a, x)",
    ),
    f(
        "gammaincc",
        2,
        2,
        Sc,
        SPECIAL,
        "gammaincc(x, a) 正则化上不完全 Γ 函数 Q(a, x)",
    ),
    f("step", 1, 1, Sc, SPECIAL, "step(x) 阶跃：x ≥ 0 为 1，否则 0"),
    f("logistic", 1, 1, Sc, SPECIAL, "logistic(x) 1/(1 + e⁻ˣ)"),
    f("if", 3, 3, Sc, LOGIC, "if(条件, 真值, 假值)"),
    f("and", 2, 2, Sc, LOGIC, "and(a, b) 两者都非 0 时为 1"),
    f("or", 2, 2, Sc, LOGIC, "or(a, b) 任一非 0 时为 1"),
    f("not", 1, 1, Sc, LOGIC, "not(a) a 为 0 时为 1"),
    f("xor", 2, 2, Sc, LOGIC, "xor(a, b) 恰有一个非 0 时为 1"),
    f("between", 3, 3, Sc, LOGIC, "between(x, a, b) a ≤ x ≤ b 时为 1"),
    f("min", 1, 2, Sc, LOGIC, "min(x, y) 较小者；min(A) 为整列最小值"),
    f("max", 1, 2, Sc, LOGIC, "max(x, y) 较大者；max(A) 为整列最大值"),
    f("normpdf", 1, 3, Sc, DIST, "normpdf(x[, μ, σ]) 正态概率密度"),
    f("normcdf", 1, 3, Sc, DIST, "normcdf(x[, μ, σ]) 正态分布函数"),
    f("norminv", 1, 3, Sc, DIST, "norminv(p[, μ, σ]) 正态分位数"),
    f("tpdf", 2, 2, Sc, DIST, "tpdf(t, ν) t 分布密度"),
    f("tcdf", 2, 2, Sc, DIST, "tcdf(t, ν) t 分布函数"),
    f("tinv", 2, 2, Sc, DIST, "tinv(p, ν) t 分位数"),
    f("fpdf", 3, 3, Sc, DIST, "fpdf(x, d1, d2) F 分布密度"),
    f("fcdf", 3, 3, Sc, DIST, "fcdf(x, d1, d2) F 分布函数"),
    f("finv", 3, 3, Sc, DIST, "finv(p, d1, d2) F 分位数"),
    f("chi2pdf", 2, 2, Sc, DIST, "chi2pdf(x, ν) χ² 分布密度"),
    f("chi2cdf", 2, 2, Sc, DIST, "chi2cdf(x, ν) χ² 分布函数"),
    f("chi2inv", 2, 2, Sc, DIST, "chi2inv(p, ν) χ² 分位数"),
    f("binopdf", 3, 3, Sc, DIST, "binopdf(k, n, p) 二项分布概率"),
    f("binocdf", 3, 3, Sc, DIST, "binocdf(k, n, p) 二项分布函数"),
    f("poisspdf", 2, 2, Sc, DIST, "poisspdf(k, λ) 泊松分布概率"),
    f("poisscdf", 2, 2, Sc, DIST, "poisscdf(k, λ) 泊松分布函数"),
    f("exppdf", 2, 2, Sc, DIST, "exppdf(x, μ) 指数分布密度（均值 μ）"),
    f("expcdf", 2, 2, Sc, DIST, "expcdf(x, μ) 指数分布函数"),
    f("sum", 1, 1, Agg, STAT, "sum(A) 整列求和"),
    f("mean", 1, 1, Agg, STAT, "mean(A) 平均值"),
    f("median", 1, 1, Agg, STAT, "median(A) 中位数"),
    f("std", 1, 1, Agg, STAT, "std(A) 样本标准差"),
    f("var", 1, 1, Agg, STAT, "var(A) 样本方差"),
    f("sem", 1, 1, Agg, STAT, "sem(A) 标准误"),
    f("count", 1, 1, Agg, STAT, "count(A) 有效值个数"),
    f("range", 1, 1, Agg, STAT, "range(A) 最大值 − 最小值"),
    f("rms", 1, 1, Agg, STAT, "rms(A) 均方根"),
    f(
        "percentile",
        2,
        2,
        Agg,
        STAT,
        "percentile(A, p) 第 p 百分位数（线性插值）",
    ),
    f(
        "skewness",
        1,
        1,
        Agg,
        STAT,
        "skewness(A) 偏度（样本校正，同 Excel SKEW）",
    ),
    f(
        "kurtosis",
        1,
        1,
        Agg,
        STAT,
        "kurtosis(A) 超额峰度（样本校正，同 Excel KURT）",
    ),
    f("cumsum", 1, 1, Ser, SEQ, "cumsum(A) 累积和"),
    f("cumprod", 1, 1, Ser, SEQ, "cumprod(A) 累积积"),
    f("diff", 1, 1, Ser, SEQ, "diff(A) 与上一行之差"),
    f("rank", 1, 1, Ser, SEQ, "rank(A) 秩（并列取平均，从 1 起）"),
    f("zscore", 1, 1, Ser, SEQ, "zscore(A) (A − 均值)/标准差"),
    f("lag", 1, 2, Ser, SEQ, "lag(A[, k]) 上移：第 i 行取第 i − k 行"),
    f("lead", 1, 2, Ser, SEQ, "lead(A[, k]) 下移：第 i 行取第 i + k 行"),
    f("year", 1, 1, Sc, DATE, "year(d) 年"),
    f("month", 1, 1, Sc, DATE, "month(d) 月"),
    f("day", 1, 1, Sc, DATE, "day(d) 日"),
    f("hour", 1, 1, Sc, DATE, "hour(d) 时"),
    f("minute", 1, 1, Sc, DATE, "minute(d) 分"),
    f("second", 1, 1, Sc, DATE, "second(d) 秒（含小数）"),
    f("weekday", 1, 1, Sc, DATE, "weekday(d) 星期：1 为周一，7 为周日"),
    f("dayofyear", 1, 1, Sc, DATE, "dayofyear(d) 一年中的第几天"),
    f("date", 3, 3, Sc, DATE, "date(年, 月, 日) 生成日期（UTC 毫秒）"),
    f("time", 3, 3, Sc, DATE, "time(时, 分, 秒) 一天内的时长（毫秒）"),
];

/// 按名称查函数。
pub fn function(name: &str) -> Option<&'static Func> {
    FUNCTIONS.iter().find(|f| f.name == name)
}

#[derive(Clone, Debug, PartialEq)]
enum Tok {
    Num(f64),
    Ident(String),
    Op(char),
}

fn lex(src: &str) -> Result<Vec<Tok>, String> {
    let cs: Vec<char> = src.chars().collect();
    let mut out = vec![];
    let mut i = 0;
    while i < cs.len() {
        let c = cs[i];
        if c.is_whitespace() {
            i += 1;
        } else if c.is_ascii_digit() || c == '.' {
            let start = i;
            while i < cs.len() && (cs[i].is_ascii_digit() || cs[i] == '.') {
                i += 1;
            }
            // 科学计数：1e-3、2.5E6
            if i < cs.len() && (cs[i] == 'e' || cs[i] == 'E') {
                let mut j = i + 1;
                if j < cs.len() && (cs[j] == '+' || cs[j] == '-') {
                    j += 1;
                }
                if j < cs.len() && cs[j].is_ascii_digit() {
                    i = j;
                    while i < cs.len() && cs[i].is_ascii_digit() {
                        i += 1;
                    }
                }
            }
            let text: String = cs[start..i].iter().collect();
            out.push(Tok::Num(text.parse().map_err(|_| format!("无法识别的数字“{text}”"))?));
        } else if c.is_alphabetic() || c == '_' {
            let start = i;
            while i < cs.len() && (cs[i].is_alphanumeric() || cs[i] == '_') {
                i += 1;
            }
            out.push(Tok::Ident(cs[start..i].iter().collect()));
        } else if "<>=!".contains(c) {
            // 比较：<  >  <=  >=  ==  !=（单个 = 也当作相等）
            let next = cs.get(i + 1) == Some(&'=');
            let op = match (c, next) {
                ('<', true) => '≤',
                ('>', true) => '≥',
                ('<', false) => '<',
                ('>', false) => '>',
                ('!', true) => '≠',
                ('=', _) => '=',
                _ => return Err(format!("无法识别的字符“{c}”")),
            };
            i += if next { 2 } else { 1 };
            out.push(Tok::Op(op));
        } else if "+-*/^(),".contains(c) {
            out.push(Tok::Op(c));
            i += 1;
        } else if c == '×' || c == '·' {
            out.push(Tok::Op('*'));
            i += 1;
        } else if c == '÷' {
            out.push(Tok::Op('/'));
            i += 1;
        } else if c == '−' {
            out.push(Tok::Op('-'));
            i += 1;
        } else if "≤≥≠".contains(c) {
            out.push(Tok::Op(c));
            i += 1;
        } else {
            return Err(format!("无法识别的字符“{c}”"));
        }
    }
    Ok(out)
}

struct Parser {
    toks: Vec<Tok>,
    pos: usize,
}

impl Parser {
    fn peek(&self) -> Option<&Tok> {
        self.toks.get(self.pos)
    }

    fn eat(&mut self, op: char) -> bool {
        if self.peek() == Some(&Tok::Op(op)) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn expect(&mut self, op: char) -> Result<(), String> {
        if self.eat(op) {
            Ok(())
        } else {
            Err(format!("缺少“{op}”"))
        }
    }

    // compare := sum (比较运算符 sum)?
    fn compare(&mut self) -> Result<Expr, String> {
        let e = self.sum()?;
        for op in ['<', '>', '≤', '≥', '=', '≠'] {
            if self.eat(op) {
                return Ok(Expr::Bin(op, Box::new(e), Box::new(self.sum()?)));
            }
        }
        Ok(e)
    }

    // sum := term (('+' | '-') term)*
    fn sum(&mut self) -> Result<Expr, String> {
        let mut e = self.term()?;
        loop {
            let op = if self.eat('+') {
                '+'
            } else if self.eat('-') {
                '-'
            } else {
                return Ok(e);
            };
            e = Expr::Bin(op, Box::new(e), Box::new(self.term()?));
        }
    }

    // term := unary (('*' | '/') unary)*
    fn term(&mut self) -> Result<Expr, String> {
        let mut e = self.unary()?;
        loop {
            let op = if self.eat('*') {
                '*'
            } else if self.eat('/') {
                '/'
            } else {
                return Ok(e);
            };
            e = Expr::Bin(op, Box::new(e), Box::new(self.unary()?));
        }
    }

    // unary := ('-' | '+') unary | power
    fn unary(&mut self) -> Result<Expr, String> {
        if self.eat('-') {
            return Ok(Expr::Neg(Box::new(self.unary()?)));
        }
        if self.eat('+') {
            return self.unary();
        }
        self.power()
    }

    // power := atom ('^' unary)?   右结合
    fn power(&mut self) -> Result<Expr, String> {
        let base = self.atom()?;
        if self.eat('^') {
            return Ok(Expr::Bin('^', Box::new(base), Box::new(self.unary()?)));
        }
        Ok(base)
    }

    fn atom(&mut self) -> Result<Expr, String> {
        match self.toks.get(self.pos).cloned() {
            Some(Tok::Num(v)) => {
                self.pos += 1;
                Ok(Expr::Num(v))
            }
            Some(Tok::Op('(')) => {
                self.pos += 1;
                let e = self.compare()?;
                self.expect(')')?;
                Ok(e)
            }
            Some(Tok::Ident(name)) => {
                self.pos += 1;
                if self.eat('(') {
                    if name == "col" {
                        let key = match self.toks.get(self.pos).cloned() {
                            Some(Tok::Ident(k)) => k,
                            _ => return Err("col( ) 里应是列的短名，例如 col(B)".into()),
                        };
                        self.pos += 1;
                        self.expect(')')?;
                        return Ok(Expr::Col(key));
                    }
                    let mut args = vec![];
                    if !self.eat(')') {
                        loop {
                            args.push(self.compare()?);
                            if self.eat(')') {
                                break;
                            }
                            self.expect(',')?;
                        }
                    }
                    let func = function(&name).ok_or_else(|| format!("没有函数“{name}”"))?;
                    let (lo, hi) = func.args;
                    if args.len() < lo || args.len() > hi {
                        return Err(if lo == hi {
                            format!("{name} 需要 {lo} 个参数")
                        } else {
                            format!("{name} 需要 {lo}–{hi} 个参数")
                        });
                    }
                    return Ok(Expr::Call(name, args));
                }
                Ok(match name.as_str() {
                    "i" => Expr::Row,
                    "pi" | "PI" | "π" => Expr::Num(std::f64::consts::PI),
                    "e" => Expr::Num(std::f64::consts::E),
                    _ => Expr::Col(name),
                })
            }
            Some(Tok::Op(c)) => Err(format!("“{c}”的位置不对")),
            None => Err("公式不完整".into()),
        }
    }
}

/// 解析公式。
pub fn parse(src: &str) -> Result<Expr, String> {
    let toks = lex(src)?;
    if toks.is_empty() {
        return Err("公式为空".into());
    }
    let mut p = Parser { toks, pos: 0 };
    let e = p.compare()?;
    match p.peek() {
        None => Ok(e),
        Some(Tok::Op(c)) => Err(format!("“{c}”的位置不对")),
        Some(Tok::Num(_)) | Some(Tok::Ident(_)) => Err("两个值之间缺少运算符".into()),
    }
}

/// 列统计与按行序列是否作用在这个调用上（`min`/`max` 一个参数时为列统计）。
fn whole_column(name: &str, args: usize) -> Option<Kind> {
    let func = function(name)?;
    match (name, args) {
        ("min" | "max", 1) => Some(Kind::Aggregate),
        _ => (func.kind != Kind::Scalar).then_some(func.kind),
    }
}

impl Expr {
    /// 公式引用的列（按出现顺序，不重复）。
    pub fn columns(&self) -> Vec<String> {
        fn walk(e: &Expr, out: &mut Vec<String>) {
            match e {
                Expr::Col(k) => {
                    if !out.contains(k) {
                        out.push(k.clone());
                    }
                }
                Expr::Neg(a) => walk(a, out),
                Expr::Bin(_, a, b) => {
                    walk(a, out);
                    walk(b, out);
                }
                Expr::Call(_, args) => args.iter().for_each(|a| walk(a, out)),
                Expr::Num(_) | Expr::Row | Expr::Vector(_) => {}
            }
        }
        let mut out = vec![];
        walk(self, &mut out);
        out
    }

    /// Variables allowed by a scalar fitting function; worksheet rows and vector operations are excluded.
    pub fn scalar_variables(&self) -> Result<Vec<String>, String> {
        fn walk(expr: &Expr, vars: &mut Vec<String>) -> Result<(), String> {
            match expr {
                Expr::Num(_) => {}
                Expr::Col(name) => {
                    if !vars.contains(name) {
                        vars.push(name.clone());
                    }
                }
                Expr::Row | Expr::Vector(_) => return Err("拟合函数不能使用行号或向量结果".into()),
                Expr::Neg(value) => walk(value, vars)?,
                Expr::Bin(_, left, right) => {
                    walk(left, vars)?;
                    walk(right, vars)?;
                }
                Expr::Call(name, args) => {
                    if whole_column(name, args.len()).is_some() {
                        return Err(format!("拟合函数不支持整列函数 {name}"));
                    }
                    for arg in args {
                        walk(arg, vars)?;
                    }
                }
            }
            Ok(())
        }

        let mut vars = Vec::new();
        walk(self, &mut vars)?;
        Ok(vars)
    }

    /// 把列统计和按行序列在 `rows` 行上先算好（替换成常数或逐行结果），之后再逐行求值。
    /// `col(key, row)` 给出列在某行的值（缺失为 NaN）。
    pub fn prepare(&self, rows: usize, col: &dyn Fn(&str, usize) -> f64) -> Expr {
        match self {
            Expr::Neg(a) => Expr::Neg(Box::new(a.prepare(rows, col))),
            Expr::Bin(op, a, b) => Expr::Bin(*op, Box::new(a.prepare(rows, col)), Box::new(b.prepare(rows, col))),
            Expr::Call(name, args) => {
                let args: Vec<Expr> = args.iter().map(|a| a.prepare(rows, col)).collect();
                match whole_column(name, args.len()) {
                    Some(kind) => {
                        let values: Vec<f64> = (0..rows).map(|i| args[0].eval(i, col)).collect();
                        let extra = args.get(1).map(|a| a.eval(0, col));
                        match kind {
                            Kind::Aggregate => Expr::Num(aggregate(name, &values, extra)),
                            _ => Expr::Vector(Arc::new(series(name, &values, extra))),
                        }
                    }
                    None => Expr::Call(name.clone(), args),
                }
            }
            other => other.clone(),
        }
    }

    /// 在第 `row` 行（从 0 开始）求值；`col(key, row)` 给出列在某行的值（缺失为 NaN）。
    /// 列统计和按行序列需先 [`Expr::prepare`]，否则为 NaN。
    pub fn eval(&self, row: usize, col: &dyn Fn(&str, usize) -> f64) -> f64 {
        match self {
            Expr::Num(v) => *v,
            Expr::Col(k) => col(k, row),
            Expr::Row => (row + 1) as f64,
            Expr::Vector(v) => v.get(row).copied().unwrap_or(f64::NAN),
            Expr::Neg(a) => -a.eval(row, col),
            Expr::Bin(op, a, b) => {
                let (x, y) = (a.eval(row, col), b.eval(row, col));
                let truth = |t: bool| {
                    if x.is_nan() || y.is_nan() {
                        f64::NAN
                    } else {
                        f64::from(u8::from(t))
                    }
                };
                match op {
                    '+' => x + y,
                    '-' => x - y,
                    '*' => x * y,
                    '/' => x / y,
                    '<' => truth(x < y),
                    '>' => truth(x > y),
                    '≤' => truth(x <= y),
                    '≥' => truth(x >= y),
                    '=' => truth(x == y),
                    '≠' => truth(x != y),
                    _ => x.powf(y),
                }
            }
            Expr::Call(name, args) => {
                if whole_column(name, args.len()).is_some() {
                    return f64::NAN;
                }
                let a: Vec<f64> = args.iter().map(|e| e.eval(row, col)).collect();
                scalar(name, &a)
            }
        }
    }
}

fn truthy(v: f64) -> bool {
    v != 0.0 && !v.is_nan()
}

fn bool_num(b: bool) -> f64 {
    f64::from(u8::from(b))
}

fn gamma(x: f64) -> f64 {
    if x <= 0.0 && x == x.floor() {
        return f64::NAN;
    }
    if x < 0.5 {
        // 反射公式
        return std::f64::consts::PI / ((std::f64::consts::PI * x).sin() * gamma(1.0 - x));
    }
    crate::stats::ln_gamma(x).exp()
}

fn lnbeta(a: f64, b: f64) -> f64 {
    crate::stats::ln_gamma(a) + crate::stats::ln_gamma(b) - crate::stats::ln_gamma(a + b)
}

/// 单调分布函数的反函数：先找包住 p 的区间，再二分到双精度极限。
fn invert(cdf: impl Fn(f64) -> f64, p: f64, lower: f64) -> f64 {
    if !(0.0..=1.0).contains(&p) || p.is_nan() {
        return f64::NAN;
    }
    let (mut lo, mut hi) = (lower, lower.max(0.0) + 1.0);
    while cdf(hi) < p {
        hi *= 2.0;
        if hi > 1e300 {
            return f64::INFINITY;
        }
    }
    for _ in 0..300 {
        let mid = 0.5 * (lo + hi);
        if mid == lo || mid == hi {
            break;
        }
        if cdf(mid) < p {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    0.5 * (lo + hi)
}

fn f_cdf(x: f64, d1: f64, d2: f64) -> f64 {
    if x <= 0.0 {
        0.0
    } else {
        crate::stats::beta_inc(d1 / 2.0, d2 / 2.0, d1 * x / (d1 * x + d2))
    }
}

fn gcd(a: f64, b: f64) -> f64 {
    let (mut a, mut b) = (a.abs().round(), b.abs().round());
    while b > 0.0 {
        (a, b) = (b, a % b);
    }
    a
}

fn datetime(ms: f64) -> Option<chrono::DateTime<chrono::Utc>> {
    if ms.is_finite() {
        chrono::DateTime::from_timestamp_millis(ms.floor() as i64)
    } else {
        None
    }
}

use crate::stats::{beta_inc, erfc, gamma_p, gamma_q, ln_gamma, norm_cdf, norm_ppf, t_cdf, t_ppf};
use chrono::{Datelike, Timelike};
use std::f64::consts::PI;

struct ScalarInput<'a> {
    name: &'a str,
    args: &'a [f64],
    x: f64,
    y: f64,
    z: f64,
    mu: f64,
    sigma: f64,
}

/// 逐行标量函数；按数学类别分派，分支计算式与原实现保持一致。
fn scalar(name: &str, a: &[f64]) -> f64 {
    let input = ScalarInput {
        name,
        args: a,
        x: a[0],
        y: a.get(1).copied().unwrap_or(f64::NAN),
        z: a.get(2).copied().unwrap_or(f64::NAN),
        mu: a.get(1).copied().unwrap_or(0.0),
        sigma: a.get(2).copied().unwrap_or(1.0),
    };
    scalar_arithmetic(&input)
        .or_else(|| scalar_trigonometric(&input))
        .or_else(|| scalar_special(&input))
        .or_else(|| scalar_logic(&input))
        .or_else(|| scalar_probability(&input))
        .or_else(|| scalar_datetime(&input))
        .unwrap_or(f64::NAN)
}

fn scalar_arithmetic(input: &ScalarInput<'_>) -> Option<f64> {
    match input.name {
        "abs" => Some(input.x.abs()),
        "sign" => Some({
            if input.x == 0.0 {
                0.0
            } else {
                input.x.signum()
            }
        }),
        "sqrt" => Some(input.x.sqrt()),
        "cbrt" => Some(input.x.cbrt()),
        "sqr" => Some(input.x * input.x),
        "exp" => Some(input.x.exp()),
        "exp2" => Some(input.x.exp2()),
        "exp10" => Some(10f64.powf(input.x)),
        "ln" => Some(input.x.ln()),
        "log" | "log10" => Some(input.x.log10()),
        "log2" => Some(input.x.log2()),
        "logb" => Some(input.x.ln() / input.y.ln()),
        "pow" => Some(input.x.powf(input.y)),
        "floor" => Some(input.x.floor()),
        "ceil" => Some(input.x.ceil()),
        "trunc" => Some(input.x.trunc()),
        "frac" => Some(input.x.fract()),
        "round" => Some({
            if input.args.len() == 1 {
                input.x.round()
            } else {
                let k = 10f64.powi(input.y.round() as i32);
                (input.x * k).round() / k
            }
        }),
        "recip" => Some(1.0 / input.x),
        "mod" => Some(input.x.rem_euclid(input.y)),
        "rem" => Some(input.x % input.y),
        "hypot" => Some(input.x.hypot(input.y)),
        "fact" => Some({
            if input.x >= 0.0 && input.x == input.x.floor() {
                gamma(input.x + 1.0).round()
            } else {
                f64::NAN
            }
        }),
        "ncr" => Some({
            if input.x >= 0.0
                && input.y >= 0.0
                && input.y <= input.x
                && input.x == input.x.floor()
                && input.y == input.y.floor()
            {
                (ln_gamma(input.x + 1.0) - ln_gamma(input.y + 1.0) - ln_gamma(input.x - input.y + 1.0))
                    .exp()
                    .round()
            } else {
                f64::NAN
            }
        }),
        "npr" => Some({
            if input.x >= 0.0
                && input.y >= 0.0
                && input.y <= input.x
                && input.x == input.x.floor()
                && input.y == input.y.floor()
            {
                (ln_gamma(input.x + 1.0) - ln_gamma(input.x - input.y + 1.0))
                    .exp()
                    .round()
            } else {
                f64::NAN
            }
        }),
        "gcd" => Some(gcd(input.x, input.y)),
        "lcm" => Some({
            let g = gcd(input.x, input.y);
            if g == 0.0 {
                0.0
            } else {
                (input.x.abs().round() / g * input.y.abs().round()).abs()
            }
        }),
        "clamp" => Some(input.x.max(input.y).min(input.z)),
        "isnan" => Some(bool_num(input.x.is_nan())),
        "isinf" => Some(bool_num(input.x.is_infinite())),
        _ => None,
    }
}

fn scalar_trigonometric(input: &ScalarInput<'_>) -> Option<f64> {
    match input.name {
        "sin" => Some(input.x.sin()),
        "cos" => Some(input.x.cos()),
        "tan" => Some(input.x.tan()),
        "sec" => Some(1.0 / input.x.cos()),
        "csc" => Some(1.0 / input.x.sin()),
        "cot" => Some(1.0 / input.x.tan()),
        "asin" => Some(input.x.asin()),
        "acos" => Some(input.x.acos()),
        "atan" => Some(input.x.atan()),
        "acot" => Some((1.0 / input.x).atan()),
        "atan2" => Some(input.x.atan2(input.y)),
        "sinh" => Some(input.x.sinh()),
        "cosh" => Some(input.x.cosh()),
        "tanh" => Some(input.x.tanh()),
        "sech" => Some(1.0 / input.x.cosh()),
        "csch" => Some(1.0 / input.x.sinh()),
        "coth" => Some(1.0 / input.x.tanh()),
        "asinh" => Some(input.x.asinh()),
        "acosh" => Some(input.x.acosh()),
        "atanh" => Some(input.x.atanh()),
        "deg" => Some(input.x.to_degrees()),
        "rad" => Some(input.x.to_radians()),
        "sinc" => Some({
            if input.x == 0.0 {
                1.0
            } else {
                (PI * input.x).sin() / (PI * input.x)
            }
        }),
        _ => None,
    }
}

fn scalar_special(input: &ScalarInput<'_>) -> Option<f64> {
    match input.name {
        "gamma" => Some(gamma(input.x)),
        "lngamma" => Some({
            if input.x > 0.0 {
                ln_gamma(input.x)
            } else {
                gamma(input.x).abs().ln()
            }
        }),
        "beta" => Some(lnbeta(input.x, input.y).exp()),
        "lnbeta" => Some(lnbeta(input.x, input.y)),
        "erf" => Some(1.0 - erfc(input.x)),
        "erfc" => Some(erfc(input.x)),
        "erfinv" => Some({
            if input.x <= -1.0 || input.x >= 1.0 {
                if input.x == 1.0 {
                    f64::INFINITY
                } else if input.x == -1.0 {
                    f64::NEG_INFINITY
                } else {
                    f64::NAN
                }
            } else {
                norm_ppf((input.x + 1.0) / 2.0) / std::f64::consts::SQRT_2
            }
        }),
        "betainc" => Some(beta_inc(input.y, input.z, input.x)),
        "gammainc" => Some(gamma_p(input.y, input.x)),
        "gammaincc" => Some(gamma_q(input.y, input.x)),
        _ => None,
    }
}

fn scalar_logic(input: &ScalarInput<'_>) -> Option<f64> {
    match input.name {
        "step" => Some(bool_num(input.x >= 0.0)),
        "logistic" => Some(1.0 / (1.0 + (-input.x).exp())),
        "if" => Some({
            if truthy(input.x) {
                input.y
            } else {
                input.z
            }
        }),
        "and" => Some(bool_num(truthy(input.x) && truthy(input.y))),
        "or" => Some(bool_num(truthy(input.x) || truthy(input.y))),
        "not" => Some(bool_num(!truthy(input.x))),
        "xor" => Some(bool_num(truthy(input.x) != truthy(input.y))),
        "between" => Some(bool_num(input.x >= input.y && input.x <= input.z)),
        "min" => Some(input.x.min(input.y)),
        "max" => Some(input.x.max(input.y)),
        _ => None,
    }
}

fn scalar_probability(input: &ScalarInput<'_>) -> Option<f64> {
    match input.name {
        "normpdf" => {
            Some((-0.5 * ((input.x - input.mu) / input.sigma).powi(2)).exp() / (input.sigma * (2.0 * PI).sqrt()))
        }
        "normcdf" => Some(norm_cdf((input.x - input.mu) / input.sigma)),
        "norminv" => Some(input.mu + input.sigma * norm_ppf(input.x)),
        "tpdf" => Some(
            (ln_gamma((input.y + 1.0) / 2.0)
                - ln_gamma(input.y / 2.0)
                - 0.5 * (input.y * PI).ln()
                - (input.y + 1.0) / 2.0 * (1.0 + input.x * input.x / input.y).ln())
            .exp(),
        ),
        "tcdf" => Some(t_cdf(input.x, input.y)),
        "tinv" => Some({
            if input.x > 0.0 && input.x < 1.0 {
                t_ppf(input.x, input.y)
            } else {
                f64::NAN
            }
        }),
        "fpdf" => Some({
            if input.x <= 0.0 {
                0.0
            } else {
                ((input.y / 2.0) * (input.y / input.z).ln() + (input.y / 2.0 - 1.0) * input.x.ln()
                    - ((input.y + input.z) / 2.0) * (1.0 + input.y * input.x / input.z).ln()
                    - lnbeta(input.y / 2.0, input.z / 2.0))
                .exp()
            }
        }),
        "fcdf" => Some(f_cdf(input.x, input.y, input.z)),
        "finv" => Some(invert(|v| f_cdf(v, input.y, input.z), input.x, 0.0)),
        "chi2pdf" => Some({
            if input.x <= 0.0 {
                0.0
            } else {
                ((input.y / 2.0 - 1.0) * input.x.ln()
                    - input.x / 2.0
                    - (input.y / 2.0) * 2f64.ln()
                    - ln_gamma(input.y / 2.0))
                .exp()
            }
        }),
        "chi2cdf" => Some(gamma_p(input.y / 2.0, input.x.max(0.0) / 2.0)),
        "chi2inv" => Some(invert(|v| gamma_p(input.y / 2.0, v / 2.0), input.x, 0.0)),
        "binopdf" => Some({
            if input.x < 0.0 || input.x > input.y || input.x != input.x.floor() {
                0.0
            } else {
                (ln_gamma(input.y + 1.0) - ln_gamma(input.x + 1.0) - ln_gamma(input.y - input.x + 1.0)
                    + input.x * input.z.ln()
                    + (input.y - input.x) * (1.0 - input.z).ln())
                .exp()
            }
        }),
        "binocdf" => Some({
            let k = input.x.floor();
            if k < 0.0 {
                0.0
            } else if k >= input.y {
                1.0
            } else {
                // P(X ≤ k) = I_{1−p}(n − k, k + 1)
                beta_inc(input.y - k, k + 1.0, 1.0 - input.z)
            }
        }),
        "poisspdf" => Some({
            if input.x < 0.0 || input.x != input.x.floor() {
                0.0
            } else {
                (input.x * input.y.ln() - input.y - ln_gamma(input.x + 1.0)).exp()
            }
        }),
        "poisscdf" => Some({
            let k = input.x.floor();
            if k < 0.0 {
                0.0
            } else {
                gamma_q(k + 1.0, input.y)
            }
        }),
        "exppdf" => Some({
            if input.x < 0.0 {
                0.0
            } else {
                (-input.x / input.y).exp() / input.y
            }
        }),
        "expcdf" => Some({
            if input.x < 0.0 {
                0.0
            } else {
                -(-input.x / input.y).exp_m1()
            }
        }),
        _ => None,
    }
}

fn scalar_datetime(input: &ScalarInput<'_>) -> Option<f64> {
    match input.name {
        "year" => Some(datetime(input.x).map_or(f64::NAN, |t| f64::from(t.year()))),
        "month" => Some(datetime(input.x).map_or(f64::NAN, |t| f64::from(t.month()))),
        "day" => Some(datetime(input.x).map_or(f64::NAN, |t| f64::from(t.day()))),
        "hour" => Some(datetime(input.x).map_or(f64::NAN, |t| f64::from(t.hour()))),
        "minute" => Some(datetime(input.x).map_or(f64::NAN, |t| f64::from(t.minute()))),
        "second" => Some(datetime(input.x).map_or(f64::NAN, |t| {
            f64::from(t.second()) + input.x.rem_euclid(1000.0) / 1000.0
        })),
        "weekday" => Some(datetime(input.x).map_or(f64::NAN, |t| f64::from(t.weekday().number_from_monday()))),
        "dayofyear" => Some(datetime(input.x).map_or(f64::NAN, |t| f64::from(t.ordinal()))),
        "date" => Some({
            let ok = [input.x, input.y, input.z]
                .iter()
                .all(|v| v.is_finite() && *v == v.floor());
            ok.then(|| chrono::NaiveDate::from_ymd_opt(input.x as i32, input.y as u32, input.z as u32))
                .flatten()
                .map_or(f64::NAN, |d| {
                    d.and_hms_opt(0, 0, 0).unwrap().and_utc().timestamp_millis() as f64
                })
        }),
        "time" => Some(((input.x * 60.0 + input.y) * 60.0 + input.z) * 1000.0),
        _ => None,
    }
}

/// 整列统计（跳过缺失值）。`extra` 是第二个参数（percentile 的 p）。
fn aggregate(name: &str, values: &[f64], extra: Option<f64>) -> f64 {
    let mut v: Vec<f64> = values.iter().copied().filter(|x| x.is_finite()).collect();
    let n = v.len() as f64;
    if v.is_empty() {
        return if name == "count" { 0.0 } else { f64::NAN };
    }
    let mean = v.iter().sum::<f64>() / n;
    let m = |k: i32| v.iter().map(|x| (x - mean).powi(k)).sum::<f64>() / n;
    let var = v.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (n - 1.0);
    let quantile = |v: &mut Vec<f64>, q: f64| {
        v.sort_by(f64::total_cmp);
        let pos = q.clamp(0.0, 1.0) * (v.len() - 1) as f64;
        let (lo, hi) = (pos.floor() as usize, pos.ceil() as usize);
        v[lo] + (v[hi] - v[lo]) * (pos - lo as f64)
    };
    match name {
        "sum" => v.iter().sum(),
        "mean" => mean,
        "median" => quantile(&mut v, 0.5),
        "std" => var.sqrt(),
        "var" => var,
        "sem" => (var / n).sqrt(),
        "count" => n,
        "range" | "min" | "max" => {
            let lo = v.iter().copied().fold(f64::INFINITY, f64::min);
            let hi = v.iter().copied().fold(f64::NEG_INFINITY, f64::max);
            match name {
                "min" => lo,
                "max" => hi,
                _ => hi - lo,
            }
        }
        "rms" => (v.iter().map(|x| x * x).sum::<f64>() / n).sqrt(),
        "percentile" => {
            let p = extra.unwrap_or(f64::NAN);
            if (0.0..=100.0).contains(&p) {
                quantile(&mut v, p / 100.0)
            } else {
                f64::NAN
            }
        }
        // 样本校正偏度 G1 = √(n(n−1))/(n−2) · m3/m2^1.5
        "skewness" => (n * (n - 1.0)).sqrt() / (n - 2.0) * m(3) / m(2).powf(1.5),
        // 样本校正超额峰度 G2
        "kurtosis" => {
            let g2 = m(4) / (m(2) * m(2)) - 3.0;
            (n - 1.0) / ((n - 2.0) * (n - 3.0)) * ((n + 1.0) * g2 + 6.0)
        }
        _ => f64::NAN,
    }
}

/// 按行序列（结果与输入逐行对齐，缺失值保持缺失）。
fn series(name: &str, values: &[f64], extra: Option<f64>) -> Vec<f64> {
    let n = values.len();
    match name {
        "cumsum" | "cumprod" => {
            let mut acc = if name == "cumsum" { 0.0 } else { 1.0 };
            values
                .iter()
                .map(|v| {
                    if v.is_finite() {
                        acc = if name == "cumsum" { acc + v } else { acc * v };
                        acc
                    } else {
                        f64::NAN
                    }
                })
                .collect()
        }
        "diff" => (0..n)
            .map(|i| if i == 0 { f64::NAN } else { values[i] - values[i - 1] })
            .collect(),
        "rank" => {
            let mut idx: Vec<usize> = (0..n).filter(|&i| values[i].is_finite()).collect();
            idx.sort_by(|&a, &b| values[a].total_cmp(&values[b]));
            let mut out = vec![f64::NAN; n];
            let mut k = 0;
            while k < idx.len() {
                let mut j = k;
                while j + 1 < idx.len() && values[idx[j + 1]] == values[idx[k]] {
                    j += 1;
                }
                let r = (k + j) as f64 / 2.0 + 1.0;
                for &i in &idx[k..=j] {
                    out[i] = r;
                }
                k = j + 1;
            }
            out
        }
        "zscore" => {
            let mean = aggregate("mean", values, None);
            let sd = aggregate("std", values, None);
            values.iter().map(|v| (v - mean) / sd).collect()
        }
        "lag" | "lead" => {
            let k = extra.unwrap_or(1.0).round() as i64;
            let k = if name == "lag" { -k } else { k };
            (0..n as i64)
                .map(|i| {
                    let j = i + k;
                    if j >= 0 && j < n as i64 {
                        values[j as usize]
                    } else {
                        f64::NAN
                    }
                })
                .collect()
        }
        _ => vec![f64::NAN; n],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn eval(src: &str, a: f64, b: f64) -> f64 {
        parse(src).unwrap().eval(4, &|k, _| match k {
            "A" => a,
            "B" => b,
            _ => f64::NAN,
        })
    }

    #[test]
    fn precedence_functions_and_columns() {
        let (a, b): (f64, f64) = (2.5, -1.25);
        assert_eq!(eval("1 + 2 * 3", a, b), 7.0);
        assert_eq!(eval("-2^2", a, b), -4.0);
        assert_eq!(eval("2^3^2", a, b), 512.0);
        assert_eq!(eval("2^-1", a, b), 0.5);
        assert_eq!(eval("(1 + 2) * 3 - 4 / 8", a, b), 8.5);
        assert_eq!(eval("A * 2 + col(B)", a, b), a * 2.0 + b);
        assert_eq!(
            eval("sin(A)^2 + ln(abs(B))/2 - 3*-A", a, b),
            a.sin().powi(2) + b.abs().ln() / 2.0 + 3.0 * a
        );
        assert_eq!(eval("exp(-A/2) * sqrt(i)", a, b), (-a / 2.0).exp() * 5f64.sqrt());
        assert_eq!(
            eval("max(A, B) + min(A, B) + pow(A, 2) + mod(-7, 3)", a, b),
            a + b + a * a + 2.0
        );
        assert_eq!(
            eval("log(1000) + log10(0.01) + 1.5e2 + 2E-1", a, b),
            3.0 - 2.0 + 150.0 + 0.2
        );
        assert_eq!(eval("2·pi × 1", a, b), 2.0 * std::f64::consts::PI);
        assert!(eval("A + C", a, b).is_nan());
        assert_eq!(parse("B*A + col(B) - i").unwrap().columns(), ["B", "A"]);
        // 比较优先级最低，结果为 1/0，缺失时为 NaN
        assert_eq!(eval("A + 1 > 3", a, b), 1.0);
        assert_eq!(eval("if(B < 0, -B, B) == 1.25", a, b), 1.0);
        assert_eq!(eval("A != 2.5", a, b), 0.0);
        assert!(eval("C <= 1", a, b).is_nan());
    }

    #[test]
    fn catalog_has_unique_names_and_at_least_120_functions() {
        let mut names: Vec<&str> = FUNCTIONS.iter().map(|f| f.name).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), FUNCTIONS.len());
        assert!(FUNCTIONS.len() >= 120, "{}", FUNCTIONS.len());
        assert!(FUNCTIONS
            .iter()
            .all(|f| CATEGORIES.contains(&f.category) && f.usage.starts_with(f.name)));
    }

    #[test]
    fn column_statistics_and_row_series() {
        let a = [3.0, f64::NAN, 1.0, 4.0, 1.0, 5.0, 9.0, 2.0, 6.0];
        let col = |k: &str, i: usize| {
            if k == "A" {
                a.get(i).copied().unwrap_or(f64::NAN)
            } else {
                f64::NAN
            }
        };
        let at = |src: &str, row: usize| parse(src).unwrap().prepare(a.len(), &col).eval(row, &col);
        assert_eq!(at("sum(A)", 0), 31.0);
        assert_eq!(at("count(A)", 3), 8.0);
        assert_eq!(at("A - mean(A)", 0), 3.0 - 31.0 / 8.0);
        assert_eq!(at("median(A)", 0), 3.5);
        assert_eq!(at("min(A) + max(A)", 0), 10.0);
        assert_eq!(at("cumsum(A)", 3), 8.0);
        assert!(at("cumsum(A)", 1).is_nan());
        assert_eq!(at("diff(A)", 3), 3.0);
        assert_eq!(at("lag(A)", 3), 1.0);
        assert_eq!(at("lead(A, 2)", 2), 1.0);
        assert_eq!(at("rank(A)", 2), 1.5);
        assert_eq!(at("rank(A)", 6), 8.0);
        // 未 prepare 的列统计为 NaN
        assert!(parse("sum(A)").unwrap().eval(0, &col).is_nan());
    }

    #[test]
    fn errors_are_readable() {
        for (src, msg) in [
            ("", "为空"),
            ("1 +", "不完整"),
            ("(1 + 2", "缺少“)”"),
            ("foo(1)", "没有函数"),
            ("sin(1, 2)", "1 个参数"),
            ("round(1, 2, 3)", "1–2 个参数"),
            ("A B", "缺少运算符"),
            ("2 $ 3", "无法识别"),
            ("col(1)", "短名"),
            ("1 )", "位置不对"),
        ] {
            let e = parse(src).unwrap_err();
            assert!(e.contains(msg), "{src}: {e}");
        }
    }

    #[test]
    #[allow(clippy::approx_constant)] // 参考值逐字取自 Python 输出
    fn scalar_functions_match_python_references() {
        // 参考值：Python math / SciPy 1.17.1 / NumPy 2.4.4
        let cases: &[(&str, f64)] = &[
            ("abs(-2.5)", 2.5),
            ("sign(-3)", -1.0),
            ("sqrt(2)", 1.4142135623730951),
            ("cbrt(-27)", -3.0),
            ("sqr(1.5)", 2.25),
            ("exp(1.3)", 3.6692966676192444),
            ("exp2(3.5)", 11.313708498984761),
            ("exp10(-1.5)", 0.03162277660168379),
            ("ln(7)", 1.9459101490553132),
            ("log(250)", 2.3979400086720375),
            ("log10(0.02)", -1.6989700043360187),
            ("log2(10)", 3.321928094887362),
            ("logb(81,3)", 4.0),
            ("pow(2,0.5)", 1.4142135623730951),
            ("floor(-2.5)", -3.0),
            ("ceil(-2.5)", -2.0),
            ("trunc(-2.7)", -2.0),
            ("frac(-2.75)", -0.75),
            ("round(2.5)", 3.0),
            ("round(3.14159, 2)", 3.14),
            ("recip(8)", 0.125),
            ("mod(-7,3)", 2.0),
            ("rem(-7,3)", -1.0),
            ("hypot(3,4)", 5.0),
            ("fact(10)", 3628800.0),
            ("ncr(10,3)", 120.0),
            ("npr(10,3)", 720.0),
            ("gcd(84,36)", 12.0),
            ("lcm(4,6)", 12.0),
            ("clamp(5,0,3)", 3.0),
            ("isnan(0/0)", 1.0),
            ("isinf(1/0)", 1.0),
            ("sin(0.7)", 0.644217687237691),
            ("cos(0.7)", 0.7648421872844885),
            ("tan(0.7)", 0.8422883804630794),
            ("sec(0.7)", 1.3074592597335937),
            ("csc(0.7)", 1.552270326957104),
            ("cot(0.7)", 1.1872418321266793),
            ("asin(0.3)", 0.3046926540153975),
            ("acos(0.3)", 1.2661036727794992),
            ("atan(2)", 1.1071487177940904),
            ("acot(2)", 0.4636476090008061),
            ("atan2(1,-1)", 2.356194490192345),
            ("sinh(1.2)", 1.5094613554121725),
            ("cosh(1.2)", 1.8106555673243747),
            ("tanh(1.2)", 0.8336546070121552),
            ("sech(1.2)", 0.5522861542782048),
            ("csch(1.2)", 0.6624879771943155),
            ("coth(1.2)", 1.1995375441923508),
            ("asinh(2)", 1.4436354751788103),
            ("acosh(2)", 1.3169578969248166),
            ("atanh(0.5)", 0.5493061443340549),
            ("deg(pi/3)", 60.0),
            ("rad(45)", 0.7853981633974483),
            ("sinc(0.3)", 0.8583936913341398),
            ("gamma(4.5)", 11.631728396567446),
            ("gamma(-1.5)", 2.3632718012073544),
            ("lngamma(30.5)", 72.9534711841694),
            ("lngamma(-2.5)", -0.05624371649767457),
            ("beta(2.5,3.5)", 0.036815538909255395),
            ("lnbeta(20,30)", -33.96882079197739),
            ("erf(0.6)", 0.6038560908479259),
            ("erfc(1.7)", 0.01620954140922544),
            ("erfinv(0.4)", 0.37080715859355795),
            ("betainc(0.3, 2, 5)", 0.5798250000000003),
            ("gammainc(1.5, 2.5)", 0.3000141641213723),
            ("gammaincc(4, 2.5)", 0.1562356275777222),
            ("step(0)", 1.0),
            ("logistic(0.8)", 0.6899744811276125),
            ("if(1>2, 5, 6)", 6.0),
            ("and(1,0)", 0.0),
            ("or(1,0)", 1.0),
            ("not(0)", 1.0),
            ("xor(1,1)", 0.0),
            ("between(2,1,3)", 1.0),
            ("normpdf(0.5)", 0.3520653267642995),
            ("normpdf(1, 2, 0.5)", 0.10798193302637613),
            ("normcdf(1.96)", 0.9750021048517795),
            ("normcdf(10, 12, 3)", 0.2524925375469229),
            ("norminv(0.975)", 1.959963984540054),
            ("norminv(0.1, 5, 2)", 2.4368968689107993),
            ("tpdf(1.2, 7)", 0.1821685741897387),
            ("tcdf(-2.1, 12)", 0.028772469367475562),
            ("tinv(0.95, 9)", 1.8331129326562368),
            ("fpdf(1.7, 4, 11)", 0.2174476413405142),
            ("fcdf(2.5, 3, 20)", 0.9111562480623107),
            ("finv(0.95, 3, 20)", 3.098391212140781),
            ("chi2pdf(3.3, 5)", 0.1530995188310899),
            ("chi2cdf(7.1, 4)", 0.8693028895732106),
            ("chi2inv(0.99, 6)", 16.811893829770927),
            ("binopdf(3, 10, 0.25)", 0.2502822875976561),
            ("binocdf(3, 10, 0.25)", 0.7758750915527344),
            ("poisspdf(4, 2.5)", 0.13360188578108528),
            ("poisscdf(4, 2.5)", 0.8911780189141513),
            ("exppdf(1.5, 2)", 0.23618327637050734),
            ("expcdf(1.5, 2)", 0.5276334472589853),
            ("year(1792333815000.0)", 2026.0),
            ("month(1792333815000.0)", 10.0),
            ("day(1792333815000.0)", 18.0),
            ("hour(1792333815000.0)", 14.0),
            ("minute(1792333815000.0)", 30.0),
            ("second(1792333815000.0)", 15.0),
            ("weekday(1792333815000.0)", 7.0),
            ("dayofyear(1792333815000.0)", 291.0),
            ("date(2026, 10, 18)", 1792281600000.0),
            ("time(1, 2, 3.5)", 3723500.0),
        ];
        let none = |_: &str, _: usize| f64::NAN;
        for (src, want) in cases {
            let got = parse(src).unwrap_or_else(|e| panic!("{src}: {e}")).eval(0, &none);
            assert!(
                (got - want).abs() <= 1e-9 * want.abs().max(1.0),
                "{src}: {got} vs {want}"
            );
        }
        // 每个标量函数都有参考值
        for func in FUNCTIONS
            .iter()
            .filter(|f| f.kind == Kind::Scalar && !["min", "max"].contains(&f.name))
        {
            assert!(
                cases.iter().any(|(src, _)| src.starts_with(&format!("{}(", func.name))),
                "{}",
                func.name
            );
        }
    }

    #[test]
    fn shape_statistics_match_scipy() {
        let a = [3.0, 1.0, 4.0, 1.0, 5.0, 9.0, 2.0, 6.0];
        let col = |_: &str, i: usize| a.get(i).copied().unwrap_or(f64::NAN);
        let at = |src: &str| parse(src).unwrap().prepare(a.len(), &col).eval(0, &col);
        for (src, want) in [
            ("skewness(A)", 0.8335031385336658),
            ("kurtosis(A)", 0.27660580453699524),
            ("percentile(A, 30)", 2.1),
            ("std(A)", 2.748376143938713),
            ("sem(A)", 0.9716977043151993),
        ] {
            let got = at(src);
            assert!((got - want).abs() < 1e-12, "{src}: {got} vs {want}");
        }
    }
}
