use crate::{
    ast::Type,
    errors::{ComputationError, ErrorSeverity},
    value::Value,
};

pub struct ALU;

impl ALU {
    fn check_int_operation<F>(val1: &Value, val2: &Value, op: F, op_name: &str) -> Result<Value, ComputationError>
    where
        F: Fn(i64, i64) -> Option<i64>,
    {
        match (val1, val2) {
            (Value::I64(a), Value::I64(b)) => match op(*a, *b) {
                Some(result) => Ok(Value::I64(result)),
                None => Err(ComputationError::new(
                    ErrorSeverity::HIGH,
                    format!("Overflow occurred when performing {} on i64s.", op_name),
                )),
            },
            _ => Err(ComputationError::new(
                ErrorSeverity::HIGH,
                format!(
                    "Cannot perform {} between values of type '{:?}' and '{:?}'.",
                    op_name,
                    val1.to_type(),
                    val2.to_type()
                ),
            )),
        }
    }

    fn check_float_operation<F>(val1: &Value, val2: &Value, op: F, op_name: &str) -> Result<Value, ComputationError>
    where
        F: Fn(f64, f64) -> f64,
    {
        match (val1, val2) {
            (Value::F64(a), Value::F64(b)) => {
                let result = op(*a, *b);
                if result.is_infinite() || result.is_nan() {
                    Err(ComputationError::new(
                        ErrorSeverity::HIGH,
                        format!("Invalid result when performing {} on f64s.", op_name),
                    ))
                } else {
                    Ok(Value::F64(result))
                }
            }
            _ => Err(ComputationError::new(
                ErrorSeverity::HIGH,
                format!(
                    "Cannot perform {} between values of type '{:?}' and '{:?}'.",
                    op_name,
                    val1.to_type(),
                    val2.to_type()
                ),
            )),
        }
    }
}

impl ALU {
    pub fn cast_to_type(val: Value, to_type: Type) -> Result<Value, ComputationError> {
        match (val, to_type) {
            (Value::I64(i64), Type::Str) => Ok(Value::String(i64.to_string())),
            (Value::F64(f64), Type::Str) => Ok(Value::String(f64.to_string())),
            (Value::I64(i64), Type::F64) => Ok(Value::F64(i64 as f64)),
            (Value::F64(f64), Type::I64) => Ok(Value::I64(f64 as i64)),
            (Value::I64(i64), Type::Bool) => Ok(Value::Bool(i64 > 0)),
            (Value::F64(f64), Type::Bool) => Ok(Value::Bool(f64 > 0.0)),
            (Value::String(string), Type::I64) => match string.parse::<i64>() {
                Ok(i64) => Ok(Value::I64(i64)),
                Err(_) => Err(ComputationError::new(
                    ErrorSeverity::HIGH,
                    format!("Cannot cast String '{}' to 'i64'.", string),
                )),
            },
            (Value::String(string), Type::F64) => match string.parse::<f64>() {
                Ok(f64) => Ok(Value::F64(f64)),
                Err(_) => Err(ComputationError::new(
                    ErrorSeverity::HIGH,
                    format!("Cannot cast String '{}' to 'f64'.", string),
                )),
            },
            (Value::String(string), Type::Bool) => Ok(Value::Bool(string.as_str() != "")),
            (value, target_type) => Err(ComputationError::new(
                ErrorSeverity::HIGH,
                format!("Cannot cast '{:?}' to '{:?}'.", value, target_type),
            )),
        }
    }

    pub fn boolean_negate(val: Value) -> Result<Value, ComputationError> {
        match val {
            Value::Bool(bool) => Ok(Value::Bool(!bool)),
            val => Err(ComputationError::new(
                ErrorSeverity::HIGH,
                format!("Cannot perform boolean negation on type '{:?}'.", val.to_type()),
            )),
        }
    }

