use std::io::BufRead;

use phf::phf_map;

use crate::errors::{IError, ErrorSeverity, LexerError};
use crate::lazy_stream_reader::{ILazyStreamReader, LazyStreamReader, Position, ETX};
use crate::tokens::{Token, TokenCategory, TokenValue};

pub struct LexerOptions {
    pub max_comment_length: u32,
    pub max_identifier_length: u32,
}

pub trait ILexer {
    fn current(&self) -> &Option<Token>;
    fn next(&mut self) -> Result<Token, Box<dyn IError>>;
}

pub struct Lexer<T: BufRead> {
    pub src: LazyStreamReader<T>,
    current: Option<Token>,
    position: Position,
    options: LexerOptions,
    on_warning: fn(warning: Box<dyn IError>),
}

impl<T: BufRead> ILexer for Lexer<T> {
    fn current(&self) -> &Option<Token> {
        &self.current
    }

    fn next(&mut self) -> Result<Token, Box<dyn IError>> {
        self.generate_token()
    }
}

impl<T: BufRead> Lexer<T> {
    pub fn new(src: LazyStreamReader<T>, options: LexerOptions, on_warning: fn(warning: Box<dyn IError>)) -> Self {
        let position = src.position().clone();
        Lexer {
            src,
            current: None,
            position,
            options,
            on_warning,
        }
    }

    #[allow(irrefutable_let_patterns)]
    pub fn generate_token(&mut self) -> Result<Token, Box<dyn IError>> {
        self.skip_whitespaces();
        self.position = self.src.position().clone();

        let result_methods = [
            Self::try_generating_sign,
            Self::try_generating_operator,
            Self::try_generating_comment,
            Self::try_generating_string,
            Self::try_generating_number,
            Self::try_creating_identifier_or_keyword,
        ];

        for generator in &result_methods {
            if let Some(token) = generator(self)? {
                self.current = Some(token.clone());
                return Ok(token);
            }
        }

        Err(self.create_lexer_error(String::from("Unexpected token")))
    }

    fn skip_whitespaces(&mut self) {
        while self.src.current().is_whitespace() {
            let _ = self.src.next();
        }
    }

    fn try_generating_comment(&mut self) -> Result<Option<Token>, Box<dyn IError>> {
        let current_char = self.src.current();
        if *current_char != '#' {
            return Ok(None);
        }

        let mut comment = String::new();
        while let Ok(current) = self.src.next() {
            if *current == '\n' || *current == ETX {
                break;
            }
            if (comment.len() as u32) == self.options.max_comment_length {
                return Err(self.create_lexer_error(format!("Comment too long. Max comment length: {}", self.options.max_comment_length)));
            }
            comment.push(*current);
        }

        Ok(Some(Token {
            category: TokenCategory::Comment,
            value: TokenValue::String(comment),
            position: self.position,
        }))
    }

    fn try_generating_sign(&mut self) -> Result<Option<Token>, Box<dyn IError>> {
        let current_char = self.src.current();
        match SIGNS.get(current_char) {
            None => Ok(None),
            Some(token_category) => {
                let token = Token {
                    category: token_category.clone(),
                    value: TokenValue::Null,
                    position: self.position,
                };
                let _ = self.src.next();
                Ok(Some(token))
            }
        }
    }

    fn try_generating_operator(&mut self) -> Result<Option<Token>, Box<dyn IError>> {
        let current_char = self.src.current();
        let token = match current_char {
            '+' => Some(self.single_char(TokenCategory::Plus)),
            '*' => Some(self.single_char(TokenCategory::Multiply)),
            '/' => Some(self.single_char(TokenCategory::Divide)),
            '-' => Some(self.extend_to_next('>', TokenCategory::Minus, TokenCategory::Arrow)),
            '<' => Some(self.extend_to_next('=', TokenCategory::Less, TokenCategory::LessOrEqual)),
            '>' => Some(self.extend_to_next('=', TokenCategory::Greater, TokenCategory::GreaterOrEqual)),
            '!' => Some(self.extend_to_next('=', TokenCategory::Negate, TokenCategory::NotEqual)),
            '=' => Some(self.extend_to_next('=', TokenCategory::Assign, TokenCategory::Equal)),
            '&' => Some(self.extend_to_next('&', TokenCategory::Reference, TokenCategory::And)),
            '|' => Some(self.extend_to_next_or_warning('|', TokenCategory::Or)),
            _ => None,
        };
        Ok(token)
    }

