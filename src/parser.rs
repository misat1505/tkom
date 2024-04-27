use crate::{lexer::ILexer, tokens::TokenCategory};

pub struct Parser<L: ILexer> {
  lexer: L
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
      let _ = self.lexer.next();    // skip STX
      loop {
        match self.lexer.next() {
            Ok(token) => {
              println!("{:?}", token);
                if token.category == TokenCategory::ETX {
                    break;
                }
            }
            Err(err) => {
                println!("{}", err.message);
                return;
            }
        }
    }
  }
}