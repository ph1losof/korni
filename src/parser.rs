use crate::error::Error;
use crate::types::{Entry, KeyValuePair, ParseOptions, QuoteType, Span};
use std::borrow::Cow;

struct ParsedValue<'a> {
    value: Cow<'a, str>,
    value_start: usize,
    raw_len: usize,
    quote: QuoteType,
}

// Minimal state for comment scanning
struct CommentScanState {
    line_start: usize,
    line_end: usize,
    scan_pos: usize,
    returned_comment: bool,
}

pub struct Parser<'a> {
    input: &'a str,
    bytes: &'a [u8],
    cursor: usize,
    options: ParseOptions,
    bom_checked: bool,
    comment_state: Option<CommentScanState>,
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
            comment_state: None,
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

    #[inline]
    pub fn next_entry(&mut self) -> Option<Entry<'a>> {
        // Fast path: check comment state
        if let Some(state) = self.comment_state.as_mut() {
            if !state.returned_comment {
                let line_start = state.line_start;
                let line_end = state.line_end;
                state.returned_comment = true;
                return Some(Entry::Comment(Span::from_offsets(
                    line_start - 1, // Include '#'
                    line_end,
                )));
            }

            // Extract values to avoid borrow conflicts
            let line_start = state.line_start;
            let line_end = state.line_end;
            let mut scan_pos = state.scan_pos;

            // Scan for next pair
            let line_bytes = &self.bytes[line_start..line_end];

            while scan_pos < line_bytes.len() {
                if line_bytes[scan_pos] == b'=' {
                    let eq_pos = line_start + scan_pos;
                    scan_pos += 1;

                    // Fast backwards key scan with unsafe for speed
                    if let Some((key_start, key_end)) =
                        unsafe { self.find_key_backwards_fast(eq_pos) }
                    {
                        if let Some(parsed) = self.parse_value_in_comment_fast(eq_pos + 1, line_end)
                        {
                            let key_str = &self.input[key_start..key_end];

                            // Update state for next call
                            if let Some(s) = self.comment_state.as_mut() {
                                s.scan_pos = parsed.value_start + parsed.raw_len - line_start;
                            }

                            let pair = if self.options.track_positions {
                                KeyValuePair::new(
                                    key_str,
                                    key_start,
                                    parsed.value,
                                    parsed.value_start,
                                    parsed.raw_len,
                                    parsed.quote,
                                    false,
                                    None,
                                    true,
                                )
                            } else {
                                KeyValuePair::new_fast(
                                    key_str,
                                    parsed.value,
                                    parsed.quote,
                                    false,
                                    true,
                                )
                            };
                            return Some(Entry::Pair(Box::new(pair)));
                        }
                    }

                    // Failed to parse, update position and continue
                    if let Some(s) = self.comment_state.as_mut() {
                        s.scan_pos = scan_pos;
                    }
                } else {
                    scan_pos += 1;
                }
            }

            // Done with comment
            self.cursor = line_end;
            if self.cursor < self.bytes.len() && self.bytes[self.cursor] == b'\n' {
                self.cursor += 1;
            }
            self.comment_state = None;
            return self.next_entry();
        }

        if !self.bom_checked {
            if let Some(err) = self.check_bom() {
                return Some(err);
            }
        }

        loop {
            if self.cursor >= self.bytes.len() {
                return None;
            }

            // Fast whitespace skip
            while self.cursor < self.bytes.len() {
                let b = unsafe { *self.bytes.get_unchecked(self.cursor) };
                if b != b' ' && b != b'\t' {
                    break;
                }
                self.cursor += 1;
            }

            if self.cursor >= self.bytes.len() {
                return None;
            }

            match unsafe { *self.bytes.get_unchecked(self.cursor) } {
                b'\n' => {
                    self.cursor += 1;
                    continue;
                }
                b'#' => return self.handle_comment_fast(),
                _ => {
                    if let Some(entry) = self.parse_pair() {
                        return Some(entry);
                    }
                }
            }
        }
    }
}

