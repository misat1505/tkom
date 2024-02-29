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

        let result = self.try_generating_sign().or_else(|| self.try_generating_operand());
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
                    value: TokenValue::Char(*current_char),
                };
                let _ = self.src.next();
                Some(token)
            }
        }
    }

    fn try_generating_operand(&mut self) -> Option<Token> {
        let current_char = self.src.current();
        let token = match current_char {
            '+' => Some(Token { category: TokenCategory::Plus, value: TokenValue::Char('+') }),
            '-' => Some(Token { category: TokenCategory::Minus, value: TokenValue::Char('-') }),
            '*' => Some(Token { category: TokenCategory::Multiply, value: TokenValue::Char('*') }),
            '/' => Some(Token { category: TokenCategory::Divide, value: TokenValue::Char('/') }),
            _ => None
        };
        if token.is_some() {
            let _ = self.src.next();
        }
        token
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
