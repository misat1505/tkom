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

#[derive(Debug, Clone)]
pub struct FunctionsManager {
    pub functions: HashMap<String, Statement>,
}

impl FunctionsManager {
    pub fn new(program: &Program) -> Result<Self, Box<dyn Issue>> {
        let mut functions: HashMap<String, Statement> = HashMap::new();

        for statement in &program.statements {
            match &statement.value {
                Statement::FunctionDeclaration { identifier, .. } => {
                    let function_name = &identifier.value.0;
                    if functions.contains_key(function_name) {
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

        Ok(Self { functions })
    }

    pub fn get(self, function_name: String) -> Option<Statement> {
        self.functions.get(&function_name).cloned()
    }
}
