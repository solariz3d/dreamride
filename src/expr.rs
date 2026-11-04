//! The expression language.
//!
//! Text is lexed and parsed once into a syntax tree, names are resolved against the definitions
//! (built-ins, parameters, `t`, user definitions) into a [`Node`] tree with indices instead of
//! strings, constants are folded, and the result is evaluated per sample by a plain tree walk.
//!
//! Grammar, loosest to tightest:
//!
//! ```text
//! expr    = mul (('+' | '-') mul)*
//! mul     = unary (('*' | '/') unary | power)*      a bare `power` after a term is implicit `*`
//! unary   = ('-' | '+') unary | power
//! power   = primary ('^' unary)?                    right-associative; -2^2 = -(2^2)
//! primary = number | name | name '(' args ')' | '(' expr ')'
//! ```
//!
//! Implicit multiplication applies when a term is followed directly by a name or `(`: `12d`,
//! `2(x + 1)`, `u^2(3 - 2u)`, `2 sin(t)`. A name followed by `(` is always a call, so a value
//! times a bracket needs `*`: write `d*(x + 1)`, not `d(x + 1)`.

use crate::error::Error;
use std::cell::RefCell;
use std::collections::HashMap;
use std::f64::consts::{PI, TAU};

/// The most parameters a user function may take.
pub const MAX_PARAMS: usize = 8;

// ---------------------------------------------------------------- lexer

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Num(f64),
    Ident(String),
    Plus,
    Minus,
    Star,
    Slash,
    Caret,
    LParen,
    RParen,
    Comma,
    /// A comparison operator: < > <= >= == !=
    Cmp(BinOp),
    End,
}

impl Tok {
    fn describe(&self) -> String {
        match self {
            Tok::Num(v) => format!("number {v}"),
            Tok::Ident(s) => format!("name '{s}'"),
            Tok::Plus => "'+'".into(),
            Tok::Minus => "'-'".into(),
            Tok::Star => "'*'".into(),
            Tok::Slash => "'/'".into(),
            Tok::Caret => "'^'".into(),
            Tok::LParen => "'('".into(),
            Tok::RParen => "')'".into(),
            Tok::Comma => "','".into(),
            Tok::Cmp(op) => format!("'{}'", op.symbol()),
            Tok::End => "the end of the expression".into(),
        }
    }
}

#[derive(Debug, Clone)]
struct Token {
    tok: Tok,
    col: usize,
}

/// Lex `src`, whose first character sits at column `col0` of line `line`.
fn lex(src: &str, line: usize, col0: usize) -> Result<Vec<Token>, Error> {
    let chars: Vec<char> = src.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        let col = col0 + i;
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        let starts_number =
            c.is_ascii_digit() || (c == '.' && chars.get(i + 1).is_some_and(|d| d.is_ascii_digit()));
        if starts_number {
            let start = i;
            let mut seen_dot = false;
            while i < chars.len() && (chars[i].is_ascii_digit() || (chars[i] == '.' && !seen_dot)) {
                if chars[i] == '.' {
                    seen_dot = true;
                }
                i += 1;
            }
            let text: String = chars[start..i].iter().collect();
            let v: f64 = text
                .parse()
                .map_err(|_| Error::at(line, col, format!("bad number '{text}'")))?;
            out.push(Token { tok: Tok::Num(v), col });
            continue;
        }
        if c.is_ascii_alphabetic() || c == '_' {
            let start = i;
            while i < chars.len() && (chars[i].is_ascii_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            out.push(Token { tok: Tok::Ident(chars[start..i].iter().collect()), col });
            continue;
        }
        if matches!(c, '<' | '>' | '=' | '!') {
            let eq_next = chars.get(i + 1) == Some(&'=');
            let op = match (c, eq_next) {
                ('<', false) => BinOp::Lt,
                ('<', true) => BinOp::Le,
                ('>', false) => BinOp::Gt,
                ('>', true) => BinOp::Ge,
                ('=', true) => BinOp::Eq,
                ('!', true) => BinOp::Ne,
                _ => {
                    return Err(Error::at(
                        line,
                        col,
                        format!("unexpected character '{c}' (the comparisons are < > <= >= == !=)"),
                    ))
                }
            };
            out.push(Token { tok: Tok::Cmp(op), col });
            i += if eq_next { 2 } else { 1 };
            continue;
        }
        let tok = match c {
            '+' => Tok::Plus,
            '-' => Tok::Minus,
            '*' => Tok::Star,
            '/' => Tok::Slash,
            '^' => Tok::Caret,
            '(' => Tok::LParen,
            ')' => Tok::RParen,
            ',' => Tok::Comma,
            _ => return Err(Error::at(line, col, format!("unexpected character '{c}'"))),
        };
        out.push(Token { tok, col });
        i += 1;
    }
    out.push(Token { tok: Tok::End, col: col0 + chars.len() });
    Ok(out)
}

// ---------------------------------------------------------------- parser

#[derive(Debug, Clone, Copy, PartialEq)]
enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Pow,
    Lt,
    Le,
    Gt,
    Ge,
    Eq,
    Ne,
}

impl BinOp {
    #[inline]
    fn apply(self, a: f64, b: f64) -> f64 {
        let truth = |r: bool| if r { 1.0 } else { 0.0 };
        match self {
            BinOp::Add => a + b,
            BinOp::Sub => a - b,
            BinOp::Mul => a * b,
            BinOp::Div => a / b,
            BinOp::Pow => a.powf(b),
            // A comparison is 1 or 0. With a NaN operand it is NaN, so the renderer reports it
            // instead of the NaN quietly becoming "false".
            _ if a.is_nan() || b.is_nan() => f64::NAN,
            BinOp::Lt => truth(a < b),
            BinOp::Le => truth(a <= b),
            BinOp::Gt => truth(a > b),
            BinOp::Ge => truth(a >= b),
            BinOp::Eq => truth(a == b),
            BinOp::Ne => truth(a != b),
        }
    }

