use crate::{
    ast::{Identifier, Node, Parameter, Statement, Type},
    errors::Issue,
    lazy_stream_reader::Position,
    value::Value,
};

#[derive(Debug)]
pub struct StdFunctionIssue {
    message: String,
}

impl Issue for StdFunctionIssue {
    fn message(&self) -> String {
        self.message.clone()
    }
}

#[derive(Debug, Clone)]
pub struct StdFunction {
    name: String,
    pub params: Vec<Type>,
    pub execute: fn(Vec<Value>) -> Result<(), StdFunctionIssue>,
}

impl StdFunction {
    pub fn print() -> Self {
        let name = "print".to_owned();
        let params = vec![Type::Str];
        let execute = |params: Vec<Value>| -> Result<(), StdFunctionIssue> {
            match params.get(0).unwrap() {
                Value::String(text) => {
                    println!("{}", text);
                    Ok(())
                }
                a => Err(StdFunctionIssue {
                    message: format!(
                        "Std function 'print' expected a string, but given a {:?}",
                        a
                    ),
                }),
            }
        };
        StdFunction {
            name,
            params,
            execute,
        }
    }
}
