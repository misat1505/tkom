pub mod ast;
use std::{env::args, fs::{self, File}, io::{BufReader, Error}};

use crate::{ast::AstNodeActions, char_reader::{CharRead, ETX}};
use ast::AstNode;
mod lexer;
use lexer::lexer;
mod char_reader;
use char_reader::CharReader;

#[allow(dead_code)]
fn test1() {
    // does 2 + 3 * 5, if error prints 0

    let mut addition = AstNode::new("+");
    let mut multiplication = AstNode::new("*");
    let num1 = AstNode::new("3");
    let num2 = AstNode::new("5");
    let num3 = AstNode::new("2");

    multiplication.add_child(num1);
    multiplication.add_child(num2);
    addition.add_child(num3);
    addition.add_child(multiplication);

    println!("Value: {}", addition.evaluate().unwrap_or(0));
}

fn parse_filename() -> String {
    let args: Vec<String> = args().collect();
    if args.len() >= 2 {
        return args[1].clone();
    }
    panic!("Path to file not given.");
}

fn read_file(path: &str) -> String {
    let content = fs::read_to_string(path).expect("File not found.");
    content
}

fn main() -> Result<(), Error> {
    // test1();
    let path = parse_filename();
    let file_content = read_file(path.as_str());
    let src_code = format!(r#"{}"#, file_content);
    println!("{}", src_code);

    let tokens = lexer(src_code.as_str());
    for token in tokens {
        println!("{:?}", token);
    }

    let file = File::open(path.as_str())?;
    let code = BufReader::new(file);
    let mut reader = CharReader::new(code);

    while let Ok(char) = reader.next() {
        if *char == ETX {break;}
        print!("{}", char);
    }

    Ok(())
}