    fn symbol(self) -> &'static str {
        match self {
            BinOp::Add => "+",
            BinOp::Sub => "-",
            BinOp::Mul => "*",
            BinOp::Div => "/",
            BinOp::Pow => "^",
            BinOp::Lt => "<",
            BinOp::Le => "<=",
            BinOp::Gt => ">",
            BinOp::Ge => ">=",
            BinOp::Eq => "==",
            BinOp::Ne => "!=",
        }
    }
}

/// Syntax tree before names are resolved.
#[derive(Debug, Clone)]
enum Ast {
    Num(f64),
    Name(String, usize),
    Call(String, Vec<Ast>, usize),
    Neg(Box<Ast>),
    Bin(BinOp, Box<Ast>, Box<Ast>),
}

struct Parser {
    toks: Vec<Token>,
    pos: usize,
    line: usize,
}

impl Parser {
    fn peek(&self) -> &Tok {
        &self.toks[self.pos].tok
    }

    fn col(&self) -> usize {
        self.toks[self.pos].col
    }

    fn bump(&mut self) -> Token {
        let t = self.toks[self.pos].clone();
        if self.pos + 1 < self.toks.len() {
            self.pos += 1;
        }
        t
    }

    fn parse_full(mut self) -> Result<Ast, Error> {
        let e = self.cmp()?;
        if *self.peek() != Tok::End {
            return Err(Error::at(self.line, self.col(), format!("unexpected {}", self.peek().describe())));
        }
        Ok(e)
    }

    /// The loosest level: one optional comparison. `a < b < c` is rejected rather than read
    /// as `(a < b) < c`, which is almost never what is meant.
    fn cmp(&mut self) -> Result<Ast, Error> {
        let lhs = self.expr()?;
        let Tok::Cmp(op) = *self.peek() else { return Ok(lhs) };
        self.bump();
        let rhs = self.expr()?;
        if let Tok::Cmp(_) = self.peek() {
            return Err(Error::at(
                self.line,
                self.col(),
                "comparisons cannot be chained; combine them with min(...) or if(...)",
            ));
        }
        Ok(Ast::Bin(op, Box::new(lhs), Box::new(rhs)))
    }

    fn expr(&mut self) -> Result<Ast, Error> {
        let mut lhs = self.mul()?;
        loop {
            let op = match self.peek() {
                Tok::Plus => BinOp::Add,
                Tok::Minus => BinOp::Sub,
                _ => return Ok(lhs),
            };
            self.bump();
            let rhs = self.mul()?;
            lhs = Ast::Bin(op, Box::new(lhs), Box::new(rhs));
        }
    }

    fn mul(&mut self) -> Result<Ast, Error> {
        let mut lhs = self.unary()?;
        loop {
            let rhs = match self.peek() {
                Tok::Star | Tok::Slash => {
                    let op = if *self.peek() == Tok::Star { BinOp::Mul } else { BinOp::Div };
                    self.bump();
                    let rhs = self.unary()?;
                    lhs = Ast::Bin(op, Box::new(lhs), Box::new(rhs));
                    continue;
                }
                // Implicit multiplication: `12d`, `2(x+1)`, `u^2(3-2u)`.
                Tok::Ident(_) | Tok::LParen => self.power()?,
                _ => return Ok(lhs),
            };
            lhs = Ast::Bin(BinOp::Mul, Box::new(lhs), Box::new(rhs));
        }
    }

    fn unary(&mut self) -> Result<Ast, Error> {
        match self.peek() {
            Tok::Minus => {
                self.bump();
                Ok(Ast::Neg(Box::new(self.unary()?)))
            }
            Tok::Plus => {
                self.bump();
                self.unary()
            }
            _ => self.power(),
        }
    }

    fn power(&mut self) -> Result<Ast, Error> {
        let base = self.primary()?;
        if *self.peek() == Tok::Caret {
            self.bump();
            let exp = self.unary()?;
            return Ok(Ast::Bin(BinOp::Pow, Box::new(base), Box::new(exp)));
        }
        Ok(base)
    }

    fn primary(&mut self) -> Result<Ast, Error> {
        let Token { tok, col } = self.bump();
        match tok {
            Tok::Num(v) => Ok(Ast::Num(v)),
            Tok::Ident(name) => {
                if *self.peek() != Tok::LParen {
                    return Ok(Ast::Name(name, col));
                }
                self.bump();
                let mut args = Vec::new();
                if *self.peek() != Tok::RParen {
                    loop {
                        args.push(self.cmp()?);
                        match self.peek() {
                            Tok::Comma => {
                                self.bump();
                            }
                            Tok::RParen => break,
                            other => {
                                return Err(Error::at(
                                    self.line,
                                    self.col(),
                                    format!(
                                        "expected ',' or ')' in the arguments of '{name}', found {}",
                                        other.describe()
                                    ),
                                ))
                            }
                        }
                    }
                }
                self.bump(); // ')'
                Ok(Ast::Call(name, args, col))
            }
            Tok::LParen => {
                let e = self.cmp()?;
                if *self.peek() != Tok::RParen {
                    return Err(Error::at(
                        self.line,
                        self.col(),
                        format!("expected ')' to close the '(' at column {col}, found {}", self.peek().describe()),
                    ));
                }
                self.bump();
                Ok(e)
            }
            other => Err(Error::at(
                self.line,
                col,
                format!("expected a number, a name or '(', found {}", other.describe()),
            )),
        }
    }
}

