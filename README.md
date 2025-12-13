# korni

an ultra-fast, stateless, and failure-tolerant parser for `.env` files, written in Rust.

Designed for high-performance tooling (LSPs, linters, formatters) and applications that need deep introspection into environment configuration files.

## Features

- 🚀 **Blazingly Fast**: heavily optimized using zero-copy parsing (`Cow` strings) and SIMD-friendly slice iterators.
- 📍 **Introspective**: Tracks exact line and column positions (spans) for keys, values, and comments.
- 💬 **Comment Support**: First-class support for parsing and preserving comments.
- 🛡️ **Failure Tolerant**: Continues parsing after errors, collecting all issues instead of halting on the first one.
- 📦 **Zero Runtime Dependencies**: Lightweight and easy to drop into any project.

## Installation

```toml
[dependencies]
korni = "0.1.0"
```

## Usage

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

## Comparison with `dotenvy`

[dotenvy](https://github.com/allan2/dotenvy) is the standard, battle-tested crate for *loading* environment variables in Rust applications. `korni` serves a different, more specialized purpose.

| Feature | `dotenvy` | `korni` |
|---------|-----------|-----------------|
| **Primary Goal** | **Load** env vars into `std::env` | **Parse** env files into structured data |
| **Output** | Modifies process environment | AST / Structured Iterators |
| **Introspection** | None (opaque loading) | Full (Spans, Line/Col, Offsets) |
| **Comments** | Ignored | Parsed & Preserved |
| **Error Handling** | Stops on first error | Failure-tolerant (collects errors) |
| **Modifies `std::env`**| ✅ Yes | ❌ No (Pure parsing) |
| **Use Case** | Application Configuration | Tooling (IDEs, Linters), Complex Configs |

### When to use `dotenvy`
- You just want `cargo run` to pick up your `.env` file.
- You need standard, 12-factor app configuration.

### When to use `korni`
- You are building an IDE plugin, linter, or formatter.
- You need to analyze the *structure* of an `.env` file (e.g. "where is `DB_PORT` defined?").
- You need performance-critical parsing of massive files.
- You want to manually control how environment variables are applied.
