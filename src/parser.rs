use crate::{
    ast::{
        Argument, ArgumentPassedBy, Block, Expression, Identifier, Literal, Node, Parameter,
        ParameterPassedBy, Program, Statement, SwitchCase, SwitchExpression, Type,
    },
    errors::{Issue, IssueLevel, ParserIssue},
    lexer::ILexer,
    tokens::{Token, TokenCategory, TokenValue},
};

pub struct Parser<L: ILexer> {
    lexer: L,
}

pub trait IParser<L: ILexer> {
    fn new(lexer: L) -> Parser<L>;
    fn parse(&mut self) -> Result<Program, Box<dyn Issue>>;
}

impl<L: ILexer> IParser<L> for Parser<L> {
    fn new(lexer: L) -> Parser<L> {
        Parser { lexer }
    }

    fn parse(&mut self) -> Result<Program, Box<dyn Issue>> {
        let _ = self.next_token()?; // initialize
        let _ = self.next_token()?; // skip STX

        let mut statements: Vec<Node<Statement>> = vec![];

        loop {
            if self.current_token().category == TokenCategory::ETX {
                break;
            }

            let node = match self.current_token().category {
                TokenCategory::Identifier => self.parse_assign_or_call()?,
                TokenCategory::Fn => self.parse_function_declaration()?,
                TokenCategory::If => self.parse_if_statement()?,
                TokenCategory::For => self.parse_for_statement()?,
                TokenCategory::Switch => self.parse_switch_statement()?,
                category
                    if category == TokenCategory::I64
                        || category == TokenCategory::F64
                        || category == TokenCategory::Bool
                        || category == TokenCategory::String =>
                {
                    let decl = self.parse_declaration()?;
                    self.consume_must_be(TokenCategory::Semicolon)?;
                    decl
                }
                _ => {
                    return Err(self.create_parser_error(format!(
                        "Can't create program statement starting with token: {:?}.",
                        self.current_token().category
                    )));
                }
            };

            statements.push(node);
        }

        let program = Program { statements };
        Ok(program)
    }
}

impl<L: ILexer> Parser<L> {
    fn next_token(&mut self) -> Result<Option<Token>, Box<dyn Issue>> {
        let mut current_token = self.lexer.next()?.clone();
        while current_token.category == TokenCategory::Comment {
            current_token = self.lexer.next()?.clone();
        }
        Ok(Some(current_token))
    }

    fn current_token(&self) -> Token {
        self.lexer.current().clone().unwrap()
    }

    fn parse_function_declaration(&mut self) -> Result<Node<Statement>, Box<dyn Issue>> {
        let fn_token = self.consume_must_be(TokenCategory::Fn)?;
        let identifier = self.parse_identifier()?;
        let _ = self.consume_must_be(TokenCategory::ParenOpen)?;
        let parameters = self.parse_parameters()?;
        let _ = self.consume_must_be(TokenCategory::ParenClose)?;
        let _ = self.consume_must_be(TokenCategory::Colon)?;
        let return_type = match self.parse_type() {
            Ok(node) => Ok(node),
            Err(_) => match self.consume_if_matches(TokenCategory::Void)? {
                Some(token) => Ok(Node {
                    value: Type::Void,
                    position: token.position,
                }),
                None => Err(self.create_parser_error(format!(
                    "Bad return type: {:?}. Expected one of: 'i64', 'f64', 'bool', 'str', 'void'.",
                    self.current_token().category
                ))),
            },
        }?;
        let block = self.parse_statement_block()?;
        let node = Node {
            value: Statement::FunctionDeclaration {
                identifier,
                parameters,
                return_type,
                block,
            },
            position: fn_token.position,
        };
        Ok(node)
    }

    fn parse_parameters(&mut self) -> Result<Vec<Node<Parameter>>, Box<dyn Issue>> {
        if self.current_token().category == TokenCategory::ParenClose {
            return Ok(vec![]);
        }

        let expression = self.parse_parameter()?;

        let mut parameters = vec![expression];
        while let Some(_) = self.consume_if_matches(TokenCategory::Comma)? {
            let parameter = self.parse_parameter()?;
            parameters.push(parameter);
        }
        Ok(parameters)
    }

    fn parse_parameter(&mut self) -> Result<Node<Parameter>, Box<dyn Issue>> {
        let position = self.current_token().position;
        let passed_by = match self.consume_if_matches(TokenCategory::Reference)? {
            Some(_) => ParameterPassedBy::Reference,
            None => ParameterPassedBy::Value,
        };
        let parameter_type = self.parse_type()?;
        let identifier = self.parse_identifier()?;
        let value = match self.consume_if_matches(TokenCategory::Assign)? {
            Some(_) => Some(self.parse_expression()?),
            None => None,
        };
        let node = Node {
            value: Parameter {
                passed_by,
                parameter_type,
                identifier,
                value,
            },
            position,
        };
        Ok(node)
    }

    fn parse_for_statement(&mut self) -> Result<Node<Statement>, Box<dyn Issue>> {
        let for_token = self.consume_must_be(TokenCategory::For)?;
        let _ = self.consume_must_be(TokenCategory::ParenOpen)?;
        let declaration = match self.parse_declaration() {
            Ok(decl) => {
                let position = decl.position;
                let node = Node {
                    value: Box::new(decl.value),
                    position,
                };
                Some(node)
            }
            Err(_) => None,
        };
        self.consume_must_be(TokenCategory::Semicolon)?;
        let condition = self.parse_expression()?;
        self.consume_must_be(TokenCategory::Semicolon)?;
        let mut assignment: Option<Node<Box<Statement>>> = None;
        if self.current_token().category == TokenCategory::Identifier {
            let identifier = self.parse_identifier()?;
            let position = identifier.position;
            let _ = self.consume_must_be(TokenCategory::Assign)?;
            let expr = self.parse_expression()?;
            let assign = Node {
                value: Box::new(Statement::Assignment {
                    identifier,
                    value: expr,
                }),
                position,
            };
            assignment = Some(assign);
        };
        self.consume_must_be(TokenCategory::ParenClose)?;
        let block = self.parse_statement_block()?;
        let node = Node {
            value: Statement::ForLoop {
                declaration,
                condition,
                assignment,
                block,
            },
            position: for_token.position,
        };
        Ok(node)
    }

