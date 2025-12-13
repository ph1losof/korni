mod common;
use common::{assert_pair, assert_error};

// =========================================================================
// SINGLE QUOTES ('...')
// =========================================================================

// --- 1. Basic Single Quotes ---

#[test]
fn test_single_simple() { assert_pair("K='val'", "K", "val"); }
#[test]
fn test_single_empty() { assert_pair("K=''", "K", ""); }
#[test]
fn test_single_space() { assert_pair("K=' '", "K", " "); }
#[test]
fn test_single_whitespace() { assert_pair("K=' \t '", "K", " \t "); }

// --- 2. Single Quotes Literals (No Escapes) ---

#[test]
fn test_single_backslash() { assert_pair("K='\\'", "K", "\\"); }
#[test]
fn test_single_two_backslashes() { assert_pair("K='\\\\'", "K", "\\\\"); }
#[test]
fn test_single_fake_newline() { assert_pair("K='\\n'", "K", "\\n"); }
#[test]
fn test_single_fake_tab() { assert_pair("K='\\t'", "K", "\\t"); }
#[test]
fn test_single_fake_quote() { assert_pair("K='\\''", "K", "\\"); } // This one is tricky. ' can't be escaped in single quotes.
// Wait, if I write 'hello\' world', the parser sees 'hello\' then space...
// Actually, ' inside single quotes closes the string immediately.
// There is NO way to put a single quote inside a single quoted string.
// So: K='a\'b' -> K="a\"
// The parser consumes 'a\' then sees b...
// Let's verify behaviors for "malformed" single quote escapes
#[test]
fn test_single_quote_cannot_be_escaped() {
    // Input: K='a\'b'
    // Parsed: K='a\', then junk b'
    // Parser behavior: 'a\' is the value. Then it sees b'.
    // Standard parser ignores junk after closing quote.
    assert_pair("K='a\\'b'", "K", "a\\");
}

#[test]
fn test_single_dollar() { assert_pair("K='$VAR'", "K", "$VAR"); }
#[test]
fn test_single_hash() { assert_pair("K='#comment'", "K", "#comment"); }
#[test]
fn test_single_double_quote() { assert_pair("K='\"'", "K", "\""); }

// --- 3. Single Quote Multiline ---

#[test]
fn test_single_multiline_basic() { assert_pair("K='Line1\nLine2'", "K", "Line1\nLine2"); }
#[test]
fn test_single_multiline_trailing_newline() { assert_pair("K='Line1\n'", "K", "Line1\n"); }
#[test]
fn test_single_multiline_leading_newline() { assert_pair("K='\nLine2'", "K", "\nLine2"); }
#[test]
fn test_single_multiline_empty_lines() { assert_pair("K='\n\n'", "K", "\n\n"); }

// --- 4. Single Quote Errors ---

#[test]
fn test_single_unclosed_eol() { assert_error("K='val", "Unclosed"); }
#[test]
fn test_single_unclosed_eof() { assert_error("K='val", "Unclosed"); }


// =========================================================================
// DOUBLE QUOTES ("...")
// =========================================================================

// --- 5. Basic Double Quotes ---

#[test]
fn test_double_simple() { assert_pair("K=\"val\"", "K", "val"); }
#[test]
fn test_double_empty() { assert_pair("K=\"\"", "K", ""); }
#[test]
fn test_double_space() { assert_pair("K=\" \"", "K", " "); }

// --- 6. Escape Sequences (Standard) ---

#[test]
fn test_double_escape_n() { assert_pair("K=\"\\n\"", "K", "\n"); }
#[test]
fn test_double_escape_r() { assert_pair("K=\"\\r\"", "K", "\r"); }
#[test]
fn test_double_escape_t() { assert_pair("K=\"\\t\"", "K", "\t"); }
#[test]
fn test_double_escape_backslash() { assert_pair("K=\"\\\\\"", "K", "\\"); }
#[test]
fn test_double_escape_quote() { assert_pair("K=\"\\\"\"", "K", "\""); }
#[test]
fn test_double_escape_dollar() { assert_pair("K=\"\\$\"", "K", "$"); }

