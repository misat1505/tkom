use std::collections::HashMap;

use crate::{errors::Issue, value::Value};

#[derive(Debug)]
pub struct ScopeManagerIssue {
  message: String
}

impl Issue for ScopeManagerIssue {
  fn message(&self) -> String {
      self.message.clone()
  }
}

#[derive(Debug, Clone)]
pub struct ScopeManager {
  // always has at least 1 scope
  scopes: Vec<Scope>
}

impl ScopeManager {
  pub fn new() -> Self {
    let root_scope = Scope::new();
    ScopeManager { scopes: vec![root_scope] }
  }

  pub fn push_scope(&mut self) {
    let new_scope = Scope::new();
    self.scopes.push(new_scope);
  }

  pub fn pop_scope(&mut self) {
    self.scopes.pop();
  }

  pub fn get_variable(&self, searched: String) -> Result<&Value, ScopeManagerIssue> {
    for scope in &self.scopes {
      match scope.get_variable(searched.clone()) {
        Some(var) => return Ok(var),
        None => {}
      }
    }

    Err(ScopeManagerIssue { message: format!("Variable '{}' not declared in this scope.", searched) })
  }

  pub fn assign_variable(&mut self, name: String, value: Value) -> Result<(), ScopeManagerIssue> {
    match self.get_variable(name.clone()) {
      Err(_) => Err(ScopeManagerIssue { message: format!("Variable '{}' not declared.", name) }),
      Ok(_) => {
        for scope in &mut self.scopes {
          match scope.get_variable(name.clone()) {
            None => {},
            Some(_) => {
              scope.assign_variable(name.clone(), value.clone());
            }
          }
        }

        Ok(())
      }
    }
  }

  pub fn declare_variable(&mut self, name: String, value: Value) -> Result<(), ScopeManagerIssue> {
    match self.get_variable(name.clone()) {
        Ok(_) => return Err(ScopeManagerIssue { message: format!("Cannot redeclare variable '{}'.", name.clone()) }),
        Err(_) => {}
    }

    if let Some(last_scope) = self.scopes.last_mut() {
      last_scope.declare_variable(name, value);
      Ok(())
  } else {
      Err(ScopeManagerIssue { message: "No scope available to set the variable.".to_string() })
  }
  }
}

#[derive(Debug, Clone)]
pub struct Scope {
  variables: HashMap<String, Value>
}

impl Scope {
  fn new() -> Self {
    Scope { variables: HashMap::new() }
  }

  fn get_variable(&self, searched: String) -> Option<&Value> {
    self.variables.get(&searched)
  }

  fn assign_variable(&mut self, name: String, value: Value) -> Result<(), ScopeManagerIssue> {
    match self.get_variable(name.clone()) {
      None => Err(ScopeManagerIssue {message: format!("Variable '{}' not declared.", name)}),
      Some(_) => {
        self.variables.insert(name, value);
        Ok(())
      }
    }
  }

  fn declare_variable(&mut self, name: String, value: Value) -> Result<(), ScopeManagerIssue> {
    match self.get_variable(name.clone()) {
      Some(_) => Err(ScopeManagerIssue {message: format!("Cannot redeclare variable '{}'.", name)}),
      None => {
        self.variables.insert(name, value);
        Ok(())
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn initializes_scope() {
    let scope = Scope::new();
    assert!(scope.variables.is_empty());
  }

  #[test]
  fn scope_variables() {
    let mut scope = Scope::new();
    let name = "x".to_owned();
    let value = Value::I64(5);

    let _ = scope.declare_variable(name.clone(), value.clone());
    assert!(scope.get_variable(name.clone()).unwrap().clone() == value);
    assert!(scope.get_variable("non-existent".to_owned()).is_none());

    let new_value = Value::I64(0);
    let _ = scope.assign_variable(name.clone(), new_value.clone());
    assert!(scope.get_variable(name.clone()).unwrap().clone() == new_value);

    assert!(scope.assign_variable("y".to_owned(), Value::Bool(true)).is_err());

  }

  #[test]
  fn initializes_scope_manager() {
    let manager = ScopeManager::new();
    assert_eq!(manager.scopes.len(), 1);
  }

  #[test]
  fn manages_scopes() {
    let mut manager = ScopeManager::new();
    assert!(manager.scopes.len() == 1);

    manager.push_scope();
    assert!(manager.scopes.len() == 2);

    manager.pop_scope();
    assert!(manager.scopes.len() == 1);
  }

  #[test]
  fn manager_variables() {
    // i64 x = 1;
    // {x = 5; i64 y = 2;}
    // {y; i64 y = 3;}

    let mut manager = ScopeManager::new();

    let _ = manager.declare_variable("x".to_owned(), Value::I64(1));
    assert!(manager.get_variable("x".to_owned()).unwrap().clone() == Value::I64(1));

    manager.push_scope();
    assert!(manager.get_variable("x".to_owned()).unwrap().clone() == Value::I64(1));

    let _ = manager.assign_variable("x".to_owned(), Value::I64(5));
    assert!(manager.get_variable("x".to_owned()).unwrap().clone() == Value::I64(5));

    let _ = manager.declare_variable("y".to_owned(), Value::I64(2));
    assert!(manager.get_variable("y".to_owned()).unwrap().clone() == Value::I64(2));

    manager.pop_scope();
    assert!(manager.get_variable("x".to_owned()).unwrap().clone() == Value::I64(5));
    assert!(manager.get_variable("y".to_owned()).is_err());

    manager.push_scope();
    assert!(manager.get_variable("y".to_owned()).is_err());

    let _ = manager.declare_variable("y".to_owned(), Value::I64(3));
    assert!(manager.get_variable("y".to_owned()).unwrap().clone() == Value::I64(3));

    manager.pop_scope();
  }
}