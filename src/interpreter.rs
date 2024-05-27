use crate::{
    ast::{
        Argument, Block, Expression, FunctionDeclaration, Literal, Node, Parameter, PassedBy, Program, Statement, SwitchCase, SwitchExpression, Type,
    },
    errors::Issue,
    functions_manager::FunctionsManager,
    lazy_stream_reader::Position,
    stack::Stack,
    std_functions::StdFunction,
    value::{ComputationIssue, Value},
    visitor::Visitor,
    ALU::ALU,
};

#[derive(Debug)]
pub struct InterpreterIssue {
    message: String,
}

impl Issue for InterpreterIssue {
    fn message(&self) -> String {
        self.message.clone()
    }
}

pub struct Interpreter {
    program: Program,
    functions_manager: FunctionsManager,
    pub stack: Stack,
    last_result: Option<Value>,
    is_breaking: bool,
    is_returning: bool,
    position: Position,
    last_arguments: Vec<Value>,
    returned_arguments: Vec<Value>,
}

impl Interpreter {
    pub fn new(program: Program) -> Self {
        Interpreter {
            program: program.clone(),
            functions_manager: FunctionsManager::new(&program).unwrap(),
            stack: Stack::new(),
            last_result: None,
            is_breaking: false,
            is_returning: false,
            position: Position {
                line: 0,
                column: 0,
                offset: 0,
            },
            last_arguments: vec![],
            returned_arguments: vec![],
        }
    }

    pub fn interpret(&mut self) -> Result<(), Box<dyn Issue>> {
        self.visit_program(&self.program.clone())
    }

    fn read_last_result(&mut self) -> Result<Value, Box<dyn Issue>> {
        match self.last_result.clone() {
            Some(result) => {
                self.last_result = None;
                Ok(result)
            }
            None => Err(Box::new(InterpreterIssue {
                message: format!("No value produced where it is needed.\nAt {:?}.", self.position),
            })),
        }
    }

    fn evaluate_binary_op<F>(&mut self, lhs: &Box<Node<Expression>>, rhs: &Box<Node<Expression>>, op: F) -> Result<(), Box<dyn Issue>>
    where
        F: Fn(Value, Value) -> Result<Value, ComputationIssue>,
    {
        self.visit_expression(lhs)?;
        let left_value = self.read_last_result()?;
        self.visit_expression(rhs)?;
        let right_value = self.read_last_result()?;

        match op(left_value, right_value) {
            Ok(val) => {
                self.last_result = Some(val);
                Ok(())
            }
            Err(mut err) => {
                err.message = format!("{}\nAt {:?}.", err.message, self.position);
                Err(Box::new(err))
            }
        }
    }

    fn evaluate_unary_op<F>(&mut self, value: &Box<Node<Expression>>, op: F) -> Result<(), Box<dyn Issue>>
    where
        F: Fn(Value) -> Result<Value, ComputationIssue>,
    {
        self.visit_expression(value)?;
        let computed_value = self.read_last_result()?;
        match op(computed_value) {
            Ok(val) => {
                self.last_result = Some(val);
                Ok(())
            }
            Err(mut err) => {
                err.message = format!("{}\nAt {:?}.", err.message, self.position);
                Err(Box::new(err))
            }
        }
    }
}

impl Visitor for Interpreter {
    fn visit_program(&mut self, program: &Program) -> Result<(), Box<dyn Issue>> {
        for statement in &program.statements {
            // if let Statement::FunctionDeclaration { .. } = statement.value {
            //     continue;
            // }

            self.visit_statement(&statement)?;
            if self.is_breaking {
                return Err(Box::new(InterpreterIssue {
                    message: format!("Break called outside 'for' or 'switch'.\nAt {:?}.", self.position),
                }));
            }

            if self.is_returning {
                return Err(Box::new(InterpreterIssue {
                    message: format!("Return called outside a function.\nAt {:?}.", self.position),
                }));
            }
        }
        Ok(())
    }

    fn visit_expression(&mut self, expression: &Node<Expression>) -> Result<(), Box<dyn Issue>> {
        self.position = expression.position;
        match &expression.value {
            Expression::Casting { value, to_type } => {
                self.visit_expression(&value)?;
                let computed_value = self.read_last_result()?;
                match ALU::cast_to_type(computed_value, to_type.value) {
                    Ok(val) => {
                        self.last_result = Some(val);
                        return Ok(());
                    }
                    Err(mut err) => {
                        err.message = format!("{}\nAt: {:?}", err.message, self.position);
                        return Err(Box::new(err));
                    }
                }
            }
            Expression::BooleanNegation(value) => self.evaluate_unary_op(value, ALU::boolean_negate)?,
            Expression::ArithmeticNegation(value) => self.evaluate_unary_op(value, ALU::arithmetic_negate)?,
            Expression::Addition(lhs, rhs) => self.evaluate_binary_op(lhs, rhs, ALU::add)?,
            Expression::Subtraction(lhs, rhs) => self.evaluate_binary_op(lhs, rhs, ALU::subtract)?,
            Expression::Multiplication(lhs, rhs) => self.evaluate_binary_op(lhs, rhs, ALU::multiplication)?,
            Expression::Division(lhs, rhs) => self.evaluate_binary_op(lhs, rhs, ALU::division)?,
            Expression::Alternative(lhs, rhs) => self.evaluate_binary_op(lhs, rhs, ALU::alternative)?,
            Expression::Concatenation(lhs, rhs) => self.evaluate_binary_op(lhs, rhs, ALU::concatenation)?,
            Expression::Greater(lhs, rhs) => self.evaluate_binary_op(lhs, rhs, ALU::greater)?,
            Expression::GreaterEqual(lhs, rhs) => self.evaluate_binary_op(lhs, rhs, ALU::greater_or_equal)?,
            Expression::Less(lhs, rhs) => self.evaluate_binary_op(lhs, rhs, ALU::less)?,
            Expression::LessEqual(lhs, rhs) => self.evaluate_binary_op(lhs, rhs, ALU::less_or_equal)?,
            Expression::Equal(lhs, rhs) => self.evaluate_binary_op(lhs, rhs, ALU::equal)?,
            Expression::NotEqual(lhs, rhs) => self.evaluate_binary_op(lhs, rhs, ALU::not_equal)?,
            Expression::Literal(literal) => self.visit_literal(literal.clone())?,
            Expression::Variable(variable) => self.visit_variable(variable.clone())?,
            Expression::FunctionCall { identifier, arguments } => self.call_function(identifier, arguments)?,
        }
        Ok(())
    }

