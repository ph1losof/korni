mod common;
use common::{assert_pair, assert_error};

// --- 1. Valid Start Characters (A-Z, a-z, _) ---

#[test]
fn test_key_start_uppercase_a() { assert_pair("A_KEY=val", "A_KEY", "val"); }
#[test]
fn test_key_start_uppercase_z() { assert_pair("Z_KEY=val", "Z_KEY", "val"); }
#[test]
fn test_key_start_lowercase_a() { assert_pair("a_key=val", "a_key", "val"); }
#[test]
fn test_key_start_lowercase_z() { assert_pair("z_key=val", "z_key", "val"); }
#[test]
fn test_key_start_underscore() { assert_pair("_KEY=val", "_KEY", "val"); }

// --- 2. Valid Characters (Alphanumeric + Underscore) ---

#[test]
fn test_key_with_digits() { assert_pair("K1=val", "K1", "val"); }
#[test]
fn test_key_with_many_digits() { assert_pair("K1234567890=val", "K1234567890", "val"); }
#[test]
fn test_key_mixed_case() { assert_pair("KeyName=val", "KeyName", "val"); }
#[test]
fn test_key_all_caps() { assert_pair("KEY=val", "KEY", "val"); }
#[test]
fn test_key_all_lower() { assert_pair("key=val", "key", "val"); }
#[test]
fn test_key_underscores_middle() { assert_pair("MY_KEY=val", "MY_KEY", "val"); }
#[test]
fn test_key_underscores_end() { assert_pair("MY_KEY_=val", "MY_KEY_", "val"); }
#[test]
fn test_key_multiple_underscores() { assert_pair("MY__KEY=val", "MY__KEY", "val"); }
#[test]
fn test_key_alternating() { assert_pair("AaBbCc_123=val", "AaBbCc_123", "val"); }

// --- 3. Invalid Start Characters (Digits) ---

#[test]
fn test_invalid_start_0() { assert_error("0KEY=val", "digit"); }
#[test]
fn test_invalid_start_1() { assert_error("1KEY=val", "digit"); }
#[test]
fn test_invalid_start_9() { assert_error("9KEY=val", "digit"); }

// --- 4. Invalid Internal Characters ---

#[test]
#[test]
fn test_invalid_char_dot() { assert_error("KEY.NAME=val", "Expected '='"); }
#[test]
fn test_invalid_char_dash() { assert_error("KEY-NAME=val", "Expected '='"); }
#[test]
fn test_invalid_char_space() { assert_error("KEY NAME=val", "Expected '='"); }
#[test]
fn test_invalid_char_colon() { assert_error("KEY:NAME=val", "Expected '='"); }
#[test]
fn test_invalid_char_slash() { assert_error("KEY/NAME=val", "Expected '='"); }
#[test]
fn test_invalid_char_bracket() { assert_error("KEY[NAME]=val", "Expected '='"); }

// --- 5. Empty Keys ---

#[test]
fn test_empty_key() { assert_error("=val", "Empty key"); }
#[test]
fn test_whitespace_key() { assert_error("   =val", "Empty key"); }
#[test]
fn test_tab_key() { assert_error("\t=val", "Empty key"); }

// --- 6. Whitespace Handling ---

#[test]
fn test_leading_space() { assert_pair(" KEY=val", "KEY", "val"); }
#[test]
fn test_leading_tab() { assert_pair("\tKEY=val", "KEY", "val"); }
#[test]
fn test_leading_mixed() { assert_pair(" \t KEY=val", "KEY", "val"); }

#[test]
fn test_space_before_equals_error() { assert_error("KEY =val", "Whitespace not allowed between key and equals"); }
#[test]
fn test_tab_before_equals_error() { assert_error("KEY\t=val", "Whitespace not allowed between key and equals"); }
#[test]
fn test_space_after_equals_error() { assert_error("KEY= val", "Whitespace not allowed after equals"); }
#[test]
fn test_tab_after_equals_error() { assert_error("KEY=\tval", "Whitespace not allowed after equals"); }

// --- 7. Case Sensitivity ---

#[test]
fn test_case_sensitivity_distinct() {
    let input = "KEY=1\nkey=2\nKey=3";
    let entries = korni::parse(input);
    let pairs: Vec<_> = entries.iter().filter_map(|e| e.as_pair()).collect();
    assert_eq!(pairs.len(), 3);
    assert_eq!(pairs[0].key, "KEY");
    assert_eq!(pairs[1].key, "key");
    assert_eq!(pairs[2].key, "Key");
}

// --- 8. Length Stress Tests ---

#[test]
fn test_single_char_keys() {
    let chars = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz_";
    for c in chars.chars() {
        let input = format!("{}=val", c);
        assert_pair(&input, &c.to_string(), "val");
    }
}

// --- 9. Export Prefix Key Confusion ---

#[test]
fn test_export_no_space_is_key() { assert_pair("exportKEY=val", "exportKEY", "val"); }
#[test]
fn test_export_underscore_is_key() { assert_pair("export_KEY=val", "export_KEY", "val"); }
#[test]
fn test_exported_is_key() { assert_pair("exported=val", "exported", "val"); }
#[test]
fn test_expor_is_key() { assert_pair("expor=val", "expor", "val"); }

// --- 10. Weird But Valid Keys ---

#[test]
fn test_all_underscores() { assert_pair("___=val", "___", "val"); }
#[test]
fn test_underscore_then_number() { assert_pair("_1=val", "_1", "val"); }
#[test]
fn test_long_mixed() { assert_pair("A_1_b_2_C_3=val", "A_1_b_2_C_3", "val"); }

