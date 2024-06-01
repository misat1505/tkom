use std::{cell::RefCell, rc::Rc};

use crate::{
    ast::{
        Argument, Block, Expression, FunctionDeclaration, Literal, Node, Parameter, PassedBy, Program, Statement, SwitchCase, SwitchExpression, Type,
    },
    errors::{ComputationIssue, InterpreterIssue, Issue},
    lazy_stream_reader::Position,
    stack::Stack,
    std_functions::StdFunction,
    value::Value,
    visitor::Visitor,
    ALU::ALU,
};

pub struct Interpreter<'a> {
    program: &'a Program,
    stack: Stack<'a>,
    last_result: Option<Value>,
    is_breaking: bool,
    is_returning: bool,
    position: Position,
    last_arguments: Vec<Rc<RefCell<Value>>>,
}

impl<'a> Interpreter<'a> {
    pub fn new(program: &'a Program) -> Self {
        Interpreter {
            program,
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
        }
    }

    pub fn interpret(&mut self) -> Result<(), Box<dyn Issue>> {
        self.visit_program(self.program)
    }

    fn read_last_result(&mut self) -> Result<Value, Box<dyn Issue>> {
        self.last_result.take().ok_or_else(|| {
            return Box::new(InterpreterIssue {
                message: format!("No value produced where it is needed.\nAt {:?}.", self.position),
            }) as Box<dyn Issue>;
        })
    }

    fn evaluate_binary_op<F>(&mut self, lhs: &'a Box<Node<Expression>>, rhs: &'a Box<Node<Expression>>, op: F) -> Result<(), Box<dyn Issue>>
    where
        F: Fn(Value, Value) -> Result<Value, ComputationIssue>,
    {
        self.visit_expression(lhs)?;
        let left_value = self.read_last_result()?;
        self.visit_expression(rhs)?;
        let right_value = self.read_last_result()?;

        let value = op(left_value, right_value).map_err(|err| self.append_position(Box::new(err)))?;
        self.last_result = Some(value);
        Ok(())
    }

