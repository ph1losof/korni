# korni

An ultra-fast, stateless, and failure-tolerant parser for `.env` files, written in Rust.

Designed for high-performance tooling (LSPs, linters, formatters) and applications that need deep introspection into environment configuration files.

## Features

- 🚀 **Blazingly Fast**: Heavily optimized using zero-copy parsing (`Cow` strings) and SIMD-friendly slice iterators.
- 📍 **Introspective**: Tracks exact line and column positions (spans) for keys, values, and comments.
- 💬 **Comment Support**: First-class support for parsing and preserving comments, including commented-out key-value pairs.
- 🛡️ **Failure Tolerant**: Continues parsing after errors, collecting all issues instead of halting on the first one.
- 📦 **Zero Runtime Dependencies**: Lightweight and easy to drop into any project.

## Installation

```toml
[dependencies]
korni = "0.1.1"
```

## Quick Start

```rust
use korni::{parse, ParseOptions};

fn main() {
    let input = r#"
# Database Configuration
DB_HOST=localhost
DB_PORT=5432
"#;

    // Simple parsing (fast mode)
    let entries = parse(input);

    for entry in entries {
        if let Some(pair) = entry.as_pair() {
            println!("Key: {}, Value: {}", pair.key, pair.value);
        }
    }

    // Advanced parsing with positions and comments
    let full_entries = korni::parse_with_options(input, ParseOptions::full());
}
```

## API Reference

### Core Types

#### `Entry<'a>`

The main parsing result type representing a single line in an `.env` file:

```rust
pub enum Entry<'a> {
    Comment(Span),           // A comment line (# ...)
    Pair(KeyValuePair<'a>),  // A key-value pair (KEY=value)
    Error(Error),            // A parsing error
}
```

#### `KeyValuePair<'a>`

Represents a parsed key-value pair with full introspection:

```rust
pub struct KeyValuePair<'a> {
    pub key: Cow<'a, str>,              // The key name
    pub key_span: Option<Span>,         // Span of the key
    pub value: Cow<'a, str>,            // The parsed value
    pub value_span: Option<Span>,       // Span of the value
    pub quote: QuoteType,               // Single, Double, or None
    pub open_quote_pos: Option<Position>,
    pub close_quote_pos: Option<Position>,
    pub equals_pos: Option<Position>,   // Position of the '='
    pub is_exported: bool,              // Whether 'export' keyword was used
    pub is_comment: bool,               // Whether this was in a comment (# KEY=value)
}
```

#### `ParseOptions`

Configure parsing behavior:

```rust
pub struct ParseOptions {
    pub include_comments: bool,  // Parse & include comments in output
    pub track_positions: bool,   // Track line/col/offset positions
}

// Presets
ParseOptions::fast()  // Default: no comments, no positions
ParseOptions::full()  // Comments + positions enabled
```

#### `Position` and `Span`

For precise location tracking:

```rust
pub struct Position {
    pub line: usize,    // 0-indexed line number
    pub col: usize,     // 0-indexed column number
    pub offset: usize,  // Byte offset from file start
}

pub struct Span {
    pub start: Position,
    pub end: Position,
}
```

#### `QuoteType`

Indicates how a value was quoted:

```rust
pub enum QuoteType {
    Single,  // 'value'
    Double,  // "value"
    None,    // value (unquoted)
}
```

### Parsing Functions

#### `parse(input: &str) -> Vec<Entry>`

Fast parsing - returns key-value pairs only, no position tracking:

```rust
let entries = korni::parse("KEY=value");
```

#### `parse_with_options(input: &str, options: ParseOptions) -> Vec<Entry>`

Configurable parsing with custom options:

```rust
let entries = korni::parse_with_options(input, ParseOptions::full());
```

### Builder API

The builder API provides a fluent interface for parsing from various sources:

#### From String

```rust
use korni::Korni;

let env = Korni::from_str("KEY=value")
    .preserve_comments()
    .track_positions()
    .parse()?;

println!("{}", env.get("KEY").unwrap());
```

#### From File

```rust
let env = Korni::from_file(".env")
    .preserve_comments()
    .parse()?;
```

#### Auto-discover File

Searches current directory and ancestors for the file:

```rust
let env = Korni::find_file(".env")?
    .parse()?;
```

#### From Bytes

```rust
let bytes = b"KEY=value";
let env = Korni::from_bytes(bytes).parse()?;
```

#### From Reader

```rust
use std::io::Cursor;

let reader = Cursor::new("KEY=value");
let env = Korni::from_reader(reader).parse()?;
```

### Environment API

The `Environment` struct provides a HashMap-like interface:

