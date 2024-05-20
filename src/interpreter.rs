use crate::{
    ast::{
        Argument, Block, Expression, Identifier, Literal, Node, Parameter, Program, Statement,
        SwitchCase, SwitchExpression, Type,
    },
    errors::Issue,
    scope_manager::ScopeManager,
    value::{ComputationIssue, Value},
    visitor::Visitor,
};

pub struct Interpreter {
    program: Program,
    scope_manager: ScopeManager,
    last_result: Option<Value>,
}

impl Interpreter {
    pub fn new(program: Program) -> Self {
        Interpreter {
            program,
            scope_manager: ScopeManager::new(),
            last_result: None,
        }
    }

    pub fn interpret(&mut self) -> Result<(), Box<dyn Issue>> {
        self.visit_program(&self.program.clone());
        Ok(())
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
    ) where
        F: Fn(&Value, Value) -> Result<Value, ComputationIssue>,
    {
        self.visit_expression(&lhs);
        let left_value = self.read_last_result();
        self.visit_expression(&rhs);
        let right_value = self.read_last_result();

        self.last_result = Some(op(&left_value, right_value).unwrap());
    }

    fn evaluate_unary_op<F>(&mut self, value: Box<Node<Expression>>, op: F)
    where
        F: Fn(&Value) -> Result<Value, ComputationIssue>,
    {
        self.visit_expression(&value);
        let computed_value = self.read_last_result();
        self.last_result = Some(op(&computed_value).unwrap());
    }
}

impl Visitor for Interpreter {
    fn visit_program(&mut self, program: &Program) {
        for statement in program.statements.clone() {
            self.visit_statement(&statement);
        }
    }

    fn visit_expression(&mut self, expression: &Node<Expression>) {
        match expression.value.clone() {
            Expression::Casting { value, to_type } => {
                self.visit_expression(&value);
                let computed_value = self.read_last_result();
                let casted_value = computed_value.cast_to_type(to_type.value).unwrap();
                self.last_result = Some(casted_value);
            },
            Expression::BooleanNegation(value) => self.evaluate_unary_op(value, Value::boolean_negate),
            Expression::ArithmeticNegation(value) => self.evaluate_unary_op(value, Value::arithmetic_negate),
            Expression::Addition(lhs, rhs) => self.evaluate_binary_op(lhs, rhs, Value::add),
            Expression::Subtraction(lhs, rhs) => self.evaluate_binary_op(lhs, rhs, Value::subtract),
            Expression::Multiplication(lhs, rhs) => {
                self.evaluate_binary_op(lhs, rhs, Value::multiplication)
            }
            Expression::Division(lhs, rhs) => self.evaluate_binary_op(lhs, rhs, Value::division),
            Expression::Alternative(lhs, rhs) => self.evaluate_binary_op(lhs, rhs, Value::alternative),
            Expression::Concatenation(lhs, rhs) => self.evaluate_binary_op(lhs, rhs, Value::concatenation),
            Expression::Greater(lhs, rhs) => self.evaluate_binary_op(lhs, rhs, Value::greater),
            Expression::GreaterEqual(lhs, rhs) => self.evaluate_binary_op(lhs, rhs, Value::greater_or_equal),
            Expression::Less(lhs, rhs) => self.evaluate_binary_op(lhs, rhs, Value::less),
            Expression::LessEqual(lhs, rhs) => self.evaluate_binary_op(lhs, rhs, Value::less_or_equal),
            Expression::Equal(lhs, rhs) => self.evaluate_binary_op(lhs, rhs, Value::equal),
            Expression::NotEqual(lhs, rhs) => self.evaluate_binary_op(lhs, rhs, Value::not_equal),
            Expression::Literal(literal) => self.visit_literal(literal),
            Expression::Variable(variable) => self.visit_variable(variable),
            Expression::FunctionCall {
                identifier,
                arguments,
            } => {
                self.visit_identifier(&identifier);
                for arg in arguments {
                    self.visit_argument(&arg);
                }
            }
        }
    }

    fn visit_statement(&mut self, statement: &Node<Statement>) {
        match statement.value.clone() {
            Statement::FunctionDeclaration {
                identifier,
                parameters,
                return_type,
                block,
            } => {
                self.visit_identifier(&identifier);
                for param in parameters {
                    self.visit_parameter(&param)
                }
                self.visit_type(&return_type);
                self.visit_block(&block);
            }
            Statement::FunctionCall {
                identifier,
                arguments,
            } => {
                self.visit_identifier(&identifier);
                for arg in arguments {
                    self.visit_argument(&arg);
                }
            }
            Statement::Declaration {
                var_type,
                identifier,
                value,
            } => {
                self.visit_type(&var_type);
                self.visit_identifier(&identifier);

                let computed_value = match value {
                    Some(val) => {
                        self.visit_expression(&val);
                        let result = self.read_last_result();
                        result
                    }
                    None => Value::default_value(var_type.value).unwrap(),
                };

                self.scope_manager
                    .declare_variable(identifier.value.0, computed_value)
                    .unwrap();
                println!("{:?}", self.scope_manager.clone());
            }
            Statement::Assignment { identifier, value } => {
                self.visit_identifier(&identifier);
                self.visit_expression(&value);
                let value = self.read_last_result();
                self.scope_manager
                    .assign_variable(identifier.value.0, value)
                    .unwrap();
                println!("{:?}", self.scope_manager.clone());
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
    }

    fn visit_argument(&mut self, argument: &Node<Argument>) {
        self.visit_expression(&argument.value.value);
    }

    fn visit_block(&mut self, block: &Node<Block>) {
        self.scope_manager.push_scope();
        println!("{:?}", self.scope_manager.clone());
        for statement in &block.value.0 {
            self.visit_statement(statement);
        }
        self.scope_manager.pop_scope();
        println!("{:?}", self.scope_manager.clone());
    }

    fn visit_parameter(&mut self, parameter: &Node<Parameter>) {
        self.visit_type(&parameter.value.parameter_type);
        self.visit_identifier(&parameter.value.identifier);
    }

    fn visit_switch_case(&mut self, switch_case: &Node<SwitchCase>) {
        self.visit_expression(&switch_case.value.condition);
        self.visit_block(&switch_case.value.block);
    }

    fn visit_switch_expression(&mut self, switch_expression: &Node<SwitchExpression>) {
        self.visit_expression(&switch_expression.value.expression);
    }

    fn visit_identifier(&mut self, _identifier: &Node<Identifier>) {
        // println!("{:?}", _identifier);
    }

    fn visit_type(&mut self, _node_type: &Node<Type>) {
        // println!("{:?}", _node_type);
    }

    fn visit_literal(&mut self, literal: Literal) {
        // change literal to value
        let value = match literal {
            Literal::F64(f64) => Value::F64(f64),
            Literal::I64(i64) => Value::I64(i64),
            Literal::String(str) => Value::String(str),
            Literal::False => Value::Bool(false),
            Literal::True => Value::Bool(true),
        };

        self.last_result = Some(value);
    }

    fn visit_variable(&mut self, variable: Identifier) {
        // read value of variable
        let value = self.scope_manager.get_variable(variable.0).unwrap();
        self.last_result = Some(value.clone());
    }
}
