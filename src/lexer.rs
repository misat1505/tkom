use std::io::BufRead;

use phf::phf_map;

use crate::lazy_stream_reader::{ILazyStreamReader, LazyStreamReader, Position, ETX};
use crate::tokens::{Token, TokenCategory, TokenValue};

pub trait ILexer<T: BufRead> {
    fn new(src: LazyStreamReader<T>, options: LexerOptions) -> Self;
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
    options: LexerOptions,
}

impl<T: BufRead> ILexer<T> for Lexer<T> {
    fn new(src: LazyStreamReader<T>, options: LexerOptions) -> Self {
        let position = src.position().clone();
        Lexer {
            src,
            current: None,
            position,
            options,
        }
    }

    fn current(&self) -> &Option<Token> {
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
            .or_else(|| self.try_generating_comment())
            .or_else(|| self.try_generating_string())
            .or_else(|| self.try_generating_number())
            .or_else(|| self.try_creating_identifier_or_keyword());
        match result {
            Some(r) => Some(r),
            None => {
                let position = self.src.position();
                let code_snippet = self.src.error_code_snippet();
                panic!(
                    "\nUnexpected character at {:?}.\n{}",
                    position, code_snippet
                );
            }
        }
    }

    fn skip_whitespaces(&mut self) {
        while self.src.current().is_whitespace() {
            let _ = self.src.next();
        }
    }

