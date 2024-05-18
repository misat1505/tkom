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

impl Node<Identifier> {
    pub fn accept<V: Visitor>(&self, visitor: &mut V) {
        visitor.visit_identifier(self);
    }
}

impl Node<Parameter> {
    pub fn accept<V: Visitor>(&self, visitor: &mut V) {
        visitor.visit_parameter(self);
    }
}

impl Node<Argument> {
    pub fn accept<V: Visitor>(&self, visitor: &mut V) {
        visitor.visit_argument(self);
    }
}

impl Node<Type> {
    pub fn accept<V: Visitor>(&self, visitor: &mut V) {
        visitor.visit_type(self);
    }
}

impl Node<Block> {
    pub fn accept<V: Visitor>(&self, visitor: &mut V) {
        visitor.visit_block(self);
    }
}

impl Node<SwitchExpression> {
    pub fn accept<V: Visitor>(&self, visitor: &mut V) {
        visitor.visit_switch_expression(self);
    }
}

impl Node<SwitchCase> {
    pub fn accept<V: Visitor>(&self, visitor: &mut V) {
        visitor.visit_switch_case(self);
    }
}

impl Literal {
    pub fn accept<V: Visitor>(&self, visitor: &mut V) {
        visitor.visit_literal(self.clone());
    }
}

impl Identifier {
    pub fn accept<V: Visitor>(&self, visitor: &mut V) {
        visitor.visit_variable(self.clone());
    }
}

impl Node<Expression> {
    pub fn accept<V: Visitor>(&self, visitor: &mut V) {
        match self.value.clone() {
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
                visitor.visit_expression(&lhs);
                visitor.visit_expression(&rhs);
            }
            Expression::BooleanNegation(value)
            | Expression::ArithmeticNegation(value)
            | Expression::Casting { value, .. } => {
                visitor.visit_expression(&value);
            }
            Expression::Literal(literal) => visitor.visit_literal(literal),
            Expression::Variable(variable) => visitor.visit_variable(variable),
            Expression::FunctionCall {..} => {}
        }
    }
}

impl Node<Statement> {
    pub fn accept<V: Visitor>(&self, visitor: &mut V) {
        visitor.visit_statement(self);
    }
}
