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
    // Spec 5.2: Empty lines consumed during continuation
    let input = "KEY=value\\\n\nmore";
    let entries = parse(input);
    let kv = entries[0].as_pair().unwrap();
    // value + (empty line consumed) + more -> valuemore
    assert_eq!(kv.value, "valuemore");
}

#[test]
fn test_continuation_space_preservation() {
    // Spec 5.2: Preserves leading whitespace of next line
    let input = "KEY=val\\\n   ue";
    let entries = parse(input);
    let kv = entries[0].as_pair().unwrap();
    assert_eq!(kv.value, "val   ue");
}

#[test]
fn test_continuation_with_inline_comment() {
    // Spec 5.2: Inline comments separate from backslash
    let input = "KEY=val \\ # comment\n  ue";
    let entries = parse(input);
    let kv = entries[0].as_pair().unwrap();
    assert_eq!(kv.value, "val   ue");
}

// --- Edge Cases from Spec ---

#[test]
fn test_consecutive_backslashes_unquoted() {
    // Spec 5.2: 
    // KEY=value\\ -> value + literal backslash (if not followed by newline?)
    // Wait, unquoted ends at newline.
    // If input is "KEY=value\\"
    // It sees backslash as last char.
    // Is it continuation?
    // "Inspect the last remaining character... If it IS \, remove... discard newline... Read Next Line"
    // Here we have input ends.
    // "Backslash at EOF: If \ is last char ... literal backslash"
    let input = "KEY=value\\";
    let entries = parse(input);
    let kv = entries[0].as_pair().unwrap();
    assert_eq!(kv.value, "value\\");
}

#[test]
fn test_consecutive_backslashes_escaped_continuation() {
    // Spec 5.2:
    // VALUE=text\\
    // more
    // "After processing text\\: ... last character is \ ... triggers continuation"
    // Result: text\more
    let input = "VALUE=text\\\\\nmore"; // source is text\\ (escaped backslash in string literal) + newline
    // Actually in rust string "text\\\\\nmore" -> bytes: t,e,x,t,\,,\,,\n,m,o,r,e
    // Parser reads: text\\
    // Last char is \.
    // But wait. "Backslash in Unquoted Values: ... backslashes are literal ... except when at line end."
    // So "text\\" -> last char is '\'.
    // Does it escape the following newline?
    // Strict order: 3. Check for Continuation Marker.
    // "Inspect last remaining character. If it IS \, remove... continuation."
    // So yes, it should continue. But what about the previous backslash?
    // It's just a char.
    // So `text\` remains (one backslash removed) + next line.
    // Result: `text\more`
    let entries = parse(input);
    let kv = entries[0].as_pair().unwrap();
    assert_eq!(kv.value, "text\\more");
}

#[test]
fn test_quote_immediately_after_equals() {
    // Spec 4.2.3/4.2.4
    let input = "KEY='value'";
    let entries = parse(input);
    let kv = entries[0].as_pair().unwrap();
    assert_eq!(kv.value, "value");
    assert_eq!(kv.quote, QuoteType::Single);
}

#[test]
fn test_space_before_quote_error() {
    // Spec 4.1.2: Whitespace forbidden after equals
    let input = "KEY= \"value\"";
    let entries = parse(input);
    match &entries[0] {
        Entry::Error(e) => assert!(e.to_string().contains("Whitespace not allowed after equals")),
         _ => panic!("Should be error"),
    }
}

// --- Additional Complex Spec Tests ---

#[test]
fn test_spec_3_1_utf8_bom_middle() {
    // Spec 3.1: BOM in middle is invalid.
    let input = "KEY=val\u{FEFF}ue";
    let entries = parse(input);
    match &entries[0] {
        Entry::Error(e) => assert!(e.to_string().contains("BOM") || e.to_string().contains("invalid")),
        _ => panic!("Should be error for BOM in middle"),
    }
}

#[test]
fn test_spec_4_1_1_export_lone_ignored() {
    // Spec 4.1.1: Export without definition should be ignored or error.
    // Our implementation ignores it (treats as empty key -> recover).
    let input = "export \nKEY=val";
    let entries = parse(input);
    assert_eq!(entries.len(), 1);
    let kv = entries[0].as_pair().unwrap();
    assert_eq!(kv.key, "KEY");
    assert_eq!(kv.value, "val");
}

#[test]
fn test_spec_4_2_2_unquoted_junk_ignored() {
    // Spec 4.2.2: Value ends at whitespace. Junk after whitespace is ignored (if not comment).
    // Implementation: parses until space. Then skips valid comment. Then `skip_to_newline` consumes junk.
    // This is "robust" parsing.
    let input = "KEY=val junk";
    let entries = parse(input);
    let kv = entries[0].as_pair().unwrap();
    assert_eq!(kv.value, "val");
}

#[test]
fn test_spec_5_2_continuation_with_backslash_in_comment() {
    // Spec 5.2: Comments processed BEFORE continuation check.
    // So a backslash INSIDE a comment does NOT trigger continuation.
    // But a backslash BEFORE a comment DOES.
    
    // Case 1: Backslash in comment (should NOT continue)
    let input = "KEY=val # comment with \\\nnext";
    let entries = parse(input);
    // Should be KEY=val. next is ignored or separate line?
    // "val" parses until space. "# comment..." is comment.
    // Line ends. `next` is... "next" is invalid line "next"? No, it's a key without equals. "next".
    // Wait, "next" is a key "next".
    // So result: KEY=val, Error(next)
    assert_eq!(entries.len(), 2);
    let kv = entries[0].as_pair().unwrap();
    assert_eq!(kv.value, "val"); // comment stripped
    match &entries[1] {
        Entry::Error(_) => {}, // Expected error for "next" without equals
        _ => panic!("Expected error for 'next'"),
    }
}

#[test]
fn test_spec_5_2_continuation_before_comment() {
    // Case 2: Backslash BEFORE comment (SHOULD continue)
    let input = "KEY=val \\ # comment\n  continued";
    let entries = parse(input);
    let kv = entries[0].as_pair().unwrap();
    assert_eq!(kv.value, "val   continued");
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
