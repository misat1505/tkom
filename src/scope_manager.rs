use std::collections::HashMap;

use crate::{errors::Issue, value::Value};

#[derive(Debug)]
pub struct ScopeManagerIssue {
    pub message: String,
}

impl Issue for ScopeManagerIssue {
    fn message(&self) -> String {
        self.message.clone()
    }
}

#[derive(Debug, Clone)]
pub struct ScopeManager {
    // always has at least 1 scope
    scopes: Vec<Scope>,
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
            if let Some(var) = scope.get_variable(searched.clone()) {
                return Ok(var);
            }
        }

        Err(ScopeManagerIssue {
            message: format!("Variable '{}' not declared in this scope.", searched),
        })
    }

    pub fn assign_variable(&mut self, name: String, value: Value) -> Result<(), ScopeManagerIssue> {
        match self.get_variable(name.clone()) {
            Err(_) => Err(ScopeManagerIssue {
                message: format!("Variable '{}' not declared.", name),
            }),
            Ok(_) => {
                for scope in &mut self.scopes {
                    if let Some(_) = scope.get_variable(name.clone()) {
                        return scope.assign_variable(name.clone(), value.clone());
                    }
                }

                Ok(())
            }
        }
    }

    pub fn declare_variable(&mut self, name: String, value: Value) -> Result<(), ScopeManagerIssue> {
        if let Ok(_) = self.get_variable(name.clone()) {
            return Err(ScopeManagerIssue {
                message: format!("Cannot redeclare variable '{}'.", name.clone()),
            });
        }

        if let Some(last_scope) = self.scopes.last_mut() {
            let _ = last_scope.declare_variable(name, value);
            Ok(())
        } else {
            Err(ScopeManagerIssue {
                message: "No scope available to set the variable.".to_string(),
            })
        }
    }

    pub fn len(&self) -> u32 {
        self.scopes.len() as u32
    }
}

#[derive(Debug, Clone)]
pub struct Scope {
    variables: HashMap<String, Value>,
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
            None => Err(ScopeManagerIssue {
                message: format!("Variable '{}' not declared.", name),
            }),
            Some(prev_val) => match (prev_val, value.clone()) {
                (Value::I64(_), Value::I64(_))
                | (Value::F64(_), Value::F64(_))
                | (Value::String(_), Value::String(_))
                | (Value::Bool(_), Value::Bool(_)) => {
                    self.variables.insert(name, value);
                    Ok(())
                }
                (a, b) => Err(ScopeManagerIssue {
                    message: format!(
                        "Cannot assign '{:?}' to variable '{}' which was previously declared as '{:?}'.",
                        b.to_type(),
                        name,
                        a.to_type()
                    ),
                }),
            },
        }
    }

    fn declare_variable(&mut self, name: String, value: Value) -> Result<(), ScopeManagerIssue> {
        match self.get_variable(name.clone()) {
            Some(_) => Err(ScopeManagerIssue {
                message: format!("Cannot redeclare variable '{}'.", name),
            }),
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

    #[test]
    fn bad_assign_type() {
        let mut manager = ScopeManager::new();

        let _ = manager.declare_variable("x".to_owned(), Value::I64(1));
        assert!(manager.assign_variable("x".to_owned(), Value::Bool(true)).is_err());
    }
}