// --- 7. Unknown Escapes (Preserved) ---

#[test]
fn test_double_unknown_a() { assert_pair("K=\"\\a\"", "K", "\\a"); }
#[test]
fn test_double_unknown_b() { assert_pair("K=\"\\b\"", "K", "\\b"); }
#[test]
fn test_double_unknown_f() { assert_pair("K=\"\\f\"", "K", "\\f"); }
#[test]
fn test_double_unknown_v() { assert_pair("K=\"\\v\"", "K", "\\v"); }
#[test]
fn test_double_unknown_z() { assert_pair("K=\"\\z\"", "K", "\\z"); }
#[test]
fn test_double_unknown_zero() { assert_pair("K=\"\\0\"", "K", "\\0"); }

// --- 8. Complex Escapes ---

#[test]
fn test_double_json_1() { assert_pair("K=\"{\\\"a\\\": 1}\"", "K", "{\"a\": 1}"); }
#[test]
fn test_double_json_2() { assert_pair("K=\"[1, \\\"2\\\"]\"", "K", "[1, \"2\"]"); }

// --- 9. Double Quote Multiline ---

#[test]
fn test_double_multiline_basic() { assert_pair("K=\"Line1\nLine2\"", "K", "Line1\nLine2"); }
#[test]
fn test_double_multiline_escaped_newline() { assert_pair("K=\"Line1\\nLine2\"", "K", "Line1\nLine2"); }
#[test]
fn test_double_multiline_backslash_continuation() { 
    // Usually double quotes don't need backslash for multiline, they just span lines.
    // If a backslash is at EOL in double quote, it is a literal backslash followed by newline chars, usually.
    // The spec 4.2.4 says: "all other instances of \ followed by any character SHOULD be preserved literally"
    // So "val\" -> val\ (literal backslash)
    // "val\
    // next"
    // -> val\ + \n + next
    // WAIT: Spec says "Trailing Escaped Quote" logic: `\"` at end acts as literal quote.
    // But what about `\` at end?
    // "KEY=\"value\\
    // more\""
    // -> value\ + newline + more
    assert_pair("K=\"A\\\nB\"", "K", "A\\\nB"); 
}

// --- 10. Trailing Junk After Quotes ---

#[test]
fn test_double_junk_after() { assert_pair("K=\"val\" junk", "K", "val"); }
#[test]
fn test_single_junk_after() { assert_pair("K='val' junk", "K", "val"); }
#[test]
fn test_double_junk_hash() { assert_pair("K=\"val\" #comment", "K", "val"); }

// --- 11. Nested Quotes ---

#[test]
fn test_double_in_single() { assert_pair("K='\"quoted\"'", "K", "\"quoted\""); }
#[test]
fn test_single_in_double() { assert_pair("K=\"'quoted'\"", "K", "'quoted'"); }
#[test]
fn test_mixed_nesting() { assert_pair("K=\"'a' \\\"b\\\"\"", "K", "'a' \"b\""); }

// --- 12. Errors ---

#[test]
fn test_double_unclosed_eof() { assert_error("K=\"val", "Unclosed"); }

// --- 13. Special Double Quote Cases ---

#[test]
fn test_double_quote_at_end_of_line_escape() {
    // Case: KEY="val\"
    //       next"
    // The \" is escaped quote.
    assert_pair("K=\"val\\\"\nnext\"", "K", "val\"\nnext");
}

#[test]
fn test_consecutive_backslashes() {
    assert_pair("K=\"a\\\\b\"", "K", "a\\b"); // 2 -> 1
    assert_pair("K=\"a\\\\\\b\"", "K", "a\\\\b"); // 3 -> 2 (first 2 make 1, 3rd is literal with b)
    assert_pair("K=\"a\\\\\\\\b\"", "K", "a\\\\b"); // 4 -> 2
}

