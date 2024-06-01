use std::{cell::RefCell, fmt::Debug, rc::Rc};

use crate::{
    issues::{ScopeManagerIssue, StackOverflowIssue},
    scope_manager::ScopeManager,
    value::Value,
};

#[derive(Debug, Clone)]
pub struct Stack<'a>(pub Vec<StackFrame<'a>>);

#[derive(Clone)]
pub struct StackFrame<'a> {
    pub scope_manager: ScopeManager<'a>,
}

impl<'a> Debug for StackFrame<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.scope_manager)
    }
}

impl<'a> StackFrame<'a> {
    pub fn new() -> Self {
        StackFrame {
            scope_manager: ScopeManager::new(),
        }
    }
}

impl<'a> Stack<'a> {
    pub fn new() -> Self {
        Stack(vec![StackFrame::new()])
    }

    pub fn push_stack_frame(&mut self) -> Result<(), StackOverflowIssue> {
        if self.0.len() == 500 {
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

    pub fn get_variable(&mut self, name: &'a str) -> Result<&Rc<RefCell<Value>>, ScopeManagerIssue> {
        if let Some(last_frame) = self.0.last_mut() {
            return last_frame.scope_manager.get_variable(name);
        }
        unreachable!();
    }

    pub fn assign_variable(&mut self, name: &'a str, value: Rc<RefCell<Value>>) -> Result<(), ScopeManagerIssue> {
        if let Some(last_frame) = self.0.last_mut() {
            last_frame.scope_manager.assign_variable(name, value)?;
        }
        Ok(())
    }

    pub fn declare_variable(&mut self, name: &'a str, value: Rc<RefCell<Value>>) -> Result<(), ScopeManagerIssue> {
        if let Some(last_frame) = self.0.last_mut() {
            last_frame.scope_manager.declare_variable(name, value)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::issues::Issue;
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

        for _ in 0..499 {
            stack.push_stack_frame().unwrap();
        }

        assert_eq!(stack.0.len(), 500);
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

        let var_name = "x";
        let var_value = Rc::new(RefCell::new(Value::I64(42)));

        stack.declare_variable(var_name, var_value.clone()).unwrap();
        let retrieved_value = stack.get_variable(var_name).unwrap();
        assert_eq!(retrieved_value, &var_value);

        let new_value = Rc::new(RefCell::new(Value::I64(43)));
        stack.assign_variable(var_name, new_value.clone()).unwrap();
        let updated_value = stack.get_variable(var_name).unwrap();
        assert_eq!(updated_value, &new_value);
    }
}
