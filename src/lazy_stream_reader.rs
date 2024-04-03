use std::error::Error;
use std::fmt::Debug;
use std::io::BufRead;

pub const STX: char = '\u{2}';
pub const ETX: char = '\u{3}';

pub trait ILazyStreamReader {
    fn current(&self) -> &char;
    fn next(&mut self) -> Result<&char, Box<dyn Error>>;
    fn position(&self) -> Position;
}

#[derive(PartialEq, Eq, Clone, Copy)]
pub struct Position {
    pub line: u32,
    pub column: u32,
    pub offset: usize,
}

impl Debug for Position {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "line: {}, column: {}", self.line, self.column)
    }
}

impl Position {
    pub fn new(line: u32, column: u32, offset: usize) -> Self {
        Position {
            line,
            column,
            offset,
        }
    }
}

pub struct LazyStreamReader<R: BufRead> {
    src: R,
    current_line: String,
    current_char: char,
    newline: Option<Vec<u8>>,
    current_position: Position,
}

impl<R: BufRead> ILazyStreamReader for LazyStreamReader<R> {
    fn current(&self) -> &char {
        &self.current_char
    }

    fn next(&mut self) -> Result<&char, Box<dyn Error>> {
        let new_char = self.read_char()?;
        self.update_position(self.current_char);
        self.current_char = new_char;
        Ok(&self.current_char)
    }

    fn position(&self) -> Position {
        self.current_position
    }
}

impl<R: BufRead> LazyStreamReader<R> {
    pub fn new(src: R) -> LazyStreamReader<R> {
        LazyStreamReader {
            src,
            current_line: String::new(),
            current_char: STX,
            newline: None,
            current_position: Position::new(0, 0, 0),
        }
    }

    fn read_char(&mut self) -> Result<char, Box<dyn Error>> {
        let new_char = match self.try_handle_newline()? {
            None => self.process_char()?,
            Some(c) => c,
        };

        Ok(new_char)
    }

    fn try_handle_newline(&mut self) -> Result<Option<char>, Box<dyn Error>> {
        // TODO: nie wciagac calego buffera, jakims read_exact
        let buffer = self.src.fill_buf()?;

        if let Some(&first_char) = buffer.get(0) {
            if let Some(&second_char) = buffer.get(1) {
                if first_char == b'\r' {
                    let mut newline_sequence = vec![first_char];
                    self.src.consume(1);
                    if second_char == b'\n' {
                        newline_sequence.push(second_char);
                        self.src.consume(1);
                    }
                    self.newline = Some(newline_sequence.clone());
                    return Ok(Some('\n'));
                } else if first_char == b'\n' {
                    self.src.consume(1);
                    self.newline = Some(vec![first_char]);
                    return Ok(Some('\n'));
                }
            }
        }

        Ok(None)
    }

    fn process_char(&mut self) -> Result<char, Box<dyn Error>> {
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
                self.current_line = String::new();
            }
            char => {
                self.current_position.offset += self.current_char.len_utf8();
                self.current_position.column += 1;
                self.current_line.push(char);
            }
        };
    }

    pub fn error_code_snippet(&mut self) -> String {
        let mut buffer = String::new();
        let _ = self.src.read_line(&mut buffer);

        let spaces = " ".repeat((self.position().column - 1) as usize);
        let caret_string = format!("{}^", spaces);

        format!(
            "\nAt line:\n{}{}{}{}",
            self.current_line, self.current_char, buffer, caret_string
        )
    }
}

#[cfg(test)]
mod tests {
    use std::io::BufReader;

    use super::*;

    #[test]
    fn test_lazy_stream_reader() {
        let code = BufReader::new(
            r#"hello
world"#
                .as_bytes(),
        );
        let mut stream_reader = LazyStreamReader::new(code);

        let expected: Vec<(char, u32, u32)> = vec![
            ('h', 1, 1),
            ('e', 1, 2),
            ('l', 1, 3),
            ('l', 1, 4),
            ('o', 1, 5),
            ('\n', 1, 6),
            ('w', 2, 1),
            ('o', 2, 2),
            ('r', 2, 3),
            ('l', 2, 4),
            ('d', 2, 5),
            (ETX, 2, 6),
            (ETX, 2, 6),
        ];

        assert_eq!(*stream_reader.current(), STX);
        assert_eq!(stream_reader.position().line, 0);
        assert_eq!(stream_reader.position().column, 0);

        for (exp_char, exp_line, exp_col) in &expected {
            assert_eq!(*stream_reader.next().unwrap(), *exp_char);
            assert_eq!(stream_reader.position().line, *exp_line);
            assert_eq!(stream_reader.position().column, *exp_col);
        }
    }
}
