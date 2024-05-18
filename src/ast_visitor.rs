use crate::ast::{Expression, Node, Program, Statement};

pub trait AstVisitor {
    fn visit_program(&mut self, program: &Program);
    fn visit_statement(&mut self, statement: &Node<Statement>);
    fn visit_expression(&mut self, expression: &Node<Expression>);
}
