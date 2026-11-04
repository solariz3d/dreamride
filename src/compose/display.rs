//! Equations as the world typesets them: a definition line becomes a small markup the overlay lays out as real math
//! (fractions stacked, floor brackets, exponents raised), with the terms that changed since the last stage marked.
//!
//! The markup is plain UTF-8 text with a few commands, each `\name{...}` (braces always balanced, and no braces or
//! backslashes ever appear in the text itself, since the song language has neither):
//!
//! | markup | drawn as |
//! |---|---|
//! | `\frac{a}{b}` | a over b, with a bar |
//! | `\floor{x}` `\ceil{x}` `\abs{x}` `\sqrt{x}` | brackets and the radical sized to their content |
//! | `\paren{x}` | parentheses sized to their content |
//! | `\sup{x}` | x raised and smaller, after the box before it |
//! | `\it{x}` `\rm{x}` | italic (a variable), upright (a function name) |
//! | `\hl{x}` | x is a term that changed at this stage |
//! | `\,` | a thin space |
//! | `\brk` | a place the line may break (before a top-level + or −) |
//!
//! The parser mirrors `crate::expr`'s grammar (loosest to tightest: comparison, sum, product with implicit
//! multiplication, unary minus, power, primary), so the layout follows the meaning, not the source's spacing; the
//! source's own brackets are re-derived from precedence.

/// A token of a definition's text, with its byte span.
#[derive(Debug, Clone, PartialEq)]
pub enum Tok {
    Num(String),
    Ident(String),
    /// `+ - * / ^ ( ) ,` and the comparisons.
    Sym(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub tok: Tok,
    pub start: usize,
    pub end: usize,
}

impl Token {
    pub fn text(&self) -> &str {
        match &self.tok {
            Tok::Num(s) | Tok::Ident(s) | Tok::Sym(s) => s,
        }
    }
}

/// Tokens of a song-language expression. Characters the language does not have are skipped (the text has already
/// been parsed by the engine, so there are none).
pub fn tokens(src: &str) -> Vec<Token> {
    let b = src.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        let c = b[i] as char;
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        let start = i;
        if c.is_ascii_digit() || (c == '.' && b.get(i + 1).is_some_and(|d| d.is_ascii_digit())) {
            let mut dot = false;
            while i < b.len() && ((b[i] as char).is_ascii_digit() || (b[i] == b'.' && !dot)) {
                dot |= b[i] == b'.';
                i += 1;
            }
            out.push(Token { tok: Tok::Num(src[start..i].to_string()), start, end: i });
        } else if c.is_ascii_alphabetic() || c == '_' {
            while i < b.len() && ((b[i] as char).is_ascii_alphanumeric() || b[i] == b'_') {
                i += 1;
            }
            out.push(Token { tok: Tok::Ident(src[start..i].to_string()), start, end: i });
        } else if matches!(c, '<' | '>' | '=' | '!') {
            i += if b.get(i + 1) == Some(&b'=') { 2 } else { 1 };
            out.push(Token { tok: Tok::Sym(src[start..i].to_string()), start, end: i });
        } else if "+-*/^(),".contains(c) {
            i += 1;
            out.push(Token { tok: Tok::Sym(c.to_string()), start, end: i });
        } else {
            i += 1;
        }
    }
    out
}

/// Which tokens of `new` are not in a longest common subsequence with `old` (compared by text): the terms an edit
/// changed or added.
pub fn changed_tokens(old: &[Token], new: &[Token]) -> Vec<bool> {
    let (n, m) = (old.len(), new.len());
    let mut l = vec![vec![0u16; m + 1]; n + 1];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            l[i][j] = if old[i].text() == new[j].text() { l[i + 1][j + 1] + 1 } else { l[i + 1][j].max(l[i][j + 1]) };
        }
    }
    let mut changed = vec![true; m];
    let (mut i, mut j) = (0, 0);
    while i < n && j < m {
        if old[i].text() == new[j].text() {
            changed[j] = false;
            i += 1;
            j += 1;
        } else if l[i + 1][j] >= l[i][j + 1] {
            i += 1;
        } else {
            j += 1;
        }
    }
    changed
}

// ------------------------------------------------------------------------------------------------ the display tree

#[derive(Debug, Clone)]
enum D {
    Num(String, bool),
    Name(String, bool),
    Call(String, bool, Vec<D>),
    /// `+ - * / ^`, and `j` for implicit multiplication.
    Bin(char, bool, Box<D>, Box<D>),
    Neg(bool, Box<D>),
    Cmp(String, bool, Box<D>, Box<D>),
}

struct P<'a> {
    t: &'a [Token],
    hl: &'a [bool],
    i: usize,
}

