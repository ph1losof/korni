mod common;
use common::{assert_pair, assert_error};
use korni::parse;

// --- 1. Valid UTF-8 ---

#[test]
fn test_utf8_valid_chars() { assert_pair("K=äöü", "K", "äöü"); }

#[test]
fn test_utf8_key_is_invalid() {
    let input = "ä=v";
    let entries = parse(input);
    // Should be skipped due to invalid start char (non-ascii)
    // The bytes are C3 A4. Not ascii.
    // So consume_key consumes 0.
    // Then check for empty key error conditions strictly?
    // Code says: if key_start == key_end: ...
    // "ä" is not '=', so it falls through to recover_line.
    assert_eq!(entries.len(), 0);
}

// --- 2. BOM Handling ---

#[test]
fn test_bom_start() {
    // BOM is EF BB BF
    // Or \u{FEFF}
    let input = "\u{FEFF}K=v";
    assert_pair(input, "K", "v");
}

#[test]
fn test_bom_middle_error() {
    let input = "K=v\n\u{FEFF}K2=v2";
    // Parser checks `find('\u{FEFF}')`.
    // If idx > 0 -> Error.
    assert_error(input, "BOM found at invalid position");
}

#[test]
fn test_bom_in_value_error() {
    let input = "K=val\u{FEFF}ue";
    assert_error(input, "BOM found at invalid position");
}

// --- 3. Invalid UTF-8 Bytes ---

// Rust strings (&str) strictly enforce UTF-8.
// We cannot pass invalid UTF-8 via &str to `parse(input: &str)`.
// The caller (e.g. file reader) would fail before calling parse if enforcing strict UTF-8.
// However, if we wanted to test behavior on invalid bytes, we'd need a byte-oriented interface,
// but the library signature is `pub fn parse(input: &str)`.
// Thus, BY DEFINITION, the input is valid UTF-8.
// We cannot verify "invalid UTF-8 rejection" inside the parser itself because the type system prevents it.
// We can assume the "File reader" layer handles this, or if the parser took `&[u8]`.
// But `Parser::new(input: &str)`.
// So we skip "invalid non-utf8 bytes" tests because they can't be compiled/run safely with this API.

#[test]
fn test_valid_utf8_emoji_in_value() {
    assert_pair("K=💩", "K", "💩");
}

