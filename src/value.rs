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
            val => Err(ComputationIssue {
                message: format!("Cannot perform boolean negation on {:?}.", val),
            }),
        }
    }

    pub fn arithmetic_negate(&self) -> Result<Value, ComputationIssue> {
        match self {
            Value::I64(i64) => Ok(Value::I64(-i64)),
            Value::F64(f64) => Ok(Value::F64(-f64)),
            val => Err(ComputationIssue {
                message: format!("Cannot perform arithmetic negation on {:?}.", val),
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
            (a, b) => Err(ComputationIssue {
                message: format!("Cannot perform concatenation between {:?} and {:?}.", a, b),
            }),
        }
    }

    pub fn alternative(&self, other: Value) -> Result<Value, ComputationIssue> {
        match (self, other) {
            (Value::Bool(bool1), Value::Bool(bool2)) => Ok(Value::Bool(*bool1 || bool2)),
            (a, b) => Err(ComputationIssue {
                message: format!("Cannot perform concatenation between {:?} and {:?}.", a, b),
            }),
        }
    }

    pub fn greater(&self, other: Value) -> Result<Value, ComputationIssue> {
        match (self, other) {
            (Value::I64(val1), Value::I64(val2)) => Ok(Value::Bool(*val1 > val2)),
            (Value::F64(val1), Value::F64(val2)) => Ok(Value::Bool(*val1 > val2)),
            (a, b) => Err(ComputationIssue {
                message: format!("Cannot perform greater between {:?} and {:?}.", a, b),
            }),
        }
    }

    pub fn greater_or_equal(&self, other: Value) -> Result<Value, ComputationIssue> {
        match (self, other) {
            (Value::I64(val1), Value::I64(val2)) => Ok(Value::Bool(*val1 >= val2)),
            (Value::F64(val1), Value::F64(val2)) => Ok(Value::Bool(*val1 >= val2)),
            (a, b) => Err(ComputationIssue {
                message: format!(
                    "Cannot perform greater or equal between {:?} and {:?}.",
                    a, b
                ),
            }),
        }
    }

    pub fn less(&self, other: Value) -> Result<Value, ComputationIssue> {
        match (self, other) {
            (Value::I64(val1), Value::I64(val2)) => Ok(Value::Bool(*val1 < val2)),
            (Value::F64(val1), Value::F64(val2)) => Ok(Value::Bool(*val1 < val2)),
            (a, b) => Err(ComputationIssue {
                message: format!("Cannot perform less between {:?} and {:?}.", a, b),
            }),
        }
    }

    pub fn less_or_equal(&self, other: Value) -> Result<Value, ComputationIssue> {
        match (self, other) {
            (Value::I64(val1), Value::I64(val2)) => Ok(Value::Bool(*val1 <= val2)),
            (Value::F64(val1), Value::F64(val2)) => Ok(Value::Bool(*val1 <= val2)),
            (a, b) => Err(ComputationIssue {
                message: format!("Cannot perform less or equal between {:?} and {:?}.", a, b),
            }),
        }
    }

    pub fn equal(&self, other: Value) -> Result<Value, ComputationIssue> {
        match (self, other) {
            (Value::I64(val1), Value::I64(val2)) => Ok(Value::Bool(*val1 == val2)),
            (Value::F64(val1), Value::F64(val2)) => Ok(Value::Bool(*val1 == val2)),
            (Value::String(val1), Value::String(val2)) => Ok(Value::Bool(*val1 == val2)),
            (Value::Bool(val1), Value::Bool(val2)) => Ok(Value::Bool(*val1 == val2)),
            (a, b) => Err(ComputationIssue {
                message: format!("Cannot perform equal between {:?} and {:?}.", a, b),
            }),
        }
    }

    pub fn not_equal(&self, other: Value) -> Result<Value, ComputationIssue> {
        match (self, other) {
            (Value::I64(val1), Value::I64(val2)) => Ok(Value::Bool(*val1 != val2)),
            (Value::F64(val1), Value::F64(val2)) => Ok(Value::Bool(*val1 != val2)),
            (Value::String(val1), Value::String(val2)) => Ok(Value::Bool(*val1 != val2)),
            (Value::Bool(val1), Value::Bool(val2)) => Ok(Value::Bool(*val1 != val2)),
            (a, b) => Err(ComputationIssue {
                message: format!("Cannot perform not equal between {:?} and {:?}.", a, b),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_values() {
        let data = [Type::Bool, Type::I64, Type::F64, Type::Str];

        let expected = [
            Value::Bool(false),
            Value::I64(0),
            Value::F64(0.0),
            Value::String(String::from("")),
        ];

        for idx in 0..data.len() {
            assert!(Value::default_value(data[idx]).unwrap() == expected[idx]);
        }
    }

    #[test]
    fn default_values_fail() {
        assert!(Value::default_value(Type::Void).is_err());
    }

    #[test]
    fn cast_to_type() {
        let data = [
            (Value::I64(1), Type::Str),
            (Value::F64(1.2), Type::Str),
            (Value::I64(1), Type::F64),
            (Value::F64(1.2), Type::I64),
            (Value::String(String::from("1")), Type::I64),
            (Value::String(String::from("1.2")), Type::F64),
            (Value::String(String::from("true")), Type::Bool),
            (Value::String(String::from("false")), Type::Bool),
        ];

        let expected = [
            Value::String(String::from("1")),
            Value::String(String::from("1.2")),
            Value::F64(1.0),
            Value::I64(1),
            Value::I64(1),
            Value::F64(1.2),
            Value::Bool(true),
            Value::Bool(false),
        ];

        for idx in 0..data.len() {
            let (init, to_type) = &data[idx];
            let exp = &expected[idx];
            assert!(init.cast_to_type(*to_type).unwrap() == *exp);
        }
    }

    #[test]
    fn cast_to_type_fail() {
        let data = [
            (Value::String(String::from("abc")), Type::I64),
            (Value::String(String::from("abc")), Type::F64),
            (Value::String(String::from("1")), Type::Bool),
            (Value::I64(1), Type::Bool),
        ];

        for (val, to_type) in data {
            assert!(val.cast_to_type(to_type).is_err());
        }
    }

    #[test]
    fn boolean_negation() {
        assert!(Value::Bool(false).boolean_negate().unwrap() == Value::Bool(true));
        assert!(Value::Bool(true).boolean_negate().unwrap() == Value::Bool(false));
        assert!(Value::I64(1).boolean_negate().is_err());
    }

    #[test]
    fn arithmetic_negation() {
        assert!(Value::I64(1).arithmetic_negate().unwrap() == Value::I64(-1));
        assert!(Value::F64(-21.37).arithmetic_negate().unwrap() == Value::F64(21.37));
        assert!(Value::String(String::from("abc"))
            .arithmetic_negate()
            .is_err());
    }

    #[test]
    fn add() {
        let data = [
            (Value::I64(1), Value::I64(2)),
            (Value::F64(1.5), Value::F64(2.5)),
            (
                Value::String(String::from("Papollo")),
                Value::String(String::from("2137")),
            ),
        ];

        let expected = [
            Value::I64(3),
            Value::F64(4.0),
            Value::String(String::from("Papollo2137")),
        ];

        for idx in 0..data.len() {
            let (val1, val2) = &data[idx];
            assert!(val1.add(val2.clone()).unwrap() == expected[idx]);
        }
    }

    #[test]
    fn add_fail() {
        assert!(Value::I64(6532475327647647762)
            .add(Value::I64(6532475327647647762))
            .is_err());
        assert!(Value::I64(1).add(Value::F64(2.0)).is_err());
    }

    #[test]
    fn subtract() {
        let data = [
            (Value::I64(1), Value::I64(2)),
            (Value::F64(1.5), Value::F64(2.5)),
        ];

        let expected = [Value::I64(-1), Value::F64(-1.0)];

        for idx in 0..data.len() {
            let (val1, val2) = &data[idx];
            assert!(val1.subtract(val2.clone()).unwrap() == expected[idx]);
        }
    }

    #[test]
    fn subtract_fail() {
        assert!(Value::I64(-6532475327647647762)
            .subtract(Value::I64(6532475327647647762))
            .is_err());
        assert!(Value::I64(1).subtract(Value::F64(2.0)).is_err());
        assert!(Value::String(String::from("a"))
            .subtract(Value::String(String::from("a")))
            .is_err());
    }

    #[test]
    fn multiplication() {
        let data = [
            (Value::I64(1), Value::I64(2)),
            (Value::F64(1.5), Value::F64(2.5)),
        ];

        let expected = [Value::I64(2), Value::F64(3.75)];

        for idx in 0..data.len() {
            let (val1, val2) = &data[idx];
            assert!(val1.multiplication(val2.clone()).unwrap() == expected[idx]);
        }
    }

    #[test]
    fn multiplication_fail() {
        assert!(Value::I64(6532475327647647762)
            .multiplication(Value::I64(6532475327647647762))
            .is_err());
        assert!(Value::I64(1).multiplication(Value::F64(2.0)).is_err());
        assert!(Value::String(String::from("a"))
            .multiplication(Value::String(String::from("a")))
            .is_err());
    }

    #[test]
    fn division() {
        let data = [
            (Value::I64(1), Value::I64(2)),
            (Value::F64(1.5), Value::F64(2.5)),
        ];

        let expected = [Value::I64(0), Value::F64(0.6)];

        for idx in 0..data.len() {
            let (val1, val2) = &data[idx];
            assert!(val1.division(val2.clone()).unwrap() == expected[idx]);
        }
    }

    #[test]
    fn division_fail() {
        assert!(Value::I64(6532475327647647762)
            .division(Value::I64(0))
            .is_err());
        assert!(Value::I64(1).division(Value::F64(2.0)).is_err());
        assert!(Value::String(String::from("a"))
            .division(Value::String(String::from("a")))
            .is_err());
    }

    #[test]
    fn concatenation() {
        assert!(Value::Bool(true).concatenation(Value::Bool(true)).unwrap() == Value::Bool(true));
        assert!(Value::Bool(false).concatenation(Value::Bool(true)).unwrap() == Value::Bool(false));
        assert!(Value::Bool(true).concatenation(Value::Bool(false)).unwrap() == Value::Bool(false));
        assert!(
            Value::Bool(false)
                .concatenation(Value::Bool(false))
                .unwrap()
                == Value::Bool(false)
        );
        assert!(Value::Bool(true).concatenation(Value::I64(1)).is_err());
    }

    #[test]
    fn alternative() {
        assert!(Value::Bool(true).alternative(Value::Bool(true)).unwrap() == Value::Bool(true));
        assert!(Value::Bool(false).alternative(Value::Bool(true)).unwrap() == Value::Bool(true));
        assert!(Value::Bool(true).alternative(Value::Bool(false)).unwrap() == Value::Bool(true));
        assert!(Value::Bool(false).alternative(Value::Bool(false)).unwrap() == Value::Bool(false));
        assert!(Value::Bool(true).alternative(Value::I64(1)).is_err());
    }

    #[test]
    fn greater() {
        assert!(Value::I64(1).greater(Value::I64(2)).unwrap() == Value::Bool(false));
        assert!(Value::I64(2).greater(Value::I64(2)).unwrap() == Value::Bool(false));
        assert!(Value::I64(3).greater(Value::I64(2)).unwrap() == Value::Bool(true));
        assert!(Value::F64(1.0).greater(Value::F64(2.0)).unwrap() == Value::Bool(false));
        assert!(Value::F64(2.0).greater(Value::F64(2.0)).unwrap() == Value::Bool(false));
        assert!(Value::F64(3.0).greater(Value::F64(2.0)).unwrap() == Value::Bool(true));
        assert!(Value::I64(2).greater(Value::F64(3.0)).is_err());
    }

    #[test]
    fn greater_or_equal() {
        assert!(Value::I64(1).greater_or_equal(Value::I64(2)).unwrap() == Value::Bool(false));
        assert!(Value::I64(2).greater_or_equal(Value::I64(2)).unwrap() == Value::Bool(true));
        assert!(Value::I64(3).greater_or_equal(Value::I64(2)).unwrap() == Value::Bool(true));
        assert!(Value::F64(1.0).greater_or_equal(Value::F64(2.0)).unwrap() == Value::Bool(false));
        assert!(Value::F64(2.0).greater_or_equal(Value::F64(2.0)).unwrap() == Value::Bool(true));
        assert!(Value::F64(3.0).greater_or_equal(Value::F64(2.0)).unwrap() == Value::Bool(true));
        assert!(Value::I64(2).greater_or_equal(Value::F64(3.0)).is_err());
    }

    #[test]
    fn less() {
        assert!(Value::I64(1).less(Value::I64(2)).unwrap() == Value::Bool(true));
        assert!(Value::I64(2).less(Value::I64(2)).unwrap() == Value::Bool(false));
        assert!(Value::I64(3).less(Value::I64(2)).unwrap() == Value::Bool(false));
        assert!(Value::F64(1.0).less(Value::F64(2.0)).unwrap() == Value::Bool(true));
        assert!(Value::F64(2.0).less(Value::F64(2.0)).unwrap() == Value::Bool(false));
        assert!(Value::F64(3.0).less(Value::F64(2.0)).unwrap() == Value::Bool(false));
        assert!(Value::I64(2).less(Value::F64(3.0)).is_err());
    }

    #[test]
    fn less_or_equal() {
        assert!(Value::I64(1).less_or_equal(Value::I64(2)).unwrap() == Value::Bool(true));
        assert!(Value::I64(2).less_or_equal(Value::I64(2)).unwrap() == Value::Bool(true));
        assert!(Value::I64(3).less_or_equal(Value::I64(2)).unwrap() == Value::Bool(false));
        assert!(Value::F64(1.0).less_or_equal(Value::F64(2.0)).unwrap() == Value::Bool(true));
        assert!(Value::F64(2.0).less_or_equal(Value::F64(2.0)).unwrap() == Value::Bool(true));
        assert!(Value::F64(3.0).less_or_equal(Value::F64(2.0)).unwrap() == Value::Bool(false));
        assert!(Value::I64(2).less_or_equal(Value::F64(3.0)).is_err());
    }

    #[test]
    fn equal() {
        assert!(Value::I64(1).equal(Value::I64(2)).unwrap() == Value::Bool(false));
        assert!(Value::I64(2).equal(Value::I64(2)).unwrap() == Value::Bool(true));
        assert!(Value::F64(1.0).equal(Value::F64(2.0)).unwrap() == Value::Bool(false));
        assert!(Value::F64(2.0).equal(Value::F64(2.0)).unwrap() == Value::Bool(true));
        assert!(
            Value::String(String::from("a"))
                .equal(Value::String(String::from("b")))
                .unwrap()
                == Value::Bool(false)
        );
        assert!(
            Value::String(String::from("a"))
                .equal(Value::String(String::from("a")))
                .unwrap()
                == Value::Bool(true)
        );
        assert!(Value::Bool(true).equal(Value::Bool(false)).unwrap() == Value::Bool(false));
        assert!(Value::Bool(true).equal(Value::Bool(true)).unwrap() == Value::Bool(true));
        assert!(Value::Bool(true).equal(Value::I64(1)).is_err());
    }
}
