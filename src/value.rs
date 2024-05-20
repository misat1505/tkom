use crate::{ast::Type, errors::Issue};

#[derive(Debug)]
pub struct ComputationIssue {
  message: String
}

impl Issue for ComputationIssue {
  fn message(&self) -> String {
      self.message.clone()
  }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    I64(i64),
    F64(f64),
    String(String),
    Bool(bool),
}

impl Value {
  pub fn default_value(var_type: Type) -> Result<Value, ComputationIssue> {
    match var_type {
      Type::Bool => Ok(Value::Bool(false)),
      Type::I64 => Ok(Value::I64(0)),
      Type::F64 => Ok(Value::F64(0.0)),
      Type::Str => Ok(Value::String("".to_owned())),
      a => Err(ComputationIssue { message: format!("Cannot create default value for type: {:?}.", a) }),
    }
  }

  pub fn add(&self, other: Value) -> Result<Value, ComputationIssue> {
    match (self, other) {
        (Value::I64(a), Value::I64(b)) => {
          match a.checked_add(b) {
            Some(_) => Ok(Value::I64(a + b)),
            None => Err(ComputationIssue {message: format!("Overflow occured when performing addition on i64s.")})
          }
        },
        (Value::F64(a), Value::F64(b)) => Ok(Value::F64(a + b)),
        (Value::String(a), Value::String(b)) => Ok(Value::String(a.clone() + &b.clone())),
        (a, b) => Err(ComputationIssue { message: format!("Cannot perform addition between {:?} and {:?}.", a, b) })
    }
  }

  pub fn subtract(&self, other: Value) -> Result<Value, ComputationIssue> {
    match (self, other) {
        (Value::I64(a), Value::I64(b)) => {
          match a.checked_sub(b) {
            Some(_) => Ok(Value::I64(a - b)),
            None => Err(ComputationIssue {message: format!("Overflow occured when performing subtraction on i64s.")})
          }
        },
        (Value::F64(a), Value::F64(b)) => Ok(Value::F64(a - b)),
        (a, b) => Err(ComputationIssue { message: format!("Cannot perform subtraction between {:?} and {:?}.", a, b) })
    }
  }
}
