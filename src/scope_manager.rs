use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::{issues::ScopeManagerIssue, value::Value};

#[derive(Debug, Clone)]
pub struct ScopeManager<'a> {
    // always has at least 1 scope
    scopes: Vec<Scope<'a>>,
}

impl<'a> ScopeManager<'a> {
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

    pub fn get_variable(&self, searched: &'a str) -> Result<&Rc<RefCell<Value>>, ScopeManagerIssue> {
        for scope in &self.scopes {
            if let Some(var) = scope.get_variable(searched) {
                return Ok(var);
            }
        }

        Err(ScopeManagerIssue {
            message: format!("Variable '{}' not declared in this scope.", searched),
        })
    }

    pub fn assign_variable(&mut self, name: &'a str, value: Rc<RefCell<Value>>) -> Result<(), ScopeManagerIssue> {
        for scope in &mut self.scopes {
            if let Some(_) = scope.get_variable(name) {
                return scope.assign_variable(name, value);
            }
        }

        Err(ScopeManagerIssue {
            message: format!("Variable '{}' not declared in this scope.", name),
        })
    }

    pub fn declare_variable(&mut self, name: &'a str, value: Rc<RefCell<Value>>) -> Result<(), ScopeManagerIssue> {
        if self.get_variable(name).is_ok() {
            return Err(ScopeManagerIssue {
                message: format!("Cannot redeclare variable '{}'.", name),
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

    #[allow(dead_code)]
    pub fn len(&self) -> u32 {
        self.scopes.len() as u32
    }
}

#[derive(Debug, Clone)]
pub struct Scope<'a> {
    variables: HashMap<&'a str, Rc<RefCell<Value>>>,
}

impl<'a> Scope<'a> {
    fn new() -> Self {
        Scope { variables: HashMap::new() }
    }

    fn get_variable(&self, searched: &'a str) -> Option<&Rc<RefCell<Value>>> {
        self.variables.get(searched)
    }

    fn assign_variable(&mut self, name: &'a str, value: Rc<RefCell<Value>>) -> Result<(), ScopeManagerIssue> {
        let current_value_option = self.get_variable(name);
        match current_value_option {
            None => Err(ScopeManagerIssue {
                message: format!("Variable '{}' not declared.", name),
            }),
            Some(prev_val) => {
                let mut prev_val_borrow = prev_val.borrow_mut();
                let new_val_borrow = value.borrow();
                match (&*prev_val_borrow, &*new_val_borrow) {
                    (Value::I64(_), Value::I64(_))
                    | (Value::F64(_), Value::F64(_))
                    | (Value::String(_), Value::String(_))
                    | (Value::Bool(_), Value::Bool(_)) => {
                        *prev_val_borrow = new_val_borrow.clone();
                        drop(prev_val_borrow);
                        drop(new_val_borrow);
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
                }
            }
        }
    }

    fn declare_variable(&mut self, name: &'a str, value: Rc<RefCell<Value>>) -> Result<(), ScopeManagerIssue> {
        match self.get_variable(name) {
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
        let name = "x";
        let value = Rc::new(RefCell::new(Value::I64(5)));

        let _ = scope.declare_variable(name, value.clone());
        assert!(scope.get_variable(name).unwrap().clone() == value);
        assert!(scope.get_variable("non-existent").is_none());

        let new_value = Rc::new(RefCell::new(Value::I64(0)));
        let _ = scope.assign_variable(name, new_value.clone());
        assert!(scope.get_variable(name).unwrap().clone() == new_value);

        assert!(scope.assign_variable("y", Rc::new(RefCell::new(Value::Bool(true)))).is_err());
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
    fn manages_variables() {
        // i64 x = 1;
        // {x = 5; i64 y = 2;}
        // {y; i64 y = 3;}

        let mut manager = ScopeManager::new();

        let _ = manager.declare_variable("x", Rc::new(RefCell::new(Value::I64(1))));
        assert!(manager.get_variable("x").unwrap().clone() == Rc::new(RefCell::new(Value::I64(1))));

        manager.push_scope();
        assert!(manager.get_variable("x").unwrap().clone() == Rc::new(RefCell::new(Value::I64(1))));

        let _ = manager.assign_variable("x", Rc::new(RefCell::new(Value::I64(5))));
        assert!(manager.get_variable("x").unwrap().clone() == Rc::new(RefCell::new(Value::I64(5))));

        let _ = manager.declare_variable("y", Rc::new(RefCell::new(Value::I64(2))));
        assert!(manager.get_variable("y").unwrap().clone() == Rc::new(RefCell::new(Value::I64(2))));

        manager.pop_scope();
        assert!(manager.get_variable("x").unwrap().clone() == Rc::new(RefCell::new(Value::I64(5))));
        assert!(manager.get_variable("y").is_err());

        manager.push_scope();
        assert!(manager.get_variable("y").is_err());

        let _ = manager.declare_variable("y", Rc::new(RefCell::new(Value::I64(3))));
        assert!(manager.get_variable("y").unwrap().clone() == Rc::new(RefCell::new(Value::I64(3))));

        manager.pop_scope();
    }

    #[test]
    fn bad_assign_type() {
        let mut manager = ScopeManager::new();

        let _ = manager.declare_variable("x", Rc::new(RefCell::new(Value::I64(1))));
        assert!(manager.assign_variable("x", Rc::new(RefCell::new(Value::Bool(true)))).is_err());
    }

    #[test]
    fn doesnt_allow_redclare() {
        let mut manager = ScopeManager::new();

        let _ = manager.declare_variable("x", Rc::new(RefCell::new(Value::I64(1))));
        assert!(manager.declare_variable("x", Rc::new(RefCell::new(Value::I64(6)))).is_err());
    }
}