impl<'a> Parser<'a> {
    #[inline]
    fn check_bom(&mut self) -> Option<Entry<'a>> {
        self.bom_checked = true;
        if self.bytes.starts_with(b"\xEF\xBB\xBF") {
            self.cursor += 3;
        }
        if let Some(idx) = self.input[self.cursor..].find('\u{FEFF}') {
            return Some(Entry::Error(Error::InvalidBom {
                offset: self.cursor + idx,
            }));
        }
        None
    }

    #[inline]
    fn handle_comment_fast(&mut self) -> Option<Entry<'a>> {
        self.cursor += 1;

        if self.options.include_comments {
            let line_start = self.cursor;

            // Fast newline search
            let mut line_end = line_start;
            while line_end < self.bytes.len() {
                let b = unsafe { *self.bytes.get_unchecked(line_end) };
                if b == b'\n' || b == b'\r' {
                    break;
                }
                line_end += 1;
            }

            // Set up state for scanning
            self.comment_state = Some(CommentScanState {
                line_start,
                line_end,
                scan_pos: 0,
                returned_comment: false,
            });

            return self.next_entry();
        } else {
            // Fast skip
            while self.cursor < self.bytes.len() {
                if unsafe { *self.bytes.get_unchecked(self.cursor) } == b'\n' {
                    break;
                }
                self.cursor += 1;
            }
        }

        if self.cursor < self.bytes.len() && self.bytes[self.cursor] == b'\n' {
            self.cursor += 1;
        }
        self.next_entry()
    }

    #[inline(always)]
    unsafe fn find_key_backwards_fast(&self, eq_pos: usize) -> Option<(usize, usize)> {
        if eq_pos == 0 {
            return None;
        }

        let mut key_end = eq_pos;

        // Skip whitespace (invalid but check)
        while key_end > 0 {
            let b = *self.bytes.get_unchecked(key_end - 1);
            if b != b' ' && b != b'\t' {
                break;
            }
            key_end -= 1;
        }

        if key_end == 0 {
            return None;
        }

        let mut key_start = key_end;

        // Fast backwards scan
        while key_start > 0 {
            let b = *self.bytes.get_unchecked(key_start - 1);
            if b.is_ascii_alphanumeric() || b == b'_' {
                key_start -= 1;
            } else {
                break;
            }
        }

        if key_start == key_end {
            return None;
        }

        // Validate
        let first_byte = *self.bytes.get_unchecked(key_start);
        if first_byte.is_ascii_digit() {
            return None;
        }

        if key_start > 0 {
            let prev = *self.bytes.get_unchecked(key_start - 1);
            if prev != b' ' && prev != b'\t' && prev != b'#' {
                return None;
            }
        }

        Some((key_start, key_end))
    }

    #[inline]
    fn parse_value_in_comment_fast(
        &self,
        value_start: usize,
        line_end: usize,
    ) -> Option<ParsedValue<'a>> {
        if value_start >= line_end {
            return Some(ParsedValue {
                value: Cow::Borrowed(""),
                value_start,
                raw_len: 0,
                quote: QuoteType::None,
            });
        }

        let first_byte = unsafe { *self.bytes.get_unchecked(value_start) };

        // Single-quoted - fast path
        if first_byte == b'\'' {
            let content_start = value_start + 1;
            let mut pos = content_start;
            while pos < line_end {
                if unsafe { *self.bytes.get_unchecked(pos) } == b'\'' {
                    return Some(ParsedValue {
                        value: Cow::Borrowed(&self.input[content_start..pos]),
                        value_start,
                        raw_len: pos + 1 - value_start,
                        quote: QuoteType::Single,
                    });
                }
                pos += 1;
            }
            return None;
        }

        // Double-quoted
        if first_byte == b'"' {
            let content_start = value_start + 1;
            let mut pos = content_start;

            // Fast scan for quote or escape
            while pos < line_end {
                let b = unsafe { *self.bytes.get_unchecked(pos) };
                if b == b'"' {
                    return Some(ParsedValue {
                        value: Cow::Borrowed(&self.input[content_start..pos]),
                        value_start,
                        raw_len: pos + 1 - value_start,
                        quote: QuoteType::Double,
                    });
                }
                if b == b'\\' {
                    // Has escapes - fallback to slow path
                    return self.parse_double_quoted_with_escapes(value_start, line_end);
                }
                pos += 1;
            }
            return None;
        }

        // Unquoted - stop at whitespace
        let mut pos = value_start;
        while pos < line_end {
            let b = unsafe { *self.bytes.get_unchecked(pos) };
            if b == b' ' || b == b'\t' {
                break;
            }
            pos += 1;
        }

        Some(ParsedValue {
            value: Cow::Borrowed(&self.input[value_start..pos]),
            value_start,
            raw_len: pos - value_start,
            quote: QuoteType::None,
        })
    }

    #[cold]
    fn parse_double_quoted_with_escapes(
        &self,
        start: usize,
        line_end: usize,
    ) -> Option<ParsedValue<'a>> {
        let mut pos = start + 1;
        let mut value = String::new();

        while pos < line_end {
            let b = unsafe { *self.bytes.get_unchecked(pos) };
            if b == b'\\' && pos + 1 < line_end {
                pos += 1;
                let c = unsafe { *self.bytes.get_unchecked(pos) };
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
                pos += 1;
            } else if b == b'"' {
                return Some(ParsedValue {
                    value: Cow::Owned(value),
                    value_start: start,
                    raw_len: pos + 1 - start,
                    quote: QuoteType::Double,
                });
            } else {
                value.push(b as char);
                pos += 1;
            }
        }
        None
    }

    fn parse_pair(&mut self) -> Option<Entry<'a>> {
        let export_span = self.consume_export_keyword();
        let is_exported = export_span.is_some();

        let key_start = self.cursor;

        // Fast key scan
        while self.cursor < self.bytes.len() {
            let b = unsafe { *self.bytes.get_unchecked(self.cursor) };
            if !b.is_ascii_alphanumeric() && b != b'_' {
                break;
            }
            self.cursor += 1;
        }
        let key_end = self.cursor;

        if key_start == key_end {
            // Fast whitespace skip
            while self.cursor < self.bytes.len() {
                let b = unsafe { *self.bytes.get_unchecked(self.cursor) };
                if b != b' ' && b != b'\t' {
                    break;
                }
                self.cursor += 1;
            }

            if self.cursor < self.bytes.len() && self.bytes[self.cursor] == b'=' {
                return Some(self.error_and_recover(Error::Generic {
                    offset: key_start,
                    message: "Empty key".into(),
                }));
            }

            if is_exported {
                self.skip_to_newline();
                if self.cursor < self.bytes.len() && self.bytes[self.cursor] == b'\n' {
                    self.cursor += 1;
                }
                return None;
            }

            self.skip_to_newline();
            if self.cursor < self.bytes.len() && self.bytes[self.cursor] == b'\n' {
                self.cursor += 1;
            }
            return None;
        }

        let key_str = &self.input[key_start..key_end];

        if unsafe { *self.bytes.get_unchecked(key_start) }.is_ascii_digit() {
            return Some(self.error_and_recover(Error::InvalidKey {
                offset: key_start,
                reason: "Key starts with digit".into(),
            }));
        }

        // Check whitespace before '='
        if self.cursor < self.bytes.len() && matches!(self.bytes[self.cursor], b' ' | b'\t') {
            let ws_start = self.cursor;
            while self.cursor < self.bytes.len() {
                let b = unsafe { *self.bytes.get_unchecked(self.cursor) };
                if b != b' ' && b != b'\t' {
                    break;
                }
                self.cursor += 1;
            }

            if self.cursor < self.bytes.len() && self.bytes[self.cursor] == b'=' {
                return Some(self.error_and_recover(Error::ForbiddenWhitespace {
                    offset: ws_start,
                    location: "between key and equals",
                }));
            }
        }

        if self.cursor >= self.bytes.len() || self.bytes[self.cursor] != b'=' {
            return Some(self.error_and_recover(Error::Expected {
                offset: self.cursor,
                expected: "'='",
            }));
        }
        self.cursor += 1;

        if self.cursor < self.bytes.len() && self.bytes[self.cursor] == b'=' {
            return Some(self.error_and_recover(Error::DoubleEquals {
                offset: self.cursor,
            }));
        }

        if self.cursor < self.bytes.len() && matches!(self.bytes[self.cursor], b' ' | b'\t') {
            return Some(self.error_and_recover(Error::ForbiddenWhitespace {
                offset: self.cursor,
                location: "after equals",
            }));
        }

        let value_start = self.cursor;
        let parsed_value = if self.cursor < self.bytes.len() && self.bytes[self.cursor] == b'\'' {
            self.parse_single_quoted_value(value_start)
        } else if self.cursor < self.bytes.len() && self.bytes[self.cursor] == b'"' {
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
                        export_span,
                        false,
                    )
                } else {
                    KeyValuePair::new_fast(key_str, pv.value, pv.quote, is_exported, false)
                };
                Entry::Pair(Box::new(pair))
            }
            Err(e) => Entry::Error(e),
        };

        // Check inline comment
        let saved = self.cursor;
        while self.cursor < self.bytes.len() {
            let b = unsafe { *self.bytes.get_unchecked(self.cursor) };
            if b != b' ' && b != b'\t' {
                break;
            }
            self.cursor += 1;
        }
        let has_comment = self.cursor < self.bytes.len() && self.bytes[self.cursor] == b'#';
        self.cursor = saved;

        if !has_comment {
            self.skip_to_newline();
        }

        if self.cursor < self.bytes.len() && self.bytes[self.cursor] == b'\n' {
            self.cursor += 1;
        }
        Some(entry)
    }
}

