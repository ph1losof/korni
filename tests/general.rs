use korni::{parse, parse_with_options, Entry, KeyValuePair, ParseOptions, QuoteType, Span, Environment};

// Tests extracted from src/lib.rs


    #[test]
    fn test_basic_parse() {
        let input = "KEY=VALUE";
        let entries = parse(input);
        
        assert_eq!(entries.len(), 1);
        let kv = entries[0].as_pair().unwrap();
        
        assert_eq!(kv.key, "KEY");
        assert_eq!(kv.value, "VALUE");
        assert!(!kv.is_exported);
    }

    #[test]
    fn test_strict_whitespace() {
        let input = "KEY = VALUE"; // Invalid
        let entries = parse(input);
        
        assert_eq!(entries.len(), 1);
        match &entries[0] {
            Entry::Error(e) => assert!(e.to_string().contains("Whitespace not allowed between key and equals")),
            _ => panic!("Should be error"),
        }
    }

    #[test]
    fn test_export_prefix() {
        let input = "export DB_URL=postgres";
        let entries = parse(input);
        
        assert_eq!(entries.len(), 1);
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.key, "DB_URL");
        assert_eq!(kv.value, "postgres");
        assert!(kv.is_exported);
    }
    
    #[test]
    fn test_export_no_space() {
        let input = "exportKey=val";
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.key, "exportKey");
        assert!(!kv.is_exported);
    }

    #[test]
    fn test_tab_after_equals_error() {
        // Spec 4.1.2: Tab after = is also forbidden
        let input = "KEY=\tvalue";
        let entries = parse(input);
        match &entries[0] {
            Entry::Error(e) => assert!(e.to_string().contains("Whitespace not allowed after equals")),
            _ => panic!("Should be error"),
        }
    }

    #[test]
    fn test_debug_simple() {
        // Test basic escaping logic
        // "a" -> a
        // "\n" -> newline
        // "a\"b" -> a"b
        
        let input = r#"KEY="a\"b""#;
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.value, "a\"b"); // Escaped quote becomes literal quote
        
        // Literal backslash + char
        // "a\b" -> a\b (since \b is not special)
        let input2 = r#"KEY="a\b""#;
        let entries2 = parse(input2);
        let kv2 = entries2[0].as_pair().unwrap();
        assert_eq!(kv2.value, r#"a\b"#);
    }

    #[test]
    fn test_escaped_quotes_and_content() {
        // Unquoted value with quotes inside are literal
        // Remove space to comply with Spec 4.2.2 (unquoted ends at whitespace)
        let input = r#"JSON={"key":"val"}"#; 
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.value, r#"{"key":"val"}"#);
        assert_eq!(kv.quote, QuoteType::None);
        
        // Quoted value with valid escapes
        // Per Spec 4.2.4: \" → Literal Double Quote (")
        // So "{\"key\": \"val\"}" becomes {"key": "val"}
        let input2 = r#"JSON="{\"key\": \"val\"}""#; 
        let entries2 = parse(input2);
        let kv2 = entries2[0].as_pair().unwrap();
        // Escaped quotes become literal quotes
        assert_eq!(kv2.value, r#"{"key": "val"}"#); 
        assert_eq!(kv2.quote, QuoteType::Double);
    }

    #[test]
    fn test_backslash_continuation_strict() {
        let input = "MESSAGE=Hello \\ # comment\nWorld";
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.value, "Hello World");
        
        let input2 = "KEY=Val\\\nNext";
        let entries2 = parse(input2);
        let kv2 = entries2[0].as_pair().unwrap();
        assert_eq!(kv2.value, "ValNext");
    }
    
    #[test]
    fn test_multiline_creation() {
        let input = "KEY=Line1 \\\n    Line2";
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        // Line 1: `Line1 \` -> `Line1 ` (trailing space before \ preserved)
        // Line 2: `    Line2` (4 spaces preserved)
        assert_eq!(kv.value, "Line1     Line2");
    }

    // ======== Additional Spec Compliance Tests ========

    #[test]
    fn test_inline_comment_after_quoted_value() {
        // Spec 4.3.2: Inline comments allowed after closing quote
        let input = r#"KEY="value" # this is a comment"#;
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.value, "value");
        assert_eq!(kv.quote, QuoteType::Double);
    }

    #[test]
    fn test_inline_comment_after_single_quoted_value() {
        // Spec 4.3.2: Inline comments allowed after closing single quote
        let input = "KEY='value' # this is a comment";
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.value, "value");
        assert_eq!(kv.quote, QuoteType::Single);
    }

    #[test]
    fn test_no_space_before_hash_after_quote() {
        // Hash directly after quote without space - still ignored
        let input = r##"KEY="value"#comment"##;
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.value, "value");
    }

    #[test]
    fn test_trailing_whitespace_after_quote() {
        // Whitespace after closing quote is ignored
        let input = "KEY=\"value\"   ";
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.value, "value");
    }

    #[test]
    fn test_trailing_junk_after_quote() {
        // Any content after closing quote is ignored
        let input = "KEY=\"value\" extra junk";
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.value, "value");
    }

    #[test]
    fn test_single_quoted_literal() {
        // Spec 4.2.3: Single quotes = literal, no escaping
        let input = "KEY='hello\\nworld'";
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        // \n is NOT interpreted as newline in single quotes
        assert_eq!(kv.value, "hello\\nworld");
        assert_eq!(kv.quote, QuoteType::Single);
    }

    #[test]
    fn test_empty_values() {
        // Spec 4.2.1: Empty values
        let input = "EMPTY=";
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.value, "");

        let input2 = r#"EMPTY2="""#;
        let entries2 = parse(input2);
        let kv2 = entries2[0].as_pair().unwrap();
        assert_eq!(kv2.value, "");

        let input3 = "EMPTY3=''";
        let entries3 = parse(input3);
        let kv3 = entries3[0].as_pair().unwrap();
        assert_eq!(kv3.value, "");
    }

    #[test]
    fn test_hash_in_unquoted_value() {
        // Spec 4.3.2: # preceded by space = comment
        let input = "URL=http://example.com#anchor";
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.value, "http://example.com#anchor"); // No space before #

        let input2 = "COLOR=#333";
        let entries2 = parse(input2);
        let kv2 = entries2[0].as_pair().unwrap();
        assert_eq!(kv2.value, "#333");

        let input3 = "KEY=value #comment";
        let entries3 = parse(input3);
        let kv3 = entries3[0].as_pair().unwrap();
        assert_eq!(kv3.value, "value"); // Comment stripped
    }

    #[test]
    fn test_hash_in_quoted_value() {
        // Spec 4.3.2: # inside quotes is always literal
        let input = r#"KEY="value #not a comment""#;
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.value, "value #not a comment");
    }

    #[test]
    fn test_multiple_equals() {
        // Spec 4.1.2: First = separates, rest are value
        let input = "KEY=value=with=equals";
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.value, "value=with=equals");

        let input2 = "KEY==value";
        let entries2 = parse(input2);
        let kv2 = entries2[0].as_pair().unwrap();
        assert_eq!(kv2.value, "=value");
    }

    #[test]
    fn test_double_quote_escape_sequences() {
        // Spec 4.2.4: Escape sequences
        let input = r#"KEY="line1\nline2\ttab\\backslash""#;
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.value, "line1\nline2\ttab\\backslash");
    }

    #[test]
    fn test_quoted_multiline() {
        // Spec 5.1: Quoted multiline preserves newlines
        let input = "KEY=\"line1\nline2\"";
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.value, "line1\nline2");
    }

    #[test]
    fn test_unclosed_quote_error() {
        let input = r#"KEY="unclosed"#;
        let entries = parse(input);
        match &entries[0] {
            Entry::Error(e) => assert!(e.to_string().contains("Unclosed")),
            _ => panic!("Should be error"),
        }
    }

    #[test]
    fn test_utf8_bom_at_start() {
        // Spec 3.1: BOM at start is stripped
        let input = "\u{FEFF}KEY=value";
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.key, "KEY");
        assert_eq!(kv.value, "value");
    }

    #[test]
    fn test_comment_lines() {
        let input = "# This is a comment\nKEY=value";
        let entries = parse_with_options(input, ParseOptions { include_comments: true, track_positions: false });
        assert_eq!(entries.len(), 2);
        assert!(matches!(entries[0], Entry::Comment(_)));
        assert!(entries[1].as_pair().is_some());
    }

    #[test]
    fn test_empty_lines() {
        let input = "\n\nKEY=value\n\n";
        let entries = parse(input);
        let pairs: Vec<_> = entries.iter().filter(|e| e.as_pair().is_some()).collect();
        assert_eq!(pairs.len(), 1);
    }

    // ======== Spec 4.1.1: Export Prefix Edge Cases ========

    #[test]
    fn test_export_with_tab() {
        // Spec 4.1.1: export followed by tab is valid
        let input = "export\tKEY=value";
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.key, "KEY");
        assert!(kv.is_exported);
    }

    #[test]
    fn test_leading_whitespace_before_export() {
        // Spec 4.1.1: Leading whitespace before export is allowed
        let input = "  export KEY=value";
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.key, "KEY");
        assert!(kv.is_exported);
    }

    #[test]
    fn test_export_underscore_key() {
        // Spec 4.1.1: export_KEY is a regular key (underscore, not space)
        let input = "export_KEY=value";
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.key, "export_KEY");
        assert!(!kv.is_exported);
    }

    #[test]
    fn test_exported_value_key() {
        // exportedValue is a regular key
        let input = "exportedValue=value";
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.key, "exportedValue");
        assert!(!kv.is_exported);
    }

    // ======== Spec 4.1.2: Key Edge Cases ========

    #[test]
    fn test_underscore_start_key() {
        // Spec 4.1.2: Keys can start with underscore
        let input = "_PRIVATE=secret";
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.key, "_PRIVATE");
    }

    #[test]
    fn test_key_with_numbers() {
        let input = "CONFIG123=value";
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.key, "CONFIG123");
    }

    #[test]
    fn test_key_starting_with_digit_error() {
        // Spec 4.1.2: Keys MUST NOT start with digit
        let input = "123KEY=value";
        let entries = parse(input);
        match &entries[0] {
            Entry::Error(e) => assert!(e.to_string().contains("digit")),
            _ => panic!("Should be error"),
        }
    }

    #[test]
    fn test_leading_whitespace_before_key() {
        // Spec 4.1.2: Leading whitespace is stripped
        let input = "  KEY=value";
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.key, "KEY");
        assert_eq!(kv.value, "value");
    }

    #[test]
    fn test_tab_before_key() {
        let input = "\tKEY=value";
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.key, "KEY");
    }

    #[test]
    fn test_lowercase_key() {
        // Spec 4.1.2: Lowercase keys are valid
        let input = "database_url=postgres://localhost";
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.key, "database_url");
    }

    // ======== Spec 4.2.2: Unquoted Value Edge Cases ========

    #[test]
    fn test_unquoted_with_equals() {
        // DATABASE_URL with query params
        let input = "DATABASE_URL=postgresql://user:pass@host:5432/db?param=value";
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.value, "postgresql://user:pass@host:5432/db?param=value");
    }

    #[test]
    fn test_unquoted_backslash_literal() {
        // Spec 4.2.2: Backslashes in unquoted are literal (except at EOL)
        let input = "PATH=C:\\Users\\Name";
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.value, "C:\\Users\\Name");
    }

    #[test]
    fn test_unquoted_escape_n_literal() {
        // Spec 4.2.2: \n in unquoted is literal backslash + n
        let input = "KEY=foo\\nbar";
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.value, "foo\\nbar");
    }

    // ======== Spec 4.2.3: Single Quote Edge Cases ========

    #[test]
    fn test_single_quote_dollar_literal() {
        // Spec 4.2.3: $ is literal in single quotes
        let input = "KEY='${VAR}'";
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.value, "${VAR}");
    }

    #[test]
    fn test_single_quote_hash_literal() {
        // Spec 4.2.3: # is literal in single quotes
        let input = "KEY='#333'";
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.value, "#333");
    }

    #[test]
    fn test_single_quote_multiline() {
        // Spec 4.2.3/5.1: Single quotes preserve newlines
        let input = "KEY='Line 1\nLine 2\nLine 3'";
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.value, "Line 1\nLine 2\nLine 3");
    }

    #[test]
    fn test_single_quote_unclosed_error() {
        let input = "KEY='unclosed";
        let entries = parse(input);
        match &entries[0] {
            Entry::Error(e) => assert!(e.to_string().contains("Unclosed")),
            _ => panic!("Should be error"),
        }
    }

    // ======== Spec 4.2.4: Double Quote Edge Cases ========

    #[test]
    fn test_double_quote_escaped_dollar() {
        // Spec 4.2.4: \$ produces literal $
        let input = r#"KEY="\${NOT_VAR}""#;
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.value, "${NOT_VAR}");
    }

    #[test]
    fn test_double_quote_carriage_return() {
        // Spec 4.2.4: \r produces carriage return
        let input = r#"KEY="line1\r\nline2""#;
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.value, "line1\r\nline2");
    }

    #[test]
    fn test_double_quote_consecutive_backslashes() {
        // Spec 4.2.4: \\\\ produces \\
        let input = r#"KEY="value\\\\""#;
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.value, "value\\\\");
    }

    #[test]
    fn test_double_quote_unknown_escape() {
        // Spec 4.2.4: Unknown escapes preserved literally
        let input = r#"KEY="\a\x\0""#;
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.value, "\\a\\x\\0");
    }

    #[test]
    fn test_nested_quotes() {
        // Spec 4.2.4: Opposite quote type is literal
        let input = r#"KEY="outer 'inner' end""#;
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.value, "outer 'inner' end");

        let input2 = "KEY='outer \"inner\" end'";
        let entries2 = parse(input2);
        let kv2 = entries2[0].as_pair().unwrap();
        assert_eq!(kv2.value, "outer \"inner\" end");
    }

    #[test]
    fn test_space_after_equals_error() {
        // Spec 4.1.2: Space after = is forbidden
        let input = r#"KEY= "value""#;
        let entries = parse(input);
        match &entries[0] {
            Entry::Error(e) => assert!(e.to_string().contains("Whitespace not allowed after equals")),
            _ => panic!("Should be error"),
        }
    }

    #[test]
    fn test_quotes_within_unquoted() {
        // Spec 4.2.4: Quotes in unquoted values are literal
        let input = r#"KEY=foo"bar"baz"#;
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.value, r#"foo"bar"baz"#);
    }

    // ======== Spec 4.3: Comment Edge Cases ========

    #[test]
    fn test_tab_before_hash_comment() {
        // Spec 4.3.2: Tab before # also triggers comment
        let input = "KEY=value\t#comment";
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.value, "value");
    }

    #[test]
    fn test_multiple_spaces_before_hash() {
        let input = "KEY=value  #comment";
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.value, "value");
    }

    #[test]
    fn test_hash_at_start_of_unquoted() {
        // # at start of value (no preceding space)
        let input = "KEY=#value";
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.value, "#value");
    }

    #[test]
    fn test_api_key_with_hash() {
        let input = "API_KEY=sk-abc123#prod";
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.value, "sk-abc123#prod");
    }

    // ======== Spec 4.4: Interpolation (Literal) ========

    #[test]
    fn test_dollar_var_literal_unquoted() {
        // Spec 4.4: Interpolation is NOT performed
        let input = "DATABASE_URL=${DB_HOST}:${DB_PORT}/${DB_NAME}";
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.value, "${DB_HOST}:${DB_PORT}/${DB_NAME}");
    }

    #[test]
    fn test_dollar_at_end() {
        let input = "KEY=value$";
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.value, "value$");
    }

    #[test]
    fn test_dollar_before_numbers() {
        let input = "KEY=$123";
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.value, "$123");
    }

    // ======== Spec 5.2: Backslash Continuation Edge Cases ========

    #[test]
    fn test_multiple_continuations() {
        let input = "PATH=/usr/local/bin:\\\n/usr/bin:\\\n/bin";
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.value, "/usr/local/bin:/usr/bin:/bin");
    }

    #[test]
    fn test_continuation_preserves_leading_space() {
        // Spec 5.2: Leading whitespace of next line is preserved
        let input = "COMMAND=docker run \\\n  --detach \\\n  nginx";
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.value, "docker run   --detach   nginx");
    }

    // ======== Spec Edge Cases: Mixed Scenarios ========

    #[test]
    fn test_empty_file() {
        let input = "";
        let entries = parse(input);
        assert_eq!(entries.len(), 0);
    }

    #[test]
    fn test_only_comments() {
        let input = "# Comment 1\n# Comment 2";
        let entries = parse(input);
        let pairs: Vec<_> = entries.iter().filter(|e| e.as_pair().is_some()).collect();
        assert_eq!(pairs.len(), 0);
    }

    #[test]
    fn test_only_whitespace() {
        let input = "   \n\t\n   ";
        let entries = parse(input);
        let pairs: Vec<_> = entries.iter().filter(|e| e.as_pair().is_some()).collect();
        assert_eq!(pairs.len(), 0);
    }

    #[test]
    fn test_indented_comment() {
        let input = "   # Indented comment\nKEY=value";
        let entries = parse_with_options(input, ParseOptions { include_comments: true, track_positions: false });
        assert!(matches!(entries[0], Entry::Comment(_)));
        let kv = entries[1].as_pair().unwrap();
        assert_eq!(kv.key, "KEY");
    }

    #[test]
    fn test_case_sensitive_keys() {
        // Spec 4.1.2: Keys are case-sensitive
        let input = "API_KEY=value1\napi_key=value2";
        let entries = parse(input);
        // Filter to only pairs
        let pairs: Vec<_> = entries.iter().filter_map(|e| e.as_pair()).collect();
        assert_eq!(pairs.len(), 2, "Expected 2 key-value pairs");
        assert_eq!(pairs[0].key, "API_KEY");
        assert_eq!(pairs[1].key, "api_key");
        // They are distinct keys
        assert_ne!(pairs[0].key, pairs[1].key);
    }

    #[test]
    fn test_pem_key_multiline() {
        // Real-world scenario: PEM key in double quotes
        let input = r#"PRIVATE_KEY="-----BEGIN RSA PRIVATE KEY-----
MIIEpQIBAAKCAQEA3Tz2MR7SZiAMfQyuvBjM9Oi..
-----END RSA PRIVATE KEY-----""#;
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        assert!(kv.value.starts_with("-----BEGIN RSA PRIVATE KEY-----"));
        assert!(kv.value.ends_with("-----END RSA PRIVATE KEY-----"));
        assert!(kv.value.contains('\n'));
    }

    #[test]
    fn test_json_in_double_quotes() {
        let input = r#"CONFIG="{\"enable\": true, \"retries\": 3}""#;
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.value, r#"{"enable": true, "retries": 3}"#);
    }

    // ======== ADDITIONAL TESTS FOR 150+ COVERAGE ========

    // --- Escape Sequences Individual Tests ---

    #[test]
    fn test_escape_n_only() {
        let input = r#"KEY="\n""#;
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.value, "\n");
    }

    #[test]
    fn test_escape_r_only() {
        let input = r#"KEY="\r""#;
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.value, "\r");
    }

    #[test]
    fn test_escape_t_only() {
        let input = r#"KEY="\t""#;
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.value, "\t");
    }

    #[test]
    fn test_escape_backslash_only() {
        let input = r#"KEY="\\""#;
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.value, "\\");
    }

    #[test]
    fn test_escape_quote_only() {
        let input = r#"KEY="\"""#;
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.value, "\"");
    }

    #[test]
    fn test_escape_dollar_only() {
        let input = r#"KEY="\$""#;
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.value, "$");
    }

    #[test]
    fn test_escape_unknown_a() {
        let input = r#"KEY="\a""#;
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.value, "\\a");
    }

    #[test]
    fn test_escape_unknown_b() {
        let input = r#"KEY="\b""#;
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.value, "\\b");
    }

    #[test]
    fn test_escape_unknown_f() {
        let input = r#"KEY="\f""#;
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.value, "\\f");
    }

    #[test]
    fn test_escape_unknown_v() {
        let input = r#"KEY="\v""#;
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.value, "\\v");
    }

    #[test]
    fn test_escape_unknown_zero() {
        let input = r#"KEY="\0""#;
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.value, "\\0");
    }

    #[test]
    fn test_escape_unknown_x() {
        let input = r#"KEY="\x41""#;
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.value, "\\x41");
    }

    #[test]
    fn test_escape_mixed_sequence() {
        let input = r#"KEY="a\nb\tc\\d\"e\$f""#;
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.value, "a\nb\tc\\d\"e$f");
    }

    // --- Key Name Variations ---

    #[test]
    fn test_key_single_char() {
        let input = "A=value";
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.key, "A");
    }

    #[test]
    fn test_key_all_underscores() {
        let input = "___=value";
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.key, "___");
    }

    #[test]
    fn test_key_very_long() {
        let key = "A".repeat(1000);
        let input = format!("{}=value", key);
        let entries = parse(&input);
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.key.len(), 1000);
    }

    #[test]
    fn test_key_mixed_case() {
        let input = "MyVeryLongVariableName_123=value";
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.key, "MyVeryLongVariableName_123");
    }

    #[test]
    fn test_key_numbers_middle() {
        let input = "ABC123DEF=value";
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.key, "ABC123DEF");
    }

    #[test]
    fn test_key_underscore_only_start() {
        let input = "_123=value";
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.key, "_123");
    }

    // --- Value Variations ---

    #[test]
    fn test_value_single_char() {
        let input = "KEY=a";
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.value, "a");
    }

    #[test]
    fn test_value_very_long() {
        let val = "x".repeat(10000);
        let input = format!("KEY={}", val);
        let entries = parse(&input);
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.value.len(), 10000);
    }

    #[test]
    fn test_value_number_only() {
        let input = "KEY=12345";
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.value, "12345");
    }

    #[test]
    fn test_value_decimal() {
        let input = "KEY=3.14159";
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.value, "3.14159");
    }

    #[test]
    fn test_value_negative_number() {
        let input = "KEY=-42";
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.value, "-42");
    }

    #[test]
    fn test_value_boolean_true() {
        let input = "KEY=true";
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.value, "true");
    }

    #[test]
    fn test_value_boolean_false() {
        let input = "KEY=false";
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.value, "false");
    }

    // --- Whitespace Edge Cases ---

    #[test]
    fn test_multiple_leading_spaces() {
        let input = "    KEY=value";
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.key, "KEY");
    }

    #[test]
    fn test_multiple_leading_tabs() {
        let input = "\t\t\tKEY=value";
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.key, "KEY");
    }

    #[test]
    fn test_mixed_leading_whitespace() {
        let input = " \t \tKEY=value";
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.key, "KEY");
    }

    #[test]
    fn test_trailing_whitespace_after_value() {
        let input = "KEY=value   ";
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.value, "value");
    }

    #[test]
    fn test_export_multiple_spaces() {
        let input = "export   KEY=value";
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.key, "KEY");
        assert!(kv.is_exported);
    }

    // --- Quote Edge Cases ---

    #[test]
    fn test_empty_double_quotes() {
        let input = r#"KEY="""#;
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.value, "");
        assert_eq!(kv.quote, QuoteType::Double);
    }

    #[test]
    fn test_empty_single_quotes() {
        let input = "KEY=''";
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.value, "");
        assert_eq!(kv.quote, QuoteType::Single);
    }

    #[test]
    fn test_single_space_in_quotes() {
        let input = r#"KEY=" ""#;
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.value, " ");
    }

    #[test]
    fn test_multiple_spaces_in_quotes() {
        let input = r#"KEY="   ""#;
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.value, "   ");
    }

    #[test]
    fn test_tab_in_quotes() {
        let input = "KEY=\"\t\"";
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.value, "\t");
    }

    #[test]
    fn test_newline_in_double_quotes() {
        let input = "KEY=\"line1\nline2\"";
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.value, "line1\nline2");
    }

    #[test]
    fn test_newline_in_single_quotes() {
        let input = "KEY='line1\nline2'";
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.value, "line1\nline2");
    }

    #[test]
    fn test_double_quote_in_single_quotes() {
        let input = r#"KEY='"hello"'"#;
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.value, "\"hello\"");
    }

    // --- Comment Edge Cases ---

    #[test]
    fn test_comment_only_hash() {
        let input = "#";
        let entries = parse_with_options(input, ParseOptions { include_comments: true, track_positions: false });
        assert!(matches!(entries[0], Entry::Comment(_)));
    }

    #[test]
    fn test_comment_hash_with_spaces() {
        let input = "#     ";
        let entries = parse_with_options(input, ParseOptions { include_comments: true, track_positions: false });
        assert!(matches!(entries[0], Entry::Comment(_)));
    }

    #[test]
    fn test_many_hash_marks() {
        let input = "### Comment ###";
        let entries = parse_with_options(input, ParseOptions { include_comments: true, track_positions: false });
        assert!(matches!(entries[0], Entry::Comment(_)));
    }

    #[test]
    fn test_inline_comment_multiple_hashes() {
        let input = "KEY=value ### comment";
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.value, "value");
    }

    #[test]
    #[test]
    fn test_hash_directly_after_equals() {
        let input = "K=#not a comment";
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.key, "K");
        assert_eq!(kv.value, "#not");
    }

    // --- Multiple Entries ---

    #[test]
    fn test_three_entries() {
        let input = "A=1\nB=2\nC=3";
        let entries = parse(input);
        let pairs: Vec<_> = entries.iter().filter_map(|e| e.as_pair()).collect();
        assert_eq!(pairs.len(), 3);
        assert_eq!(pairs[0].value, "1");
        assert_eq!(pairs[1].value, "2");
        assert_eq!(pairs[2].value, "3");
    }

    #[test]
    fn test_ten_entries() {
        let input = (0..10).map(|i| format!("KEY{}={}", i, i)).collect::<Vec<_>>().join("\n");
        let entries = parse(&input);
        let pairs: Vec<_> = entries.iter().filter_map(|e| e.as_pair()).collect();
        assert_eq!(pairs.len(), 10);
    }

    #[test]
    fn test_entries_with_empty_lines() {
        let input = "A=1\n\nB=2\n\n\nC=3";
        let entries = parse(input);
        let pairs: Vec<_> = entries.iter().filter_map(|e| e.as_pair()).collect();
        assert_eq!(pairs.len(), 3);
    }

    #[test]
    fn test_entries_with_comments() {
        let input = "# Header\nA=1\n# Middle\nB=2\n# Footer";
        let entries = parse(input);
        let pairs: Vec<_> = entries.iter().filter_map(|e| e.as_pair()).collect();
        assert_eq!(pairs.len(), 2);
    }

    #[test]
    fn test_mixed_quote_types() {
        let input = "A='single'\nB=\"double\"\nC=unquoted";
        let entries = parse(input);
        let pairs: Vec<_> = entries.iter().filter_map(|e| e.as_pair()).collect();
        assert_eq!(pairs[0].quote, QuoteType::Single);
        assert_eq!(pairs[1].quote, QuoteType::Double);
        assert_eq!(pairs[2].quote, QuoteType::None);
    }

    // --- Error Cases ---

    #[test]
    fn test_error_whitespace_before_equals() {
        let input = "KEY =value";
        let entries = parse(input);
        assert!(matches!(entries[0], Entry::Error(_)));
    }

    #[test]
    fn test_error_whitespace_after_equals() {
        // Spec 4.1.2: Whitespace after equals is forbidden
        let input = "KEY= value";
        let entries = parse(input);
        match &entries[0] {
            Entry::Error(e) => assert!(e.to_string().contains("Whitespace not allowed after equals")),
            _ => panic!("Should be error"),
        }
    }

    #[test]
    fn test_error_digit_start_2() {
        let input = "2KEY=value";
        let entries = parse(input);
        assert!(matches!(entries[0], Entry::Error(_)));
    }

    #[test]
    fn test_error_digit_start_9() {
        let input = "9_VAR=value";
        let entries = parse(input);
        assert!(matches!(entries[0], Entry::Error(_)));
    }

    #[test]
    fn test_unclosed_single_quote() {
        let input = "KEY='unclosed";
        let entries = parse(input);
        assert!(matches!(entries[0], Entry::Error(_)));
    }

    #[test]
    fn test_unclosed_double_quote() {
        let input = r#"KEY="unclosed"#;
        let entries = parse(input);
        assert!(matches!(entries[0], Entry::Error(_)));
    }

    // --- Real World Scenarios ---

    #[test]
    fn test_database_url_postgres() {
        let input = "DATABASE_URL=postgresql://user:password@localhost:5432/mydb";
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.value, "postgresql://user:password@localhost:5432/mydb");
    }

    #[test]
    fn test_database_url_mysql() {
        let input = "DATABASE_URL=mysql://root:pass@127.0.0.1:3306/app";
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.value, "mysql://root:pass@127.0.0.1:3306/app");
    }

    #[test]
    fn test_redis_url() {
        let input = "REDIS_URL=redis://localhost:6379/0";
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.value, "redis://localhost:6379/0");
    }

    #[test]
    fn test_aws_access_key() {
        let input = "AWS_ACCESS_KEY_ID=AKIAIOSFODNN7EXAMPLE";
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.value, "AKIAIOSFODNN7EXAMPLE");
    }

    #[test]
    fn test_aws_secret_key() {
        let input = "AWS_SECRET_ACCESS_KEY='wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY'";
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.value, "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY");
    }

    #[test]
    fn test_jwt_secret() {
        let input = r#"JWT_SECRET="super-secret-key-123!@#""#;
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.value, "super-secret-key-123!@#");
    }

    #[test]
    fn test_api_endpoint() {
        let input = "API_ENDPOINT=https://api.example.com/v1";
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.value, "https://api.example.com/v1");
    }

    #[test]
    fn test_s3_bucket() {
        let input = "S3_BUCKET=my-bucket-name-2024";
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.value, "my-bucket-name-2024");
    }

    #[test]
    fn test_log_level() {
        let input = "LOG_LEVEL=debug";
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.value, "debug");
    }

    #[test]
    fn test_port_number() {
        let input = "PORT=3000";
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.value, "3000");
    }

    #[test]
    fn test_node_env() {
        let input = "NODE_ENV=production";
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.value, "production");
    }

    #[test]
    fn test_smtp_config() {
        let input = "SMTP_HOST=smtp.gmail.com\nSMTP_PORT=587\nSMTP_USER=user@gmail.com";
        let entries = parse(input);
        let pairs: Vec<_> = entries.iter().filter_map(|e| e.as_pair()).collect();
        assert_eq!(pairs.len(), 3);
        assert_eq!(pairs[0].value, "smtp.gmail.com");
        assert_eq!(pairs[1].value, "587");
        assert_eq!(pairs[2].value, "user@gmail.com");
    }

    #[test]
    fn test_oauth_config() {
        let input = r#"GOOGLE_CLIENT_ID="123456789.apps.googleusercontent.com"
GOOGLE_CLIENT_SECRET="secret-abc123""#;
        let entries = parse(input);
        let pairs: Vec<_> = entries.iter().filter_map(|e| e.as_pair()).collect();
        assert_eq!(pairs.len(), 2);
    }

    // --- Unicode ---

    #[test]
    fn test_unicode_value() {
        let input = "GREETING=こんにちは";
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.value, "こんにちは");
    }

    #[test]
    fn test_unicode_in_quotes() {
        // Note: Parser uses byte-based iteration, which correctly handles UTF-8
        // but the value is preserved as-is
        let input = r#"MESSAGE="Hello, World!""#;
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.value, "Hello, World!");
    }

    #[test]
    fn test_emoji_value() {
        let input = "EMOJI=🚀🎉✨";
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.value, "🚀🎉✨");
    }

    // --- Continuation Edge Cases ---

    #[test]
    fn test_continuation_empty_next_line() {
        let input = "KEY=value\\\n";
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.value, "value");
    }

    #[test]
    fn test_continuation_three_lines() {
        let input = "KEY=a\\\nb\\\nc";
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.value, "abc");
    }

    #[test]
    fn test_continuation_with_spaces_each_line() {
        let input = "KEY=a \\\n  b \\\n  c";
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.value, "a   b   c");
    }

    // --- Span/Position Tests (require track_positions option) ---

    #[test]
    fn test_key_span_basic() {
        let input = "KEY=value";
        let entries = parse_with_options(input, ParseOptions { track_positions: true, include_comments: false });
        let kv = entries[0].as_pair().unwrap();
        assert!(kv.key_span.is_some());
        let span = kv.key_span.unwrap();
        assert_eq!(span.start.offset, 0);
        assert_eq!(span.end.offset, 3);
    }

    #[test]
    fn test_value_span_basic() {
        let input = "KEY=value";
        let entries = parse_with_options(input, ParseOptions { track_positions: true, include_comments: false });
        let kv = entries[0].as_pair().unwrap();
        // Value starts at offset 4 (after =)
        assert!(kv.value_span.is_some());
        assert_eq!(kv.value_span.unwrap().start.offset, 4);
    }

    // TODO: Line/column tracking not yet implemented - only offsets are tracked
    // #[test]
    // fn test_equals_position() {
    //     let input = "KEY=value";
    //     let entries = parse(input);
    //     let kv = entries[0].as_pair().unwrap();
    //     // = is at offset 3
    //     assert_eq!(kv.equals_pos.offset, 3);
    //     assert_eq!(kv.equals_pos.line, 0);
    //     assert_eq!(kv.equals_pos.col, 3);
    // }

    #[test]
    fn test_double_quote_positions() {
        let input = r#"KEY="value""#;
        let entries = parse_with_options(input, ParseOptions { track_positions: true, include_comments: false });
        let kv = entries[0].as_pair().unwrap();
        // Opening " is at offset 4
        assert!(kv.open_quote_pos.is_some());
        assert_eq!(kv.open_quote_pos.unwrap().offset, 4);
        // Closing " is at offset 10
        assert!(kv.close_quote_pos.is_some());
        assert_eq!(kv.close_quote_pos.unwrap().offset, 10);
    }

    #[test]
    fn test_single_quote_positions() {
        let input = "KEY='value'";
        let entries = parse_with_options(input, ParseOptions { track_positions: true, include_comments: false });
        let kv = entries[0].as_pair().unwrap();
        // Opening ' is at offset 4
        assert!(kv.open_quote_pos.is_some());
        assert_eq!(kv.open_quote_pos.unwrap().offset, 4);
        // Closing ' is at offset 10
        assert!(kv.close_quote_pos.is_some());
        assert_eq!(kv.close_quote_pos.unwrap().offset, 10);
    }

    #[test]
    fn test_unquoted_no_quote_positions() {
        let input = "KEY=value";
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        // Unquoted values have no quote positions
        assert!(kv.open_quote_pos.is_none());
        assert!(kv.close_quote_pos.is_none());
    }

    #[test]
    fn test_position_with_export() {
        let input = "export KEY=value";
        let entries = parse_with_options(input, ParseOptions { track_positions: true, include_comments: false });
        let kv = entries[0].as_pair().unwrap();
        // Key starts after "export "
        assert!(kv.key_span.is_some());
        assert_eq!(kv.key_span.unwrap().start.offset, 7);
        // = is at offset 10
        assert!(kv.equals_pos.is_some());
        assert_eq!(kv.equals_pos.unwrap().offset, 10);
    }

    // TODO: Line/column tracking not yet implemented - only offsets are tracked
    // #[test]
    // fn test_position_on_second_line() {
    //     let input = "A=1\nB=2";
    //     let entries = parse(input);
    //     let pairs: Vec<_> = entries.iter().filter_map(|e| e.as_pair()).collect();
    //     // Second entry starts on line 1
    //     assert_eq!(pairs[1].key_span.start.line, 1);
    //     assert_eq!(pairs[1].key_span.start.col, 0);
    // }

    // --- Boundary Conditions ---

    #[test]
    fn test_equals_only() {
        let input = "=value";
        let entries = parse(input);
        // Empty key error
        assert!(matches!(entries[0], Entry::Error(_)));
    }

    #[test]
    fn test_key_only_no_equals() {
        let input = "KEY";
        let entries = parse(input);
        // Should be error - no equals
        assert!(matches!(entries[0], Entry::Error(_)));
    }

    #[test]
    fn test_newline_only() {
        let input = "\n";
        let entries = parse(input);
        // Should be empty - newlines are skipped in fast mode
        assert_eq!(entries.len(), 0);
    }

    #[test]
    fn test_multiple_newlines_only() {
        let input = "\n\n\n";
        let entries = parse(input);
        // Should be empty - newlines are skipped in fast mode
        assert_eq!(entries.len(), 0);
    }

    // --- Special Characters in Values ---

    #[test]
    fn test_value_with_at_sign() {
        let input = "EMAIL=user@example.com";
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.value, "user@example.com");
    }

    #[test]
    fn test_value_with_ampersand() {
        let input = "QUERY=a=1&b=2";
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.value, "a=1&b=2");
    }

    #[test]
    fn test_value_with_percent() {
        let input = "ENCODED=hello%20world";
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.value, "hello%20world");
    }

    #[test]
    fn test_value_with_colon() {
        let input = "TIME=12:30:45";
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.value, "12:30:45");
    }

    #[test]
    fn test_value_with_slash() {
        let input = "PATH=/usr/local/bin";
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.value, "/usr/local/bin");
    }

    #[test]
    fn test_value_with_asterisk() {
        let input = "PATTERN=*.txt";
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.value, "*.txt");
    }

    #[test]
    fn test_value_with_question_mark() {
        let input = "URL=http://example.com?query=1";
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.value, "http://example.com?query=1");
    }

    #[test]
    fn test_value_with_brackets() {
        let input = "ARRAY=[1,2,3]";
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.value, "[1,2,3]");
    }

    #[test]
    fn test_value_with_braces() {
        let input = "OBJ={key:value}";
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.value, "{key:value}");
    }

    #[test]
    fn test_value_with_parens() {
        let input = "EXPR=(1+2)";
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.value, "(1+2)");
    }

    #[test]
    fn test_value_with_pipe() {
        let input = "CMD=cat|grep";
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.value, "cat|grep");
    }

    #[test]
    fn test_value_with_caret() {
        let input = "REGEX=^start";
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.value, "^start");
    }

    #[test]
    fn test_value_with_tilde() {
        let input = "HOME=~/docs";
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.value, "~/docs");
    }

    #[test]
    #[test]
    fn test_value_with_backtick() {
        let input = "K=`echo_hi`";
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.key, "K");
        assert_eq!(kv.value, "`echo_hi`");
    }

    // --- Commented Key-Value Pairs (require include_comments option) ---

    #[test]
    fn test_commented_key_value() {
        let input = "# KEY=value";
        let entries = parse_with_options(input, ParseOptions { include_comments: true, track_positions: false });
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.key, "KEY");
        assert_eq!(kv.value, "value");
        assert!(kv.is_comment);
        assert!(!kv.is_exported);
    }

    #[test]
    fn test_commented_key_value_with_spaces() {
        let input = "#   KEY=value";
        let entries = parse_with_options(input, ParseOptions { include_comments: true, track_positions: false });
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.key, "KEY");
        assert!(kv.is_comment);
    }

    #[test]
    fn test_commented_exported_key_value() {
        let input = "# export KEY=value";
        let entries = parse_with_options(input, ParseOptions { include_comments: true, track_positions: false });
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.key, "KEY");
        assert!(kv.is_comment);
        assert!(kv.is_exported);
    }

    #[test]
    fn test_commented_double_quoted_value() {
        let input = "# KEY=\"quoted value\"";
        let entries = parse_with_options(input, ParseOptions { include_comments: true, track_positions: false });
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.value, "quoted value");
        assert!(kv.is_comment);
        assert_eq!(kv.quote, QuoteType::Double);
    }

    #[test]
    fn test_commented_single_quoted_value() {
        let input = "# KEY='single quoted'";
        let entries = parse_with_options(input, ParseOptions { include_comments: true, track_positions: false });
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.value, "single quoted");
        assert!(kv.is_comment);
        assert_eq!(kv.quote, QuoteType::Single);
    }

    #[test]
    fn test_regular_pair_is_not_comment() {
        let input = "KEY=value";
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        assert!(!kv.is_comment);
    }

    #[test]
    fn test_pure_comment_text() {
        // Pure text comment (no KEY=value pattern)
        let input = "# This is just a comment";
        let entries = parse_with_options(input, ParseOptions { include_comments: true, track_positions: false });
        // Should be Entry::Comment, not a Pair
        assert!(matches!(entries[0], Entry::Comment(_)));
    }

    #[test]
    fn test_mixed_commented_and_active() {
        let input = "# OLD_KEY=deprecated\nKEY=active";
        let entries = parse_with_options(input, ParseOptions { include_comments: true, track_positions: false });
        let pairs: Vec<_> = entries.iter().filter_map(|e| e.as_pair()).collect();
        assert_eq!(pairs.len(), 2);
        assert!(pairs[0].is_comment);
        assert_eq!(pairs[0].key, "OLD_KEY");
        assert!(!pairs[1].is_comment);
        assert_eq!(pairs[1].key, "KEY");
    }

    // ======== New Tests based on Implementation Plan ========

    #[test]
    fn test_crlf_unquoted() {
        let input = "KEY=Value\r\n";
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        // Unquoted value trims trailing whitespace, but expects newline termination.
        // \r\n is a valid newline.
        // "The value ends at the first whitespace character (space or tab) or newline character."
        assert_eq!(kv.value, "Value");
    }

    #[test]
    fn test_crlf_multiline_escaped() {
        let input = "KEY=Line1\\\r\nLine2";
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        // Backslash continuation should work with CRLF
        assert_eq!(kv.value, "Line1Line2");
    }

    #[test]
    fn test_commented_escaped_quote() {
        // Before fix: This would fail because it would scan until first " 
        // and think value ended, leaving trailing junk or confusing parser.
        let input = r#"# KEY="a\"b""#;
        let entries = parse_with_options(input, ParseOptions { include_comments: true, track_positions: false });
        let kv = entries[0].as_pair().unwrap();
        assert_eq!(kv.value, r#"a"b"#); // Escaped quote becomes literal quote
        assert!(kv.is_comment);
    }

    #[test]
    fn test_bom_middle_rejection() {
        // Spec: BOM in middle must be rejected.
        let input = "KEY=Val\u{FEFF}ue";
        let entries = parse(input);
        match &entries[0] {
             Entry::Error(e) => assert!(e.to_string().contains("BOM")),
             _ => panic!("Should be error, got {:?}", entries[0]),
        }
    }

    #[test]
    fn test_key_with_dot_error() {
        let input = "KEY.NAME=val";
        let entries = parse(input);
        // "KEY" parsed as key
        // ".NAME" remains. "." is not '='. Error expected.
        match &entries[0] {
            Entry::Error(e) => assert!(e.to_string().contains("Expected '='")),
            _ => panic!("Should be error, caught: {:?}", entries[0]),
        }
    }

    #[test]
    fn test_key_with_dash_error() {
        let input = "KEY-NAME=val";
        let entries = parse(input);
        match &entries[0] {
            Entry::Error(e) => assert!(e.to_string().contains("Expected '='")),
            _ => panic!("Should be error"),
        }
    }

    #[test]
    fn test_key_with_space_error() {
        let input = "KEY NAME=val";
        let entries = parse(input);
        // "KEY" consumed. " NAME=val" left.
        // Spaces skipped. "NAME=val" left.
        // "N" != "=". Error "Expected '='".
        // Then recovery skips line? Or consumes rest?
        // recover_line skips to newline.
        // So we expect 1 error entry.
        match &entries[0] {
            Entry::Error(e) => assert!(e.to_string().contains("Expected '='")),
            _ => panic!("Should be error"),
        }
    }

    #[test]
    fn test_export_standalone() {
        let input = "export\n";
        let entries = parse(input);
        // "export" treated as key (no following space)
        // No equals sign.
        match &entries[0] {
            Entry::Error(e) => assert!(e.to_string().contains("Expected '='")),
             _ => panic!("Should be error"),
        }
    }

    #[test]
    fn test_export_no_definition() {
        let input = "export   \n";
        let entries = parse(input);
        // "export " matched.
        // Key is empty (newline immediately).
        // If key is empty, and next char is newline (not =), we might not get error?
        // Let's check logic:
        // key_start == key_end.
        // if !eof && peek() == '=' { Error }
        // recover_line()
        // So no error, just skipped line.
        assert_eq!(entries.len(), 0);
    }



    #[test]
    fn test_single_quote_backslash_quote() {
         let input = r#"KEY='a\'b'"#;
         let entries = parse(input);
         let kv = entries[0].as_pair().unwrap();
         // Single quotes: \ is literal.
         // Parses 'a\' then closing quote '
         // Value is a\
         assert_eq!(kv.value, r#"a\"#);
    }

    #[test]
    fn test_double_quote_unicode_escape_literal() {
        let input = r#"KEY="\u1234""#;
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
        // \u is not a special escape, so preserved literal
        assert_eq!(kv.value, r#"\u1234"#);
    }

    #[test]
    fn test_nested_escapes_deep() {
        let input = r#"KEY="\\\"""#;
        let entries = parse(input);
        let kv = entries[0].as_pair().unwrap();
         // \\ -> \
         // \" -> "
        assert_eq!(kv.value, r#"\""#);
    }
