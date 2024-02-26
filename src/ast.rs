use std::io::{Error, ErrorKind};

pub struct AstNode {
    pub token: String,
    pub children: Vec<AstNode>,
}

pub trait AstNodeActions {
    fn new(token: &str) -> Self;
    fn add_child(&mut self, ast_node: AstNode);
    fn evaluate(&self) -> Result<u32, Error>;
}

impl AstNodeActions for AstNode {
    fn new(token: &str) -> AstNode {
        let children: Vec<AstNode> = Vec::new();
        AstNode {
            token: token.to_string(),
            children,
        }
    }

    fn add_child(&mut self, ast_node: AstNode) {
        self.children.push(ast_node);
    }

    fn evaluate(&self) -> Result<u32, Error> {
        if self.token == "+" {
            let mut total = 0;
            for child in &self.children {
                total += child.evaluate()?;
            }
            Ok(total)
        } else if self.token == "-" {
            let mut total = 0;
            for child in &self.children {
                total -= child.evaluate()?;
            }
            Ok(total)
        } else if self.token == "*" {
            let mut total = 1;
            for child in &self.children {
                total *= child.evaluate()?;
            }
            Ok(total)
        } else {
            match self.token.parse::<u32>() {
                Ok(number) => Ok(number),
                Err(_) => Err(Error::new(ErrorKind::Unsupported, "Encountered bad token.")),
            }
        }
    }
}
