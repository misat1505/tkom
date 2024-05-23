use crate::{
    ast::{
        Argument, Block, Expression, Literal, Node, Parameter, Program, Statement,
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
    #![allow(unused_must_use)]
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
                let name = &identifier.value;
                if let Some(std_function) = self
                    .functions_manager
                    .std_functions
                    .get(&String::from(name))
                {
                    if arguments.len() != std_function.params.len() {
                        self.errors.push(SemanticCheckerIssue { message: format!("Invalid number of arguments for function '{}'. Expected {}, given {}.\nAt {:?}.\n", name, std_function.params.len(), arguments.len(), position) });
                    }
                    return;
                }
                match self.functions_manager.functions.get(&String::from(name)) {
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
                                            self.errors.push(SemanticCheckerIssue { message: format!("Parameter '{}' in function '{}' passed by {:?} - should be passed by {:?}.\nAt {:?}.\n", parameter.value.identifier.value, identifier.value, argument.value.passed_by, parameter.value.passed_by, argument.position) });
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
    #![allow(unused_must_use)]
    fn visit_program(&mut self, program: &Program) -> Result<(), Box<dyn Issue>> {
        for statement in program.statements.clone() {
            self.visit_statement(&statement);
        }
        Ok(())
    }

    fn visit_expression(&mut self, expression: &Node<Expression>) -> Result<(), Box<dyn Issue>> {
        match &expression.value {
            Expression::FunctionCall { .. } => {
                self.check_function_call(FunctionCallType::Expression(expression.clone()));
            }
            _ => {}
        }

        match expression.value.clone() {
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
            Expression::BooleanNegation(value)
            | Expression::ArithmeticNegation(value)
            | Expression::Casting { value, .. } => {
                self.visit_expression(&value);
            }
            Expression::Literal(literal) => {
                self.visit_literal(literal);
            }
            Expression::Variable(variable) => {
                self.visit_variable(variable);
            }
            Expression::FunctionCall {
                arguments,
                ..
            } => {
                for arg in arguments {
                    self.visit_argument(&arg);
                }
            }
        }
        Ok(())
    }

    fn visit_statement(&mut self, statement: &Node<Statement>) -> Result<(), Box<dyn Issue>> {
        match &statement.value {
            &Statement::FunctionCall { .. } => {
                self.check_function_call(FunctionCallType::Statement(statement.clone()));
            }
            _ => {}
        }

        match statement.value.clone() {
            Statement::FunctionDeclaration {
                parameters,
                return_type,
                block,
                ..
            } => {
                for param in parameters {
                    self.visit_parameter(&param);
                }
                self.visit_type(&return_type);
                self.visit_block(&block);
            }
            Statement::FunctionCall {
                arguments,
                ..
            } => {
                for arg in arguments {
                    self.visit_argument(&arg);
                }
            }
            Statement::Declaration {
                var_type,
                value,
                ..
            } => {
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

    fn visit_argument(&mut self, argument: &Node<Argument>) -> Result<(), Box<dyn Issue>> {
        self.visit_expression(&argument.value.value);
        Ok(())
    }

    fn visit_block(&mut self, block: &Node<Block>) -> Result<(), Box<dyn Issue>> {
        for statement in &block.value.0 {
            self.visit_statement(statement);
        }
        Ok(())
    }

    fn visit_parameter(&mut self, parameter: &Node<Parameter>) -> Result<(), Box<dyn Issue>> {
        self.visit_type(&parameter.value.parameter_type);
        Ok(())
    }

    fn visit_switch_case(&mut self, switch_case: &Node<SwitchCase>) -> Result<(), Box<dyn Issue>> {
        self.visit_expression(&switch_case.value.condition);
        self.visit_block(&switch_case.value.block);
        Ok(())
    }

    fn visit_switch_expression(
        &mut self,
        switch_expression: &Node<SwitchExpression>,
    ) -> Result<(), Box<dyn Issue>> {
        self.visit_expression(&switch_expression.value.expression);
        Ok(())
    }

    fn visit_type(&mut self, _node_type: &Node<Type>) -> Result<(), Box<dyn Issue>> {
        // println!("{:?}", _node_type);
        Ok(())
    }

    fn visit_literal(&mut self, _literal: Literal) -> Result<(), Box<dyn Issue>> {
        // println!("{:?}", _literal);
        Ok(())
    }

    fn visit_variable(&mut self, _variable: String) -> Result<(), Box<dyn Issue>> {
        // println!("{:?}", _variable);
        Ok(())
    }
}