    fn parse_if_statement(&mut self) -> Result<Node<Statement>, Box<dyn Issue>> {
        let if_token = self.consume_must_be(TokenCategory::If)?;
        let _ = self.consume_must_be(TokenCategory::ParenOpen)?;
        let condition = self.parse_expression()?;
        let _ = self.consume_must_be(TokenCategory::ParenClose)?;
        let true_block = self.parse_statement_block()?;

        let false_block = match self.consume_if_matches(TokenCategory::Else)? {
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

    fn parse_statement_block(&mut self) -> Result<Node<Block>, Box<dyn Issue>> {
        let position = self.consume_must_be(TokenCategory::BraceOpen)?.position;
        let mut statements: Vec<Node<Statement>> = vec![];
        while self
            .consume_if_matches(TokenCategory::BraceClose)?
            .is_none()
        {
            let statement = self.parse_statement()?;
            statements.push(statement);
        }
        Ok(Node {
            value: Block(statements),
            position,
        })
    }

    fn parse_statement(&mut self) -> Result<Node<Statement>, Box<dyn Issue>> {
        let node = match self.current_token().category {
            TokenCategory::Identifier => self.parse_assign_or_call()?,
            TokenCategory::If => self.parse_if_statement()?,
            TokenCategory::For => self.parse_for_statement()?,
            TokenCategory::Switch => self.parse_switch_statement()?,
            TokenCategory::Return => self.parse_return_statement()?,
            TokenCategory::Break => self.parse_break_statement()?,
            category
                if category == TokenCategory::I64
                    || category == TokenCategory::F64
                    || category == TokenCategory::Bool
                    || category == TokenCategory::String =>
            {
                let decl = self.parse_declaration()?;
                self.consume_must_be(TokenCategory::Semicolon)?;
                decl
            }
            _ => {
                return Err(self.create_parser_error(format!(
                    "Can't create block statement starting with token: {:?}.",
                    self.current_token().category
                )));
            }
        };

        Ok(node)
    }

    fn parse_assign_or_call(&mut self) -> Result<Node<Statement>, Box<dyn Issue>> {
        let identifier = self.parse_identifier()?;
        let position = identifier.position;

        if self.consume_if_matches(TokenCategory::Assign)?.is_some() {
            let expr = self.parse_expression()?;
            let node = Node {
                value: Statement::Assignment {
                    identifier,
                    value: expr,
                },
                position,
            };
            self.consume_must_be(TokenCategory::Semicolon)?;
            return Ok(node);
        }

        if self.consume_if_matches(TokenCategory::ParenOpen)?.is_some() {
            let arguments = self.parse_arguments()?.into_iter().map(Box::new).collect();
            let node = Node {
                value: Statement::FunctionCall {
                    identifier,
                    arguments,
                },
                position,
            };
            self.consume_must_be(TokenCategory::ParenClose)?;
            self.consume_must_be(TokenCategory::Semicolon)?;
            return Ok(node);
        }

        Err(self.create_parser_error(format!("Could not create assignment or call.")))
    }

    fn parse_declaration(&mut self) -> Result<Node<Statement>, Box<dyn Issue>> {
        let declaration_type = self.parse_type()?;
        let position = declaration_type.position;
        let identifier = self.parse_identifier()?;
        let value = match self.consume_if_matches(TokenCategory::Assign)? {
            Some(_) => Some(self.parse_expression()?),
            None => None,
        };
        let node = Node {
            value: Statement::Declaration {
                var_type: declaration_type,
                identifier,
                value,
            },
            position,
        };
        Ok(node)
    }

    fn parse_return_statement(&mut self) -> Result<Node<Statement>, Box<dyn Issue>> {
        let token = self.consume_must_be(TokenCategory::Return)?;
        let returned_value = match self.parse_expression() {
            Ok(expr) => Some(expr),
            Err(_) => None,
        };
        self.consume_must_be(TokenCategory::Semicolon)?;
        let node = Node {
            value: Statement::Return(returned_value),
            position: token.position,
        };
        Ok(node)
    }

    fn parse_break_statement(&mut self) -> Result<Node<Statement>, Box<dyn Issue>> {
        let token = self.consume_must_be(TokenCategory::Break)?;
        let _ = self.consume_must_be(TokenCategory::Semicolon)?;
        let node = Node {
            value: Statement::Break,
            position: token.position,
        };
        Ok(node)
    }

    fn parse_arguments(&mut self) -> Result<Vec<Node<Argument>>, Box<dyn Issue>> {
        if self.current_token().category == TokenCategory::ParenClose {
            return Ok(vec![]);
        }

        let expression = self.parse_argument()?;

        let mut arguments = vec![expression];
        while let Some(_) = self.consume_if_matches(TokenCategory::Comma)? {
            let argument = self.parse_argument()?;
            arguments.push(argument);
        }
        Ok(arguments)
    }

    fn parse_argument(&mut self) -> Result<Node<Argument>, Box<dyn Issue>> {
        let mut passed_by = ArgumentPassedBy::Value;
        if self.consume_if_matches(TokenCategory::Reference)?.is_some() {
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

    fn parse_expression(&mut self) -> Result<Node<Expression>, Box<dyn Issue>> {
        let mut left_side = self.parse_concatenation_term()?;
        let mut current_token = self.current_token();
        while current_token.category == TokenCategory::Or {
            let _ = self.next_token()?;
            let right_side = self.parse_concatenation_term()?;
            let expression_type =
                Expression::Alternative(Box::new(left_side.clone()), Box::new(right_side.clone()));
            left_side = Node {
                value: expression_type,
                position: current_token.position,
            };
            current_token = self.current_token();
        }
        Ok(left_side)
    }

    fn parse_concatenation_term(&mut self) -> Result<Node<Expression>, Box<dyn Issue>> {
        let mut left_side = self.parse_relation_term()?;
        let mut current_token = self.current_token();
        while current_token.category == TokenCategory::And {
            let _ = self.next_token()?;
            let right_side = self.parse_relation_term()?;
            let expression_type = Expression::Concatenation(
                Box::new(left_side.clone()),
                Box::new(right_side.clone()),
            );
            left_side = Node {
                value: expression_type,
                position: current_token.position,
            };
            current_token = self.current_token();
        }
        Ok(left_side)
    }

    fn parse_relation_term(&mut self) -> Result<Node<Expression>, Box<dyn Issue>> {
        let left_side = self.parse_additive_term()?;
        if let Some(token) = self.consume_if_matches(TokenCategory::Equal)? {
            let right_side = self.parse_additive_term()?;
            return Ok(Node {
                value: Expression::Equal(Box::new(left_side), Box::new(right_side)),
                position: token.position,
            });
        }
        if let Some(token) = self.consume_if_matches(TokenCategory::NotEqual)? {
            let right_side = self.parse_additive_term()?;
            return Ok(Node {
                value: Expression::NotEqual(Box::new(left_side), Box::new(right_side)),
                position: token.position,
            });
        }
        if let Some(token) = self.consume_if_matches(TokenCategory::Greater)? {
            let right_side = self.parse_additive_term()?;
            return Ok(Node {
                value: Expression::Greater(Box::new(left_side), Box::new(right_side)),
                position: token.position,
            });
        }
        if let Some(token) = self.consume_if_matches(TokenCategory::GreaterOrEqual)? {
            let right_side = self.parse_additive_term()?;
            return Ok(Node {
                value: Expression::GreaterEqual(Box::new(left_side), Box::new(right_side)),
                position: token.position,
            });
        }
        if let Some(token) = self.consume_if_matches(TokenCategory::Less)? {
            let right_side = self.parse_additive_term()?;
            return Ok(Node {
                value: Expression::Less(Box::new(left_side), Box::new(right_side)),
                position: token.position,
            });
        }
        if let Some(token) = self.consume_if_matches(TokenCategory::LessOrEqual)? {
            let right_side = self.parse_additive_term()?;
            return Ok(Node {
                value: Expression::LessEqual(Box::new(left_side), Box::new(right_side)),
                position: token.position,
            });
        }
        Ok(left_side)
    }

    fn parse_additive_term(&mut self) -> Result<Node<Expression>, Box<dyn Issue>> {
        let mut left_side = self.parse_multiplicative_term()?;
        let mut current_token = self.current_token();
        while current_token.category == TokenCategory::Plus
            || current_token.category == TokenCategory::Minus
        {
            let _ = self.next_token()?;
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
            current_token = self.current_token();
        }
        Ok(left_side)
    }

    fn parse_multiplicative_term(&mut self) -> Result<Node<Expression>, Box<dyn Issue>> {
        let mut left_side = self.parse_casted_term()?;
        let mut current_token = self.current_token();
        while current_token.category == TokenCategory::Multiply
            || current_token.category == TokenCategory::Divide
        {
            let _ = self.next_token()?;
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
            current_token = self.current_token();
        }
        Ok(left_side)
    }

    fn parse_casted_term(&mut self) -> Result<Node<Expression>, Box<dyn Issue>> {
        let unary_term = self.parse_unary_term()?;
        let position = unary_term.position.clone();
        match self.consume_if_matches(TokenCategory::As)? {
            Some(_) => {
                let type_parsed = self.parse_type()?;
                return Ok(Node {
                    value: Expression::Casting {
                        value: Box::new(unary_term),
                        to_type: type_parsed,
                    },
                    position,
                });
            }
            None => Ok(unary_term),
        }
    }

    fn parse_unary_term(&mut self) -> Result<Node<Expression>, Box<dyn Issue>> {
        if let Some(token) = self.consume_if_matches(TokenCategory::Negate)? {
            let factor = self.parse_factor()?;
            return Ok(Node {
                value: Expression::BooleanNegation(Box::new(factor)),
                position: token.position,
            });
        }
        if let Some(token) = self.consume_if_matches(TokenCategory::Minus)? {
            let factor = self.parse_factor()?;
            return Ok(Node {
                value: Expression::ArithmeticNegation(Box::new(factor)),
                position: token.position,
            });
        }
        let factor = self.parse_factor()?;
        Ok(factor)
    }

    fn parse_factor(&mut self) -> Result<Node<Expression>, Box<dyn Issue>> {
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
        if self.consume_if_matches(TokenCategory::ParenOpen)?.is_some() {
            let expression = self.parse_expression()?;
            self.consume_must_be(TokenCategory::ParenClose)?;
            return Ok(expression);
        }
        self.parse_identifier_or_call()
    }

    fn parse_identifier_or_call(&mut self) -> Result<Node<Expression>, Box<dyn Issue>> {
        let identifier = self.parse_identifier()?;
        let position = identifier.position;

        let result = match self.consume_if_matches(TokenCategory::ParenOpen)? {
            Some(_) => {
                let args = self.parse_arguments()?.into_iter().map(Box::new).collect();
                let _ = self.consume_must_be(TokenCategory::ParenClose)?;
                Expression::FunctionCall {
                    identifier: identifier.value,
                    arguments: args,
                }
            }
            None => Expression::Variable(identifier.value),
        };
        Ok(Node {
            value: result,
            position,
        })
    }

    fn parse_switch_statement(&mut self) -> Result<Node<Statement>, Box<dyn Issue>> {
        let switch_token = self.consume_must_be(TokenCategory::Switch)?;
        let _ = self.consume_must_be(TokenCategory::ParenOpen)?;
        let switch_expressions = self.parse_switch_expressions()?;
        let _ = self.consume_must_be(TokenCategory::ParenClose)?;
        let _ = self.consume_must_be(TokenCategory::BraceOpen)?;
        let mut switch_cases: Vec<Node<SwitchCase>> = vec![];
        while self.current_token().category != TokenCategory::BraceClose {
            let switch_case = self.parse_switch_case()?;
            switch_cases.push(switch_case);
        }
        let _ = self.consume_must_be(TokenCategory::BraceClose)?;
        let node = Node {
            value: Statement::Switch {
                expressions: switch_expressions,
                cases: switch_cases,
            },
            position: switch_token.position,
        };
        Ok(node)
    }

    fn parse_switch_expressions(&mut self) -> Result<Vec<Node<SwitchExpression>>, Box<dyn Issue>> {
        let mut switch_expressions: Vec<Node<SwitchExpression>> = vec![];
        let mut expression = self.parse_switch_expression()?;
        switch_expressions.push(expression);
        while let Some(_) = self.consume_if_matches(TokenCategory::Comma)? {
            expression = self.parse_switch_expression()?;
            switch_expressions.push(expression);
        }
        Ok(switch_expressions)
    }

    fn parse_switch_expression(&mut self) -> Result<Node<SwitchExpression>, Box<dyn Issue>> {
        let expression = self.parse_expression()?;
        let position = expression.position;
        let alias = match self.consume_if_matches(TokenCategory::Colon)? {
            Some(_) => Some(self.parse_identifier()?),
            None => None,
        };
        let node = Node {
            value: SwitchExpression { expression, alias },
            position,
        };
        Ok(node)
    }

    fn parse_switch_case(&mut self) -> Result<Node<SwitchCase>, Box<dyn Issue>> {
        let paren_open_token = self.consume_must_be(TokenCategory::ParenOpen)?;
        let condition = self.parse_expression()?;
        let _ = self.consume_must_be(TokenCategory::ParenClose)?;
        let _ = self.consume_must_be(TokenCategory::Arrow)?;
        let block = self.parse_statement_block()?;
        let node = Node {
            value: SwitchCase { condition, block },
            position: paren_open_token.position,
        };
        Ok(node)
    }

    fn parse_type(&mut self) -> Result<Node<Type>, Box<dyn Issue>> {
        let token = self.current_token();
        let result = match token.category {
            TokenCategory::Bool => Type::Bool,
            TokenCategory::String => Type::Str,
            TokenCategory::I64 => Type::I64,
            TokenCategory::F64 => Type::F64,
            _ => {
                return Err(
                    self.create_parser_error(format!("Can't cast to type: {:?}.", token.category))
                );
            }
        };

        let _ = self.next_token()?;
        Ok(Node {
            value: result,
            position: token.position,
        })
    }

    fn parse_literal(&mut self) -> Result<Node<Literal>, Box<dyn Issue>> {
        let token = self.current_token();
        if self.consume_if_matches(TokenCategory::True)?.is_some() {
            return Ok(Node {
                value: Literal::True,
                position: token.position,
            });
        } else if self.consume_if_matches(TokenCategory::False)?.is_some() {
            return Ok(Node {
                value: Literal::False,
                position: token.position,
            });
        } else if self
            .consume_if_matches(TokenCategory::StringValue)?
            .is_some()
        {
            if let TokenValue::String(string) = token.value {
                return Ok(Node {
                    value: Literal::String(string),
                    position: token.position,
                });
            }
        } else if self.consume_if_matches(TokenCategory::I64Value)?.is_some() {
            if let TokenValue::I64(int) = token.value {
                return Ok(Node {
                    value: Literal::I64(int),
                    position: token.position,
                });
            }
        } else if self.consume_if_matches(TokenCategory::F64Value)?.is_some() {
            if let TokenValue::F64(float) = token.value {
                return Ok(Node {
                    value: Literal::F64(float),
                    position: token.position,
                });
            }
        }
        return Err(self.create_parser_error("Invalid literal".to_owned()));
    }

    fn parse_identifier(&mut self) -> Result<Node<Identifier>, Box<dyn Issue>> {
        let token = self.consume_must_be(TokenCategory::Identifier)?;
        if let TokenValue::String(name) = token.value {
            let node = Node {
                value: Identifier(name),
                position: token.position,
            };
            return Ok(node);
        }
        Err(self.create_parser_error(format!(
            "Wrong token value type - given: {:?}, expected: {:?}.",
            token.value,
            TokenValue::String("".to_owned())
        )))
    }

    fn consume_must_be(&mut self, category: TokenCategory) -> Result<Token, Box<dyn Issue>> {
        let current_token = self.current_token();
        if current_token.category == category {
            let _ = self.next_token()?;
            return Ok(current_token.clone());
        }
        Err(self.create_parser_error(format!(
            "Unexpected token - {:?}. Expected {:?}.",
            current_token.category, category
        )))
    }

    fn consume_if_matches(
        &mut self,
        category: TokenCategory,
    ) -> Result<Option<Token>, Box<dyn Issue>> {
        let current_token = self.current_token();
        if current_token.category == category {
            let _ = self.next_token()?;
            return Ok(Some(current_token.clone()));
        }
        Ok(None)
    }

    fn create_parser_error(&self, text: String) -> Box<dyn Issue> {
        let position = self.current_token().position;
        Box::new(ParserIssue {
            level: IssueLevel::ERROR,
            message: format!(
                "{}\nAt line: {}, column: {}",
                text, position.line, position.column
            ),
        })
    }
}

#[cfg(test)]
mod tests {
    use std::vec;

    use crate::{
        errors::{IssueLevel, LexerIssue},
        lazy_stream_reader::Position,
    };

    use super::*;

    struct LexerMock {
        current_token: Option<Token>,
        pub tokens: Vec<Token>,
    }

    impl LexerMock {
        fn new(mut tokens: Vec<Token>) -> LexerMock {
            let current_token = tokens.remove(0);
            LexerMock {
                current_token: Some(current_token),
                tokens,
            }
        }
    }

    impl ILexer for LexerMock {
        fn current(&self) -> &Option<Token> {
            &self.current_token
        }

        fn next(&mut self) -> Result<Token, Box<dyn Issue>> {
            if self.tokens.len() == 0 {
                return Err(Box::new(LexerIssue {
                    level: IssueLevel::ERROR,
                    message: "".to_owned(),
                }));
            }
            let next_token = self.tokens.remove(0);
            self.current_token = Some(next_token.clone());
            Ok(next_token)
        }
    }

    fn default_position() -> Position {
        Position {
            line: 0,
            column: 0,
            offset: 0,
        }
    }

    fn create_token(category: TokenCategory, value: TokenValue) -> Token {
        Token {
            category,
            value,
            position: default_position(),
        }
    }

    // tests

    #[test]
    fn parse_statement_block_fail() {
        let token_series = vec![vec![
            create_token(TokenCategory::BraceOpen, TokenValue::Null),
            create_token(TokenCategory::ETX, TokenValue::Null),
        ]];

        for series in token_series {
            let mock_lexer = LexerMock::new(series);
            let mut parser = Parser::new(mock_lexer);

            assert!(parser.parse_statement_block().is_err());
        }
    }

    #[test]
    fn parse_statement_block() {
        let token_series = vec![
            vec![
                create_token(TokenCategory::BraceOpen, TokenValue::Null),
                create_token(TokenCategory::BraceClose, TokenValue::Null),
                create_token(TokenCategory::ETX, TokenValue::Null),
            ],
            vec![
                create_token(TokenCategory::BraceOpen, TokenValue::Null),
                create_token(
                    TokenCategory::Identifier,
                    TokenValue::String("x".to_owned()),
                ),
                create_token(TokenCategory::Assign, TokenValue::Null),
                create_token(TokenCategory::I64Value, TokenValue::I64(5)),
                create_token(TokenCategory::Semicolon, TokenValue::Null),
                create_token(TokenCategory::BraceClose, TokenValue::Null),
                create_token(TokenCategory::ETX, TokenValue::Null),
            ],
            vec![
                create_token(TokenCategory::BraceOpen, TokenValue::Null),
                create_token(
                    TokenCategory::Identifier,
                    TokenValue::String("x".to_owned()),
                ),
                create_token(TokenCategory::Assign, TokenValue::Null),
                create_token(TokenCategory::I64Value, TokenValue::I64(5)),
                create_token(TokenCategory::Semicolon, TokenValue::Null),
                create_token(
                    TokenCategory::Identifier,
                    TokenValue::String("x".to_owned()),
                ),
                create_token(TokenCategory::Assign, TokenValue::Null),
                create_token(TokenCategory::I64Value, TokenValue::I64(5)),
                create_token(TokenCategory::Semicolon, TokenValue::Null),
                create_token(TokenCategory::BraceClose, TokenValue::Null),
                create_token(TokenCategory::ETX, TokenValue::Null),
            ],
        ];

        let expected = [
            Block(vec![]),
            Block(vec![Node {
                value: Statement::Assignment {
                    identifier: Node {
                        value: Identifier("x".to_owned()),
                        position: default_position(),
                    },
                    value: Node {
                        value: Expression::Literal(Literal::I64(5)),
                        position: default_position(),
                    },
                },
                position: default_position(),
            }]),
            Block(vec![
                Node {
                    value: Statement::Assignment {
                        identifier: Node {
                            value: Identifier("x".to_owned()),
                            position: default_position(),
                        },
                        value: Node {
                            value: Expression::Literal(Literal::I64(5)),
                            position: default_position(),
                        },
                    },
                    position: default_position(),
                },
                Node {
                    value: Statement::Assignment {
                        identifier: Node {
                            value: Identifier("x".to_owned()),
                            position: default_position(),
                        },
                        value: Node {
                            value: Expression::Literal(Literal::I64(5)),
                            position: default_position(),
                        },
                    },
                    position: default_position(),
                },
            ]),
        ];

        for (idx, series) in token_series.iter().enumerate() {
            let mock_lexer = LexerMock::new(series.to_vec());
            let mut parser = Parser::new(mock_lexer);

            let node = parser.parse_statement_block().unwrap();
            assert!(node.value == expected[idx]);
        }
    }

    #[test]
    fn parse_statement_fail() {
        let token_series = vec![
            vec![
                // i64 a = 5
                create_token(TokenCategory::I64, TokenValue::Null),
                create_token(
                    TokenCategory::Identifier,
                    TokenValue::String("a".to_owned()),
                ),
                create_token(TokenCategory::Assign, TokenValue::Null),
                create_token(TokenCategory::I64Value, TokenValue::I64(5)),
                create_token(TokenCategory::ETX, TokenValue::Null),
            ],
            vec![create_token(TokenCategory::And, TokenValue::Null)],
            vec![create_token(TokenCategory::ETX, TokenValue::Null)],
        ];

        for series in token_series {
            let mock_lexer = LexerMock::new(series);
            let mut parser = Parser::new(mock_lexer);

            assert!(parser.parse_statement().is_err());
        }
    }

    #[test]
    fn parse_statement() {
        let token_series = vec![
            vec![
                // x = 5;
                create_token(
                    TokenCategory::Identifier,
                    TokenValue::String("x".to_owned()),
                ),
                create_token(TokenCategory::Assign, TokenValue::Null),
                create_token(TokenCategory::I64Value, TokenValue::I64(5)),
                create_token(TokenCategory::Semicolon, TokenValue::Null),
                create_token(TokenCategory::ETX, TokenValue::Null),
            ],
            vec![
                // print();
                create_token(
                    TokenCategory::Identifier,
                    TokenValue::String("print".to_owned()),
                ),
                create_token(TokenCategory::ParenOpen, TokenValue::Null),
                create_token(TokenCategory::ParenClose, TokenValue::Null),
                create_token(TokenCategory::Semicolon, TokenValue::Null),
                create_token(TokenCategory::ETX, TokenValue::Null),
            ],
            vec![
                // if (true) {}
                create_token(TokenCategory::If, TokenValue::Null),
                create_token(TokenCategory::ParenOpen, TokenValue::Null),
                create_token(TokenCategory::True, TokenValue::Null),
                create_token(TokenCategory::ParenClose, TokenValue::Null),
                create_token(TokenCategory::BraceOpen, TokenValue::Null),
                create_token(TokenCategory::BraceClose, TokenValue::Null),
                create_token(TokenCategory::ETX, TokenValue::Null),
            ],
            vec![
                // for(;true;) {}
                create_token(TokenCategory::For, TokenValue::Null),
                create_token(TokenCategory::ParenOpen, TokenValue::Null),
                create_token(TokenCategory::Semicolon, TokenValue::Null),
                create_token(TokenCategory::True, TokenValue::Null),
                create_token(TokenCategory::Semicolon, TokenValue::Null),
                create_token(TokenCategory::ParenClose, TokenValue::Null),
                create_token(TokenCategory::BraceOpen, TokenValue::Null),
                create_token(TokenCategory::BraceClose, TokenValue::Null),
                create_token(TokenCategory::ETX, TokenValue::Null),
            ],
            vec![
                // switch(x) {
                //      (true) -> {}
                // }
                create_token(TokenCategory::Switch, TokenValue::Null),
                create_token(TokenCategory::ParenOpen, TokenValue::Null),
                create_token(
                    TokenCategory::Identifier,
                    TokenValue::String("x".to_owned()),
                ),
                create_token(TokenCategory::ParenClose, TokenValue::Null),
                create_token(TokenCategory::BraceOpen, TokenValue::Null),
                create_token(TokenCategory::ParenOpen, TokenValue::Null),
                create_token(TokenCategory::True, TokenValue::Null),
                create_token(TokenCategory::ParenClose, TokenValue::Null),
                create_token(TokenCategory::Arrow, TokenValue::Null),
                create_token(TokenCategory::BraceOpen, TokenValue::Null),
                create_token(TokenCategory::BraceClose, TokenValue::Null),
                create_token(TokenCategory::BraceClose, TokenValue::Null),
                create_token(TokenCategory::ETX, TokenValue::Null),
            ],
            vec![
                // return;
                create_token(TokenCategory::Return, TokenValue::Null),
                create_token(TokenCategory::Semicolon, TokenValue::Null),
                create_token(TokenCategory::ETX, TokenValue::Null),
            ],
            vec![
                // break;
                create_token(TokenCategory::Break, TokenValue::Null),
                create_token(TokenCategory::Semicolon, TokenValue::Null),
                create_token(TokenCategory::ETX, TokenValue::Null),
            ],
            vec![
                // i64 a = 5;
                create_token(TokenCategory::I64, TokenValue::Null),
                create_token(
                    TokenCategory::Identifier,
                    TokenValue::String("a".to_owned()),
                ),
                create_token(TokenCategory::Assign, TokenValue::Null),
                create_token(TokenCategory::I64Value, TokenValue::I64(5)),
                create_token(TokenCategory::Semicolon, TokenValue::Null),
                create_token(TokenCategory::ETX, TokenValue::Null),
            ],
        ];

        let expected = [
            Statement::Assignment {
                identifier: Node {
                    value: Identifier("x".to_owned()),
                    position: default_position(),
                },
                value: Node {
                    value: Expression::Literal(Literal::I64(5)),
                    position: default_position(),
                },
            },
            Statement::FunctionCall {
                identifier: Node {
                    value: Identifier("print".to_owned()),
                    position: default_position(),
                },
                arguments: vec![],
            },
            Statement::Conditional {
                condition: Node {
                    value: Expression::Literal(Literal::True),
                    position: default_position(),
                },
                if_block: Node {
                    value: Block(vec![]),
                    position: default_position(),
                },
                else_block: None,
            },
            Statement::ForLoop {
                declaration: None,
                condition: Node {
                    value: Expression::Literal(Literal::True),
                    position: default_position(),
                },
                assignment: None,
                block: Node {
                    value: Block(vec![]),
                    position: default_position(),
                },
            },
            Statement::Switch {
                expressions: vec![Node {
                    value: SwitchExpression {
                        expression: Node {
                            value: Expression::Variable(Identifier("x".to_owned())),
                            position: default_position(),
                        },
                        alias: None,
                    },
                    position: default_position(),
                }],
                cases: vec![Node {
                    value: SwitchCase {
                        condition: Node {
                            value: Expression::Literal(Literal::True),
                            position: default_position(),
                        },
                        block: Node {
                            value: Block(vec![]),
                            position: default_position(),
                        },
                    },
                    position: default_position(),
                }],
            },
            Statement::Return(None),
            Statement::Break,
            Statement::Declaration {
                var_type: Node {
                    value: Type::I64,
                    position: default_position(),
                },
                identifier: Node {
                    value: Identifier("a".to_owned()),
                    position: default_position(),
                },
                value: Some(Node {
                    value: Expression::Literal(Literal::I64(5)),
                    position: default_position(),
                }),
            },
        ];

        for (idx, series) in token_series.iter().enumerate() {
            let mock_lexer = LexerMock::new(series.to_vec());
            let mut parser = Parser::new(mock_lexer);

            let node = parser.parse_statement().unwrap();
            assert!(node.value == expected[idx]);
        }
    }

    #[test]
    fn parse_function_declaration_fail() {
        let token_series = vec![vec![
            // fn add(): , {}
            create_token(TokenCategory::Fn, TokenValue::Null),
            create_token(
                TokenCategory::Identifier,
                TokenValue::String("add".to_owned()),
            ),
            create_token(TokenCategory::ParenOpen, TokenValue::Null),
            create_token(TokenCategory::ParenClose, TokenValue::Null),
            create_token(TokenCategory::Colon, TokenValue::Null),
            create_token(TokenCategory::Comma, TokenValue::Null),
            create_token(TokenCategory::BraceOpen, TokenValue::Null),
            create_token(TokenCategory::BraceClose, TokenValue::Null),
            create_token(TokenCategory::ETX, TokenValue::Null),
        ]];

        for series in token_series {
            let mock_lexer = LexerMock::new(series);
            let mut parser = Parser::new(mock_lexer);

            assert!(parser.parse_function_declaration().is_err());
        }
    }

    #[test]
    fn parse_function_declaration() {
        let token_series = vec![
            vec![
                // fn add(): i64 {}
                create_token(TokenCategory::Fn, TokenValue::Null),
                create_token(
                    TokenCategory::Identifier,
                    TokenValue::String("add".to_owned()),
                ),
                create_token(TokenCategory::ParenOpen, TokenValue::Null),
                create_token(TokenCategory::ParenClose, TokenValue::Null),
                create_token(TokenCategory::Colon, TokenValue::Null),
                create_token(TokenCategory::I64, TokenValue::Null),
                create_token(TokenCategory::BraceOpen, TokenValue::Null),
                create_token(TokenCategory::BraceClose, TokenValue::Null),
                create_token(TokenCategory::ETX, TokenValue::Null),
            ],
            vec![
                // fn add(): void {}
                create_token(TokenCategory::Fn, TokenValue::Null),
                create_token(
                    TokenCategory::Identifier,
                    TokenValue::String("add".to_owned()),
                ),
                create_token(TokenCategory::ParenOpen, TokenValue::Null),
                create_token(TokenCategory::ParenClose, TokenValue::Null),
                create_token(TokenCategory::Colon, TokenValue::Null),
                create_token(TokenCategory::Void, TokenValue::Null),
                create_token(TokenCategory::BraceOpen, TokenValue::Null),
                create_token(TokenCategory::BraceClose, TokenValue::Null),
                create_token(TokenCategory::ETX, TokenValue::Null),
            ],
        ];

        let expected = [
            Statement::FunctionDeclaration {
                identifier: Node {
                    value: Identifier("add".to_owned()),
                    position: default_position(),
                },
                parameters: vec![],
                return_type: Node {
                    value: Type::I64,
                    position: default_position(),
                },
                block: Node {
                    value: Block(vec![]),
                    position: default_position(),
                },
            },
            Statement::FunctionDeclaration {
                identifier: Node {
                    value: Identifier("add".to_owned()),
                    position: default_position(),
                },
                parameters: vec![],
                return_type: Node {
                    value: Type::Void,
                    position: default_position(),
                },
                block: Node {
                    value: Block(vec![]),
                    position: default_position(),
                },
            },
        ];

        for (idx, series) in token_series.iter().enumerate() {
            let mock_lexer = LexerMock::new(series.to_vec());
            let mut parser = Parser::new(mock_lexer);

            let node = parser.parse_function_declaration().unwrap();
            assert!(node.value == expected[idx]);
        }
    }

    #[test]
    fn parse_parameters_fail() {
        let tokens = vec![
            // i64 x,
            create_token(TokenCategory::I64, TokenValue::Null),
            create_token(
                TokenCategory::Identifier,
                TokenValue::String("x".to_owned()),
            ),
            create_token(TokenCategory::Comma, TokenValue::Null),
            create_token(TokenCategory::ETX, TokenValue::Null),
        ];

        let mock_lexer = LexerMock::new(tokens);
        let mut parser = Parser::new(mock_lexer);

        assert!(parser.parse_parameters().is_err());
    }

    #[test]
    fn parse_parameters() {
        let token_series = vec![
            vec![
                create_token(TokenCategory::ParenClose, TokenValue::Null),
                create_token(TokenCategory::ETX, TokenValue::Null),
            ],
            vec![
                // i64 x
                create_token(TokenCategory::I64, TokenValue::Null),
                create_token(
                    TokenCategory::Identifier,
                    TokenValue::String("x".to_owned()),
                ),
                create_token(TokenCategory::ETX, TokenValue::Null),
            ],
            vec![
                // i64 x, i64 y
                create_token(TokenCategory::I64, TokenValue::Null),
                create_token(
                    TokenCategory::Identifier,
                    TokenValue::String("x".to_owned()),
                ),
                create_token(TokenCategory::Comma, TokenValue::Null),
                create_token(TokenCategory::I64, TokenValue::Null),
                create_token(
                    TokenCategory::Identifier,
                    TokenValue::String("y".to_owned()),
                ),
                create_token(TokenCategory::ETX, TokenValue::Null),
            ],
        ];

        let expected = [
            vec![],
            vec![Node {
                value: Parameter {
                    passed_by: ParameterPassedBy::Value,
                    parameter_type: Node {
                        value: Type::I64,
                        position: default_position(),
                    },
                    identifier: Node {
                        value: Identifier("x".to_owned()),
                        position: default_position(),
                    },
                    value: None,
                },
                position: default_position(),
            }],
            vec![
                Node {
                    value: Parameter {
                        passed_by: ParameterPassedBy::Value,
                        parameter_type: Node {
                            value: Type::I64,
                            position: default_position(),
                        },
                        identifier: Node {
                            value: Identifier("x".to_owned()),
                            position: default_position(),
                        },
                        value: None,
                    },
                    position: default_position(),
                },
                Node {
                    value: Parameter {
                        passed_by: ParameterPassedBy::Value,
                        parameter_type: Node {
                            value: Type::I64,
                            position: default_position(),
                        },
                        identifier: Node {
                            value: Identifier("y".to_owned()),
                            position: default_position(),
                        },
                        value: None,
                    },
                    position: default_position(),
                },
            ],
        ];

        for (idx, series) in token_series.iter().enumerate() {
            let mock_lexer = LexerMock::new(series.to_vec());
            let mut parser = Parser::new(mock_lexer);

            let vector = parser.parse_parameters().unwrap();
            assert!(vector == expected[idx]);
        }
    }

    #[test]
    fn parse_parameter() {
        let token_series = vec![
            vec![
                // &i64 x = 0
                create_token(TokenCategory::Reference, TokenValue::Null),
                create_token(TokenCategory::I64, TokenValue::Null),
                create_token(
                    TokenCategory::Identifier,
                    TokenValue::String("x".to_owned()),
                ),
                create_token(TokenCategory::Assign, TokenValue::Null),
                create_token(TokenCategory::I64Value, TokenValue::I64(0)),
                create_token(TokenCategory::ETX, TokenValue::Null),
            ],
            vec![
                // i64 x
                create_token(TokenCategory::I64, TokenValue::Null),
                create_token(
                    TokenCategory::Identifier,
                    TokenValue::String("x".to_owned()),
                ),
                create_token(TokenCategory::ETX, TokenValue::Null),
            ],
        ];

        let expected = [
            Parameter {
                passed_by: ParameterPassedBy::Reference,
                parameter_type: Node {
                    value: Type::I64,
                    position: default_position(),
                },
                identifier: Node {
                    value: Identifier("x".to_owned()),
                    position: default_position(),
                },
                value: Some(Node {
                    value: Expression::Literal(Literal::I64(0)),
                    position: default_position(),
                }),
            },
            Parameter {
                passed_by: ParameterPassedBy::Value,
                parameter_type: Node {
                    value: Type::I64,
                    position: default_position(),
                },
                identifier: Node {
                    value: Identifier("x".to_owned()),
                    position: default_position(),
                },
                value: None,
            },
        ];

        for (idx, series) in token_series.iter().enumerate() {
            let mock_lexer = LexerMock::new(series.to_vec());
            let mut parser = Parser::new(mock_lexer);

            let node = parser.parse_parameter().unwrap();
            assert!(node.value == expected[idx]);
        }
    }

    #[test]
    fn parse_for_statement_fail() {
        let token_series = vec![
            vec![
                // for (
                create_token(TokenCategory::For, TokenValue::Null),
                create_token(TokenCategory::ParenOpen, TokenValue::Null),
                create_token(TokenCategory::ETX, TokenValue::Null),
            ],
            vec![
                // for (;;) {}
                create_token(TokenCategory::For, TokenValue::Null),
                create_token(TokenCategory::ParenOpen, TokenValue::Null),
                create_token(TokenCategory::Semicolon, TokenValue::Null),
                create_token(TokenCategory::Semicolon, TokenValue::Null),
                create_token(TokenCategory::ParenClose, TokenValue::Null),
                create_token(TokenCategory::BraceOpen, TokenValue::Null),
                create_token(TokenCategory::BraceClose, TokenValue::Null),
                create_token(TokenCategory::ETX, TokenValue::Null),
            ],
            vec![
                //  for (;x; {}
                create_token(TokenCategory::For, TokenValue::Null),
                create_token(TokenCategory::ParenOpen, TokenValue::Null),
                create_token(TokenCategory::Semicolon, TokenValue::Null),
                create_token(
                    TokenCategory::Identifier,
                    TokenValue::String("x".to_owned()),
                ),
                create_token(TokenCategory::Semicolon, TokenValue::Null),
                create_token(TokenCategory::BraceOpen, TokenValue::Null),
                create_token(TokenCategory::BraceClose, TokenValue::Null),
                create_token(TokenCategory::ETX, TokenValue::Null),
            ],
        ];

        for series in token_series {
            let mock_lexer = LexerMock::new(series);
            let mut parser = Parser::new(mock_lexer);

            assert!(parser.parse_for_statement().is_err());
        }
    }

    #[test]
    fn parse_for_statement() {
        let token_series = vec![
            vec![
                // for (i64 x = 0; x < 5; x = x + 1) {}
                create_token(TokenCategory::For, TokenValue::Null),
                create_token(TokenCategory::ParenOpen, TokenValue::Null),
                create_token(TokenCategory::I64, TokenValue::Null),
                create_token(
                    TokenCategory::Identifier,
                    TokenValue::String("x".to_owned()),
                ),
                create_token(TokenCategory::Assign, TokenValue::Null),
                create_token(TokenCategory::I64Value, TokenValue::I64(0)),
                create_token(TokenCategory::Semicolon, TokenValue::Null),
                create_token(
                    TokenCategory::Identifier,
                    TokenValue::String("x".to_owned()),
                ),
                create_token(TokenCategory::Less, TokenValue::Null),
                create_token(TokenCategory::I64Value, TokenValue::I64(5)),
                create_token(TokenCategory::Semicolon, TokenValue::Null),
                create_token(
                    TokenCategory::Identifier,
                    TokenValue::String("x".to_owned()),
                ),
                create_token(TokenCategory::Assign, TokenValue::Null),
                create_token(
                    TokenCategory::Identifier,
                    TokenValue::String("x".to_owned()),
                ),
                create_token(TokenCategory::Plus, TokenValue::Null),
                create_token(TokenCategory::I64Value, TokenValue::I64(1)),
                create_token(TokenCategory::ParenClose, TokenValue::Null),
                create_token(TokenCategory::BraceOpen, TokenValue::Null),
                create_token(TokenCategory::BraceClose, TokenValue::Null),
                create_token(TokenCategory::ETX, TokenValue::Null),
            ],
            vec![
                // for (;x < 5;) {}
                create_token(TokenCategory::For, TokenValue::Null),
                create_token(TokenCategory::ParenOpen, TokenValue::Null),
                create_token(TokenCategory::Semicolon, TokenValue::Null),
                create_token(
                    TokenCategory::Identifier,
                    TokenValue::String("x".to_owned()),
                ),
                create_token(TokenCategory::Less, TokenValue::Null),
                create_token(TokenCategory::I64Value, TokenValue::I64(5)),
                create_token(TokenCategory::Semicolon, TokenValue::Null),
                create_token(TokenCategory::ParenClose, TokenValue::Null),
                create_token(TokenCategory::BraceOpen, TokenValue::Null),
                create_token(TokenCategory::BraceClose, TokenValue::Null),
                create_token(TokenCategory::ETX, TokenValue::Null),
            ],
        ];

        let expected = vec![
            Statement::ForLoop {
                declaration: Some(Node {
                    value: Box::new(Statement::Declaration {
                        var_type: Node {
                            value: Type::I64,
                            position: default_position(),
                        },
                        identifier: Node {
                            value: Identifier("x".to_owned()),
                            position: default_position(),
                        },
                        value: Some(Node {
                            value: Expression::Literal(Literal::I64(0)),
                            position: default_position(),
                        }),
                    }),
                    position: default_position(),
                }),
                condition: Node {
                    value: Expression::Less(
                        Box::new(Node {
                            value: Expression::Variable(Identifier("x".to_owned())),
                            position: default_position(),
                        }),
                        Box::new(Node {
                            value: Expression::Literal(Literal::I64(5)),
                            position: default_position(),
                        }),
                    ),
                    position: default_position(),
                },
                assignment: Some(Node {
                    value: Box::new(Statement::Assignment {
                        identifier: Node {
                            value: Identifier("x".to_owned()),
                            position: default_position(),
                        },
                        value: Node {
                            value: Expression::Addition(
                                Box::new(Node {
                                    value: Expression::Variable(Identifier("x".to_owned())),
                                    position: default_position(),
                                }),
                                Box::new(Node {
                                    value: Expression::Literal(Literal::I64(1)),
                                    position: default_position(),
                                }),
                            ),
                            position: default_position(),
                        },
                    }),
                    position: default_position(),
                }),
                block: Node {
                    value: Block(vec![]),
                    position: default_position(),
                },
            },
            Statement::ForLoop {
                declaration: None,
                condition: Node {
                    value: Expression::Less(
                        Box::new(Node {
                            value: Expression::Variable(Identifier("x".to_owned())),
                            position: default_position(),
                        }),
                        Box::new(Node {
                            value: Expression::Literal(Literal::I64(5)),
                            position: default_position(),
                        }),
                    ),
                    position: default_position(),
                },
                assignment: None,
                block: Node {
                    value: Block(vec![]),
                    position: default_position(),
                },
            },
        ];

        for (idx, series) in token_series.iter().enumerate() {
            let mock_lexer = LexerMock::new(series.to_vec());
            let mut parser = Parser::new(mock_lexer);

            let node = parser.parse_for_statement().unwrap();
            assert!(node.value == expected[idx]);
        }
    }

    #[test]
    fn parse_if_statement_fail() {
        let token_series = vec![
            vec![
                // (true) {}
                create_token(TokenCategory::ParenOpen, TokenValue::Null),
                create_token(TokenCategory::True, TokenValue::Null),
                create_token(TokenCategory::ParenClose, TokenValue::Null),
                create_token(TokenCategory::BraceOpen, TokenValue::Null),
                create_token(TokenCategory::BraceClose, TokenValue::Null),
                create_token(TokenCategory::ETX, TokenValue::Null),
            ],
            vec![
                // if true) {}
                create_token(TokenCategory::If, TokenValue::Null),
                create_token(TokenCategory::True, TokenValue::Null),
                create_token(TokenCategory::ParenClose, TokenValue::Null),
                create_token(TokenCategory::BraceOpen, TokenValue::Null),
                create_token(TokenCategory::BraceClose, TokenValue::Null),
                create_token(TokenCategory::ETX, TokenValue::Null),
            ],
            vec![
                // if (True {}
                create_token(TokenCategory::If, TokenValue::Null),
                create_token(TokenCategory::ParenOpen, TokenValue::Null),
                create_token(TokenCategory::True, TokenValue::Null),
                create_token(TokenCategory::BraceOpen, TokenValue::Null),
                create_token(TokenCategory::BraceClose, TokenValue::Null),
                create_token(TokenCategory::ETX, TokenValue::Null),
            ],
        ];

        for series in token_series {
            let mock_lexer = LexerMock::new(series.to_vec());
            let mut parser = Parser::new(mock_lexer);

            assert!(parser.parse_assign_or_call().is_err());
        }
    }

    #[test]
    fn parse_if_statement() {
        let token_series = vec![
            vec![
                // if (true) {}
                create_token(TokenCategory::If, TokenValue::Null),
                create_token(TokenCategory::ParenOpen, TokenValue::Null),
                create_token(TokenCategory::True, TokenValue::Null),
                create_token(TokenCategory::ParenClose, TokenValue::Null),
                create_token(TokenCategory::BraceOpen, TokenValue::Null),
                create_token(TokenCategory::BraceClose, TokenValue::Null),
                create_token(TokenCategory::ETX, TokenValue::Null),
            ],
            vec![
                // if (true) {} else {}
                create_token(TokenCategory::If, TokenValue::Null),
                create_token(TokenCategory::ParenOpen, TokenValue::Null),
                create_token(TokenCategory::True, TokenValue::Null),
                create_token(TokenCategory::ParenClose, TokenValue::Null),
                create_token(TokenCategory::BraceOpen, TokenValue::Null),
                create_token(TokenCategory::BraceClose, TokenValue::Null),
                create_token(TokenCategory::Else, TokenValue::Null),
                create_token(TokenCategory::BraceOpen, TokenValue::Null),
                create_token(TokenCategory::BraceClose, TokenValue::Null),
                create_token(TokenCategory::ETX, TokenValue::Null),
            ],
        ];

        let expected = vec![
            Statement::Conditional {
                condition: Node {
                    value: Expression::Literal(Literal::True),
                    position: default_position(),
                },
                if_block: Node {
                    value: Block(vec![]),
                    position: default_position(),
                },
                else_block: None,
            },
            Statement::Conditional {
                condition: Node {
                    value: Expression::Literal(Literal::True),
                    position: default_position(),
                },
                if_block: Node {
                    value: Block(vec![]),
                    position: default_position(),
                },
                else_block: Some(Node {
                    value: Block(vec![]),
                    position: default_position(),
                }),
            },
        ];

        for (idx, series) in token_series.iter().enumerate() {
            let mock_lexer = LexerMock::new(series.to_vec());
            let mut parser = Parser::new(mock_lexer);

            let node = parser.parse_if_statement().unwrap();
            assert!(node.value == expected[idx]);
        }
    }

    #[test]
    fn parse_assign_or_call_fail() {
        let token_series = vec![
            vec![
                // print(;
                create_token(
                    TokenCategory::Identifier,
                    TokenValue::String("print".to_owned()),
                ),
                create_token(TokenCategory::ParenOpen, TokenValue::Null),
                create_token(TokenCategory::Semicolon, TokenValue::Null),
                create_token(TokenCategory::ETX, TokenValue::Null),
            ],
            vec![
                // print()
                create_token(
                    TokenCategory::Identifier,
                    TokenValue::String("print".to_owned()),
                ),
                create_token(TokenCategory::ParenOpen, TokenValue::Null),
                create_token(TokenCategory::ParenClose, TokenValue::Null),
                create_token(TokenCategory::ETX, TokenValue::Null),
            ],
            vec![
                // x = 5
                create_token(
                    TokenCategory::Identifier,
                    TokenValue::String("x".to_owned()),
                ),
                create_token(TokenCategory::Assign, TokenValue::Null),
                create_token(TokenCategory::I64Value, TokenValue::I64(5)),
                create_token(TokenCategory::ETX, TokenValue::Null),
            ],
            vec![
                create_token(
                    TokenCategory::Identifier,
                    TokenValue::String("x".to_owned()),
                ),
                create_token(TokenCategory::Comma, TokenValue::Null),
                create_token(TokenCategory::ETX, TokenValue::Null),
            ],
        ];

        for series in token_series {
            let mock_lexer = LexerMock::new(series.to_vec());
            let mut parser = Parser::new(mock_lexer);

            assert!(parser.parse_assign_or_call().is_err());
        }
    }

    #[test]
    fn parse_assign_or_call() {
        let token_series = vec![
            vec![
                // print();
                create_token(
                    TokenCategory::Identifier,
                    TokenValue::String("print".to_owned()),
                ),
                create_token(TokenCategory::ParenOpen, TokenValue::Null),
                create_token(TokenCategory::ParenClose, TokenValue::Null),
                create_token(TokenCategory::Semicolon, TokenValue::Null),
                create_token(TokenCategory::ETX, TokenValue::Null),
            ],
            vec![
                // x = 5;
                create_token(
                    TokenCategory::Identifier,
                    TokenValue::String("x".to_owned()),
                ),
                create_token(TokenCategory::Assign, TokenValue::Null),
                create_token(TokenCategory::I64Value, TokenValue::I64(5)),
                create_token(TokenCategory::Semicolon, TokenValue::Null),
                create_token(TokenCategory::ETX, TokenValue::Null),
            ],
        ];

        let expected = vec![
            Statement::FunctionCall {
                identifier: Node {
                    value: Identifier("print".to_owned()),
                    position: default_position(),
                },
                arguments: vec![],
            },
            Statement::Assignment {
                identifier: Node {
                    value: Identifier("x".to_owned()),
                    position: default_position(),
                },
                value: Node {
                    value: Expression::Literal(Literal::I64(5)),
                    position: default_position(),
                },
            },
        ];

        for (idx, series) in token_series.iter().enumerate() {
            let mock_lexer = LexerMock::new(series.to_vec());
            let mut parser = Parser::new(mock_lexer);

            let node = parser.parse_assign_or_call().unwrap();
            assert!(node.value == expected[idx]);
        }
    }

    #[test]
    fn parse_declaration() {
        let token_series = vec![
            vec![
                // i64 a
                create_token(TokenCategory::I64, TokenValue::Null),
                create_token(
                    TokenCategory::Identifier,
                    TokenValue::String("a".to_owned()),
                ),
                create_token(TokenCategory::ETX, TokenValue::Null),
            ],
            vec![
                // i64 a = 5
                create_token(TokenCategory::I64, TokenValue::Null),
                create_token(
                    TokenCategory::Identifier,
                    TokenValue::String("a".to_owned()),
                ),
                create_token(TokenCategory::Assign, TokenValue::Null),
                create_token(TokenCategory::I64Value, TokenValue::I64(5)),
                create_token(TokenCategory::ETX, TokenValue::Null),
            ],
        ];

        let expected = [
            Statement::Declaration {
                var_type: Node {
                    value: Type::I64,
                    position: default_position(),
                },
                identifier: Node {
                    value: Identifier("a".to_owned()),
                    position: default_position(),
                },
                value: None,
            },
            Statement::Declaration {
                var_type: Node {
                    value: Type::I64,
                    position: default_position(),
                },
                identifier: Node {
                    value: Identifier("a".to_owned()),
                    position: default_position(),
                },
                value: Some(Node {
                    value: Expression::Literal(Literal::I64(5)),
                    position: default_position(),
                }),
            },
        ];

        for (idx, series) in token_series.iter().enumerate() {
            let mock_lexer = LexerMock::new(series.to_vec());
            let mut parser = Parser::new(mock_lexer);

            let node = parser.parse_declaration().unwrap();
            assert!(node.value == expected[idx]);
        }
    }

    #[test]
    fn parse_return_statement_fail() {
        let token_series = vec![
            vec![
                create_token(TokenCategory::Comma, TokenValue::Null),
                create_token(TokenCategory::ETX, TokenValue::Null),
            ],
            vec![
                // return
                create_token(TokenCategory::Return, TokenValue::Null),
                create_token(TokenCategory::ETX, TokenValue::Null),
            ],
            vec![
                // return 5
                create_token(TokenCategory::Return, TokenValue::Null),
                create_token(TokenCategory::I64Value, TokenValue::I64(5)),
                create_token(TokenCategory::ETX, TokenValue::Null),
            ],
        ];

        for series in token_series {
            let mock_lexer = LexerMock::new(series);
            let mut parser = Parser::new(mock_lexer);

            assert!(parser.parse_return_statement().is_err());
        }
    }

    #[test]
    fn parse_return_statement() {
        let token_series = vec![
            vec![
                // return;
                create_token(TokenCategory::Return, TokenValue::Null),
                create_token(TokenCategory::Semicolon, TokenValue::Null),
                create_token(TokenCategory::ETX, TokenValue::Null),
            ],
            vec![
                // return 5;
                create_token(TokenCategory::Return, TokenValue::Null),
                create_token(TokenCategory::I64Value, TokenValue::I64(5)),
                create_token(TokenCategory::Semicolon, TokenValue::Null),
                create_token(TokenCategory::ETX, TokenValue::Null),
            ],
        ];

        let expected = [
            Statement::Return(None),
            Statement::Return(Some(Node {
                value: Expression::Literal(Literal::I64(5)),
                position: default_position(),
            })),
        ];

        for (idx, series) in token_series.iter().enumerate() {
            let mock_lexer = LexerMock::new(series.to_vec());
            let mut parser = Parser::new(mock_lexer);

            let node = parser.parse_return_statement().unwrap();
            assert!(node.value == expected[idx]);
        }
    }

    #[test]
    fn parse_break_statement_fail() {
        let token_series = vec![
            vec![
                create_token(TokenCategory::ParenClose, TokenValue::Null),
                create_token(TokenCategory::ETX, TokenValue::Null),
            ],
            vec![
                // break
                create_token(TokenCategory::Break, TokenValue::Null),
                create_token(TokenCategory::ETX, TokenValue::Null),
            ],
        ];

        for series in token_series {
            let mock_lexer = LexerMock::new(series);
            let mut parser = Parser::new(mock_lexer);

            assert!(parser.parse_break_statement().is_err());
        }
    }

    #[test]
    fn parse_break_statement() {
        let tokens = vec![
            // break;
            create_token(TokenCategory::Break, TokenValue::Null),
            create_token(TokenCategory::Semicolon, TokenValue::Null),
            create_token(TokenCategory::ETX, TokenValue::Null),
        ];

        let mock_lexer = LexerMock::new(tokens);
        let mut parser = Parser::new(mock_lexer);

        let node = parser.parse_break_statement().unwrap();
        assert!(node.value == Statement::Break);
    }

    #[test]
    fn parse_arguments_comma_end() {
        let tokens = vec![
            // 1,
            create_token(TokenCategory::I64Value, TokenValue::I64(1)),
            create_token(TokenCategory::Comma, TokenValue::Null),
            create_token(TokenCategory::ETX, TokenValue::Null),
        ];

        let mock_lexer = LexerMock::new(tokens);
        let mut parser = Parser::new(mock_lexer);

        assert!(parser.parse_arguments().is_err());
    }

    #[test]
    fn parse_arguments() {
        let token_series = vec![
            vec![
                create_token(TokenCategory::ParenClose, TokenValue::Null),
                create_token(TokenCategory::ETX, TokenValue::Null),
            ],
            vec![
                // 1
                create_token(TokenCategory::I64Value, TokenValue::I64(1)),
                create_token(TokenCategory::ETX, TokenValue::Null),
            ],
            vec![
                // 1, 2
                create_token(TokenCategory::Reference, TokenValue::Null),
                create_token(TokenCategory::I64Value, TokenValue::I64(1)),
                create_token(TokenCategory::Comma, TokenValue::Null),
                create_token(TokenCategory::I64Value, TokenValue::I64(2)),
                create_token(TokenCategory::ETX, TokenValue::Null),
            ],
        ];

        let expected = [
            vec![],
            vec![Node {
                value: Argument {
                    value: Expression::Literal(Literal::I64(1)),
                    passed_by: ArgumentPassedBy::Value,
                },
                position: default_position(),
            }],
            vec![
                Node {
                    value: Argument {
                        value: Expression::Literal(Literal::I64(1)),
                        passed_by: ArgumentPassedBy::Reference,
                    },
                    position: default_position(),
                },
                Node {
                    value: Argument {
                        value: Expression::Literal(Literal::I64(2)),
                        passed_by: ArgumentPassedBy::Value,
                    },
                    position: default_position(),
                },
            ],
        ];

        for (idx, series) in token_series.iter().enumerate() {
            let mock_lexer = LexerMock::new(series.to_vec());
            let mut parser = Parser::new(mock_lexer);

            let vector = parser.parse_arguments().unwrap();
            assert!(vector == expected[idx]);
        }
    }

    #[test]
    fn parse_argument() {
        let token_series = vec![
            vec![
                // 1
                create_token(TokenCategory::I64Value, TokenValue::I64(1)),
                create_token(TokenCategory::ETX, TokenValue::Null),
            ],
            vec![
                // &x
                create_token(TokenCategory::Reference, TokenValue::Null),
                create_token(
                    TokenCategory::Identifier,
                    TokenValue::String("x".to_owned()),
                ),
                create_token(TokenCategory::ETX, TokenValue::Null),
            ],
        ];

        let expected = [
            Argument {
                value: Expression::Literal(Literal::I64(1)),
                passed_by: ArgumentPassedBy::Value,
            },
            Argument {
                value: Expression::Variable(Identifier("x".to_owned())),
                passed_by: ArgumentPassedBy::Reference,
            },
        ];

        for (idx, series) in token_series.iter().enumerate() {
            let mock_lexer = LexerMock::new(series.to_vec());
            let mut parser = Parser::new(mock_lexer);

            let node = parser.parse_argument().unwrap();
            assert!(node.value == expected[idx]);
        }
    }

    #[test]
    fn parse_expression() {
        let tokens = vec![
            // a || b || c
            create_token(
                TokenCategory::Identifier,
                TokenValue::String("a".to_owned()),
            ),
            create_token(TokenCategory::Or, TokenValue::Null),
            create_token(
                TokenCategory::Identifier,
                TokenValue::String("b".to_owned()),
            ),
            create_token(TokenCategory::Or, TokenValue::Null),
            create_token(
                TokenCategory::Identifier,
                TokenValue::String("c".to_owned()),
            ),
            create_token(TokenCategory::ETX, TokenValue::Null),
        ];

        let mock_lexer = LexerMock::new(tokens);
        let mut parser = Parser::new(mock_lexer);

        let node = parser.parse_expression().unwrap();
        assert!(
            node.value
                == Expression::Alternative(
                    Box::new(Node {
                        value: Expression::Alternative(
                            Box::new(Node {
                                value: Expression::Variable(Identifier("a".to_owned())),
                                position: default_position()
                            }),
                            Box::new(Node {
                                value: Expression::Variable(Identifier("b".to_owned())),
                                position: default_position()
                            })
                        ),
                        position: default_position()
                    }),
                    Box::new(Node {
                        value: Expression::Variable(Identifier("c".to_owned())),
                        position: default_position()
                    })
                )
        );
    }

    #[test]
    fn parse_concatenation_term() {
        let tokens = vec![
            // a && b && c
            create_token(
                TokenCategory::Identifier,
                TokenValue::String("a".to_owned()),
            ),
            create_token(TokenCategory::And, TokenValue::Null),
            create_token(
                TokenCategory::Identifier,
                TokenValue::String("b".to_owned()),
            ),
            create_token(TokenCategory::And, TokenValue::Null),
            create_token(
                TokenCategory::Identifier,
                TokenValue::String("c".to_owned()),
            ),
            create_token(TokenCategory::ETX, TokenValue::Null),
        ];

        let mock_lexer = LexerMock::new(tokens);
        let mut parser = Parser::new(mock_lexer);

        let node = parser.parse_concatenation_term().unwrap();
        assert!(
            node.value
                == Expression::Concatenation(
                    Box::new(Node {
                        value: Expression::Concatenation(
                            Box::new(Node {
                                value: Expression::Variable(Identifier("a".to_owned())),
                                position: default_position()
                            }),
                            Box::new(Node {
                                value: Expression::Variable(Identifier("b".to_owned())),
                                position: default_position()
                            })
                        ),
                        position: default_position()
                    }),
                    Box::new(Node {
                        value: Expression::Variable(Identifier("c".to_owned())),
                        position: default_position()
                    })
                )
        );
    }

    #[test]
    fn parse_relation_term() {
        let token_series = vec![
            vec![
                // 1 == 2
                create_token(TokenCategory::I64Value, TokenValue::I64(1)),
                create_token(TokenCategory::Equal, TokenValue::Null),
                create_token(TokenCategory::I64Value, TokenValue::I64(2)),
                create_token(TokenCategory::ETX, TokenValue::Null),
            ],
            vec![
                // 1 != 2
                create_token(TokenCategory::I64Value, TokenValue::I64(1)),
                create_token(TokenCategory::NotEqual, TokenValue::Null),
                create_token(TokenCategory::I64Value, TokenValue::I64(2)),
                create_token(TokenCategory::ETX, TokenValue::Null),
            ],
            vec![
                // 1 > 2
                create_token(TokenCategory::I64Value, TokenValue::I64(1)),
                create_token(TokenCategory::Greater, TokenValue::Null),
                create_token(TokenCategory::I64Value, TokenValue::I64(2)),
                create_token(TokenCategory::ETX, TokenValue::Null),
            ],
            vec![
                // 1 >= 2
                create_token(TokenCategory::I64Value, TokenValue::I64(1)),
                create_token(TokenCategory::GreaterOrEqual, TokenValue::Null),
                create_token(TokenCategory::I64Value, TokenValue::I64(2)),
                create_token(TokenCategory::ETX, TokenValue::Null),
            ],
            vec![
                // 1 < 2
                create_token(TokenCategory::I64Value, TokenValue::I64(1)),
                create_token(TokenCategory::Less, TokenValue::Null),
                create_token(TokenCategory::I64Value, TokenValue::I64(2)),
                create_token(TokenCategory::ETX, TokenValue::Null),
            ],
            vec![
                // 1 <= 2
                create_token(TokenCategory::I64Value, TokenValue::I64(1)),
                create_token(TokenCategory::LessOrEqual, TokenValue::Null),
                create_token(TokenCategory::I64Value, TokenValue::I64(2)),
                create_token(TokenCategory::ETX, TokenValue::Null),
            ],
            vec![
                // 1
                create_token(TokenCategory::I64Value, TokenValue::I64(1)),
                create_token(TokenCategory::ETX, TokenValue::Null),
            ],
        ];

        let expected = [
            Expression::Equal(
                Box::new(Node {
                    value: Expression::Literal(Literal::I64(1)),
                    position: default_position(),
                }),
                Box::new(Node {
                    value: Expression::Literal(Literal::I64(2)),
                    position: default_position(),
                }),
            ),
            Expression::NotEqual(
                Box::new(Node {
                    value: Expression::Literal(Literal::I64(1)),
                    position: default_position(),
                }),
                Box::new(Node {
                    value: Expression::Literal(Literal::I64(2)),
                    position: default_position(),
                }),
            ),
            Expression::Greater(
                Box::new(Node {
                    value: Expression::Literal(Literal::I64(1)),
                    position: default_position(),
                }),
                Box::new(Node {
                    value: Expression::Literal(Literal::I64(2)),
                    position: default_position(),
                }),
            ),
            Expression::GreaterEqual(
                Box::new(Node {
                    value: Expression::Literal(Literal::I64(1)),
                    position: default_position(),
                }),
                Box::new(Node {
                    value: Expression::Literal(Literal::I64(2)),
                    position: default_position(),
                }),
            ),
            Expression::Less(
                Box::new(Node {
                    value: Expression::Literal(Literal::I64(1)),
                    position: default_position(),
                }),
                Box::new(Node {
                    value: Expression::Literal(Literal::I64(2)),
                    position: default_position(),
                }),
            ),
            Expression::LessEqual(
                Box::new(Node {
                    value: Expression::Literal(Literal::I64(1)),
                    position: default_position(),
                }),
                Box::new(Node {
                    value: Expression::Literal(Literal::I64(2)),
                    position: default_position(),
                }),
            ),
            Expression::Literal(Literal::I64(1)),
        ];

        for (idx, series) in token_series.iter().enumerate() {
            let mock_lexer = LexerMock::new(series.to_vec());
            let mut parser = Parser::new(mock_lexer);

            let node = parser.parse_relation_term().unwrap();
            assert!(node.value == expected[idx]);
        }
    }

    #[test]
    fn parse_additive_term() {
        // 5 + 2.0 - x
        let tokens = vec![
            create_token(TokenCategory::I64Value, TokenValue::I64(5)),
            create_token(TokenCategory::Plus, TokenValue::Null),
            create_token(TokenCategory::F64Value, TokenValue::F64(2.0)),
            create_token(TokenCategory::Minus, TokenValue::Null),
            create_token(
                TokenCategory::Identifier,
                TokenValue::String("x".to_owned()),
            ),
            create_token(TokenCategory::ETX, TokenValue::Null),
        ];

        let mock_lexer = LexerMock::new(tokens);
        let mut parser = Parser::new(mock_lexer);

        let node = parser.parse_additive_term().unwrap();
        assert!(
            node.value
                == Expression::Subtraction(
                    Box::new(Node {
                        value: Expression::Addition(
                            Box::new(Node {
                                value: Expression::Literal(Literal::I64(5)),
                                position: default_position()
                            }),
                            Box::new(Node {
                                value: Expression::Literal(Literal::F64(2.0)),
                                position: default_position()
                            })
                        ),
                        position: default_position()
                    }),
                    Box::new(Node {
                        value: Expression::Variable(Identifier("x".to_owned())),
                        position: default_position()
                    })
                )
        )
    }

    #[test]
    fn parse_multiplicative_term() {
        let tokens = vec![
            // 5 * 2.0 / x
            create_token(TokenCategory::I64Value, TokenValue::I64(5)),
            create_token(TokenCategory::Multiply, TokenValue::Null),
            create_token(TokenCategory::F64Value, TokenValue::F64(2.0)),
            create_token(TokenCategory::Divide, TokenValue::Null),
            create_token(
                TokenCategory::Identifier,
                TokenValue::String("x".to_owned()),
            ),
            create_token(TokenCategory::ETX, TokenValue::Null),
        ];

        let mock_lexer = LexerMock::new(tokens);
        let mut parser = Parser::new(mock_lexer);

        let node = parser.parse_multiplicative_term().unwrap();
        assert!(
            node.value
                == Expression::Division(
                    Box::new(Node {
                        value: Expression::Multiplication(
                            Box::new(Node {
                                value: Expression::Literal(Literal::I64(5)),
                                position: default_position()
                            }),
                            Box::new(Node {
                                value: Expression::Literal(Literal::F64(2.0)),
                                position: default_position()
                            })
                        ),
                        position: default_position()
                    }),
                    Box::new(Node {
                        value: Expression::Variable(Identifier("x".to_owned())),
                        position: default_position()
                    })
                )
        )
    }

    #[test]
    fn parse_casted_term() {
        let token_series = vec![
            vec![
                // 5 as str
                create_token(TokenCategory::I64Value, TokenValue::I64(5)),
                create_token(TokenCategory::As, TokenValue::Null),
                create_token(TokenCategory::String, TokenValue::Null),
                create_token(TokenCategory::ETX, TokenValue::Null),
            ],
            vec![
                // 5
                create_token(TokenCategory::I64Value, TokenValue::I64(5)),
                create_token(TokenCategory::ETX, TokenValue::Null),
            ],
        ];

        let expected = vec![
            Expression::Casting {
                value: Box::new(Node {
                    value: Expression::Literal(Literal::I64(5)),
                    position: default_position(),
                }),
                to_type: Node {
                    value: Type::Str,
                    position: default_position(),
                },
            },
            Expression::Literal(Literal::I64(5)),
        ];

        for (idx, series) in token_series.iter().enumerate() {
            let mock_lexer = LexerMock::new(series.to_vec());
            let mut parser = Parser::new(mock_lexer);

            let node = parser.parse_casted_term().unwrap();
            assert!(node.value == expected[idx]);
        }
    }

    #[test]
    fn parse_unary_term() {
        let token_series = vec![
            vec![
                // !True
                create_token(TokenCategory::Negate, TokenValue::Null),
                create_token(TokenCategory::True, TokenValue::Null),
                create_token(TokenCategory::ETX, TokenValue::Null),
            ],
            vec![
                // -5
                create_token(TokenCategory::Minus, TokenValue::Null),
                create_token(TokenCategory::I64Value, TokenValue::I64(5)),
                create_token(TokenCategory::ETX, TokenValue::Null),
            ],
            vec![
                // 5
                create_token(TokenCategory::I64Value, TokenValue::I64(5)),
                create_token(TokenCategory::ETX, TokenValue::Null),
            ],
        ];

        let expected = [
            Expression::BooleanNegation(Box::new(Node {
                value: Expression::Literal(Literal::True),
                position: default_position(),
            })),
            Expression::ArithmeticNegation(Box::new(Node {
                value: Expression::Literal(Literal::I64(5)),
                position: default_position(),
            })),
            Expression::Literal(Literal::I64(5)),
        ];

        for (idx, series) in token_series.iter().enumerate() {
            let mock_lexer = LexerMock::new(series.to_vec());
            let mut parser = Parser::new(mock_lexer);

            let node = parser.parse_unary_term().unwrap();
            assert!(node.value == expected[idx]);
        }
    }

    #[test]
    fn parse_factor() {
        let token_series = vec![
            // (5 + 2)
            vec![
                create_token(TokenCategory::ParenOpen, TokenValue::Null),
                create_token(TokenCategory::I64Value, TokenValue::I64(5)),
                create_token(TokenCategory::Plus, TokenValue::Null),
                create_token(TokenCategory::I64Value, TokenValue::I64(2)),
                create_token(TokenCategory::ParenClose, TokenValue::Null),
                create_token(TokenCategory::ETX, TokenValue::Null),
            ],
            vec![
                // 5
                create_token(TokenCategory::I64Value, TokenValue::I64(5)),
                create_token(TokenCategory::ETX, TokenValue::Null),
            ],
            vec![
                // print
                create_token(
                    TokenCategory::Identifier,
                    TokenValue::String("print".to_owned()),
                ),
                create_token(TokenCategory::ETX, TokenValue::Null),
            ],
        ];

        let expected = vec![
            Expression::Addition(
                Box::new(Node {
                    value: Expression::Literal(Literal::I64(5)),
                    position: default_position(),
                }),
                Box::new(Node {
                    value: Expression::Literal(Literal::I64(2)),
                    position: default_position(),
                }),
            ),
            Expression::Literal(Literal::I64(5)),
            Expression::Variable(Identifier("print".to_owned())),
        ];

        for (idx, series) in token_series.iter().enumerate() {
            let mock_lexer = LexerMock::new(series.to_vec());
            let mut parser = Parser::new(mock_lexer);

            let node = parser.parse_factor().unwrap();
            assert!(node.value == expected[idx]);
        }
    }

    #[test]
    fn parse_factor_nested_expression_unclosed() {
        let tokens = vec![
            // (5 + 2
            create_token(TokenCategory::ParenOpen, TokenValue::Null),
            create_token(TokenCategory::I64Value, TokenValue::I64(5)),
            create_token(TokenCategory::Plus, TokenValue::Null),
            create_token(TokenCategory::I64Value, TokenValue::I64(2)),
            create_token(TokenCategory::ETX, TokenValue::Null),
        ];

        let mock_lexer = LexerMock::new(tokens);
        let mut parser = Parser::new(mock_lexer);

        assert!(parser.parse_factor().is_err());
    }

    #[test]
    fn parse_identifier_or_call_fail() {
        let token_series = vec![
            vec![
                // print(5,)
                create_token(
                    TokenCategory::Identifier,
                    TokenValue::String("print".to_owned()),
                ),
                create_token(TokenCategory::ParenOpen, TokenValue::Null),
                create_token(TokenCategory::I64Value, TokenValue::I64(5)),
                create_token(TokenCategory::Comma, TokenValue::Null),
                create_token(TokenCategory::ParenClose, TokenValue::Null),
                create_token(TokenCategory::ETX, TokenValue::Null),
            ],
            vec![
                create_token(
                    // print(
                    TokenCategory::Identifier,
                    TokenValue::String("print".to_owned()),
                ),
                create_token(TokenCategory::ParenOpen, TokenValue::Null),
                create_token(TokenCategory::ETX, TokenValue::Null),
            ],
        ];

        for series in token_series {
            let mock_lexer = LexerMock::new(series);
            let mut parser = Parser::new(mock_lexer);

            assert!(parser.parse_identifier_or_call().is_err());
        }
    }

    #[test]
    fn parse_identifier_or_call() {
        let token_series = vec![
            vec![
                // print
                create_token(
                    TokenCategory::Identifier,
                    TokenValue::String("print".to_owned()),
                ),
                create_token(TokenCategory::ETX, TokenValue::Null),
            ],
            vec![
                // print()
                create_token(
                    TokenCategory::Identifier,
                    TokenValue::String("print".to_owned()),
                ),
                create_token(TokenCategory::ParenOpen, TokenValue::Null),
                create_token(TokenCategory::ParenClose, TokenValue::Null),
                create_token(TokenCategory::ETX, TokenValue::Null),
            ],
            vec![
                // print(5)
                create_token(
                    TokenCategory::Identifier,
                    TokenValue::String("print".to_owned()),
                ),
                create_token(TokenCategory::ParenOpen, TokenValue::Null),
                create_token(TokenCategory::I64Value, TokenValue::I64(5)),
                create_token(TokenCategory::ParenClose, TokenValue::Null),
                create_token(TokenCategory::ETX, TokenValue::Null),
            ],
            vec![
                // print(5, x)
                create_token(
                    TokenCategory::Identifier,
                    TokenValue::String("print".to_owned()),
                ),
                create_token(TokenCategory::ParenOpen, TokenValue::Null),
                create_token(TokenCategory::Reference, TokenValue::Null),
                create_token(TokenCategory::I64Value, TokenValue::I64(5)),
                create_token(TokenCategory::Comma, TokenValue::Null),
                create_token(
                    TokenCategory::Identifier,
                    TokenValue::String("x".to_owned()),
                ),
                create_token(TokenCategory::ParenClose, TokenValue::Null),
                create_token(TokenCategory::ETX, TokenValue::Null),
            ],
        ];

        let expected = [
            Expression::Variable(Identifier("print".to_owned())),
            Expression::FunctionCall {
                identifier: Identifier("print".to_owned()),
                arguments: vec![],
            },
            Expression::FunctionCall {
                identifier: Identifier("print".to_owned()),
                arguments: vec![Box::new(Node {
                    value: Argument {
                        value: Expression::Literal(Literal::I64(5)),
                        passed_by: ArgumentPassedBy::Value,
                    },
                    position: default_position(),
                })],
            },
            Expression::FunctionCall {
                identifier: Identifier("print".to_owned()),
                arguments: vec![
                    Box::new(Node {
                        value: Argument {
                            value: Expression::Literal(Literal::I64(5)),
                            passed_by: ArgumentPassedBy::Reference,
                        },
                        position: default_position(),
                    }),
                    Box::new(Node {
                        value: Argument {
                            value: Expression::Variable(Identifier("x".to_owned())),
                            passed_by: ArgumentPassedBy::Value,
                        },
                        position: default_position(),
                    }),
                ],
            },
        ];

        for (idx, series) in token_series.iter().enumerate() {
            let mock_lexer = LexerMock::new(series.to_vec());
            let mut parser = Parser::new(mock_lexer);

            let node = parser.parse_identifier_or_call().unwrap();
            assert!(node.value == expected[idx]);
        }
    }

    #[test]
    fn parse_switch_statement() {
        let token_series = vec![vec![
            // switch(x) {
            //      (true) -> {}
            // }
            create_token(TokenCategory::Switch, TokenValue::Null),
            create_token(TokenCategory::ParenOpen, TokenValue::Null),
            create_token(
                TokenCategory::Identifier,
                TokenValue::String("x".to_owned()),
            ),
            create_token(TokenCategory::ParenClose, TokenValue::Null),
            create_token(TokenCategory::BraceOpen, TokenValue::Null),
            create_token(TokenCategory::ParenOpen, TokenValue::Null),
            create_token(TokenCategory::True, TokenValue::Null),
            create_token(TokenCategory::ParenClose, TokenValue::Null),
            create_token(TokenCategory::Arrow, TokenValue::Null),
            create_token(TokenCategory::BraceOpen, TokenValue::Null),
            create_token(TokenCategory::BraceClose, TokenValue::Null),
            create_token(TokenCategory::BraceClose, TokenValue::Null),
            create_token(TokenCategory::ETX, TokenValue::Null),
        ]];

        let expected_types = [Statement::Switch {
            expressions: vec![Node {
                value: SwitchExpression {
                    expression: Node {
                        value: Expression::Variable(Identifier("x".to_owned())),
                        position: default_position(),
                    },
                    alias: None,
                },
                position: default_position(),
            }],
            cases: vec![Node {
                value: SwitchCase {
                    condition: Node {
                        value: Expression::Literal(Literal::True),
                        position: default_position(),
                    },
                    block: Node {
                        value: Block(vec![]),
                        position: default_position(),
                    },
                },
                position: default_position(),
            }],
        }];

        for (idx, series) in token_series.iter().enumerate() {
            let mock_lexer = LexerMock::new(series.to_vec());
            let mut parser = Parser::new(mock_lexer);

            let node = parser.parse_switch_statement().unwrap();
            assert!(node.value == expected_types[idx]);
        }
    }

    #[test]
    fn parse_switch_expressions_fail() {
        let token_series = vec![vec![
            // x: temp,
            create_token(
                TokenCategory::Identifier,
                TokenValue::String("x".to_owned()),
            ),
            create_token(TokenCategory::Colon, TokenValue::Null),
            create_token(
                TokenCategory::Identifier,
                TokenValue::String("temp".to_owned()),
            ),
            create_token(TokenCategory::Comma, TokenValue::Null),
            create_token(TokenCategory::ETX, TokenValue::Null),
        ]];

        for series in token_series {
            let mock_lexer = LexerMock::new(series);
            let mut parser = Parser::new(mock_lexer);

            assert!(parser.parse_switch_expressions().is_err());
        }
    }

    #[test]
    fn parse_switch_expressions() {
        let token_series = vec![
            vec![
                // x: temp, y
                create_token(
                    TokenCategory::Identifier,
                    TokenValue::String("x".to_owned()),
                ),
                create_token(TokenCategory::Colon, TokenValue::Null),
                create_token(
                    TokenCategory::Identifier,
                    TokenValue::String("temp".to_owned()),
                ),
                create_token(TokenCategory::Comma, TokenValue::Null),
                create_token(
                    TokenCategory::Identifier,
                    TokenValue::String("y".to_owned()),
                ),
                create_token(TokenCategory::ETX, TokenValue::Null),
            ],
            vec![
                // x
                create_token(
                    TokenCategory::Identifier,
                    TokenValue::String("x".to_owned()),
                ),
                create_token(TokenCategory::ETX, TokenValue::Null),
            ],
        ];

        let expected_types = [
            vec![
                Node {
                    value: SwitchExpression {
                        expression: Node {
                            value: Expression::Variable(Identifier("x".to_owned())),
                            position: default_position(),
                        },
                        alias: Some(Node {
                            value: Identifier("temp".to_owned()),
                            position: default_position(),
                        }),
                    },
                    position: default_position(),
                },
                Node {
                    value: SwitchExpression {
                        expression: Node {
                            value: Expression::Variable(Identifier("y".to_owned())),
                            position: default_position(),
                        },
                        alias: None,
                    },
                    position: default_position(),
                },
            ],
            vec![Node {
                value: SwitchExpression {
                    expression: Node {
                        value: Expression::Variable(Identifier("x".to_owned())),
                        position: default_position(),
                    },
                    alias: None,
                },
                position: default_position(),
            }],
        ];

        for (idx, series) in token_series.iter().enumerate() {
            let mock_lexer = LexerMock::new(series.to_vec());
            let mut parser = Parser::new(mock_lexer);

            let vector = parser.parse_switch_expressions().unwrap();
            assert!(vector == expected_types[idx]);
        }
    }

    #[test]
    fn parse_switch_expression() {
        let token_series = vec![
            vec![
                // x: temp
                create_token(
                    TokenCategory::Identifier,
                    TokenValue::String("x".to_owned()),
                ),
                create_token(TokenCategory::Colon, TokenValue::Null),
                create_token(
                    TokenCategory::Identifier,
                    TokenValue::String("temp".to_owned()),
                ),
                create_token(TokenCategory::ETX, TokenValue::Null),
            ],
            vec![
                // x
                create_token(
                    TokenCategory::Identifier,
                    TokenValue::String("x".to_owned()),
                ),
                create_token(TokenCategory::ETX, TokenValue::Null),
            ],
        ];

        let expected_types = [
            SwitchExpression {
                expression: Node {
                    value: Expression::Variable(Identifier("x".to_owned())),
                    position: default_position(),
                },
                alias: Some(Node {
                    value: Identifier("temp".to_owned()),
                    position: default_position(),
                }),
            },
            SwitchExpression {
                expression: Node {
                    value: Expression::Variable(Identifier("x".to_owned())),
                    position: default_position(),
                },
                alias: None,
            },
        ];

        for (idx, series) in token_series.iter().enumerate() {
            let mock_lexer = LexerMock::new(series.to_vec());
            let mut parser = Parser::new(mock_lexer);

            let node = parser.parse_switch_expression().unwrap();
            assert!(node.value == expected_types[idx]);
        }
    }

    #[test]
    fn parse_switch_case() {
        let token_series = vec![vec![
            // (true) -> {}
            create_token(TokenCategory::ParenOpen, TokenValue::Null),
            create_token(TokenCategory::True, TokenValue::Null),
            create_token(TokenCategory::ParenClose, TokenValue::Null),
            create_token(TokenCategory::Arrow, TokenValue::Null),
            create_token(TokenCategory::BraceOpen, TokenValue::Null),
            create_token(TokenCategory::BraceClose, TokenValue::Null),
            create_token(TokenCategory::ETX, TokenValue::Null),
        ]];

        let expected_types = [SwitchCase {
            condition: Node {
                value: Expression::Literal(Literal::True),
                position: default_position(),
            },
            block: Node {
                value: Block(vec![]),
                position: default_position(),
            },
        }];

        for (idx, series) in token_series.iter().enumerate() {
            let mock_lexer = LexerMock::new(series.to_vec());
            let mut parser = Parser::new(mock_lexer);

            let node = parser.parse_switch_case().unwrap();
            assert!(node.value == expected_types[idx]);
        }
    }

    #[test]
    fn parse_type() {
        let token_series = vec![
            vec![
                create_token(TokenCategory::I64, TokenValue::Null),
                create_token(TokenCategory::ETX, TokenValue::Null),
            ],
            vec![
                create_token(TokenCategory::F64, TokenValue::Null),
                create_token(TokenCategory::ETX, TokenValue::Null),
            ],
            vec![
                create_token(TokenCategory::String, TokenValue::Null),
                create_token(TokenCategory::ETX, TokenValue::Null),
            ],
            vec![
                create_token(TokenCategory::Bool, TokenValue::Null),
                create_token(TokenCategory::ETX, TokenValue::Null),
            ],
        ];

        let expected_types = [Type::I64, Type::F64, Type::Str, Type::Bool];

        for (idx, series) in token_series.iter().enumerate() {
            let mock_lexer = LexerMock::new(series.to_vec());
            let mut parser = Parser::new(mock_lexer);

            let node = parser.parse_type().unwrap();
            assert!(node.value == expected_types[idx]);
        }
    }

    #[test]
    fn parse_type_fail() {
        let token_series = vec![
            vec![
                create_token(TokenCategory::Void, TokenValue::Null),
                create_token(TokenCategory::ETX, TokenValue::Null),
            ],
            vec![
                create_token(TokenCategory::Comma, TokenValue::Null),
                create_token(TokenCategory::ETX, TokenValue::Null),
            ],
        ];

        for series in token_series {
            let mock_lexer = LexerMock::new(series);
            let mut parser = Parser::new(mock_lexer);

            assert!(parser.parse_type().is_err());
        }
    }

    #[test]
    fn parse_literals() {
        let tokens = vec![
            create_token(TokenCategory::True, TokenValue::Null),
            create_token(TokenCategory::False, TokenValue::Null),
            create_token(
                TokenCategory::StringValue,
                TokenValue::String("a".to_owned()),
            ),
            create_token(TokenCategory::I64Value, TokenValue::I64(5)),
            create_token(TokenCategory::F64Value, TokenValue::F64(5.0)),
            create_token(TokenCategory::ETX, TokenValue::Null),
        ];

        let mock_lexer = LexerMock::new(tokens);
        let mut parser = Parser::new(mock_lexer);

        let mut literal = parser.parse_literal().unwrap();
        assert!(literal.value == Literal::True);

        literal = parser.parse_literal().unwrap();
        assert!(literal.value == Literal::False);

        literal = parser.parse_literal().unwrap();
        assert!(literal.value == Literal::String("a".to_owned()));

        literal = parser.parse_literal().unwrap();
        assert!(literal.value == Literal::I64(5));

        literal = parser.parse_literal().unwrap();
        assert!(literal.value == Literal::F64(5.0));
    }

    #[test]
    fn parse_literals_bad_value_types() {
        let token_series = vec![
            vec![
                create_token(TokenCategory::StringValue, TokenValue::I64(5)),
                create_token(TokenCategory::ETX, TokenValue::Null),
            ],
            vec![
                create_token(TokenCategory::I64Value, TokenValue::F64(5.0)),
                create_token(TokenCategory::ETX, TokenValue::Null),
            ],
            vec![
                create_token(TokenCategory::F64Value, TokenValue::I64(5)),
                create_token(TokenCategory::ETX, TokenValue::Null),
            ],
        ];

        for series in token_series {
            let mock_lexer = LexerMock::new(series);
            let mut parser = Parser::new(mock_lexer);

            let result = parser.parse_literal();
            assert!(result.is_err());
        }
    }

    #[test]
    fn parse_identifier() {
        let tokens = vec![
            create_token(
                TokenCategory::Identifier,
                TokenValue::String("print".to_owned()),
            ),
            create_token(TokenCategory::ETX, TokenValue::Null),
        ];

        let mock_lexer = LexerMock::new(tokens);
        let mut parser = Parser::new(mock_lexer);

        let node = parser.parse_identifier().unwrap();
        assert!(node.value == Identifier("print".to_owned()));
    }

    #[test]
    fn parse_identifier_bad_value_type() {
        let tokens = vec![
            // 5 is not string
            create_token(TokenCategory::Identifier, TokenValue::I64(5)),
            create_token(TokenCategory::ETX, TokenValue::Null),
        ];

        let mock_lexer = LexerMock::new(tokens);
        let mut parser = Parser::new(mock_lexer);

        let result = parser.parse_identifier();
        assert!(result.is_err());
    }

    #[test]
    fn consume_must_be() {
        let tokens = vec![
            create_token(TokenCategory::ParenOpen, TokenValue::Null),
            create_token(TokenCategory::ETX, TokenValue::Null),
        ];

        let mock_lexer = LexerMock::new(tokens);
        let mut parser = Parser::new(mock_lexer);
        assert!(parser.current_token().clone().category == TokenCategory::ParenOpen);
        let _ = parser.consume_must_be(TokenCategory::ParenOpen).unwrap();

        assert!(parser.current_token().clone().category == TokenCategory::ETX);
    }

    #[test]
    fn consume_must_be_fail() {
        let tokens = vec![
            create_token(TokenCategory::ParenOpen, TokenValue::Null),
            create_token(TokenCategory::ETX, TokenValue::Null),
        ];

        let mock_lexer = LexerMock::new(tokens);
        let mut parser = Parser::new(mock_lexer);
        assert!(parser.current_token().clone().category == TokenCategory::ParenOpen);
        let result = parser.consume_must_be(TokenCategory::Semicolon);

        assert!(result.is_err());
        assert!(parser.current_token().clone().category == TokenCategory::ParenOpen);
    }

    #[test]
    fn consume_if_matches() {
        let tokens = vec![
            create_token(TokenCategory::ParenOpen, TokenValue::Null),
            create_token(TokenCategory::ETX, TokenValue::Null),
        ];

        let mock_lexer = LexerMock::new(tokens);
        let mut parser = Parser::new(mock_lexer);
        assert!(parser.current_token().clone().category == TokenCategory::ParenOpen);
        let _ = parser.consume_if_matches(TokenCategory::ParenOpen).unwrap();

        assert!(parser.current_token().clone().category == TokenCategory::ETX);
    }

    #[test]
    fn consume_if_matches_fail() {
        let tokens = vec![
            create_token(TokenCategory::ParenOpen, TokenValue::Null),
            create_token(TokenCategory::ETX, TokenValue::Null),
        ];

        let mock_lexer = LexerMock::new(tokens);
        let mut parser = Parser::new(mock_lexer);
        assert!(parser.current_token().clone().category == TokenCategory::ParenOpen);
        let result = parser.consume_if_matches(TokenCategory::Semicolon);

        assert!(result.unwrap().is_none());
        assert!(parser.current_token().clone().category == TokenCategory::ParenOpen);
    }
}
