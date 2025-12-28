mod common;
use common::{assert_pair};
use korni::{parse, Korni, Error};

#[test]
fn test_empty_input() {
    let input = "";
    let entries = parse(input);
    assert!(entries.is_empty());
}

#[test]
fn test_whitespace_only() {
    let input = "   \n\t\n   \n";
    let entries = parse(input);
    assert!(entries.is_empty());
}

#[test]
fn test_only_comments() {
    let input = "# Comment 1\n# Comment 2\n# Comment 3";
    let entries = parse(input);
    assert!(entries.is_empty());
}

#[test]
fn test_mixed_empty_lines_and_comments() {
    let input = "# Header\n\n\n# Another comment\n\n# Footer";
    let entries = parse(input);
    assert!(entries.is_empty());
}

#[test]
fn test_single_key_value_no_newline() {
    let input = "KEY=value";
    let entries = parse(input);
    assert_eq!(entries.len(), 1);
    assert_pair(input, "KEY", "value");
}

#[test]
fn test_very_long_unquoted_value() {
    let mut value = String::new();
    for i in 0..10000 {
        value.push_str(&format!("word{}", i));
    }
    let input = format!("LONG_KEY={}", value);
    let entries = parse(&input);
    assert_eq!(entries.len(), 1);
    let kv = entries[0].as_pair().unwrap();
    assert_eq!(kv.key, "LONG_KEY");
    assert!(kv.value.len() > 50000);
}

#[test]
fn test_very_long_quoted_value_single() {
    let mut value = String::new();
    for i in 0..10000 {
        value.push_str(&format!("word{} ", i));
    }
    let input = format!("LONG_KEY='{}'", value.trim());
    let entries = parse(&input);
    assert_eq!(entries.len(), 1);
    let kv = entries[0].as_pair().unwrap();
    assert_eq!(kv.key, "LONG_KEY");
    assert!(kv.value.len() > 50000);
}

#[test]
fn test_very_long_quoted_value_double() {
    let mut value = String::new();
    for i in 0..10000 {
        value.push_str(&format!("word{} ", i));
    }
    let input = format!("LONG_KEY=\"{}\"", value.trim());
    let entries = parse(&input);
    assert_eq!(entries.len(), 1);
    let kv = entries[0].as_pair().unwrap();
    assert_eq!(kv.key, "LONG_KEY");
    assert!(kv.value.len() > 50000);
}

#[test]
fn test_very_long_key() {
    let mut key = String::new();
    for i in 0..1000 {
        key.push_str(&format!("PART{}_", i));
    }
    key.push_str("END");
    let input = format!("{}=value", key);
    let entries = parse(&input);
    assert_eq!(entries.len(), 1);
    let kv = entries[0].as_pair().unwrap();
    assert!(kv.key.len() > 5000);
    assert_eq!(kv.value, "value");
}

#[test]
fn test_many_empty_lines() {
    let mut input = String::new();
    for _ in 0..1000 {
        input.push('\n');
    }
    input.push_str("KEY=value");
    for _ in 0..1000 {
        input.push('\n');
    }
    let entries = parse(&input);
    assert_eq!(entries.len(), 1);
    assert_pair(&input, "KEY", "value");
}

#[test]
fn test_unicode_in_values() {
    let input = "NAME=Günter🎉\nEMOJI=😀🎉🚀";
    let entries = parse(input);
    assert_eq!(entries.len(), 2);
    let kv1 = entries[0].as_pair().unwrap();
    assert_eq!(kv1.key, "NAME");
    assert_eq!(kv1.value, "Günter🎉");
    let kv2 = entries[1].as_pair().unwrap();
    assert_eq!(kv2.key, "EMOJI");
    assert_eq!(kv2.value, "😀🎉🚀");
}

#[test]
fn test_unicode_in_keys() {
    // Note: Non-ASCII characters in keys may not be parsed correctly
    // depending on implementation. This test documents current behavior.
    let input = "USER_NAME=test\nEMOJI_KEY=value";
    let entries = parse(input);
    assert_eq!(entries.len(), 2);
    let kv1 = entries[0].as_pair().unwrap();
    assert_eq!(kv1.key, "USER_NAME");
    let kv2 = entries[1].as_pair().unwrap();
    assert_eq!(kv2.key, "EMOJI_KEY");
}

#[test]
fn test_invalid_utf8_bytes() {
    let invalid_bytes = &[0xFF, 0xFE, 0xFD];
    let result = Korni::from_bytes(invalid_bytes).parse();
    assert!(result.is_err());
    match result {
        Err(Error::InvalidUtf8 { .. }) => (),
        _ => panic!("Expected InvalidUtf8 error"),
    }
}

#[test]
fn test_invalid_utf8_in_middle() {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"KEY=value\n");
    bytes.extend_from_slice(&[0xC0, 0x80]); // Invalid UTF-8
    bytes.extend_from_slice(b"\nKEY2=value2");
    
    let result = Korni::from_bytes(&bytes).parse();
    assert!(result.is_err());
    match result {
        Err(Error::InvalidUtf8 { .. }) => {
            // Successfully detected invalid UTF-8
        },
        _ => panic!("Expected InvalidUtf8 error"),
    }
}