    pub fn arithmetic_negate(val: Value) -> Result<Value, ComputationError> {
        match val {
            Value::I64(i64) => Ok(Value::I64(-i64)),
            Value::F64(f64) => Ok(Value::F64(-f64)),
            val => Err(ComputationError::new(
                ErrorSeverity::HIGH,
                format!("Cannot perform arithmetic negation on type '{:?}'.", val.to_type()),
            )),
        }
    }

    pub fn add(val1: Value, val2: Value) -> Result<Value, ComputationError> {
        match (&val1, &val2) {
            (Value::I64(_), Value::I64(_)) => Self::check_int_operation(&val1, &val2, i64::checked_add, "addition"),
            (Value::F64(_), Value::F64(_)) => Self::check_float_operation(&val1, &val2, |a, b| a + b, "addition"),
            (Value::String(a), Value::String(b)) => Ok(Value::String(a.clone() + b)),
            (a, b) => Err(ComputationError::new(
                ErrorSeverity::HIGH,
                format!(
                    "Cannot perform addition between values of type '{:?}' and '{:?}'.",
                    a.to_type(),
                    b.to_type()
                ),
            )),
        }
    }

    pub fn subtract(val1: Value, val2: Value) -> Result<Value, ComputationError> {
        match (&val1, &val2) {
            (Value::I64(_), Value::I64(_)) => Self::check_int_operation(&val1, &val2, i64::checked_sub, "subtraction"),
            (Value::F64(_), Value::F64(_)) => Self::check_float_operation(&val1, &val2, |a, b| a - b, "subtraction"),
            (a, b) => Err(ComputationError::new(
                ErrorSeverity::HIGH,
                format!(
                    "Cannot perform subtraction between values of type '{:?}' and '{:?}'.",
                    a.to_type(),
                    b.to_type()
                ),
            )),
        }
    }

    pub fn multiplication(val1: Value, val2: Value) -> Result<Value, ComputationError> {
        match (&val1, &val2) {
            (Value::I64(_), Value::I64(_)) => Self::check_int_operation(&val1, &val2, i64::checked_mul, "multiplication"),
            (Value::F64(_), Value::F64(_)) => Self::check_float_operation(&val1, &val2, |a, b| a * b, "multiplication"),
            (a, b) => Err(ComputationError::new(
                ErrorSeverity::HIGH,
                format!(
                    "Cannot perform multiplication between values of type '{:?}' and '{:?}'.",
                    a.to_type(),
                    b.to_type()
                ),
            )),
        }
    }

    pub fn division(val1: Value, val2: Value) -> Result<Value, ComputationError> {
        match (&val1, &val2) {
            (Value::I64(_), Value::I64(_)) => Self::check_int_operation(&val1, &val2, i64::checked_div, "division"),
            (Value::F64(_), Value::F64(_)) => Self::check_float_operation(&val1, &val2, |a, b| a / b, "division"),
            (a, b) => Err(ComputationError::new(
                ErrorSeverity::HIGH,
                format!(
                    "Cannot perform division between values of type '{:?}' and '{:?}'.",
                    a.to_type(),
                    b.to_type()
                ),
            )),
        }
    }

    pub fn concatenation(val1: Value, val2: Value) -> Result<Value, ComputationError> {
        match (val1, val2) {
            (Value::Bool(bool1), Value::Bool(bool2)) => Ok(Value::Bool(bool1 && bool2)),
            (a, b) => Err(ComputationError::new(
                ErrorSeverity::HIGH,
                format!(
                    "Cannot perform concatenation between values of type '{:?}' and '{:?}'.",
                    a.to_type(),
                    b.to_type()
                ),
            )),
        }
    }

    pub fn alternative(val1: Value, val2: Value) -> Result<Value, ComputationError> {
        match (val1, val2) {
            (Value::Bool(bool1), Value::Bool(bool2)) => Ok(Value::Bool(bool1 || bool2)),
            (a, b) => Err(ComputationError::new(
                ErrorSeverity::HIGH,
                format!(
                    "Cannot perform alternative between values of type '{:?}' and '{:?}'.",
                    a.to_type(),
                    b.to_type()
                ),
            )),
        }
    }

