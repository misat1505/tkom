use crate::{
    ast::{Argument, Block, Expression, Literal, Node, Parameter, PassedBy, Program, Statement, SwitchCase, SwitchExpression, Type},
    issues::{Issue, IssueLevel, SemanticCheckerIssue},
    visitor::Visitor,
};

enum FunctionCallType {
    Statement(Node<Statement>),
    Expression(Node<Expression>),
}

pub struct SemanticChecker<'a> {
    program: &'a Program,
    pub errors: Vec<SemanticCheckerIssue>,
}

impl<'a> SemanticChecker<'a> {
    #![allow(unused_must_use)]
    pub fn new(program: &'a Program) -> Result<Self, Box<dyn Issue>> {
        let errors: Vec<SemanticCheckerIssue> = vec![];
        Ok(Self { program, errors })
    }

    pub fn check(&mut self) {
        self.visit_program(self.program);
    }

    fn check_function_call(&mut self, function: FunctionCallType) {
        match function {
            FunctionCallType::Statement(Node {
                value: Statement::FunctionCall { identifier, arguments },
                position,
            })
            | FunctionCallType::Expression(Node {
                value: Expression::FunctionCall { identifier, arguments },
                position,
            }) => {
                let name = &identifier.value;

                // std function
                if let Some(std_function) = self.program.std_functions.get(&String::from(name)) {
                    if arguments.len() != std_function.params.len() {
                        self.errors.push(SemanticCheckerIssue::new(
                            IssueLevel::ERROR,
                            format!(
                                "Invalid number of arguments for function '{}'. Expected {}, given {}.\nAt {:?}.\n",
                                name,
                                std_function.params.len(),
                                arguments.len(),
                                position
                            ),
                        ));
                    }

                    for argument in arguments {
                        if argument.value.passed_by == PassedBy::Reference {
                            self.errors.push(SemanticCheckerIssue::new(
                                IssueLevel::ERROR,
                                format!(
                                    "Parameter in function '{}' passed by {:?} - should be passed by {:?}.\nAt {:?}.\n",
                                    identifier.value,
                                    argument.value.passed_by,
                                    PassedBy::Value,
                                    argument.position
                                ),
                            ))
                        }
                    }

                    return;
                }

                // user function
                if let Some(function_declaration) = self.program.functions.get(&String::from(name)) {
                    let parameters = &function_declaration.value.parameters;
                    if arguments.len() != parameters.len() {
                        self.errors.push(SemanticCheckerIssue::new(
                            IssueLevel::ERROR,
                            format!(
                                "Invalid number of arguments for function '{}'. Expected {}, given {}.\nAt {:?}.\n",
                                name,
                                parameters.len(),
                                arguments.len(),
                                position
                            ),
                        ))
                    }

                    for idx in 0..parameters.len() {
                        let parameter = parameters.get(idx).unwrap();
                        if let Some(argument) = arguments.get(idx) {
                            if argument.value.passed_by != parameter.value.passed_by {
                                self.errors.push(SemanticCheckerIssue::new(
                                    IssueLevel::ERROR,
                                    format!(
                                        "Parameter '{}' in function '{}' passed by {:?} - should be passed by {:?}.\nAt {:?}.\n",
                                        parameter.value.identifier.value,
                                        identifier.value,
                                        argument.value.passed_by,
                                        parameter.value.passed_by,
                                        argument.position
                                    ),
                                ));
                            }

                            if argument.value.passed_by == PassedBy::Reference {
                                if let Expression::Variable(_) = argument.value.value.value {
                                } else {
                                    self.errors.push(SemanticCheckerIssue::new(IssueLevel::ERROR, format!(
                                            "Parameter '{}' in function '{}' is passed by {:?}. Thus it needs to an identifier, but a complex expression was found.\nAt {:?}.\n",
                                            parameter.value.identifier.value,
                                            identifier.value,
                                            PassedBy::Reference,
                                            argument.position
                                        ),
                                    ));
                                }
                            }
                        }
                    }

                    return;
                }

                self.errors.push(SemanticCheckerIssue::new(
                    IssueLevel::ERROR,
                    format!("Use of undeclared function '{}'.\nAt {:?}.\n", name, position),
                ))
            }
            _ => {}
        }
    }
}

