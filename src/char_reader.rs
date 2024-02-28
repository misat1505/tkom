use read_char::read_next_char;
use std::fmt::{Debug, Display};
use std::io;
use std::io::BufRead;
use thiserror::Error;

pub const STX: char = '\u{2}';
pub const ETX: char = '\u{3}';

pub trait CharRead {
    fn current(&self) -> &char;
    fn next(&mut self) -> Result<&char>;
    fn position(&self) -> Position;
}

#[derive(PartialEq, Eq, Clone, Copy)]
pub struct Position {
    pub line: u32,
    pub column: u32,
    pub offset: usize,
}

impl Display for Position {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}:{}]", self.line, self.column)
    }
}

impl Debug for Position {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}:{},{}]", self.line, self.column, self.offset)
    }
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("inconsistent newline sequence encountered: expected {expected:?}, got {got:?}")]
    InconsistentNewline { expected: Vec<u8>, got: Vec<u8> },
    #[error("reading character")]
    Read(#[from] read_char::Error),
}

impl From<io::Error> for Error {
    fn from(value: io::Error) -> Self {
        Error::Read(read_char::Error::Io(value))
    }
}

pub struct CharReader<R: BufRead> {
    src: R,
    current_char: char,
    char_len: usize,
    newline: Option<Vec<u8>>,
    current_position: Position,
}

impl<R: BufRead> CharRead for CharReader<R> {
    fn current(&self) -> &char {
        &self.current_char
    }

    fn next(&mut self) -> Result<&char> {
        let new_char = self.read_char()?;
        self.update_position(self.current_char);
        self.current_char = new_char;
        Ok(&self.current_char)
    }

    fn position(&self) -> Position {
        self.current_position
    }
}

impl<R: BufRead> CharReader<R> {
    pub fn new(src: R) -> CharReader<R> {
        CharReader {
            src,
            current_char: STX,
            char_len: 0,
            newline: None,
            current_position: Position {
                line: 0,
                column: 0,
                offset: 0,
            },
        }
    }

    fn read_char(&mut self) -> Result<char> {
        let new_char = match self.try_process_newline()? {
            None => self.process_char()?,
            Some(c) => c,
        };

        Ok(new_char)
    }

    fn try_process_newline(&mut self) -> Result<Option<char>> {
        let newline: Vec<u8> = match self.src.fill_buf()? {
            &[first, ..] if [b'\n', b'\r'].contains(&first) => {
                self.src.consume(1);
                let mut bytes = vec![first];
                if let &[second, ..] = self.src.fill_buf()? {
                    if [b"\n\r", b"\r\n"].contains(&&[first, second]) {
                        self.src.consume(1);
                        bytes.push(second)
                    }
                };
                bytes
            }
            _ => return Ok(None),
        };

        if let Some(expected_newline) = &self.newline {
            if newline != *expected_newline {
                return Err(Error::InconsistentNewline {
                    expected: expected_newline.clone(),
                    got: newline,
                });
            }
        } else {
            self.newline = Some(newline.clone())
        }

        Ok(Some('\n'))
    }

    fn process_char(&mut self) -> Result<char> {
        let char = match read_next_char(&mut self.src) {
            Ok(c) => c,
            Err(read_char::Error::EOF) => ETX,
            Err(x) => return Err(Error::Read(x)),
        };
        self.char_len = char.len_utf8();
        Ok(char)
    }

    fn update_position(&mut self, read_character: char) {
        match read_character {
            STX => {
                self.current_position = Position {
                    line: 1,
                    column: 1,
                    offset: 0,
                }
            }
            ETX => {}
            '\n' => {
                self.current_position.offset += self.newline.as_ref().unwrap().len();
                self.current_position.line += 1;
                self.current_position.column = 1;
            }
            _ => {
                self.current_position.offset += self.current_char.len_utf8();
                self.current_position.column += 1;
            }
        };
    }
}
