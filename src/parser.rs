use std::{collections::HashMap, rc::Rc};

use crate::{
    ast::{
        Argument, Block, Expression, FunctionDeclaration, Literal, Node, Parameter, PassedBy, Program, Statement, SwitchCase, SwitchExpression, Type,
    },
    issues::{Issue, IssueLevel, ParserIssue},
    lexer::ILexer,
    std_functions::get_std_functions,
    tokens::{Token, TokenCategory, TokenValue},
};

macro_rules! try_consume_token {
    ($self:ident, $token_category:expr) => {
        match $self.consume_must_be($token_category) {
            Ok(t) => t,
            Err(_) => return Ok(None),
        }
    };
}

macro_rules! try_consume {
    ($self:ident, $method:ident) => {
        match $self.$method()? {
            Some(t) => t,
            None => return Ok(None),
        }
    };
}


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
        // program = { function_declaration | assign_or_call | if_statement | for_statement | switch_statement | declaration, ";" };
        let _ = self.next_token()?; // initialize
        let _ = self.next_token()?; // skip STX

        let mut statements: Vec<Node<Statement>> = vec![];
        let mut functions: HashMap<String, Rc<Node<FunctionDeclaration>>> = HashMap::new();
        let std_functions = get_std_functions();

        loop {
            if let Some(statement) = self.parse_program_statement()? {
                statements.push(statement);
            } else if let Some(function_declaration) = self.parse_function_declaration()? {
                let function_name = function_declaration.value.identifier.value.clone();
                if functions.contains_key(&function_name) || std_functions.contains_key(&function_name) {
                    return Err(Box::new(ParserIssue::new(
                        IssueLevel::ERROR,
                        format!("Redeclaration of function '{}'.\nAt: {:?}.", function_name, function_declaration.position),
                    )));
                }
                functions.insert(function_name, Rc::new(function_declaration));
            } else {
                break;
            }
        }

        self.consume_must_be(TokenCategory::ETX)?;

        let program = Program {
            statements,
            functions,
            std_functions,
        };
        Ok(program)
    }
}

impl<L: ILexer> Parser<L> {
    fn next_token(&mut self) -> Result<Option<Token>, Box<dyn Issue>> {
        // returns next token (skips comments)
        let mut current_token = self.lexer.next()?;
        while current_token.category == TokenCategory::Comment {
            current_token = self.lexer.next()?;
        }
        Ok(Some(current_token))
    }

    fn current_token(&self) -> Token {
        self.lexer.current().clone().unwrap()
    }

    fn consume_must_be(&mut self, category: TokenCategory) -> Result<Token, Box<dyn Issue>> {
        // consumes on match else throws error
        let current_token = self.current_token();
        if current_token.category == category {
            let _ = self.next_token()?;
            return Ok(current_token.clone());
        }
        Err(self.create_parser_error(format!("Unexpected token - {:?}. Expected {:?}.", current_token.category, category)))
    }

    fn consume_if_matches(&mut self, category: TokenCategory) -> Result<Option<Token>, Box<dyn Issue>> {
        // consumes on match, else does nothing
        let current_token = self.current_token();
        if current_token.category == category {
            let _ = self.next_token()?;
            return Ok(Some(current_token.clone()));
        }
        Ok(None)
    }

    fn parse_program_statement(&mut self) -> Result<Option<Node<Statement>>, Box<dyn Issue>> {
        // program = { assign_or_call | if_statement | for_statement | switch_statement | declaration, ";" };
        let generators = [
            Self::parse_assign_or_call,
            Self::parse_if_statement,
            Self::parse_for_statement,
            Self::parse_switch_statement,
            Self::parse_variable_declaration,
        ];

        for generator in &generators {
            if let Some(statement) = generator(self)? {
                return Ok(Some(statement));
            }
        }

        Ok(None)
    }

    fn void_type_or_error(&mut self) -> Result<Node<Type>, Box<dyn Issue>> {
        match self.consume_if_matches(TokenCategory::Void)? {
            Some(token) => Ok(Node {
                value: Type::Void,
                position: token.position,
            }),
            None => {
                return Err(self.create_parser_error(format!(
                    "Bad return type: {:?}. Expected one of: 'i64', 'f64', 'bool', 'str', 'void'.",
                    self.current_token().category
                )))
            }
        }
    }

    fn parse_function_declaration(&mut self) -> Result<Option<Node<FunctionDeclaration>>, Box<dyn Issue>> {
        // function_declaration = “fn”, identifier, "(", parameters, ")", “:”, type | “void”, statement_block;
        let fn_token = try_consume_token!(self, TokenCategory::Fn);

        let identifier = self
            .parse_identifier()?
            .ok_or_else(|| self.create_parser_error(String::from("Couldn't create identifier while parsing function declaration.")))?;

        let _ = self.consume_must_be(TokenCategory::ParenOpen)?;
        let parameters = self.parse_parameters()?;
        let _ = self.consume_must_be(TokenCategory::ParenClose)?;
        let _ = self.consume_must_be(TokenCategory::Colon)?;
        let return_type = match self.parse_type() {
            Ok(Some(t)) => t,
            _ => self.void_type_or_error()?,
        };
        let block = self
            .parse_statement_block()?
            .ok_or_else(|| self.create_parser_error(String::from("Couldn't create statement block while parsing function declaration.")))?;

        let node = Node {
            value: FunctionDeclaration {
                identifier,
                parameters,
                return_type,
                block,
            },
            position: fn_token.position,
        };

        Ok(Some(node))
    }

    fn parse_parameters(&mut self) -> Result<Vec<Node<Parameter>>, Box<dyn Issue>> {
        // parameters = [ parameter, { ",", parameter } ];
        let expression = match self.parse_parameter()? {
            Some(t) => t,
            None => return Ok(vec![]),
        };

        let mut parameters = vec![expression];
        while let Some(_) = self.consume_if_matches(TokenCategory::Comma)? {
            let parameter = self
                .parse_parameter()?
                .ok_or_else(|| self.create_parser_error(String::from("Couldn't create parameter while parsing parameters.")))?;
            parameters.push(parameter);
        }

        Ok(parameters)
    }

    fn parse_parameter(&mut self) -> Result<Option<Node<Parameter>>, Box<dyn Issue>> {
        // parameter = [“&”], type, identifier, [ "=", expression ];
        let position = self.current_token().position;
        let passed_by = match self.consume_if_matches(TokenCategory::Reference)? {
            Some(_) => PassedBy::Reference,
            None => PassedBy::Value,
        };

        let parameter_type = try_consume!(self, parse_type);
        let identifier = self
            .parse_identifier()?
            .ok_or_else(|| self.create_parser_error(String::from("Couldn't create identifier while parsing parameter.")))?;

        let node = Node {
            value: Parameter {
                passed_by,
                parameter_type,
                identifier,
            },
            position,
        };
        Ok(Some(node))
    }

