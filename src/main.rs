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

fn parse_filename() -> Option<String> {
    let args: Vec<String> = args().collect();
    args.get(1).cloned()
}

fn on_warning(warning: Box<dyn Issue>) {
    eprintln!("{}", warning.message());
}

fn main() {
    let path = match parse_filename() {
        Some(p) => p,
        None => return eprintln!("Path to file not given."),
    };

    let file = match File::open(path.as_str()) {
        Ok(f) => f,
        Err(_) => return eprintln!("File '{}' not found.", path),
    };

    let code = BufReader::new(file);
    let reader = LazyStreamReader::new(code);

    let lexer_options = LexerOptions {
        max_comment_length: 100,
        max_identifier_length: 20,
    };

    let lexer = Lexer::new(reader, lexer_options, on_warning);
    let mut parser = Parser::new(lexer);

    let start = Instant::now();
    let program = match parser.parse() {
        Ok(p) => p,
        Err(err) => return eprintln!("{}", err.message()),
    };

    let mut semantic_checker = match SemanticChecker::new(program.clone()) {
        Ok(checker) => checker,
        Err(err) => return eprintln!("{}", err.message()),
    };
    semantic_checker.check();

    for error in &semantic_checker.errors {
        eprintln!("{}", error.message());
    }

    if semantic_checker.errors.len() > 0 {
        return;
    }

    let mut interpreter = Interpreter::new(program.clone());
    match interpreter.interpret() {
        Ok(_) => {}
        Err(err) => {
            eprintln!("{}", err.message())
        }
    };

    println!("\nExecution time: {:?}", Instant::now() - start);
}