    fn evaluate_unary_op<F>(&mut self, value: &'a Box<Node<Expression>>, op: F) -> Result<(), Box<dyn Issue>>
    where
        F: Fn(Value) -> Result<Value, ComputationIssue>,
    {
        self.visit_expression(value)?;
        let computed_value = self.read_last_result()?;
        let value = op(computed_value).map_err(|err| self.append_position(Box::new(err)))?;
        self.last_result = Some(value);
        Ok(())
    }
}

impl<'a> Visitor<'a> for Interpreter<'a> {
    fn visit_program(&mut self, program: &'a Program) -> Result<(), Box<dyn Issue>> {
        for statement in &program.statements {
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

    fn visit_expression(&mut self, expression: &'a Node<Expression>) -> Result<(), Box<dyn Issue>> {
        self.position = expression.position;
        match &expression.value {
            Expression::Casting { value, to_type } => {
                self.visit_expression(&value)?;
                let computed_value = self.read_last_result()?;
                let value = ALU::cast_to_type(computed_value, to_type.value).map_err(|err| self.append_position(Box::new(err)))?;
                self.last_result = Some(value);
                return Ok(());
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
            Expression::Literal(literal) => self.visit_literal(literal)?,
            Expression::Variable(variable) => self.visit_variable(variable)?,
            Expression::FunctionCall { identifier, arguments } => self.call_function(identifier, arguments)?,
        }
        Ok(())
    }

    fn visit_statement(&mut self, statement: &'a Node<Statement>) -> Result<(), Box<dyn Issue>> {
        self.position = statement.position;
        match &statement.value {
            Statement::FunctionCall { identifier, arguments } => self.call_function(identifier, arguments)?,
            Statement::Declaration { var_type, identifier, value } => {
                self.visit_type(&var_type)?;

                let computed_value = match value {
                    Some(val) => {
                        self.visit_expression(&val)?;
                        self.read_last_result().map_err(|_| {
                            Box::new(InterpreterIssue {
                                message: format!("Cannot declare variable '{}' with no value.\nAt {:?}.", identifier.value, self.position),
                            }) as Box<dyn Issue>
                        })?
                    }
                    None => Value::default_value(var_type.value).map_err(|err| Box::new(err) as Box<dyn Issue>)?,
                };

                match (var_type.value, &computed_value) {
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

                self.stack
                    .declare_variable(identifier.value.as_str(), Rc::new(RefCell::new(computed_value)))
                    .map_err(|err| self.append_position(Box::new(err)))?;
            }
            Statement::Assignment { identifier, value } => {
                self.visit_expression(&value)?;
                let value = self.read_last_result().map_err(|_| {
                    Box::new(InterpreterIssue {
                        message: format!("Cannot assign no value to variable '{}'.\nAt {:?}.", identifier.value, self.position),
                    }) as Box<dyn Issue>
                })?;

                self.stack
                    .assign_variable(identifier.value.as_str(), Rc::new(RefCell::new(value)))
                    .map_err(|err| self.append_position(Box::new(err)))?;
            }
            Statement::Conditional {
                condition,
                if_block,
                else_block,
            } => {
                self.visit_expression(&condition)?;
                let computed_condition = self.read_last_result()?;
                let boolean_value = computed_condition
                    .try_into_bool()
                    .map_err(|_| self.condition_error(computed_condition, "if statement"))?;

                if boolean_value {
                    self.visit_block(&if_block)?;
                } else if let Some(else_blk) = else_block {
                    self.visit_block(&else_blk)?;
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
                let mut boolean_value = computed_condition
                    .try_into_bool()
                    .map_err(|_| self.condition_error(computed_condition, "for statement"))?;

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
                    boolean_value = computed_condition
                        .try_into_bool()
                        .map_err(|_| self.condition_error(computed_condition, "for statement"))?;
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
                let mut returned_value = None;
                if let Some(val) = value {
                    self.visit_expression(&val)?;
                    returned_value = Some(self.read_last_result()?);
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

    fn visit_argument(&mut self, argument: &'a Node<Argument>) -> Result<(), Box<dyn Issue>> {
        self.visit_expression(&argument.value.value)?;
        Ok(())
    }

    fn visit_block(&mut self, block: &'a Node<Block>) -> Result<(), Box<dyn Issue>> {
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

    fn visit_parameter(&mut self, parameter: &'a Node<Parameter>) -> Result<(), Box<dyn Issue>> {
        self.visit_type(&parameter.value.parameter_type)?;
        Ok(())
    }

    fn visit_switch_case(&mut self, switch_case: &'a Node<SwitchCase>) -> Result<(), Box<dyn Issue>> {
        self.visit_expression(&switch_case.value.condition)?;
        let computed_value = self.read_last_result()?;
        let boolean_value = computed_value
            .try_into_bool()
            .map_err(|_| self.condition_error(computed_value, "switch case"))?;

        if boolean_value {
            self.visit_block(&switch_case.value.block)?;
        }
        Ok(())
    }

    fn visit_switch_expression(&mut self, switch_expression: &'a Node<SwitchExpression>) -> Result<(), Box<dyn Issue>> {
        if let Some(alias) = &switch_expression.value.alias {
            self.visit_expression(&switch_expression.value.expression)?;
            let computed_value = self.read_last_result()?;
            self.stack
                .declare_variable(alias.value.as_str(), Rc::new(RefCell::new(computed_value)))
                .map_err(|err| self.append_position(Box::new(err)))?;
        }
        Ok(())
    }

    fn visit_type(&mut self, _node_type: &Node<Type>) -> Result<(), Box<dyn Issue>> {
        Ok(())
    }

    fn visit_literal(&mut self, literal: &Literal) -> Result<(), Box<dyn Issue>> {
        // change literal to value
        let value = match literal {
            Literal::F64(f64) => Value::F64(*f64),
            Literal::I64(i64) => Value::I64(*i64),
            Literal::String(str) => Value::String(str.to_string()),
            Literal::False => Value::Bool(false),
            Literal::True => Value::Bool(true),
        };

        self.last_result = Some(value);
        Ok(())
    }

    fn visit_variable(&mut self, variable: &'a String) -> Result<(), Box<dyn Issue>> {
        // read value of variable
        let value = self
            .stack
            .get_variable(variable.as_str())
            .map_err(|err| Box::new(err) as Box<dyn Issue>)?;
        self.last_result = Some(value.borrow().to_owned());
        Ok(())
    }
}

impl<'a> Interpreter<'a> {
    fn append_position(&self, mut error: Box<dyn Issue>) -> Box<dyn Issue> {
        let positon = self.position;
        let prev_message = error.message();
        error.set_message(format!("{}\nAt {:?}.", prev_message, positon));
        error
    }

    #[allow(dead_code)]
    pub fn stack(&mut self) -> Stack {
        // only for accept tests
        self.stack.clone()
    }

    fn condition_error(&self, value: Value, place: &'a str) -> Box<dyn Issue> {
        Box::new(InterpreterIssue {
            message: format!(
                "Condition in '{}' has to evaluate to type '{:?}' - got '{:?}'.\nAt {:?}.",
                place,
                Type::Bool,
                value.to_type(),
                self.position
            ),
        })
    }

    fn execute_std_function(std_function: &StdFunction, arguments: &Vec<Rc<RefCell<Value>>>) -> Result<Option<Value>, Box<dyn Issue>> {
        (std_function.execute)(arguments).map_err(|err| Box::new(err) as Box<dyn Issue>)
    }

    fn call_function(&mut self, identifier: &Node<String>, arguments: &'a Vec<Box<Node<Argument>>>) -> Result<(), Box<dyn Issue>> {
        let name = identifier.value.as_str();

        let mut args: Vec<Rc<RefCell<Value>>> = vec![];
        for arg in arguments {
            self.visit_expression(&arg.value.value)?;
            let value = self.read_last_result()?;
            match arg.value.passed_by {
                PassedBy::Value => args.push(Rc::new(RefCell::new(value))),
                PassedBy::Reference => {
                    if let Expression::Variable(var_name) = &arg.value.value.value {
                        let var_ref = self
                            .stack
                            .get_variable(var_name.as_str())
                            .map_err(|err| Box::new(err) as Box<dyn Issue>)?;
                        args.push(Rc::clone(var_ref));
                    }
                }
            };
        }

        self.last_arguments = args;

        if let Some(std_function) = self.program.std_functions.get(name) {
            if let Some(return_value) = Self::execute_std_function(std_function, &self.last_arguments).map_err(|err| self.append_position(err))? {
                self.last_result = Some(return_value);
            }
        }

        if let Some(function_declaration) = self.program.functions.get(name) {
            self.execute_function(&(*function_declaration).value)?;
        }

        if self.is_returning {
            self.is_returning = false;
        }

        self.last_arguments = vec![];

        Ok(())
    }

    fn execute_function(&mut self, function_declaration: &'a FunctionDeclaration) -> Result<(), Box<dyn Issue>> {
        let name = function_declaration.identifier.value.as_str();
        let statements = &function_declaration.block.value.0;
        self.stack.push_stack_frame().map_err(|err| Box::new(err) as Box<dyn Issue>)?;

        // args
        for idx in 0..self.last_arguments.len() {
            let desired_type = function_declaration.parameters.get(idx).unwrap().value.parameter_type.value;
            let param_name = &function_declaration.parameters.get(idx).unwrap().value.identifier.value;
            let value = self.last_arguments.get(idx).unwrap();
            match (desired_type, &*value.borrow()) {
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
            self.stack
                .declare_variable(param_name.as_str(), Rc::clone(value))
                .map_err(|err| self.append_position(Box::new(err)))?;
        }

        // execute
        for statement in statements {
            if self.is_returning {
                self.is_returning = false;
                break;
            }

            self.visit_statement(&statement)?;

            if self.is_breaking {
                return Err(Box::new(InterpreterIssue {
                    message: format!("Break called outside 'for' or 'switch'.\nAt {:?}.", self.position),
                }));
            }
        }

        // check return type
        match (&self.last_result, function_declaration.return_type.value) {
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

    fn setup_program() -> Program {
        Program {
            statements: vec![],
            functions: HashMap::new(),
            std_functions: HashMap::new(),
        }
    }

    fn create_interpreter<'a>(program: &'a Program) -> Interpreter<'a> {
        Interpreter::new(program)
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

        let program = setup_program();
        let mut interpreter = create_interpreter(&program);

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

        let program = setup_program();
        let mut interpreter = create_interpreter(&program);

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

        let program = setup_program();
        let mut interpreter = create_interpreter(&program);

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

        let program = setup_program();
        let mut interpreter = create_interpreter(&program);

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

        let program = setup_program();
        let mut interpreter = create_interpreter(&program);

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

        let program = setup_program();
        let mut interpreter = create_interpreter(&program);

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

        let program = setup_program();
        let mut interpreter = create_interpreter(&program);

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

        let program = setup_program();
        let mut interpreter = create_interpreter(&program);

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

        let program = setup_program();
        let mut interpreter = create_interpreter(&program);

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

        let program = setup_program();
        let mut interpreter = create_interpreter(&program);

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

        let program = setup_program();
        let mut interpreter = create_interpreter(&program);

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

        let program = setup_program();
        let mut interpreter = create_interpreter(&program);

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

        let program = setup_program();
        let mut interpreter = create_interpreter(&program);

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

        let program = setup_program();
        let mut interpreter = create_interpreter(&program);

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

        let program = setup_program();
        let mut interpreter = create_interpreter(&program);

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

        let program = setup_program();
        let mut interpreter = create_interpreter(&program);

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

        let program = setup_program();
        let mut interpreter = create_interpreter(&program);
        let _ = interpreter.stack.declare_variable("x", Rc::new(RefCell::new(Value::I64(5))));

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

        let program = setup_program();
        let mut interpreter = create_interpreter(&program);

        let _ = interpreter.visit_statement(&ast);
        assert!(interpreter.stack.get_variable("x").unwrap().clone() == Rc::new(RefCell::new(Value::I64(5))));
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

        let program = setup_program();
        let mut interpreter = create_interpreter(&program);

        let _ = interpreter.visit_statement(&ast);
        assert!(interpreter.stack.get_variable("x").unwrap().clone() == Rc::new(RefCell::new(Value::I64(0))));
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

        let program = setup_program();
        let mut interpreter = create_interpreter(&program);

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

        let program = setup_program();
        let mut interpreter = create_interpreter(&program);

        let _ = interpreter.visit_statement(&ast);
        assert!(interpreter.stack.get_variable("x").unwrap().clone() == Rc::new(RefCell::new(Value::I64(0))));

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

        let program = setup_program();
        let mut interpreter = create_interpreter(&program);
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

        let program = setup_program();
        let mut interpreter = create_interpreter(&program);
        let _ = interpreter.stack.declare_variable("x", Rc::new(RefCell::new(Value::I64(0))));

        assert!(interpreter.visit_statement(&ast).is_ok());
        assert!(interpreter.stack.get_variable("x").unwrap().clone() == Rc::new(RefCell::new(Value::I64(1))));
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

        let program = setup_program();
        let mut interpreter = create_interpreter(&program);
        let _ = interpreter.stack.declare_variable("x", Rc::new(RefCell::new(Value::I64(0))));

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

        let program = setup_program();
        let mut interpreter = create_interpreter(&program);
        let _ = interpreter.stack.declare_variable("x", Rc::new(RefCell::new(Value::I64(0))));

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

        let program = setup_program();
        let mut interpreter = create_interpreter(&program);
        let _ = interpreter.stack.declare_variable("x", Rc::new(RefCell::new(Value::I64(0))));

        assert!(interpreter.visit_statement(&ast).is_ok());
        assert!(interpreter.stack.get_variable("x").unwrap().clone() == Rc::new(RefCell::new(Value::I64(1))));
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

        let program = setup_program();
        let mut interpreter = create_interpreter(&program);
        let _ = interpreter.stack.declare_variable("x", Rc::new(RefCell::new(Value::I64(0))));

        assert!(interpreter.visit_statement(&ast).is_ok());
        assert!(interpreter.stack.get_variable("x").unwrap().clone() == Rc::new(RefCell::new(Value::I64(2))));
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

        let program = setup_program();
        let mut interpreter = create_interpreter(&program);
        let _ = interpreter.stack.declare_variable("x", Rc::new(RefCell::new(Value::I64(0))));

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

        let program = setup_program();
        let mut interpreter = create_interpreter(&program);
        let _ = interpreter.stack.declare_variable("total", Rc::new(RefCell::new(Value::I64(0))));

        assert!(interpreter.visit_statement(&ast).is_ok());
        assert!(interpreter.stack.get_variable("total").unwrap().clone() == Rc::new(RefCell::new(Value::I64(15))));
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

        let program = setup_program();
        let mut interpreter = create_interpreter(&program);
        let _ = interpreter.stack.declare_variable("total", Rc::new(RefCell::new(Value::I64(0))));
        let _ = interpreter.stack.declare_variable("i", Rc::new(RefCell::new(Value::I64(1))));

        assert!(interpreter.visit_statement(&ast).is_ok());
        assert!(interpreter.stack.get_variable("total").unwrap().clone() == Rc::new(RefCell::new(Value::I64(15))));
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

        let program = setup_program();
        let mut interpreter = create_interpreter(&program);

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

        let program = setup_program();
        let mut interpreter = create_interpreter(&program);
        let _ = interpreter.stack.declare_variable("i", Rc::new(RefCell::new(Value::I64(0))));

        assert!(interpreter.visit_statement(&ast).is_ok());
        assert!(interpreter.is_breaking == false);
        assert!(interpreter.stack.get_variable("i").unwrap().clone() == Rc::new(RefCell::new(Value::I64(5))));
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

        let mut functions: HashMap<String, Rc<Node<FunctionDeclaration>>> = HashMap::new();

        functions.insert(
            String::from("add"),
            Rc::new(Node {
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
            }),
        );

        let program = Program {
            statements: vec![],
            std_functions: HashMap::new(),
            functions,
        };
        let mut interpreter = Interpreter::new(&program);
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
        let program = setup_program();
        let mut interpreter = create_interpreter(&program);
        let _ = interpreter.stack.declare_variable("x", Rc::new(RefCell::new(Value::I64(12))));
        let _ = interpreter
            .stack
            .declare_variable("result", Rc::new(RefCell::new(Value::default_value(Type::I64).unwrap())));

        let switch_case = &create_test_switch_case();
        let _ = interpreter.visit_statement(switch_case);

        assert!(interpreter.stack.get_variable("result").unwrap().clone() == Rc::new(RefCell::new(Value::I64(15))));
        assert!(interpreter.is_breaking == false);
    }

    #[test]
    fn switch_breaks() {
        let program = setup_program();
        let mut interpreter = create_interpreter(&program);
        let _ = interpreter.stack.declare_variable("x", Rc::new(RefCell::new(Value::I64(3))));
        let _ = interpreter
            .stack
            .declare_variable("result", Rc::new(RefCell::new(Value::default_value(Type::I64).unwrap())));

        let switch_case = &create_test_switch_case();
        let _ = interpreter.visit_statement(switch_case);

        assert!(interpreter.stack.get_variable("result").unwrap().clone() == Rc::new(RefCell::new(Value::I64(10))));
        assert!(interpreter.is_breaking == false);
    }

    #[test]
    fn switch_no_entry() {
        let program = setup_program();
        let mut interpreter = create_interpreter(&program);
        let _ = interpreter.stack.declare_variable("x", Rc::new(RefCell::new(Value::I64(2137))));
        let _ = interpreter
            .stack
            .declare_variable("result", Rc::new(RefCell::new(Value::default_value(Type::I64).unwrap())));

        let switch_case = &create_test_switch_case();
        let _ = interpreter.visit_statement(switch_case);

        assert!(interpreter.stack.get_variable("result").unwrap().clone() == Rc::new(RefCell::new(Value::I64(0))));
        assert!(interpreter.is_breaking == false);
    }
}