impl P<'_> {
    fn peek(&self) -> Option<&Tok> {
        self.t.get(self.i).map(|t| &t.tok)
    }
    fn sym(&self, s: &str) -> bool {
        matches!(self.peek(), Some(Tok::Sym(x)) if x == s)
    }
    fn take(&mut self) -> (Option<Tok>, bool) {
        let r = (self.t.get(self.i).map(|t| t.tok.clone()), self.hl.get(self.i).copied().unwrap_or(false));
        self.i += 1;
        r
    }
    fn cmp(&mut self) -> D {
        let l = self.sum();
        if let Some(Tok::Sym(s)) = self.peek() {
            if matches!(s.as_str(), "<" | ">" | "<=" | ">=" | "==" | "!=") {
                let s = s.clone();
                let (_, h) = self.take();
                let r = self.sum();
                return D::Cmp(s, h, Box::new(l), Box::new(r));
            }
        }
        l
    }
    fn sum(&mut self) -> D {
        let mut l = self.mul();
        while self.sym("+") || self.sym("-") {
            let op = if self.sym("+") { '+' } else { '-' };
            let (_, h) = self.take();
            let r = self.mul();
            l = D::Bin(op, h, Box::new(l), Box::new(r));
        }
        l
    }
    fn mul(&mut self) -> D {
        let mut l = self.unary();
        loop {
            if self.sym("*") || self.sym("/") {
                let op = if self.sym("*") { '*' } else { '/' };
                let (_, h) = self.take();
                let r = self.unary();
                l = D::Bin(op, h, Box::new(l), Box::new(r));
            } else if matches!(self.peek(), Some(Tok::Ident(_))) || self.sym("(") {
                let r = self.power();
                l = D::Bin('j', false, Box::new(l), Box::new(r));
            } else {
                return l;
            }
        }
    }
    fn unary(&mut self) -> D {
        if self.sym("-") {
            let (_, h) = self.take();
            return D::Neg(h, Box::new(self.unary()));
        }
        if self.sym("+") {
            self.take();
            return self.unary();
        }
        self.power()
    }
    fn power(&mut self) -> D {
        let base = self.primary();
        if self.sym("^") {
            let (_, h) = self.take();
            let e = self.unary();
            return D::Bin('^', h, Box::new(base), Box::new(e));
        }
        base
    }
    fn primary(&mut self) -> D {
        match self.take() {
            (Some(Tok::Num(s)), h) => D::Num(s, h),
            (Some(Tok::Ident(s)), h) => {
                if self.sym("(") {
                    self.take();
                    let mut args = Vec::new();
                    if !self.sym(")") {
                        loop {
                            args.push(self.cmp());
                            if self.sym(",") {
                                self.take();
                            } else {
                                break;
                            }
                        }
                    }
                    if self.sym(")") {
                        self.take();
                    }
                    D::Call(s, h, args)
                } else {
                    D::Name(s, h)
                }
            }
            (Some(Tok::Sym(s)), _) if s == "(" => {
                let e = self.cmp();
                if self.sym(")") {
                    self.take();
                }
                e
            }
            _ => D::Num("?".into(), false),
        }
    }
}

fn prec(d: &D) -> u8 {
    match d {
        D::Cmp(..) => 0,
        D::Bin('+' | '-', ..) => 1,
        D::Bin('*' | 'j', ..) => 2,
        D::Bin('/', ..) => 5, // stacked: a box of its own
        D::Neg(..) => 3,
        D::Bin('^', ..) => 4,
        _ => 5,
    }
}

fn hl(s: String, on: bool) -> String {
    if on {
        format!("\\hl{{{s}}}")
    } else {
        s
    }
}

fn greek(name: &str) -> Option<&'static str> {
    Some(match name {
        "tau" => "τ",
        "pi" => "π",
        "phi" => "φ",
        "theta" => "θ",
        _ => return None,
    })
}

/// A name as typeset: one letter (or a Greek name) in italic, longer names upright, `x_1` with a subscript.
pub fn name_markup(name: &str) -> String {
    if let Some(g) = greek(name) {
        return format!("\\it{{{g}}}");
    }
    if let Some((a, b)) = name.split_once('_') {
        if a.chars().count() == 1 && !b.is_empty() {
            return format!("\\it{{{a}}}\\sub{{{b}}}");
        }
    }
    if name.chars().count() == 1 {
        format!("\\it{{{name}}}")
    } else {
        format!("\\rm{{{name}}}")
    }
}

/// Starts with a digit (for juxtaposition: `12d` needs no space, `d c` does).
fn is_num(d: &D) -> bool {
    matches!(d, D::Num(..))
}

struct Emit {
    /// Fractions nest at most this deep; deeper ones are written inline with a slash.
    frac_depth: u32,
}

impl Emit {
    fn wrap(&mut self, d: &D, need: u8) -> String {
        let s = self.go(d, false);
        if prec(d) < need {
            format!("\\paren{{{s}}}")
        } else {
            s
        }
    }

