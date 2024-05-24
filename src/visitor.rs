use crate::{
    ast::{Argument, Block, Expression, Literal, Node, Parameter, Program, Statement, SwitchCase, SwitchExpression, Type},
    errors::Issue,
};

pub trait Visitor {
    fn visit_program(&mut self, program: &Program) -> Result<(), Box<dyn Issue>>;
    fn visit_statement(&mut self, statement: &Node<Statement>) -> Result<(), Box<dyn Issue>>;
    fn visit_expression(&mut self, expression: &Node<Expression>) -> Result<(), Box<dyn Issue>>;
    fn visit_parameter(&mut self, parameter: &Node<Parameter>) -> Result<(), Box<dyn Issue>>;
    fn visit_argument(&mut self, argument: &Node<Argument>) -> Result<(), Box<dyn Issue>>;
    fn visit_type(&mut self, node_type: &Node<Type>) -> Result<(), Box<dyn Issue>>;
    fn visit_block(&mut self, block: &Node<Block>) -> Result<(), Box<dyn Issue>>;
    fn visit_switch_expression(&mut self, switch_expression: &Node<SwitchExpression>) -> Result<(), Box<dyn Issue>>;
    fn visit_switch_case(&mut self, switch_case: &Node<SwitchCase>) -> Result<(), Box<dyn Issue>>;
    fn visit_literal(&mut self, literal: Literal) -> Result<(), Box<dyn Issue>>;
    fn visit_variable(&mut self, variable: String) -> Result<(), Box<dyn Issue>>;
}