    fn parse_for_statement(&mut self) -> Result<Option<Node<Statement>>, Box<dyn Issue>> {
        // for_statement = "for", "(", [ declaration ], “;”, expression, “;”, [ identifier, "=", expression ], ")", statement_block;
        let for_token = try_consume_token!(self, TokenCategory::For);

        let _ = self.consume_must_be(TokenCategory::ParenOpen)?;
        let declaration = self
            .parse_declaration()
            .map_err(|_| self.create_parser_error(String::from("Couldn't create declaration while parsing for statement.")))?
            .map(|t| {
                let position = t.position;
                let node = Node { value: t.value, position };
                Box::new(node)
            });

        self.consume_must_be(TokenCategory::Semicolon)?;
        let condition = self
            .parse_expression()?
            .ok_or_else(|| self.create_parser_error(String::from("Couldn't create expression while parsing for statement.")))?;

        self.consume_must_be(TokenCategory::Semicolon)?;
        let mut assignment: Option<Box<Node<Statement>>> = None;
        if self.current_token().category == TokenCategory::Identifier {
            let identifier = self
                .parse_identifier()?
                .ok_or_else(|| self.create_parser_error(String::from("Couldn't create identifier while parsing for statement.")))?;

            let position = identifier.position;
            let _ = self.consume_must_be(TokenCategory::Assign)?;
            let expr = self
                .parse_expression()?
                .ok_or_else(|| self.create_parser_error(String::from("Couldn't create expression while parsing for statement.")))?;

            let assign = Box::new(Node {
                value: Statement::Assignment { identifier, value: expr },
                position,
            });
            assignment = Some(assign);
        };

        self.consume_must_be(TokenCategory::ParenClose)?;
        let block = self
            .parse_statement_block()?
            .ok_or_else(|| self.create_parser_error(String::from("Couldn't create statement block while parsing for statement.")))?;

        let node = Node {
            value: Statement::ForLoop {
                declaration,
                condition,
                assignment,
                block,
            },
            position: for_token.position,
        };
        Ok(Some(node))
    }

