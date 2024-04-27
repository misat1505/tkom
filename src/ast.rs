use crate::lazy_stream_reader::Position;

#[derive(Debug)]
pub struct Node<T> {
    pub value: T,
    pub position: Position,
}

type BNode<T> = Box<Node<T>>;

#[derive(Debug)]
pub enum Expression {
    // Boolean operations (non-unary)
    Alternative(BNode<Expression>, BNode<Expression>),
    Conjunction(BNode<Expression>, BNode<Expression>),
    // Reloations
    Greater(BNode<Expression>, BNode<Expression>),
    GreaterEqual(BNode<Expression>, BNode<Expression>),
    Less(BNode<Expression>, BNode<Expression>),
    LessEqual(BNode<Expression>, BNode<Expression>),
    Equal(BNode<Expression>, BNode<Expression>),
    NotEqual(BNode<Expression>, BNode<Expression>),
    // Arithmetic operations (non-unary)
    Addition(BNode<Expression>, BNode<Expression>),
    Subtraction(BNode<Expression>, BNode<Expression>),
    Multiplication(BNode<Expression>, BNode<Expression>),
    Division(BNode<Expression>, BNode<Expression>),
    // Unary operations
    BooleanNegation(BNode<Expression>),
    ArithmeticNegation(BNode<Expression>),
    // Casting, conversion
    Casting {
        value: BNode<Expression>,
        to_type: Node<Type>,
    },
    // Values
    Literal(Literal),
    Variable(Identifier),
    FunctionCall {
        identifier: Identifier,
        arguments: Vec<BNode<Expression>>,
    },
}

#[derive(Debug)]
pub enum Literal {
    True,
    False,
    String(String),
    I64(i32),
    F64(f64),
}

#[derive(Debug)]
pub enum Type {
    Bool,
    Str,
    I64,
    F64,
}

#[derive(Debug)]
pub struct Identifier(pub String);
