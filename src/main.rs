use std::{
    env::args,
    fs::File,
    io::{BufReader, Error},
    time::Instant,
};

use lexer::Lexer;
mod lazy_stream_reader;
use lazy_stream_reader::LazyStreamReader;
use lexer_utils::LexerIssue;

use crate::{
    lexer_utils::LexerOptions,
    parser::{IParser, Parser},
};
mod lexer_utils;

mod ast;
mod lexer;
mod parser;
mod tokens;

mod tests;

fn parse_filename() -> String {
    let args: Vec<String> = args().collect();
    if args.len() >= 2 {
        return args[1].clone();
    }
    panic!("Path to file not given.");
}

fn on_warning(warning: LexerIssue) {
    println!("{}", warning.message);
}

fn main() -> Result<(), Error> {
    let path = parse_filename();

    let file = File::open(path.as_str())?;
    let code = BufReader::new(file);
    let reader = LazyStreamReader::new(code);

    let lexer_options = LexerOptions {
        max_comment_length: 100,
        max_identifier_length: 20,
    };

    let lexer = Lexer::new(reader, lexer_options, on_warning);
    let mut parser = Parser::new(lexer);
    let start = Instant::now();
    let parser_result = parser.parse();
    let finish = Instant::now();
    match parser_result {
        Ok(program) => {
            println!("{:?}", program);
        }
        Err(err) => {
            println!("{}", err.message);
        }
    }
    println!("Parsed in: {:?}", finish - start);

    Ok(())
}
