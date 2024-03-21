use crate::lazy_stream_reader::Position;

#[derive(Debug, PartialEq, Clone)]
pub enum TokenCategory {
    // Comparison
    Greater,
    GreaterOrEqual,
    Less,
    LessOrEqual,
    Equal,
    NotEqual,
    // Arithmetic
    Plus,
    Minus,
    Multiply,
    Divide,
    // Boolean arithmetic
    Negate,
    And,
    Or,
    // Parentheses
    ParenOpen,
    ParenClose,
    BracketOpen,
    BracketClose,
    BraceOpen,
    BraceClose,
    // Keywords
    For,
    While,
    If,
    Else,
    As,
    Fn,
    True,
    False,
    Return,
    Switch,
    Break,
    // Type keywords
    Bool,
    String,
    I64,
    F64,
    Void,
    // Others
    Assign,
    Colon,
    Semicolon,
    Comma,
    Reference,
    Arrow,
    STX,
    ETX,
    // Complex
    Identifier,
    Comment,
    // Literals
    StringValue,
    I64Value,
    F64Value,
}

#[derive(Debug, Clone)]
pub enum TokenValue {
    String(String),
    Bool(bool),
    F64(f64),
    I64(i64),
    Undefined,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub category: TokenCategory,
    pub value: TokenValue,
    pub position: Position
}
