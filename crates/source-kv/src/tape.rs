//! Flat, borrowed form of a document that the serde layer walks
//!
//! Nodes sit in one `Vec` in depth-first order, an object knows where its subtree ends,
//! so siblings are reached by jumping over it. Strings point into the input (or into a `Value`),
//! which leaves one allocation for the whole document instead of a few per entry

use crate::error::Result;
use crate::lexer::{Lexer, Token};
use crate::value::Value;

/// Same cap as the engine. Parsing itself doesn't care, but everything that walks the tree afterwards
/// is recursive, and 300 KB of `a{a{a{` used to kill the process with a stack overflow
const MAX_DEPTH: usize = 100;

pub(crate) struct Node<'a> {
    pub key: &'a str,
    pub val: NodeVal<'a>,
}

pub(crate) enum NodeVal<'a> {
    Str(&'a str),
    /// `len` direct children, the subtree ends right before node `end`
    Obj { len: u32, end: u32 },
}

pub(crate) struct Tape<'a> {
    pub nodes: Vec<Node<'a>>,
}

impl<'a> Tape<'a> {
    /// Parses a whole document. Node 0 is a nameless object holding the top-level entries
    pub fn parse(input: &'a str) -> Result<Self> {
        let mut lx = Lexer::new(input);
        // Overshoots for most files, untouched pages cost nothing and the tape dies right after use
        let mut nodes = Vec::with_capacity(input.len() / 16 + 1);
        nodes.push(Node { key: "", val: NodeVal::Obj { len: 0, end: 0 } });
        // (node index, children so far) of every object still open
        let mut open: Vec<(u32, u32)> = vec![(0, 0)];

        loop {
            let key = match lx.next()? {
                Token::Str(k) => k,
                Token::Close if open.len() > 1 => {
                    let (idx, len) = open.pop().unwrap();
                    nodes[idx as usize].val = NodeVal::Obj { len, end: nodes.len() as u32 };
                    continue;
                }

                Token::Eof if open.len() == 1 => break,
                Token::Eof => return Err(lx.error("expected '}', found end of file")),
                Token::Open | Token::Close => return Err(lx.error("expected a key")),
            };
            open.last_mut().unwrap().1 += 1;

            match lx.next()? {
                Token::Str(v) => nodes.push(Node { key, val: NodeVal::Str(v) }),
                
                Token::Open if open.len() > MAX_DEPTH => return Err(lx.error("blocks are nested deeper than 100 levels")),
                Token::Open => {
                    open.push((nodes.len() as u32, 0));
                    nodes.push(Node { key, val: NodeVal::Obj { len: 0, end: 0 } });
                }
                
                Token::Close => return Err(lx.error("expected a value or '{', found '}'")),
                Token::Eof => return Err(lx.error("key without a value at the end of file")),
            }
        }

        let len = open[0].1;
        nodes[0].val = NodeVal::Obj { len, end: nodes.len() as u32 };

        Ok(Tape { nodes })
    }

    /// Borrows an existing `Value`. Node 0 is the value itself
    pub fn from_value(value: &'a Value) -> Self {
        let mut tape = Tape { nodes: Vec::new() };
        tape.push_value("", value);
        tape
    }

    fn push_value(&mut self, key: &'a str, value: &'a Value) {
        match value {
            Value::Str(s) => self.nodes.push(Node { key, val: NodeVal::Str(s) }),
            Value::Obj(entries) => {
                let idx = self.nodes.len();
                self.nodes.push(Node { key, val: NodeVal::Obj { len: entries.len() as u32, end: 0 } });
                
                for (k, v) in entries {
                    self.push_value(k, v);
                }

                let end = self.nodes.len() as u32;
                if let NodeVal::Obj { end: e, .. } = &mut self.nodes[idx].val {
                    *e = end;
                }
            }
        }
    }

    /// Builds an owned `Value` out of node `idx`
    pub fn to_value(&self, idx: usize) -> Value {
        match self.nodes[idx].val {
            NodeVal::Str(s) => Value::Str(s.to_owned()),
            NodeVal::Obj { len, .. } => {
                let mut entries = Vec::with_capacity(len as usize);
                
                for c in self.children(idx) {
                    entries.push((self.nodes[c].key.to_owned(), self.to_value(c)));
                }
                
                Value::Obj(entries)
            }
        }
    }

    /// Index of the node right after `idx` and its subtree
    #[inline]
    pub fn skip(&self, idx: usize) -> usize {
        match self.nodes[idx].val {
            NodeVal::Obj { end, .. } => end as usize,
            NodeVal::Str(_) => idx + 1,
        }
    }

    pub fn children(&self, idx: usize) -> impl Iterator<Item = usize> + '_ {
        let end = self.skip(idx);
        let mut c = idx + 1;
        std::iter::from_fn(move || {
            (c < end).then(|| {
                let cur = c;
                c = self.skip(cur);
                cur
            })
        })
    }
}
