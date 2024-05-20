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
    pub fn cast_to_type(&self, to_type: Type) -> Result<Value, ComputationIssue> {
        match (self, to_type) {
            (Value::I64(i64), Type::Str) => Ok(Value::String(i64.to_string())),
            (Value::F64(f64), Type::Str) => Ok(Value::String(f64.to_string())),
            (Value::I64(i64), Type::F64) => Ok(Value::F64(*i64 as f64)),
            (Value::F64(f64), Type::I64) => Ok(Value::I64(*f64 as i64)),
            (Value::String(string), Type::I64) => match string.parse::<i64>() {
                Ok(i64) => Ok(Value::I64(i64)),
                Err(_) => Err(ComputationIssue {
                    message: format!("Cannot cast String '{}' to i64.", string),
                }),
            },
            (Value::String(string), Type::F64) => match string.parse::<f64>() {
                Ok(f64) => Ok(Value::F64(f64)),
                Err(_) => Err(ComputationIssue {
                    message: format!("Cannot cast String '{}' to f64.", string),
                }),
            },
            (Value::String(string), Type::Bool) => match string.as_str() {
                "true" => Ok(Value::Bool(true)),
                "false" => Ok(Value::Bool(false)),
                _ => Err(ComputationIssue {
                    message: format!("Cannot cast String '{}' to bool.", string),
                }),
            },
            (value, target_type) => Err(ComputationIssue {
              message: format!("Cannot cast {:?} to {:?}.", value, target_type),
          }),
        }
    }

    pub fn boolean_negate(&self) -> Result<Value, ComputationIssue> {
      match self {
          Value::Bool(bool) => Ok(Value::Bool(!bool)),
          val => Err(ComputationIssue { message: format!("Cannot perform boolean negation on {:?}.", val) })
      }
    }

    pub fn arithmetic_negate(&self) -> Result<Value, ComputationIssue> {
      match self {
        Value::I64(i64) => Ok(Value::I64(-i64)),
          Value::F64(f64) => Ok(Value::F64(-f64)),
          val => Err(ComputationIssue { message: format!("Cannot perform arithmetic negation on {:?}.", val) })
      }
    }

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

    pub fn concatenation(&self, other: Value) -> Result<Value, ComputationIssue> {
      match (self, other) {
          (Value::Bool(bool1), Value::Bool(bool2)) => Ok(Value::Bool(*bool1 && bool2)),
          (a, b) => Err(ComputationIssue { message: format!("Cannot perform concatenation between {:?} and {:?}.", a, b) })
      }
    }

    pub fn alternative(&self, other: Value) -> Result<Value, ComputationIssue> {
      match (self, other) {
          (Value::Bool(bool1), Value::Bool(bool2)) => Ok(Value::Bool(*bool1 || bool2)),
          (a, b) => Err(ComputationIssue { message: format!("Cannot perform concatenation between {:?} and {:?}.", a, b) })
      }
    }
}
