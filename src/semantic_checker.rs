use crate::{
    ast::{
        Argument, Block, Expression, Identifier, Literal, Node, Parameter, Program, Statement,
        SwitchCase, SwitchExpression, Type,
    },
    errors::Issue,
    functions_manager::FunctionsManager,
    visitor::Visitor,
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

impl Visitor for SemanticChecker {
    fn visit_program(&mut self, program: &Program) {
        for statement in program.statements.clone() {
            statement.accept(self);
        }
    }

    fn visit_expression(&mut self, expression: &Node<Expression>) {
        match &expression.value {
            Expression::FunctionCall { .. } => {
                self.check_function_call(FunctionCallType::Expression(expression.clone()));
            }
            _ => {}
        }

        // expression.accept(self);
    }

    fn visit_statement(&mut self, statement: &Node<Statement>) {
        match &statement.value {
            &Statement::FunctionCall { .. } => {
                self.check_function_call(FunctionCallType::Statement(statement.clone()));
            }
            _ => {}
        }
    }

    fn visit_argument(&mut self, argument: &Node<Argument>) {
        self.visit_expression(&argument.value.value);
        argument.value.value.accept(self);
    }

    fn visit_block(&mut self, block: &Node<Block>) {
        for statement in &block.value.0 {
            statement.accept(self);
        }
    }

    fn visit_identifier(&mut self, _identifier: &Node<Identifier>) {}

    fn visit_parameter(&mut self, _parameter: &Node<Parameter>) {}

    fn visit_switch_case(&mut self, switch_case: &Node<SwitchCase>) {
        switch_case.value.condition.accept(self);
        switch_case.value.block.accept(self);
    }

    fn visit_switch_expression(&mut self, switch_expression: &Node<SwitchExpression>) {
        switch_expression.value.expression.accept(self);
    }

    fn visit_type(&mut self, _node_type: &Node<Type>) {}

    fn visit_literal(&mut self, _literal: Literal) {
        println!("{:?}", _literal);
    }

    fn visit_variable(&mut self, _variable: Identifier) {}
}
