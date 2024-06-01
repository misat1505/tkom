use std::fmt::Debug;

pub trait Issue: Debug {
    fn message(&self) -> String;
    fn set_message(&mut self, text: String);
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

    fn set_message(&mut self, text: String) {
        self.message = text;
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

    fn set_message(&mut self, text: String) {
        self.message = text;
    }
}

// semantic checker
#[derive(Debug)]
pub struct SemanticCheckerIssue {
    pub message: String,
}

impl Issue for SemanticCheckerIssue {
    fn message(&self) -> String {
        self.message.clone()
    }

    fn set_message(&mut self, text: String) {
        self.message = text;
    }
}

// interpreter
#[derive(Debug)]
pub struct InterpreterIssue {
    pub message: String,
}

impl Issue for InterpreterIssue {
    fn message(&self) -> String {
        self.message.clone()
    }

    fn set_message(&mut self, text: String) {
        self.message = text;
    }
}

#[derive(Debug)]
pub struct ComputationIssue {
    pub message: String,
}

impl Issue for ComputationIssue {
    fn message(&self) -> String {
        self.message.clone()
    }

    fn set_message(&mut self, text: String) {
        self.message = text;
    }
}

#[derive(Debug)]
pub struct ScopeManagerIssue {
    pub message: String,
}

impl Issue for ScopeManagerIssue {
    fn message(&self) -> String {
        self.message.clone()
    }

    fn set_message(&mut self, text: String) {
        self.message = text;
    }
}

#[derive(Debug)]
pub struct StackOverflowIssue {
    pub message: String,
}

impl Issue for StackOverflowIssue {
    fn message(&self) -> String {
        self.message.clone()
    }

    fn set_message(&mut self, text: String) {
        self.message = text;
    }
}

#[derive(Debug)]
pub struct StdFunctionIssue {
    pub message: String,
}

impl Issue for StdFunctionIssue {
    fn message(&self) -> String {
        self.message.clone()
    }

    fn set_message(&mut self, text: String) {
        self.message = text;
    }
}
