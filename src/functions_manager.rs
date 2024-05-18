use std::collections::HashMap;

use crate::{
    ast::{Program, Statement},
    errors::Issue,
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

pub struct FunctionsManager {
    pub functions: HashMap<String, Statement>,
}

impl FunctionsManager {
    pub fn new(program: &Program) -> Result<Self, Box<dyn Issue>> {
        let mut functions: HashMap<String, Statement> = HashMap::new();

        for statement in &program.statements {
            match &statement.value {
                Statement::FunctionDeclaration {
                    identifier,
                    parameters: _,
                    return_type: _,
                    block: _,
                } => {
                    let function_name = &identifier.value.0;
                    if functions.contains_key(function_name) {
                        return Err(Box::new(FunctionManagerIssue {
                            message: format!(
                                "Redeclaration of function '{}' in {:?}",
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

        Ok(Self { functions })
    }
}
