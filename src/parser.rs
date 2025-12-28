use std::borrow::Cow;
use crate::types::{Entry, KeyValuePair, ParseOptions, QuoteType, Span};
use crate::error::Error;

struct ParsedValue<'a> {
    value: Cow<'a, str>,
    value_start: usize,
    raw_len: usize,
    quote: QuoteType,
}

pub struct Parser<'a> {
    input: &'a str,
    bytes: &'a [u8],
    cursor: usize,
    options: ParseOptions,
    bom_checked: bool,
}

impl<'a> Parser<'a> {
    #[inline(always)]
    pub fn new(input: &'a str) -> Self {
        Self::with_options(input, ParseOptions::default())
    }

    #[inline(always)]
    pub fn with_options(input: &'a str, options: ParseOptions) -> Self {
        Self {
            input,
            bytes: input.as_bytes(),
            cursor: 0,
            options,
            bom_checked: false,
        }
    }

    pub fn parse(&mut self) -> Vec<Entry<'a>> {
        let mut entries = Vec::with_capacity(32);
        while let Some(entry) = self.next_entry() {
            entries.push(entry);
        }
        entries
    }

    pub fn iter(self) -> EnvIterator<'a> {
        EnvIterator { parser: self }
    }

    pub fn next_entry(&mut self) -> Option<Entry<'a>> {
        if !self.bom_checked {
            if let Some(err) = self.check_bom() {
                return Some(err);
            }
        }

        loop {
            if self.is_eof() { return None; }
            self.skip_horizontal_whitespace();
            if self.is_eof() { return None; }

            if self.peek() == b'\n' {
                self.cursor += 1;
                continue;
            }

            if self.peek() == b'#' {
                return self.handle_comment();
            }

            if let Some(entry) = self.parse_pair() {
                return Some(entry);
            }
            // If None, loop continues (ignoring the skipped line)
        }
    }
}

impl<'a> Parser<'a> {
    fn check_bom(&mut self) -> Option<Entry<'a>> {
        self.bom_checked = true;
        if self.bytes.starts_with(b"\xEF\xBB\xBF") {
            self.cursor += 3;
        }
        // Search only remaining slice
        if let Some(idx) = self.input[self.cursor..].find('\u{FEFF}') {
            return Some(Entry::Error(Error::InvalidBom { offset: self.cursor + idx }));
        }
        None
    }

    fn handle_comment(&mut self) -> Option<Entry<'a>> {
        if self.options.include_comments {
            let comment_start = self.cursor;
            self.cursor += 1; 
            self.skip_horizontal_whitespace();

            if let Some(pair) = self.try_parse_commented_pair() {
                 return Some(Entry::Pair(Box::new(pair)));
            } else {
                self.skip_to_newline();
                return Some(Entry::Comment(Span::from_offsets(comment_start, self.cursor)));
            }
        } else {
            self.skip_to_newline();
        }
        
        if !self.is_eof() && self.peek() == b'\n' { 
            self.cursor += 1; 
        }
        
        self.next_entry()
    }

