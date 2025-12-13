use crate::{Entry, Parser};

/// Iterator over environment entries
pub struct EnvIterator<'a> {
    parser: Parser<'a>,
}

impl<'a> EnvIterator<'a> {
    pub fn new(parser: Parser<'a>) -> Self {
        Self { parser }
    }
}

impl<'a> Iterator for EnvIterator<'a> {
    type Item = Entry<'a>;
    
    fn next(&mut self) -> Option<Self::Item> {
        self.parser.next_entry()
    }
}
