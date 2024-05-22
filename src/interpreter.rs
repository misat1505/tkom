use crate::{
    ast::{
        Argument, Block, Expression, Identifier, Literal, Node, Parameter, Program, Statement,
        SwitchCase, SwitchExpression, Type,
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
    stack: Stack,
    last_result: Option<Value>,
    is_breaking: bool,
    is_returning: bool,
    position: Position,
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
                message: format!(
                    "No value produced where it is needed.\nAt {:?}.",
                    self.position
                ),
            })),
        }
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
        let left_value = self.read_last_result()?;
        self.visit_expression(&rhs)?;
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

    fn evaluate_unary_op<F>(
        &mut self,
        value: Box<Node<Expression>>,
        op: F,
    ) -> Result<(), Box<dyn Issue>>
    where
        F: Fn(Value) -> Result<Value, ComputationIssue>,
    {
        self.visit_expression(&value)?;
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
        for statement in program.statements.clone() {
            if let Statement::FunctionDeclaration { .. } = statement.value {
                continue;
            }

            self.visit_statement(&statement)?;
            if self.is_breaking && self.stack.is_last_scope() {
                return Err(Box::new(InterpreterIssue {
                    message: format!(
                        "Break called outside for or switch.\nAt {:?}.",
                        self.position
                    ),
                }));
            }
        }
        Ok(())
    }

    fn visit_expression(&mut self, expression: &Node<Expression>) -> Result<(), Box<dyn Issue>> {
        self.position = expression.position;
        match expression.value.clone() {
            Expression::Casting { value, to_type } => {
                self.visit_expression(&value)?;
                let computed_value = self.read_last_result()?;
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
                let name = identifier.value.0;

                let mut args: Vec<Value> = vec![];
                for arg in arguments {
                    self.visit_expression(&arg.value.value)?;
                    let value = self.read_last_result()?;
                    args.push(value);
                }

                if let Some(std_function) = self.functions_manager.std_functions.get(&name) {
                    Self::execute_std_function(std_function, args.clone())?;
                }

                if let Some(function_declaration) =
                    self.functions_manager.functions.clone().get(&name)
                {
                    self.execute_function(function_declaration, args)?;
                }

                if self.is_returning {
                    self.is_returning = false;
                }
            }
        }
        Ok(())
    }

    fn visit_statement(&mut self, statement: &Node<Statement>) -> Result<(), Box<dyn Issue>> {
        self.position = statement.position;
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
                let name = identifier.value.0;

                let mut args: Vec<Value> = vec![];
                for arg in arguments {
                    self.visit_expression(&arg.value.value)?;
                    let value = self.read_last_result()?;
                    args.push(value);
                }

                if let Some(std_function) = self.functions_manager.std_functions.get(&name) {
                    Self::execute_std_function(std_function, args.clone())?;
                }

                if let Some(function_declaration) =
                    self.functions_manager.functions.clone().get(&name)
                {
                    self.execute_function(function_declaration, args)?;
                }

                if self.is_returning {
                    self.is_returning = false;
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
                        let result = self.read_last_result()?;
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
                                "Cannot assign variable of type {:?} to type {:?}.\nAt {:?}.",
                                computed_type, declared_type, self.position
                            ),
                        }))
                    }
                }

                if let Err(mut err) = self
                    .stack
                    .declare_variable(identifier.value.0, computed_value)
                {
                    err.message = format!("{}\nAt {:?}.", err.message, self.position);
                    return Err(Box::new(err));
                }
            }
            Statement::Assignment { identifier, value } => {
                self.visit_identifier(&identifier)?;
                self.visit_expression(&value)?;
                let value = self.read_last_result()?;
                if let Err(mut err) = self.stack.assign_variable(identifier.value.0, value) {
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
                    a => return Err(Box::new(InterpreterIssue {message: format!("Condition in if statement has to evaulate to boolean - got {:?}.\nAt {:?}.", a, self.position)})),
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
                    a => return Err(Box::new(InterpreterIssue {message: format!("Condition in for statement has to evaulate to boolean - got {:?}.\nAt {:?}.", a, self.position)})),
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
                    computed_condition = self.read_last_result()?;
                    boolean_value = match computed_condition {
                        Value::Bool(bool) => bool,
                        a => return Err(Box::new(InterpreterIssue {message: format!("Condition in for statement has to evaulate to boolean - got {:?}.\nAt {:?}.", a, self.position)})),
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
            if self.is_breaking && self.stack.is_last_scope() {
                return Err(Box::new(InterpreterIssue {
                    message: format!(
                        "Break called outside for or switch.\nAt {:?}.",
                        self.position
                    ),
                }));
            }

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
        self.visit_identifier(&parameter.value.identifier)?;
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
                        "Condition in switch case has to evaluate to boolean - got {:?}.\nAt {:?}.",
                        a, self.position
                    ),
                }))
            }
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
        if let Some(alias) = &switch_expression.value.alias {
            self.visit_expression(&switch_expression.value.expression)?;
            let computed_value = self.read_last_result()?;
            if let Err(mut err) = self
                .stack
                .declare_variable(alias.value.0.clone(), computed_value)
            {
                err.message = format!("{}\nAt {:?}.", err.message, self.position);
                return Err(Box::new(err));
            }
        }
        Ok(())
    }

    fn visit_identifier(&mut self, _identifier: &Node<Identifier>) -> Result<(), Box<dyn Issue>> {
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

    fn visit_variable(&mut self, variable: Identifier) -> Result<(), Box<dyn Issue>> {
        // read value of variable
        let value = match self.stack.get_variable(variable.0) {
            Ok(val) => val,
            Err(err) => return Err(Box::new(err)),
        };
        self.last_result = Some(value.clone());
        Ok(())
    }
}