    fn single_char(&mut self, category: TokenCategory) -> Token {
        let _ = self.src.next();
        Token {
            category,
            value: TokenValue::Null,
            position: self.position,
        }
    }

    fn extend_to_next(&mut self, char_to_search: char, not_found: TokenCategory, found: TokenCategory) -> Token {
        let next_char = self.src.next().unwrap();
        if *next_char == char_to_search {
            let _ = self.src.next();
            return Token {
                category: found,
                value: TokenValue::Null,
                position: self.position,
            };
        }
        return Token {
            category: not_found,
            value: TokenValue::Null,
            position: self.position,
        };
    }

    fn extend_to_next_or_warning(&mut self, char_to_search: char, found: TokenCategory) -> Token {
        let next_char = self.src.next().unwrap();
        if *next_char == char_to_search {
            let _ = self.src.next();
        } else {
            (self.on_warning)(Box::new(LexerError::new(
                ErrorSeverity::LOW,
                self.prepare_warning_message(format!("Expected '{}'", char_to_search)),
            )));
        }
        return Token {
            category: found,
            value: TokenValue::Null,
            position: self.position,
        };
    }

    fn try_generating_string(&mut self) -> Result<Option<Token>, Box<dyn IError>> {
        // current_char do lexera
        let mut current_char = self.src.current().clone();
        if current_char != '"' {
            return Ok(None);
        }
        let mut created_string = String::new();
        current_char = self.src.next().unwrap().clone();
        while current_char != '"' {
            // escaping
            if current_char == '\\' {
                let next_char = self.src.next().unwrap().clone();
                match ESCAPES.get(&next_char) {
                    Some(char) => {
                        created_string.push(*char);
                        current_char = *self.src.next().unwrap();
                        continue;
                    }
                    None => {
                        (self.on_warning)(Box::new(LexerError::new(
                            ErrorSeverity::LOW,
                            self.prepare_warning_message(format!("Invalid escape symbol detected '\\{}'", next_char)),
                        )));
                        let default_escape = '\\';
                        created_string.push(default_escape);
                        current_char = next_char;
                        continue;
                    }
                }
            }
            if current_char == '\n' {
                return Err(self.create_lexer_error(String::from("Unexpected newline in string")));
            }
            if current_char == ETX {
                (self.on_warning)(Box::new(LexerError::new(
                    ErrorSeverity::LOW,
                    self.prepare_warning_message(String::from("String not closed")),
                )));
                return Ok(Some(Token {
                    category: TokenCategory::StringValue,
                    value: TokenValue::String(created_string),
                    position: self.position,
                }));
            }
            created_string.push(current_char);
            current_char = self.src.next().unwrap().clone();
        }
        // consume next "
        let _ = self.src.next();
        Ok(Some(Token {
            category: TokenCategory::StringValue,
            value: TokenValue::String(created_string),
            position: self.position,
        }))
    }

    fn try_generating_number(&mut self) -> Result<Option<Token>, Box<dyn IError>> {
        let mut current_char = self.src.current().clone();
        if !current_char.is_ascii_digit() {
            return Ok(None);
        }

        let mut decimal = 0;
        if current_char != '0' {
            (decimal, _) = self.parse_integer()?;
        } else {
            let next_char = self.src.next().unwrap();
            if next_char.is_ascii_digit() {
                return Err(self.create_lexer_error(String::from("Cannot prefix number with 0's.")));
            }
        }

        current_char = self.src.current().clone();
        if current_char != '.' {
            return Ok(Some(Token {
                category: TokenCategory::I64Value,
                value: TokenValue::I64(decimal),
                position: self.position,
            }));
        }

        let _ = self.src.next();
        let (fraction, fraction_length) = self.parse_integer()?;
        let float_value = Self::merge_to_float(decimal, fraction, fraction_length);
        Ok(Some(Token {
            category: TokenCategory::F64Value,
            value: TokenValue::F64(float_value),
            position: self.position,
        }))
    }

