use crate::{
    ast::{Expression, Identifier, Literal, Node, Type},
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
            if self.lexer.current().clone().unwrap().category == TokenCategory::ETX {
                break;
            }
            match self.parse_casted_term() {
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

    fn parse_casted_term(&mut self) -> Result<Node<Expression>, ParserIssue> {
        let unary_term = self.parse_unary_term()?;
        let position = unary_term.position.clone();
        match self.consume_if(TokenCategory::As) {
            Some(_) => {
                let type_parsed = self.parse_type()?;
                return Ok(Node { value: Expression::Casting { value: Box::new(unary_term), to_type: type_parsed }, position: position });
            }
            None => Ok(unary_term)
        }
    }

    fn parse_unary_term(&mut self) -> Result<Node<Expression>, ParserIssue> {
        if let Some(token) = self.consume_if(TokenCategory::Negate) {
            let factor = self.parse_factor()?;
            return Ok(Node { value: Expression::BooleanNegation(Box::new(factor)), position: token.position });
        }
        if let Some(token) = self.consume_if(TokenCategory::Minus) {
            let factor = self.parse_factor()?;
            return Ok(Node { value: Expression::ArithmeticNegation(Box::new(factor)), position: token.position });
        }
        let factor = self.parse_factor()?;
        Ok(factor)
    }

    fn parse_factor(&mut self) -> Result<Node<Expression>, ParserIssue> {
        // TODO expression
        match self.parse_literal() {
            Ok(result) => {
                let node = Node { value: Expression::Literal(result.value), position: result.position };
                return Ok(node);
            },
            Err(_) => {}
        }
        self.parse_identifier_or_call()
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

    fn parse_type(&mut self) -> Result<Node<Type>, ParserIssue> {
        let token = self.lexer.current().clone().unwrap();
        let result = match token.category {
            TokenCategory::Bool => Type::Bool,
            TokenCategory::String => Type::Str,
            TokenCategory::I64 => Type::I64,
            TokenCategory::F64 => Type::F64,
            _ => {
                return Err(Self::create_parser_error("Can't cast".to_owned()));
            }
        };

        let _ = self.lexer.next();
        Ok(Node { value: result, position: token.position })
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