impl Interpreter {
    fn execute_std_function(
        std_function: &StdFunction,
        arguments: Vec<Value>,
    ) -> Result<(), Box<dyn Issue>> {
        return match (std_function.execute)(arguments) {
            Ok(_) => Ok(()),
            Err(err) => Err(Box::new(err)),
        };
    }

    fn execute_function(
        &mut self,
        function_declaration: &Statement,
        arguments: Vec<Value>,
    ) -> Result<(), Box<dyn Issue>> {
        if let Statement::FunctionDeclaration {
            identifier,
            parameters,
            return_type,
            block,
        } = function_declaration
        {
            let name = identifier.value.0.clone();
            let statements = &block.value.0;
            if let Err(err) = self.stack.push_stack_frame() {
                return Err(Box::new(err));
            };

            // args
            for idx in 0..arguments.len() {
                let desired_type = parameters.get(idx).unwrap().value.parameter_type.value;
                let param_name = parameters
                    .get(idx)
                    .unwrap()
                    .value
                    .identifier
                    .value
                    .0
                    .clone();
                let value = arguments.get(idx).unwrap().clone();
                match (desired_type, value.clone()) {
                    (Type::Bool, Value::Bool(_))
                    | (Type::F64, Value::F64(_))
                    | (Type::I64, Value::I64(_))
                    | (Type::Str, Value::String(_)) => {}
                    (des, got) => {
                        return Err(Box::new(InterpreterIssue {
                            message: format!(
                                "Function '{}' expected {:?} but got {:?}.\nAt {:?}.",
                                name, des, got, self.position
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
            for statement in statements.clone() {
                if self.is_returning {
                    self.is_returning = false;
                    break;
                }

                if let Statement::FunctionDeclaration { .. } = statement.value {
                    continue;
                }

                self.visit_statement(&statement)?;
                if self.is_breaking
                    && self
                        .stack
                        .0
                        .get(self.stack.0.len() - 1)
                        .unwrap()
                        .scope_manager
                        .len()
                        == 1
                {
                    return Err(Box::new(InterpreterIssue {
                        message: format!(
                            "Break called outside for or switch.\nAt {:?}.",
                            self.position
                        ),
                    }));
                }
            }
            // check return type
            match (self.last_result.clone(), return_type.value) {
                (None, Type::Void)
                | (Some(Value::I64(_)), Type::I64)
                | (Some(Value::F64(_)), Type::F64)
                | (Some(Value::String(_)), Type::Str)
                | (Some(Value::Bool(_)), Type::Bool) => {}
                (res, exp) => {
                    return Err(Box::new(InterpreterIssue {
                        message: format!(
                            "Bad return type. Expected: {:?} but got {:?}.\nAt {:?}.",
                            exp, res, self.position
                        ),
                    }))
                }
            }

            self.stack.pop_stack_frame();
        }
        Ok(())
    }
}