fn parse_expr(src: &str, line: usize, col0: usize) -> Result<Ast, Error> {
    Parser { toks: lex(src, line, col0)?, pos: 0, line }.parse_full()
}

// ---------------------------------------------------------------- built-ins

#[derive(Debug, Clone, Copy, PartialEq)]
enum Fn1 {
    Floor,
    Ceil,
    Frac,
    Abs,
    Sign,
    Sin,
    Cos,
    Sqrt,
    Log2,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Fn2 {
    Mod,
    Min,
    Max,
}

/// The expressive built-ins (see [`crate::expressive`]), any number of arguments.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Fx {
    Arch,
    Phrase,
    Ritard,
    Charge,
    Vibrato,
    Db,
    Smooth,
    Ca,
}

impl Fx {
    #[inline]
    fn apply(self, a: &[f64]) -> f64 {
        use crate::expressive as e;
        match self {
            Fx::Arch => e::arch(a[0], a[1]),
            Fx::Phrase => e::phrase(a[0], a[1], a[2]),
            Fx::Ritard => e::ritard(a[0], a[1], a[2]),
            Fx::Charge => e::charge(a[0]),
            Fx::Vibrato => e::vibrato(a[0], a[1], a[2], a[3], a[4]),
            Fx::Db => e::db(a[0]),
            Fx::Smooth => e::smooth(a[0]),
            Fx::Ca => ca_step(a[0], a[1], a[2]),
        }
    }
}

/// The most arguments any built-in takes (`vibrato`).
const MAX_FX_ARGS: usize = 5;

/// The most times `iter` applies its function in one evaluation (a guard on the cost per sample, not a musical limit:
/// stream species read orbits a few hundred steps long at most).
pub const ITER_MAX: f64 = 4096.0;

/// The widest row `ca` steps: the cells are the bits of an integer below 2^53.
pub const CA_MAX_WIDTH: f64 = 52.0;

/// One step of an elementary cellular automaton (Wolfram's numbering: bit `4l + 2c + r` of `rule` is the new cell for
/// left, centre and right neighbours `l`, `c`, `r`) on a circular row of `width` cells, stored as the bits of `row`
/// (cell 0 the lowest bit). NaN when the rule is not 0..255, the width not 1..52, or the row not a whole number in
/// 0..2^width.
pub fn ca_step(rule: f64, row: f64, width: f64) -> f64 {
    if !(rule.is_finite() && row.is_finite() && width.is_finite()) {
        return f64::NAN;
    }
    let (rule, w) = (rule.floor(), width.floor());
    if !(0.0..=255.0).contains(&rule) || !(1.0..=CA_MAX_WIDTH).contains(&w) {
        return f64::NAN;
    }
    let w = w as u32;
    let row = row.floor();
    if row < 0.0 || row >= (2f64).powi(w as i32) {
        return f64::NAN;
    }
    let (rule, x) = (rule as u64, row as u64);
    let mut out = 0u64;
    for i in 0..w {
        let bit = |k: u32| (x >> k) & 1;
        let l = bit((i + 1) % w);
        let c = bit(i);
        let r = bit((i + w - 1) % w);
        out |= ((rule >> (4 * l + 2 * c + r)) & 1) << i;
    }
    out as f64
}

#[derive(Debug, Clone, Copy)]
enum Builtin {
    F1(Fn1),
    F2(Fn2),
    Clamp,
    If,
    Fx(Fx),
    /// `iter(F, x0, n)`: user function `F` applied `n` times to `x0`.
    Iter,
}

/// The time an expression is evaluated at: the beat `t`, and for a tonal voice's pitch, amp and
/// cutoff, the position inside the current note (`note_age`, `note_left`, in seconds).
#[derive(Debug, Clone, Copy, Default)]
pub struct At {
    pub t: f64,
    /// Seconds since the current note's onset.
    pub age: f64,
    /// Seconds until the current note's gate closes (0 or negative once it has closed).
    pub left: f64,
}

impl At {
    pub fn beat(t: f64) -> At {
        At { t, age: 0.0, left: 0.0 }
    }
}

fn builtin_fn(name: &str) -> Option<(Builtin, usize)> {
    use Builtin::*;
    Some(match name {
        "floor" => (F1(Fn1::Floor), 1),
        "ceil" => (F1(Fn1::Ceil), 1),
        "frac" => (F1(Fn1::Frac), 1),
        "abs" => (F1(Fn1::Abs), 1),
        "sign" => (F1(Fn1::Sign), 1),
        "sin" => (F1(Fn1::Sin), 1),
        "cos" => (F1(Fn1::Cos), 1),
        "sqrt" => (F1(Fn1::Sqrt), 1),
        "log2" => (F1(Fn1::Log2), 1),
        "mod" => (F2(Fn2::Mod), 2),
        "min" => (F2(Fn2::Min), 2),
        "max" => (F2(Fn2::Max), 2),
        "clamp" => (Clamp, 3),
        "if" => (If, 3),
        "arch" => (Fx(self::Fx::Arch), 2),
        "phrase" => (Fx(self::Fx::Phrase), 3),
        "ritard" => (Fx(self::Fx::Ritard), 3),
        "charge" => (Fx(self::Fx::Charge), 1),
        "vibrato" => (Fx(self::Fx::Vibrato), 5),
        "db" => (Fx(self::Fx::Db), 1),
        "smooth" => (Fx(self::Fx::Smooth), 1),
        "ca" => (Fx(self::Fx::Ca), 3),
        "iter" => (Iter, 3),
        _ => return None,
    })
}

