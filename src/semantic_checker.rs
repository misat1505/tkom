use crate::{
    ast::{Expression, Node, Program, Statement},
    ast_visitor::AstVisitor,
    errors::Issue,
    functions_manager::FunctionsManager,
    lazy_stream_reader::Position,
};

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

    fn check_function_call(&mut self, name: String, arguments_count: usize, position: Position) {
        match self.functions_manager.clone().get(name.clone()) {
            None => self.errors.push(SemanticCheckerIssue {
                message: format!("Use of undeclared function '{}' at {:?}.", name, position),
            }),
            Some(function_declaration) => {
                if let Statement::FunctionDeclaration { parameters, .. } = function_declaration {
                    if arguments_count != parameters.len() {
                        self.errors.push(SemanticCheckerIssue { message: format!("Invalid number of arguments for function '{}'. Expected {}, given {}. at {:?}.", name, parameters.len(), arguments_count, position) })
                    }
                }
            }
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
            Statement::FunctionCall {
                identifier,
                arguments,
            } => {
              let function_name = identifier.value.0.to_string();
              self.check_function_call(function_name, arguments.len(), statement.position);
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
            Expression::FunctionCall {
                identifier,
                arguments,
            } => {
              let function_name = identifier.0.to_string();
              self.check_function_call(function_name, arguments.len(), expression.position);
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
