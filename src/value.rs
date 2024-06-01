use crate::{ast::Type, issues::ComputationIssue};

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
            a => Err(ComputationIssue {
                message: format!("Cannot create default value for type '{:?}'.", a),
            }),
        }
    }

    pub fn to_type(&self) -> Type {
        match self {
            Value::Bool(_) => Type::Bool,
            Value::F64(_) => Type::F64,
            Value::I64(_) => Type::I64,
            Value::String(_) => Type::Str,
        }
    }

    pub fn try_into_bool(&self) -> Result<bool, ComputationIssue> {
        match self {
            Value::Bool(bool) => Ok(*bool),
            _ => Err(ComputationIssue {
                message: format!("Given value is not a boolean."),
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

        let expected = [Value::Bool(false), Value::I64(0), Value::F64(0.0), Value::String(String::from(""))];

        for idx in 0..data.len() {
            assert!(Value::default_value(data[idx]).unwrap() == expected[idx]);
        }
    }

    #[test]
    fn default_values_fail() {
        assert!(Value::default_value(Type::Void).is_err());
    }

    #[test]
    fn value_to_type() {
        let values = [Value::Bool(true), Value::I64(5), Value::F64(5.5), Value::String(String::from("hello"))];

        let exp = [Type::Bool, Type::I64, Type::F64, Type::Str];

        for idx in 0..values.len() {
            assert!(values[idx].to_type() == exp[idx]);
        }
    }

    #[test]
    fn try_into_bool() {
        assert!(Value::Bool(true).try_into_bool().unwrap() == true);
        assert!(Value::I64(5).try_into_bool().is_err());
    }
}
