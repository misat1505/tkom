use std::io::BufRead;

use phf::phf_map;

use crate::lazy_stream_reader::{ILazyStreamReader, LazyStreamReader, Position, ETX};
use crate::tokens::{Token, TokenCategory, TokenValue};

pub trait ILexer<T: BufRead> {
    fn new(src: LazyStreamReader<T>, options: LexerOptions) -> Self;
    fn next(&mut self) -> &Option<Token>;
    fn current(&self) -> &Option<Token>;
}

pub struct LexerOptions {
    pub max_comment_length: u32,
    pub max_identifier_length: u32,
}

pub struct Lexer<T: BufRead> {
    pub src: LazyStreamReader<T>,
    current: Option<Token>,
    position: Position,
    options: LexerOptions
}

impl<T: BufRead> ILexer<T> for Lexer<T> {
    fn new(src: LazyStreamReader<T>, options: LexerOptions) -> Self {
        let position = src.position().clone();
        Lexer { src, current: None, position, options }
    }

    fn current(&self) -> &Option<Token> {
        &self.current
    }

    fn next(&mut self) -> &Option<Token> {
        &self.current
    }
}

impl<T: BufRead> Lexer<T> {
    pub fn generate_token(&mut self) -> Option<Token> {
        self.skip_whitespaces();
        self.position = self.src.position().clone();

        let result = self
            .try_generating_sign()
            .or_else(|| self.try_generating_operand())
            .or_else(|| self.try_genarting_comment())
            .or_else(|| self.try_generating_string())
            .or_else(|| self.try_generating_number())
            .or_else(|| self.try_creating_identifier_or_keyword());
        match result {
            Some(r) => Some(r),
            None => {
                let position = self.src.position();
                let code_snippet = self.src.error_code_snippet();
                panic!("Unexpected character at {:?}.\n{}", position, code_snippet);
            }
        }
    }

    fn skip_whitespaces(&mut self) {
        while self.src.current().is_whitespace() {
            let _ = self.src.next();
        }
    }

    fn try_genarting_comment(&mut self) -> Option<Token> {
        let current_char = self.src.current();
        if *current_char != '#' {
            return None;
        }

        let mut comment = String::new();
        let mut comment_length: u32 = 0;
        while let Ok(current) = self.src.next() {
            if comment_length > self.options.max_comment_length {
                panic!("Comment too long. Max comment length: {}", self.options.max_comment_length);
            }
            if *current == '\n' || *current == ETX { break; }
            comment.push(*current);
            comment_length += 1;
        }

        Some(Token { category: TokenCategory::Comment, value: TokenValue::String(comment), position: self.position })
    }

    fn try_generating_sign(&mut self) -> Option<Token> {
        let current_char = self.src.current();
        let token_category_result = SIGNS.get(current_char);
        match token_category_result {
            None => None,
            Some(token_category) => {
                let token = Token {
                    category: token_category.clone(),
                    value: TokenValue::Undefined,
                    position: self.position
                };
                let _ = self.src.next();
                Some(token)
            }
        }
    }

    fn try_generating_operand(&mut self) -> Option<Token> {
        let current_char = self.src.current();
        let token = match current_char {
            '+' => Some(Token {
                category: TokenCategory::Plus,
                value: TokenValue::Undefined,
                position: self.position
            }),
            '*' => Some(Token {
                category: TokenCategory::Multiply,
                value: TokenValue::Undefined,
                position: self.position
            }),
            '/' => Some(Token {
                category: TokenCategory::Divide,
                value: TokenValue::Undefined,
                position: self.position
            }),
            '-' => Some(self.extend_to_next('>', TokenCategory::Minus, TokenCategory::Arrow)),
            '<' => Some(self.extend_to_next('=', TokenCategory::Less, TokenCategory::LessOrEqual)),
            '>' => Some(self.extend_to_next(
                '=',
                TokenCategory::Greater,
                TokenCategory::GreaterOrEqual,
            )),
            '!' => Some(self.extend_to_next(
                '=',
                TokenCategory::Negate,
                TokenCategory::NotEqual,
            )),
            '=' => Some(self.extend_to_next('=', TokenCategory::Assign, TokenCategory::Equal)),
            '&' => Some(self.extend_to_next('&', TokenCategory::Reference, TokenCategory::And)),
            '|' => Some(self.extend_to_next_or_panic('|', TokenCategory::Or)),
            _ => None,
        };
        if token.is_some() {
            let _ = self.src.next();
        }
        token
    }