    fn parse_if_statement(&mut self) -> Result<Option<Node<Statement>>, Box<dyn Issue>> {
        // if_statement = "if", "(", expression, ")", statement_block, [ "else", statement_block ];
        let if_token = try_consume_token!(self, TokenCategory::If);

        let _ = self.consume_must_be(TokenCategory::ParenOpen)?;
        let condition = self
            .parse_expression()?
            .ok_or_else(|| self.create_parser_error(String::from("Couldn't create expression while parsing if statement.")))?;

        let _ = self.consume_must_be(TokenCategory::ParenClose)?;
        let true_block = self
            .parse_statement_block()?
            .ok_or_else(|| self.create_parser_error(String::from("Couldn't create statement block while parsing if statement.")))?;

        let false_block = match self.consume_if_matches(TokenCategory::Else)? {
            Some(_) => self.parse_statement_block()?,
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
        Ok(Some(node))
    }

    fn parse_statement_block(&mut self) -> Result<Option<Node<Block>>, Box<dyn Issue>> {
        // statement_block = "{", {statement}, "}";
        let token = try_consume_token!(self, TokenCategory::BraceOpen);

        let mut statements: Vec<Node<Statement>> = vec![];
        while self.consume_if_matches(TokenCategory::BraceClose)?.is_none() {
            let statement = self
                .parse_statement()?
                .ok_or_else(|| self.create_parser_error(String::from("Couldn't create statement while parsing statement block.")))?;

            statements.push(statement);
        }
        Ok(Some(Node {
            value: Block(statements),
            position: token.position,
        }))
    }

    fn parse_variable_declaration(&mut self) -> Result<Option<Node<Statement>>, Box<dyn Issue>> {
        let decl = try_consume!(self, parse_declaration);

        self.consume_must_be(TokenCategory::Semicolon)?;
        Ok(Some(decl))
    }

    fn parse_statement(&mut self) -> Result<Option<Node<Statement>>, Box<dyn Issue>> {
        // statement = assign_or_call | if_statement | for_statement | switch_statement | declaration, ";" | return_statement | break_statement;
        let generators = [
            Self::parse_assign_or_call,
            Self::parse_if_statement,
            Self::parse_for_statement,
            Self::parse_switch_statement,
            Self::parse_return_statement,
            Self::parse_break_statement,
            Self::parse_variable_declaration,
        ];

        for generator in &generators {
            if let Some(statement) = generator(self)? {
                return Ok(Some(statement));
            }
        }

        Ok(None)
    }

    fn parse_assign_or_call(&mut self) -> Result<Option<Node<Statement>>, Box<dyn Issue>> {
        // assign_or_call = identifier, ("=", expression | "(", arguments, ")"), ";";
        let identifier = try_consume!(self, parse_identifier);

        let position = identifier.position;

        if self.consume_if_matches(TokenCategory::Assign)?.is_some() {
            let expr = self
                .parse_expression()?
                .ok_or_else(|| self.create_parser_error(String::from("Couldn't create expression while parsing assignment.")))?;

            let node = Node {
                value: Statement::Assignment { identifier, value: expr },
                position,
            };
            self.consume_must_be(TokenCategory::Semicolon)?;
            return Ok(Some(node));
        }

        if self.consume_if_matches(TokenCategory::ParenOpen)?.is_some() {
            let arguments = self.parse_arguments()?.into_iter().map(Box::new).collect();
            let node = Node {
                value: Statement::FunctionCall { identifier, arguments },
                position,
            };
            self.consume_must_be(TokenCategory::ParenClose)?;
            self.consume_must_be(TokenCategory::Semicolon)?;
            return Ok(Some(node));
        }

        Err(self.create_parser_error(String::from("Couldn't create assignment or call.")))
    }

    fn parse_declaration(&mut self) -> Result<Option<Node<Statement>>, Box<dyn Issue>> {
        // declaration = type, identifier, [ "=", expression ];
        let declaration_type = try_consume!(self, parse_type);

        let position = declaration_type.position;
        let identifier = self
            .parse_identifier()?
            .ok_or_else(|| self.create_parser_error(String::from("Couldn't create identifier while parsing variable declaration.")))?;

        let value = match self.consume_if_matches(TokenCategory::Assign)? {
            Some(_) => self.parse_expression()?,
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
        Ok(Some(node))
    }

    fn parse_return_statement(&mut self) -> Result<Option<Node<Statement>>, Box<dyn Issue>> {
        // return_statement = "return", [ expression ], ";";
        let token = try_consume_token!(self, TokenCategory::Return);

        let returned_value = self.parse_expression()?;
        self.consume_must_be(TokenCategory::Semicolon)?;
        let node = Node {
            value: Statement::Return(returned_value),
            position: token.position,
        };
        Ok(Some(node))
    }

    fn parse_break_statement(&mut self) -> Result<Option<Node<Statement>>, Box<dyn Issue>> {
        // break_statement = "break", ";";
        let token = try_consume_token!(self, TokenCategory::Break);

        let _ = self.consume_must_be(TokenCategory::Semicolon)?;
        let node = Node {
            value: Statement::Break,
            position: token.position,
        };
        Ok(Some(node))
    }

    fn parse_arguments(&mut self) -> Result<Vec<Node<Argument>>, Box<dyn Issue>> {
        // arguments = [ argument, {",", argument} ];
        let expression = match self.parse_argument()? {
            Some(t) => t,
            None => return Ok(vec![]),
        };

        let mut arguments = vec![expression];
        while let Some(_) = self.consume_if_matches(TokenCategory::Comma)? {
            let argument = self
                .parse_argument()?
                .ok_or_else(|| self.create_parser_error(String::from("Couldn't create argument while parsing arguments.")))?;

            arguments.push(argument);
        }
        Ok(arguments)
    }

    fn parse_argument(&mut self) -> Result<Option<Node<Argument>>, Box<dyn Issue>> {
        // argument = [“&”], expression;
        let passed_by = match self.consume_if_matches(TokenCategory::Reference)? {
            Some(_) => PassedBy::Reference,
            None => PassedBy::Value,
        };

        let expression = try_consume!(self, parse_expression);
        let argument = Argument {
            value: expression.clone(),
            passed_by,
        };
        Ok(Some(Node {
            value: argument,
            position: expression.position,
        }))
    }

    fn parse_expression(&mut self) -> Result<Option<Node<Expression>>, Box<dyn Issue>> {
        // expression = concatenation_term { “||”, concatenation_term };
        let mut left_side = try_consume!(self, parse_concatenation_term);

        let mut current_token = self.current_token();
        while current_token.category == TokenCategory::Or {
            let _ = self.next_token()?;
            let right_side = self
                .parse_concatenation_term()?
                .ok_or_else(|| self.create_parser_error(String::from("Couldn't create concatenation term while parsing expression.")))?;

            let expression_type = Expression::Alternative(Box::new(left_side.clone()), Box::new(right_side.clone()));
            left_side = Node {
                value: expression_type,
                position: current_token.position,
            };
            current_token = self.current_token();
        }
        Ok(Some(left_side))
    }

    fn parse_concatenation_term(&mut self) -> Result<Option<Node<Expression>>, Box<dyn Issue>> {
        // concatenation_term = relation_term, { “&&”, relation_term };
        let mut left_side = try_consume!(self, parse_relation_term);

        let mut current_token = self.current_token();
        while current_token.category == TokenCategory::And {
            let _ = self.next_token()?;
            let right_side = self
                .parse_relation_term()?
                .ok_or_else(|| self.create_parser_error(String::from("Couldn't create relation term while parsing concatenation term.")))?;

            let expression_type = Expression::Concatenation(Box::new(left_side.clone()), Box::new(right_side.clone()));
            left_side = Node {
                value: expression_type,
                position: current_token.position,
            };
            current_token = self.current_token();
        }
        Ok(Some(left_side))
    }

    fn parse_relation_term(&mut self) -> Result<Option<Node<Expression>>, Box<dyn Issue>> {
        // relation_term = additive_term, [ relation_operands, additive_term ];
        let left_side = try_consume!(self, parse_additive_term);

        let operands = [
            TokenCategory::Equal,
            TokenCategory::NotEqual,
            TokenCategory::Greater,
            TokenCategory::GreaterOrEqual,
            TokenCategory::Less,
            TokenCategory::LessOrEqual,
        ];

        let current_token = self.current_token();
        if !operands.contains(&current_token.category) {
            return Ok(Some(left_side));
        }

        let _ = self.next_token()?;
        let right_side = self
            .parse_additive_term()?
            .ok_or_else(|| self.create_parser_error(String::from("Couldn't create additive term while parsing relation term.")))?;

        let box_l = Box::new(left_side.clone());
        let box_r = Box::new(right_side);

        let expr = match current_token.category {
            TokenCategory::Equal => Expression::Equal(box_l, box_r),
            TokenCategory::NotEqual => Expression::NotEqual(box_l, box_r),
            TokenCategory::Greater => Expression::Greater(box_l, box_r),
            TokenCategory::GreaterOrEqual => Expression::GreaterEqual(box_l, box_r),
            TokenCategory::Less => Expression::Less(box_l, box_r),
            TokenCategory::LessOrEqual => Expression::LessEqual(box_l, box_r),
            _ => return Err(self.create_parser_error(String::from("Couldn't create additive term while parsing relation term."))),
        };

        let node = Node {
            value: expr,
            position: left_side.position,
        };
        Ok(Some(node))
    }

    fn parse_additive_term(&mut self) -> Result<Option<Node<Expression>>, Box<dyn Issue>> {
        // additive_term = multiplicative_term , { ("+" | "-"), multiplicative_term };
        let mut left_side = try_consume!(self, parse_multiplicative_term);

        let mut current_token = self.current_token();
        while current_token.category == TokenCategory::Plus || current_token.category == TokenCategory::Minus {
            let _ = self.next_token()?;
            let right_side = self
                .parse_multiplicative_term()?
                .ok_or_else(|| self.create_parser_error(String::from("Couldn't create multiplicative term while parsing additive term.")))?;

            let mut expression_type = Expression::Addition(Box::new(left_side.clone()), Box::new(right_side.clone()));
            if current_token.category == TokenCategory::Minus {
                expression_type = Expression::Subtraction(Box::new(left_side), Box::new(right_side))
            }
            left_side = Node {
                value: expression_type,
                position: current_token.position,
            };
            current_token = self.current_token();
        }
        Ok(Some(left_side))
    }

    fn parse_multiplicative_term(&mut self) -> Result<Option<Node<Expression>>, Box<dyn Issue>> {
        // multiplicative_term = casted_term, { ("*" | "/"), casted_term };
        let mut left_side = try_consume!(self, parse_casted_term);

        let mut current_token = self.current_token();
        while current_token.category == TokenCategory::Multiply || current_token.category == TokenCategory::Divide {
            let _ = self.next_token()?;
            let right_side = self
                .parse_casted_term()?
                .ok_or_else(|| self.create_parser_error(String::from("Couldn't create casted term while parsing multiplicative term.")))?;

            let mut expression_type = Expression::Multiplication(Box::new(left_side.clone()), Box::new(right_side.clone()));
            if current_token.category == TokenCategory::Divide {
                expression_type = Expression::Division(Box::new(left_side), Box::new(right_side))
            }
            left_side = Node {
                value: expression_type,
                position: current_token.position,
            };
            current_token = self.current_token();
        }
        Ok(Some(left_side))
    }

    fn parse_casted_term(&mut self) -> Result<Option<Node<Expression>>, Box<dyn Issue>> {
        // casted_term = unary_term, [ “as”, type ];
        let unary_term = try_consume!(self, parse_unary_term);

        let position = unary_term.position.clone();
        match self.consume_if_matches(TokenCategory::As)? {
            Some(_) => {
                let type_parsed = self
                    .parse_type()?
                    .ok_or_else(|| self.create_parser_error(String::from("Couldn't parse type.")))?;

                Ok(Some(Node {
                    value: Expression::Casting {
                        value: Box::new(unary_term),
                        to_type: type_parsed,
                    },
                    position,
                }))
            }
            None => Ok(Some(unary_term)),
        }
    }

    fn parse_unary_term_factor(&mut self) -> Result<Node<Expression>, Box<dyn Issue>> {
        match self.parse_factor()? {
            Some(t) => Ok(t),
            None => return Err(self.create_parser_error(String::from("Couldn't create factor while parsing unary term."))),
        }
    }

    fn parse_unary_term(&mut self) -> Result<Option<Node<Expression>>, Box<dyn Issue>> {
        // unary_term = [ ("-", "!") ], factor;
        if let Some(token) = self.consume_if_matches(TokenCategory::Negate)? {
            let factor = self.parse_unary_term_factor()?;
            return Ok(Some(Node {
                value: Expression::BooleanNegation(Box::new(factor)),
                position: token.position,
            }));
        }

        if let Some(token) = self.consume_if_matches(TokenCategory::Minus)? {
            let factor = self.parse_unary_term_factor()?;
            return Ok(Some(Node {
                value: Expression::ArithmeticNegation(Box::new(factor)),
                position: token.position,
            }));
        }

        let factor = self.parse_factor()?;
        Ok(factor)
    }

    fn parse_factor(&mut self) -> Result<Option<Node<Expression>>, Box<dyn Issue>> {
        // factor = literal | ( "(", expression, ")" ) | identifier_or_call;
        if let Ok(Some(literal)) = self.parse_literal() {
            let node = Node {
                value: Expression::Literal(literal.value),
                position: literal.position,
            };
            return Ok(Some(node));
        }

        if self.consume_if_matches(TokenCategory::ParenOpen)?.is_some() {
            let expression = self
                .parse_expression()?
                .ok_or_else(|| self.create_parser_error(String::from("Couldn't create expression while parsing nested expression.")))?;

            self.consume_must_be(TokenCategory::ParenClose)?;
            return Ok(Some(expression));
        }
        self.parse_identifier_or_call()
    }

    fn parse_identifier_or_call(&mut self) -> Result<Option<Node<Expression>>, Box<dyn Issue>> {
        // identifier_or_call = identifier, [ "(", arguments, ")" ];
        let identifier = try_consume!(self, parse_identifier);

        let position = identifier.position;

        let result = match self.consume_if_matches(TokenCategory::ParenOpen)? {
            Some(_) => {
                let args = self.parse_arguments()?.into_iter().map(Box::new).collect();
                let _ = self.consume_must_be(TokenCategory::ParenClose)?;
                Expression::FunctionCall { identifier, arguments: args }
            }
            None => Expression::Variable(identifier.value),
        };
        Ok(Some(Node { value: result, position }))
    }

    fn parse_switch_statement(&mut self) -> Result<Option<Node<Statement>>, Box<dyn Issue>> {
        // switch_statement = "switch", "(", switch_expressions, ")", "{", {switch_case}, "}";
        let switch_token = try_consume_token!(self, TokenCategory::Switch);

        let _ = self.consume_must_be(TokenCategory::ParenOpen)?;
        let switch_expressions = self.parse_switch_expressions()?;
        let _ = self.consume_must_be(TokenCategory::ParenClose)?;
        let _ = self.consume_must_be(TokenCategory::BraceOpen)?;

        let mut switch_cases: Vec<Node<SwitchCase>> = vec![];
        while self.current_token().category != TokenCategory::BraceClose {
            let switch_case = self
                .parse_switch_case()?
                .ok_or_else(|| self.create_parser_error(String::from("Couldn't create switch case while parsing switch statement.")))?;

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
        Ok(Some(node))
    }

    fn parse_switch_expressions(&mut self) -> Result<Vec<Node<SwitchExpression>>, Box<dyn Issue>> {
        // switch_expressions = switch_expression, { “,”, switch_expression };
        let mut switch_expressions: Vec<Node<SwitchExpression>> = vec![];
        let mut expression = match self.parse_switch_expression()? {
            Some(t) => t,
            None => return Ok(vec![]),
        };

        switch_expressions.push(expression);
        while let Some(_) = self.consume_if_matches(TokenCategory::Comma)? {
            expression = self
                .parse_switch_expression()?
                .ok_or_else(|| self.create_parser_error(String::from("Couldn't create swicth expression while parsing switch expressions.")))?;

            switch_expressions.push(expression);
        }
        Ok(switch_expressions)
    }

    fn parse_switch_expression(&mut self) -> Result<Option<Node<SwitchExpression>>, Box<dyn Issue>> {
        // switch_expression = expression, [ ":", identifier ];
        let expression = try_consume!(self, parse_expression);

        let position = expression.position;
        let mut alias = None;
        if let Some(_) = self.consume_if_matches(TokenCategory::Colon)? {
            alias = self.parse_identifier()?;
        };
        let node = Node {
            value: SwitchExpression { expression, alias },
            position,
        };
        Ok(Some(node))
    }

    fn parse_switch_case(&mut self) -> Result<Option<Node<SwitchCase>>, Box<dyn Issue>> {
        // switch_case = "(", expression, ")", "->", statement_block;
        let paren_open_token = try_consume_token!(self, TokenCategory::ParenOpen);

        let condition = self
            .parse_expression()?
            .ok_or_else(|| self.create_parser_error(String::from("Couldn't create expression while parsing switch case.")))?;

        let _ = self.consume_must_be(TokenCategory::ParenClose)?;
        let _ = self.consume_must_be(TokenCategory::Arrow)?;
        let block = self
            .parse_statement_block()?
            .ok_or_else(|| self.create_parser_error(String::from("Couldn't create statement block while parsing switch case.")))?;

        let node = Node {
            value: SwitchCase { condition, block },
            position: paren_open_token.position,
        };
        Ok(Some(node))
    }

    fn parse_type(&mut self) -> Result<Option<Node<Type>>, Box<dyn Issue>> {
        let token = self.current_token();

        let result = match token.category {
            TokenCategory::Bool => Type::Bool,
            TokenCategory::String => Type::Str,
            TokenCategory::I64 => Type::I64,
            TokenCategory::F64 => Type::F64,
            _ => return Ok(None),
        };

        let _ = self.next_token()?;

        Ok(Some(Node {
            value: result,
            position: token.position,
        }))
    }

    fn parse_literal(&mut self) -> Result<Option<Node<Literal>>, Box<dyn Issue>> {
        let token = self.current_token();
        let position = token.position;

        let literal = match (token.category, token.value) {
            (TokenCategory::True, _) => Literal::True,
            (TokenCategory::False, _) => Literal::False,
            (TokenCategory::I64Value, TokenValue::I64(int)) => Literal::I64(int),
            (TokenCategory::F64Value, TokenValue::F64(float)) => Literal::F64(float),
            (TokenCategory::StringValue, TokenValue::String(string)) => Literal::String(string),
            _ => return Ok(None),
        };

        let _ = self.next_token();

        let node = Node { value: literal, position };
        Ok(Some(node))
    }

    fn parse_identifier(&mut self) -> Result<Option<Node<String>>, Box<dyn Issue>> {
        let token = try_consume_token!(self, TokenCategory::Identifier);

        if let TokenValue::String(name) = token.value {
            let node = Node {
                value: name,
                position: token.position,
            };
            return Ok(Some(node));
        }
        Err(self.create_parser_error(format!(
            "Wrong token value type - given: {:?}, expected: {:?}.",
            token.value,
            TokenValue::String(String::new())
        )))
    }

    fn create_parser_error(&self, text: String) -> Box<dyn Issue> {
        let position = self.current_token().position;
        Box::new(ParserIssue::new(IssueLevel::ERROR, format!("{}\nAt {:?}.", text, position)))
    }
}

#[cfg(test)]
mod tests {
    use std::vec;

    use crate::{
        issues::{IssueLevel, LexerIssue},
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
                return Err(Box::new(LexerIssue::new(IssueLevel::ERROR, String::new())));
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
                create_token(TokenCategory::Identifier, TokenValue::String(String::from("x"))),
                create_token(TokenCategory::Assign, TokenValue::Null),
                create_token(TokenCategory::I64Value, TokenValue::I64(5)),
                create_token(TokenCategory::Semicolon, TokenValue::Null),
                create_token(TokenCategory::BraceClose, TokenValue::Null),
                create_token(TokenCategory::ETX, TokenValue::Null),
            ],
            vec![
                create_token(TokenCategory::BraceOpen, TokenValue::Null),
                create_token(TokenCategory::Identifier, TokenValue::String(String::from("x"))),
                create_token(TokenCategory::Assign, TokenValue::Null),
                create_token(TokenCategory::I64Value, TokenValue::I64(5)),
                create_token(TokenCategory::Semicolon, TokenValue::Null),
                create_token(TokenCategory::Identifier, TokenValue::String(String::from("x"))),
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
                        value: String::from("x"),
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
                            value: String::from("x"),
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
                            value: String::from("x"),
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

            let node = parser.parse_statement_block().unwrap().unwrap();
            assert!(node.value == expected[idx]);
        }
    }

    #[test]
    fn parse_statement_fail() {
        let token_series = vec![vec![
            // i64 a = 5
            create_token(TokenCategory::I64, TokenValue::Null),
            create_token(TokenCategory::Identifier, TokenValue::String(String::from("a"))),
            create_token(TokenCategory::Assign, TokenValue::Null),
            create_token(TokenCategory::I64Value, TokenValue::I64(5)),
            create_token(TokenCategory::ETX, TokenValue::Null),
        ]];

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
                create_token(TokenCategory::Identifier, TokenValue::String(String::from("x"))),
                create_token(TokenCategory::Assign, TokenValue::Null),
                create_token(TokenCategory::I64Value, TokenValue::I64(5)),
                create_token(TokenCategory::Semicolon, TokenValue::Null),
                create_token(TokenCategory::ETX, TokenValue::Null),
            ],
            vec![
                // print();
                create_token(TokenCategory::Identifier, TokenValue::String(String::from("print"))),
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
                create_token(TokenCategory::Identifier, TokenValue::String(String::from("x"))),
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
                create_token(TokenCategory::Identifier, TokenValue::String(String::from("a"))),
                create_token(TokenCategory::Assign, TokenValue::Null),
                create_token(TokenCategory::I64Value, TokenValue::I64(5)),
                create_token(TokenCategory::Semicolon, TokenValue::Null),
                create_token(TokenCategory::ETX, TokenValue::Null),
            ],
        ];

        let expected = [
            Statement::Assignment {
                identifier: Node {
                    value: String::from("x"),
                    position: default_position(),
                },
                value: Node {
                    value: Expression::Literal(Literal::I64(5)),
                    position: default_position(),
                },
            },
            Statement::FunctionCall {
                identifier: Node {
                    value: String::from("print"),
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
                            value: Expression::Variable(String::from("x")),
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
                    value: String::from("a"),
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

            let node = parser.parse_statement().unwrap().unwrap();
            assert!(node.value == expected[idx]);
        }
    }

    #[test]
    fn parse_function_declaration_fail() {
        let token_series = vec![vec![
            // fn add(): , {}
            create_token(TokenCategory::Fn, TokenValue::Null),
            create_token(TokenCategory::Identifier, TokenValue::String(String::from("add"))),
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
                create_token(TokenCategory::Identifier, TokenValue::String(String::from("add"))),
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
                create_token(TokenCategory::Identifier, TokenValue::String(String::from("add"))),
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
            FunctionDeclaration {
                identifier: Node {
                    value: String::from("add"),
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
            FunctionDeclaration {
                identifier: Node {
                    value: String::from("add"),
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

            let node = parser.parse_function_declaration().unwrap().unwrap();
            assert!(node.value == expected[idx]);
        }
    }

    #[test]
    fn parse_parameters_fail() {
        let tokens = vec![
            // i64 x,
            create_token(TokenCategory::I64, TokenValue::Null),
            create_token(TokenCategory::Identifier, TokenValue::String(String::from("x"))),
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
                create_token(TokenCategory::Identifier, TokenValue::String(String::from("x"))),
                create_token(TokenCategory::ETX, TokenValue::Null),
            ],
            vec![
                // i64 x, i64 y
                create_token(TokenCategory::I64, TokenValue::Null),
                create_token(TokenCategory::Identifier, TokenValue::String(String::from("x"))),
                create_token(TokenCategory::Comma, TokenValue::Null),
                create_token(TokenCategory::I64, TokenValue::Null),
                create_token(TokenCategory::Identifier, TokenValue::String(String::from("y"))),
                create_token(TokenCategory::ETX, TokenValue::Null),
            ],
        ];

        let expected = [
            vec![],
            vec![Node {
                value: Parameter {
                    passed_by: PassedBy::Value,
                    parameter_type: Node {
                        value: Type::I64,
                        position: default_position(),
                    },
                    identifier: Node {
                        value: String::from("x"),
                        position: default_position(),
                    },
                },
                position: default_position(),
            }],
            vec![
                Node {
                    value: Parameter {
                        passed_by: PassedBy::Value,
                        parameter_type: Node {
                            value: Type::I64,
                            position: default_position(),
                        },
                        identifier: Node {
                            value: String::from("x"),
                            position: default_position(),
                        },
                    },
                    position: default_position(),
                },
                Node {
                    value: Parameter {
                        passed_by: PassedBy::Value,
                        parameter_type: Node {
                            value: Type::I64,
                            position: default_position(),
                        },
                        identifier: Node {
                            value: String::from("y"),
                            position: default_position(),
                        },
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
                create_token(TokenCategory::Identifier, TokenValue::String(String::from("x"))),
                create_token(TokenCategory::Assign, TokenValue::Null),
                create_token(TokenCategory::I64Value, TokenValue::I64(0)),
                create_token(TokenCategory::ETX, TokenValue::Null),
            ],
            vec![
                // i64 x
                create_token(TokenCategory::I64, TokenValue::Null),
                create_token(TokenCategory::Identifier, TokenValue::String(String::from("x"))),
                create_token(TokenCategory::ETX, TokenValue::Null),
            ],
        ];

        let expected = [
            Parameter {
                passed_by: PassedBy::Reference,
                parameter_type: Node {
                    value: Type::I64,
                    position: default_position(),
                },
                identifier: Node {
                    value: String::from("x"),
                    position: default_position(),
                },
            },
            Parameter {
                passed_by: PassedBy::Value,
                parameter_type: Node {
                    value: Type::I64,
                    position: default_position(),
                },
                identifier: Node {
                    value: String::from("x"),
                    position: default_position(),
                },
            },
        ];

        for (idx, series) in token_series.iter().enumerate() {
            let mock_lexer = LexerMock::new(series.to_vec());
            let mut parser = Parser::new(mock_lexer);

            let node = parser.parse_parameter().unwrap().unwrap();
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
                create_token(TokenCategory::Identifier, TokenValue::String(String::from("x"))),
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
                create_token(TokenCategory::Identifier, TokenValue::String(String::from("x"))),
                create_token(TokenCategory::Assign, TokenValue::Null),
                create_token(TokenCategory::I64Value, TokenValue::I64(0)),
                create_token(TokenCategory::Semicolon, TokenValue::Null),
                create_token(TokenCategory::Identifier, TokenValue::String(String::from("x"))),
                create_token(TokenCategory::Less, TokenValue::Null),
                create_token(TokenCategory::I64Value, TokenValue::I64(5)),
                create_token(TokenCategory::Semicolon, TokenValue::Null),
                create_token(TokenCategory::Identifier, TokenValue::String(String::from("x"))),
                create_token(TokenCategory::Assign, TokenValue::Null),
                create_token(TokenCategory::Identifier, TokenValue::String(String::from("x"))),
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
                create_token(TokenCategory::Identifier, TokenValue::String(String::from("x"))),
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
                declaration: Some(Box::new(Node {
                    value: Statement::Declaration {
                        var_type: Node {
                            value: Type::I64,
                            position: default_position(),
                        },
                        identifier: Node {
                            value: String::from("x"),
                            position: default_position(),
                        },
                        value: Some(Node {
                            value: Expression::Literal(Literal::I64(0)),
                            position: default_position(),
                        }),
                    },
                    position: default_position(),
                })),
                condition: Node {
                    value: Expression::Less(
                        Box::new(Node {
                            value: Expression::Variable(String::from("x")),
                            position: default_position(),
                        }),
                        Box::new(Node {
                            value: Expression::Literal(Literal::I64(5)),
                            position: default_position(),
                        }),
                    ),
                    position: default_position(),
                },
                assignment: Some(Box::new(Node {
                    value: Statement::Assignment {
                        identifier: Node {
                            value: String::from("x"),
                            position: default_position(),
                        },
                        value: Node {
                            value: Expression::Addition(
                                Box::new(Node {
                                    value: Expression::Variable(String::from("x")),
                                    position: default_position(),
                                }),
                                Box::new(Node {
                                    value: Expression::Literal(Literal::I64(1)),
                                    position: default_position(),
                                }),
                            ),
                            position: default_position(),
                        },
                    },
                    position: default_position(),
                })),
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
                            value: Expression::Variable(String::from("x")),
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

            let node = parser.parse_for_statement().unwrap().unwrap();
            assert!(node.value == expected[idx]);
        }
    }

    #[test]
    fn parse_if_statement_fail() {
        let token_series = vec![
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

            assert!(parser.parse_if_statement().is_err());
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

            let node = parser.parse_if_statement().unwrap().unwrap();
            assert!(node.value == expected[idx]);
        }
    }

    #[test]
    fn parse_assign_or_call_fail() {
        let token_series = vec![
            vec![
                // print(;
                create_token(TokenCategory::Identifier, TokenValue::String(String::from("print"))),
                create_token(TokenCategory::ParenOpen, TokenValue::Null),
                create_token(TokenCategory::Semicolon, TokenValue::Null),
                create_token(TokenCategory::ETX, TokenValue::Null),
            ],
            vec![
                // print()
                create_token(TokenCategory::Identifier, TokenValue::String(String::from("print"))),
                create_token(TokenCategory::ParenOpen, TokenValue::Null),
                create_token(TokenCategory::ParenClose, TokenValue::Null),
                create_token(TokenCategory::ETX, TokenValue::Null),
            ],
            vec![
                // x = 5
                create_token(TokenCategory::Identifier, TokenValue::String(String::from("x"))),
                create_token(TokenCategory::Assign, TokenValue::Null),
                create_token(TokenCategory::I64Value, TokenValue::I64(5)),
                create_token(TokenCategory::ETX, TokenValue::Null),
            ],
            vec![
                create_token(TokenCategory::Identifier, TokenValue::String(String::from("x"))),
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
                create_token(TokenCategory::Identifier, TokenValue::String(String::from("print"))),
                create_token(TokenCategory::ParenOpen, TokenValue::Null),
                create_token(TokenCategory::ParenClose, TokenValue::Null),
                create_token(TokenCategory::Semicolon, TokenValue::Null),
                create_token(TokenCategory::ETX, TokenValue::Null),
            ],
            vec![
                // x = 5;
                create_token(TokenCategory::Identifier, TokenValue::String(String::from("x"))),
                create_token(TokenCategory::Assign, TokenValue::Null),
                create_token(TokenCategory::I64Value, TokenValue::I64(5)),
                create_token(TokenCategory::Semicolon, TokenValue::Null),
                create_token(TokenCategory::ETX, TokenValue::Null),
            ],
        ];

        let expected = vec![
            Statement::FunctionCall {
                identifier: Node {
                    value: String::from("print"),
                    position: default_position(),
                },
                arguments: vec![],
            },
            Statement::Assignment {
                identifier: Node {
                    value: String::from("x"),
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

            let node = parser.parse_assign_or_call().unwrap().unwrap();
            assert!(node.value == expected[idx]);
        }
    }

    #[test]
    fn parse_declaration() {
        let token_series = vec![
            vec![
                // i64 a
                create_token(TokenCategory::I64, TokenValue::Null),
                create_token(TokenCategory::Identifier, TokenValue::String(String::from("a"))),
                create_token(TokenCategory::ETX, TokenValue::Null),
            ],
            vec![
                // i64 a = 5
                create_token(TokenCategory::I64, TokenValue::Null),
                create_token(TokenCategory::Identifier, TokenValue::String(String::from("a"))),
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
                    value: String::from("a"),
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
                    value: String::from("a"),
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

            let node = parser.parse_declaration().unwrap().unwrap();
            assert!(node.value == expected[idx]);
        }
    }

    #[test]
    fn parse_return_statement_fail() {
        let token_series = vec![
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

            let node = parser.parse_return_statement().unwrap().unwrap();
            assert!(node.value == expected[idx]);
        }
    }

    #[test]
    fn parse_break_statement_fail() {
        let token_series = vec![vec![
            // break
            create_token(TokenCategory::Break, TokenValue::Null),
            create_token(TokenCategory::ETX, TokenValue::Null),
        ]];

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

        let node = parser.parse_break_statement().unwrap().unwrap();
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
                    value: Node {
                        value: Expression::Literal(Literal::I64(1)),
                        position: default_position(),
                    },
                    passed_by: PassedBy::Value,
                },
                position: default_position(),
            }],
            vec![
                Node {
                    value: Argument {
                        value: Node {
                            value: Expression::Literal(Literal::I64(1)),
                            position: default_position(),
                        },
                        passed_by: PassedBy::Reference,
                    },
                    position: default_position(),
                },
                Node {
                    value: Argument {
                        value: Node {
                            value: Expression::Literal(Literal::I64(2)),
                            position: default_position(),
                        },
                        passed_by: PassedBy::Value,
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
                create_token(TokenCategory::Identifier, TokenValue::String(String::from("x"))),
                create_token(TokenCategory::ETX, TokenValue::Null),
            ],
        ];

        let expected = [
            Argument {
                value: Node {
                    value: Expression::Literal(Literal::I64(1)),
                    position: default_position(),
                },
                passed_by: PassedBy::Value,
            },
            Argument {
                value: Node {
                    value: Expression::Variable(String::from("x")),
                    position: default_position(),
                },
                passed_by: PassedBy::Reference,
            },
        ];

        for (idx, series) in token_series.iter().enumerate() {
            let mock_lexer = LexerMock::new(series.to_vec());
            let mut parser = Parser::new(mock_lexer);

            let node = parser.parse_argument().unwrap().unwrap();
            assert!(node.value == expected[idx]);
        }
    }

    #[test]
    fn parse_expression() {
        let tokens = vec![
            // a || b || c
            create_token(TokenCategory::Identifier, TokenValue::String(String::from("a"))),
            create_token(TokenCategory::Or, TokenValue::Null),
            create_token(TokenCategory::Identifier, TokenValue::String(String::from("b"))),
            create_token(TokenCategory::Or, TokenValue::Null),
            create_token(TokenCategory::Identifier, TokenValue::String(String::from("c"))),
            create_token(TokenCategory::ETX, TokenValue::Null),
        ];

        let mock_lexer = LexerMock::new(tokens);
        let mut parser = Parser::new(mock_lexer);

        let node = parser.parse_expression().unwrap().unwrap();
        assert!(
            node.value
                == Expression::Alternative(
                    Box::new(Node {
                        value: Expression::Alternative(
                            Box::new(Node {
                                value: Expression::Variable(String::from("a")),
                                position: default_position()
                            }),
                            Box::new(Node {
                                value: Expression::Variable(String::from("b")),
                                position: default_position()
                            })
                        ),
                        position: default_position()
                    }),
                    Box::new(Node {
                        value: Expression::Variable(String::from("c")),
                        position: default_position()
                    })
                )
        );
    }

    #[test]
    fn parse_concatenation_term() {
        let tokens = vec![
            // a && b && c
            create_token(TokenCategory::Identifier, TokenValue::String(String::from("a"))),
            create_token(TokenCategory::And, TokenValue::Null),
            create_token(TokenCategory::Identifier, TokenValue::String(String::from("b"))),
            create_token(TokenCategory::And, TokenValue::Null),
            create_token(TokenCategory::Identifier, TokenValue::String(String::from("c"))),
            create_token(TokenCategory::ETX, TokenValue::Null),
        ];

        let mock_lexer = LexerMock::new(tokens);
        let mut parser = Parser::new(mock_lexer);

        let node = parser.parse_concatenation_term().unwrap().unwrap();
        assert!(
            node.value
                == Expression::Concatenation(
                    Box::new(Node {
                        value: Expression::Concatenation(
                            Box::new(Node {
                                value: Expression::Variable(String::from("a")),
                                position: default_position()
                            }),
                            Box::new(Node {
                                value: Expression::Variable(String::from("b")),
                                position: default_position()
                            })
                        ),
                        position: default_position()
                    }),
                    Box::new(Node {
                        value: Expression::Variable(String::from("c")),
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

            let node = parser.parse_relation_term().unwrap().unwrap();
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
            create_token(TokenCategory::Identifier, TokenValue::String(String::from("x"))),
            create_token(TokenCategory::ETX, TokenValue::Null),
        ];

        let mock_lexer = LexerMock::new(tokens);
        let mut parser = Parser::new(mock_lexer);

        let node = parser.parse_additive_term().unwrap().unwrap();
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
                        value: Expression::Variable(String::from("x")),
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
            create_token(TokenCategory::Identifier, TokenValue::String(String::from("x"))),
            create_token(TokenCategory::ETX, TokenValue::Null),
        ];

        let mock_lexer = LexerMock::new(tokens);
        let mut parser = Parser::new(mock_lexer);

        let node = parser.parse_multiplicative_term().unwrap().unwrap();
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
                        value: Expression::Variable(String::from("x")),
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

            let node = parser.parse_casted_term().unwrap().unwrap();
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

            let node = parser.parse_unary_term().unwrap().unwrap();
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
                create_token(TokenCategory::Identifier, TokenValue::String(String::from("print"))),
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
            Expression::Variable(String::from("print")),
        ];

        for (idx, series) in token_series.iter().enumerate() {
            let mock_lexer = LexerMock::new(series.to_vec());
            let mut parser = Parser::new(mock_lexer);

            let node = parser.parse_factor().unwrap().unwrap();
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
                create_token(TokenCategory::Identifier, TokenValue::String(String::from("print"))),
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
                    TokenValue::String(String::from("print")),
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
                create_token(TokenCategory::Identifier, TokenValue::String(String::from("print"))),
                create_token(TokenCategory::ETX, TokenValue::Null),
            ],
            vec![
                // print()
                create_token(TokenCategory::Identifier, TokenValue::String(String::from("print"))),
                create_token(TokenCategory::ParenOpen, TokenValue::Null),
                create_token(TokenCategory::ParenClose, TokenValue::Null),
                create_token(TokenCategory::ETX, TokenValue::Null),
            ],
            vec![
                // print(5)
                create_token(TokenCategory::Identifier, TokenValue::String(String::from("print"))),
                create_token(TokenCategory::ParenOpen, TokenValue::Null),
                create_token(TokenCategory::I64Value, TokenValue::I64(5)),
                create_token(TokenCategory::ParenClose, TokenValue::Null),
                create_token(TokenCategory::ETX, TokenValue::Null),
            ],
            vec![
                // print(5, x)
                create_token(TokenCategory::Identifier, TokenValue::String(String::from("print"))),
                create_token(TokenCategory::ParenOpen, TokenValue::Null),
                create_token(TokenCategory::Reference, TokenValue::Null),
                create_token(TokenCategory::I64Value, TokenValue::I64(5)),
                create_token(TokenCategory::Comma, TokenValue::Null),
                create_token(TokenCategory::Identifier, TokenValue::String(String::from("x"))),
                create_token(TokenCategory::ParenClose, TokenValue::Null),
                create_token(TokenCategory::ETX, TokenValue::Null),
            ],
        ];

        let expected = [
            Expression::Variable(String::from("print")),
            Expression::FunctionCall {
                identifier: Node {
                    value: String::from("print"),
                    position: default_position(),
                },
                arguments: vec![],
            },
            Expression::FunctionCall {
                identifier: Node {
                    value: String::from("print"),
                    position: default_position(),
                },
                arguments: vec![Box::new(Node {
                    value: Argument {
                        value: Node {
                            value: Expression::Literal(Literal::I64(5)),
                            position: default_position(),
                        },
                        passed_by: PassedBy::Value,
                    },
                    position: default_position(),
                })],
            },
            Expression::FunctionCall {
                identifier: Node {
                    value: String::from("print"),
                    position: default_position(),
                },
                arguments: vec![
                    Box::new(Node {
                        value: Argument {
                            value: Node {
                                value: Expression::Literal(Literal::I64(5)),
                                position: default_position(),
                            },
                            passed_by: PassedBy::Reference,
                        },
                        position: default_position(),
                    }),
                    Box::new(Node {
                        value: Argument {
                            value: Node {
                                value: Expression::Variable(String::from("x")),
                                position: default_position(),
                            },
                            passed_by: PassedBy::Value,
                        },
                        position: default_position(),
                    }),
                ],
            },
        ];

        for (idx, series) in token_series.iter().enumerate() {
            let mock_lexer = LexerMock::new(series.to_vec());
            let mut parser = Parser::new(mock_lexer);

            let node = parser.parse_identifier_or_call().unwrap().unwrap();
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
            create_token(TokenCategory::Identifier, TokenValue::String(String::from("x"))),
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
                        value: Expression::Variable(String::from("x")),
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

            let node = parser.parse_switch_statement().unwrap().unwrap();
            assert!(node.value == expected_types[idx]);
        }
    }

    #[test]
    fn parse_switch_expressions_fail() {
        let token_series = vec![vec![
            // x: temp,
            create_token(TokenCategory::Identifier, TokenValue::String(String::from("x"))),
            create_token(TokenCategory::Colon, TokenValue::Null),
            create_token(TokenCategory::Identifier, TokenValue::String(String::from("temp"))),
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
                create_token(TokenCategory::Identifier, TokenValue::String(String::from("x"))),
                create_token(TokenCategory::Colon, TokenValue::Null),
                create_token(TokenCategory::Identifier, TokenValue::String(String::from("temp"))),
                create_token(TokenCategory::Comma, TokenValue::Null),
                create_token(TokenCategory::Identifier, TokenValue::String(String::from("y"))),
                create_token(TokenCategory::ETX, TokenValue::Null),
            ],
            vec![
                // x
                create_token(TokenCategory::Identifier, TokenValue::String(String::from("x"))),
                create_token(TokenCategory::ETX, TokenValue::Null),
            ],
        ];

        let expected_types = [
            vec![
                Node {
                    value: SwitchExpression {
                        expression: Node {
                            value: Expression::Variable(String::from("x")),
                            position: default_position(),
                        },
                        alias: Some(Node {
                            value: String::from("temp"),
                            position: default_position(),
                        }),
                    },
                    position: default_position(),
                },
                Node {
                    value: SwitchExpression {
                        expression: Node {
                            value: Expression::Variable(String::from("y")),
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
                        value: Expression::Variable(String::from("x")),
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
                create_token(TokenCategory::Identifier, TokenValue::String(String::from("x"))),
                create_token(TokenCategory::Colon, TokenValue::Null),
                create_token(TokenCategory::Identifier, TokenValue::String(String::from("temp"))),
                create_token(TokenCategory::ETX, TokenValue::Null),
            ],
            vec![
                // x
                create_token(TokenCategory::Identifier, TokenValue::String(String::from("x"))),
                create_token(TokenCategory::ETX, TokenValue::Null),
            ],
        ];

        let expected_types = [
            SwitchExpression {
                expression: Node {
                    value: Expression::Variable(String::from("x")),
                    position: default_position(),
                },
                alias: Some(Node {
                    value: String::from("temp"),
                    position: default_position(),
                }),
            },
            SwitchExpression {
                expression: Node {
                    value: Expression::Variable(String::from("x")),
                    position: default_position(),
                },
                alias: None,
            },
        ];

        for (idx, series) in token_series.iter().enumerate() {
            let mock_lexer = LexerMock::new(series.to_vec());
            let mut parser = Parser::new(mock_lexer);

            let node = parser.parse_switch_expression().unwrap().unwrap();
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

            let node = parser.parse_switch_case().unwrap().unwrap();
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

            let node = parser.parse_type().unwrap().unwrap();
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

            assert!(parser.parse_type().is_ok());
            assert!(parser.parse_type().unwrap().is_none());
        }
    }

    #[test]
    fn parse_literals() {
        let tokens = vec![
            create_token(TokenCategory::True, TokenValue::Null),
            create_token(TokenCategory::False, TokenValue::Null),
            create_token(TokenCategory::StringValue, TokenValue::String(String::from("a"))),
            create_token(TokenCategory::I64Value, TokenValue::I64(5)),
            create_token(TokenCategory::F64Value, TokenValue::F64(5.0)),
            create_token(TokenCategory::ETX, TokenValue::Null),
        ];

        let mock_lexer = LexerMock::new(tokens);
        let mut parser = Parser::new(mock_lexer);

        let mut literal = parser.parse_literal().unwrap().unwrap();
        assert!(literal.value == Literal::True);

        literal = parser.parse_literal().unwrap().unwrap();
        assert!(literal.value == Literal::False);

        literal = parser.parse_literal().unwrap().unwrap();
        assert!(literal.value == Literal::String(String::from("a")));

        literal = parser.parse_literal().unwrap().unwrap();
        assert!(literal.value == Literal::I64(5));

        literal = parser.parse_literal().unwrap().unwrap();
        assert!(literal.value == Literal::F64(5.0));
    }

    #[test]
    fn parse_identifier() {
        let tokens = vec![
            create_token(TokenCategory::Identifier, TokenValue::String(String::from("print"))),
            create_token(TokenCategory::ETX, TokenValue::Null),
        ];

        let mock_lexer = LexerMock::new(tokens);
        let mut parser = Parser::new(mock_lexer);

        let node = parser.parse_identifier().unwrap().unwrap();
        assert!(node.value == String::from("print"));
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