    pub fn greater(val1: Value, val2: Value) -> Result<Value, ComputationError> {
        match (val1, val2) {
            (Value::I64(val1), Value::I64(val2)) => Ok(Value::Bool(val1 > val2)),
            (Value::F64(val1), Value::F64(val2)) => Ok(Value::Bool(val1 > val2)),
            (a, b) => Err(ComputationError::new(
                ErrorSeverity::HIGH,
                format!(
                    "Cannot perform greater between values of type '{:?}' and '{:?}'.",
                    a.to_type(),
                    b.to_type()
                ),
            )),
        }
    }

    pub fn greater_or_equal(val1: Value, val2: Value) -> Result<Value, ComputationError> {
        match (val1, val2) {
            (Value::I64(val1), Value::I64(val2)) => Ok(Value::Bool(val1 >= val2)),
            (Value::F64(val1), Value::F64(val2)) => Ok(Value::Bool(val1 >= val2)),
            (a, b) => Err(ComputationError::new(
                ErrorSeverity::HIGH,
                format!(
                    "Cannot perform greater or equal between values of type '{:?}' and '{:?}'.",
                    a.to_type(),
                    b.to_type()
                ),
            )),
        }
    }

    pub fn less(val1: Value, val2: Value) -> Result<Value, ComputationError> {
        match (val1, val2) {
            (Value::I64(val1), Value::I64(val2)) => Ok(Value::Bool(val1 < val2)),
            (Value::F64(val1), Value::F64(val2)) => Ok(Value::Bool(val1 < val2)),
            (a, b) => Err(ComputationError::new(
                ErrorSeverity::HIGH,
                format!("Cannot perform less between values of type '{:?}' and '{:?}'.", a.to_type(), b.to_type()),
            )),
        }
    }

    pub fn less_or_equal(val1: Value, val2: Value) -> Result<Value, ComputationError> {
        match (val1, val2) {
            (Value::I64(val1), Value::I64(val2)) => Ok(Value::Bool(val1 <= val2)),
            (Value::F64(val1), Value::F64(val2)) => Ok(Value::Bool(val1 <= val2)),
            (a, b) => Err(ComputationError::new(
                ErrorSeverity::HIGH,
                format!(
                    "Cannot perform less or equal between values of type '{:?}' and '{:?}'.",
                    a.to_type(),
                    b.to_type()
                ),
            )),
        }
    }

    pub fn equal(val1: Value, val2: Value) -> Result<Value, ComputationError> {
        match (val1, val2) {
            (Value::I64(val1), Value::I64(val2)) => Ok(Value::Bool(val1 == val2)),
            (Value::F64(val1), Value::F64(val2)) => Ok(Value::Bool(val1 == val2)),
            (Value::String(val1), Value::String(val2)) => Ok(Value::Bool(val1 == val2)),
            (Value::Bool(val1), Value::Bool(val2)) => Ok(Value::Bool(val1 == val2)),
            (a, b) => Err(ComputationError::new(
                ErrorSeverity::HIGH,
                format!("Cannot perform equal between values of type '{:?}' and '{:?}'.", a.to_type(), b.to_type()),
            )),
        }
    }

