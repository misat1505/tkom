use std::{
    collections::HashMap,
    io::{self, Write},
};

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

#[derive(Debug, Clone, PartialEq)]
pub struct StdFunction {
    pub params: Vec<Type>,
    pub execute: fn(Vec<Value>) -> Result<Option<Value>, StdFunctionIssue>,
}

impl StdFunction {
    fn print() -> Self {
        let params = vec![Type::Str];
        let execute = |params: Vec<Value>| -> Result<Option<Value>, StdFunctionIssue> {
            match params.get(0).unwrap() {
                Value::String(text) => {
                    println!("{}", text);
                    Ok(None)
                }
                a => Err(StdFunctionIssue {
                    message: format!(
                        "Std function 'print' expected '{:?}' as only argument, but was given '{:?}'.",
                        Type::Str,
                        a.to_type()
                    ),
                }),
            }
        };
        StdFunction { params, execute }
    }

    fn input() -> Self {
        let params = vec![Type::Str];
        let execute = |params: Vec<Value>| -> Result<Option<Value>, StdFunctionIssue> {
            match params.get(0).unwrap() {
                Value::String(text) => {
                    print!("{}", text);
                    io::stdout().flush().unwrap(); // Flush stdout to ensure prompt is displayed
                    let mut input = String::new();
                    match io::stdin().read_line(&mut input) {
                        Ok(_) => Ok(Some(Value::String(input.trim().to_string()))),
                        Err(_) => Err(StdFunctionIssue {
                            message: "Failed to read input".to_string(),
                        }),
                    }
                }
                a => Err(StdFunctionIssue {
                    message: format!(
                        "Std function 'input' expected '{:?}' as only argument, but was given '{:?}'.",
                        Type::Str,
                        a.to_type()
                    ),
                }),
            }
        };
        StdFunction { params, execute }
    }

    fn modulo() -> Self {
        let params = vec![Type::I64, Type::I64];
        let execute = |params: Vec<Value>| -> Result<Option<Value>, StdFunctionIssue> {
            match (params.get(0), params.get(1)) {
                (Some(Value::I64(val1)), Some(Value::I64(val2))) => Ok(Some(Value::I64(*val1 % *val2))),
                (a, b) => Err(StdFunctionIssue {
                    message: format!(
                        "Cannot perform modulo operation between values of types '{:?}' and '{:?}'.",
                        a.unwrap().to_type(),
                        b.unwrap().to_type()
                    ),
                }),
            }
        };
        StdFunction { params, execute }
    }
}

pub fn get_std_functions() -> HashMap<String, StdFunction> {
    let mut std_functions: HashMap<String, StdFunction> = HashMap::new();
    std_functions.insert("print".to_owned(), StdFunction::print());
    std_functions.insert("input".to_owned(), StdFunction::input());
    std_functions.insert("mod".to_owned(), StdFunction::modulo());
    std_functions
}
