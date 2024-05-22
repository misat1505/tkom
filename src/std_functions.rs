use crate::{ast::Type, errors::Issue, value::Value};

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
    pub params: Vec<Type>,
    pub execute: fn(Vec<Value>) -> Result<(), StdFunctionIssue>,
}

impl StdFunction {
    pub fn print() -> Self {
        let params = vec![Type::Str];
        let execute = |params: Vec<Value>| -> Result<(), StdFunctionIssue> {
            match params.get(0).unwrap() {
                Value::String(text) => {
                    println!("{}", text);
                    Ok(())
                }
                a => Err(StdFunctionIssue {
                    message: format!(
                        "Std function 'print' expected '{:?}' as only argument, but was given '{:?}'.",
                        Type::Str, a.to_type()
                    ),
                }),
            }
        };
        StdFunction { params, execute }
    }
}
