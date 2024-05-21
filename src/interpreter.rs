use crate::{
    ast::{
        Argument, Block, Expression, Identifier, Literal, Node, Parameter, Program, Statement,
        SwitchCase, SwitchExpression, Type,
    },
    errors::Issue,
    scope_manager::ScopeManager,
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
    scope_manager: ScopeManager,
    last_result: Option<Value>,
    is_breaking: bool
}

impl Interpreter {
    pub fn new(program: Program) -> Self {
        Interpreter {
            program,
            scope_manager: ScopeManager::new(),
            last_result: None,
            is_breaking: false
        }
    }

    pub fn interpret(&mut self) -> Result<(), Box<dyn Issue>> {
        self.visit_program(&self.program.clone())
    }

    fn read_last_result(&mut self) -> Value {
        let read_value = self.last_result.clone().unwrap();
        self.last_result = None;
        read_value
    }

    fn evaluate_binary_op<F>(
        &mut self,
        lhs: Box<Node<Expression>>,
        rhs: Box<Node<Expression>>,
        op: F,
    ) -> Result<(), Box<dyn Issue>>
    where
        F: Fn(Value, Value) -> Result<Value, ComputationIssue>,
    {
        self.visit_expression(&lhs)?;
        let left_value = self.read_last_result();
        self.visit_expression(&rhs)?;
        let right_value = self.read_last_result();

        match op(left_value, right_value) {
            Ok(val) => {
                self.last_result = Some(val);
                Ok(())
            }
            Err(err) => Err(Box::new(err)),
        }
    }

    fn evaluate_unary_op<F>(
        &mut self,
        value: Box<Node<Expression>>,
        op: F,
    ) -> Result<(), Box<dyn Issue>>
    where
        F: Fn(Value) -> Result<Value, ComputationIssue>,
    {
        self.visit_expression(&value)?;
        let computed_value = self.read_last_result();
        match op(computed_value) {
            Ok(val) => {
                self.last_result = Some(val);
                Ok(())
            }
            Err(err) => Err(Box::new(err)),
        }
    }
}

impl Visitor for Interpreter {
    fn visit_program(&mut self, program: &Program) -> Result<(), Box<dyn Issue>> {
        for statement in program.statements.clone() {
            self.visit_statement(&statement)?;
            if self.is_breaking && self.scope_manager.len() == 1 {
                return Err(Box::new(InterpreterIssue {message: format!("Break called outside for or switch.")}));
            }
        }
        Ok(())
    }

    fn visit_expression(&mut self, expression: &Node<Expression>) -> Result<(), Box<dyn Issue>> {
        match expression.value.clone() {
            Expression::Casting { value, to_type } => {
                self.visit_expression(&value)?;
                let computed_value = self.read_last_result();
                match ALU::cast_to_type(computed_value, to_type.value) {
                    Ok(val) => {
                        self.last_result = Some(val);
                        return Ok(());
                    }
                    Err(err) => return Err(Box::new(err)),
                }
            }
            Expression::BooleanNegation(value) => {
                self.evaluate_unary_op(value, ALU::boolean_negate)?
            }
            Expression::ArithmeticNegation(value) => {
                self.evaluate_unary_op(value, ALU::arithmetic_negate)?
            }
            Expression::Addition(lhs, rhs) => self.evaluate_binary_op(lhs, rhs, ALU::add)?,
            Expression::Subtraction(lhs, rhs) => {
                self.evaluate_binary_op(lhs, rhs, ALU::subtract)?
            }
            Expression::Multiplication(lhs, rhs) => {
                self.evaluate_binary_op(lhs, rhs, ALU::multiplication)?
            }
            Expression::Division(lhs, rhs) => self.evaluate_binary_op(lhs, rhs, ALU::division)?,
            Expression::Alternative(lhs, rhs) => {
                self.evaluate_binary_op(lhs, rhs, ALU::alternative)?
            }
            Expression::Concatenation(lhs, rhs) => {
                self.evaluate_binary_op(lhs, rhs, ALU::concatenation)?
            }
            Expression::Greater(lhs, rhs) => self.evaluate_binary_op(lhs, rhs, ALU::greater)?,
            Expression::GreaterEqual(lhs, rhs) => {
                self.evaluate_binary_op(lhs, rhs, ALU::greater_or_equal)?
            }
            Expression::Less(lhs, rhs) => self.evaluate_binary_op(lhs, rhs, ALU::less)?,
            Expression::LessEqual(lhs, rhs) => {
                self.evaluate_binary_op(lhs, rhs, ALU::less_or_equal)?
            }
            Expression::Equal(lhs, rhs) => self.evaluate_binary_op(lhs, rhs, ALU::equal)?,
            Expression::NotEqual(lhs, rhs) => self.evaluate_binary_op(lhs, rhs, ALU::not_equal)?,
            Expression::Literal(literal) => self.visit_literal(literal)?,
            Expression::Variable(variable) => self.visit_variable(variable)?,
            Expression::FunctionCall {
                identifier,
                arguments,
            } => {
                self.visit_identifier(&identifier)?;
                for arg in arguments {
                    self.visit_argument(&arg)?;
                }
            }
        }
        Ok(())
    }