    fn extend_to_next(
        &mut self,
        char_to_search: char,
        not_found: TokenCategory,
        found: TokenCategory,
    ) -> Token {
        let next_char = self.src.next().unwrap();
        if *next_char == char_to_search {
            return Token {
                category: found,
                value: TokenValue::Undefined,
                position: self.position
            };
        }
        return Token {
            category: not_found,
            value: TokenValue::Undefined,
            position: self.position
        };
    }

    fn extend_to_next_or_panic(&mut self, char_to_search: char, found: TokenCategory) -> Token {
        let next_char = self.src.next().unwrap();
        if *next_char == char_to_search {
            return Token {
                category: found,
                value: TokenValue::Undefined,
                position: self.position
            };
        }
        panic!("Expected {} in {:?}", char_to_search, self.src.position());
    }

    fn try_generating_string(&mut self) -> Option<Token> {
        let mut current_char = self.src.current();
        if *current_char != '"' {
            return None;
        }
        let mut created_string = String::new();
        current_char = self.src.next().unwrap();
        while *current_char != '"' {
            if *current_char == '\n' {
                panic!("Unexpected newline in string in {:?}\n{}", self.src.position(), self.src.error_code_snippet());
            } 
            if *current_char == ETX {
                panic!("String not closed at {:?}", self.src.position());
            }
            created_string.push(*current_char);
            current_char = self.src.next().unwrap();
        }
        // consume next "
        let _ = self.src.next();
        Some(Token {
            category: TokenCategory::StringValue,
            value: TokenValue::String(created_string),
            position: self.position
        })
    }

    fn try_generating_number(&mut self) -> Option<Token> {
        let mut current_char = self.src.current();
        if !current_char.is_ascii_digit() {
            return None;
        }
        let (decimal, _) = self.parse_integer();
        current_char = self.src.current();
        if *current_char != '.' {
            return Some(Token {
                category: TokenCategory::I64,
                value: TokenValue::I64(decimal),
                position: self.position
            });
        }
        let _ = self.src.next();
        let (fraction, fraction_length) = self.parse_integer();
        let float_value = Self::merge_to_float(decimal, fraction, fraction_length);
        Some(Token {
            category: TokenCategory::F64,
            value: TokenValue::F64(float_value),
            position: self.position
        })
    }

    fn parse_integer(&mut self) -> (i64, i64) {
        let mut current_char = self.src.current();
        let mut length = 0;
        let mut total: i64 = 0;
        while current_char.is_ascii_digit() {
            let digit = *current_char as i64 - '0' as i64;
            match total.checked_mul(10) {
                Some(result) => total = result,
                None => panic!("Overflow occurred while parsing integer."),
            }
            total += digit;
            length += 1;
            current_char = self.src.next().unwrap();
        }
        (total, length)
    }

    fn merge_to_float(decimal: i64, fraction: i64, fraction_length: i64) -> f64 {
        let fraction_value = fraction as f64 / f64::powi(10.0, fraction_length as i32);
        let float_value = decimal as f64 + fraction_value;
        float_value
    }

    fn try_creating_identifier_or_keyword(&mut self) -> Option<Token> {
        let mut current_char = self.src.current();
        if !current_char.is_ascii_alphabetic() {
            return None;
        }
        let mut created_string = String::new();
        let mut string_length: u32 = 0;
        while current_char.is_ascii_digit() || current_char.is_ascii_alphabetic() || *current_char == '_' {
            if string_length > self.options.max_identifier_length {
                panic!("Identifier name too long. Max identifier length: {}", self.options.max_identifier_length);
            }
            created_string.push(*current_char);
            current_char = self.src.next().unwrap();
            string_length += 1;
        }
        match KEYWORDS.get(created_string.as_str()) {
            Some(category) => Some(Token {
                category: category.clone(),
                value: TokenValue::Undefined,
                position: self.position
            }),
            None => Some(Token {
                category: TokenCategory::Identifier,
                value: TokenValue::String(created_string),
                position: self.position
            }),
        }
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
    "while" => TokenCategory::While,
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
    "switch" => TokenCategory::Switch
};
