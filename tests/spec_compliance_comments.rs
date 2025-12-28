mod common;
use common::assert_pair;
use korni::{parse, Entry, ParseOptions};

// --- 1. Line Comments ---

#[test]
fn test_comment_only() {
    let input = "# This is a comment";
    let entries = parse(input);
    assert_eq!(entries.len(), 0); // Fast mode ignores comments
}

#[test]
fn test_comment_indent() {
    let input = "   # Indented comment";
    let entries = parse(input);
    assert_eq!(entries.len(), 0);
}

#[test]
fn test_comment_tab_indent() {
    let input = "\t# Tab indented";
    let entries = parse(input);
    assert_eq!(entries.len(), 0);
}

#[test]
fn test_comment_multiple_lines() {
    let input = "# C1\n# C2\n# C3";
    let entries = parse(input);
    assert_eq!(entries.len(), 0);
}

// --- 2. Inline Comments ---

#[test]
fn test_inline_comment_space() { assert_pair("K=v # comment", "K", "v"); }
#[test]
fn test_inline_comment_tab() { assert_pair("K=v\t# comment", "K", "v"); }
#[test]
fn test_inline_comment_many_spaces() { assert_pair("K=v   # comment", "K", "v"); }
#[test]
fn test_inline_comment_quoted_double() { assert_pair("K=\"v\" # comment", "K", "v"); }
#[test]
fn test_inline_comment_quoted_single() { assert_pair("K='v' # comment", "K", "v"); }
#[test]
fn test_inline_comment_no_space_quoted() { 
    // "KEY=\"val\"#comment" -> value is "val", rest is junk/ignored
    assert_pair("K=\"v\"#comment", "K", "v"); 
}

// --- 3. Not Comments (Hash in Value) ---

#[test]
fn test_hash_no_space() { assert_pair("K=val#ue", "K", "val#ue"); }
#[test]
fn test_hash_at_start() { assert_pair("K=#val", "K", "#val"); }
#[test]
fn test_hash_in_double_quote() { assert_pair("K=\"val # comment\"", "K", "val # comment"); }
#[test]
fn test_hash_in_single_quote() { assert_pair("K='val # comment'", "K", "val # comment"); }

// Spec: "Unrecognized Escape Sequences: ... SHOULD be preserved literally"
// So "\#" -> "\#"
// Actually, is # special in double quotes? No. So \ is not ignored?
// "\#" -> \ is unknown escape -> "\#"
#[test]
fn test_hash_escaped_double_check() { assert_pair("K=\"\\#\"", "K", "\\#"); }

// --- 4. Include Comments Option ---

#[test]
fn test_option_include_comments() {
    let input = "# C1\nK=v\n# C2";
    let options = ParseOptions { include_comments: true, track_positions: false };
    let entries = korni::parse_with_options(input, options);
    assert_eq!(entries.len(), 3);
    assert!(matches!(entries[0], Entry::Comment(_)));
    assert!(matches!(entries[1], Entry::Pair(_)));
    assert!(matches!(entries[2], Entry::Comment(_)));
}

// --- 5. Commented Out Pairs ---

#[test]
fn test_commented_pair_detection() {
    let input = "# K=v";
    let options = ParseOptions { include_comments: true, track_positions: false };
    let entries = korni::parse_with_options(input, options);
    
    // Parser logic for commented pair:
    // It should detect "K=v" inside the comment and return Entry::Pair with is_commented=true
    let pair = entries[0].as_pair().expect("Should be parsed as commented pair");
    assert_eq!(pair.key, "K");
    assert_eq!(pair.value, "v");
    assert!(pair.is_comment);
}

#[test]
fn test_commented_pair_indent() {
    let input = "  # K=v";
    let options = ParseOptions { include_comments: true, track_positions: false };
    let entries = korni::parse_with_options(input, options);
    let pair = entries[0].as_pair().expect("Should be parsed as commented pair");
    assert_eq!(pair.key, "K");
    assert!(pair.is_comment);
}

#[test]
fn test_commented_pair_quoted() {
    let input = "# K=\"v\"";
    let options = ParseOptions { include_comments: true, track_positions: false };
    let entries = korni::parse_with_options(input, options);
    let pair = entries[0].as_pair().unwrap();
    assert_eq!(pair.value, "v");
    assert!(pair.is_comment);
}

#[test]
fn test_commented_pair_invalid() {
    // "# 1K=v" -> Invalid key 1K
    // Should fallback to Entry::Comment
    let input = "# 1K=v";
    let options = ParseOptions { include_comments: true, track_positions: false };
    let entries = korni::parse_with_options(input, options);
    assert!(matches!(entries[0], Entry::Comment(_)));
}

// --- 6. Empty Lines ---

#[test]
fn test_empty_lines_ignored() {
    let input = "\n   \n\t\n";
    let entries = parse(input);
    assert_eq!(entries.len(), 0);
}
