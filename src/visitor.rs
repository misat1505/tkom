use crate::ast::{
    Argument, Block, Expression, Identifier, Literal, Node, Parameter, Program, Statement,
    SwitchCase, SwitchExpression, Type,
};

pub trait Visitor {
    fn visit_program(&mut self, program: &Program);
    fn visit_statement(&mut self, statement: &Node<Statement>);
    fn visit_expression(&mut self, expression: &Node<Expression>);
    fn visit_identifier(&mut self, identifier: &Node<Identifier>);
    fn visit_parameter(&mut self, parameter: &Node<Parameter>);
    fn visit_argument(&mut self, argument: &Node<Argument>);
    fn visit_type(&mut self, node_type: &Node<Type>);
    fn visit_block(&mut self, block: &Node<Block>);
    fn visit_switch_expression(&mut self, switch_expression: &Node<SwitchExpression>);
    fn visit_switch_case(&mut self, switch_case: &Node<SwitchCase>);
    fn visit_literal(&mut self, literal: Literal);
    fn visit_variable(&mut self, variable: Identifier);
}
