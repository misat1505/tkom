use std::fmt::Debug;

use crate::lazy_stream_reader::Position;

#[derive(Debug, Clone, PartialEq)]
pub struct Node<T> {
    pub value: T,
    pub position: Position,
}

type BNode<T> = Box<Node<T>>;

#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    // Boolean operations
    Alternative(BNode<Expression>, BNode<Expression>),
    Concatenation(BNode<Expression>, BNode<Expression>),
    // Relations
    Greater(BNode<Expression>, BNode<Expression>),
    GreaterEqual(BNode<Expression>, BNode<Expression>),
    Less(BNode<Expression>, BNode<Expression>),
    LessEqual(BNode<Expression>, BNode<Expression>),
    Equal(BNode<Expression>, BNode<Expression>),
    NotEqual(BNode<Expression>, BNode<Expression>),
    // Arithmetic operations
    Addition(BNode<Expression>, BNode<Expression>),
    Subtraction(BNode<Expression>, BNode<Expression>),
    Multiplication(BNode<Expression>, BNode<Expression>),
    Division(BNode<Expression>, BNode<Expression>),
    // Unary operations
    BooleanNegation(BNode<Expression>),
    ArithmeticNegation(BNode<Expression>),
    // Casting
    Casting {
        value: BNode<Expression>,
        to_type: Node<Type>,
    },
    // Values
    Literal(Literal),
    Variable(String),
    FunctionCall {
        identifier: Node<String>,
        arguments: Vec<BNode<Argument>>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    True,
    False,
    String(String),
    I64(i64),
    F64(f64),
}

#[derive(Clone, PartialEq, Copy)]
pub enum Type {
    Bool,
    Str,
    I64,
    F64,
    Void,
}

impl Debug for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::Bool => {
                write!(f, "bool")
            }
            Type::F64 => {
                write!(f, "f64")
            }
            Type::I64 => {
                write!(f, "i64")
            }
            Type::Str => {
                write!(f, "str")
            }
            Type::Void => {
                write!(f, "void")
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum PassedBy {
    Reference,
    Value,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Argument {
    pub value: Node<Expression>,
    pub passed_by: PassedBy,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    FunctionDeclaration {
        identifier: Node<String>,
        parameters: Vec<Node<Parameter>>,
        return_type: Node<Type>,
        block: Node<Block>,
    },
    FunctionCall {
        identifier: Node<String>,
        arguments: Vec<BNode<Argument>>,
    },
    Declaration {
        var_type: Node<Type>,
        identifier: Node<String>,
        value: Option<Node<Expression>>,
    },
    Assignment {
        identifier: Node<String>,
        value: Node<Expression>,
    },
    Conditional {
        condition: Node<Expression>,
        if_block: Node<Block>,
        else_block: Option<Node<Block>>,
    },
    ForLoop {
        declaration: Option<Box<Node<Statement>>>,
        condition: Node<Expression>,
        assignment: Option<Box<Node<Statement>>>,
        block: Node<Block>,
    },
    Switch {
        expressions: Vec<Node<SwitchExpression>>,
        cases: Vec<Node<SwitchCase>>,
    },
    Return(Option<Node<Expression>>),
    Break,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Parameter {
    pub passed_by: PassedBy,
    pub parameter_type: Node<Type>,
    pub identifier: Node<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SwitchExpression {
    pub expression: Node<Expression>,
    pub alias: Option<Node<String>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SwitchCase {
    pub condition: Node<Expression>,
    pub block: Node<Block>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Block(pub Vec<Node<Statement>>);

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub statements: Vec<Node<Statement>>,
}
