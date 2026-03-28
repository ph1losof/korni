use crate::error::Error;
use crate::types::{Entry, KeyValuePair, Span};
use std::borrow::Cow;
use std::collections::HashMap;
use std::hash::{BuildHasherDefault, Hasher};

const FNV_OFFSET_BASIS: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;

#[derive(Clone)]
pub(crate) struct Fnv1aHasher(u64);

impl Default for Fnv1aHasher {
    #[inline]
    fn default() -> Self {
        Self(FNV_OFFSET_BASIS)
    }
}

impl Hasher for Fnv1aHasher {
    #[inline]
    fn finish(&self) -> u64 {
        self.0
    }

    #[inline]
    fn write(&mut self, bytes: &[u8]) {
        let mut hash = self.0;
        for &b in bytes {
            hash ^= b as u64;
            hash = hash.wrapping_mul(FNV_PRIME);
        }
        self.0 = hash;
    }
}

type FastMap<K, V> = HashMap<K, V, BuildHasherDefault<Fnv1aHasher>>;

/// Parsed environment with rich query API.
#[derive(Debug, Clone, Default)]
pub struct Environment<'a> {
    pub(crate) pairs: FastMap<Cow<'a, str>, KeyValuePair<'a>>,
    pub(crate) comments: Vec<Span>,
    pub(crate) errors: Vec<Error>,
}

impl<'a> Environment<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_entries(entries: Vec<Entry<'a>>) -> Self {
        let mut env = Environment {
            pairs: HashMap::with_capacity_and_hasher(entries.len(), BuildHasherDefault::default()),
            comments: Vec::with_capacity(entries.len() / 4),
            errors: Vec::new(),
        };

        for entry in entries {
            match entry {
                Entry::Pair(kv) => {
                    env.pairs.insert(kv.key.clone(), *kv);
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

    pub fn get(&self, key: &str) -> Option<&str> {
        self.pairs.get(key).map(|kv| kv.value.as_ref())
    }

    pub fn get_or<'b>(&'b self, key: &str, default: &'b str) -> &'b str {
        self.get(key).unwrap_or(default)
    }

    pub fn get_entry(&self, key: &str) -> Option<&KeyValuePair<'a>> {
        self.pairs.get(key)
    }

    pub fn iter(&self) -> impl Iterator<Item = &KeyValuePair<'a>> {
        self.pairs.values()
    }

    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    pub fn errors(&self) -> &[Error] {
        &self.errors
    }

    pub fn to_map(&self) -> HashMap<String, String> {
        self.pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.value.to_string()))
            .collect()
    }

    pub fn into_owned(self) -> Environment<'static> {
        Environment {
            pairs: self
                .pairs
                .into_iter()
                .map(|(k, v)| (Cow::Owned(k.into_owned()), v.into_owned()))
                .collect(),
            comments: self.comments,
            errors: self.errors,
        }
    }
}
