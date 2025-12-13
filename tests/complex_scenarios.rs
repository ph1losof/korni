mod common;
use common::{assert_pair, assert_error};
use korni::{parse, Entry};

// --- 1. Real World PEM Keys ---

#[test]
fn test_complex_pem_key() {
    let input = r#"PRIVATE_KEY="-----BEGIN RSA PRIVATE KEY-----
MIIEpQIBAAKCAQEA3Tz2MR7SZiAMfQyuvBjM9Oi..
..more base64 data..
-----END RSA PRIVATE KEY-----""#;
    let entries = parse(input);
    let kv = entries[0].as_pair().unwrap();
    assert!(kv.value.contains("BEGIN RSA PRIVATE KEY"));
    assert!(kv.value.contains("END RSA PRIVATE KEY"));
    assert!(kv.value.contains('\n'));
}

#[test]
fn test_complex_pem_key_single_quote() {
    let input = "PRIVATE_KEY='-----BEGIN RSA PRIVATE KEY-----
MIIEpQIBAAKCAQEA3Tz2MR7SZiAMfQyuvBjM9Oi..
-----END RSA PRIVATE KEY-----'";
    let entries = parse(input);
    let kv = entries[0].as_pair().unwrap();
    assert!(kv.value.contains("BEGIN RSA PRIVATE KEY"));
}

// --- 2. JSON Configs ---

#[test]
fn test_complex_json_config() {
    let input = r#"APP_CONFIG="{\"database\": {\"host\": \"localhost\", \"port\": 5432}, \"debug\": true}""#;
    let entries = parse(input);
    let kv = entries[0].as_pair().unwrap();
    assert_eq!(kv.value, r#"{"database": {"host": "localhost", "port": 5432}, "debug": true}"#);
}

// --- 3. Duplicate Keys (Last Wins) ---

#[test]
fn test_duplicate_keys_last_wins() {
    let input = "KEY=val1\nKEY=val2\nKEY=val3";
    let entries = parse(input);
    // Parser returns ALL entries (order preserved). 
    // Consumer logic decides last wins. Parser duty is to preserve order.
    let pairs: Vec<_> = entries.iter().filter_map(|e| e.as_pair()).collect();
    assert_eq!(pairs.len(), 3);
    assert_eq!(pairs[0].value, "val1");
    assert_eq!(pairs[1].value, "val2");
    assert_eq!(pairs[2].value, "val3");
}

// --- 4. Order Preservation ---

#[test]
fn test_order_preservation_mixed() {
    let input = "A=1\nB=2\nA=3\nC=4";
    let entries = parse(input);
    let pairs: Vec<_> = entries.iter().filter_map(|e| e.as_pair()).collect();
    assert_eq!(pairs[0].key, "A");
    assert_eq!(pairs[0].value, "1");
    assert_eq!(pairs[1].key, "B");
    assert_eq!(pairs[2].key, "A");
    assert_eq!(pairs[2].value, "3");
    assert_eq!(pairs[3].key, "C");
}

// --- 5. Stress Test (Volume) ---

#[test]
fn test_stress_many_lines() {
    let mut input = String::new();
    for i in 0..1000 {
        input.push_str(&format!("KEY{}=val{}\n", i, i));
    }
    let entries = parse(&input);
    assert_eq!(entries.len(), 1000);
    entries.into_iter().enumerate().for_each(|(i, e)| {
        let kv = e.as_pair().unwrap();
        assert_eq!(kv.key, format!("KEY{}", i));
        assert_eq!(kv.value, format!("val{}", i));
    });
}

// --- 6. Nasty Combination ---

#[test]
fn test_nasty_combination() {
    let input = r#"
# Header
export VAR1='simple'
VAR2="escaped \"quote\" and \n newline"
VAR3=unquoted value with # hash
VAR4="nested 'quotes' work"
VAR5='nested "quotes" work'
VAR6=continuation \
     next line
    "#;
    let entries = parse(input);
    let pairs: Vec<_> = entries.iter().filter_map(|e| e.as_pair()).collect();
    
    assert_eq!(pairs.len(), 6);
    
    assert_eq!(pairs[0].key, "VAR1");
    assert_eq!(pairs[0].value, "simple");
    assert!(pairs[0].is_exported);
    
    assert_eq!(pairs[1].key, "VAR2");
    assert_eq!(pairs[1].value, "escaped \"quote\" and \n newline");
    
    assert_eq!(pairs[2].key, "VAR3");

    // Wait, unquoted parser logic:
    // VAR3=unquoted value with # hash
    // The # is preceded by space. 
    // Spec 4.2.2: Unquoted values end at first whitespace.
    // So `unquoted` is the value. The rest is ignored trailing content.
    assert_eq!(pairs[2].value, "unquoted");
    
    assert_eq!(pairs[3].key, "VAR4");
    assert_eq!(pairs[3].value, "nested 'quotes' work");
    
    assert_eq!(pairs[4].key, "VAR5");
    assert_eq!(pairs[4].value, "nested \"quotes\" work");
    
    assert_eq!(pairs[5].key, "VAR6");
    assert_eq!(pairs[5].value, "continuation      next line"); // spaces preserved in next line start
}

// --- 7. Whitespace Torture ---

#[test]
fn test_whitespace_torture() {
    let input = "   KEY  =  val"; // Error: space around equals
    assert_error(input, "Whitespace not allowed between key and equals");
}

#[test]
fn test_whitespace_valid_torture() {
    let input = "\t\tKEY=\"  val  \"\t\t# comment";
    assert_pair(input, "KEY", "  val  "); // Quotes preserve space, tabs ignored, comment ignored
}

