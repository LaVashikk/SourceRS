use crate::error::{Error, Result};

pub(crate) enum Token<'de> {
    Str(&'de str),
    Open,
    Close,
    Eof,
}

/// Bytes that end an unquoted token
static DELIM: [bool; 256] = {
    let mut t = [false; 256];
    t[b' ' as usize] = true;
    t[b'\t' as usize] = true;
    t[b'\n' as usize] = true;
    t[b'\r' as usize] = true;
    t[0x0c] = true;
    t[b'{' as usize] = true;
    t[b'}' as usize] = true;
    t[b'"' as usize] = true;
    t
};

/// Splits KeyValues text into tokens borrowed from the input
///
/// Only the byte offset is tracked. Line and column are counted from it when an error is built,
/// so the hot loops never touch them
pub(crate) struct Lexer<'de> {
    input: &'de str,
    pos: usize,
    tok: usize,
}

impl<'de> Lexer<'de> {
    pub fn new(input: &'de str) -> Self {
        // Notepad-saved configs start with a BOM, which would otherwise glue itself to the first key
        let pos = if input.starts_with('\u{feff}') { 3 } else { 0 };
        Lexer { input, pos, tok: pos }
    }

    pub fn next(&mut self) -> Result<Token<'de>> {
        self.skip_trivia();
        self.tok = self.pos;
        let bytes = self.input.as_bytes();
        // A bunch of Valve .res files (localizationdialog.res & co) have a C string terminator after the last brace,
        // the engine stops reading there, so do we
        let Some(&b) = bytes.get(self.pos).filter(|&&b| b != 0) else {
            return Ok(Token::Eof);
        };

        match b {
            b'{' => {
                self.pos += 1;
                Ok(Token::Open)
            }
            b'}' => {
                self.pos += 1;
                Ok(Token::Close)
            }
            b'"' => self.quoted(),
            _ => {
                let start = self.pos;
                let mut i = start;
                while i < bytes.len() && !DELIM[bytes[i] as usize] {
                    i += 1;
                }
                self.pos = i;
                Ok(Token::Str(&self.input[start..i]))
            }
        }
    }

    fn quoted(&mut self) -> Result<Token<'de>> {
        let start = self.pos + 1;
        // No escapes, the first quote closes the string. Same in the engine's KeyValues and in Hammer's
        // tokenizer, and that's how `"GameExeDir" "...\bin\win64\"` from a real gameconfig.txt reads fine
        let Some(len) = self.input.as_bytes()[start..].iter().position(|&b| b == b'"') else {
            return Err(self.error("unexpected end of file in a quoted string"));
        };
        self.pos = start + len + 1;
        Ok(Token::Str(&self.input[start..start + len]))
    }

    fn skip_trivia(&mut self) {
        let bytes = self.input.as_bytes();
        let mut i = self.pos;
        loop {
            match bytes.get(i) {
                Some(b) if b.is_ascii_whitespace() => i += 1,
                Some(b'/') if bytes.get(i + 1) == Some(&b'/') => {
                    i = match bytes[i + 2..].iter().position(|&b| b == b'\n') {
                        Some(off) => i + 2 + off + 1,
                        None => bytes.len(),
                    };
                }
                _ => break,
            }
        }
        self.pos = i;
    }

    /// A syntax error pointing at the start of the last token
    pub fn error(&self, msg: &str) -> Error {
        let before = &self.input.as_bytes()[..self.tok];
        let line_start = before.iter().rposition(|&b| b == b'\n').map_or(0, |p| p + 1);
        Error::Syntax {
            line: before.iter().filter(|&&b| b == b'\n').count() + 1,
            column: self.input[line_start..self.tok].chars().count() + 1,
            msg: msg.to_owned(),
        }
    }
}
