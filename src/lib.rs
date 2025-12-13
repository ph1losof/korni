mod ast;
mod parser;
pub mod env;
pub mod builder;
pub mod iter;
pub mod error;

pub use ast::*;
pub use env::Environment;
pub use builder::{Korni, KorniBuilder};
pub use iter::EnvIterator;
pub use parser::Parser;

/// Parse input string into a list of entries (fast mode: key-value pairs only).
pub fn parse(input: &str) -> Vec<Entry> {
    let mut parser = Parser::new(input);
    parser.parse()
}

/// Parse with custom options.
pub fn parse_with_options(input: &str, options: ParseOptions) -> Vec<Entry> {
    let mut parser = Parser::with_options(input, options);
    parser.parse()
}

impl<'a> Entry<'a> {
    pub fn as_pair(&self) -> Option<&KeyValuePair> {
        match self {
            Entry::Pair(kv) => Some(kv),
            _ => None,
        }
    }
}