impl<'a> Visitor<'a> for SemanticChecker<'a> {
    #![allow(unused_must_use)]
    fn visit_program(&mut self, program: &'a Program) -> Result<(), Box<dyn Issue>> {
        for statement in &program.statements {
            self.visit_statement(&statement);
        }

        for (_, function) in &program.functions {
            self.visit_block(&function.value.block);
        }
        Ok(())
    }

    fn visit_expression(&mut self, expression: &'a Node<Expression>) -> Result<(), Box<dyn Issue>> {
        match &expression.value {
            Expression::FunctionCall { .. } => {
                self.check_function_call(FunctionCallType::Expression(expression.clone()));
            }
            _ => {}
        }

        match &expression.value {
            Expression::Alternative(lhs, rhs)
            | Expression::Concatenation(lhs, rhs)
            | Expression::Greater(lhs, rhs)
            | Expression::GreaterEqual(lhs, rhs)
            | Expression::Less(lhs, rhs)
            | Expression::LessEqual(lhs, rhs)
            | Expression::Equal(lhs, rhs)
            | Expression::NotEqual(lhs, rhs)
            | Expression::Addition(lhs, rhs)
            | Expression::Subtraction(lhs, rhs)
            | Expression::Multiplication(lhs, rhs)
            | Expression::Division(lhs, rhs) => {
                self.visit_expression(&lhs);
                self.visit_expression(&rhs);
            }
            Expression::BooleanNegation(value) | Expression::ArithmeticNegation(value) | Expression::Casting { value, .. } => {
                self.visit_expression(&value);
            }
            Expression::Literal(literal) => {
                self.visit_literal(&literal);
            }
            Expression::Variable(variable) => {
                self.visit_variable(&variable);
            }
            Expression::FunctionCall { arguments, .. } => {
                for arg in arguments {
                    self.visit_argument(&arg);
                }
            }
        }
        Ok(())
    }

    fn visit_statement(&mut self, statement: &'a Node<Statement>) -> Result<(), Box<dyn Issue>> {
        match &statement.value {
            &Statement::FunctionCall { .. } => {
                self.check_function_call(FunctionCallType::Statement(statement.clone()));
            }
            _ => {}
        }

        match &statement.value {
            Statement::FunctionCall { arguments, .. } => {
                for arg in arguments {
                    self.visit_argument(&arg);
                }
            }
            Statement::Declaration { var_type, value, .. } => {
                self.visit_type(&var_type);
                if let Some(val) = value {
                    self.visit_expression(&val);
                }
            }
            Statement::Assignment { value, .. } => {
                self.visit_expression(&value);
            }
            Statement::Conditional {
                condition,
                if_block,
                else_block,
            } => {
                self.visit_expression(&condition);
                self.visit_block(&if_block);
                if let Some(else_blk) = else_block {
                    self.visit_block(&else_blk);
                }
            }
            Statement::ForLoop {
                declaration,
                condition,
                assignment,
                block,
            } => {
                if let Some(decl) = declaration {
                    self.visit_statement(&decl);
                }
                self.visit_expression(&condition);
                if let Some(assign) = assignment {
                    self.visit_statement(&assign);
                }
                self.visit_block(&block);
            }
            Statement::Switch { expressions, cases } => {
                for expr in expressions {
                    self.visit_switch_expression(&expr);
                }
                for case in cases {
                    self.visit_switch_case(&case);
                }
            }
            Statement::Return(value) => {
                if let Some(val) = value {
                    self.visit_expression(&val);
                }
            }
            Statement::Break => {}
        }
        Ok(())
    }

    fn visit_argument(&mut self, argument: &'a Node<Argument>) -> Result<(), Box<dyn Issue>> {
        self.visit_expression(&argument.value.value);
        Ok(())
    }

    fn visit_block(&mut self, block: &'a Node<Block>) -> Result<(), Box<dyn Issue>> {
        for statement in &block.value.0 {
            self.visit_statement(statement);
        }
        Ok(())
    }

    fn visit_parameter(&mut self, parameter: &'a Node<Parameter>) -> Result<(), Box<dyn Issue>> {
        self.visit_type(&parameter.value.parameter_type);
        Ok(())
    }

    fn visit_switch_case(&mut self, switch_case: &'a Node<SwitchCase>) -> Result<(), Box<dyn Issue>> {
        self.visit_expression(&switch_case.value.condition);
        self.visit_block(&switch_case.value.block);
        Ok(())
    }

    fn visit_switch_expression(&mut self, switch_expression: &'a Node<SwitchExpression>) -> Result<(), Box<dyn Issue>> {
        self.visit_expression(&switch_expression.value.expression);
        Ok(())
    }

    fn visit_type(&mut self, _node_type: &'a Node<Type>) -> Result<(), Box<dyn Issue>> {
        // println!("{:?}", _node_type);
        Ok(())
    }

    fn visit_literal(&mut self, _literal: &'a Literal) -> Result<(), Box<dyn Issue>> {
        // println!("{:?}", _literal);
        Ok(())
    }

    fn visit_variable(&mut self, _variable: &'a String) -> Result<(), Box<dyn Issue>> {
        // println!("{:?}", _variable);
        Ok(())
    }
}