    fn visit_statement(&mut self, statement: &Node<Statement>) -> Result<(), Box<dyn Issue>> {
        self.position = statement.position;
        match &statement.value {
            // Statement::FunctionDeclaration { .. } => self.execute_function(&statement.value)?,
            Statement::FunctionCall { identifier, arguments } => self.call_function(identifier, arguments)?,
            Statement::Declaration { var_type, identifier, value } => {
                self.visit_type(&var_type)?;

                let computed_value = match value {
                    Some(val) => {
                        self.visit_expression(&val)?;
                        let result = match self.read_last_result() {
                            Ok(val) => val,
                            Err(_) => {
                                return Err(Box::new(InterpreterIssue {
                                    message: format!("Cannot declare variable '{}' with no value.\nAt {:?}.", identifier.value, self.position),
                                }))
                            }
                        };
                        result
                    }
                    None => match Value::default_value(var_type.value) {
                        Ok(val) => val,
                        Err(err) => return Err(Box::new(err)),
                    },
                };

                match (var_type.value, computed_value.clone()) {
                    (Type::I64, Value::I64(_)) | (Type::F64, Value::F64(_)) | (Type::Str, Value::String(_)) | (Type::Bool, Value::Bool(_)) => {}
                    (declared_type, computed_type) => {
                        return Err(Box::new(InterpreterIssue {
                            message: format!(
                                "Cannot assign value of type '{:?}' to variable '{}' of type '{:?}'.\nAt {:?}.",
                                computed_type.to_type(),
                                identifier.value,
                                declared_type,
                                self.position
                            ),
                        }))
                    }
                }

                if let Err(mut err) = self.stack.declare_variable(identifier.value.clone(), computed_value) {
                    err.message = format!("{}\nAt {:?}.", err.message, self.position);
                    return Err(Box::new(err));
                }
            }
            Statement::Assignment { identifier, value } => {
                self.visit_expression(&value)?;
                let value = match self.read_last_result() {
                    Ok(val) => val,
                    Err(_) => {
                        return Err(Box::new(InterpreterIssue {
                            message: format!("Cannot assign no value to variable '{}'.\nAt {:?}.", identifier.value, self.position),
                        }))
                    }
                };
                if let Err(mut err) = self.stack.assign_variable(identifier.value.clone(), value) {
                    err.message = format!("{}\nAt {:?}.", err.message, self.position);
                    return Err(Box::new(err));
                }
            }
            Statement::Conditional {
                condition,
                if_block,
                else_block,
            } => {
                self.visit_expression(&condition)?;
                let computed_condition = self.read_last_result()?;
                let boolean_value = match computed_condition {
                    Value::Bool(bool) => bool,
                    a => {
                        return Err(Box::new(InterpreterIssue {
                            message: format!(
                                "Condition in if statement has to evaulate to type '{:?}' - got '{:?}'.\nAt {:?}.",
                                Type::Bool,
                                a.to_type(),
                                self.position
                            ),
                        }))
                    }
                };
                if boolean_value {
                    self.visit_block(&if_block)?;
                } else {
                    if let Some(else_blk) = else_block {
                        self.visit_block(&else_blk)?;
                    }
                }
            }
            Statement::ForLoop {
                declaration,
                condition,
                assignment,
                block,
            } => {
                self.stack.push_scope();
                if let Some(decl) = declaration {
                    self.visit_statement(&decl)?;
                }

                self.visit_expression(&condition)?;
                let mut computed_condition = self.read_last_result()?;
                let mut boolean_value = match computed_condition {
                    Value::Bool(bool) => bool,
                    a => {
                        return Err(Box::new(InterpreterIssue {
                            message: format!(
                                "Condition in for statement has to evaulate to type '{:?}' - got '{:?}'.\nAt {:?}.",
                                Type::Bool,
                                a.to_type(),
                                self.position
                            ),
                        }))
                    }
                };

                while boolean_value {
                    self.visit_block(&block)?;

                    if self.is_returning {
                        break;
                    }

                    if self.is_breaking {
                        self.is_breaking = false;
                        break;
                    }

                    if let Some(assign) = assignment {
                        self.visit_statement(&assign)?;
                    }

                    self.visit_expression(&condition)?;
                    computed_condition = self.read_last_result()?;
                    boolean_value = match computed_condition {
                        Value::Bool(bool) => bool,
                        a => {
                            return Err(Box::new(InterpreterIssue {
                                message: format!(
                                    "Condition in for statement has to evaulate to '{:?}' - got '{:?}'.\nAt {:?}.",
                                    Type::Bool,
                                    a.to_type(),
                                    self.position
                                ),
                            }))
                        }
                    };
                }
                self.stack.pop_scope();
            }
            Statement::Switch { expressions, cases } => {
                self.stack.push_scope();
                for expr in expressions {
                    self.visit_switch_expression(&expr)?;
                }
                for case in cases {
                    self.visit_switch_case(&case)?;
                    if self.is_returning {
                        break;
                    }

                    if self.is_breaking {
                        self.is_breaking = false;
                        break;
                    }
                }
                self.stack.pop_scope();
            }
            Statement::Return(value) => {
                let returned_value = match value {
                    Some(val) => {
                        self.visit_expression(&val)?;
                        Some(self.read_last_result()?)
                    }
                    None => None,
                };
                self.is_returning = true;
                self.last_result = returned_value;
            }
            Statement::Break => {
                self.is_breaking = true;
            }
        }
        Ok(())
    }

    fn visit_argument(&mut self, argument: &Node<Argument>) -> Result<(), Box<dyn Issue>> {
        self.visit_expression(&argument.value.value)?;
        Ok(())
    }

