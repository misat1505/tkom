use std::fmt::Debug;

pub trait Issue: Debug {
    fn message(&self) -> String;
}

#[derive(Debug, Clone)]
pub enum IssueLevel {
    WARNING,
    ERROR,
}

// lexer

#[derive(Debug, Clone)]
pub struct LexerIssue {
    pub level: IssueLevel,
    pub message: String,
}

impl LexerIssue {
    pub fn new(level: IssueLevel, message: String) -> Self {
        LexerIssue { level, message }
    }
}

impl Issue for LexerIssue {
    fn message(&self) -> String {
        self.message.clone()
    }
}

// parser

#[derive(Debug, Clone)]
pub struct ParserIssue {
    pub level: IssueLevel,
    pub message: String,
}

impl Issue for ParserIssue {
    fn message(&self) -> String {
        self.message.clone()
    }
}
