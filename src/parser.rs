use crate::{
    ast::{Expression, Identifier, Literal, Node},
    lexer::ILexer,
    tokens::{Token, TokenCategory, TokenValue},
};

pub struct Parser<L: ILexer> {
    lexer: L,
}

#[derive(Debug, Clone)]
pub enum ParserIssueKind {
    WARNING,
    ERROR,
}

#[derive(Debug, Clone)]
pub struct ParserIssue {
    pub kind: ParserIssueKind,
    pub message: String,
}

pub trait IParser<L: ILexer> {
    fn new(lexer: L) -> Parser<L>;
    fn parse(&mut self);
}

impl<L: ILexer> IParser<L> for Parser<L> {
    fn new(lexer: L) -> Parser<L> {
        Parser { lexer }
    }

    fn parse(&mut self) {
        let _ = self.lexer.next(); // initialize
        let _ = self.lexer.next(); // skip STX

        loop {
            match self.parse_identifier_or_call() {
                Ok(node) => {
                    println!("{:?}", node);
                }
                Err(err) => {
                    println!("{}", err.message);
                    return;
                }
            }
        }
    }
}

impl<L: ILexer> Parser<L> {
    fn parse_identifier(&mut self) -> Result<Node<Identifier>, ParserIssue> {
        let token = self.consume_match(TokenCategory::Identifier)?;
        if let TokenValue::String(name) = token.value {
            let node = Node {
                value: Identifier(name),
                position: token.position,
            };
            return Ok(node);
        }
        Err(Self::create_parser_error("".to_owned()))
    }

    fn parse_arguments(&mut self) -> Vec<Node<Expression>> {
        vec![]
    }

    fn parse_identifier_or_call(&mut self) -> Result<Node<Expression>, ParserIssue> {
        let identifier = self.parse_identifier()?;
        let position = identifier.position;

        let result = match self.consume_if(TokenCategory::ParenOpen) {
            Some(_) => {
                let args = self.parse_arguments().into_iter().map(Box::new).collect();
                let _ = self.consume_match(TokenCategory::ParenClose)?;
                Expression::FunctionCall {
                    identifier: identifier.value,
                    arguments: args
                }
            },
            None => Expression::Variable(identifier.value)
        };

        Ok(Node { value: result, position: position })
    }

    fn parse_literal(&mut self) -> Result<Node<Literal>, ParserIssue> {
        let token = self.lexer.current().clone().unwrap();
        if self.consume_if(TokenCategory::True).is_some() {
            return Ok(Node {
                value: Literal::True,
                position: token.position,
            });
        } else if self.consume_if(TokenCategory::False).is_some() {
            return Ok(Node {
                value: Literal::False,
                position: token.position,
            });
        } else if self.consume_if(TokenCategory::StringValue).is_some() {
            if let TokenValue::String(string) = token.value {
                return Ok(Node {
                    value: Literal::String(string),
                    position: token.position,
                });
            }
        } else if self.consume_if(TokenCategory::I64Value).is_some() {
            if let TokenValue::I64(int) = token.value {
                return Ok(Node {
                    value: Literal::I64(int),
                    position: token.position,
                });
            }
        } else if self.consume_if(TokenCategory::F64Value).is_some() {
            if let TokenValue::F64(float) = token.value {
                return Ok(Node {
                    value: Literal::F64(float),
                    position: token.position,
                });
            }
        }
        return Err(Self::create_parser_error("Invalid literal".to_owned()));
    }

    fn consume_match(&mut self, desired_category: TokenCategory) -> Result<Token, ParserIssue> {
        let current_token = self.lexer.current().clone().unwrap();
        if current_token.category == desired_category {
            let _ = self.lexer.next();
            return Ok(current_token.clone());
        }
        Err(Self::create_parser_error(format!(
            "Unexpected token - {:?}. Expected {:?}.",
            current_token.category, desired_category
        )))
    }

    fn consume_if(&mut self, desired_category: TokenCategory) -> Option<Token> {
        let current_token = self.lexer.current().clone().unwrap();
        if current_token.category == desired_category {
            let _ = self.lexer.next();
            return Some(current_token.clone());
        }
        None
    }

    fn create_parser_error(text: String) -> ParserIssue {
        ParserIssue {
            kind: ParserIssueKind::ERROR,
            message: text,
        }
    }
}
