use crate::{
    ast::{
        Argument, ArgumentPassedBy, Block, Expression, Identifier, Literal, Node, Parameter,
        ParameterPassedBy, Program, Statement, SwitchCase, SwitchExpression, Type,
    },
    lexer::ILexer,
    tokens::{Token, TokenCategory, TokenValue},
};

pub struct Parser<L: ILexer> {
    lexer: L,
}

#[allow(dead_code)]
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
    fn parse(&mut self) -> Result<Program, ParserIssue>;
}

impl<L: ILexer> IParser<L> for Parser<L> {
    fn new(lexer: L) -> Parser<L> {
        Parser { lexer }
    }

    fn parse(&mut self) -> Result<Program, ParserIssue> {
        let _ = self.next_token(); // initialize
        let _ = self.next_token(); // skip STX

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
                    return Err(Self::create_parser_error(format!(
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
    fn next_token(&mut self) -> Option<Token> {
        let mut current_token = self.lexer.next().clone().unwrap();
        while current_token.category == TokenCategory::Comment {
            current_token = self.lexer.next().unwrap();
        }
        Some(current_token)
    }

    fn current_token(&self) -> Token {
        self.lexer.current().clone().unwrap()
    }

    fn parse_function_declaration(&mut self) -> Result<Node<Statement>, ParserIssue> {
        let fn_token = self.consume_must_be(TokenCategory::Fn)?;
        let identifier = self.parse_identifier()?;
        let _ = self.consume_must_be(TokenCategory::ParenOpen)?;
        let parameters = self.parse_parameters()?;
        let _ = self.consume_must_be(TokenCategory::ParenClose)?;
        let _ = self.consume_must_be(TokenCategory::Colon)?;
        let return_type = match self.parse_type() {
            Ok(node) => Ok(node),
            Err(_) => match self.consume_if_matches(TokenCategory::Void) {
                Some(token) => Ok(Node {
                    value: Type::Void,
                    position: token.position,
                }),
                None => Err(Self::create_parser_error("Bad return type".to_owned())),
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

    fn parse_parameters(&mut self) -> Result<Vec<Node<Parameter>>, ParserIssue> {
        if self.current_token().category == TokenCategory::ParenClose {
            return Ok(vec![]);
        }

        let expression = self.parse_parameter()?;

        let mut parameters = vec![expression];
        while let Some(_) = self.consume_if_matches(TokenCategory::Comma) {
            let parameter = self.parse_parameter()?;
            parameters.push(parameter);
        }
        Ok(parameters)
    }

    fn parse_parameter(&mut self) -> Result<Node<Parameter>, ParserIssue> {
        let position = self.current_token().position;
        let passed_by = match self.consume_if_matches(TokenCategory::Reference) {
            Some(_) => ParameterPassedBy::Reference,
            None => ParameterPassedBy::Value,
        };
        let parameter_type = self.parse_type()?;
        let identifier = self.parse_identifier()?;
        let value = match self.consume_if_matches(TokenCategory::Assign) {
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

    fn parse_for_statement(&mut self) -> Result<Node<Statement>, ParserIssue> {
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

    fn parse_if_statement(&mut self) -> Result<Node<Statement>, ParserIssue> {
        let if_token = self.consume_must_be(TokenCategory::If)?;
        let _ = self.consume_must_be(TokenCategory::ParenOpen)?;
        let condition = self.parse_expression()?;
        let _ = self.consume_must_be(TokenCategory::ParenClose)?;
        let true_block = self.parse_statement_block()?;

        let false_block = match self.consume_if_matches(TokenCategory::Else) {
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
        let position = self.consume_must_be(TokenCategory::BraceOpen)?.position;
        let mut statements: Vec<Node<Statement>> = vec![];
        while self.consume_if_matches(TokenCategory::BraceClose).is_none() {
            let statement = self.parse_statement()?;
            statements.push(statement);
        }
        Ok(Node {
            value: Block(statements),
            position,
        })
    }

    fn parse_statement(&mut self) -> Result<Node<Statement>, ParserIssue> {
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
                return Err(Self::create_parser_error(format!(
                    "Can't create block statement starting with token: {:?}.",
                    self.current_token().category
                )));
            }
        };

        Ok(node)
    }

    fn parse_assign_or_call(&mut self) -> Result<Node<Statement>, ParserIssue> {
        let identifier = self.parse_identifier()?;
        let position = identifier.position;

        if self.consume_if_matches(TokenCategory::Assign).is_some() {
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

        if self.consume_if_matches(TokenCategory::ParenOpen).is_some() {
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

        Err(Self::create_parser_error(format!(
            "Could not create assignment or call."
        )))
    }

    fn parse_declaration(&mut self) -> Result<Node<Statement>, ParserIssue> {
        let declaration_type = self.parse_type()?;
        let position = declaration_type.position;
        let identifier = self.parse_identifier()?;
        let value = match self.consume_if_matches(TokenCategory::Assign) {
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

    fn parse_break_statement(&mut self) -> Result<Node<Statement>, ParserIssue> {
        let token = self.consume_must_be(TokenCategory::Break)?;
        let _ = self.consume_must_be(TokenCategory::Semicolon)?;
        let node = Node {
            value: Statement::Break,
            position: token.position,
        };
        Ok(node)
    }

    fn parse_arguments(&mut self) -> Result<Vec<Node<Argument>>, ParserIssue> {
        if self.current_token().category == TokenCategory::ParenClose {
            return Ok(vec![]);
        }

        let expression = self.parse_argument()?;

        let mut arguments = vec![expression];
        while let Some(_) = self.consume_if_matches(TokenCategory::Comma) {
            let argument = self.parse_argument()?;
            arguments.push(argument);
        }
        Ok(arguments)
    }

    fn parse_argument(&mut self) -> Result<Node<Argument>, ParserIssue> {
        let mut passed_by = ArgumentPassedBy::Value;
        if self.consume_if_matches(TokenCategory::Reference).is_some() {
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
        if self.consume_if_matches(TokenCategory::Or).is_some() {
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
        if self.consume_if_matches(TokenCategory::And).is_some() {
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
        if let Some(token) = self.consume_if_matches(TokenCategory::Equal) {
            let right_side = self.parse_additive_term()?;
            return Ok(Node {
                value: Expression::Equal(Box::new(left_side), Box::new(right_side)),
                position: token.position,
            });
        }
        if let Some(token) = self.consume_if_matches(TokenCategory::NotEqual) {
            let right_side = self.parse_additive_term()?;
            return Ok(Node {
                value: Expression::NotEqual(Box::new(left_side), Box::new(right_side)),
                position: token.position,
            });
        }
        if let Some(token) = self.consume_if_matches(TokenCategory::Greater) {
            let right_side = self.parse_additive_term()?;
            return Ok(Node {
                value: Expression::Greater(Box::new(left_side), Box::new(right_side)),
                position: token.position,
            });
        }
        if let Some(token) = self.consume_if_matches(TokenCategory::GreaterOrEqual) {
            let right_side = self.parse_additive_term()?;
            return Ok(Node {
                value: Expression::GreaterEqual(Box::new(left_side), Box::new(right_side)),
                position: token.position,
            });
        }
        if let Some(token) = self.consume_if_matches(TokenCategory::Less) {
            let right_side = self.parse_additive_term()?;
            return Ok(Node {
                value: Expression::Less(Box::new(left_side), Box::new(right_side)),
                position: token.position,
            });
        }
        if let Some(token) = self.consume_if_matches(TokenCategory::LessOrEqual) {
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
        let mut current_token = self.current_token();
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
            current_token = self.current_token();
        }
        Ok(left_side)
    }

    fn parse_multiplicative_term(&mut self) -> Result<Node<Expression>, ParserIssue> {
        let mut left_side = self.parse_casted_term()?;
        let mut current_token = self.current_token();
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
            current_token = self.current_token();
        }
        Ok(left_side)
    }

    fn parse_casted_term(&mut self) -> Result<Node<Expression>, ParserIssue> {
        let unary_term = self.parse_unary_term()?;
        let position = unary_term.position.clone();
        match self.consume_if_matches(TokenCategory::As) {
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

    fn parse_unary_term(&mut self) -> Result<Node<Expression>, ParserIssue> {
        if let Some(token) = self.consume_if_matches(TokenCategory::Negate) {
            let factor = self.parse_factor()?;
            return Ok(Node {
                value: Expression::BooleanNegation(Box::new(factor)),
                position: token.position,
            });
        }
        if let Some(token) = self.consume_if_matches(TokenCategory::Minus) {
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
        if self.consume_if_matches(TokenCategory::ParenOpen).is_some() {
            let expression = self.parse_expression()?;
            self.consume_must_be(TokenCategory::ParenClose)?;
            return Ok(expression);
        }
        self.parse_identifier_or_call()
    }

    fn parse_identifier_or_call(&mut self) -> Result<Node<Expression>, ParserIssue> {
        let identifier = self.parse_identifier()?;
        let position = identifier.position;

        let result = match self.consume_if_matches(TokenCategory::ParenOpen) {
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
            position: position,
        })
    }

    fn parse_switch_statement(&mut self) -> Result<Node<Statement>, ParserIssue> {
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

    fn parse_switch_expressions(&mut self) -> Result<Vec<Node<SwitchExpression>>, ParserIssue> {
        let mut switch_expressions: Vec<Node<SwitchExpression>> = vec![];
        let mut expression = self.parse_switch_expression()?;
        switch_expressions.push(expression);
        while let Some(_) = self.consume_if_matches(TokenCategory::Comma) {
            expression = self.parse_switch_expression()?;
            switch_expressions.push(expression);
        }
        Ok(switch_expressions)
    }

    fn parse_switch_expression(&mut self) -> Result<Node<SwitchExpression>, ParserIssue> {
        let expression = self.parse_expression()?;
        let position = expression.position;
        let alias = match self.consume_if_matches(TokenCategory::Colon) {
            Some(_) => Some(self.parse_identifier()?),
            None => None,
        };
        let node = Node {
            value: SwitchExpression { expression, alias },
            position,
        };
        Ok(node)
    }

    fn parse_switch_case(&mut self) -> Result<Node<SwitchCase>, ParserIssue> {
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

    fn parse_type(&mut self) -> Result<Node<Type>, ParserIssue> {
        let token = self.current_token();
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
        let token = self.current_token();
        if self.consume_if_matches(TokenCategory::True).is_some() {
            return Ok(Node {
                value: Literal::True,
                position: token.position,
            });
        } else if self.consume_if_matches(TokenCategory::False).is_some() {
            return Ok(Node {
                value: Literal::False,
                position: token.position,
            });
        } else if self
            .consume_if_matches(TokenCategory::StringValue)
            .is_some()
        {
            if let TokenValue::String(string) = token.value {
                return Ok(Node {
                    value: Literal::String(string),
                    position: token.position,
                });
            }
        } else if self.consume_if_matches(TokenCategory::I64Value).is_some() {
            if let TokenValue::I64(int) = token.value {
                return Ok(Node {
                    value: Literal::I64(int),
                    position: token.position,
                });
            }
        } else if self.consume_if_matches(TokenCategory::F64Value).is_some() {
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
        let token = self.consume_must_be(TokenCategory::Identifier)?;
        if let TokenValue::String(name) = token.value {
            let node = Node {
                value: Identifier(name),
                position: token.position,
            };
            return Ok(node);
        }
        Err(Self::create_parser_error(format!(
            "Wrong token value type - given: {:?}, expected: {:?}.",
            token.value,
            TokenValue::String("".to_owned())
        )))
    }

    fn consume_must_be(&mut self, category: TokenCategory) -> Result<Token, ParserIssue> {
        let current_token = self.current_token();
        if current_token.category == category {
            let _ = self.next_token();
            return Ok(current_token.clone());
        }
        Err(Self::create_parser_error(format!(
            "Unexpected token - {:?}. Expected {:?}.",
            current_token.category, category
        )))
    }

    fn consume_if_matches(&mut self, category: TokenCategory) -> Option<Token> {
        let current_token = self.current_token();
        if current_token.category == category {
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

#[cfg(test)]
mod tests {
    use crate::{
        lazy_stream_reader::Position,
        lexer_utils::{LexerIssue, LexerIssueKind},
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

        fn next(&mut self) -> Result<Token, LexerIssue> {
            if self.tokens.len() == 0 {
                return Err(LexerIssue {
                    kind: LexerIssueKind::ERROR,
                    message: "".to_owned(),
                });
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

    #[test]
    fn parse_assign_or_call_call() {
        let tokens = vec![
            create_token(
                TokenCategory::Identifier,
                TokenValue::String("print".to_owned()),
            ),
            create_token(TokenCategory::ParenOpen, TokenValue::Null),
            create_token(
                TokenCategory::StringValue,
                TokenValue::String("Hello world".to_owned()),
            ),
            create_token(TokenCategory::ParenClose, TokenValue::Null),
            create_token(TokenCategory::Semicolon, TokenValue::Null),
            create_token(TokenCategory::ETX, TokenValue::Null),
        ];

        let mock_lexer = LexerMock::new(tokens);
        let mut parser = Parser::new(mock_lexer);
        let result = parser.parse_assign_or_call();

        assert!(result.is_ok());
        let parsed = result.unwrap();
        assert!(
            parsed.value
                == Statement::FunctionCall {
                    identifier: Node {
                        value: Identifier("print".to_owned()),
                        position: default_position()
                    },
                    arguments: vec![Box::new(Node {
                        value: Argument {
                            value: Expression::Literal(Literal::String("Hello world".to_owned())),
                            passed_by: ArgumentPassedBy::Value
                        },
                        position: default_position()
                    })]
                }
        );
    }

    #[test]
    fn parse_literals() {
        let tokens = vec![
            create_token(TokenCategory::True, TokenValue::Null),
            create_token(TokenCategory::False, TokenValue::Null),
            create_token(TokenCategory::StringValue, TokenValue::String("a".to_owned())),
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
    fn parse_identifier() {
        let tokens = vec![
            create_token(
                TokenCategory::Identifier,
                TokenValue::String("print".to_owned()),
            ),
            create_token(TokenCategory::ETX, TokenValue::Null)
        ];

        let mock_lexer = LexerMock::new(tokens);
        let mut parser = Parser::new(mock_lexer);

        let node = parser.parse_identifier().unwrap();
        assert!(node.value == Identifier("print".to_owned()));
    }

    #[test]
    fn parse_identifier_bad_value_type() {
        let tokens = vec![
            create_token(
                TokenCategory::Identifier,
                TokenValue::I64(5),
            ),
            create_token(TokenCategory::ETX, TokenValue::Null)
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

        assert!(result.is_none());
        assert!(parser.current_token().clone().category == TokenCategory::ParenOpen);
    }
}
