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
                if functions.contains_key(function_name)
                    || std_functions.contains_key(function_name)
                {
                    return Err(Box::new(FunctionManagerIssue {
                        message: format!(
                            "Redeclaration of function '{}'.\nAt {:?}.\n",
                            function_name, statement.position
                        ),
                    }));
                }
                functions.insert(function_name.to_string(), statement.clone());
            }
        }

        Ok(Self {
            functions,
            std_functions,
        })
    }

    fn init_std() -> HashMap<String, StdFunction> {
        let mut std_functions: HashMap<String, StdFunction> = HashMap::new();
        std_functions.insert("print".to_owned(), StdFunction::print());
        std_functions.insert("input".to_owned(), StdFunction::input());
        std_functions.insert("mod".to_owned(), StdFunction::modulo());
        std_functions
    }
}
