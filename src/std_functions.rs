use std::{
    cell::RefCell,
    collections::HashMap,
    io::{self, Write},
    rc::Rc,
};

use crate::{ast::Type, errors::StdFunctionIssue, value::Value};

#[derive(Debug, Clone, PartialEq)]
pub struct StdFunction {
    pub params: Vec<Type>,
    pub execute: fn(&Vec<Rc<RefCell<Value>>>) -> Result<Option<Value>, StdFunctionIssue>,
}

impl StdFunction {
    fn print() -> Self {
        let params = vec![Type::Str];
        let execute = |params: &Vec<Rc<RefCell<Value>>>| -> Result<Option<Value>, StdFunctionIssue> {
            if let Some(value) = params.get(0) {
                let value = value.borrow();
                match &*value {
                    Value::String(text) => {
                        println!("{}", text);
                        Ok(None)
                    }
                    _ => Err(StdFunctionIssue {
                        message: format!(
                            "Std function 'print' expected '{:?}' as the only argument, but was given '{:?}'.",
                            Type::Str,
                            value.to_type()
                        ),
                    }),
                }
            } else {
                Err(StdFunctionIssue {
                    message: "Missing argument for 'print' function.".to_owned(),
                })
            }
        };
        StdFunction { params, execute }
    }

    fn input() -> Self {
        let params = vec![Type::Str];
        let execute = |params: &Vec<Rc<RefCell<Value>>>| -> Result<Option<Value>, StdFunctionIssue> {
            if let Some(value) = params.get(0) {
                let value = value.borrow();
                match &*value {
                    Value::String(prompt) => {
                        print!("{}", prompt);
                        io::stdout().flush().unwrap();
                        let mut input = String::new();
                        match io::stdin().read_line(&mut input) {
                            Ok(_) => Ok(Some(Value::String(input.trim().to_string()))),
                            Err(_) => Err(StdFunctionIssue {
                                message: "Failed to read input.".to_owned(),
                            }),
                        }
                    }
                    _ => Err(StdFunctionIssue {
                        message: format!(
                            "Std function 'input' expected '{:?}' as the only argument, but was given '{:?}'.",
                            Type::Str,
                            value.to_type()
                        ),
                    }),
                }
            } else {
                Err(StdFunctionIssue {
                    message: "Missing argument for 'input' function.".to_owned(),
                })
            }
        };
        StdFunction { params, execute }
    }

    fn modulo() -> Self {
        let params = vec![Type::I64, Type::I64];
        let execute = |params: &Vec<Rc<RefCell<Value>>>| -> Result<Option<Value>, StdFunctionIssue> {
            if let (Some(val1), Some(val2)) = (params.get(0), params.get(1)) {
                let val1 = val1.borrow();
                let val2 = val2.borrow();
                match (&*val1, &*val2) {
                    (Value::I64(val1), Value::I64(val2)) => Ok(Some(Value::I64(*val1 % *val2))),
                    _ => Err(StdFunctionIssue {
                        message: format!(
                            "Cannot perform modulo operation between values of types '{:?}' and '{:?}'.",
                            val1.to_type(),
                            val2.to_type()
                        ),
                    }),
                }
            } else {
                Err(StdFunctionIssue {
                    message: "Missing arguments for 'mod' function.".to_owned(),
                })
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
