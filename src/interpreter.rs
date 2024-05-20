use crate::{ast::{Argument, Block, Expression, Identifier, Literal, Node, Parameter, Program, Statement, SwitchCase, SwitchExpression, Type}, errors::Issue, scope_manager::ScopeManager, value::Value, visitor::Visitor};

pub struct Interpreter {
  program: Program,
  scope_manager: ScopeManager
}

impl Interpreter {
  pub fn new(program: Program) -> Self {
    Interpreter {program, scope_manager: ScopeManager::new()}
  }

  pub fn interpret(&mut self) -> Result<(), Box<dyn Issue>> {
    self.visit_program(&self.program.clone());
    Ok(())
  }
}

impl Visitor for Interpreter {
  fn visit_program(&mut self, program: &Program) {
      for statement in program.statements.clone() {
          self.visit_statement(&statement);
      }
  }

  fn visit_expression(&mut self, expression: &Node<Expression>) {
      match expression.value.clone() {
          Expression::Alternative(lhs, rhs)
          | Expression::Concatenation(lhs, rhs)
          | Expression::Greater(lhs, rhs)
          | Expression::GreaterEqual(lhs, rhs)
          | Expression::Less(lhs, rhs)
          | Expression::LessEqual(lhs, rhs)
          | Expression::Equal(lhs, rhs)
          | Expression::NotEqual(lhs, rhs)
          | Expression::Addition(lhs, rhs)
          | Expression::Subtraction(lhs, rhs)
          | Expression::Multiplication(lhs, rhs)
          | Expression::Division(lhs, rhs) => {
              self.visit_expression(&lhs);
              self.visit_expression(&rhs);
          }
          Expression::BooleanNegation(value)
          | Expression::ArithmeticNegation(value)
          | Expression::Casting { value, .. } => {
              self.visit_expression(&value);
          }
          Expression::Literal(literal) => self.visit_literal(literal),
          Expression::Variable(variable) => self.visit_variable(variable),
          Expression::FunctionCall {
              identifier,
              arguments,
          } => {
              self.visit_identifier(&identifier);
              for arg in arguments {
                  self.visit_argument(&arg);
              }
          }
      }
  }

  fn visit_statement(&mut self, statement: &Node<Statement>) {
      match statement.value.clone() {
          Statement::FunctionDeclaration {
              identifier,
              parameters,
              return_type,
              block,
          } => {
              self.visit_identifier(&identifier);
              for param in parameters {
                  self.visit_parameter(&param)
              }
              self.visit_type(&return_type);
              self.visit_block(&block);
          }
          Statement::FunctionCall {
              identifier,
              arguments,
          } => {
              self.visit_identifier(&identifier);
              for arg in arguments {
                  self.visit_argument(&arg);
              }
          }
          Statement::Declaration {
              var_type,
              identifier,
              value,
          } => {
              self.visit_type(&var_type);
              self.visit_identifier(&identifier);
              if let Some(val) = value {
                  self.visit_expression(&val)
              }
              // get value from expression somehow
              self.scope_manager.declare_variable(identifier.value.0, Value::I64(5)).unwrap();
              println!("{:?}", self.scope_manager.clone());
          }
          Statement::Assignment { identifier, value } => {
              self.visit_identifier(&identifier);
              self.visit_expression(&value);
              self.scope_manager.assign_variable(identifier.value.0, Value::I64(1)).unwrap();
              println!("{:?}", self.scope_manager.clone());
          }
          Statement::Conditional {
              condition,
              if_block,
              else_block,
          } => {
              self.visit_expression(&condition);
              self.visit_block(&if_block);
              if let Some(else_blk) = else_block {
                  self.visit_block(&else_blk);
              }
          }
          Statement::ForLoop {
              declaration,
              condition,
              assignment,
              block,
          } => {
              if let Some(decl) = declaration {
                  self.visit_statement(&decl);
              }
              self.visit_expression(&condition);
              if let Some(assign) = assignment {
                  self.visit_statement(&assign);
              }
              self.visit_block(&block);
          }
          Statement::Switch { expressions, cases } => {
              for expr in expressions {
                  self.visit_switch_expression(&expr);
              }
              for case in cases {
                  self.visit_switch_case(&case);
              }
          }
          Statement::Return(value) => {
              if let Some(val) = value {
                  self.visit_expression(&val);
              }
          }
          Statement::Break => {}
      }
  }

  fn visit_argument(&mut self, argument: &Node<Argument>) {
      self.visit_expression(&argument.value.value);
  }

  fn visit_block(&mut self, block: &Node<Block>) {
      self.scope_manager.push_scope();
      println!("{:?}", self.scope_manager.clone());
      for statement in &block.value.0 {
          self.visit_statement(statement);
      }
      self.scope_manager.pop_scope();
      println!("{:?}", self.scope_manager.clone());
  }

  fn visit_parameter(&mut self, parameter: &Node<Parameter>) {
      self.visit_type(&parameter.value.parameter_type);
      self.visit_identifier(&parameter.value.identifier);
  }

  fn visit_switch_case(&mut self, switch_case: &Node<SwitchCase>) {
      self.visit_expression(&switch_case.value.condition);
      self.visit_block(&switch_case.value.block);
  }

  fn visit_switch_expression(&mut self, switch_expression: &Node<SwitchExpression>) {
      self.visit_expression(&switch_expression.value.expression);
  }

  fn visit_identifier(&mut self, _identifier: &Node<Identifier>) {
      // println!("{:?}", _identifier);
  }

  fn visit_type(&mut self, _node_type: &Node<Type>) {
      // println!("{:?}", _node_type);
  }

  fn visit_literal(&mut self, _literal: Literal) {
      // println!("{:?}", _literal);
  }

  fn visit_variable(&mut self, _variable: Identifier) {
      // println!("{:?}", _variable);
      println!("{:?}", self.scope_manager.get_variable(_variable.0));
  }
}