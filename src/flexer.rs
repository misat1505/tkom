use std::io::BufRead;

use phf::phf_map;

use crate::lazy_stream_reader::{ILazyStreamReader, LazyStreamReader};
use crate::tokens::{Token, TokenCategory, TokenValue};

pub trait ILexer<T: BufRead> {
    fn new(src: LazyStreamReader<T>) -> Self;
    fn next(&mut self) -> &Option<Token>;
    fn current(&self) -> &Option<Token>;
}

pub struct Lexer<T: BufRead> {
    pub src: LazyStreamReader<T>,
    current: Option<Token>,
}

impl<T: BufRead> ILexer<T> for Lexer<T> {
    fn new(src: LazyStreamReader<T>) -> Self {
        Lexer { src, current: None }
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

        let result = self
            .try_generating_sign()
            .or_else(|| self.try_generating_operand())
            .or_else(|| self.try_generating_string())
            .or_else(|| self.try_generating_number())
            .or_else(|| self.try_creating_identifier_or_keyword());
        match result {
            Some(r) => Some(r),
            None => {
                let position = self.src.position();
                panic!("Bad sign at {:?}", position);
            }
        }
    }

    fn skip_whitespaces(&mut self) {
        while self.src.current().is_whitespace() {
            let _ = self.src.next();
        }
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
            }),
            '-' => Some(Token {
                category: TokenCategory::Minus,
                value: TokenValue::Undefined,
            }),
            '*' => Some(Token {
                category: TokenCategory::Multiply,
                value: TokenValue::Undefined,
            }),
            '/' => Some(Token {
                category: TokenCategory::Divide,
                value: TokenValue::Undefined,
            }),
            '<' => Some(self.extend_to_next('=', TokenCategory::Less, TokenCategory::LessOrEqual)),
            '>' => Some(self.extend_to_next(
                '=',
                TokenCategory::Greater,
                TokenCategory::GreaterOrEqual,
            )),
            '=' => Some(self.extend_to_next('=', TokenCategory::Assign, TokenCategory::Equal)),
            '&' => Some(self.extend_to_next_or_panic('&', TokenCategory::And)),
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
            };
        }
        return Token {
            category: not_found,
            value: TokenValue::Undefined,
        };
    }

    fn extend_to_next_or_panic(&mut self, char_to_search: char, found: TokenCategory) -> Token {
        let next_char = self.src.next().unwrap();
        if *next_char == char_to_search {
            return Token {
                category: found,
                value: TokenValue::Undefined,
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
                panic!("Unexpected newline in string in {:?}", self.src.position());
            }
            created_string.push(*current_char);
            current_char = self.src.next().unwrap();
        }
        // consume next "
        let _ = self.src.next();
        Some(Token {
            category: TokenCategory::StringValue,
            value: TokenValue::String(created_string),
        })
    }

    fn try_generating_number(&mut self) -> Option<Token> {
        let mut current_char = self.src.current();
        if !current_char.is_ascii_digit() {
            return None;
        }
        let mut decimal = self.parse_integer();
        current_char = self.src.current();
        if *current_char != '.' {
            return Some(Token {
                category: TokenCategory::I64,
                value: TokenValue::I64(decimal),
            });
        }
        let _ = self.src.next();
        let fraction = self.parse_integer();
        let float_value = Self::merge_to_float(decimal, fraction);
        Some(Token {
            category: TokenCategory::F64,
            value: TokenValue::F64(float_value),
        })
    }

    fn parse_integer(&mut self) -> i64 {
        let mut current_char = self.src.current();
        let mut stringified_number = String::new();
        while current_char.is_ascii_digit() {
            stringified_number.push(*current_char);
            current_char = self.src.next().unwrap();
        }
        let number = stringified_number.parse::<i64>();
        match number {
            Ok(num) => num,
            Err(err) => panic!("Bad conversion in {:?}", self.src.position()),
        }
    }

    fn merge_to_float(decimal: i64, fraction: i64) -> f64 {
        let fraction_digits = (fraction as f64).log10().ceil() as i32;
        let fraction_value = fraction as f64 / f64::powi(10.0, fraction_digits);
        let float_value = decimal as f64 + fraction_value;
        float_value
    }

    fn try_creating_identifier_or_keyword(&mut self) -> Option<Token> {
        let mut current_char = self.src.current();
        let mut created_string = String::new();
        while current_char.is_ascii_digit() || current_char.is_ascii_alphabetic() {
            created_string.push(*current_char);
            current_char = self.src.next().unwrap();
        }
        match KEYWORDS.get(created_string.as_str()) {
            Some(category) => Some(Token {
                category: category.clone(),
                value: TokenValue::Undefined,
            }),
            None => Some(Token {
                category: TokenCategory::Identifier,
                value: TokenValue::String(created_string),
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
    "bool" => TokenCategory::Bool,
    "true" => TokenCategory::True,
    "false" => TokenCategory::False
};
