use crate::lazy_stream_reader::Position;
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

macro_rules! define_issue {
    ($name:ident) => {
        #[derive(Debug, Clone)]
        pub struct $name {
            _message: String,
            _level: IssueLevel,
        }

        impl $name {
            pub fn new(level: IssueLevel, message: String) -> Self {
                $name {
                    _message: message,
                    _level: level,
                }
            }
        }

        impl Issue for $name {
            fn message(&self) -> String {
                self._message.clone()
            }

            fn set_message(&mut self, text: String) {
                self._message = text;
            }
        }
    };
}

define_issue!(LexerIssue);
define_issue!(ParserIssue);
define_issue!(SemanticCheckerIssue);
define_issue!(InterpreterIssue);
define_issue!(ComputationIssue);
define_issue!(ScopeManagerIssue);
define_issue!(StackOverflowIssue);
define_issue!(StdFunctionIssue);

pub struct IssuesManager;

impl IssuesManager {
    pub fn append_position(mut issue: Box<dyn Issue>, position: Position) -> Box<dyn Issue> {
        issue.set_message(format!("{}\nAt {:?}.", issue.message(), position));
        issue
    }
}