fn parse_pair(&mut self) -> Option<Entry<'a>> {
        let is_exported = self.consume_export_keyword();

        let key_start = self.cursor;
        self.consume_key_chars();
        let key_end = self.cursor;
        let key_str = &self.input[key_start..key_end];

        if key_start == key_end {
            self.skip_horizontal_whitespace();
            if !self.is_eof() && self.peek() == b'=' {
                return Some(self.error_and_recover(Error::Generic { 
                    offset: key_start, 
                    message: "Empty key".into() 
                }));
            }

            if is_exported {
                self.skip_to_newline();
                if !self.is_eof() && self.peek() == b'\n' { self.cursor += 1; }
                return None; 
            }

            self.skip_to_newline();
            if !self.is_eof() && self.peek() == b'\n' { self.cursor += 1; }
            return None;
        }
        if key_str.as_bytes()[0].is_ascii_digit() {
            return Some(self.error_and_recover(Error::InvalidKey { offset: key_start, reason: "Key starts with digit".into() }));
        }

        // Space before equals
        if !self.is_eof() && matches!(self.peek(), b' ' | b'\t') {
            self.skip_horizontal_whitespace();
            if !self.is_eof() && self.peek() == b'=' {
                return Some(self.error_and_recover(Error::ForbiddenWhitespace { offset: key_start, location: "between key and equals" }));
            }
        }

        // Expect Equals
        if self.is_eof() || self.peek() != b'=' {
            return Some(self.error_and_recover(Error::Expected { offset: self.cursor, expected: "'='" }));
        }
        self.cursor += 1; // consume '='

        // Double equals check
        if !self.is_eof() && self.peek() == b'=' {
            return Some(self.error_and_recover(Error::DoubleEquals { offset: self.cursor }));
        }
        // Space after equals
        if !self.is_eof() && matches!(self.peek(), b' ' | b'\t') {
            return Some(self.error_and_recover(Error::ForbiddenWhitespace { offset: self.cursor, location: "after equals" }));
        }

        // Parse Value
        let value_start = self.cursor;
        let parsed_value = if !self.is_eof() && self.peek() == b'\'' {
            self.parse_single_quoted_value(value_start)
        } else if !self.is_eof() && self.peek() == b'"' {
            self.parse_double_quoted_value(value_start)
        } else {
            self.parse_unquoted_value(value_start)
        };

        let entry = match parsed_value {
            Ok(pv) => {
                let pair = if self.options.track_positions {
                    KeyValuePair::new(key_str, key_start, pv.value, pv.value_start, pv.raw_len, pv.quote, is_exported, false)
                } else {
                    KeyValuePair::new_fast(key_str, pv.value, pv.quote, is_exported, false)
                };
                Entry::Pair(Box::new(pair))
            },
            Err(e) => Entry::Error(e),
        };

        self.skip_to_newline();
        if !self.is_eof() && self.peek() == b'\n' { self.cursor += 1; }
        Some(entry)
    }

    fn try_parse_commented_pair(&mut self) -> Option<KeyValuePair<'a>> {
        let saved = self.cursor;
        let is_exported = self.consume_export_keyword();
        
        let key_start = self.cursor;
        self.consume_key_chars();
        let key_end = self.cursor;

        if key_start == key_end || self.is_eof() || self.peek() != b'=' {
            self.cursor = saved;
            return None;
        }

        let key_str = &self.input[key_start..key_end];
        if key_str.as_bytes()[0].is_ascii_digit() {
            self.cursor = saved;
            return None;
        }

        self.cursor += 1;
        let value_start = self.cursor;
        
        let parsed_value = if !self.is_eof() && self.peek() == b'\'' {
            self.parse_single_quoted_value(value_start)
        } else if !self.is_eof() && self.peek() == b'"' {
            self.parse_double_quoted_value(value_start)
        } else {
            self.parse_unquoted_value(value_start)
        };

        match parsed_value {
            Ok(pv) => {
                let pair = if self.options.track_positions {
                    KeyValuePair::new(key_str, key_start, pv.value, pv.value_start, pv.raw_len, pv.quote, is_exported, true)
                } else {
                    KeyValuePair::new_fast(key_str, pv.value, pv.quote, is_exported, true)
                };
                self.skip_to_newline();
                Some(pair)
            },
            Err(_) => {
                self.cursor = saved;
                None
            }
        }
    }
}

impl<'a> Parser<'a> {
    #[inline]
    fn parse_single_quoted_value(&mut self, start: usize) -> Result<ParsedValue<'a>, Error> {
        self.cursor += 1; // '
        let content_start = self.cursor;
        let remaining = &self.bytes[self.cursor..];
        