    fn go(&mut self, d: &D, top: bool) -> String {
        match d {
            D::Num(s, h) => hl(s.clone(), *h),
            D::Name(s, h) => hl(name_markup(s), *h),
            D::Neg(h, x) => format!("{}{}", hl("−".into(), *h), self.wrap(x, 3)),
            D::Cmp(op, h, l, r) => {
                let sym = match op.as_str() {
                    "==" => "=",
                    "!=" => "≠",
                    "<=" => "≤",
                    ">=" => "≥",
                    o => o,
                };
                format!("{} {} {}", self.wrap(l, 1), hl(sym.into(), *h), self.wrap(r, 1))
            }
            D::Bin(op @ ('+' | '-'), h, l, r) => {
                let ls = if top { self.go(l, true) } else { self.wrap(l, 1) };
                let rs = self.wrap(r, 2);
                let sym = if *op == '+' { "+" } else { "−" };
                format!("{ls}{}{} {rs}", if top { "\\brk " } else { " " }, hl(sym.into(), *h))
            }
            D::Bin('*', h, l, r) => {
                let (ls, rs) = (self.wrap(l, 2), self.wrap(r, 3));
                if *h {
                    format!("{ls}\\,{}\\,{rs}", hl("·".into(), true))
                } else if is_num(l) && is_num(r) {
                    format!("{ls}·{rs}")
                } else {
                    format!("{ls}\\,{rs}")
                }
            }
            D::Bin('j', _, l, r) => {
                let (ls, rs) = (self.wrap(l, 2), self.wrap(r, 3));
                // `12d`, `3T(u)` and `2(x + 1)` set tight; before a word-like name (`1 sq(...)`), a thin space.
                let tight = match &**r {
                    D::Name(n, _) | D::Call(n, _, _) => n.chars().count() == 1 || greek(n).is_some(),
                    other => prec(other) < 3,
                };
                if is_num(l) && tight {
                    format!("{ls}{rs}")
                } else {
                    format!("{ls}\\,{rs}")
                }
            }
            D::Bin('/', h, l, r) => {
                if self.frac_depth >= 2 {
                    format!("{}{}{}", self.wrap(l, 3), hl("/".into(), *h), self.wrap(r, 4))
                } else {
                    self.frac_depth += 1;
                    let s = format!("\\frac{{{}}}{{{}}}", self.go(l, false), self.go(r, false));
                    self.frac_depth -= 1;
                    hl(s, *h)
                }
            }
            D::Bin('^', h, b, e) => {
                let bs = self.wrap(b, 5);
                let es = self.go(e, false);
                format!("{bs}\\sup{{{}}}", hl(es, *h))
            }
            D::Bin(..) => String::new(),
            D::Call(name, h, args) => {
                let inner = |me: &mut Emit| args.iter().map(|a| me.go(a, false)).collect::<Vec<_>>().join(", ");
                let bracket = match name.as_str() {
                    "floor" => Some("floor"),
                    "ceil" => Some("ceil"),
                    "abs" => Some("abs"),
                    "sqrt" => Some("sqrt"),
                    _ => None,
                };
                if let (Some(b), 1) = (bracket, args.len()) {
                    let s = format!("\\{b}{{{}}}", inner(self));
                    return hl(s, *h);
                }
                let n = if name == "log2" {
                    "\\rm{log}\\sub{2}".to_string() // log₂, as written by hand
                } else if crate::expr::is_reserved(name) {
                    format!("\\rm{{{name}}}")
                } else {
                    name_markup(name)
                };
                format!("{}\\paren{{{}}}", hl(n, *h), inner(self))
            }
        }
    }
}

/// The markup of an expression, with `changed` marking which of its tokens to highlight (all false for none).
pub fn expr_markup(src: &str, changed: Option<&[bool]>) -> String {
    let toks = tokens(src);
    let none = vec![false; toks.len()];
    let hl = changed.filter(|c| c.len() == toks.len()).unwrap_or(&none);
    let mut p = P { t: &toks, hl, i: 0 };
    let d = p.cmp();
    Emit { frac_depth: 0 }.go(&d, true)
}

/// The markup of a definition line `name(a, b) = rhs`, highlighting the terms that differ from `old` (the same
/// definition at the stage before; None when it is new or unchanged).
pub fn def_markup(line: &str, old: Option<&str>) -> String {
    let Some(eq) = line.find('=') else { return line.to_string() };
    let (lhs, rhs) = (line[..eq].trim(), line[eq + 1..].trim());
    let lhs_m = match lhs.split_once('(') {
        Some((name, params)) => {
            let ps: Vec<String> = params.trim_end_matches(')').split(',').map(|p| name_markup(p.trim())).collect();
            format!("{}\\paren{{{}}}", name_markup(name.trim()), ps.join(", "))
        }
        None => name_markup(lhs),
    };
    let changed = old.and_then(|o| o.find('=').map(|e| o[e + 1..].trim().to_string())).map(|o| changed_tokens(&tokens(&o), &tokens(rhs)));
    format!("{lhs_m} = {}", expr_markup(rhs, changed.as_deref()))
}
