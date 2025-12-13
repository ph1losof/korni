use std::ops::Range;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParseOptions {
    pub include_comments: bool,
    pub track_positions: bool,
}

impl Default for ParseOptions {
    #[inline]
    fn default() -> Self {
        Self {
            include_comments: false,
            track_positions: false,
        }
    }
}

impl ParseOptions {
    #[inline]
    pub fn fast() -> Self {
        Self::default()
    }

    #[inline]
    pub fn full() -> Self {
        Self {
            include_comments: true,
            track_positions: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Position {
    pub line: usize,
    pub col: usize,
    pub offset: usize,
}

impl Position {
    #[inline]
    pub fn from_offset(offset: usize) -> Self {
        Self { line: 0, col: 0, offset }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Span {
    pub start: Position,
    pub end: Position,
}

impl Span {
    #[inline]
    pub fn new(start: Position, end: Position) -> Self {
        Self { start, end }
    }

    #[inline]
    pub fn from_offsets(start: usize, end: usize) -> Self {
        Self {
            start: Position::from_offset(start),
            end: Position::from_offset(end),
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.end.offset - self.start.offset
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[inline]
    pub fn range(&self) -> Range<usize> {
        self.start.offset..self.end.offset
    }
}

/// Type of quoting used for a value
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum QuoteType {
    Single, // '
    Double, // "
    #[default]
    None,   // No quotes
}

use std::borrow::Cow;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Entry<'a> {
    Comment(Span),
    Pair(KeyValuePair<'a>),
    Error(Error),
}

use crate::error::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyValuePair<'a> {

    pub key: Cow<'a, str>,
    pub key_span: Option<Span>,

    pub value: Cow<'a, str>,
    pub value_span: Option<Span>,

    pub quote: QuoteType,
    pub open_quote_pos: Option<Position>,
    pub close_quote_pos: Option<Position>,

    pub equals_pos: Option<Position>,

    pub is_exported: bool,
    pub is_comment: bool,
}

impl<'a> KeyValuePair<'a> {
    #[inline]
    pub fn new_fast(
        key: impl Into<Cow<'a, str>>,
        value: Cow<'a, str>,
        quote: QuoteType,
        is_exported: bool,
        is_comment: bool,
    ) -> Self {
        Self {
            key: key.into(),
            key_span: None,
            value,
            value_span: None,
            quote,
            open_quote_pos: None,
            close_quote_pos: None,
            equals_pos: None,
            is_exported,
            is_comment,
        }
    }

    #[inline]
    pub fn new(
        key: &'a str,
        key_start: usize,
        value: Cow<'a, str>,
        value_start: usize,
        raw_len: usize,
        quote: QuoteType,
        is_exported: bool,
        is_comment: bool,
    ) -> Self {
        let key_end = key_start + key.len();
        let equals_offset = key_end; // = is right after key according to the spec
        let value_end = value_start + raw_len;
        
        Self {
            key: Cow::Borrowed(key),
            key_span: Some(Span::from_offsets(key_start, key_end)),
            value,
            value_span: Some(Span::from_offsets(value_start, value_end)),
            quote,
            open_quote_pos: if quote != QuoteType::None { Some(Position::from_offset(value_start)) } else { None },
            close_quote_pos: if quote != QuoteType::None { Some(Position::from_offset(value_end - 1)) } else { None },
            equals_pos: Some(Position::from_offset(equals_offset)),
            is_exported,
            is_comment,
        }
    }
}

impl<'a> Entry<'a> {
    pub fn into_owned(self) -> Entry<'static> {
        match self {
            Entry::Pair(kv) => Entry::Pair(kv.into_owned()),
            Entry::Comment(span) => Entry::Comment(span),
            Entry::Error(e) => Entry::Error(e),
        }
    }
}

impl<'a> KeyValuePair<'a> {
    pub fn into_owned(self) -> KeyValuePair<'static> {
        KeyValuePair {
            key: Cow::Owned(self.key.into_owned()),
            key_span: self.key_span,
            value: Cow::Owned(self.value.into_owned()),
            value_span: self.value_span,
            quote: self.quote,
            open_quote_pos: self.open_quote_pos,
            close_quote_pos: self.close_quote_pos,
            equals_pos: self.equals_pos,
            is_exported: self.is_exported,
            is_comment: self.is_comment,
        }
    }
}
