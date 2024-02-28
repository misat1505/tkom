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

impl Position {
    pub fn new(line: u32, column: u32, offset: usize) -> Self {
        Position { line, column, offset }
    }
}

// impl Display for Position {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         write!(f, "[{}:{}]", self.line, self.column)
//     }
// }

// impl Debug for Position {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         write!(f, "[{}:{},{}]", self.line, self.column, self.offset)
//     }
// }

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
            current_position: Position::new(0, 0, 0)
        }
    }

    fn read_char(&mut self) -> Result<char> {
        let new_char = match self.try_handle_newline()? {
            None => self.process_char()?,
            Some(c) => c,
        };

        Ok(new_char)
    }

    fn try_handle_newline(&mut self) -> Result<Option<char>> {
        let buffer = self.src.fill_buf()?;
        
        if let Some(&first_char) = buffer.get(0) {
            if let Some(&second_char) = buffer.get(1) {
                let skippable = [b'\n', b'\r'];
                if skippable.contains(&first_char) {
                    let mut newline_sequence = vec![first_char];
                    self.src.consume(1);
                    if skippable.contains(&second_char) {
                        newline_sequence.push(second_char);
                        self.src.consume(1);
                    }
                    self.newline = Some(newline_sequence.clone());
                    return Ok(Some('\n'));
                }
            }
        }
    
        Ok(None)
    }

    fn process_char(&mut self) -> Result<char> {
        let buffer = self.src.fill_buf()?;
    
        if buffer.is_empty() {
            return Ok(ETX);
        }
    
        let first_byte = *buffer.get(0).unwrap();
        let char = first_byte as char;

        self.src.consume(1);
    
        Ok(char)
    }

    fn update_position(&mut self, read_character: char) {
        match read_character {
            STX => {
                self.current_position = Position::new(1, 1, 0);
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
