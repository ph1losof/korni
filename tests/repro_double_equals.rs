use korni::{parse, Entry};

#[test]
fn test_double_equals_should_fail() {
    let input = "KEY==value";
    let entries = parse(input);
    
    // CURRENT BEHAVIOR: This is expected to fail with "Unexpected entry type" if the parser
    // currently parses this as KEY = "=value". If it already fails, then great.
    // The user wants clear error for this.
    match entries.first().unwrap() {
        Entry::Error(e) => {
             let msg = e.to_string();
             println!("Got expected error: {}", msg);
             assert!(msg.contains("Double equals sign"), "Error message should mention double equals");
        },
        Entry::Pair(pair) => panic!("Should result in error, but got Pair: {:?}={:?}", pair.key, pair.value),
        _ => panic!("Unexpected entry type"),
    }
}

#[test]
fn test_multiple_equals_in_value_ok() {
    let input = "KEY=value=with=equals";
    let entries = parse(input);
    
    match entries.first().unwrap() {
        Entry::Pair(pair) => {
            assert_eq!(pair.key, "KEY");
            assert_eq!(pair.value, "value=with=equals");
        },
        _ => panic!("Should be valid pair: {:?}", entries.first()),
    }
}

#[test]
fn test_multiple_equals_quoted_ok() {
    // Spec: Quoted values can contain equals
    let input = "KEY=\"=value\"";
    let entries = parse(input);
    
    match entries.first().unwrap() {
        Entry::Pair(pair) => {
            assert_eq!(pair.key, "KEY");
            assert_eq!(pair.value, "=value");
        },
        _ => panic!("Should be valid pair"),
    }
}

#[test]
fn test_multiple_equals_quoted_single_ok() {
    let input = "KEY='=value'";
    let entries = parse(input);
    
    match entries.first().unwrap() {
        Entry::Pair(pair) => {
            assert_eq!(pair.key, "KEY");
            assert_eq!(pair.value, "=value");
        },
        _ => panic!("Should be valid pair"),
    }
}
