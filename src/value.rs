use crate::{ast::Type, errors::Issue};

#[derive(Debug)]
pub struct ComputationIssue {
    message: String,
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
    fn check_int_operation<F>(
        &self,
        other: Value,
        op: F,
        op_name: &str,
    ) -> Result<Value, ComputationIssue>
    where
        F: Fn(i64, i64) -> Option<i64>,
    {
        match (self, other.clone()) {
            (Value::I64(a), Value::I64(b)) => match op(*a, b) {
                Some(result) => Ok(Value::I64(result)),
                None => Err(ComputationIssue {
                    message: format!("Overflow occurred when performing {} on i64s.", op_name),
                }),
            },
            _ => Err(ComputationIssue {
                message: format!(
                    "Cannot perform {} between {:?} and {:?}.",
                    op_name, self, other
                ),
            }),
        }
    }

    fn check_float_operation<F>(
        &self,
        other: Value,
        op: F,
        op_name: &str,
    ) -> Result<Value, ComputationIssue>
    where
        F: Fn(f64, f64) -> f64,
    {
        match (self, other.clone()) {
            (Value::F64(a), Value::F64(b)) => {
                let result = op(*a, b);
                if result.is_infinite() || result.is_nan() {
                    Err(ComputationIssue {
                        message: format!("Invalid result when performing {} on f64s.", op_name),
                    })
                } else {
                    Ok(Value::F64(result))
                }
            }
            _ => Err(ComputationIssue {
                message: format!(
                    "Cannot perform {} between {:?} and {:?}.",
                    op_name, self, other
                ),
            }),
        }
    }
}

impl Value {
    pub fn default_value(var_type: Type) -> Result<Value, ComputationIssue> {
        match var_type {
            Type::Bool => Ok(Value::Bool(false)),
            Type::I64 => Ok(Value::I64(0)),
            Type::F64 => Ok(Value::F64(0.0)),
            Type::Str => Ok(Value::String("".to_owned())),
            a => Err(ComputationIssue {
                message: format!("Cannot create default value for type: {:?}.", a),
            }),
        }
    }

    pub fn add(&self, other: Value) -> Result<Value, ComputationIssue> {
        match (self, &other) {
            (Value::I64(_), Value::I64(_)) => {
                self.check_int_operation(other, i64::checked_add, "addition")
            }
            (Value::F64(_), Value::F64(_)) => {
                self.check_float_operation(other, |a, b| a + b, "addition")
            }
            (Value::String(a), Value::String(b)) => Ok(Value::String(a.clone() + &b.clone())),
            (a, b) => Err(ComputationIssue {
                message: format!("Cannot perform addition between {:?} and {:?}.", a, b),
            }),
        }
    }

    pub fn subtract(&self, other: Value) -> Result<Value, ComputationIssue> {
        match (self, &other) {
            (Value::I64(_), Value::I64(_)) => {
                self.check_int_operation(other, i64::checked_sub, "subtraction")
            }
            (Value::F64(_), Value::F64(_)) => {
                self.check_float_operation(other, |a, b| a - b, "subtraction")
            }
            (a, b) => Err(ComputationIssue {
                message: format!("Cannot perform subtraction between {:?} and {:?}.", a, b),
            }),
        }
    }

    pub fn multiplication(&self, other: Value) -> Result<Value, ComputationIssue> {
      match (self, &other) {
          (Value::I64(_), Value::I64(_)) => {
              self.check_int_operation(other, i64::checked_mul, "multiplication")
          }
          (Value::F64(_), Value::F64(_)) => {
              self.check_float_operation(other, |a, b| a * b, "multiplication")
          }
          (a, b) => Err(ComputationIssue {
              message: format!("Cannot perform multiplication between {:?} and {:?}.", a, b),
          }),
      }
  }


  pub fn division(&self, other: Value) -> Result<Value, ComputationIssue> {
    match (self, &other) {
        (Value::I64(_), Value::I64(_)) => {
            self.check_int_operation(other, i64::checked_div, "division")
        }
        (Value::F64(_), Value::F64(_)) => {
            self.check_float_operation(other, |a, b| a / b, "division")
        }
        (a, b) => Err(ComputationIssue {
            message: format!("Cannot perform division between {:?} and {:?}.", a, b),
        }),
    }
}
}