    fn visit_block(&mut self, block: &Node<Block>) -> Result<(), Box<dyn Issue>> {
        self.stack.push_scope();
        for statement in &block.value.0 {
            if self.is_breaking || self.is_returning {
                break;
            }

            self.visit_statement(statement)?;
        }
        self.stack.pop_scope();
        Ok(())
    }

    fn visit_parameter(&mut self, parameter: &Node<Parameter>) -> Result<(), Box<dyn Issue>> {
        self.visit_type(&parameter.value.parameter_type)?;
        Ok(())
    }

    fn visit_switch_case(&mut self, switch_case: &Node<SwitchCase>) -> Result<(), Box<dyn Issue>> {
        self.visit_expression(&switch_case.value.condition)?;
        let computed_value = self.read_last_result()?;
        let boolean_value = match computed_value {
            Value::Bool(bool) => bool,
            a => {
                return Err(Box::new(InterpreterIssue {
                    message: format!(
                        "Condition in switch case has to evaluate to type '{:?}' - got '{:?}'.\nAt {:?}.",
                        Type::Bool,
                        a.to_type(),
                        self.position
                    ),
                }))
            }
        };
        if boolean_value {
            self.visit_block(&switch_case.value.block)?;
        }
        Ok(())
    }

    fn visit_switch_expression(&mut self, switch_expression: &Node<SwitchExpression>) -> Result<(), Box<dyn Issue>> {
        if let Some(alias) = &switch_expression.value.alias {
            self.visit_expression(&switch_expression.value.expression)?;
            let computed_value = self.read_last_result()?;
            if let Err(mut err) = self.stack.declare_variable(alias.value.clone(), computed_value) {
                err.message = format!("{}\nAt {:?}.", err.message, self.position);
                return Err(Box::new(err));
            }
        }
        Ok(())
    }

    fn visit_type(&mut self, _node_type: &Node<Type>) -> Result<(), Box<dyn Issue>> {
        Ok(())
    }

    fn visit_literal(&mut self, literal: Literal) -> Result<(), Box<dyn Issue>> {
        // change literal to value
        let value = match literal {
            Literal::F64(f64) => Value::F64(f64),
            Literal::I64(i64) => Value::I64(i64),
            Literal::String(str) => Value::String(str),
            Literal::False => Value::Bool(false),
            Literal::True => Value::Bool(true),
        };

        self.last_result = Some(value);
        Ok(())
    }

    fn visit_variable(&mut self, variable: String) -> Result<(), Box<dyn Issue>> {
        // read value of variable
        let value = match self.stack.get_variable(variable) {
            Ok(val) => val,
            Err(err) => return Err(Box::new(err)),
        };
        self.last_result = Some(value.clone());
        Ok(())
    }
}

impl Interpreter {
    fn execute_std_function(std_function: &StdFunction, arguments: Vec<Value>) -> Result<Option<Value>, Box<dyn Issue>> {
        return match (std_function.execute)(arguments) {
            Ok(val) => Ok(val),
            Err(err) => Err(Box::new(err)),
        };
    }

    fn call_function(&mut self, identifier: &Node<String>, arguments: &Vec<Box<Node<Argument>>>) -> Result<(), Box<dyn Issue>> {
        let name = identifier.value.clone();

        let mut args: Vec<Value> = vec![];
        for arg in arguments {
            self.visit_expression(&arg.value.value)?;
            let value = self.read_last_result()?;
            args.push(value);
        }

        self.last_arguments = args.clone();

        if let Some(std_function) = self.functions_manager.std_functions.get(&name) {
            if let Some(return_value) = Self::execute_std_function(std_function, args.clone())? {
                self.last_result = Some(return_value);
            }
        }

        if let Some(function_declaration) = self.program.functions.get(&name).cloned() {
            // self.visit_statement(&function_declaration)?;
            self.execute_function(&function_declaration.value)?;
        }

        // update these passed by reference
        for idx in 0..arguments.len() {
            let arg = arguments.get(idx).unwrap().value.clone();
            if arg.passed_by == PassedBy::Value {
                continue;
            }

            if let Expression::Variable(name) = arg.value.value {
                if let Err(mut err) = self.stack.assign_variable(name, self.returned_arguments.get(idx).unwrap().clone()) {
                    err.message = format!("{}\nAt {:?}.", err.message, self.position);
                    return Err(Box::new(err));
                };
            }
        }

        if self.is_returning {
            self.is_returning = false;
        }

        self.last_arguments = vec![];
        self.returned_arguments = vec![];

        Ok(())
    }

