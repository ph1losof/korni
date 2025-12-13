use korni::{parse, Entry, KeyValuePair};

pub fn parse_single_pair(input: &str) -> Option<(String, String)> {
    let entries = parse(input);
    entries.into_iter().find_map(|e| {
        if let Entry::Pair(kv) = e {
            Some((kv.key.to_string(), kv.value.to_string()))
        } else {
            None
        }
    })
}

pub fn assert_pair(input: &str, expected_key: &str, expected_value: &str) {
    let pair = parse_single_pair(input).expect(&format!("Failed to parse pair from input: {:?}", input));
    assert_eq!(pair.0, expected_key, "Key mismatch");
    assert_eq!(pair.1, expected_value, "Value mismatch");
}

pub fn assert_error(input: &str, expected_part: &str) {
    let entries = parse(input);
    let error = entries.iter().find_map(|e| {
        if let Entry::Error(err) = e {
            Some(err.to_string())
        } else {
            None
        }
    });

    assert!(error.is_some(), "Expected error for input: {:?} but got success", input);
    let msg = error.unwrap();
    assert!(msg.contains(expected_part), "Error message {:?} did not contain {:?}", msg, expected_part);
}

pub fn assert_exported(input: &str, expected_key: &str, expected_value: &str) {
    let entries = parse(input);
    let kv = entries.iter().find_map(|e| e.as_pair().map(|p| p.clone())).unwrap();
    assert_eq!(kv.key, expected_key);
    assert_eq!(kv.value, expected_value);
    assert!(kv.is_exported, "Expected exported flag to be true");
}

pub fn assert_not_exported(input: &str, expected_key: &str, expected_value: &str) {
    let entries = parse(input);
    let kv = entries.iter().find_map(|e| e.as_pair().map(|p| p.clone())).unwrap();
    assert_eq!(kv.key, expected_key);
    assert_eq!(kv.value, expected_value);
    assert!(!kv.is_exported, "Expected exported flag to be false");
}
