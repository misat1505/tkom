use crate::{
    ast::{
        Argument, ArgumentPassedBy, Block, Expression, Identifier, Literal, Node, Statement, Type,
    },
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
        let _ = self.next_token(); // initialize
        let _ = self.next_token(); // skip STX

        loop {
            if self.lexer.current().clone().unwrap().category == TokenCategory::ETX {
                break;
            }
            match self.parse_for_statement() {
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
    fn next_token(&mut self) -> Option<Token> {
        let mut current_token = self.lexer.next().clone().unwrap();
        while current_token.category == TokenCategory::Comment {
            current_token = self.lexer.next().unwrap();
        }
        Some(current_token)
    }

    fn parse_for_statement(&mut self) -> Result<Node<Statement>, ParserIssue> {
        let for_token = self.consume_must(TokenCategory::For)?;
        let _ = self.consume_must(TokenCategory::ParenOpen)?;
        let declaration = match self.parse_declaration() {
            Ok(decl) => {
                let position = decl.position;
                let node = Node { value: Box::new(decl.value), position };
                Some(node)
            },
            Err(_) => None
        };
        self.consume_must(TokenCategory::Semicolon)?;
        let condition = self.parse_expression()?;
        self.consume_must(TokenCategory::Semicolon)?;
        let mut assignment: Option<Node<Box<Statement>>> = None;
        if self.lexer.current().clone().unwrap().category == TokenCategory::Identifier {
            let identifier = self.parse_identifier()?;
            let position = identifier.position;
            let _ = self.consume_must(TokenCategory::Assign)?;
            let expr = self.parse_expression()?;
            let assign = Node { value: Box::new(Statement::Assignment { identifier, value: expr }), position };
            assignment = Some(assign);
        };
        self.consume_must(TokenCategory::ParenClose)?;
        let block = self.parse_statement_block()?;
        let node = Node { value: Statement::ForLoop { declaration, condition, assignment, block }, position: for_token.position };
        Ok(node)
    }

    fn parse_if_statement(&mut self) -> Result<Node<Statement>, ParserIssue> {
        let if_token = self.consume_must(TokenCategory::If)?;
        let _ = self.consume_must(TokenCategory::ParenOpen)?;
        let condition = self.parse_expression()?;
        let _ = self.consume_must(TokenCategory::ParenClose)?;
        let true_block = self.parse_statement_block()?;

        let false_block = match self.consume_if(TokenCategory::Else) {
            Some(_) => Some(self.parse_statement_block()?),
            None => None,
        };

        let node = Node {
            value: Statement::Conditional {
                condition,
                if_block: true_block,
                else_block: false_block,
            },
            position: if_token.position,
        };
        Ok(node)
    }

    fn parse_statement_block(&mut self) -> Result<Node<Block>, ParserIssue> {
        let position = self.consume_must(TokenCategory::BraceOpen)?.position;
        let mut statements: Vec<Node<Statement>> = vec![];
        while self.consume_if(TokenCategory::BraceClose).is_none() {
            let statement = self.parse_statement()?;
            statements.push(statement);
        }
        Ok(Node {
            value: Block(statements),
            position,
        })
    }

    fn parse_statement(&mut self) -> Result<Node<Statement>, ParserIssue> {
        // TODO better error handling
        let node = self
            .parse_assign_or_call()
            .or_else(|_| {
                let decl = self.parse_declaration()?;
                self.consume_must(TokenCategory::Semicolon)?;
                Ok(decl)
            })
            .or_else(|_: ParserIssue| self.parse_return_statement())
            .or_else(|_| self.parse_break_statement())?;

        Ok(node)
    }

    fn parse_assign_or_call(&mut self) -> Result<Node<Statement>, ParserIssue> {
        let identifier = self.parse_identifier()?;
        let position = identifier.position;

        if self.consume_if(TokenCategory::Assign).is_some() {
            let expr = self.parse_expression()?;
            let node = Node {
                value: Statement::Assignment {
                    identifier,
                    value: expr,
                },
                position,
            };
            self.consume_must(TokenCategory::Semicolon)?;
            return Ok(node);
        }

        if self.consume_if(TokenCategory::ParenOpen).is_some() {
            let arguments = self.parse_arguments()?.into_iter().map(Box::new).collect();
            let node = Node {
                value: Statement::FunctionCall {
                    identifier,
                    arguments,
                },
                position,
            };
            self.consume_must(TokenCategory::ParenClose)?;
            self.consume_must(TokenCategory::Semicolon)?;
            return Ok(node);
        }

        Err(Self::create_parser_error(format!(
            "Could not create assignment or call."
        )))
    }

    fn parse_declaration(&mut self) -> Result<Node<Statement>, ParserIssue> {
        let declaration_type = self.parse_type()?;
        let position = declaration_type.position;
        let identifier = self.parse_identifier()?;
        let value = match self.consume_if(TokenCategory::Assign) {
            Some(_) => Some(self.parse_expression()?),
            None => None,
        };
        let node = Node {
            value: Statement::Declaration {
                var_type: declaration_type,
                identifier,
                value,
            },
            position: position,
        };
        Ok(node)
    }

    fn parse_return_statement(&mut self) -> Result<Node<Statement>, ParserIssue> {
        let token = self.consume_must(TokenCategory::Return)?;
        let returned_value = match self.parse_expression() {
            Ok(expr) => Some(expr),
            Err(_) => None,
        };
        self.consume_must(TokenCategory::Semicolon)?;
        let node = Node {
            value: Statement::Return(returned_value),
            position: token.position,
        };
        Ok(node)
    }

    fn parse_break_statement(&mut self) -> Result<Node<Statement>, ParserIssue> {
        let token = self.consume_must(TokenCategory::Break)?;
        let _ = self.consume_must(TokenCategory::Semicolon)?;
        let node = Node {
            value: Statement::Break,
            position: token.position,
        };
        Ok(node)
    }

    fn parse_arguments(&mut self) -> Result<Vec<Node<Argument>>, ParserIssue> {
        if self.consume_if(TokenCategory::ParenClose).is_some() {
            return Ok(Vec::new());
        }

        let expression = self.parse_argument()?;

        let mut arguments = vec![expression];
        while let Some(_) = self.consume_if(TokenCategory::Comma) {
            let argument = self.parse_argument()?;
            arguments.push(argument);
        }
        Ok(arguments)
    }

    fn parse_argument(&mut self) -> Result<Node<Argument>, ParserIssue> {
        let mut passed_by = ArgumentPassedBy::Value;
        if self.consume_if(TokenCategory::Reference).is_some() {
            passed_by = ArgumentPassedBy::Reference;
        }
        let expression = self.parse_expression()?;
        let argument = Argument {
            value: expression.value,
            passed_by: passed_by,
        };
        Ok(Node {
            value: argument,
            position: expression.position,
        })
    }

    fn parse_expression(&mut self) -> Result<Node<Expression>, ParserIssue> {
        let left_side = self.parse_concatenation_term()?;
        let position = left_side.position;
        if self.consume_if(TokenCategory::Or).is_some() {
            let right_side = self.parse_concatenation_term()?;
            return Ok(Node {
                value: Expression::Alternative(Box::new(left_side), Box::new(right_side)),
                position,
            });
        }
        Ok(left_side)
    }

    fn parse_concatenation_term(&mut self) -> Result<Node<Expression>, ParserIssue> {
        let left_side = self.parse_relation_term()?;
        let position = left_side.position;
        if self.consume_if(TokenCategory::And).is_some() {
            let right_side = self.parse_relation_term()?;
            return Ok(Node {
                value: Expression::Concatenation(Box::new(left_side), Box::new(right_side)),
                position,
            });
        }
        Ok(left_side)
    }

    fn parse_relation_term(&mut self) -> Result<Node<Expression>, ParserIssue> {
        let left_side = self.parse_additive_term()?;
        if let Some(token) = self.consume_if(TokenCategory::Equal) {
            let right_side = self.parse_additive_term()?;
            return Ok(Node {
                value: Expression::Equal(Box::new(left_side), Box::new(right_side)),
                position: token.position,
            });
        }
        if let Some(token) = self.consume_if(TokenCategory::NotEqual) {
            let right_side = self.parse_additive_term()?;
            return Ok(Node {
                value: Expression::NotEqual(Box::new(left_side), Box::new(right_side)),
                position: token.position,
            });
        }
        if let Some(token) = self.consume_if(TokenCategory::Greater) {
            let right_side = self.parse_additive_term()?;
            return Ok(Node {
                value: Expression::Greater(Box::new(left_side), Box::new(right_side)),
                position: token.position,
            });
        }
        if let Some(token) = self.consume_if(TokenCategory::GreaterOrEqual) {
            let right_side = self.parse_additive_term()?;
            return Ok(Node {
                value: Expression::GreaterEqual(Box::new(left_side), Box::new(right_side)),
                position: token.position,
            });
        }
        if let Some(token) = self.consume_if(TokenCategory::Less) {
            let right_side = self.parse_additive_term()?;
            return Ok(Node {
                value: Expression::Less(Box::new(left_side), Box::new(right_side)),
                position: token.position,
            });
        }
        if let Some(token) = self.consume_if(TokenCategory::LessOrEqual) {
            let right_side = self.parse_additive_term()?;
            return Ok(Node {
                value: Expression::LessEqual(Box::new(left_side), Box::new(right_side)),
                position: token.position,
            });
        }
        Ok(left_side)
    }

    fn parse_additive_term(&mut self) -> Result<Node<Expression>, ParserIssue> {
        let mut left_side = self.parse_multiplicative_term()?;
        let mut current_token = self.lexer.current().clone().unwrap();
        while current_token.category == TokenCategory::Plus
            || current_token.category == TokenCategory::Minus
        {
            let _ = self.next_token();
            let right_side = self.parse_multiplicative_term()?;
            let mut expression_type =
                Expression::Addition(Box::new(left_side.clone()), Box::new(right_side.clone()));
            if current_token.category == TokenCategory::Minus {
                expression_type = Expression::Subtraction(Box::new(left_side), Box::new(right_side))
            }
            left_side = Node {
                value: expression_type,
                position: current_token.position,
            };
            current_token = self.lexer.current().clone().unwrap();
        }
        Ok(left_side)
    }

    fn parse_multiplicative_term(&mut self) -> Result<Node<Expression>, ParserIssue> {
        let mut left_side = self.parse_casted_term()?;
        let mut current_token = self.lexer.current().clone().unwrap();
        while current_token.category == TokenCategory::Multiply
            || current_token.category == TokenCategory::Divide
        {
            let _ = self.next_token();
            let right_side = self.parse_casted_term()?;
            let mut expression_type = Expression::Multiplication(
                Box::new(left_side.clone()),
                Box::new(right_side.clone()),
            );
            if current_token.category == TokenCategory::Divide {
                expression_type = Expression::Division(Box::new(left_side), Box::new(right_side))
            }
            left_side = Node {
                value: expression_type,
                position: current_token.position,
            };
            current_token = self.lexer.current().clone().unwrap();
        }
        Ok(left_side)
    }

    fn parse_casted_term(&mut self) -> Result<Node<Expression>, ParserIssue> {
        let unary_term = self.parse_unary_term()?;
        let position = unary_term.position.clone();
        match self.consume_if(TokenCategory::As) {
            Some(_) => {
                let type_parsed = self.parse_type()?;
                return Ok(Node {
                    value: Expression::Casting {
                        value: Box::new(unary_term),
                        to_type: type_parsed,
                    },
                    position: position,
                });
            }
            None => Ok(unary_term),
        }
    }

    fn parse_unary_term(&mut self) -> Result<Node<Expression>, ParserIssue> {
        if let Some(token) = self.consume_if(TokenCategory::Negate) {
            let factor = self.parse_factor()?;
            return Ok(Node {
                value: Expression::BooleanNegation(Box::new(factor)),
                position: token.position,
            });
        }
        if let Some(token) = self.consume_if(TokenCategory::Minus) {
            let factor = self.parse_factor()?;
            return Ok(Node {
                value: Expression::ArithmeticNegation(Box::new(factor)),
                position: token.position,
            });
        }
        let factor = self.parse_factor()?;
        Ok(factor)
    }

    fn parse_factor(&mut self) -> Result<Node<Expression>, ParserIssue> {
        match self.parse_literal() {
            Ok(result) => {
                let node = Node {
                    value: Expression::Literal(result.value),
                    position: result.position,
                };
                return Ok(node);
            }
            Err(_) => {}
        }
        if self.consume_if(TokenCategory::ParenOpen).is_some() {
            let expression = self.parse_expression()?;
            self.consume_must(TokenCategory::ParenClose)?;
            return Ok(expression);
        }
        self.parse_identifier_or_call()
    }

    fn parse_identifier_or_call(&mut self) -> Result<Node<Expression>, ParserIssue> {
        let identifier = self.parse_identifier()?;
        let position = identifier.position;

        let result = match self.consume_if(TokenCategory::ParenOpen) {
            Some(_) => {
                let args = self.parse_arguments()?.into_iter().map(Box::new).collect();
                let _ = self.consume_must(TokenCategory::ParenClose)?;
                Expression::FunctionCall {
                    identifier: identifier.value,
                    arguments: args,
                }
            }
            None => Expression::Variable(identifier.value),
        };
        Ok(Node {
            value: result,
            position: position,
        })
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

        let _ = self.next_token();
        Ok(Node {
            value: result,
            position: token.position,
        })
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

    fn parse_identifier(&mut self) -> Result<Node<Identifier>, ParserIssue> {
        let token = self.consume_must(TokenCategory::Identifier)?;
        if let TokenValue::String(name) = token.value {
            let node = Node {
                value: Identifier(name),
                position: token.position,
            };
            return Ok(node);
        }
        Err(Self::create_parser_error("".to_owned()))
    }

    fn consume_must(&mut self, desired_category: TokenCategory) -> Result<Token, ParserIssue> {
        let current_token = self.lexer.current().clone().unwrap();
        if current_token.category == desired_category {
            let _ = self.next_token();
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
            let _ = self.next_token();
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