    fn execute_function(&mut self, function_declaration: &FunctionDeclaration) -> Result<(), Box<dyn Issue>> {
        // if let Statement::FunctionDeclaration {
        //     identifier,
        //     parameters,
        //     return_type,
        //     block,
        // } = function_declaration

        let name = function_declaration.identifier.value.clone();
        let statements = &function_declaration.block.value.0;
        if let Err(err) = self.stack.push_stack_frame() {
            return Err(Box::new(err));
        };

        // args
        for idx in 0..self.last_arguments.len() {
            let desired_type = function_declaration.parameters.get(idx).unwrap().value.parameter_type.value;
            let param_name = function_declaration.parameters.get(idx).unwrap().value.identifier.value.clone();
            let value = self.last_arguments.get(idx).unwrap().clone();
            match (desired_type, value.clone()) {
                (Type::Bool, Value::Bool(_)) | (Type::F64, Value::F64(_)) | (Type::I64, Value::I64(_)) | (Type::Str, Value::String(_)) => {}
                (des, got) => {
                    return Err(Box::new(InterpreterIssue {
                        message: format!(
                            "Function '{}' expected '{:?}', but got '{:?}'.\nAt {:?}.",
                            name,
                            des,
                            got.to_type(),
                            self.position
                        ),
                    }))
                }
            }
            if let Err(mut err) = self.stack.declare_variable(param_name, value) {
                err.message = format!("{}\nAt {:?}.", err.message, self.position);
                return Err(Box::new(err));
            };
        }

        // execute
        for statement in statements {
            if self.is_returning {
                self.is_returning = false;
                break;
            }

            // if let Statement::FunctionDeclaration { .. } = statement.value {
            //     continue;
            // }

            self.visit_statement(&statement)?;

            if self.is_breaking {
                return Err(Box::new(InterpreterIssue {
                    message: format!("Break called outside 'for' or 'switch'.\nAt {:?}.", self.position),
                }));
            }
        }

        // self.visit_block(block)?;

        // if self.is_breaking {
        //     return Err(Box::new(InterpreterIssue {
        //         message: format!("Break called outside 'for' or 'switch'.\nAt {:?}.", self.position),
        //     }));
        // }

        // check return type
        match (self.last_result.clone(), function_declaration.return_type.value) {
            (None, Type::Void)
            | (Some(Value::I64(_)), Type::I64)
            | (Some(Value::F64(_)), Type::F64)
            | (Some(Value::String(_)), Type::Str)
            | (Some(Value::Bool(_)), Type::Bool) => {}
            (res, exp) => {
                return Err(Box::new(InterpreterIssue {
                    message: format!(
                        "Bad return type from function '{}'. Expected '{:?}', but got '{:?}'.\nAt {:?}.",
                        name, exp, res, self.position
                    ),
                }))
            }
        }

        // for reference
        let mut returned_arguments: Vec<Value> = vec![];
        for parameter in &function_declaration.parameters {
            let param_name = parameter.value.identifier.value.clone();
            returned_arguments.push(self.stack.get_variable(param_name).unwrap().clone());
        }

        self.returned_arguments = returned_arguments;

        self.stack.pop_stack_frame();

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::ast::FunctionDeclaration;

    use super::*;

    fn default_position() -> Position {
        Position {
            line: 0,
            column: 0,
            offset: 0,
        }
    }

    fn create_interpreter() -> Interpreter {
        Interpreter::new(Program {
            statements: vec![],
            functions: HashMap::new(),
        })
    }

    fn create_interpreter_with_add_function() -> Interpreter {
        let mut functions: HashMap<String, Node<FunctionDeclaration>> = HashMap::new();
        functions.insert(
            String::from("add"),
            Node {
                value: FunctionDeclaration {
                    identifier: Node {
                        value: String::from("add"),
                        position: default_position(),
                    },
                    parameters: vec![
                        Node {
                            value: Parameter {
                                passed_by: PassedBy::Value,
                                parameter_type: Node {
                                    value: Type::I64,
                                    position: default_position(),
                                },
                                identifier: Node {
                                    value: String::from("a"),
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
                                    value: String::from("b"),
                                    position: default_position(),
                                },
                            },
                            position: default_position(),
                        },
                    ],
                    return_type: Node {
                        value: Type::I64,
                        position: default_position(),
                    },
                    block: Node {
                        value: Block(vec![Node {
                            value: Statement::Return(Some(Node {
                                value: Expression::Addition(
                                    Box::new(Node {
                                        value: Expression::Variable(String::from("a")),
                                        position: default_position(),
                                    }),
                                    Box::new(Node {
                                        value: Expression::Variable(String::from("b")),
                                        position: default_position(),
                                    }),
                                ),
                                position: default_position(),
                            })),
                            position: default_position(),
                        }]),
                        position: default_position(),
                    },
                },
                position: default_position(),
            },
        );

        Interpreter::new(Program {
            statements: vec![],
            functions,
        })
    }

    #[test]
    fn interpret_casting() {
        let ast = Node {
            value: Expression::Casting {
                value: Box::new(Node {
                    value: Expression::Literal(Literal::I64(2)),
                    position: default_position(),
                }),
                to_type: Node {
                    value: Type::F64,
                    position: default_position(),
                },
            },

            position: default_position(),
        };

        let exp = Some(Value::F64(2.0));

        let mut interpreter = create_interpreter();

        let _ = interpreter.visit_expression(&ast);
        assert!(interpreter.last_result == exp);
    }

    #[test]
    fn interpret_boolean_negation() {
        let ast = Node {
            value: Expression::BooleanNegation(Box::new(Node {
                value: Expression::Literal(Literal::False),
                position: default_position(),
            })),
            position: default_position(),
        };

        let exp = Some(Value::Bool(true));

        let mut interpreter = create_interpreter();

        let _ = interpreter.visit_expression(&ast);
        assert!(interpreter.last_result == exp);
    }

    #[test]
    fn interpret_arithmetic_negation() {
        let ast = Node {
            value: Expression::ArithmeticNegation(Box::new(Node {
                value: Expression::Literal(Literal::I64(5)),
                position: default_position(),
            })),
            position: default_position(),
        };

        let exp = Some(Value::I64(-5));

        let mut interpreter = create_interpreter();

        let _ = interpreter.visit_expression(&ast);
        assert!(interpreter.last_result == exp);
    }

    #[test]
    fn interpret_addition() {
        let ast = Node {
            value: Expression::Addition(
                Box::new(Node {
                    value: Expression::Literal(Literal::I64(5)),
                    position: default_position(),
                }),
                Box::new(Node {
                    value: Expression::Literal(Literal::I64(2)),
                    position: default_position(),
                }),
            ),
            position: default_position(),
        };

        let exp = Some(Value::I64(7));

        let mut interpreter = create_interpreter();

        let _ = interpreter.visit_expression(&ast);
        assert!(interpreter.last_result == exp);
    }

    #[test]
    fn interpret_subtraction() {
        let ast = Node {
            value: Expression::Subtraction(
                Box::new(Node {
                    value: Expression::Literal(Literal::I64(5)),
                    position: default_position(),
                }),
                Box::new(Node {
                    value: Expression::Literal(Literal::I64(2)),
                    position: default_position(),
                }),
            ),
            position: default_position(),
        };

        let exp = Some(Value::I64(3));

        let mut interpreter = create_interpreter();

        let _ = interpreter.visit_expression(&ast);
        assert!(interpreter.last_result == exp);
    }

    #[test]
    fn interpret_multiplication() {
        let ast = Node {
            value: Expression::Multiplication(
                Box::new(Node {
                    value: Expression::Literal(Literal::I64(5)),
                    position: default_position(),
                }),
                Box::new(Node {
                    value: Expression::Literal(Literal::I64(2)),
                    position: default_position(),
                }),
            ),
            position: default_position(),
        };

        let exp = Some(Value::I64(10));

        let mut interpreter = create_interpreter();

        let _ = interpreter.visit_expression(&ast);
        assert!(interpreter.last_result == exp);
    }

    #[test]
    fn interpret_division() {
        let ast = Node {
            value: Expression::Division(
                Box::new(Node {
                    value: Expression::Literal(Literal::I64(5)),
                    position: default_position(),
                }),
                Box::new(Node {
                    value: Expression::Literal(Literal::I64(2)),
                    position: default_position(),
                }),
            ),
            position: default_position(),
        };

        let exp = Some(Value::I64(2));

        let mut interpreter = create_interpreter();

        let _ = interpreter.visit_expression(&ast);
        assert!(interpreter.last_result == exp);
    }

    #[test]
    fn interpret_concatenation() {
        let ast = Node {
            value: Expression::Concatenation(
                Box::new(Node {
                    value: Expression::Literal(Literal::True),
                    position: default_position(),
                }),
                Box::new(Node {
                    value: Expression::Literal(Literal::False),
                    position: default_position(),
                }),
            ),
            position: default_position(),
        };

        let exp = Some(Value::Bool(false));

        let mut interpreter = create_interpreter();

        let _ = interpreter.visit_expression(&ast);
        assert!(interpreter.last_result == exp);
    }

    #[test]
    fn interpret_alternative() {
        let ast = Node {
            value: Expression::Alternative(
                Box::new(Node {
                    value: Expression::Literal(Literal::True),
                    position: default_position(),
                }),
                Box::new(Node {
                    value: Expression::Literal(Literal::False),
                    position: default_position(),
                }),
            ),
            position: default_position(),
        };

        let exp = Some(Value::Bool(true));

        let mut interpreter = create_interpreter();

        let _ = interpreter.visit_expression(&ast);
        assert!(interpreter.last_result == exp);
    }

    #[test]
    fn interpret_greater() {
        let ast = Node {
            value: Expression::Greater(
                Box::new(Node {
                    value: Expression::Literal(Literal::I64(5)),
                    position: default_position(),
                }),
                Box::new(Node {
                    value: Expression::Literal(Literal::I64(5)),
                    position: default_position(),
                }),
            ),
            position: default_position(),
        };

        let exp = Some(Value::Bool(false));

        let mut interpreter = create_interpreter();

        let _ = interpreter.visit_expression(&ast);
        assert!(interpreter.last_result == exp);
    }

    #[test]
    fn interpret_greater_equal() {
        let ast = Node {
            value: Expression::GreaterEqual(
                Box::new(Node {
                    value: Expression::Literal(Literal::I64(5)),
                    position: default_position(),
                }),
                Box::new(Node {
                    value: Expression::Literal(Literal::I64(5)),
                    position: default_position(),
                }),
            ),
            position: default_position(),
        };

        let exp = Some(Value::Bool(true));

        let mut interpreter = create_interpreter();

        let _ = interpreter.visit_expression(&ast);
        assert!(interpreter.last_result == exp);
    }

    #[test]
    fn interpret_less() {
        let ast = Node {
            value: Expression::Less(
                Box::new(Node {
                    value: Expression::Literal(Literal::I64(5)),
                    position: default_position(),
                }),
                Box::new(Node {
                    value: Expression::Literal(Literal::I64(5)),
                    position: default_position(),
                }),
            ),
            position: default_position(),
        };

        let exp = Some(Value::Bool(false));

        let mut interpreter = create_interpreter();

        let _ = interpreter.visit_expression(&ast);
        assert!(interpreter.last_result == exp);
    }
    #[test]
    fn interpret_less_equal() {
        let ast = Node {
            value: Expression::LessEqual(
                Box::new(Node {
                    value: Expression::Literal(Literal::I64(5)),
                    position: default_position(),
                }),
                Box::new(Node {
                    value: Expression::Literal(Literal::I64(5)),
                    position: default_position(),
                }),
            ),
            position: default_position(),
        };

        let exp = Some(Value::Bool(true));

        let mut interpreter = create_interpreter();

        let _ = interpreter.visit_expression(&ast);
        assert!(interpreter.last_result == exp);
    }

    #[test]
    fn interpret_equal() {
        let ast = Node {
            value: Expression::Equal(
                Box::new(Node {
                    value: Expression::Literal(Literal::I64(5)),
                    position: default_position(),
                }),
                Box::new(Node {
                    value: Expression::Literal(Literal::I64(5)),
                    position: default_position(),
                }),
            ),
            position: default_position(),
        };

        let exp = Some(Value::Bool(true));

        let mut interpreter = create_interpreter();

        let _ = interpreter.visit_expression(&ast);
        assert!(interpreter.last_result == exp);
    }

    #[test]
    fn interpret_not_equal() {
        let ast = Node {
            value: Expression::NotEqual(
                Box::new(Node {
                    value: Expression::Literal(Literal::I64(5)),
                    position: default_position(),
                }),
                Box::new(Node {
                    value: Expression::Literal(Literal::I64(5)),
                    position: default_position(),
                }),
            ),
            position: default_position(),
        };

        let exp = Some(Value::Bool(false));

        let mut interpreter = create_interpreter();

        let _ = interpreter.visit_expression(&ast);
        assert!(interpreter.last_result == exp);
    }

    #[test]
    fn interpret_literal() {
        let ast = Node {
            value: Expression::Literal(Literal::I64(5)),
            position: default_position(),
        };

        let exp = Some(Value::I64(5));

        let mut interpreter = create_interpreter();

        let _ = interpreter.visit_expression(&ast);
        assert!(interpreter.last_result == exp);
    }

    #[test]
    fn interpret_variable() {
        let ast = Node {
            value: Expression::Variable(String::from("x")),
            position: default_position(),
        };

        let exp = Some(Value::I64(5));

        let mut interpreter = create_interpreter();
        let _ = interpreter.stack.declare_variable(String::from("x"), Value::I64(5));

        let _ = interpreter.visit_expression(&ast);
        assert!(interpreter.last_result == exp);
    }

    #[test]
    fn declare_variable() {
        // i64 x = 5;
        let ast = Node {
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
                    value: Expression::Literal(Literal::I64(5)),
                    position: default_position(),
                }),
            },
            position: default_position(),
        };

        let mut interpreter = create_interpreter();

        let _ = interpreter.visit_statement(&ast);
        assert!(interpreter.stack.get_variable(String::from("x")).unwrap().clone() == Value::I64(5));
    }

    #[test]
    fn declare_variable_with_default_value() {
        // i64 x;
        let ast = Node {
            value: Statement::Declaration {
                var_type: Node {
                    value: Type::I64,
                    position: default_position(),
                },
                identifier: Node {
                    value: String::from("x"),
                    position: default_position(),
                },
                value: None,
            },
            position: default_position(),
        };

        let mut interpreter = create_interpreter();

        let _ = interpreter.visit_statement(&ast);
        assert!(interpreter.stack.get_variable(String::from("x")).unwrap().clone() == Value::I64(0));
    }

    #[test]
    fn declare_variable_bad_type() {
        // i64 x = false;
        let ast = Node {
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
                    value: Expression::Literal(Literal::False),
                    position: default_position(),
                }),
            },
            position: default_position(),
        };

        let mut interpreter = create_interpreter();

        assert!(interpreter.visit_statement(&ast).is_err());
    }

    #[test]
    fn redeclare_variable_fails() {
        let ast = Node {
            value: Statement::Declaration {
                var_type: Node {
                    value: Type::I64,
                    position: default_position(),
                },
                identifier: Node {
                    value: String::from("x"),
                    position: default_position(),
                },
                value: None,
            },
            position: default_position(),
        };

        let mut interpreter = create_interpreter();

        let _ = interpreter.visit_statement(&ast);
        assert!(interpreter.stack.get_variable(String::from("x")).unwrap().clone() == Value::I64(0));

        assert!(interpreter.visit_statement(&ast).is_err());
    }

    #[test]
    fn declare_with_none_value_fails() {
        // i64 x = print("hello world");
        let ast = Node {
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
                    value: Expression::FunctionCall {
                        identifier: Node {
                            value: String::from("print"),
                            position: default_position(),
                        },
                        arguments: vec![Box::new(Node {
                            value: Argument {
                                value: Node {
                                    value: Expression::Literal(Literal::String(String::from("hello world"))),
                                    position: default_position(),
                                },
                                passed_by: PassedBy::Value,
                            },
                            position: default_position(),
                        })],
                    },
                    position: default_position(),
                }),
            },
            position: default_position(),
        };

        let mut interpreter = create_interpreter();
        assert!(interpreter.visit_statement(&ast).is_err());
    }

    #[test]
    fn assigns_to_variable() {
        // i64 x = 0;
        // x = 5;
        let ast = Node {
            value: Statement::Assignment {
                identifier: Node {
                    value: String::from("x"),
                    position: default_position(),
                },
                value: Node {
                    value: Expression::Literal(Literal::I64(1)),
                    position: default_position(),
                },
            },
            position: default_position(),
        };

        let mut interpreter = create_interpreter();
        let _ = interpreter.stack.declare_variable(String::from("x"), Value::I64(0));

        assert!(interpreter.visit_statement(&ast).is_ok());
        assert!(interpreter.stack.get_variable(String::from("x")).unwrap().clone() == Value::I64(1));
    }

    #[test]
    fn assigns_bad_type_fails() {
        // i64 x = 0;
        // x = false;
        let ast = Node {
            value: Statement::Assignment {
                identifier: Node {
                    value: String::from("x"),
                    position: default_position(),
                },
                value: Node {
                    value: Expression::Literal(Literal::False),
                    position: default_position(),
                },
            },
            position: default_position(),
        };

        let mut interpreter = create_interpreter();
        let _ = interpreter.stack.declare_variable(String::from("x"), Value::I64(0));

        assert!(interpreter.visit_statement(&ast).is_err());
    }

    #[test]
    fn assign_with_none_value_fails() {
        // x = print("hello world");
        let ast = Node {
            value: Statement::Assignment {
                identifier: Node {
                    value: String::from("x"),
                    position: default_position(),
                },
                value: Node {
                    value: Expression::FunctionCall {
                        identifier: Node {
                            value: String::from("print"),
                            position: default_position(),
                        },
                        arguments: vec![Box::new(Node {
                            value: Argument {
                                value: Node {
                                    value: Expression::Literal(Literal::String(String::from("hello world"))),
                                    position: default_position(),
                                },
                                passed_by: PassedBy::Value,
                            },
                            position: default_position(),
                        })],
                    },
                    position: default_position(),
                },
            },
            position: default_position(),
        };

        let mut interpreter = create_interpreter();
        let _ = interpreter.stack.declare_variable(String::from("x"), Value::I64(0));

        assert!(interpreter.visit_statement(&ast).is_err());
    }

    #[test]
    fn if_true_branch() {
        // i64 x = 0;
        // if (true) {x = 1;} else {x = 2;}
        let ast = Node {
            value: Statement::Conditional {
                condition: Node {
                    value: Expression::Literal(Literal::True),
                    position: default_position(),
                },
                if_block: Node {
                    value: Block(vec![Node {
                        value: Statement::Assignment {
                            identifier: Node {
                                value: String::from("x"),
                                position: default_position(),
                            },
                            value: Node {
                                value: Expression::Literal(Literal::I64(1)),
                                position: default_position(),
                            },
                        },
                        position: default_position(),
                    }]),
                    position: default_position(),
                },
                else_block: Some(Node {
                    value: Block(vec![Node {
                        value: Statement::Assignment {
                            identifier: Node {
                                value: String::from("x"),
                                position: default_position(),
                            },
                            value: Node {
                                value: Expression::Literal(Literal::I64(2)),
                                position: default_position(),
                            },
                        },
                        position: default_position(),
                    }]),
                    position: default_position(),
                }),
            },
            position: default_position(),
        };

        let mut interpreter = create_interpreter();
        let _ = interpreter.stack.declare_variable(String::from("x"), Value::I64(0));

        assert!(interpreter.visit_statement(&ast).is_ok());
        assert!(interpreter.stack.get_variable(String::from("x")).unwrap().clone() == Value::I64(1));
    }

    #[test]
    fn if_false_branch() {
        // i64 x = 0;
        // if (true) {x = 1;} else {x = 2;}
        let ast = Node {
            value: Statement::Conditional {
                condition: Node {
                    value: Expression::Literal(Literal::False),
                    position: default_position(),
                },
                if_block: Node {
                    value: Block(vec![Node {
                        value: Statement::Assignment {
                            identifier: Node {
                                value: String::from("x"),
                                position: default_position(),
                            },
                            value: Node {
                                value: Expression::Literal(Literal::I64(1)),
                                position: default_position(),
                            },
                        },
                        position: default_position(),
                    }]),
                    position: default_position(),
                },
                else_block: Some(Node {
                    value: Block(vec![Node {
                        value: Statement::Assignment {
                            identifier: Node {
                                value: String::from("x"),
                                position: default_position(),
                            },
                            value: Node {
                                value: Expression::Literal(Literal::I64(2)),
                                position: default_position(),
                            },
                        },
                        position: default_position(),
                    }]),
                    position: default_position(),
                }),
            },
            position: default_position(),
        };

        let mut interpreter = create_interpreter();
        let _ = interpreter.stack.declare_variable(String::from("x"), Value::I64(0));

        assert!(interpreter.visit_statement(&ast).is_ok());
        assert!(interpreter.stack.get_variable(String::from("x")).unwrap().clone() == Value::I64(2));
    }

    #[test]
    fn if_bad_condition_type_fails() {
        // i64 x = 0;
        // if (2137) {}
        let ast = Node {
            value: Statement::Conditional {
                condition: Node {
                    value: Expression::Literal(Literal::I64(2137)),
                    position: default_position(),
                },
                if_block: Node {
                    value: Block(vec![]),
                    position: default_position(),
                },
                else_block: None,
            },
            position: default_position(),
        };

        let mut interpreter = create_interpreter();
        let _ = interpreter.stack.declare_variable(String::from("x"), Value::I64(0));

        assert!(interpreter.visit_statement(&ast).is_err());
    }

    #[test]
    fn for_loop() {
        // i64 total = 0;
        // for (i64 i = 1; i <= 5; i = i + 1) {total = total + i;}
        let ast = Node {
            value: Statement::ForLoop {
                declaration: Some(Box::new(Node {
                    value: Statement::Declaration {
                        var_type: Node {
                            value: Type::I64,
                            position: default_position(),
                        },
                        identifier: Node {
                            value: String::from("i"),
                            position: default_position(),
                        },
                        value: Some(Node {
                            value: Expression::Literal(Literal::I64(1)),
                            position: default_position(),
                        }),
                    },
                    position: default_position(),
                })),
                condition: Node {
                    value: Expression::LessEqual(
                        Box::new(Node {
                            value: Expression::Variable(String::from("i")),
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
                            value: String::from("i"),
                            position: default_position(),
                        },
                        value: Node {
                            value: Expression::Addition(
                                Box::new(Node {
                                    value: Expression::Variable(String::from("i")),
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
                    value: Block(vec![Node {
                        value: Statement::Assignment {
                            identifier: Node {
                                value: String::from("total"),
                                position: default_position(),
                            },
                            value: Node {
                                value: Expression::Addition(
                                    Box::new(Node {
                                        value: Expression::Variable(String::from("total")),
                                        position: default_position(),
                                    }),
                                    Box::new(Node {
                                        value: Expression::Variable(String::from("i")),
                                        position: default_position(),
                                    }),
                                ),
                                position: default_position(),
                            },
                        },
                        position: default_position(),
                    }]),
                    position: default_position(),
                },
            },
            position: default_position(),
        };

        let mut interpreter = create_interpreter();
        let _ = interpreter.stack.declare_variable(String::from("total"), Value::I64(0));

        assert!(interpreter.visit_statement(&ast).is_ok());
        assert!(interpreter.stack.get_variable(String::from("total")).unwrap().clone() == Value::I64(15));
    }

    #[test]
    fn for_loop_second_variant() {
        // i64 total = 0;
        // i64 i = 1;
        // for (;i <= 5;) {total = total + i; i = i + 1}
        let ast = Node {
            value: Statement::ForLoop {
                declaration: None,
                condition: Node {
                    value: Expression::LessEqual(
                        Box::new(Node {
                            value: Expression::Variable(String::from("i")),
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
                    value: Block(vec![
                        Node {
                            value: Statement::Assignment {
                                identifier: Node {
                                    value: String::from("total"),
                                    position: default_position(),
                                },
                                value: Node {
                                    value: Expression::Addition(
                                        Box::new(Node {
                                            value: Expression::Variable(String::from("total")),
                                            position: default_position(),
                                        }),
                                        Box::new(Node {
                                            value: Expression::Variable(String::from("i")),
                                            position: default_position(),
                                        }),
                                    ),
                                    position: default_position(),
                                },
                            },
                            position: default_position(),
                        },
                        Node {
                            value: Statement::Assignment {
                                identifier: Node {
                                    value: String::from("i"),
                                    position: default_position(),
                                },
                                value: Node {
                                    value: Expression::Addition(
                                        Box::new(Node {
                                            value: Expression::Variable(String::from("i")),
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
                        },
                    ]),
                    position: default_position(),
                },
            },
            position: default_position(),
        };

        let mut interpreter = create_interpreter();
        let _ = interpreter.stack.declare_variable(String::from("total"), Value::I64(0));
        let _ = interpreter.stack.declare_variable(String::from("i"), Value::I64(1));

        assert!(interpreter.visit_statement(&ast).is_ok());
        assert!(interpreter.stack.get_variable(String::from("total")).unwrap().clone() == Value::I64(15));
    }

    #[test]
    fn for_loop_bad_condition_type() {
        // for (;1;) {}
        let ast = Node {
            value: Statement::ForLoop {
                declaration: None,
                condition: Node {
                    value: Expression::Literal(Literal::I64(1)),
                    position: default_position(),
                },
                assignment: None,
                block: Node {
                    value: Block(vec![]),
                    position: default_position(),
                },
            },
            position: default_position(),
        };

        let mut interpreter = create_interpreter();

        assert!(interpreter.visit_statement(&ast).is_err());
    }

    #[test]
    fn for_loop_with_break() {
        // i64 i = 0;
        // for (;true; i = i + 1) {if (i == 5) {break;}}
        let ast = Node {
            value: Statement::ForLoop {
                declaration: None,
                condition: Node {
                    value: Expression::Literal(Literal::True),
                    position: default_position(),
                },
                assignment: Some(Box::new(Node {
                    value: Statement::Assignment {
                        identifier: Node {
                            value: String::from("i"),
                            position: default_position(),
                        },
                        value: Node {
                            value: Expression::Addition(
                                Box::new(Node {
                                    value: Expression::Variable(String::from("i")),
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
                    value: Block(vec![Node {
                        value: Statement::Conditional {
                            condition: Node {
                                value: Expression::Equal(
                                    Box::new(Node {
                                        value: Expression::Variable(String::from("i")),
                                        position: default_position(),
                                    }),
                                    Box::new(Node {
                                        value: Expression::Literal(Literal::I64(5)),
                                        position: default_position(),
                                    }),
                                ),
                                position: default_position(),
                            },
                            if_block: Node {
                                value: Block(vec![Node {
                                    value: Statement::Break,
                                    position: default_position(),
                                }]),
                                position: default_position(),
                            },
                            else_block: None,
                        },
                        position: default_position(),
                    }]),
                    position: default_position(),
                },
            },
            position: default_position(),
        };

        let mut interpreter = create_interpreter();
        let _ = interpreter.stack.declare_variable(String::from("i"), Value::I64(0));

        assert!(interpreter.visit_statement(&ast).is_ok());
        assert!(interpreter.is_breaking == false);
        assert!(interpreter.stack.get_variable(String::from("i")).unwrap().clone() == Value::I64(5));
    }

    #[test]
    fn test_function_call() {
        let ast = Node {
            value: Statement::FunctionCall {
                identifier: Node {
                    value: String::from("add"),
                    position: default_position(),
                },
                arguments: vec![
                    Box::new(Node {
                        value: Argument {
                            value: Node {
                                value: Expression::Literal(Literal::I64(3)),
                                position: default_position(),
                            },
                            passed_by: PassedBy::Value,
                        },
                        position: default_position(),
                    }),
                    Box::new(Node {
                        value: Argument {
                            value: Node {
                                value: Expression::Literal(Literal::I64(4)),
                                position: default_position(),
                            },
                            passed_by: PassedBy::Value,
                        },
                        position: default_position(),
                    }),
                ],
            },
            position: default_position(),
        };

        let mut interpreter = create_interpreter_with_add_function();
        assert!(interpreter.visit_statement(&ast).is_ok());
        assert!(interpreter.last_result == Some(Value::I64(7)));
        assert!(interpreter.is_returning == false);
    }

    fn create_test_switch_case() -> Node<Statement> {
        // switch (x) {
        //      (x < 15) {
        //          result = 15;
        //      } (x < 10) {
        //          result = 10;
        //          break;
        //      } (x < 5) {
        //          result = 5;
        //      }
        // }

        fn create_assignment(val: i64) -> Node<Statement> {
            Node {
                value: Statement::Assignment {
                    identifier: Node {
                        value: String::from("result"),
                        position: default_position(),
                    },
                    value: Node {
                        value: Expression::Literal(Literal::I64(val)),
                        position: default_position(),
                    },
                },
                position: default_position(),
            }
        }

        fn create_condition(val: i64) -> Node<Expression> {
            Node {
                value: Expression::Less(
                    Box::new(Node {
                        value: Expression::Variable(String::from("x")),
                        position: default_position(),
                    }),
                    Box::new(Node {
                        value: Expression::Literal(Literal::I64(val)),
                        position: default_position(),
                    }),
                ),
                position: default_position(),
            }
        }

        Node {
            value: Statement::Switch {
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
                cases: vec![
                    Node {
                        value: SwitchCase {
                            condition: create_condition(15),
                            block: Node {
                                value: Block(vec![create_assignment(15)]),
                                position: default_position(),
                            },
                        },
                        position: default_position(),
                    },
                    Node {
                        value: SwitchCase {
                            condition: create_condition(10),
                            block: Node {
                                value: Block(vec![
                                    create_assignment(10),
                                    Node {
                                        value: Statement::Break,
                                        position: default_position(),
                                    },
                                ]),
                                position: default_position(),
                            },
                        },
                        position: default_position(),
                    },
                    Node {
                        value: SwitchCase {
                            condition: create_condition(5),
                            block: Node {
                                value: Block(vec![create_assignment(5)]),
                                position: default_position(),
                            },
                        },
                        position: default_position(),
                    },
                ],
            },
            position: default_position(),
        }
    }

    #[test]
    fn switch_enters() {
        let mut interpreter = create_interpreter();
        let _ = interpreter.stack.declare_variable(String::from("x"), Value::I64(12));
        let _ = interpreter
            .stack
            .declare_variable(String::from("result"), Value::default_value(Type::I64).unwrap());

        let _ = interpreter.visit_statement(&create_test_switch_case());

        assert!(interpreter.stack.get_variable(String::from("result")).unwrap().clone() == Value::I64(15));
        assert!(interpreter.is_breaking == false);
    }

    #[test]
    fn switch_breaks() {
        let mut interpreter = create_interpreter();
        let _ = interpreter.stack.declare_variable(String::from("x"), Value::I64(3));
        let _ = interpreter
            .stack
            .declare_variable(String::from("result"), Value::default_value(Type::I64).unwrap());

        let _ = interpreter.visit_statement(&create_test_switch_case());

        assert!(interpreter.stack.get_variable(String::from("result")).unwrap().clone() == Value::I64(10));
        assert!(interpreter.is_breaking == false);
    }

    #[test]
    fn switch_no_entry() {
        let mut interpreter = create_interpreter();
        let _ = interpreter.stack.declare_variable(String::from("x"), Value::I64(2137));
        let _ = interpreter
            .stack
            .declare_variable(String::from("result"), Value::default_value(Type::I64).unwrap());

        let _ = interpreter.visit_statement(&create_test_switch_case());

        assert!(interpreter.stack.get_variable(String::from("result")).unwrap().clone() == Value::I64(0));
        assert!(interpreter.is_breaking == false);
    }
}