    fn visit_statement(&mut self, statement: &Node<Statement>) -> Result<(), Box<dyn Issue>> {
        match statement.value.clone() {
            Statement::FunctionDeclaration {
                // wykonanie funckji
                identifier,
                parameters,
                return_type,
                block,
            } => {
                self.visit_identifier(&identifier)?;
                for param in parameters {
                    self.visit_parameter(&param)?
                }
                self.visit_type(&return_type)?;
                self.visit_block(&block)?;
            }
            Statement::FunctionCall {
                identifier,
                arguments,
            } => {
                self.visit_identifier(&identifier)?;
                for arg in arguments {
                    self.visit_argument(&arg)?;
                }
                // przygotowqanie wywolania funckji
                // sprawdzenie czy funckja uzytkownika czy wbudowana
            }
            Statement::Declaration {
                var_type,
                identifier,
                value,
            } => {
                self.visit_type(&var_type)?;
                self.visit_identifier(&identifier)?;

                let computed_value = match value {
                    Some(val) => {
                        self.visit_expression(&val)?;
                        let result = self.read_last_result();
                        result
                    }
                    None => match Value::default_value(var_type.value) {
                        Ok(val) => val,
                        Err(err) => return Err(Box::new(err)),
                    },
                };

                match (var_type.value, computed_value.clone()) {
                    (Type::I64, Value::I64(_))
                    | (Type::F64, Value::F64(_))
                    | (Type::Str, Value::String(_))
                    | (Type::Bool, Value::Bool(_)) => {}
                    (declared_type, computed_type) => {
                        return Err(Box::new(InterpreterIssue {
                            message: format!(
                                "Cannot assign variable of type {:?} to type {:?}",
                                computed_type, declared_type
                            ),
                        }))
                    }
                }

                match self
                    .scope_manager
                    .declare_variable(identifier.value.0, computed_value)
                {
                    Ok(_) => {}
                    Err(err) => return Err(Box::new(err)),
                }
                println!("{:?}", self.scope_manager.clone());
            }
            Statement::Assignment { identifier, value } => {
                self.visit_identifier(&identifier)?;
                self.visit_expression(&value)?;
                let value = self.read_last_result();
                match self
                    .scope_manager
                    .assign_variable(identifier.value.0, value)
                {
                    Ok(_) => {}
                    Err(err) => return Err(Box::new(err)),
                }
                println!("{:?}", self.scope_manager.clone());
            }
            Statement::Conditional {
                condition,
                if_block,
                else_block,
            } => {
                self.visit_expression(&condition)?;
                let computed_condition = self.read_last_result();
                let boolean_value = match computed_condition {
                    Value::Bool(bool) => bool,
                    a => return Err(Box::new(InterpreterIssue {message: format!("Bad value for condition in if statement. Given {:?}, expected a boolean.", a)})),
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
                self.scope_manager.push_scope();
                if let Some(decl) = declaration {
                    self.visit_statement(&decl)?;
                }

                self.visit_expression(&condition)?;
                let mut computed_condition = self.read_last_result();
                let mut boolean_value = match computed_condition {
                    Value::Bool(bool) => bool,
                    a => return Err(Box::new(InterpreterIssue {message: format!("Bad value for condition in for statement. Given {:?}, expected a boolean.", a)})),
                };

                while boolean_value {
                    self.visit_block(&block)?;

                    if self.is_breaking {
                        self.is_breaking = false;
                        break;
                    }

                    if let Some(assign) = assignment.clone() {
                        self.visit_statement(&assign)?;
                    }

                    self.visit_expression(&condition)?;
                    computed_condition = self.read_last_result();
                    boolean_value = match computed_condition {
                        Value::Bool(bool) => bool,
                        a => return Err(Box::new(InterpreterIssue {message: format!("Bad value for condition in for statement. Given {:?}, expected a boolean.", a)})),
                    };
                }
                self.scope_manager.pop_scope();
            }
            Statement::Switch { expressions, cases } => {
                self.scope_manager.push_scope();
                for expr in expressions {
                    self.visit_switch_expression(&expr)?;
                }
                for case in cases {
                    self.visit_switch_case(&case)?;
                    if self.is_breaking {
                        self.is_breaking = false;
                        break;
                    }
                }
                self.scope_manager.pop_scope();
            }
            Statement::Return(value) => {
                if let Some(val) = value {
                    self.visit_expression(&val)?;
                }
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
        self.scope_manager.push_scope();
        println!("{:?}", self.scope_manager.clone());
        for statement in &block.value.0 {
            println!("{}", self.scope_manager.len());
            if self.is_breaking && self.scope_manager.len() == 1 {
                return Err(Box::new(InterpreterIssue {message: format!("Break called outside for or switch.")}));
            }

            if self.is_breaking {
                break;
            }
            self.visit_statement(statement)?;
        }
        self.scope_manager.pop_scope();
        println!("{:?}", self.scope_manager.clone());
        Ok(())
    }

    fn visit_parameter(&mut self, parameter: &Node<Parameter>) -> Result<(), Box<dyn Issue>> {
        self.visit_type(&parameter.value.parameter_type)?;
        self.visit_identifier(&parameter.value.identifier)?;
        Ok(())
    }

    fn visit_switch_case(&mut self, switch_case: &Node<SwitchCase>) -> Result<(), Box<dyn Issue>> {
        self.visit_expression(&switch_case.value.condition)?;
        let computed_value = self.read_last_result();
        let boolean_value = match computed_value {
            Value::Bool(bool) => bool,
            a => return Err(Box::new(InterpreterIssue {message: format!("Condition in switch case has to evaluate to boolean - got {:?}.", a)}))
        };
        if boolean_value {
            self.visit_block(&switch_case.value.block)?;
        }
        Ok(())
    }

    fn visit_switch_expression(
        &mut self,
        switch_expression: &Node<SwitchExpression>,
    ) -> Result<(), Box<dyn Issue>> {
        self.visit_expression(&switch_expression.value.expression)?;
        Ok(())
    }

    fn visit_identifier(&mut self, _identifier: &Node<Identifier>) -> Result<(), Box<dyn Issue>> {
        Ok(())
        // println!("{:?}", _identifier);
    }

    fn visit_type(&mut self, _node_type: &Node<Type>) -> Result<(), Box<dyn Issue>> {
        Ok(())
        // println!("{:?}", _node_type);
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

    fn visit_variable(&mut self, variable: Identifier) -> Result<(), Box<dyn Issue>> {
        // read value of variable
        let value = match self.scope_manager.get_variable(variable.0) {
            Ok(val) => val,
            Err(err) => return Err(Box::new(err)),
        };
        self.last_result = Some(value.clone());
        Ok(())
    }
}
