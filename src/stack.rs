use std::fmt::Debug;

use crate::{
    errors::Issue,
    scope_manager::{ScopeManager, ScopeManagerIssue},
    value::Value,
};

#[derive(Debug)]
pub struct StackOverflowIssue {
    message: String,
}

impl Issue for StackOverflowIssue {
    fn message(&self) -> String {
        self.message.clone()
    }
}

#[derive(Debug, Clone)]
pub struct Stack(pub Vec<StackFrame>);

#[derive(Clone)]
pub struct StackFrame {
    pub scope_manager: ScopeManager,
}

impl Debug for StackFrame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.scope_manager)
    }
}

impl StackFrame {
    pub fn new() -> Self {
        StackFrame {
            scope_manager: ScopeManager::new(),
        }
    }
}

impl Stack {
    pub fn new() -> Self {
        Stack(vec![StackFrame::new()])
    }

    pub fn push_stack_frame(&mut self) -> Result<(), StackOverflowIssue> {
        if self.0.len() == 50 {
            return Err(StackOverflowIssue {
                message: "Stack overflow.".to_owned(),
            });
        }
        self.0.push(StackFrame::new());
        Ok(())
    }

    pub fn pop_stack_frame(&mut self) {
        self.0.pop();
    }

    pub fn push_scope(&mut self) {
        if let Some(last_frame) = self.0.last_mut() {
            last_frame.scope_manager.push_scope();
        }
    }

    pub fn pop_scope(&mut self) {
        if let Some(last_frame) = self.0.last_mut() {
            last_frame.scope_manager.pop_scope();
        }
    }

    pub fn get_variable(&mut self, name: String) -> Result<&Value, ScopeManagerIssue> {
        if let Some(last_frame) = self.0.last_mut() {
            return last_frame.scope_manager.get_variable(name);
        }
        panic!();
    }

    pub fn assign_variable(&mut self, name: String, value: Value) -> Result<(), ScopeManagerIssue> {
        if let Some(last_frame) = self.0.last_mut() {
            last_frame.scope_manager.assign_variable(name, value)?;
        }
        Ok(())
    }

    pub fn declare_variable(&mut self, name: String, value: Value) -> Result<(), ScopeManagerIssue> {
        if let Some(last_frame) = self.0.last_mut() {
            last_frame.scope_manager.declare_variable(name, value)?;
        }
        Ok(())
    }

    pub fn is_last_scope(&self) -> bool {
        self.0.get(self.0.len() - 1).unwrap().scope_manager.len() == 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::Value;

    #[test]
    fn test_stack_push_pop_frame() {
        let mut stack = Stack::new();

        assert_eq!(stack.0.len(), 1);

        stack.push_stack_frame().unwrap();
        assert_eq!(stack.0.len(), 2);

        stack.pop_stack_frame();
        assert_eq!(stack.0.len(), 1);
    }

    #[test]
    fn test_stack_overflow() {
        let mut stack = Stack::new();

        for _ in 0..49 {
            stack.push_stack_frame().unwrap();
        }

        assert_eq!(stack.0.len(), 50);
        let result = stack.push_stack_frame();
        assert!(result.is_err());
        if let Err(e) = result {
            assert_eq!(e.message(), "Stack overflow.");
        }
    }

    #[test]
    fn test_scope_push_pop() {
        let mut stack = Stack::new();

        stack.push_scope();
        if let Some(last_frame) = stack.0.last() {
            assert_eq!(last_frame.scope_manager.len(), 2);
        }

        stack.pop_scope();
        if let Some(last_frame) = stack.0.last() {
            assert_eq!(last_frame.scope_manager.len(), 1);
        }
    }

    #[test]
    fn test_variable_operations() {
        let mut stack = Stack::new();

        let var_name = "x".to_string();
        let var_value = Value::I64(42);

        stack.declare_variable(var_name.clone(), var_value.clone()).unwrap();
        let retrieved_value = stack.get_variable(var_name.clone()).unwrap();
        assert_eq!(retrieved_value, &var_value);

        let new_value = Value::I64(43);
        stack.assign_variable(var_name.clone(), new_value.clone()).unwrap();
        let updated_value = stack.get_variable(var_name).unwrap();
        assert_eq!(updated_value, &new_value);
    }
}
