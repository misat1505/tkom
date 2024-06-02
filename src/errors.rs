use crate::lazy_stream_reader::Position;
use std::fmt::Debug;

pub trait IError: Debug {
    fn message(&self) -> String;
    fn set_message(&mut self, text: String);
}

#[derive(Debug, Clone)]
pub enum ErrorLevel {
    WARNING,
    ERROR,
}

macro_rules! define_error {
    ($name:ident) => {
        #[derive(Debug, Clone)]
        pub struct $name {
            _message: String,
            _level: ErrorLevel,
        }

        impl $name {
            pub fn new(level: ErrorLevel, message: String) -> Self {
                $name {
                    _message: message,
                    _level: level,
                }
            }
        }

        impl IError for $name {
            fn message(&self) -> String {
                self._message.clone()
            }

            fn set_message(&mut self, text: String) {
                self._message = text;
            }
        }
    };
}

define_error!(LexerError);
define_error!(ParserError);
define_error!(SemanticCheckerError);
define_error!(InterpreterError);
define_error!(ComputationError);
define_error!(ScopeManagerError);
define_error!(StackOverflowError);
define_error!(StdFunctionError);

pub struct ErrorsManager;

impl ErrorsManager {
    pub fn append_position(mut error: Box<dyn IError>, position: Position) -> Box<dyn IError> {
        error.set_message(format!("{}\nAt {:?}.", error.message(), position));
        error
    }
}
