#[cfg(test)]
mod tests {
    use std::io::BufReader;

    use crate::{
        issues::Issue,
        lazy_stream_reader::LazyStreamReader,
        lexer::{ILexer, Lexer, LexerOptions},
        tokens::{TokenCategory, TokenValue},
    };

    fn on_warning(warning: Box<dyn Issue>) {
        println!("{}", warning.message());
    }

    fn create_lexer(text: &str) -> Lexer<BufReader<&[u8]>> {
        let code = BufReader::new(text.as_bytes());
        let reader = LazyStreamReader::new(code);

        let lexer_options = LexerOptions {
            max_comment_length: 100,
            max_identifier_length: 20,
        };

        let lexer = Lexer::new(reader, lexer_options, on_warning);

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
    fn operators() {
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
        assert!(token.value == TokenValue::String(String::from(" this is a comment")));

        token = lexer.generate_token().unwrap();
        assert!(token.category == TokenCategory::Comment);
        assert!(token.value == TokenValue::String(String::from(" another")));
    }

    #[test]
    fn string() {
        let text = r#""string1"    " string2  ""string3""#;
        let mut lexer = create_lexer_with_skip(text);

        let mut token = lexer.generate_token().unwrap();
        assert!(token.category == TokenCategory::StringValue);
        assert!(token.value == TokenValue::String(String::from("string1")));

        token = lexer.generate_token().unwrap();
        assert!(token.category == TokenCategory::StringValue);
        assert!(token.value == TokenValue::String(String::from(" string2  ")));

        token = lexer.generate_token().unwrap();
        assert!(token.category == TokenCategory::StringValue);
        assert!(token.value == TokenValue::String(String::from("string3")));
    }

    #[test]
    fn escapes() {
        let text = r#""ala\"ma\nkota\tjana\\i\szympansa""#;
        let mut lexer = create_lexer_with_skip(text);

        let expected = "ala\"ma\nkota\tjana\\i\\szympansa";

        let token = lexer.generate_token().unwrap();
        assert!(token.category == TokenCategory::StringValue);
        assert!(token.value == TokenValue::String(expected.to_string()));
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
        let text = "fn for if else return i64 f64
        str void bool true false as switch break my_identifier1";
        let mut lexer = create_lexer_with_skip(text);

        let expected: Vec<(TokenCategory, TokenValue)> = vec![
            (TokenCategory::Fn, TokenValue::Null),
            (TokenCategory::For, TokenValue::Null),
            (TokenCategory::If, TokenValue::Null),
            (TokenCategory::Else, TokenValue::Null),
            (TokenCategory::Return, TokenValue::Null),
            (TokenCategory::I64, TokenValue::Null),
            (TokenCategory::F64, TokenValue::Null),
            (TokenCategory::String, TokenValue::Null),
            (TokenCategory::Void, TokenValue::Null),
            (TokenCategory::Bool, TokenValue::Null),
            (TokenCategory::True, TokenValue::Null),
            (TokenCategory::False, TokenValue::Null),
            (TokenCategory::As, TokenValue::Null),
            (TokenCategory::Switch, TokenValue::Null),
            (TokenCategory::Break, TokenValue::Null),
            (TokenCategory::Identifier, TokenValue::String("my_identifier1".to_owned())),
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

    use crate::{
        issues::Issue,
        lazy_stream_reader::LazyStreamReader,
        lexer::{Lexer, LexerOptions},
        tokens::TokenCategory,
    };

    fn on_warning(warning: Box<dyn Issue>) {
        println!("{}", warning.message());
    }

    fn create_lexer(text: &str) -> Lexer<BufReader<&[u8]>> {
        let code = BufReader::new(text.as_bytes());
        let reader = LazyStreamReader::new(code);

        let lexer_options = LexerOptions {
            max_comment_length: 100,
            max_identifier_length: 20,
        };

        let lexer = Lexer::new(reader, lexer_options, on_warning);

        lexer
    }

    fn create_lexer_with_skip(text: &str) -> Lexer<BufReader<&[u8]>> {
        let mut lexer = create_lexer(text);
        let _ = lexer.generate_token().unwrap();

        lexer
    }

    #[test]
    fn too_long_comment() {
        let chars = "a".repeat(150);
        let text = format!("# {}", chars);
        let mut lexer = create_lexer_with_skip(text.as_str());

        let result = lexer.generate_token();
        assert!(result.is_err());
    }

    #[test]
    fn too_long_identifier() {
        let text = "a".repeat(30);
        let mut lexer = create_lexer_with_skip(text.as_str());

        let result = lexer.generate_token();
        assert!(result.is_err());
    }

    #[test]
    fn extend_to_next_or_warning() {
        let text = "|";
        let mut lexer = create_lexer_with_skip(text);

        let result = lexer.generate_token();
        assert!(result.unwrap().category == TokenCategory::Or);
    }

    #[test]
    fn newline_in_string() {
        let text = r#""my
        string""#;
        let mut lexer = create_lexer_with_skip(text);

        let result = lexer.generate_token();
        assert!(result.is_err());
    }

    #[test]
    fn string_unclosed() {
        let text = r#""my_string"#;
        let mut lexer = create_lexer_with_skip(text);

        let result = lexer.generate_token();
        assert!(result.unwrap().category == TokenCategory::StringValue);
    }

    #[test]
    fn int_overflow() {
        // 1 more than limit
        let text = "9223372036854775808";
        let mut lexer = create_lexer_with_skip(text);

        let result = lexer.generate_token();
        assert!(result.is_err());
    }

    #[test]
    fn disallow_zero_prefix() {
        let text = "007";
        let mut lexer = create_lexer_with_skip(text);

        let result = lexer.generate_token();
        assert!(result.is_err());
    }
}
