//! 列公式：设置列值用的表达式。
//!
//! 语法：数字、列（短名 `A`、`B` 或 `col(A)`）、行号 `i`（从 1 开始）、常数 `pi`、`e`，
//! 运算符 `+ - * / ^`（`^` 右结合，优先级高于负号：`-2^2 = -4`）与括号，
//! 函数 `sin cos tan asin acos atan sinh cosh tanh exp ln log log10 sqrt abs floor ceil round sign`
//! 与两个参数的 `pow atan2 min max mod`。

#[derive(Clone, Debug, PartialEq)]
pub enum Expr {
    Num(f64),
    Col(String),
    Row,
    Neg(Box<Expr>),
    Bin(char, Box<Expr>, Box<Expr>),
    Call(String, Vec<Expr>),
}

const FUNCS1: [&str; 19] = [
    "sin", "cos", "tan", "asin", "acos", "atan", "sinh", "cosh", "tanh", "exp", "ln", "log", "log10", "sqrt", "abs",
    "floor", "ceil", "round", "sign",
];
const FUNCS2: [&str; 5] = ["pow", "atan2", "min", "max", "mod"];

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
                let e = self.sum()?;
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
                            args.push(self.sum()?);
                            if self.eat(')') {
                                break;
                            }
                            self.expect(',')?;
                        }
                    }
                    let want = if FUNCS1.contains(&name.as_str()) {
                        1
                    } else if FUNCS2.contains(&name.as_str()) {
                        2
                    } else {
                        return Err(format!("没有函数“{name}”"));
                    };
                    if args.len() != want {
                        return Err(format!("{name} 需要 {want} 个参数"));
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
    let e = p.sum()?;
    match p.peek() {
        None => Ok(e),
        Some(Tok::Op(c)) => Err(format!("“{c}”的位置不对")),
        Some(Tok::Num(_)) | Some(Tok::Ident(_)) => Err("两个值之间缺少运算符".into()),
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
                Expr::Num(_) | Expr::Row => {}
            }
        }
        let mut out = vec![];
        walk(self, &mut out);
        out
    }

    /// 在第 `row` 行（从 0 开始）求值；`col` 给出列在该行的值（缺失为 NaN）。
    pub fn eval(&self, row: usize, col: &dyn Fn(&str) -> f64) -> f64 {
        match self {
            Expr::Num(v) => *v,
            Expr::Col(k) => col(k),
            Expr::Row => (row + 1) as f64,
            Expr::Neg(a) => -a.eval(row, col),
            Expr::Bin(op, a, b) => {
                let (x, y) = (a.eval(row, col), b.eval(row, col));
                match op {
                    '+' => x + y,
                    '-' => x - y,
                    '*' => x * y,
                    '/' => x / y,
                    _ => x.powf(y),
                }
            }
            Expr::Call(name, args) => {
                let x = args[0].eval(row, col);
                let y = args.get(1).map_or(f64::NAN, |a| a.eval(row, col));
                match name.as_str() {
                    "sin" => x.sin(),
                    "cos" => x.cos(),
                    "tan" => x.tan(),
                    "asin" => x.asin(),
                    "acos" => x.acos(),
                    "atan" => x.atan(),
                    "sinh" => x.sinh(),
                    "cosh" => x.cosh(),
                    "tanh" => x.tanh(),
                    "exp" => x.exp(),
                    "ln" => x.ln(),
                    "log" | "log10" => x.log10(),
                    "sqrt" => x.sqrt(),
                    "abs" => x.abs(),
                    "floor" => x.floor(),
                    "ceil" => x.ceil(),
                    "round" => x.round(),
                    "sign" => {
                        if x == 0.0 {
                            0.0
                        } else {
                            x.signum()
                        }
                    }
                    "pow" => x.powf(y),
                    "atan2" => x.atan2(y),
                    "min" => x.min(y),
                    "max" => x.max(y),
                    _ => x.rem_euclid(y),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn eval(src: &str, a: f64, b: f64) -> f64 {
        parse(src).unwrap().eval(4, &|k| match k {
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
    }

    #[test]
    fn errors_are_readable() {
        for (src, msg) in [
            ("", "为空"),
            ("1 +", "不完整"),
            ("(1 + 2", "缺少“)”"),
            ("foo(1)", "没有函数"),
            ("sin(1, 2)", "1 个参数"),
            ("A B", "缺少运算符"),
            ("2 $ 3", "无法识别"),
            ("col(1)", "短名"),
            ("1 )", "位置不对"),
        ] {
            let e = parse(src).unwrap_err();
            assert!(e.contains(msg), "{src}: {e}");
        }
    }
}
