use std::{env::args, fs::File, io::BufReader, time::Instant};

use errors::Issue;
use lexer::Lexer;
mod lazy_stream_reader;
use lazy_stream_reader::LazyStreamReader;

use crate::{
    interpreter::Interpreter,
    lexer::LexerOptions,
    parser::{IParser, Parser},
    semantic_checker::SemanticChecker,
};

#[allow(non_snake_case)]
mod ALU;
mod ast;
mod errors;
mod functions_manager;
mod interpreter;
mod lexer;
mod parser;
mod scope_manager;
mod semantic_checker;
mod stack;
mod std_functions;
mod tokens;
mod value;
mod visitor;

mod tests;

fn parse_filename() -> String {
    let args: Vec<String> = args().collect();
    if args.len() >= 2 {
        return args[1].clone();
    }
    panic!("Path to file not given.");
}

fn on_warning(warning: Box<dyn Issue>) {
    println!("{}", warning.message());
}

fn main() -> Result<(), Box<dyn Issue>> {
    let path = parse_filename();

    let file = File::open(path.as_str()).unwrap();
    let code = BufReader::new(file);
    let reader = LazyStreamReader::new(code);

    let lexer_options = LexerOptions {
        max_comment_length: 100,
        max_identifier_length: 20,
    };

    let lexer = Lexer::new(reader, lexer_options, on_warning);
    let mut parser = Parser::new(lexer);

    let start = Instant::now();
    let program = parser.parse()?;

    let mut semantic_checker = SemanticChecker::new(program.clone())?;
    semantic_checker.check();

    for error in &semantic_checker.errors {
        println!("{}", error.message());
    }

    if semantic_checker.errors.len() > 0 {
        return Ok(());
    }

    let mut interpreter = Interpreter::new(program.clone());
    match interpreter.interpret() {
        Ok(_) => {}
        Err(err) => {
            println!("{}", err.message())
        }
    };

    println!("Execution time: {:?}", Instant::now() - start);

    Ok(())
}
