use std::collections::HashMap;
use std::borrow::Cow;
use crate::ast::{Entry, KeyValuePair, Span};
use crate::error::Error;

/// Parsed environment with rich query API
#[derive(Debug, Clone, Default)]
pub struct Environment<'a> {
    pub(crate) pairs: HashMap<Cow<'a, str>, KeyValuePair<'a>>,
    pub(crate) comments: Vec<Span>,
    pub(crate) errors: Vec<Error>,
}

impl<'a> Environment<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    /// consume a list of entries into an Environment
    pub fn from_entries(entries: Vec<Entry<'a>>) -> Self {
        let mut env = Environment {
            pairs: HashMap::with_capacity(entries.len()),
            comments: Vec::with_capacity(entries.len() / 4), // Heuristic: 25% comments
            errors: Vec::new(), // Errors are rare, keep default
        };
        
        for entry in entries {
            match entry {
                Entry::Pair(kv) => {
                    env.pairs.insert(kv.key.clone(), kv);
                }
                Entry::Comment(span) => {
                    env.comments.push(span);
                }
                Entry::Error(err) => {
                    env.errors.push(err);
                }
            }
        }
        
        env
    }

    /// Get value by key (returns None if not found)
    pub fn get(&self, key: &str) -> Option<&str> {
        self.pairs.get(key).map(|kv| kv.value.as_ref())
    }
    
    /// Get value or default
    pub fn get_or<'b>(&'b self, key: &str, default: &'b str) -> &'b str {
        self.get(key).unwrap_or(default)
    }

    /// Get the full KeyValuePair object
    pub fn get_entry(&self, key: &str) -> Option<&KeyValuePair<'a>> {
        self.pairs.get(key)
    }
    
    /// Iterate over all pairs
    pub fn iter(&self) -> impl Iterator<Item = &KeyValuePair<'a>> {
        self.pairs.values()
    }
    
    /// Check if any errors occurred during parsing
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }
    
    /// Get all errors
    pub fn errors(&self) -> &[Error] {
        &self.errors
    }
    
    /// Export to HashMap<String, String> (owned copies)
    pub fn to_map(&self) -> HashMap<String, String> {
        self.pairs.iter()
            .map(|(k, v)| (k.to_string(), v.value.to_string()))
            .collect()
    }
    
    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.pairs.keys().map(|k| k.as_ref())
    }
    
    /// Check if key exists
    pub fn contains_key(&self, key: &str) -> bool {
        self.pairs.contains_key(key)
    }

    /// Convert to an owned Environment with 'static lifetime
    /// This is useful when the source string is temporary (e.g. read from file)
    pub fn into_owned(self) -> Environment<'static> {
        Environment {
            pairs: self.pairs.into_iter()
                .map(|(k, v)| (Cow::Owned(k.into_owned()), v.into_owned()))
                .collect(),
            comments: self.comments,
            errors: self.errors,
        }
    }
}