fn builtin_const(name: &str) -> Option<f64> {
    match name {
        "pi" => Some(PI),
        "tau" => Some(TAU),
        _ => None,
    }
}

/// The note-time variables, readable in a tonal voice's pitch, amp and cutoff.
fn note_var(name: &str) -> Option<Node> {
    match name {
        "note_age" => Some(Node::NoteAge),
        "note_left" => Some(Node::NoteLeft),
        _ => None,
    }
}

/// Names a user may not define: `t`, the note-time variables, and every built-in function or constant.
pub fn is_reserved(name: &str) -> bool {
    name == "t" || note_var(name).is_some() || builtin_fn(name).is_some() || builtin_const(name).is_some()
}

#[inline]
fn apply1(f: Fn1, x: f64) -> f64 {
    match f {
        Fn1::Floor => x.floor(),
        Fn1::Ceil => x.ceil(),
        Fn1::Frac => x - x.floor(),
        Fn1::Abs => x.abs(),
        // f64::signum(0.0) is 1; Desmos and maths say sign(0) = 0. NaN stays NaN.
        Fn1::Sign => {
            if x > 0.0 {
                1.0
            } else if x < 0.0 {
                -1.0
            } else {
                x
            }
        }
        Fn1::Sin => x.sin(),
        Fn1::Cos => x.cos(),
        Fn1::Sqrt => x.sqrt(),
        // log2 of a ratio is its interval in octaves: `12 log2(3/2)` is a just fifth in semitones.
        Fn1::Log2 => x.log2(),
    }
}

#[inline]
fn apply2(f: Fn2, a: f64, b: f64) -> f64 {
    match f {
        // Floored modulo, as in Desmos: the result has the sign of the divisor, so it is in
        // [0, b) for b > 0. Rounding can produce exactly b for a tiny negative a; that is 0.
        Fn2::Mod => {
            let r = a - b * (a / b).floor();
            if r == b {
                0.0
            } else {
                r
            }
        }
        Fn2::Min => a.min(b),
        Fn2::Max => a.max(b),
    }
}

#[inline]
fn clamp(x: f64, lo: f64, hi: f64) -> f64 {
    // Not f64::clamp, which panics when lo > hi or either bound is NaN.
    x.max(lo).min(hi)
}

// ---------------------------------------------------------------- resolved tree

#[derive(Debug, Clone)]
enum Node {
    Const(f64),
    T,
    Param(u8),
    Neg(Box<Node>),
    Bin(BinOp, Box<Node>, Box<Node>),
    F1(Fn1, Box<Node>),
    F2(Fn2, Box<Node>, Box<Node>),
    Clamp(Box<Node>, Box<Node>, Box<Node>),
    /// `if(c, a, b)`: `a` when `c > 0`, else `b`; only the branch taken is evaluated.
    If(Box<Node>, Box<Node>, Box<Node>),
    /// A call of user definition `.0` with the given arguments.
    Call(usize, Vec<Node>),
    /// An expressive built-in with its arguments.
    Fx(Fx, Vec<Node>),
    /// `iter(F, x0, n)`: user definition `.0` (one parameter) applied `floor(n)` times to `x0`.
    Iter(usize, Box<Node>, Box<Node>),
    /// Seconds since the current note's onset.
    NoteAge,
    /// Seconds until the current note's gate closes.
    NoteLeft,
}

impl Node {
    fn eval(&self, defs: &[Def], t: &At, p: &[f64]) -> f64 {
        match self {
            Node::Const(v) => *v,
            Node::T => t.t,
            Node::NoteAge => t.age,
            Node::NoteLeft => t.left,
            Node::Fx(f, args) => {
                let mut frame = [0.0; MAX_FX_ARGS];
                for (slot, a) in frame.iter_mut().zip(args) {
                    *slot = a.eval(defs, t, p);
                }
                f.apply(&frame[..args.len()])
            }
            Node::Param(i) => p[*i as usize],
            Node::Neg(a) => -a.eval(defs, t, p),
            Node::Bin(op, a, b) => op.apply(a.eval(defs, t, p), b.eval(defs, t, p)),
            Node::F1(f, a) => apply1(*f, a.eval(defs, t, p)),
            Node::F2(f, a, b) => apply2(*f, a.eval(defs, t, p), b.eval(defs, t, p)),
            Node::Clamp(x, lo, hi) => clamp(x.eval(defs, t, p), lo.eval(defs, t, p), hi.eval(defs, t, p)),
            Node::If(c, a, b) => {
                let c = c.eval(defs, t, p);
                if c.is_nan() {
                    f64::NAN
                } else if c > 0.0 {
                    a.eval(defs, t, p)
                } else {
                    b.eval(defs, t, p)
                }
            }
            Node::Call(i, args) => {
                let mut frame = [0.0; MAX_PARAMS];
                for (slot, a) in frame.iter_mut().zip(args) {
                    *slot = a.eval(defs, t, p);
                }
                defs[*i].call(defs, t, &frame[..args.len()])
            }
            Node::Iter(i, x0, n) => {
                let n = n.eval(defs, t, p);
                let mut x = x0.eval(defs, t, p);
                if !n.is_finite() || n > ITER_MAX {
                    return f64::NAN;
                }
                for _ in 0..(n.max(0.0).floor() as usize) {
                    x = defs[*i].call(defs, t, &[x]);
                }
                x
            }
        }
    }