```rust
use korni::Korni;

let env = Korni::from_str("
    DB_HOST=localhost
    DB_PORT=5432
").parse()?;

// Get a value
let host = env.get("DB_HOST");           // Option<&str>
let port = env.get_or("DB_PORT", "3306"); // &str with default

// Get full entry with metadata
if let Some(entry) = env.get_entry("DB_HOST") {
    println!("Quoted: {:?}", entry.quote);
    println!("Exported: {}", entry.is_exported);
}

// Iterate all pairs
for pair in env.iter() {
    println!("{} = {}", pair.key, pair.value);
}

// Check for errors
if env.has_errors() {
    for error in env.errors() {
        eprintln!("Error: {}", error);
    }
}

// Export to HashMap<String, String>
let map = env.to_map();
```

### Iterator API

Stream entries one at a time for memory efficiency:

```rust
use korni::{Parser, EnvIterator};

let parser = Parser::new("KEY1=a\nKEY2=b");
let iter = EnvIterator::new(parser);

for entry in iter {
    // Process entry
}
```

### Error Types

All parsing errors include byte offsets for precise error reporting:

```rust
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

// Get the byte offset of any error
let offset = error.offset();
```

## Parsing Rules

### Keys

- Must contain only ASCII alphanumeric characters and underscores (`[A-Za-z0-9_]`)
- Must NOT start with a digit
- No whitespace allowed between key and `=`

```env
VALID_KEY=value
_also_valid=value
# Invalid: 123_KEY=value (starts with digit)
# Invalid: KEY = value (whitespace around =)
```

### Values

#### Unquoted Values

- Terminate at first whitespace or end of line
- Support line continuation with trailing backslash

```env
KEY=simple_value
MULTI=line1\
line2
```

#### Single-Quoted Values

- Literal strings - no escape processing
- Must be closed on the same line

```env
KEY='raw value with $VAR not expanded'
```

#### Double-Quoted Values

- Support escape sequences: `\n`, `\r`, `\t`, `\\`, `\"`, `\$`
- Unknown escapes are preserved literally

```env
KEY="line1\nline2"
ESCAPED="contains \"quotes\""
```

### Comments

- Lines starting with `#` are comments
- Inline comments: `KEY=value # comment` (requires whitespace before `#`)
- Commented-out pairs (`# KEY=value`) are parsed with `is_comment: true`

### Export Keyword

The optional `export` prefix is supported:

```env
export DATABASE_URL=postgres://localhost/db
```

### BOM Handling

- UTF-8 BOM (`\xEF\xBB\xBF`) at file start is silently skipped
- BOM in middle of file produces an error

## Comparison with `dotenvy`

[dotenvy](https://github.com/allan2/dotenvy) is the standard, battle-tested crate for _loading_ environment variables in Rust applications. `korni` serves a different, more specialized purpose.

| Feature                 | `dotenvy`                         | `korni`                                  |
| ----------------------- | --------------------------------- | ---------------------------------------- |
| **Primary Goal**        | **Load** env vars into `std::env` | **Parse** env files into structured data |
| **Output**              | Modifies process environment      | AST / Structured Iterators               |
| **Introspection**       | None (opaque loading)             | Full (Spans, Line/Col, Offsets)          |
| **Comments**            | Ignored                           | Parsed & Preserved                       |
| **Error Handling**      | Stops on first error              | Failure-tolerant (collects all errors)   |
| **Modifies `std::env`** | ✅ Yes                            | ❌ No (Pure parsing)                     |
| **Use Case**            | Application Configuration         | Tooling (IDEs, Linters), Complex Configs |

### When to use `dotenvy`

- You just want `cargo run` to pick up your `.env` file.
- You need standard, 12-factor app configuration.

### When to use `korni`

- You are building an IDE plugin, linter, or formatter.
- You need to analyze the _structure_ of an `.env` file (e.g. "where is `DB_PORT` defined?").
- You need performance-critical parsing of massive files.
- You want to manually control how environment variables are applied.

## Specification & Compliance

### EDF Specification

This parser implements the [EDF (Ecolog Dotenv File Format) 1.0.0 specification](https://github.com/ph1losof/ecolog-spec).

### Compliance Statement

**`korni` aims for EDF 1.0.0 compliance.** Per the specification's compliance requirements:

> A parser implementation claiming EDF 1.0.0 Compliance MUST adhere to ALL requirements specified in the specification. This is a strict, all-or-nothing compliance model.

#### Requirements Implemented

- ✅ **Parsing Rules**: Keys, values (quoted/unquoted), comments, and multiline handling per Section 4
- ✅ **Error Handling**: Failure-tolerant parsing with detailed error messages including byte offsets
- ✅ **Security**: UTF-8 validation, BOM handling, error message safety
- ✅ **Edge Cases**: Empty values, escape sequences, line continuations

#### Allowed Variations (per spec)

This implementation varies in:

- **Performance**: Heavily optimized with zero-copy parsing and SIMD-friendly iterators
- **API Design**: Rust-idiomatic with `Cow` strings, builders, and iterators
- **Additional Features**: Position tracking, comment parsing, and `is_comment` flag for commented-out pairs

#### Version Compatibility

- **Specification Version**: EDF 1.0.0
- **Semantic Versioning**: This library follows semver. Major version bumps indicate potential parsing behavior changes.

## License

MIT