        if let Some(pos) = remaining.iter().position(|&b| b == b'\'') {
            self.cursor += pos;
            let content = &self.input[content_start..self.cursor];
            self.cursor += 1;
            Ok(ParsedValue {
                value: Cow::Borrowed(content),
                value_start: start,
                raw_len: self.cursor - start,
                quote: QuoteType::Single,
            })
        } else {
            self.cursor = self.bytes.len();
            Err(Error::UnclosedQuote { offset: start, quote_type: "single" })
        }
    }

    #[inline]
    fn parse_double_quoted_value(&mut self, start: usize) -> Result<ParsedValue<'a>, Error> {
        self.cursor += 1;
        let content_start = self.cursor;
        let remaining = &self.bytes[self.cursor..];

        if let Some(pos) = remaining.iter().position(|&b| b == b'"' || b == b'\\') {
            if remaining[pos] == b'"' {
                self.cursor += pos;
                let content = &self.input[content_start..self.cursor];
                self.cursor += 1;
                return Ok(ParsedValue {
                    value: Cow::Borrowed(content),
                    value_start: start,
                    raw_len: self.cursor - start,
                    quote: QuoteType::Double,
                });
            }
            self.cursor += pos;
        } else {
            self.cursor += remaining.len();
        }

        self.cursor = content_start;
        let remaining_len = self.bytes.len() - self.cursor;
        let mut value = String::with_capacity(remaining_len.saturating_sub(1));
        
        loop {
            if self.is_eof() {
                return Err(Error::UnclosedQuote { offset: start, quote_type: "double" });
            }
            let b = self.peek();
            if b == b'\\' && self.cursor + 1 < self.bytes.len() {
                self.cursor += 1;
                let c = self.peek();
                match c {
                    b'n' => value.push('\n'),
                    b'r' => value.push('\r'),
                    b't' => value.push('\t'),
                    b'\\' => value.push('\\'),
                    b'"' => value.push('"'),
                    b'$' => value.push('$'),
                    _ => { value.push('\\'); value.push(c as char); }
                }
                self.cursor += 1;
            } else if b == b'"' {
                self.cursor += 1;
                return Ok(ParsedValue {
                    value: Cow::Owned(value),
                    value_start: start,
                    raw_len: self.cursor - start,
                    quote: QuoteType::Double,
                });
            } else {
                value.push(b as char);
                self.cursor += 1;
            }
        }
    }

    #[inline]
    fn parse_unquoted_value(&mut self, start: usize) -> Result<ParsedValue<'a>, Error> {
        let start_pos = self.cursor;
        let mut needs_allocation = false;
        let mut trailing_backslash = false;
        
        loop {
            if self.is_eof() { break; }
            let line_start = self.cursor;
            
            let remaining = &self.bytes[self.cursor..];
            let (limit, stop_char) = match remaining.iter().position(|&b| matches!(b, b' ' | b'\t' | b'\n' | b'\r')) {
                Some(pos) => (self.cursor + pos, Some(remaining[pos])),
                None => (self.cursor + remaining.len(), None)
            };

            let stopped_at_eol = matches!(stop_char, Some(b'\n') | Some(b'\r') | None);
            let is_continuation = stopped_at_eol && limit > line_start && self.bytes[limit - 1] == b'\\';

            if is_continuation {
                needs_allocation = true;
                self.cursor = limit;
                if !self.is_eof() {
                    if self.peek() == b'\r' { self.cursor += 1; }
                    if !self.is_eof() && self.peek() == b'\n' { self.cursor += 1; }
                } else {
                    trailing_backslash = true;
                    break;
                }
            } else {
                self.cursor = limit;
                break;
            }
        }

        let value = if needs_allocation {
            let mut value = String::with_capacity(self.cursor - start_pos);
            self.cursor = start_pos;
            
            loop {
                if self.is_eof() { break; }
                let line_start = self.cursor;
                
                let remaining = &self.bytes[self.cursor..];
                let (limit, stop_char) = match remaining.iter().position(|&b| matches!(b, b' ' | b'\t' | b'\n' | b'\r')) {
                    Some(pos) => (self.cursor + pos, Some(remaining[pos])),
                    None => (self.cursor + remaining.len(), None)
                };

                let chunk = &self.input[self.cursor..limit];
                let stopped_at_eol = matches!(stop_char, Some(b'\n') | Some(b'\r') | None);
                let is_continuation = stopped_at_eol && limit > line_start && self.bytes[limit - 1] == b'\\';

                if is_continuation {
                    value.push_str(&chunk[..chunk.len()-1]);
                    self.cursor = limit;
                    if !self.is_eof() {
                        if self.peek() == b'\r' { self.cursor += 1; }
                        if !self.is_eof() && self.peek() == b'\n' { self.cursor += 1; }
                    } else {
                        break;
                    }
                } else {
                    value.push_str(chunk);
                    self.cursor = limit;
                    break;
                }
            }
            if trailing_backslash {
                value.push('\\');
            }
            Cow::Owned(value)
        } else {
            Cow::Borrowed(&self.input[start_pos..self.cursor])
        };
        
        Ok(ParsedValue {
            value,
            value_start: start,
            raw_len: self.cursor - start,
            quote: QuoteType::None,
        })
    }
}

impl<'a> Parser<'a> {
    #[inline(always)]
    fn peek(&self) -> u8 {
        debug_assert!(self.cursor < self.bytes.len(), "peek() called when at EOF");
        self.bytes[self.cursor]
    }

    #[inline(always)]
    fn is_eof(&self) -> bool { self.cursor >= self.bytes.len() }

    #[inline]
    fn skip_horizontal_whitespace(&mut self) {
        if self.cursor < self.bytes.len() {
             let remaining = &self.bytes[self.cursor..];
             let advance = remaining.iter().position(|&b| b != b' ' && b != b'\t').unwrap_or(remaining.len());
             self.cursor += advance;
        }
    }

    #[inline]
    fn consume_key_chars(&mut self) {
        if self.cursor < self.bytes.len() {
            let remaining = &self.bytes[self.cursor..];
            let advance = remaining.iter().position(|&b| !b.is_ascii_alphanumeric() && b != b'_').unwrap_or(remaining.len());
            self.cursor += advance;
        }
    }

    fn consume_export_keyword(&mut self) -> bool {
        if self.cursor + 6 < self.bytes.len() && &self.bytes[self.cursor..self.cursor+6] == b"export" {
            let next = self.bytes.get(self.cursor + 6).copied().unwrap_or(0);
            if matches!(next, b' ' | b'\t') {
                self.cursor += 6;
                self.skip_horizontal_whitespace();
                return true;
            }
        }
        false
    }

    #[inline]
    fn skip_to_newline(&mut self) {
        if self.cursor < self.bytes.len() {
            let remaining = &self.bytes[self.cursor..];
            let advance = remaining.iter().position(|&b| b == b'\n').unwrap_or(remaining.len());
            self.cursor += advance;
        }
    }

    fn error_and_recover(&mut self, err: Error) -> Entry<'a> {
        self.skip_to_newline();
        if !self.is_eof() { self.cursor += 1; }
        Entry::Error(err)
    }
}

pub struct EnvIterator<'a> {
    parser: Parser<'a>,
}

impl<'a> Iterator for EnvIterator<'a> {
    type Item = Entry<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        self.parser.next_entry()
    }
}
