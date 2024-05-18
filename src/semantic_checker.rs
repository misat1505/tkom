use crate::{
    ast::{Expression, Node, Program, Statement},
    ast_visitor::AstVisitor,
    errors::Issue,
    functions_manager::FunctionsManager,
};

enum FunctionCallType {
    Statement(Node<Statement>),
    Expression(Node<Expression>),
}

#[derive(Debug)]
pub struct SemanticCheckerIssue {
    pub message: String,
}

impl Issue for SemanticCheckerIssue {
    fn message(&self) -> String {
        self.message.clone()
    }
}

pub struct SemanticChecker {
    program: Program,
    functions_manager: FunctionsManager,
    pub errors: Vec<SemanticCheckerIssue>,
}

impl SemanticChecker {
    pub fn new(program: Program) -> Result<Self, Box<dyn Issue>> {
        let functions_manager = FunctionsManager::new(&program)?;
        let errors: Vec<SemanticCheckerIssue> = vec![];
        Ok(Self {
            program,
            functions_manager,
            errors,
        })
    }

    pub fn check(&mut self) {
        self.visit_program(&self.program.clone());
    }

    fn check_function_call(&mut self, function: FunctionCallType) {
        match function {
            FunctionCallType::Statement(Node {
                value:
                    Statement::FunctionCall {
                        identifier,
                        arguments,
                    },
                position,
            })
            | FunctionCallType::Expression(Node {
                value:
                    Expression::FunctionCall {
                        identifier,
                        arguments,
                    },
                position,
            }) => {
                let name = &identifier.value.0;
                match self.functions_manager.clone().get(name.clone()) {
                    None => self.errors.push(SemanticCheckerIssue {
                        message: format!(
                            "Use of undeclared function '{}'.\nAt {:?}.\n",
                            name, position
                        ),
                    }),
                    Some(function_declaration) => {
                        if let Statement::FunctionDeclaration { parameters, .. } =
                            function_declaration
                        {
                            if arguments.len() != parameters.len() {
                                self.errors.push(SemanticCheckerIssue { message: format!("Invalid number of arguments for function '{}'. Expected {}, given {}.\nAt {:?}.\n", name, parameters.len(), arguments.len(), position) })
                            }

                            for idx in 0..parameters.len() {
                                let parameter = parameters.get(idx).unwrap();
                                match arguments.get(idx) {
                                    None => {}
                                    Some(argument) => {
                                        if argument.value.passed_by != parameter.value.passed_by {
                                            self.errors.push(SemanticCheckerIssue { message: format!("Parameter '{}' in function '{}' passed by {:?} - should be passed by {:?}.\nAt {:?}.\n", parameter.value.identifier.value.0, identifier.value.0, argument.value.passed_by, parameter.value.passed_by, argument.position) });
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

impl AstVisitor for SemanticChecker {
    fn visit_program(&mut self, program: &Program) {
        for statement in &program.statements {
            self.visit_statement(statement);
        }
    }

    fn visit_statement(&mut self, statement: &Node<Statement>) {
        match &statement.value {
            Statement::FunctionCall { .. } => {
                self.check_function_call(FunctionCallType::Statement(statement.clone()));
            }
            _ => {}
        }

        match &statement.value {
            Statement::FunctionDeclaration { block, .. } => {
                for statement in block.value.0.clone() {
                    self.visit_statement(&statement);
                }
            }
            Statement::FunctionCall { arguments, .. } => {
                for argument in arguments.clone() {
                    let arg_node = Node {
                        value: argument.value.value,
                        position: argument.position,
                    };
                    self.visit_expression(&arg_node);
                }
            }
            Statement::Declaration { value, .. } => match value {
                Some(expr) => self.visit_expression(expr),
                None => {}
            },
            Statement::Assignment { value, .. } => {
                self.visit_expression(value);
            }
            Statement::Conditional {
                condition,
                if_block,
                else_block,
            } => {
                self.visit_expression(condition);
                for statement in if_block.value.0.clone() {
                    self.visit_statement(&statement);
                }
                match else_block {
                    Some(block) => {
                        for statement in block.value.0.clone() {
                            self.visit_statement(&statement);
                        }
                    }
                    None => {}
                }
            }
            Statement::ForLoop {
                declaration,
                condition,
                assignment,
                block,
            } => {
                match declaration {
                    None => {}
                    Some(decl) => {
                        self.visit_statement(decl);
                    }
                }
                self.visit_expression(condition);
                match assignment {
                    None => {}
                    Some(assign) => {
                        self.visit_statement(assign);
                    }
                }
                for statement in block.value.0.clone() {
                    self.visit_statement(&statement);
                }
            }
            Statement::Switch { expressions, cases } => {
                for switch_expr in expressions {
                    self.visit_expression(&switch_expr.value.expression);
                }

                for case in cases {
                    self.visit_expression(&case.value.condition);

                    for statement in case.value.block.value.0.clone() {
                        self.visit_statement(&statement);
                    }
                }
            }
            Statement::Return(value) => match value {
                None => {}
                Some(val) => {
                    self.visit_expression(val);
                }
            },
            _ => {}
        }
    }

    fn visit_expression(&mut self, expression: &Node<Expression>) {
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
                self.visit_expression(lhs);
                self.visit_expression(rhs);
            }
            Expression::BooleanNegation(inner) | Expression::ArithmeticNegation(inner) => {
                self.visit_expression(inner);
            }
            Expression::Casting { value, .. } => {
                self.visit_expression(value);
            }
            Expression::FunctionCall { arguments, .. } => {
                for argument in arguments {
                    let arg_node = Node {
                        value: argument.value.value.clone(),
                        position: argument.position,
                    };
                    self.visit_expression(&arg_node);
                }
            }
            _ => {}
        }
    }
}