    /// Reads the time (`t` or a note-time variable) directly, not counting what it calls.
    fn uses_t(&self) -> bool {
        self.reads(&|n| matches!(n, Node::T | Node::NoteAge | Node::NoteLeft))
    }

    /// Reads a note-time variable directly, not counting what it calls.
    fn uses_note(&self) -> bool {
        self.reads(&|n| matches!(n, Node::NoteAge | Node::NoteLeft))
    }

    fn reads(&self, leaf: &dyn Fn(&Node) -> bool) -> bool {
        match self {
            Node::T | Node::NoteAge | Node::NoteLeft | Node::Const(_) | Node::Param(_) => leaf(self),
            Node::Neg(a) | Node::F1(_, a) => a.reads(leaf),
            Node::Bin(_, a, b) | Node::F2(_, a, b) => a.reads(leaf) || b.reads(leaf),
            Node::Clamp(a, b, c) | Node::If(a, b, c) => a.reads(leaf) || b.reads(leaf) || c.reads(leaf),
            Node::Call(_, args) | Node::Fx(_, args) => args.iter().any(|a| a.reads(leaf)),
            Node::Iter(_, a, b) => a.reads(leaf) || b.reads(leaf),
        }
    }

    /// Reads a note-time variable, directly or through any definition it calls.
    fn uses_note_deep(&self, def_uses_note: &[bool]) -> bool {
        match self {
            Node::NoteAge | Node::NoteLeft => true,
            Node::T | Node::Const(_) | Node::Param(_) => false,
            Node::Neg(a) | Node::F1(_, a) => a.uses_note_deep(def_uses_note),
            Node::Bin(_, a, b) | Node::F2(_, a, b) => a.uses_note_deep(def_uses_note) || b.uses_note_deep(def_uses_note),
            Node::Clamp(a, b, c) | Node::If(a, b, c) => {
                a.uses_note_deep(def_uses_note) || b.uses_note_deep(def_uses_note) || c.uses_note_deep(def_uses_note)
            }
            Node::Call(i, args) => def_uses_note[*i] || args.iter().any(|a| a.uses_note_deep(def_uses_note)),
            Node::Fx(_, args) => args.iter().any(|a| a.uses_note_deep(def_uses_note)),
            Node::Iter(i, a, b) => def_uses_note[*i] || a.uses_note_deep(def_uses_note) || b.uses_note_deep(def_uses_note),
        }
    }
}

/// Replace references to constant definitions by their values and fold constant subtrees.
fn simplify(node: Node, consts: &[Option<f64>]) -> Node {
    let s = |n: Box<Node>| Box::new(simplify(*n, consts));
    match node {
        Node::Call(i, args) => {
            if args.is_empty() {
                if let Some(v) = consts[i] {
                    return Node::Const(v);
                }
            }
            Node::Call(i, args.into_iter().map(|a| simplify(a, consts)).collect())
        }
        Node::Neg(a) => match *s(a) {
            Node::Const(v) => Node::Const(-v),
            a => Node::Neg(Box::new(a)),
        },
        Node::Bin(op, a, b) => match (*s(a), *s(b)) {
            (Node::Const(x), Node::Const(y)) => Node::Const(op.apply(x, y)),
            (a, b) => Node::Bin(op, Box::new(a), Box::new(b)),
        },
        Node::F1(f, a) => match *s(a) {
            Node::Const(x) => Node::Const(apply1(f, x)),
            a => Node::F1(f, Box::new(a)),
        },
        Node::F2(f, a, b) => match (*s(a), *s(b)) {
            (Node::Const(x), Node::Const(y)) => Node::Const(apply2(f, x, y)),
            (a, b) => Node::F2(f, Box::new(a), Box::new(b)),
        },
        Node::Clamp(x, lo, hi) => match (*s(x), *s(lo), *s(hi)) {
            (Node::Const(x), Node::Const(lo), Node::Const(hi)) => Node::Const(clamp(x, lo, hi)),
            (x, lo, hi) => Node::Clamp(Box::new(x), Box::new(lo), Box::new(hi)),
        },
        Node::If(c, a, b) => match *s(c) {
            Node::Const(v) if v.is_nan() => Node::Const(f64::NAN),
            Node::Const(v) if v > 0.0 => simplify(*a, consts),
            Node::Const(_) => simplify(*b, consts),
            c => Node::If(Box::new(c), s(a), s(b)),
        },
        Node::Fx(f, args) => {
            let args: Vec<Node> = args.into_iter().map(|a| simplify(a, consts)).collect();
            if args.iter().all(|a| matches!(a, Node::Const(_))) {
                let vals: Vec<f64> = args.iter().map(|a| if let Node::Const(v) = a { *v } else { 0.0 }).collect();
                return Node::Const(f.apply(&vals));
            }
            Node::Fx(f, args)
        }
        Node::Iter(i, x0, n) => Node::Iter(i, s(x0), s(n)),
        other => other,
    }
}

fn plural(n: usize) -> String {
    if n == 1 {
        "1 argument".into()
    } else {
        format!("{n} arguments")
    }
}

struct Resolver<'a> {
    index: &'a HashMap<String, usize>,
    arity: &'a [usize],
    params: &'a [String],
    line: usize,
    deps: Vec<usize>,
}

