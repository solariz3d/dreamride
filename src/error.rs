//! One error type for the whole engine. Every error says what went wrong and, when it comes from
//! the song text, where (1-based line and column).

use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub struct Error {
    pub line: Option<usize>,
    pub col: Option<usize>,
    pub message: String,
}

impl Error {
    pub fn at(line: usize, col: usize, message: impl Into<String>) -> Self {
        Error { line: Some(line), col: Some(col), message: message.into() }
    }

    pub fn line(line: usize, message: impl Into<String>) -> Self {
        Error { line: Some(line), col: None, message: message.into() }
    }

    pub fn msg(message: impl Into<String>) -> Self {
        Error { line: None, col: None, message: message.into() }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (self.line, self.col) {
            (Some(l), Some(c)) => write!(f, "line {l}, column {c}: {}", self.message),
            (Some(l), None) => write!(f, "line {l}: {}", self.message),
            _ => write!(f, "{}", self.message),
        }
    }
}

impl std::error::Error for Error {}
