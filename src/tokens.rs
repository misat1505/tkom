use std::fmt::Debug;

use crate::lazy_stream_reader::Position;

#[derive(PartialEq, Clone)]
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

impl Debug for TokenCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use TokenCategory::*;

        let text = match self {
            Greater => ">",
            GreaterOrEqual => ">=",
            Less => "<",
            LessOrEqual => "<=",
            Equal => "==",
            NotEqual => "!=",
            Plus => "+",
            Minus => "-",
            Multiply => "*",
            Divide => "/",
            Negate => "!",
            And => "&&",
            Or => "||",
            ParenOpen => "(",
            ParenClose => ")",
            BracketOpen => "[",
            BracketClose => "]",
            BraceOpen => "{",
            BraceClose => "}",
            For => "for",
            If => "if",
            Else => "else",
            As => "as",
            Fn => "fn",
            True => "true",
            False => "false",
            Return => "return",
            Switch => "switch",
            Break => "break",
            Bool => "bool type",
            String => "str type",
            I64 => "i64 type",
            F64 => "f64 type",
            Void => "void",
            Assign => "=",
            Colon => ":",
            Semicolon => ";",
            Comma => ",",
            Reference => "&",
            Arrow => "->",
            STX => "STX",
            ETX => "ETX",
            Identifier => "identifier",
            Comment => "comment",
            StringValue => "str value",
            I64Value => "i64 value",
            F64Value => "f64 value",
        };

        write!(f, "{}", text)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TokenValue {
    String(String),
    F64(f64),
    I64(i64),
    Null,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub category: TokenCategory,
    pub value: TokenValue,
    pub position: Position,
}
