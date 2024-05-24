use std::collections::HashMap;

use crate::{
    ast::{Node, Program, Statement},
    errors::Issue,
    std_functions::StdFunction,
};

#[derive(Debug)]
pub struct FunctionManagerIssue {
    message: String,
}

impl Issue for FunctionManagerIssue {
    fn message(&self) -> String {
        self.message.clone()
    }
}

#[derive(Debug, Clone)]
pub struct FunctionsManager {
    pub functions: HashMap<String, Node<Statement>>,
    pub std_functions: HashMap<String, StdFunction>,
}

impl FunctionsManager {
    pub fn new(program: &Program) -> Result<Self, Box<dyn Issue>> {
        let std_functions = Self::init_std();
        let mut functions: HashMap<String, Node<Statement>> = HashMap::new();

        for statement in &program.statements {
            if let Statement::FunctionDeclaration { identifier, .. } = statement.value.clone() {
                let function_name = &identifier.value;
                if functions.contains_key(function_name) || std_functions.contains_key(function_name) {
                    return Err(Box::new(FunctionManagerIssue {
                        message: format!("Redeclaration of function '{}'.\nAt {:?}.\n", function_name, statement.position),
                    }));
                }
                functions.insert(function_name.to_string(), statement.clone());
            }
        }

        Ok(Self { functions, std_functions })
    }

    fn init_std() -> HashMap<String, StdFunction> {
        let mut std_functions: HashMap<String, StdFunction> = HashMap::new();
        std_functions.insert("print".to_owned(), StdFunction::print());
        std_functions.insert("input".to_owned(), StdFunction::input());
        std_functions.insert("mod".to_owned(), StdFunction::modulo());
        std_functions
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        ast::{Block, Type},
        lazy_stream_reader::Position,
    };

    use super::*;

    fn default_position() -> Position {
        Position {
            line: 0,
            column: 0,
            offset: 0,
        }
    }

    fn create_function_ast(name: &str) -> Node<Statement> {
        Node {
            value: Statement::FunctionDeclaration {
                identifier: Node {
                    value: String::from(name),
                    position: default_position(),
                },
                parameters: vec![],
                return_type: Node {
                    value: Type::Void,
                    position: default_position(),
                },
                block: Node {
                    value: Block(vec![]),
                    position: default_position(),
                },
            },
            position: default_position(),
        }
    }

    #[test]
    fn inserts_new_function() {
        let program = Program {
            statements: vec![create_function_ast("my_func")],
        };
        let manager = FunctionsManager::new(&program).unwrap();
        assert!(manager.functions.get(&String::from("my_func")).unwrap().clone() == create_function_ast("my_func"));
    }

    #[test]
    fn doesnt_allow_overwriting_defined_functions() {
        let program = Program {
            statements: vec![create_function_ast("my_func"), create_function_ast("my_func")],
        };
        assert!(FunctionsManager::new(&program).is_err());
    }

    #[test]
    fn doesnt_allow_overwriting_std_functions() {
        let program = Program {
            statements: vec![create_function_ast("print")],
        };
        assert!(FunctionsManager::new(&program).is_err());
    }
}