impl<'a> Parser<'a> {
    #[inline]
    fn parse_single_quoted_value(&mut self, start: usize) -> Result<ParsedValue<'a>, Error> {
        self.cursor += 1;
        let content_start = self.cursor;

        while self.cursor < self.bytes.len() {
            if unsafe { *self.bytes.get_unchecked(self.cursor) } == b'\'' {
                let content_end = self.cursor;
                self.cursor += 1;
                return Ok(ParsedValue {
                    value: Cow::Borrowed(&self.input[content_start..content_end]),
                    value_start: start,
                    raw_len: self.cursor - start,
                    quote: QuoteType::Single,
                });
            }
            self.cursor += 1;
        }

        self.cursor = self.bytes.len();
        Err(Error::UnclosedQuote {
            offset: start,
            quote_type: "single",
        })
    }

    #[inline]
    fn parse_double_quoted_value(&mut self, start: usize) -> Result<ParsedValue<'a>, Error> {
        self.cursor += 1;
        let content_start = self.cursor;

        // Fast path: scan for quote or escape
        while self.cursor < self.bytes.len() {
            let b = unsafe { *self.bytes.get_unchecked(self.cursor) };
            if b == b'"' {
                let content_end = self.cursor;
                self.cursor += 1;
                return Ok(ParsedValue {
                    value: Cow::Borrowed(&self.input[content_start..content_end]),
                    value_start: start,
                    raw_len: self.cursor - start,
                    quote: QuoteType::Double,
                });
            }
            if b == b'\\' {
                break;
            }
            self.cursor += 1;
        }

        // Slow path: has escapes
        self.cursor = content_start;
        let mut value = String::with_capacity(64);

        loop {
            if self.cursor >= self.bytes.len() {
                return Err(Error::UnclosedQuote {
                    offset: start,
                    quote_type: "double",
                });
            }
            let b = unsafe { *self.bytes.get_unchecked(self.cursor) };
            if b == b'\\' && self.cursor + 1 < self.bytes.len() {
                self.cursor += 1;
                let c = unsafe { *self.bytes.get_unchecked(self.cursor) };
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
        let start_pos = self.cursor;
        let mut needs_allocation = false;
        let mut trailing_backslash = false;

        loop {
            if self.cursor >= self.bytes.len() {
                break;
            }
            let line_start = self.cursor;

            let mut limit = self.cursor;
            let mut stop_char = None;
            while limit < self.bytes.len() {
                let b = unsafe { *self.bytes.get_unchecked(limit) };
                if matches!(b, b' ' | b'\t' | b'\n' | b'\r') {
                    stop_char = Some(b);
                    break;
                }
                limit += 1;
            }

            let stopped_at_eol = matches!(stop_char, Some(b'\n') | Some(b'\r') | None);
            let is_continuation = stopped_at_eol
                && limit > line_start
                && unsafe { *self.bytes.get_unchecked(limit - 1) } == b'\\';

            if is_continuation {
                needs_allocation = true;
                self.cursor = limit;
                if self.cursor < self.bytes.len() {
                    if unsafe { *self.bytes.get_unchecked(self.cursor) } == b'\r' {
                        self.cursor += 1;
                    }
                    if self.cursor < self.bytes.len()
                        && unsafe { *self.bytes.get_unchecked(self.cursor) } == b'\n'
                    {
                        self.cursor += 1;
                    }
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
                if self.cursor >= self.bytes.len() {
                    break;
                }
                let line_start = self.cursor;

                let mut limit = self.cursor;
                let mut stop_char = None;
                while limit < self.bytes.len() {
                    let b = unsafe { *self.bytes.get_unchecked(limit) };
                    if matches!(b, b' ' | b'\t' | b'\n' | b'\r') {
                        stop_char = Some(b);
                        break;
                    }
                    limit += 1;
                }

                let chunk = &self.input[self.cursor..limit];
                let stopped_at_eol = matches!(stop_char, Some(b'\n') | Some(b'\r') | None);
                let is_continuation = stopped_at_eol
                    && limit > line_start
                    && unsafe { *self.bytes.get_unchecked(limit - 1) } == b'\\';

                if is_continuation {
                    value.push_str(&chunk[..chunk.len() - 1]);
                    self.cursor = limit;
                    if self.cursor < self.bytes.len() {
                        if unsafe { *self.bytes.get_unchecked(self.cursor) } == b'\r' {
                            self.cursor += 1;
                        }
                        if self.cursor < self.bytes.len()
                            && unsafe { *self.bytes.get_unchecked(self.cursor) } == b'\n'
                        {
                            self.cursor += 1;
                        }
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
    #[inline]
    fn consume_export_keyword(&mut self) -> Option<Span> {
        if self.cursor + 6 < self.bytes.len()
            && unsafe { *self.bytes.get_unchecked(self.cursor..self.cursor + 6) == *b"export" }
        {
            let next = unsafe { *self.bytes.get_unchecked(self.cursor + 6) };
            if matches!(next, b' ' | b'\t') {
                let start_pos = self.cursor;
                self.cursor += 6;
                let end_pos = self.cursor;
                while self.cursor < self.bytes.len() {
                    let b = unsafe { *self.bytes.get_unchecked(self.cursor) };
                    if b != b' ' && b != b'\t' {
                        break;
                    }
                    self.cursor += 1;
                }
                return Some(Span::from_offsets(start_pos, end_pos));
            }
        }
        None
    }

    #[inline]
    fn skip_to_newline(&mut self) {
        while self.cursor < self.bytes.len() {
            if unsafe { *self.bytes.get_unchecked(self.cursor) } == b'\n' {
                break;
            }
            self.cursor += 1;
        }
    }

    fn error_and_recover(&mut self, err: Error) -> Entry<'a> {
        self.skip_to_newline();
        if self.cursor < self.bytes.len() {
            self.cursor += 1;
        }
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
