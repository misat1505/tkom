use crate::{
    ast::{Identifier, Node}, lexer::ILexer, tokens::{Token, TokenCategory, TokenValue}
};

pub struct Parser<L: ILexer> {
    lexer: L,
}

#[derive(Debug, Clone)]
pub enum ParserIssueKind {
    WARNING,
    ERROR,
}

#[derive(Debug, Clone)]
pub struct ParserIssue {
    pub kind: ParserIssueKind,
    pub message: String,
}

pub trait IParser<L: ILexer> {
    fn new(lexer: L) -> Parser<L>;
    fn parse(&mut self);
}

impl<L: ILexer> IParser<L> for Parser<L> {
    fn new(lexer: L) -> Parser<L> {
        Parser { lexer }
    }

    fn parse(&mut self) {
        let _ = self.lexer.next(); // initialize
        let _ = self.lexer.next(); // skip STX

        loop {
            match self.parse_identifier() {
                Ok(node) => {
                    println!("{:?}", node);
                }
                Err(err) => {
                    println!("{}", err.message);
                    return;
                }
            }
        }
    }
}

impl<L: ILexer> Parser<L> {
  fn parse_identifier(&mut self) -> Result<Node<Identifier>, ParserIssue> {
      let token = self.consume_match(TokenCategory::Identifier)?;
      if let TokenValue::String(name) = token.value {
        let node = Node { value: Identifier(name), position: token.position };
        return Ok(node);
      }
      Err(ParserIssue { kind: ParserIssueKind::ERROR, message: "".to_owned() })
  }


    fn consume_match(&mut self, desired_category: TokenCategory) -> Result<Token, ParserIssue> {
        let current_token = self.lexer.current().clone().unwrap();
        if current_token.category == desired_category {
            let _ = self.lexer.next();
            return Ok(current_token.clone());
        }
        Err(ParserIssue {
            kind: ParserIssueKind::ERROR,
            message: format!(
                "Unexpected token - {:?}. Expected {:?}.",
                current_token.category, desired_category
            )
            .to_owned(),
        })
    }
}
