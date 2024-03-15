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
    Power,
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
    Use,
    As,
    In,
    Unit,
    Fn,
    True,
    False,
    Return,
    Switch,
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
    Comment(String),
    // Literals
    StringValue,
    I64Value(i64),
    F64Value(f64),
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
}