impl Resolver<'_> {
    fn resolve(&mut self, ast: &Ast) -> Result<Node, Error> {
        Ok(match ast {
            Ast::Num(v) => Node::Const(*v),
            Ast::Neg(a) => Node::Neg(Box::new(self.resolve(a)?)),
            Ast::Bin(op, a, b) => Node::Bin(*op, Box::new(self.resolve(a)?), Box::new(self.resolve(b)?)),
            Ast::Name(name, col) => self.name(name, *col)?,
            Ast::Call(name, args, col) => self.call(name, args, *col)?,
        })
    }

    fn name(&mut self, name: &str, col: usize) -> Result<Node, Error> {
        if let Some(i) = self.params.iter().position(|p| p == name) {
            return Ok(Node::Param(i as u8));
        }
        if name == "t" {
            return Ok(Node::T);
        }
        if let Some(v) = note_var(name) {
            return Ok(v);
        }
        if let Some(v) = builtin_const(name) {
            return Ok(Node::Const(v));
        }
        if let Some(&i) = self.index.get(name) {
            if self.arity[i] == 0 {
                self.deps.push(i);
                return Ok(Node::Call(i, Vec::new()));
            }
            return Err(Error::at(
                self.line,
                col,
                format!("'{name}' is a function of {}; call it as {name}(...)", plural(self.arity[i])),
            ));
        }
        if builtin_fn(name).is_some() {
            return Err(Error::at(self.line, col, format!("'{name}' is a function; call it as {name}(...)")));
        }
        Err(Error::at(self.line, col, format!("unknown name '{name}'")))
    }

    fn call(&mut self, name: &str, args: &[Ast], col: usize) -> Result<Node, Error> {
        let is_value = self.params.iter().any(|p| p == name)
            || name == "t"
            || note_var(name).is_some()
            || builtin_const(name).is_some()
            || self.index.get(name).is_some_and(|&i| self.arity[i] == 0);
        if is_value {
            return Err(Error::at(
                self.line,
                col,
                format!("'{name}' is not a function (to multiply, write {name}*(...))"),
            ));
        }
        let check = |want: usize| -> Result<(), Error> {
            if args.len() != want {
                return Err(Error::at(
                    self.line,
                    col,
                    format!("'{name}' takes {}, got {}", plural(want), args.len()),
                ));
            }
            Ok(())
        };
        if let Some((Builtin::Iter, n)) = builtin_fn(name) {
            check(n)?;
            let f = match &args[0] {
                Ast::Name(f, fcol) => match self.index.get(f.as_str()) {
                    Some(&i) if self.arity[i] == 1 => {
                        self.deps.push(i);
                        i
                    }
                    _ => {
                        return Err(Error::at(
                            self.line,
                            *fcol,
                            format!("iter's first argument must name a definition of one parameter, like F(x) = ...; '{f}' is not one"),
                        ))
                    }
                },
                _ => return Err(Error::at(self.line, col, "iter's first argument must be the name of a definition, like iter(F, 0.5, n)")),
            };
            let x0 = self.resolve(&args[1])?;
            let n = self.resolve(&args[2])?;
            return Ok(Node::Iter(f, Box::new(x0), Box::new(n)));
        }
        if let Some((b, n)) = builtin_fn(name) {
            check(n)?;
            let mut r: Vec<Node> = Vec::with_capacity(n);
            for a in args {
                r.push(self.resolve(a)?);
            }
            let mut it = r.into_iter().map(Box::new);
            let mut next = || it.next().expect("arity checked");
            return Ok(match b {
                Builtin::F1(f) => Node::F1(f, next()),
                Builtin::F2(f) => {
                    let a = next();
                    Node::F2(f, a, next())
                }
                Builtin::Clamp => {
                    let (x, lo) = (next(), next());
                    Node::Clamp(x, lo, next())
                }
                Builtin::If => {
                    let (c, a) = (next(), next());
                    Node::If(c, a, next())
                }
                Builtin::Fx(f) => Node::Fx(f, it.map(|b| *b).collect()),
                Builtin::Iter => unreachable!("handled above"),
            });
        }
        if let Some(&i) = self.index.get(name) {
            check(self.arity[i])?;
            self.deps.push(i);
            let mut r = Vec::with_capacity(args.len());
            for a in args {
                r.push(self.resolve(a)?);
            }
            return Ok(Node::Call(i, r));
        }
        Err(Error::at(self.line, col, format!("unknown function '{name}'")))
    }
}

// ---------------------------------------------------------------- definitions and programs

/// A definition as written, before resolution.
#[derive(Debug, Clone)]
pub(crate) struct DefSrc {
    name: String,
    params: Vec<String>,
    body: Ast,
    line: usize,
    col: usize,
}

#[derive(Debug, Clone)]
struct Def {
    body: Node,
    /// Takes parameters and does not depend on `t`, directly or through what it calls: a pure
    /// function of its arguments, so its results can be cached.
    cacheable: bool,
    cache: RefCell<Vec<CacheEntry>>,
}

/// Slots in each definition's result cache (direct-mapped by a hash of the argument bits).
const CACHE_SLOTS: usize = 64;

#[derive(Debug, Clone, Copy)]
struct CacheEntry {
    key: [u64; MAX_PARAMS],
    value: f64,
    valid: bool,
}

impl Def {
    fn new(body: Node, cacheable: bool) -> Def {
        let empty = CacheEntry { key: [0; MAX_PARAMS], value: 0.0, valid: false };
        Def { body, cacheable, cache: RefCell::new(if cacheable { vec![empty; CACHE_SLOTS] } else { Vec::new() }) }
    }

