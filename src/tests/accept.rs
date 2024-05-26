#[cfg(test)]
mod tests {
    use std::io::BufReader;

    use crate::{
        errors::Issue,
        interpreter::Interpreter,
        lazy_stream_reader::LazyStreamReader,
        lexer::{Lexer, LexerOptions},
        parser::{IParser, Parser},
        semantic_checker::SemanticChecker,
        value::Value,
    };

    fn on_warning(_err: Box<dyn Issue>) {}

    fn before_each(text: BufReader<&[u8]>) -> Interpreter {
        let options = LexerOptions {
            max_comment_length: 100,
            max_identifier_length: 100,
        };
        let reader = LazyStreamReader::new(text);
        let lexer = Lexer::new(reader, options, on_warning);
        let mut parser = Parser::new(lexer);
        let program = parser.parse().unwrap();
        let mut checker = SemanticChecker::new(program.clone()).unwrap();
        checker.check();
        assert!(checker.errors.len() == 0);
        let interpreter = Interpreter::new(program);
        return interpreter;
    }

    #[test]
    fn if_statement() {
        let text = BufReader::new(
            r#"
    i64 x = 2;
    i64 y = 2;
    str text;
    if (x == y) {
        text = "equal";
    } else {
        text = "not equal";
    }
    "#
            .as_bytes(),
        );

        let mut interpreter = before_each(text);
        interpreter.interpret().unwrap();
        assert!(interpreter.stack.get_variable(String::from("text")).unwrap().clone() == Value::String(String::from("equal")));
    }

    #[test]
    fn loop_with_break() {
        let text = BufReader::new(
            r#"
    i64 i = 0;
    for (; i < 5; i = i + 1) {
      if (i == 2) {
        break;
      }
    }
    "#
            .as_bytes(),
        );

        let mut interpreter = before_each(text);
        interpreter.interpret().unwrap();
        assert!(interpreter.stack.get_variable(String::from("i")).unwrap().clone() == Value::I64(2));
    }

    #[test]
    fn functions() {
        let text = BufReader::new(
            r#"
    fn add(i64 a, i64 b): i64 {
      return a + b;
    }

    i64 a = add(1, 2);
    "#
            .as_bytes(),
        );

        let mut interpreter = before_each(text);
        interpreter.interpret().unwrap();
        assert!(interpreter.stack.get_variable(String::from("a")).unwrap().clone() == Value::I64(3));
    }

    #[test]
    fn reference() {
        let text = BufReader::new(
            r#"
    fn foo(&i64 x): void {
      x = x + 1;
    }

    i64 x = 2;
    foo(&x);
    "#
            .as_bytes(),
        );

        let mut interpreter = before_each(text);
        interpreter.interpret().unwrap();
        assert!(interpreter.stack.get_variable(String::from("x")).unwrap().clone() == Value::I64(3));
    }

    #[test]
    fn recursion() {
        let text = BufReader::new(
            r#"
    fn fib(i64 x): i64 {
      if (x == 1 || x == 2) {
        return 1;
      }

      return fib(x - 1) + fib(x - 2);
    }

    i64 x = fib(6);
    "#
            .as_bytes(),
        );

        let mut interpreter = before_each(text);
        interpreter.interpret().unwrap();
        assert!(interpreter.stack.get_variable(String::from("x")).unwrap().clone() == Value::I64(8));
    }

    #[test]
    fn is_prime() {
        let text = BufReader::new(
            r#"
    fn is_prime(i64 x): bool {
      if (x < 2) {
        return false;
      }
    
      for (i64 i = 2; i < x / 2; i = i + 1) {
        if (mod(x, i) == 0) {
          return false;
        }
      }
    
      return true;
    }

    bool is_5 = is_prime(5);
    bool is_6 = is_prime(6);
    "#
            .as_bytes(),
        );

        let mut interpreter = before_each(text);
        interpreter.interpret().unwrap();
        assert!(interpreter.stack.get_variable(String::from("is_5")).unwrap().clone() == Value::Bool(true));
        assert!(interpreter.stack.get_variable(String::from("is_6")).unwrap().clone() == Value::Bool(false));
    }

    #[test]
    fn pattern_matching() {
        let text = BufReader::new(
            r#"
    str text;
    i64 x = 10;
    switch (x) {
      (x > 0) -> {
        text = ">0";
      }
      (x > 1) -> {
        text = ">1";
        break;
      }
      (x > 2) -> {
        text = ">2";
      }
    }
    "#
            .as_bytes(),
        );

        let mut interpreter = before_each(text);
        interpreter.interpret().unwrap();
        assert!(interpreter.stack.get_variable(String::from("text")).unwrap().clone() == Value::String(String::from(">1")));
    }
}