mod error;
mod types;
mod parser;
mod env;
mod loader;

pub use error::Error;
pub use types::{Entry, KeyValuePair, ParseOptions, QuoteType, Span, Position};
pub use env::Environment;
pub use parser::{Parser, EnvIterator};
pub use loader::{Korni, KorniBuilder, OwnedKorniBuilder};

pub fn parse(input: &str) -> Vec<Entry> {
    Parser::new(input).parse()
}

pub fn parse_with_options(input: &str, options: ParseOptions) -> Vec<Entry> {
    Parser::with_options(input, options).parse()
}