    /// Evaluate the body with `args`. Musical functions are mostly called with step or bar
    /// numbers, the same integers for thousands of samples in a row, so a pure definition keeps a
    /// small cache keyed by the exact bits of its arguments. The values are identical with or
    /// without it; only the time changes.
    #[inline]
    fn call(&self, defs: &[Def], t: &At, args: &[f64]) -> f64 {
        if !self.cacheable {
            return self.body.eval(defs, t, args);
        }
        let mut key = [0u64; MAX_PARAMS];
        let mut h: u64 = 0x9E37_79B9_7F4A_7C15;
        for (k, a) in key.iter_mut().zip(args) {
            *k = a.to_bits();
            h = (h ^ *k).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            h ^= h >> 29;
        }
        let slot = (h as usize) % CACHE_SLOTS;
        {
            let cache = self.cache.borrow();
            let e = &cache[slot];
            if e.valid && e.key == key {
                return e.value;
            }
        } // the borrow ends before the body runs, which may call other definitions
        let value = self.body.eval(defs, t, args);
        self.cache.borrow_mut()[slot] = CacheEntry { key, value, valid: true };
        value
    }
}

/// Parse one definition line (comment already stripped): `name = expr` or `name(a, b) = expr`.
pub(crate) fn parse_definition(text: &str, line: usize) -> Result<DefSrc, Error> {
    let shape = "expected a definition like `name = ...` or `name(a, b) = ...`";
    let first_col = text.chars().take_while(|c| c.is_whitespace()).count() + 1;
    let Some(eq) = text.find('=') else {
        return Err(Error::at(line, first_col, shape));
    };
    let lhs = lex(&text[..eq], line, 1)?;
    let toks: Vec<&Tok> = lhs.iter().map(|t| &t.tok).collect();
    let (name, col) = match toks.first() {
        Some(Tok::Ident(n)) => (n.clone(), lhs[0].col),
        _ => return Err(Error::at(line, first_col, shape)),
    };
    let mut params = Vec::new();
    match &toks[1..] {
        [Tok::End] => {}
        [Tok::LParen, rest @ ..] => {
            // ident (',' ident)* ')' End
            let mut k = 0;
            loop {
                match rest.get(k) {
                    Some(Tok::Ident(p)) => params.push(p.clone()),
                    _ => return Err(Error::at(line, first_col, shape)),
                }
                match rest.get(k + 1) {
                    Some(Tok::Comma) => k += 2,
                    Some(Tok::RParen) if rest.get(k + 2) == Some(&&Tok::End) => break,
                    _ => return Err(Error::at(line, first_col, shape)),
                }
            }
        }
        _ => return Err(Error::at(line, first_col, shape)),
    }
    let rhs = &text[eq + 1..];
    if rhs.trim().is_empty() {
        return Err(Error::at(line, col, format!("nothing after '=' in the definition of '{name}'")));
    }
    let rhs_col = text[..eq + 1].chars().count() + 1;
    let body = parse_expr(rhs, line, rhs_col)?;
    Ok(DefSrc { name, params, body, line, col })
}

/// Strip a `#` comment from a line.
pub(crate) fn strip_comment(line: &str) -> &str {
    match line.find('#') {
        Some(i) => &line[..i],
        None => line,
    }
}

/// A set of compiled definitions. Expressions are compiled against a program and evaluated with it.
#[derive(Debug, Clone, Default)]
pub struct Program {
    defs: Vec<Def>,
    index: HashMap<String, usize>,
    arity: Vec<usize>,
    consts: Vec<Option<f64>>,
    /// Whether each definition reads `note_age` or `note_left`, directly or through what it calls.
    uses_note: Vec<bool>,
}

/// A compiled expression of `t`.
#[derive(Debug, Clone)]
pub struct Expr {
    node: Node,
    uses_note: bool,
}

impl Expr {
    /// Evaluate at time `t` (in beats). Never fails; a NaN or infinity is returned as is, and the
    /// renderer reports it. The note-time variables read as 0.
    #[inline]
    pub fn eval(&self, program: &Program, t: f64) -> f64 {
        self.node.eval(&program.defs, &At::beat(t), &[])
    }

    /// Evaluate at a time that includes the position inside the current note.
    #[inline]
    pub fn eval_at(&self, program: &Program, at: &At) -> f64 {
        self.node.eval(&program.defs, at, &[])
    }

    /// Whether the expression reads `note_age` or `note_left`, directly or through a definition.
    pub fn uses_note(&self) -> bool {
        self.uses_note
    }

    /// The value, if the expression does not depend on `t`.
    pub fn constant(&self) -> Option<f64> {
        match self.node {
            Node::Const(v) => Some(v),
            _ => None,
        }
    }
}

impl Program {
    /// Compile a text of definitions only, one per line, with `#` comments and blank lines allowed.
    pub fn from_source(src: &str) -> Result<Program, Error> {
        let mut defs = Vec::new();
        for (n, raw) in src.lines().enumerate() {
            let text = strip_comment(raw);
            if text.trim().is_empty() {
                continue;
            }
            defs.push(parse_definition(text, n + 1)?);
        }
        Program::compile(defs)
    }

