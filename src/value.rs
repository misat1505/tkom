#[derive(Debug, Clone, PartialEq)]
pub enum Value {
  I64(i64),
  F64(f64),
  String(String),
  Bool(bool)
}