    fn parse_integer(&mut self) -> Result<(i64, i64), Box<dyn IError>> {
        let mut current_char = self.src.current();
        let mut length = 0;
        let mut total: i64 = 0;
        while current_char.is_ascii_digit() {
            let digit = *current_char as i64 - '0' as i64;
            match total.checked_mul(10) {
                Some(result) => total = result,
                None => {
                    return Err(self.create_lexer_error(String::from("Overflow occurred while parsing integer")));
                }
            }
            match total.checked_add(digit) {
                Some(result) => {
                    total = result;
                    length += 1;
                    current_char = self.src.next().unwrap();
                }
                None => {
                    return Err(self.create_lexer_error(String::from("Overflow occurred while parsing integer")));
                }
            }
        }
        Ok((total, length))
    }

    fn merge_to_float(decimal: i64, fraction: i64, fraction_length: i64) -> f64 {
        let fraction_value = fraction as f64 / f64::powi(10.0, fraction_length as i32);
        let float_value = decimal as f64 + fraction_value;
        float_value
    }

    fn try_creating_identifier_or_keyword(&mut self) -> Result<Option<Token>, Box<dyn IError>> {
        let mut current_char = self.src.current().clone();
        if !current_char.is_ascii_alphabetic() {
            return Ok(None);
        }
        let mut created_string = String::new();
        while current_char.is_ascii_digit() || current_char.is_ascii_alphabetic() || current_char == '_' {
            if (created_string.len() as u32) == self.options.max_identifier_length {
                return Err(self.create_lexer_error(format!(
                    "Identifier name too long. Max identifier length: {}",
                    self.options.max_identifier_length
                )));
            }
            created_string.push(current_char);
            current_char = self.src.next().unwrap().clone();
        }
        match KEYWORDS.get(created_string.as_str()) {
            Some(category) => Ok(Some(Token {
                category: category.clone(),
                value: TokenValue::Null,
                position: self.position,
            })),
            None => Ok(Some(Token {
                category: TokenCategory::Identifier,
                value: TokenValue::String(created_string),
                position: self.position,
            })),
        }
    }

    fn create_lexer_error(&mut self, text: String) -> Box<dyn IError> {
        let position = self.src.position();
        let code_snippet = self.src.error_code_snippet();
        let message = format!("\n{}\nAt {:?}\n{}\n", text, position, code_snippet);
        Box::new(LexerError::new(ErrorSeverity::HIGH, message))
    }

    fn prepare_warning_message(&self, text: String) -> String {
        let position = self.src.position();
        format!("\nWarning:\n{}\nAt {:?}\n", text, position)
    }
}

static SIGNS: phf::Map<char, TokenCategory> = phf_map! {
    '('     => TokenCategory::ParenOpen,
    ')'     => TokenCategory::ParenClose,
    '['     => TokenCategory::BracketOpen,
    ']'     => TokenCategory::BracketClose,
    '{'     => TokenCategory::BraceOpen,
    '}'     => TokenCategory::BraceClose,
    ';'     => TokenCategory::Semicolon,
    ':'     => TokenCategory::Colon,
    ','     => TokenCategory::Comma,
    '\u{2}' => TokenCategory::STX,
    '\u{3}' => TokenCategory::ETX,

};

static KEYWORDS: phf::Map<&'static str, TokenCategory> = phf_map! {
    "fn" => TokenCategory::Fn,
    "for" => TokenCategory::For,
    "if" => TokenCategory::If,
    "else" => TokenCategory::Else,
    "return" => TokenCategory::Return,
    "i64" => TokenCategory::I64,
    "f64" => TokenCategory::F64,
    "str" => TokenCategory::String,
    "void" => TokenCategory::Void,
    "bool" => TokenCategory::Bool,
    "true" => TokenCategory::True,
    "false" => TokenCategory::False,
    "as" => TokenCategory::As,
    "switch" => TokenCategory::Switch,
    "break" => TokenCategory::Break
};

static ESCAPES: phf::Map<char, char> = phf_map! {
    'n'  => '\n',
    'r'  => '\r',
    't'  => '\t',
    '"'  => '"',
    '\\' => '\\',
};
