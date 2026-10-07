mod executor;
mod lexer;
mod parser;

use std::io::{self, Write};

use crate::{executor::execute_pipeline, lexer::tokenize, parser::Parser};

fn main() {
    loop {
        print!("$");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();

        let tokens = match tokenize(&input) {
            Ok(toks) => toks,
            Err(err) => {
                eprintln!("Lexer error: {err}");
                continue;
            }
        };

        let mut parser = Parser::new(tokens);
        let ast = match parser.parse() {
            Ok(pipeline) => pipeline,
            Err(err) => {
                eprintln!("parser error: {err}");
                continue;
            }
        };

        let _ = execute_pipeline(ast);
    }
}
