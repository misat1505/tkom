use std::collections::HashMap;

use crate::{
    ast::{Program, Statement},
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
    pub functions: HashMap<String, Statement>,
    pub std_functions: HashMap<String, StdFunction>,
}

impl FunctionsManager {
    pub fn new(program: &Program) -> Result<Self, Box<dyn Issue>> {
        let std_functions = Self::init_std();
        let mut functions: HashMap<String, Statement> = HashMap::new();

        for statement in &program.statements {
            match &statement.value {
                Statement::FunctionDeclaration { identifier, .. } => {
                    let function_name = &identifier.value.0;
                    if functions.contains_key(function_name)
                        || std_functions.contains_key(function_name)
                    {
                        return Err(Box::new(FunctionManagerIssue {
                            message: format!(
                                "Redeclaration of function '{}' in {:?}\n",
                                function_name, statement.position
                            ),
                        }));
                    }
                    let function_declaration = statement.value.clone();
                    functions.insert(function_name.to_string(), function_declaration);
                }
                _ => {}
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
        std_functions
    }
}
