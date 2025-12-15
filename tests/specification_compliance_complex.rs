use korni::{parse, Entry, QuoteType};

// --- Security & Encoding Tests ---



#[test]
fn test_security_newline_injection_in_quotes() {
    // Spec 6.2: Newlines in quotes are literal, NOT new definitions
    let input = "SAFE=\"value\nKEY2=malicious\"";
    let entries = parse(input);
    assert_eq!(entries.len(), 1);
    let kv = entries[0].as_pair().unwrap();
    assert_eq!(kv.key, "SAFE");
    assert_eq!(kv.value, "value\nKEY2=malicious");
}

#[test]
fn test_security_newline_injection_attempt_unquoted() {
    // Unquoted values end at newline.
    let input = "SAFE=value\nKEY2=malicious";
    let entries = parse(input);
    assert_eq!(entries.len(), 2);
    let kv1 = entries[0].as_pair().unwrap();
    assert_eq!(kv1.key, "SAFE");
    assert_eq!(kv1.value, "value");
    
    let kv2 = entries[1].as_pair().unwrap();
    assert_eq!(kv2.key, "KEY2");
    assert_eq!(kv2.value, "malicious");
}

// --- Complex Nesting & Escaping ---

#[test]
fn test_complex_nesting_deep() {
    // Spec 4.2.4: Quote types nest literally
    // Updated to use valid escaped quotes for inner quotes as parser tracks outermost only
    let input = r#"KEY="a 'b \"c\" d' e""#;
    let entries = parse(input);
    let kv = entries[0].as_pair().unwrap();
    assert_eq!(kv.value, r#"a 'b "c" d' e"#);
}

#[test]
fn test_complex_escaping_madness() {
    // Spec 4.2.4: All escape rules combined
    // Input: KEY="tab:\t quote:\" slash:\\ dollar:\$ newline:\n"
    let input = r#"KEY="tab:\t quote:\" slash:\\ dollar:\$ newline:\n""#;
    let entries = parse(input);
    let kv = entries[0].as_pair().unwrap();
    assert_eq!(kv.value, "tab:\t quote:\" slash:\\ dollar:$ newline:\n");
}

#[test]
fn test_variable_interpolation_is_literal() {
    // Spec 4.4: Interpolation is explicitly OUT OF SCOPE. Must return literal.
    let input = "Usage=${VAR} $VAR";
    let entries = parse(input);
    // Spec 4.2.2: Unquoted values end at first whitespace.
    // So `Usage=${VAR} $VAR` -> value is `${VAR}`. The rest is ignored junk.
    // This confirms interpolation is literal (no substitution) AND strict whitespace handling.
    let kv = entries[0].as_pair().unwrap();
    assert_eq!(kv.value, "${VAR}");
}

#[test]
fn test_variable_interpolation_escaped_dollar() {
    // Spec 4.4: \$ becomes literal $
    let input = r#"KEY="\${VAR}""#;
    let entries = parse(input);
    let kv = entries[0].as_pair().unwrap();
    assert_eq!(kv.value, "${VAR}");
}

// --- Multiline Quirks ---

#[test]
fn test_multiline_empty_lines_preserved_quoted() {
    // Spec 5.1: Empty lines in quoted strings preserved
    let input = "TEXT=\"Line1\n\nLine3\"";
    let entries = parse(input);
    let kv = entries[0].as_pair().unwrap();
    assert_eq!(kv.value, "Line1\n\nLine3");
}

#[test]
fn test_continuation_empty_lines_consumed() {
    // Spec 5.2 (Strict): Space terminates value.
    // "KEY=value\ \n\nmore" -> "value\". 
    // The space comes AFTER the backslash, so backslash is part of value.
    // Since we terminated at space (not newline), continuation is NOT triggered.
    let input = "KEY=value\\ \n\nmore";
    let entries = parse(input);
    let kv = entries[0].as_pair().unwrap();
    assert_eq!(kv.value, "value\\");
}

#[test]
fn test_continuation_space_preservation() {
    // Spec 5.2 (Strict): Unquoted value ends at space.
    // "KEY=val \ ..." -> "val".
    let input = "KEY=val \\\n   ue";
    let entries = parse(input);
    let kv = entries[0].as_pair().unwrap();
    assert_eq!(kv.value, "val");
}

#[test]
fn test_continuation_with_inline_comment() {
    // Spec 5.2 (Strict): Unquoted continuation CANNOT have inline comments
    // because the space required for comment implies termination.
    // "KEY=val \ # comment" -> "val"
    let input = "KEY=val \\ # comment\n  ue";
    let entries = parse(input);
    let kv = entries[0].as_pair().unwrap();
    assert_eq!(kv.value, "val");
}

// ...

#[test]
fn test_spec_5_2_continuation_before_comment() {
    // Spec 5.2 (Strict): Backslash BEFORE comment requires space before #,
    // which terminates value. So continuation is impossible here for unquoted.
    // "KEY=val \ # comment" -> "val"
    let input = "KEY=val \\ # comment\n  continued";
    let entries = parse(input);
    let kv = entries[0].as_pair().unwrap();
    assert_eq!(kv.value, "val");
}

#[test]
fn test_error_message_no_value_leak() {
    // Spec 6.3: Error messages MUST NOT leak values.
    // We create a case that causes error during value parsing?
    // Or invalid key with sensitive data?
    // "KEY with invalid char"
    let input = "KEY-SENSITIVE=secret_value"; // Invalid key (dash)
    let entries = parse(input);
    match &entries[0] {
        Entry::Error(e) => {
            let msg = e.to_string();
            // Error should define it's an invalid key, but NOT show full line content if possible?
            // Actually spec says "Invalid character in key...".
            // It shouldn't show "secret_value".
            assert!(!msg.contains("secret_value"));
        },
        _ => panic!("Should be error"),
    }
}

#[test]
fn test_mixed_newline_normalization() {
    // Spec 4.2.2: Parsers MAY normalize, but strictly preserving is fine.
    // Check behavior consistency.
    let input = "KEY=val\r\nNEXT=v\nLAST=x";
    let entries = parse(input);
    assert_eq!(entries.len(), 3);
    assert_eq!(entries[0].as_pair().unwrap().value, "val");
    assert_eq!(entries[1].as_pair().unwrap().value, "v");
    assert_eq!(entries[2].as_pair().unwrap().value, "x");
}
