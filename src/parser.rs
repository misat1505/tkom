use crate::{
    ast::{Identifier, Literal, Node},
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
            match self.parse_literal() {
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

    fn parse_literal(&mut self) -> Result<Node<Literal>, ParserIssue> {
        let token = self.lexer.current().clone().unwrap();
        match token.category {
            TokenCategory::True => {
                let _ = self.lexer.next();
                return Ok(Node {
                    value: Literal::True,
                    position: token.position.clone(),
                });
            }
            TokenCategory::False => {
                let _ = self.lexer.next();
                return Ok(Node {
                    value: Literal::False,
                    position: token.position.clone(),
                });
            }
            TokenCategory::StringValue => {
                let _ = self.lexer.next();
                if let TokenValue::String(string) = token.value {
                    return Ok(Node {
                        value: Literal::String(string),
                        position: token.position.clone(),
                    });
                }
            }
            TokenCategory::F64Value => {
                let _ = self.lexer.next();
                if let TokenValue::F64(f64_value) = token.value {
                    return Ok(Node {
                        value: Literal::F64(f64_value),
                        position: token.position.clone(),
                    });
                }
            }
            TokenCategory::I64Value => {
                let _ = self.lexer.next();
                if let TokenValue::I64(i64_value) = token.value {
                    return Ok(Node {
                        value: Literal::I64(i64_value),
                        position: token.position.clone(),
                    });
                }
            }
            _ => return Err(Self::create_parser_error("Invalid literal".to_owned())),
        }

        return Err(Self::create_parser_error("Invalid token type".to_owned()));
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

    fn create_parser_error(text: String) -> ParserIssue {
        ParserIssue {
            kind: ParserIssueKind::ERROR,
            message: text,
        }
    }
}
