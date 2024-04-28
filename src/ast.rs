use crate::lazy_stream_reader::Position;

#[derive(Debug, Clone)]
pub struct Node<T> {
    pub value: T,
    pub position: Position,
}

type BNode<T> = Box<Node<T>>;

#[derive(Debug, Clone)]
pub enum Expression {
    // Boolean operations (non-unary)
    Alternative(BNode<Expression>, BNode<Expression>),
    Concatenation(BNode<Expression>, BNode<Expression>),
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
        arguments: Vec<BNode<Argument>>,
    },
}

#[derive(Debug, Clone)]
pub enum Literal {
    True,
    False,
    String(String),
    I64(i64),
    F64(f64),
}

#[derive(Debug, Clone)]
pub enum Type {
    Bool,
    Str,
    I64,
    F64,
}

#[derive(Debug, Clone)]
pub struct Identifier(pub String);

#[derive(Debug, Clone)]
pub enum ArgumentPassedBy {
    Reference,
    Value,
}

#[derive(Debug, Clone)]
pub struct Argument {
    pub value: Expression,
    pub passed_by: ArgumentPassedBy,
}

#[derive(Debug, Clone)]
pub enum Statement {
    // TODO switch
    FunctionCall {
        identifier: Node<Identifier>,
        arguments: Vec<BNode<Argument>>,
    },
    Declaration {
        var_type: Node<Type>,
        identifier: Node<Identifier>,
        value: Option<Node<Expression>>,
    },
    Assignment {
        identifier: Node<Identifier>,
        value: Node<Expression>,
    },
    Conditional {
        condition: Node<Expression>,
        if_block: Node<Block>,
        else_block: Option<Node<Block>>,
    },
    ForLoop {
        declaration: Option<Node<Box<Statement>>>,
        condition: Node<Expression>,
        assignment: Option<Node<Box<Statement>>>,
        block: Node<Block>,
    },
    Return(Option<Node<Expression>>),
    Block(Node<Block>),
    Break,
}

#[derive(Debug, Clone)]
pub struct Block(pub Vec<Node<Statement>>);
