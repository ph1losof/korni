use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    InvalidUtf8 { offset: usize, reason: String },
    UnclosedQuote { quote_type: &'static str, offset: usize },
    InvalidKey { offset: usize, reason: String },
    ForbiddenWhitespace { location: &'static str, offset: usize },
    DoubleEquals { offset: usize },
    InvalidBom { offset: usize },
    Expected { offset: usize, expected: &'static str },
    Generic { offset: usize, message: String },
    Io(String),
}

impl Error {
    pub fn offset(&self) -> usize {
        match self {
            Error::InvalidUtf8 { offset, .. } => *offset,
            Error::UnclosedQuote { offset, .. } => *offset,
            Error::InvalidKey { offset, .. } => *offset,
            Error::ForbiddenWhitespace { offset, .. } => *offset,
            Error::DoubleEquals { offset } => *offset,
            Error::InvalidBom { offset } => *offset,
            Error::Expected { offset, .. } => *offset,
            Error::Generic { offset, .. } => *offset,
            Error::Io(_) => 0,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::InvalidUtf8 { offset, reason } => write!(f, "Invalid UTF-8 at byte {}: {}", offset, reason),
            Error::UnclosedQuote { quote_type, offset } => write!(f, "Unclosed {} quote starting at byte {}", quote_type, offset),
            Error::InvalidKey { offset, reason } => write!(f, "Invalid key at byte {}: {}", offset, reason),
            Error::ForbiddenWhitespace { location, offset } => write!(f, "Whitespace not allowed {} at byte {}", location, offset),
            Error::DoubleEquals { offset } => write!(f, "Double equals sign detected at byte {}. Use quotes: KEY=\"=val\"", offset),
            Error::InvalidBom { offset } => write!(f, "BOM found at invalid position (byte {})", offset),
            Error::Expected { offset, expected } => write!(f, "Expected {} at byte {}", expected, offset),
            Error::Generic { offset, message } => write!(f, "{} at byte {}", message, offset),
            Error::Io(msg) => write!(f, "IO Error: {}", msg),
        }
    }
}

impl std::error::Error for Error {}
