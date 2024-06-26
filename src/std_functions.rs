use std::{
    cell::RefCell,
    collections::HashMap,
    io::{self, Write},
    rc::Rc,
};

use crate::{
    ast::Type,
    errors::{ErrorSeverity, StdFunctionError},
    value::Value,
};

#[derive(Debug, Clone, PartialEq)]
pub struct StdFunction {
    pub params: Vec<Type>,
    pub execute: fn(&Vec<Rc<RefCell<Value>>>) -> Result<Option<Value>, StdFunctionError>,
}

impl StdFunction {
    fn print() -> Self {
        let params = vec![Type::Str];
        let execute = |params: &Vec<Rc<RefCell<Value>>>| -> Result<Option<Value>, StdFunctionError> {
            if let Some(value) = params.get(0) {
                let value = value.borrow();
                match &*value {
                    Value::String(text) => {
                        println!("{}", text);
                        Ok(None)
                    }
                    _ => Err(StdFunctionError::new(
                        ErrorSeverity::HIGH,
                        format!(
                            "Std function 'print' expected '{:?}' as the only argument, but was given '{:?}'.",
                            Type::Str,
                            value.to_type()
                        ),
                    )),
                }
            } else {
                Err(StdFunctionError::new(
                    ErrorSeverity::HIGH,
                    String::from("Missing argument for 'print' function."),
                ))
            }
        };
        StdFunction { params, execute }
    }

    fn input() -> Self {
        let params = vec![Type::Str];
        let execute = |params: &Vec<Rc<RefCell<Value>>>| -> Result<Option<Value>, StdFunctionError> {
            if let Some(value) = params.get(0) {
                let value = value.borrow();
                match &*value {
                    Value::String(prompt) => {
                        print!("{}", prompt);
                        io::stdout().flush().unwrap();
                        let mut input = String::new();
                        match io::stdin().read_line(&mut input) {
                            Ok(_) => Ok(Some(Value::String(input.trim().to_string()))),
                            Err(_) => Err(StdFunctionError::new(ErrorSeverity::HIGH, String::from("Failed to read input."))),
                        }
                    }
                    _ => Err(StdFunctionError::new(
                        ErrorSeverity::HIGH,
                        format!(
                            "Std function 'input' expected '{:?}' as the only argument, but was given '{:?}'.",
                            Type::Str,
                            value.to_type()
                        ),
                    )),
                }
            } else {
                Err(StdFunctionError::new(
                    ErrorSeverity::HIGH,
                    String::from("Missing argument for 'input' function."),
                ))
            }
        };
        StdFunction { params, execute }
    }

    fn modulo() -> Self {
        let params = vec![Type::I64, Type::I64];
        let execute = |params: &Vec<Rc<RefCell<Value>>>| -> Result<Option<Value>, StdFunctionError> {
            if let (Some(val1), Some(val2)) = (params.get(0), params.get(1)) {
                let val1 = val1.borrow();
                let val2 = val2.borrow();
                match (&*val1, &*val2) {
                    (Value::I64(val1), Value::I64(val2)) => Ok(Some(Value::I64(*val1 % *val2))),
                    _ => Err(StdFunctionError::new(
                        ErrorSeverity::HIGH,
                        format!(
                            "Cannot perform modulo operation between values of types '{:?}' and '{:?}'.",
                            val1.to_type(),
                            val2.to_type()
                        ),
                    )),
                }
            } else {
                Err(StdFunctionError::new(
                    ErrorSeverity::HIGH,
                    String::from("Missing arguments for 'mod' function."),
                ))
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
