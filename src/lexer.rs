use std::collections::HashMap;

use regex::Regex;

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub enum TOKEN_TYPE {
    Number,
    Identifier,
    String,
    // Keywords
    Let,
    Const,
    Fn,
    If,
    Else,
    For,

    // Grouping * Operators
    Addition,
    Subtraction,
    Multiplication,
    Division,
    BinaryOperator,
    Equals,           // =
    Comma,            // ,
    Colon,            // :
    Semicolon,        // ;
    Dot,              // .
    OpenParen,        // (
    CloseParen,       // )
    OpenBrace,        // {
    CloseBrace,       // }
    OpenBracket,      // [
    CloseBracket,     // ]
    Quotation,        // "
    Greater,          // >
    Lesser,           // <
    EqualsCompare,    // ==
    NotEqualsCompare, // !=
    Exclamation,      // !
    And,              // &&
    Ampersand,        // &
    Bar,              // |
    EOF,              // Signified the end of file.
    Whitespace,
    Newline
}


fn create_hashmap() -> HashMap<TOKEN_TYPE, &'static str> {
    let mut hashmap: HashMap<TOKEN_TYPE, &'static str> = HashMap::new();

    hashmap.insert(TOKEN_TYPE::Number, r"^-?\d+(\.\d+)?");
    hashmap.insert(TOKEN_TYPE::Identifier, r"^[a-zA-Z_]\w*");
    hashmap.insert(TOKEN_TYPE::Addition, r"^\+");
    hashmap.insert(TOKEN_TYPE::Subtraction, r"^\-");
    hashmap.insert(TOKEN_TYPE::Multiplication, r"^\*");
    hashmap.insert(TOKEN_TYPE::Division, r"^/");
    hashmap.insert(TOKEN_TYPE::OpenParen, r"^\(");
    hashmap.insert(TOKEN_TYPE::CloseParen, r"^\)");
    hashmap.insert(TOKEN_TYPE::Whitespace, r"^\s+");
    hashmap.insert(TOKEN_TYPE::Equals, r"^\=");
    hashmap.insert(TOKEN_TYPE::String, r#""(?:[^"\\]|\\.)*""#);
    hashmap.insert(TOKEN_TYPE::Newline, r#"^(\n)"#);

    hashmap
}

pub fn lexer(input_code: &str) -> Vec<(TOKEN_TYPE, String)> {
    let mut tokens: Vec<(TOKEN_TYPE, String)> = Vec::new();
    let mut iterator = 0;
    let hashmap = create_hashmap();

    while iterator < input_code.len() {
        let mut match_found = false;

        for (token_type, pattern) in hashmap.iter() {
            let regex = Regex::new(&pattern).unwrap();
            if let Some(mat) = regex.find(&input_code[iterator..]) {
                let value = mat.as_str().to_string();
                tokens.push((*token_type, value));
                iterator += mat.end();
                match_found = true;
                break;
            }
        }

        if !match_found {
            panic!(
                "Invalid character at position {}: {}",
                iterator,
                &input_code[iterator..=iterator]
            );
        }
    }

    tokens
}
