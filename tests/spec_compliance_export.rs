mod common;
use common::{assert_exported, assert_not_exported, assert_error};
use korni::{parse, Entry};

// --- 1. Basic Export ---

#[test]
fn test_export_basic() { assert_exported("export K=v", "K", "v"); }
#[test]
fn test_export_tab() { assert_exported("export\tK=v", "K", "v"); }

// --- 2. Leading Whitespace ---

#[test]
fn test_export_leading_space() { assert_exported(" export K=v", "K", "v"); }
#[test]
fn test_export_leading_tab() { assert_exported("\texport K=v", "K", "v"); }
#[test]
fn test_export_leading_mixed() { assert_exported(" \t export K=v", "K", "v"); }

// --- 3. Key False Alarms (Looks like export but isn't prefix) ---

#[test]
fn test_export_as_part_of_key() { assert_not_exported("exportK=v", "exportK", "v"); }
#[test]
fn test_export_underscore() { assert_not_exported("export_K=v", "export_K", "v"); }
#[test]
fn test_exported_key() { assert_not_exported("exported=v", "exported", "v"); }
#[test]
fn test_exporting_key() { assert_not_exported("exporting=v", "exporting", "v"); }
#[test]
fn test_expor_key() { assert_not_exported("expor=v", "expor", "v"); }

// --- 4. Value Side (export in value is just value) ---

#[test]
fn test_export_in_value() { 
    // "K=export v" -> Value is "export". "v" is ignored junk.
    let input = "K=export v";
    let entries = parse(input);
    let kv = entries[0].as_pair().unwrap();
    assert_eq!(kv.value, "export");
    assert!(!kv.is_exported);
}

// --- 5. Export with Quotes ---

#[test]
fn test_export_quoted_double() { assert_exported("export K=\"v\"", "K", "v"); }
#[test]
fn test_export_quoted_single() { assert_exported("export K='v'", "K", "v"); }

// --- 6. Export Edge Cases ---

#[test]
fn test_export_many_spaces() { assert_exported("export    K=v", "K", "v"); }

#[test]
fn test_export_no_key_error() {
    // "export =v" -> Empty key
    assert_error("export =v", "Empty key");
}

#[test]
fn test_export_only_error() {
    // "export" line
    // Spec says: "export keyword without definition ... is invalid"
    // Our parser currently might ignore it or error.
    // Let's check parser.rs logic:
    // It sees "export", consumes it if followed by space.
    // Then consumes whitespace.
    // Then expects Key. If EOF or no key char -> Error?
    // "export\n" -> Consumes export, then newline.
    // `key_start == key_end` check -> if !eof && peek == =, empty key.
    // But if peek == newline, it recovers line.
    // So "export\n" simply produces NO entry (recover_line skips).
    // Let's verify:
    let input = "export\n";
    let entries = parse(input);
    // Should be 0 entries if ignored, or error.
    // Actually, `consume_key` consumes 0 chars. `key_start == key_end`.
    // Then `if key_start == key_end`: if peek == '=' -> Error("Empty key").
    // Else `recover_line()` -> skips to newline.
    // So "export" on a line results in 0 entries (ignored).
    assert_eq!(entries.len(), 1);
    assert!(matches!(entries[0], Entry::Error(_)));
}

#[test]
fn test_export_space_only() {
    let input = "export ";
    let entries = parse(input);
    assert_eq!(entries.len(), 0);
}

