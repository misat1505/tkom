use std::{
    env::args,
    fs::File,
    io::{BufReader, Error},
    time::Instant,
};

use crate::lexer::LexerOptions;
use lexer::{ILexer, Lexer};
mod lazy_stream_reader;
use lazy_stream_reader::LazyStreamReader;
use tokens::{Token, TokenCategory};

mod lexer;
mod tokens;

fn parse_filename() -> String {
    let args: Vec<String> = args().collect();
    if args.len() >= 2 {
        return args[1].clone();
    }
    panic!("Path to file not given.");
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
    let mut lexer = Lexer::new(reader, lexer_options);
    let mut tokens: Vec<Token> = vec![];

    let start = Instant::now();
    loop {
        match lexer.generate_token() {
            Ok(token) => {
                tokens.push(token.clone());
                if token.category == TokenCategory::ETX {
                    break;
                }
            }
            Err(err) => {
                println!("{}", err.message);
                return Ok(())
            }
        }
    }
    let finish = Instant::now();

    for token in &tokens {
        println!("{:?}", token);
    }

    println!("Time {:?}", finish - start);

    Ok(())
}