    pub fn not_equal(val1: Value, val2: Value) -> Result<Value, ComputationError> {
        match (val1, val2) {
            (Value::I64(val1), Value::I64(val2)) => Ok(Value::Bool(val1 != val2)),
            (Value::F64(val1), Value::F64(val2)) => Ok(Value::Bool(val1 != val2)),
            (Value::String(val1), Value::String(val2)) => Ok(Value::Bool(val1 != val2)),
            (Value::Bool(val1), Value::Bool(val2)) => Ok(Value::Bool(val1 != val2)),
            (a, b) => Err(ComputationError::new(
                ErrorSeverity::HIGH,
                format!(
                    "Cannot perform not equal between values of type '{:?}' and '{:?}'.",
                    a.to_type(),
                    b.to_type()
                ),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::errors::IError;

    use super::*;

    #[test]
    fn cast_to_type() {
        let data = [
            (Value::I64(1), Type::Str),
            (Value::F64(1.2), Type::Str),
            (Value::I64(1), Type::F64),
            (Value::F64(1.2), Type::I64),
            (Value::I64(1), Type::Bool),
            (Value::I64(0), Type::Bool),
            (Value::F64(1.2), Type::Bool),
            (Value::F64(0.0), Type::Bool),
            (Value::String(String::from("1")), Type::I64),
            (Value::String(String::from("1.2")), Type::F64),
            (Value::String(String::from("some string")), Type::Bool),
            (Value::String(String::from("")), Type::Bool),
        ];

        let expected = [
            Value::String(String::from("1")),
            Value::String(String::from("1.2")),
            Value::F64(1.0),
            Value::I64(1),
            Value::Bool(true),
            Value::Bool(false),
            Value::Bool(true),
            Value::Bool(false),
            Value::I64(1),
            Value::F64(1.2),
            Value::Bool(true),
            Value::Bool(false),
        ];

        for idx in 0..data.len() {
            let (init, to_type) = &data[idx];
            let exp = &expected[idx];
            assert_eq!(ALU::cast_to_type(init.clone(), *to_type).unwrap(), *exp);
        }
    }

    #[test]
    fn cast_to_type_fail() {
        let data = [
            (Value::String(String::from("abc")), Type::I64),
            (Value::String(String::from("abc")), Type::F64),
        ];

        for (val, to_type) in data {
            assert_eq!(
                ALU::cast_to_type(val, to_type).err().unwrap().message(),
                format!("Cannot cast String 'abc' to '{:?}'.", to_type)
            );
        }
    }

    #[test]
    fn boolean_negation() {
        assert_eq!(ALU::boolean_negate(Value::Bool(false)).unwrap(), Value::Bool(true));
        assert_eq!(ALU::boolean_negate(Value::Bool(true)).unwrap(), Value::Bool(false));
        assert_eq!(
            ALU::boolean_negate(Value::I64(1)).err().unwrap().message(),
            String::from("Cannot perform boolean negation on type 'i64'.")
        );
    }

    #[test]
    fn arithmetic_negation() {
        assert_eq!(ALU::arithmetic_negate(Value::I64(1)).unwrap(), Value::I64(-1));
        assert_eq!(ALU::arithmetic_negate(Value::F64(-21.37)).unwrap(), Value::F64(21.37));
        assert_eq!(
            ALU::arithmetic_negate(Value::String(String::from("abc"))).err().unwrap().message(),
            String::from("Cannot perform arithmetic negation on type 'str'.")
        );
    }

    #[test]
    fn add() {
        let data = [
            (Value::I64(1), Value::I64(2)),
            (Value::F64(1.5), Value::F64(2.5)),
            (Value::String(String::from("Papollo")), Value::String(String::from("2137"))),
        ];

        let expected = [Value::I64(3), Value::F64(4.0), Value::String(String::from("Papollo2137"))];

        for idx in 0..data.len() {
            let (val1, val2) = &data[idx];
            assert_eq!(ALU::add(val1.clone(), val2.clone()).unwrap(), expected[idx]);
        }
    }

    #[test]
    fn add_fail() {
        assert_eq!(
            ALU::add(Value::I64(6532475327647647762), Value::I64(6532475327647647762))
                .err()
                .unwrap()
                .message(),
            String::from("Overflow occurred when performing addition on i64s.")
        );
        assert_eq!(
            ALU::add(Value::I64(1), Value::F64(2.0)).err().unwrap().message(),
            String::from("Cannot perform addition between values of type 'i64' and 'f64'.")
        );
    }

    #[test]
    fn subtract() {
        let data = [(Value::I64(1), Value::I64(2)), (Value::F64(1.5), Value::F64(2.5))];

        let expected = [Value::I64(-1), Value::F64(-1.0)];

        for idx in 0..data.len() {
            let (val1, val2) = &data[idx];
            assert_eq!(ALU::subtract(val1.clone(), val2.clone()).unwrap(), expected[idx]);
        }
    }

    #[test]
    fn subtract_fail() {
        assert_eq!(
            ALU::subtract(Value::I64(-6532475327647647762), Value::I64(6532475327647647762))
                .err()
                .unwrap()
                .message(),
            String::from("Overflow occurred when performing subtraction on i64s.")
        );
        assert_eq!(
            ALU::subtract(Value::I64(1), Value::F64(2.0)).err().unwrap().message(),
            String::from("Cannot perform subtraction between values of type 'i64' and 'f64'.")
        );
        assert_eq!(
            ALU::subtract(Value::String(String::from("a")), Value::String(String::from("a")))
                .err()
                .unwrap()
                .message(),
            String::from("Cannot perform subtraction between values of type 'str' and 'str'.")
        );
    }

    #[test]
    fn multiplication() {
        let data = [(Value::I64(1), Value::I64(2)), (Value::F64(1.5), Value::F64(2.5))];

        let expected = [Value::I64(2), Value::F64(3.75)];

        for idx in 0..data.len() {
            let (val1, val2) = &data[idx];
            assert_eq!(ALU::multiplication(val1.clone(), val2.clone()).unwrap(), expected[idx]);
        }
    }

    #[test]
    fn multiplication_fail() {
        assert_eq!(
            ALU::multiplication(Value::I64(6532475327647647762), Value::I64(6532475327647647762))
                .err()
                .unwrap()
                .message(),
            String::from("Overflow occurred when performing multiplication on i64s.")
        );
        assert_eq!(
            ALU::multiplication(Value::I64(1), Value::F64(2.0)).err().unwrap().message(),
            String::from("Cannot perform multiplication between values of type 'i64' and 'f64'.")
        );
        assert_eq!(
            ALU::multiplication(Value::String(String::from("a")), Value::String(String::from("a")))
                .err()
                .unwrap()
                .message(),
            String::from("Cannot perform multiplication between values of type 'str' and 'str'.")
        );
    }

    #[test]
    fn division() {
        let data = [(Value::I64(1), Value::I64(2)), (Value::F64(1.5), Value::F64(2.5))];

        let expected = [Value::I64(0), Value::F64(0.6)];

        for idx in 0..data.len() {
            let (val1, val2) = &data[idx];
            assert_eq!(ALU::division(val1.clone(), val2.clone()).unwrap(), expected[idx]);
        }
    }

    #[test]
    fn division_fail() {
        assert_eq!(
            ALU::division(Value::I64(6532475327647647762), Value::I64(0)).err().unwrap().message(),
            String::from("Overflow occurred when performing division on i64s.")
        );
        assert_eq!(
            ALU::division(Value::I64(1), Value::F64(2.0)).err().unwrap().message(),
            String::from("Cannot perform division between values of type 'i64' and 'f64'.")
        );
        assert_eq!(
            ALU::division(Value::String(String::from("a")), Value::String(String::from("a")))
                .err()
                .unwrap()
                .message(),
            String::from("Cannot perform division between values of type 'str' and 'str'.")
        );
    }

    #[test]
    fn concatenation() {
        assert_eq!(ALU::concatenation(Value::Bool(true), Value::Bool(true)).unwrap(), Value::Bool(true));
        assert_eq!(ALU::concatenation(Value::Bool(false), Value::Bool(true)).unwrap(), Value::Bool(false));
        assert_eq!(ALU::concatenation(Value::Bool(true), Value::Bool(false)).unwrap(), Value::Bool(false));
        assert_eq!(ALU::concatenation(Value::Bool(false), Value::Bool(false)).unwrap(), Value::Bool(false));
        assert_eq!(
            ALU::concatenation(Value::Bool(true), Value::I64(1)).err().unwrap().message(),
            String::from("Cannot perform concatenation between values of type 'bool' and 'i64'.")
        );
    }

    #[test]
    fn alternative() {
        assert_eq!(ALU::alternative(Value::Bool(true), Value::Bool(true)).unwrap(), Value::Bool(true));
        assert_eq!(ALU::alternative(Value::Bool(false), Value::Bool(true)).unwrap(), Value::Bool(true));
        assert_eq!(ALU::alternative(Value::Bool(true), Value::Bool(false)).unwrap(), Value::Bool(true));
        assert_eq!(ALU::alternative(Value::Bool(false), Value::Bool(false)).unwrap(), Value::Bool(false));
        assert_eq!(
            ALU::alternative(Value::Bool(true), Value::I64(1)).err().unwrap().message(),
            String::from("Cannot perform alternative between values of type 'bool' and 'i64'.")
        );
    }

    #[test]
    fn greater() {
        assert_eq!(ALU::greater(Value::I64(1), Value::I64(2)).unwrap(), Value::Bool(false));
        assert_eq!(ALU::greater(Value::I64(2), Value::I64(2)).unwrap(), Value::Bool(false));
        assert_eq!(ALU::greater(Value::I64(3), Value::I64(2)).unwrap(), Value::Bool(true));
        assert_eq!(ALU::greater(Value::F64(1.0), Value::F64(2.0)).unwrap(), Value::Bool(false));
        assert_eq!(ALU::greater(Value::F64(2.0), Value::F64(2.0)).unwrap(), Value::Bool(false));
        assert_eq!(ALU::greater(Value::F64(3.0), Value::F64(2.0)).unwrap(), Value::Bool(true));
        assert_eq!(
            ALU::greater(Value::I64(2), Value::F64(3.0)).err().unwrap().message(),
            String::from("Cannot perform greater between values of type 'i64' and 'f64'.")
        );
    }

    #[test]
    fn greater_or_equal() {
        assert_eq!(ALU::greater_or_equal(Value::I64(1), Value::I64(2)).unwrap(), Value::Bool(false));
        assert_eq!(ALU::greater_or_equal(Value::I64(2), Value::I64(2)).unwrap(), Value::Bool(true));
        assert_eq!(ALU::greater_or_equal(Value::I64(3), Value::I64(2)).unwrap(), Value::Bool(true));
        assert_eq!(ALU::greater_or_equal(Value::F64(1.0), Value::F64(2.0)).unwrap(), Value::Bool(false));
        assert_eq!(ALU::greater_or_equal(Value::F64(2.0), Value::F64(2.0)).unwrap(), Value::Bool(true));
        assert_eq!(ALU::greater_or_equal(Value::F64(3.0), Value::F64(2.0)).unwrap(), Value::Bool(true));
        assert_eq!(
            ALU::greater_or_equal(Value::I64(2), Value::F64(3.0)).err().unwrap().message(),
            String::from("Cannot perform greater or equal between values of type 'i64' and 'f64'.")
        );
    }

    #[test]
    fn less() {
        assert_eq!(ALU::less(Value::I64(1), Value::I64(2)).unwrap(), Value::Bool(true));
        assert_eq!(ALU::less(Value::I64(2), Value::I64(2)).unwrap(), Value::Bool(false));
        assert_eq!(ALU::less(Value::I64(3), Value::I64(2)).unwrap(), Value::Bool(false));
        assert_eq!(ALU::less(Value::F64(1.0), Value::F64(2.0)).unwrap(), Value::Bool(true));
        assert_eq!(ALU::less(Value::F64(2.0), Value::F64(2.0)).unwrap(), Value::Bool(false));
        assert_eq!(ALU::less(Value::F64(3.0), Value::F64(2.0)).unwrap(), Value::Bool(false));
        assert_eq!(
            ALU::less(Value::I64(2), Value::F64(3.0)).err().unwrap().message(),
            String::from("Cannot perform less between values of type 'i64' and 'f64'.")
        );
    }

    #[test]
    fn less_or_equal() {
        assert_eq!(ALU::less_or_equal(Value::I64(1), Value::I64(2)).unwrap(), Value::Bool(true));
        assert_eq!(ALU::less_or_equal(Value::I64(2), Value::I64(2)).unwrap(), Value::Bool(true));
        assert_eq!(ALU::less_or_equal(Value::I64(3), Value::I64(2)).unwrap(), Value::Bool(false));
        assert_eq!(ALU::less_or_equal(Value::F64(1.0), Value::F64(2.0)).unwrap(), Value::Bool(true));
        assert_eq!(ALU::less_or_equal(Value::F64(2.0), Value::F64(2.0)).unwrap(), Value::Bool(true));
        assert_eq!(ALU::less_or_equal(Value::F64(3.0), Value::F64(2.0)).unwrap(), Value::Bool(false));
        assert_eq!(
            ALU::less_or_equal(Value::I64(2), Value::F64(3.0)).err().unwrap().message(),
            String::from("Cannot perform less or equal between values of type 'i64' and 'f64'.")
        );
    }

    #[test]
    fn equal() {
        assert_eq!(ALU::equal(Value::I64(1), Value::I64(2)).unwrap(), Value::Bool(false));
        assert_eq!(ALU::equal(Value::I64(2), Value::I64(2)).unwrap(), Value::Bool(true));
        assert_eq!(ALU::equal(Value::F64(1.0), Value::F64(2.0)).unwrap(), Value::Bool(false));
        assert_eq!(ALU::equal(Value::F64(2.0), Value::F64(2.0)).unwrap(), Value::Bool(true));
        assert_eq!(
            ALU::equal(Value::String(String::from("a")), Value::String(String::from("b"))).unwrap(),
            Value::Bool(false)
        );
        assert_eq!(
            ALU::equal(Value::String(String::from("a")), Value::String(String::from("a"))).unwrap(),
            Value::Bool(true)
        );
        assert_eq!(ALU::equal(Value::Bool(true), Value::Bool(false)).unwrap(), Value::Bool(false));
        assert_eq!(ALU::equal(Value::Bool(true), Value::Bool(true)).unwrap(), Value::Bool(true));
        assert_eq!(
            ALU::equal(Value::Bool(true), Value::I64(1)).err().unwrap().message(),
            String::from("Cannot perform equal between values of type 'bool' and 'i64'.")
        );
    }

    #[test]
    fn not_equal() {
        assert_eq!(ALU::not_equal(Value::I64(1), Value::I64(2)).unwrap(), Value::Bool(true));
        assert_eq!(ALU::not_equal(Value::I64(2), Value::I64(2)).unwrap(), Value::Bool(false));
        assert_eq!(ALU::not_equal(Value::F64(1.0), Value::F64(2.0)).unwrap(), Value::Bool(true));
        assert_eq!(ALU::not_equal(Value::F64(2.0), Value::F64(2.0)).unwrap(), Value::Bool(false));
        assert_eq!(
            ALU::not_equal(Value::String(String::from("a")), Value::String(String::from("b"))).unwrap(),
            Value::Bool(true)
        );
        assert_eq!(
            ALU::not_equal(Value::String(String::from("a")), Value::String(String::from("a"))).unwrap(),
            Value::Bool(false)
        );
        assert_eq!(ALU::not_equal(Value::Bool(true), Value::Bool(false)).unwrap(), Value::Bool(true));
        assert_eq!(ALU::not_equal(Value::Bool(true), Value::Bool(true)).unwrap(), Value::Bool(false));
        assert_eq!(
            ALU::not_equal(Value::Bool(true), Value::I64(1)).err().unwrap().message(),
            String::from("Cannot perform not equal between values of type 'bool' and 'i64'.")
        );
    }
}