    pub(crate) fn compile(src: Vec<DefSrc>) -> Result<Program, Error> {
        let mut index = HashMap::new();
        for (i, d) in src.iter().enumerate() {
            if is_reserved(&d.name) {
                return Err(Error::at(d.line, d.col, format!("'{}' is a built-in name and cannot be redefined", d.name)));
            }
            if let Some(prev) = index.insert(d.name.clone(), i) {
                return Err(Error::at(
                    d.line,
                    d.col,
                    format!("'{}' is defined twice (first on line {})", d.name, src[prev].line),
                ));
            }
            if d.params.len() > MAX_PARAMS {
                return Err(Error::at(d.line, d.col, format!("'{}' has more than {MAX_PARAMS} parameters", d.name)));
            }
            for (k, p) in d.params.iter().enumerate() {
                if d.params[..k].contains(p) {
                    return Err(Error::at(d.line, d.col, format!("parameter '{p}' of '{}' is named twice", d.name)));
                }
                if p != "t" && is_reserved(p) {
                    return Err(Error::at(d.line, d.col, format!("parameter '{p}' of '{}' is a built-in name", d.name)));
                }
            }
        }
        let arity: Vec<usize> = src.iter().map(|d| d.params.len()).collect();

        let mut bodies = Vec::with_capacity(src.len());
        let mut deps = Vec::with_capacity(src.len());
        for d in &src {
            let mut r = Resolver { index: &index, arity: &arity, params: &d.params, line: d.line, deps: Vec::new() };
            bodies.push(r.resolve(&d.body)?);
            deps.push(r.deps);
        }

        // Reject recursion, and get an order in which every definition follows what it uses.
        let mut state = vec![0u8; src.len()];
        let mut order = Vec::with_capacity(src.len());
        let mut stack = Vec::new();
        for i in 0..src.len() {
            visit(i, &deps, &mut state, &mut stack, &mut order, &src)?;
        }

        // `uses_t` counts the note-time variables too: a definition that reads them is neither a
        // constant nor a pure function of its arguments.
        let mut uses_t = vec![false; src.len()];
        let mut uses_note = vec![false; src.len()];
        for &i in &order {
            uses_t[i] = bodies[i].uses_t() || deps[i].iter().any(|&d| uses_t[d]);
            uses_note[i] = bodies[i].uses_note() || deps[i].iter().any(|&d| uses_note[d]);
        }

        let mut program = Program {
            defs: bodies
                .into_iter()
                .enumerate()
                .map(|(i, body)| Def::new(body, !src[i].params.is_empty() && !uses_t[i]))
                .collect(),
            index,
            arity,
            consts: vec![None; src.len()],
            uses_note,
        };
        // Evaluate constant definitions (no parameters, no t) once, in dependency order.
        for &i in &order {
            if src[i].params.is_empty() && !uses_t[i] {
                let v = program.defs[i].body.eval(&program.defs, &At::default(), &[]);
                if !v.is_finite() {
                    return Err(Error::at(
                        src[i].line,
                        src[i].col,
                        format!("'{}' evaluates to {v}, which is not a finite number", src[i].name),
                    ));
                }
                program.consts[i] = Some(v);
            }
        }
        let consts = program.consts.clone();
        for d in &mut program.defs {
            let body = std::mem::replace(&mut d.body, Node::Const(0.0));
            d.body = simplify(body, &consts);
            // Entries made while evaluating the constants came from the unsimplified bodies.
            for e in d.cache.get_mut().iter_mut() {
                e.valid = false;
            }
        }
        Ok(program)
    }

    /// Compile an expression written on its own (reported as line 1).
    pub fn compile_expr(&self, src: &str) -> Result<Expr, Error> {
        self.compile_expr_at(src, 1, 1)
    }

    /// Compile an expression that starts at `line`, `col` of a larger text.
    pub fn compile_expr_at(&self, src: &str, line: usize, col: usize) -> Result<Expr, Error> {
        let ast = parse_expr(src, line, col)?;
        let mut r = Resolver { index: &self.index, arity: &self.arity, params: &[], line, deps: Vec::new() };
        let node = r.resolve(&ast)?;
        let uses_note = node.uses_note_deep(&self.uses_note);
        Ok(Expr { node: simplify(node, &self.consts), uses_note })
    }

    /// Call a user definition by name with the given arguments, at t = 0.
    pub fn call(&self, name: &str, args: &[f64]) -> Result<f64, Error> {
        self.call_at(name, args, 0.0)
    }

    /// Call a user definition by name with the given arguments, at time `t`.
    pub fn call_at(&self, name: &str, args: &[f64], t: f64) -> Result<f64, Error> {
        let &i = self.index.get(name).ok_or_else(|| Error::msg(format!("unknown function '{name}'")))?;
        if args.len() != self.arity[i] {
            return Err(Error::msg(format!("'{name}' takes {}, got {}", plural(self.arity[i]), args.len())));
        }
        Ok(self.defs[i].body.eval(&self.defs, &At::beat(t), args))
    }
}

fn visit(
    i: usize,
    deps: &[Vec<usize>],
    state: &mut [u8],
    stack: &mut Vec<usize>,
    order: &mut Vec<usize>,
    src: &[DefSrc],
) -> Result<(), Error> {
    match state[i] {
        2 => return Ok(()),
        1 => {
            let start = stack.iter().position(|&s| s == i).unwrap_or(0);
            let mut names: Vec<&str> = stack[start..].iter().map(|&s| src[s].name.as_str()).collect();
            names.push(&src[i].name);
            return Err(Error::at(
                src[i].line,
                src[i].col,
                format!("definitions refer to each other in a loop ({}); recursion is not allowed", names.join(" -> ")),
            ));
        }
        _ => {}
    }
    state[i] = 1;
    stack.push(i);
    for &d in &deps[i] {
        visit(d, deps, state, stack, order, src)?;
    }
    stack.pop();
    state[i] = 2;
    order.push(i);
    Ok(())
}
