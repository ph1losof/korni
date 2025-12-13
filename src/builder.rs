use std::path::PathBuf;
use std::fs;
use std::io::Read;
use crate::{Environment, ParseOptions};
use crate::error::Error;

#[derive(Debug)]
enum Source<'a> {
    Str(&'a str),
    Bytes(&'a [u8]),
}

pub struct KorniBuilder<'a> {
    source: Source<'a>,
    options: ParseOptions,
}

impl<'a> KorniBuilder<'a> {
    pub fn new(source_str: &'a str) -> Self {
        Self {
            source: Source::Str(source_str),
            options: ParseOptions::default(),
        }
    }

    pub fn from_bytes(bytes: &'a [u8]) -> Self {
        Self {
            source: Source::Bytes(bytes),
            options: ParseOptions::default(),
        }
    }

    /// Enable comment preservation
    pub fn preserve_comments(mut self) -> Self {
        self.options.include_comments = true;
        self
    }

    /// Enable position tracking
    pub fn track_positions(mut self) -> Self {
        self.options.track_positions = true;
        self
    }
    
    /// Parse the input into an Environment
    pub fn parse(self) -> Result<Environment<'a>, Error> {
        match self.source {
            Source::Str(s) => {
                let entries = crate::parse_with_options(s, self.options);
                Ok(Environment::from_entries(entries))
            }
            Source::Bytes(b) => {
                let s = std::str::from_utf8(b).map_err(|e| Error::InvalidUtf8 {
                    offset: e.valid_up_to(),
                    reason: format!("Invalid UTF-8: {}", e),
                })?;
                let entries = crate::parse_with_options(s, self.options);
                Ok(Environment::from_entries(entries))
            }
        }
    }
}

/// Builder for configuring and parsing environment variables from owned sources (files)
pub struct OwnedKorniBuilder {
    path: Option<PathBuf>,
    reader: Option<Box<dyn Read>>,
    options: ParseOptions,
}

impl OwnedKorniBuilder {
    pub fn from_file(path: impl Into<PathBuf>) -> Self {
        Self {
            path: Some(path.into()),
            reader: None,
            options: ParseOptions::default(),
        }
    }

    pub fn from_reader(reader: impl Read + 'static) -> Self {
        Self {
            path: None,
            reader: Some(Box::new(reader)),
            options: ParseOptions::default(),
        }
    }

    /// Enable comment preservation
    pub fn preserve_comments(mut self) -> Self {
        self.options.include_comments = true;
        self
    }

    /// Enable position tracking
    pub fn track_positions(mut self) -> Self {
        self.options.track_positions = true;
        self
    }
    
    /// Parse the input into an owned Environment
    pub fn parse(self) -> Result<Environment<'static>, Error> {
        let content = if let Some(path) = self.path {
            fs::read_to_string(&path).map_err(|e| Error::Io(format!("Failed to read file {}: {}", path.display(), e)))?
        } else if let Some(mut reader) = self.reader {
            let mut s = String::new();
            reader.read_to_string(&mut s).map_err(|e| Error::Io(format!("Failed to read from reader: {}", e)))?;
            s
        } else {
             return Err(Error::Generic { offset: 0, message: "No source provided".into() });
        };

        // We have strict zero-copy parser that takes &'a str.
        // We have owned content String.
        // We parse it, getting Entry<'local>.
        // We convert to Entry<'static> using into_owned().
        
        let entries = crate::parse_with_options(&content, self.options);
        
        let env_local = Environment::from_entries(entries);
        Ok(env_local.into_owned())
    }
}

/// Main entry point for Korni configuration and parsing
pub struct Korni;

impl Korni {
    /// Create a builder from a string slice
    pub fn from_str(input: &str) -> KorniBuilder {
        KorniBuilder::new(input)
    }
    
    /// Create a builder from bytes
    pub fn from_bytes(input: &[u8]) -> KorniBuilder {
        KorniBuilder::from_bytes(input)
    }
    
    /// Create a builder from file path
    pub fn from_file(path: impl Into<PathBuf>) -> OwnedKorniBuilder {
        OwnedKorniBuilder::from_file(path)
    }
    
    /// Search for file in current directory and ancestors
    pub fn find_file(filename: &str) -> Result<OwnedKorniBuilder, Error> {
        let current = std::env::current_dir().map_err(|e| Error::Io(format!("Failed to get current directory: {}", e)))?;
        
        let mut dir = current.as_path();
        loop {
            let file_path = dir.join(filename);
            if file_path.exists() {
                return Ok(OwnedKorniBuilder::from_file(file_path));
            }
            
            if let Some(parent) = dir.parent() {
                dir = parent;
            } else {
                break;
            }
        }
        
        Err(Error::Io(format!("File '{}' not found in current directory or ancestors", filename)))
    }
    
    /// Create builder from reader
    pub fn from_reader(reader: impl Read + 'static) -> OwnedKorniBuilder {
        OwnedKorniBuilder::from_reader(reader)
    }
}
