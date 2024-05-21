use std::fmt::Debug;

use crate::{
    ast::{Node, Statement},
    scope_manager::{ScopeManager, ScopeManagerIssue},
    value::Value,
};

#[derive(Debug, Clone)]
pub struct Stack(pub Vec<StackFrame>);

#[derive(Clone)]
pub struct StackFrame {
    pub scope_manager: ScopeManager,
    statements: Vec<Node<Statement>>,
}

impl Debug for StackFrame {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "{:?}", self.scope_manager)
  }
}

impl StackFrame {
    pub fn new(statements: Vec<Node<Statement>>) -> Self {
        StackFrame {
            scope_manager: ScopeManager::new(),
            statements,
        }
    }
}

impl Stack {
    pub fn new(statements: Vec<Node<Statement>>) -> Self {
        Stack(vec![StackFrame::new(statements)])
    }

    pub fn push_stack_frame(&mut self, statements: Vec<Node<Statement>>) {
        self.0.push(StackFrame::new(statements));
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

    pub fn declare_variable(
        &mut self,
        name: String,
        value: Value,
    ) -> Result<(), ScopeManagerIssue> {
        if let Some(last_frame) = self.0.last_mut() {
            last_frame.scope_manager.declare_variable(name, value)?;
        }
        Ok(())
    }
}