    fn try_generating_comment(&mut self) -> Option<Token> {
        let current_char = self.src.current().clone();
        if current_char != '#' {
            return None;
        }

        let mut comment = String::new();
        let mut comment_length: u32 = 0;
        while let Ok(current) = self.src.next().cloned() {
            if comment_length > self.options.max_comment_length {
                self.create_panic(format!(
                    "Comment too long. Max comment length: {}",
                    self.options.max_comment_length
                ));
            }
            if current == '\n' || current == ETX {
                break;
            }
            comment.push(current);
            comment_length += 1;
        }

        Some(Token {
            category: TokenCategory::Comment,
            value: TokenValue::String(comment),
            position: self.position,
        })
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
                    position: self.position,
                };
                let _ = self.src.next();
                Some(token)
            }
        }
    }

    fn try_generating_operand(&mut self) -> Option<Token> {
        let current_char = self.src.current();
        let token = match current_char {
            '+' => Some(self.single_char(TokenCategory::Plus)),
            '*' => Some(self.single_char(TokenCategory::Multiply)),
            '/' => Some(self.single_char(TokenCategory::Divide)),
            '-' => Some(self.extend_to_next('>', TokenCategory::Minus, TokenCategory::Arrow)),
            '<' => Some(self.extend_to_next('=', TokenCategory::Less, TokenCategory::LessOrEqual)),
            '>' => Some(self.extend_to_next(
                '=',
                TokenCategory::Greater,
                TokenCategory::GreaterOrEqual,
            )),
            '!' => Some(self.extend_to_next('=', TokenCategory::Negate, TokenCategory::NotEqual)),
            '=' => Some(self.extend_to_next('=', TokenCategory::Assign, TokenCategory::Equal)),
            '&' => Some(self.extend_to_next('&', TokenCategory::Reference, TokenCategory::And)),
            '|' => Some(self.extend_to_next_or_panic('|', TokenCategory::Or)),
            _ => None,
        };
        token
    }

    fn single_char(&mut self, category: TokenCategory) -> Token {
        let _ = self.src.next();
        Token {
            category,
            value: TokenValue::Undefined,
            position: self.position,
        }
    }

    fn extend_to_next(
        &mut self,
        char_to_search: char,
        not_found: TokenCategory,
        found: TokenCategory,
    ) -> Token {
        let next_char = self.src.next().unwrap();
        if *next_char == char_to_search {
            let _ = self.src.next();
            return Token {
                category: found,
                value: TokenValue::Undefined,
                position: self.position,
            };
        }
        return Token {
            category: not_found,
            value: TokenValue::Undefined,
            position: self.position,
        };
    }

    fn extend_to_next_or_panic(&mut self, char_to_search: char, found: TokenCategory) -> Token {
        let next_char = self.src.next().unwrap().clone();
        if next_char == char_to_search {
            let _ = self.src.next();
            return Token {
                category: found,
                value: TokenValue::Undefined,
                position: self.position,
            };
        }
        // self.create_panic(format!("Expected {}", char_to_search));
        let code_snippet = self.src.error_code_snippet();
        panic!(
            "Expected {}\nAt: {:?}\n{}\n",
            char_to_search,
            self.src.position(),
            code_snippet
        );
    }

    fn try_generating_string(&mut self) -> Option<Token> {
        let mut current_char = self.src.current().clone();
        if current_char != '"' {
            return None;
        }
        let mut created_string = String::new();
        current_char = self.src.next().unwrap().clone();
        while current_char != '"' {
            if current_char == '\n' {
                self.create_panic("Unexpected newline in string".to_owned());
            }
            if current_char == ETX {
                self.create_panic("String not closed".to_owned());
            }
            created_string.push(current_char);
            current_char = self.src.next().unwrap().clone();
        }
        // consume next "
        let _ = self.src.next();
        Some(Token {
            category: TokenCategory::StringValue,
            value: TokenValue::String(created_string),
            position: self.position,
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
                category: TokenCategory::I64Value,
                value: TokenValue::I64(decimal),
                position: self.position,
            });
        }
        let _ = self.src.next();
        let (fraction, fraction_length) = self.parse_integer();
        let float_value = Self::merge_to_float(decimal, fraction, fraction_length);
        Some(Token {
            category: TokenCategory::F64Value,
            value: TokenValue::F64(float_value),
            position: self.position,
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
                None => self.create_panic("Overflow occurred while parsing integer".to_owned()),
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
        let mut current_char = self.src.current().clone();
        if !current_char.is_ascii_alphabetic() {
            return None;
        }
        let mut created_string = String::new();
        let mut string_length: u32 = 0;
        while current_char.is_ascii_digit()
            || current_char.is_ascii_alphabetic()
            || current_char == '_'
        {
            if string_length > self.options.max_identifier_length {
                self.create_panic(format!(
                    "Identifier name too long. Max identifier length: {}",
                    self.options.max_identifier_length
                ));
            }
            created_string.push(current_char);
            current_char = self.src.next().unwrap().clone();
            string_length += 1;
        }
        match KEYWORDS.get(created_string.as_str()) {
            Some(category) => Some(Token {
                category: category.clone(),
                value: TokenValue::Undefined,
                position: self.position,
            }),
            None => Some(Token {
                category: TokenCategory::Identifier,
                value: TokenValue::String(created_string),
                position: self.position,
            }),
        }
    }

    fn create_panic(&mut self, text: String) {
        let position = self.src.position();
        let code_snippet = self.src.error_code_snippet();
        panic!("\n{} at {:?}\n{}", text, position, code_snippet);
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
    "switch" => TokenCategory::Switch,
    "break" => TokenCategory::Break
};

#[cfg(test)]
mod tests {
    use std::io::BufReader;

    use super::*;

    fn create_lexer(text: &str) -> Lexer<BufReader<&[u8]>> {
        let code = BufReader::new(text.as_bytes());
        let reader = LazyStreamReader::new(code);

        let lexer_options = LexerOptions {
            max_comment_length: 100,
            max_identifier_length: 20,
        };

        let lexer = Lexer::new(reader, lexer_options);

        lexer
    }

    fn create_lexer_with_skip(text: &str) -> Lexer<BufReader<&[u8]>> {
        let mut lexer = create_lexer(text);
        let _ = lexer.generate_token().unwrap();

        lexer
    }

    #[test]
    fn constructor() {
        let text = "123";
        let mut lexer = create_lexer(text);
        assert!(lexer.current().is_none());
        let token = lexer.generate_token().unwrap();
        assert!(token.category == TokenCategory::STX);
    }

    #[test]
    fn last_token() {
        let mut lexer = create_lexer_with_skip("");
        let mut token = lexer.generate_token().unwrap();
        assert!(token.category == TokenCategory::ETX);
        token = lexer.generate_token().unwrap();
        assert!(token.category == TokenCategory::ETX);
    }

    #[test]
    fn signs() {
        let text = "( ) [  ] {} ;   :, ";
        let mut lexer = create_lexer_with_skip(text);
        let expected_tokens: Vec<TokenCategory> = vec![
            TokenCategory::ParenOpen,
            TokenCategory::ParenClose,
            TokenCategory::BracketOpen,
            TokenCategory::BracketClose,
            TokenCategory::BraceOpen,
            TokenCategory::BraceClose,
            TokenCategory::Semicolon,
            TokenCategory::Colon,
            TokenCategory::Comma,
        ];

        for expected_token in &expected_tokens {
            let token = lexer.generate_token().unwrap();
            assert!(token.category == *expected_token);
        }
    }

    #[test]
    fn operands() {
        let text = "+* / --> <<= > >= ! != = == & && || ";
        let mut lexer = create_lexer_with_skip(text);
        let expected_tokens: Vec<TokenCategory> = vec![
            TokenCategory::Plus,
            TokenCategory::Multiply,
            TokenCategory::Divide,
            TokenCategory::Minus,
            TokenCategory::Arrow,
            TokenCategory::Less,
            TokenCategory::LessOrEqual,
            TokenCategory::Greater,
            TokenCategory::GreaterOrEqual,
            TokenCategory::Negate,
            TokenCategory::NotEqual,
            TokenCategory::Assign,
            TokenCategory::Equal,
            TokenCategory::Reference,
            TokenCategory::And,
            TokenCategory::Or,
        ];

        for expected_token in &expected_tokens {
            let token = lexer.generate_token().unwrap();
            assert!(token.category == *expected_token);
        }
    }

    #[test]
    fn comment() {
        let text = "# this is a comment
        # another";
        let mut lexer = create_lexer_with_skip(text);

        let mut token = lexer.generate_token().unwrap();
        assert!(token.category == TokenCategory::Comment);
        assert!(token.value == TokenValue::String(" this is a comment".to_string()));

        token = lexer.generate_token().unwrap();
        assert!(token.category == TokenCategory::Comment);
        assert!(token.value == TokenValue::String(" another".to_string()));
    }

    #[test]
    fn string() {
        let text = r#""string1"    " string2  ""string3""#;
        let mut lexer = create_lexer_with_skip(text);

        let mut token = lexer.generate_token().unwrap();
        assert!(token.category == TokenCategory::StringValue);
        assert!(token.value == TokenValue::String("string1".to_owned()));

        token = lexer.generate_token().unwrap();
        assert!(token.category == TokenCategory::StringValue);
        assert!(token.value == TokenValue::String(" string2  ".to_owned()));

        token = lexer.generate_token().unwrap();
        assert!(token.category == TokenCategory::StringValue);
        assert!(token.value == TokenValue::String("string3".to_owned()));
    }

    #[test]
    fn numbers() {
        let text = "123 0 5 12.3 2.0 0.0";
        let mut lexer = create_lexer_with_skip(text);

        let expected: Vec<(TokenCategory, TokenValue)> = vec![
            (TokenCategory::I64Value, TokenValue::I64(123)),
            (TokenCategory::I64Value, TokenValue::I64(0)),
            (TokenCategory::I64Value, TokenValue::I64(5)),
            (TokenCategory::F64Value, TokenValue::F64(12.3)),
            (TokenCategory::F64Value, TokenValue::F64(2.0)),
            (TokenCategory::F64Value, TokenValue::F64(0.0)),
        ];

        for (category, value) in &expected {
            let token = lexer.generate_token().unwrap();
            assert!(token.category == *category);
            assert!(token.value == *value);
        }
    }

    #[test]
    fn keyword_or_identifier() {
        let text = "fn for while if else return i64 f64
        str void bool true false as switch break my_identifier1";
        let mut lexer = create_lexer_with_skip(text);

        let expected: Vec<(TokenCategory, TokenValue)> = vec![
            (TokenCategory::Fn, TokenValue::Undefined),
            (TokenCategory::For, TokenValue::Undefined),
            (TokenCategory::While, TokenValue::Undefined),
            (TokenCategory::If, TokenValue::Undefined),
            (TokenCategory::Else, TokenValue::Undefined),
            (TokenCategory::Return, TokenValue::Undefined),
            (TokenCategory::I64, TokenValue::Undefined),
            (TokenCategory::F64, TokenValue::Undefined),
            (TokenCategory::String, TokenValue::Undefined),
            (TokenCategory::Void, TokenValue::Undefined),
            (TokenCategory::Bool, TokenValue::Undefined),
            (TokenCategory::True, TokenValue::Undefined),
            (TokenCategory::False, TokenValue::Undefined),
            (TokenCategory::As, TokenValue::Undefined),
            (TokenCategory::Switch, TokenValue::Undefined),
            (TokenCategory::Break, TokenValue::Undefined),
            (
                TokenCategory::Identifier,
                TokenValue::String("my_identifier1".to_owned()),
            ),
        ];

        for (category, value) in &expected {
            let token = lexer.generate_token().unwrap();
            assert!(token.category == *category);
            assert!(token.value == *value);
        }
    }
}

#[cfg(test)]
mod edge_case_tests {
    use std::io::BufReader;

    use super::*;

    fn create_lexer(text: &str) -> Lexer<BufReader<&[u8]>> {
        let code = BufReader::new(text.as_bytes());
        let reader = LazyStreamReader::new(code);

        let lexer_options = LexerOptions {
            max_comment_length: 100,
            max_identifier_length: 20,
        };

        let lexer = Lexer::new(reader, lexer_options);

        lexer
    }

    fn create_lexer_with_skip(text: &str) -> Lexer<BufReader<&[u8]>> {
        let mut lexer = create_lexer(text);
        let _ = lexer.generate_token().unwrap();

        lexer
    }

    #[test]
    #[should_panic]
    fn too_long_comment() {
        let chars = "a".repeat(150);
        let text = format!("# {}", chars);
        let mut lexer = create_lexer_with_skip(text.as_str());

        lexer.generate_token();
    }

    #[test]
    #[should_panic]
    fn too_long_identifier() {
        let text = "a".repeat(30);
        let mut lexer = create_lexer_with_skip(text.as_str());

        lexer.generate_token();
    }

    #[test]
    #[should_panic]
    fn extend_to_next_or_panic() {
        let text = "|";
        let mut lexer = create_lexer_with_skip(text);

        lexer.generate_token();
    }

    #[test]
    #[should_panic]
    fn newline_in_string() {
        let text = r#""my
        string""#;
        let mut lexer = create_lexer_with_skip(text);

        lexer.generate_token();
    }

    #[test]
    #[should_panic]
    fn string_unclosed() {
        let text = r#""my_string"#;
        let mut lexer = create_lexer_with_skip(text);

        lexer.generate_token();
    }

    #[test]
    #[should_panic]
    fn int_overflow() {
        // 1 more than limit
        let text = "9223372036854775808";
        let mut lexer = create_lexer_with_skip(text);

        lexer.generate_token();
    }
}
