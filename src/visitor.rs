use crate::{
    ast::{Argument, Block, Expression, Literal, Node, Parameter, Program, Statement, SwitchCase, SwitchExpression, Type},
    errors::Issue,
};

pub trait Visitor<'a> {
    fn visit_program(&mut self, program: &'a Program) -> Result<(), Box<dyn Issue>>;
    fn visit_statement(&mut self, statement: &'a Node<Statement>) -> Result<(), Box<dyn Issue>>;
    fn visit_expression(&mut self, expression: &'a Node<Expression>) -> Result<(), Box<dyn Issue>>;
    fn visit_parameter(&mut self, parameter: &'a Node<Parameter>) -> Result<(), Box<dyn Issue>>;
    fn visit_argument(&mut self, argument: &'a Node<Argument>) -> Result<(), Box<dyn Issue>>;
    fn visit_type(&mut self, node_type: &'a Node<Type>) -> Result<(), Box<dyn Issue>>;
    fn visit_block(&mut self, block: &'a Node<Block>) -> Result<(), Box<dyn Issue>>;
    fn visit_switch_expression(&mut self, switch_expression: &'a Node<SwitchExpression>) -> Result<(), Box<dyn Issue>>;
    fn visit_switch_case(&mut self, switch_case: &'a Node<SwitchCase>) -> Result<(), Box<dyn Issue>>;
    fn visit_literal(&mut self, literal: &'a Literal) -> Result<(), Box<dyn Issue>>;
    fn visit_variable(&mut self, variable: &'a String) -> Result<(), Box<dyn Issue>>;
}
