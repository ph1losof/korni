use crate::ast::{Entry, KeyValuePair, ParseOptions, QuoteType, Span};
use crate::error::Error;
use std::borrow::Cow;

/// Parsed value result with minimal position tracking
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
        Self {
            input,
            bytes: input.as_bytes(),
            cursor: 0,
            options: ParseOptions::default(),
            bom_checked: false,
        }
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

    /// Parse with default options (fast mode: key-value pairs only)
    pub fn parse(&mut self) -> Vec<Entry<'a>> {
        self.parse_internal()
    }
    
    /// Parse with custom options
    pub fn parse_with_options(&mut self, options: ParseOptions) -> Vec<Entry<'a>> {
        self.options = options;
        self.parse_internal()
    }
    
    #[inline]
    fn parse_internal(&mut self) -> Vec<Entry<'a>> {
        let mut entries = Vec::with_capacity(32);
        while let Some(entry) = self.next_entry() {
            entries.push(entry);
        }
        entries
    }

    /// Iterator support: get next entry
    pub fn next_entry(&mut self) -> Option<Entry<'a>> {
        if !self.bom_checked {
            self.bom_checked = true;
            // Optional BOM check at start
            if self.bytes.starts_with(b"\xEF\xBB\xBF") {
                self.cursor += 3;
            }

            // Strict BOM check: BOM in middle of file is invalid
            if let Some(idx) = self.input.find('\u{FEFF}') {
                if idx > 0 {
                     return Some(Entry::Error(Error::InvalidBom {
                         offset: idx,
                     }));
                }
            }
        }

        loop {
            if self.is_eof() {
                return None;
            }

            self.skip_horizontal_whitespace();

            if self.is_eof() {
                return None;
            }

            if self.peek() == b'\n' {
                self.cursor += 1;
                continue;
            }

            if self.peek() == b'#' {
                if self.options.include_comments {
                    let comment_start = self.cursor;
                    self.cursor += 1; // consume #
                    self.skip_horizontal_whitespace();
                    
                    if let Some(pair) = self.try_parse_commented_pair() {
                        return Some(Entry::Pair(pair));
                    } else {
                        self.skip_to_newline();
                        return Some(Entry::Comment(Span::from_offsets(comment_start, self.cursor)));
                    }
                } else {
                    // Fast mode: skip comment lines entirely
                    self.skip_to_newline();
                }
                if !self.is_eof() && self.peek() == b'\n' { self.cursor += 1; }
                continue;
            }

            // Variable Definition
            let mut is_exported = false;
            // Check for "export" keyword
            if self.cursor + 6 < self.bytes.len() && &self.bytes[self.cursor..self.cursor+6] == b"export" {
                let next = self.bytes.get(self.cursor + 6).copied().unwrap_or(0);
                if next == b' ' || next == b'\t' {
                    is_exported = true;
                    self.cursor += 6;
                    self.skip_horizontal_whitespace();
                }
            }
            
            // Parse Key
            let key_start = self.cursor;
            self.consume_key();
            let key_end = self.cursor;
            
            if key_start == key_end {
                if !self.is_eof() && self.peek() == b'=' {
                     let err = Entry::Error(Error::Generic {
                        offset: key_start,
                        message: "Empty key".into(),
                    });
                    self.recover_line();
                    return Some(err);
                }
                self.recover_line();
                continue;
            }

            let key_str = &self.input[key_start..key_end];
            
            // Validate key doesn't start with digit
            if key_str.as_bytes()[0].is_ascii_digit() {
                let err = Entry::Error(Error::InvalidKey {
                    offset: key_start,
                    reason: "Key starts with digit".into(),
                });
                self.recover_line();
                return Some(err);
            }

            // Check for forbidden whitespace before =
            if !self.is_eof() && (self.peek() == b' ' || self.peek() == b'\t') {
                self.skip_horizontal_whitespace();
                if !self.is_eof() && self.peek() == b'=' {
                    let err = Entry::Error(Error::ForbiddenWhitespace {
                        offset: key_start,
                        location: "between key and equals",
                    });
                    self.recover_line();
                    return Some(err);
                }
            }

            if self.is_eof() || self.peek() != b'=' {
                let err = Entry::Error(Error::Expected {
                    offset: self.cursor,
                    expected: "'='",
                });
                self.recover_line();
                return Some(err);
            }
            
            self.cursor += 1; // consume '='
            
            // Check for double equals
            if !self.is_eof() && self.peek() == b'=' {
                let err = Entry::Error(Error::DoubleEquals {
                    offset: self.cursor,
                });
                self.recover_line();
                return Some(err);
            }

            // Check for forbidden whitespace after =
            if !self.is_eof() && (self.peek() == b' ' || self.peek() == b'\t') {
                let err = Entry::Error(Error::ForbiddenWhitespace {
                    offset: self.cursor,
                    location: "after equals",
                });
                self.recover_line();
                return Some(err);
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
                        KeyValuePair::new(
                            key_str,
                            key_start,
                            pv.value,
                            pv.value_start,
                            pv.raw_len,
                            pv.quote,
                            is_exported,
                            false,
                        )
                    } else {
                        KeyValuePair::new_fast(
                            key_str,
                            pv.value,
                            pv.quote,
                            is_exported,
                            false,
                        )
                    };
                    Entry::Pair(pair)
                }
                Err(e) => {
                    Entry::Error(e)
                }
            };

            // Consume rest of line
            self.skip_to_newline();
            if !self.is_eof() && self.peek() == b'\n' { self.cursor += 1; }
            
            return Some(entry);
        }
    }

    #[inline]
    fn try_parse_commented_pair(&mut self) -> Option<KeyValuePair<'a>> {
        let saved = self.cursor;
        
        // Check for optional "export "
        let mut is_exported = false;
        if self.cursor + 6 < self.bytes.len() && &self.bytes[self.cursor..self.cursor+6] == b"export" {
            let next = self.bytes.get(self.cursor + 6).copied().unwrap_or(0);
            if next == b' ' || next == b'\t' {
                is_exported = true;
                self.cursor += 6;
                self.skip_horizontal_whitespace();
            }
        }
        
        let key_start = self.cursor;
        self.consume_key();
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
        
        self.cursor += 1; // consume =
        
        // Reuse the main parsing logic for values to ensure consistency
        let value_start = self.cursor;
        
        let parsed_value = if !self.is_eof() && self.peek() == b'\'' {
            self.parse_single_quoted_value(value_start)
        } else if !self.is_eof() && self.peek() == b'"' {
            self.parse_double_quoted_value(value_start)
        } else {
            // For unquoted in comments, we need slightly different termination logic?
            // "The value consists of all characters from the start of the value up to, but not including, the whitespace that precedes an inline comment."
            // But here we ARE inside a comment line.
            // "Parsing Rule: ... logic is same as standard"
            // Actually, for "commented out" pairs like `# KEY=value`, it usually just parses until end of line.
            // But if we want to support `# KEY="val" # comment`, we should probably stick to standard parsing
            // but ignore the result's "comment" part if it has one?
            // Let's reuse standard unquoted parser.
            self.parse_unquoted_value(value_start)
        };
        
        match parsed_value {
            Ok(pv) => {
                let pair = if self.options.track_positions {
                    KeyValuePair::new(
                        key_str,
                        key_start,
                        pv.value,
                        pv.value_start,
                        pv.raw_len,
                        pv.quote,
                        is_exported,
                        true,
                    )
                } else {
                    KeyValuePair::new_fast(
                        key_str,
                        pv.value,
                        pv.quote,
                        is_exported,
                        true,
                    )
                };
                 self.skip_to_newline();
                 Some(pair)
            },
            Err(_) => {
                // If parsing value failed (e.g. unclosed quote), we just treat line as comment
                self.cursor = saved;
                None
            }
        }
    }

    #[inline]
    fn parse_single_quoted_value(&mut self, start: usize) -> Result<ParsedValue<'a>, Error> {
        self.cursor += 1; // '
        let content_start = self.cursor;
        
        // Optimize: use memchr-like search via slice iterator
        // This is safe because we are searching for a byte
        let remaining = &self.bytes[self.cursor..];
        if let Some(pos) = remaining.iter().position(|&b| b == b'\'') {
             self.cursor += pos;
             let content = &self.input[content_start..self.cursor];
             self.cursor += 1; // closing '
             
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
        self.cursor += 1; // "
        let content_start = self.cursor;
        
        // Fast scan for closing quote or escapes using slice iterator
        // This avoids bounds checking in the loop
        let remaining = &self.bytes[self.cursor..];
        
        // Search for either " or \
        // position() often vectorizes
        match remaining.iter().position(|&b| b == b'"' || b == b'\\') {
            Some(pos) => {
                let b = remaining[pos];
                if b == b'"' {
                    // Happy path: found closing quote, no escapes
                    self.cursor += pos;
                    let content = &self.input[content_start..self.cursor];
                    self.cursor += 1; // consume "
                    return Ok(ParsedValue {
                        value: Cow::Borrowed(content),
                        value_start: start,
                        raw_len: self.cursor - start,
                        quote: QuoteType::Double,
                    });
                }
                // Found escape, fall through to slow path
                self.cursor += pos;
            }
            None => {
                // Reached EOF without finding closing quote or escape
                self.cursor += remaining.len();
                // Let slow path handle EOF error
            }
        }
        
        // Slow path: contains escapes
        // Reset cursor to start of content to re-parse with escaping
        self.cursor = content_start;
        let mut value = String::with_capacity(32);
        
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
                    _ => {
                        value.push('\\');
                        value.push(c as char);
                    }
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
        let mut value = String::new();
        let mut has_continuation = false;
        
        loop {
            if self.is_eof() {
                break;
            }
            let line_start = self.cursor;
            let mut comment_idx: Option<usize> = None;
            
            // Optimize: scan for delimiters or comment start using slice iterator
            let remaining = &self.bytes[self.cursor..];
            let mut offset = 0;
            
            while offset < remaining.len() {
                // Search for interesting characters: \n, \r, #
                // We restart the search if we find a # that isn't a comment start
                match remaining[offset..].iter().position(|&b| b == b'\n' || b == b'\r' || b == b'#') {
                    Some(pos) => {
                        let absolute_pos = offset + pos;
                        let found_char = remaining[absolute_pos];
                        
                        if found_char == b'#' {
                            // Check if it's a valid comment start (preceded by whitespace)
                            // We need to check relative to cursor+absolute_pos
                            let current_idx = self.cursor + absolute_pos;
                            if current_idx > line_start {
                                let prev = self.bytes[current_idx - 1];
                                if prev == b' ' || prev == b'\t' {
                                    if comment_idx.is_none() {
                                        comment_idx = Some(current_idx - 1); // include the whitespace
                                    }
                                    // Found comment, we can stop the scan for this line
                                    offset = absolute_pos; // will be added to cursor
                                    break;
                                }
                            }
                            // Not a comment start, continue scanning after this #
                            offset = absolute_pos + 1;
                        } else {
                            // Found newline (\n or \r), end of line
                            offset = absolute_pos;
                            break;
                        }
                    }
                    None => {
                        // Reached EOF for this slice
                        offset = remaining.len();
                        break;
                    }
                }
            }
            self.cursor += offset;
            
            let mut end = comment_idx.unwrap_or(self.cursor);
            while end > line_start && (self.bytes[end - 1] == b' ' || self.bytes[end - 1] == b'\t') {
                end -= 1;
            }
            
            // Spec 5.2: Backslash at EOF is literal (not continuation).
            // Spec 5.2: Empty lines during continuation are consumed and continuation continues.
            let mut is_continuation = end > line_start && self.bytes[end - 1] == b'\\' && !self.is_eof();
            
            if has_continuation && end == line_start {
                // If we are already continuing, an empty line is treated as a continuation
                // that adds nothing but keeps the loop going.
                is_continuation = true;
            }

            if is_continuation && end > line_start {
                end -= 1;
            }
            
            let chunk = &self.input[line_start..end];
            
            if !has_continuation && !is_continuation {
                // Spec 4.2.2: Unquoted values end at the first whitespace.
                let mut final_chunk = chunk;
                if let Some(idx) = chunk.find(|c: char| c == ' ' || c == '\t') {
                    final_chunk = &chunk[..idx];
                }
                
                return Ok(ParsedValue {
                    value: Cow::Borrowed(final_chunk),
                    value_start: start,
                    raw_len: end - start,
                    quote: QuoteType::None,
                });
            }
            
            value.push_str(chunk);
            has_continuation = true;
            
            if !self.is_eof() && self.peek() == b'\r' {
                self.cursor += 1;
            }
            if !self.is_eof() && self.peek() == b'\n' {
                self.cursor += 1;
            }
            
            if !is_continuation {
                break;
            }
        }
        
        // Spec 4.2.2: Unquoted values end at the first whitespace.
        // Spec 5.2: Legacy continuation allows spaces (e.g. `docker run \`).
        // So we strictly enforce separation only for simple single-line values.
        let mut final_val = value;
        if !has_continuation {
             if let Some(idx) = final_val.find(|c: char| c == ' ' || c == '\t') {
                 final_val.truncate(idx);
             }
        }

        Ok(ParsedValue {
            value: Cow::Owned(final_val),
            value_start: start,
            raw_len: self.cursor - start,
            quote: QuoteType::None,
        })
    }
    
    #[inline(always)]
    fn peek(&self) -> u8 {
        self.bytes[self.cursor]
    }
    
    #[inline(always)]
    fn is_eof(&self) -> bool {
        self.cursor >= self.bytes.len()
    }

    #[inline(always)]
    fn skip_horizontal_whitespace(&mut self) {
        if self.cursor < self.bytes.len() {
             let remaining = &self.bytes[self.cursor..];
             let advance = remaining.iter()
                 .position(|&b| b != b' ' && b != b'\t')
                 .unwrap_or(remaining.len());
             self.cursor += advance;
        }
    }

    #[inline(always)]
    fn consume_key(&mut self) {
        if self.cursor < self.bytes.len() {
            let remaining = &self.bytes[self.cursor..];
            let advance = remaining.iter()
                .position(|&b| !b.is_ascii_alphanumeric() && b != b'_')
                .unwrap_or(remaining.len());
            self.cursor += advance;
        }
    }

    #[inline(always)]
    fn skip_to_newline(&mut self) {
        if self.cursor < self.bytes.len() {
            let remaining = &self.bytes[self.cursor..];
            let advance = remaining.iter()
                .position(|&b| b == b'\n')
                .unwrap_or(remaining.len());
            self.cursor += advance;
        }
    }

    #[inline(always)]
    fn recover_line(&mut self) {
        self.skip_to_newline();
        if !self.is_eof() { self.cursor += 1; }
    }
}
