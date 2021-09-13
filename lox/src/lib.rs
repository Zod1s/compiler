// cargo run 2>/dev/null to suppress warnings
#![allow(dead_code, unused_variables)]
// #![deny(clippy::all)]
// #![allow(clippy::map_entry, clippy::enum_variant_names)]
// #![allow()]

pub mod traits;
pub mod environment;
pub mod expr;
pub mod interpreter;
pub mod parser;
pub mod scanner;
pub mod stmt;
pub mod token;
pub mod function;

use interpreter::{Interpreter, InterpreterError};
use parser::{Parser, ParserError};
use rustyline::Editor;
use scanner::{Scanner, ScannerError};
use std::fs;

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum LoxError {
    NoError,
    Error,
    ScanningError,
    ParsingError,
    RuntimeError,
}

impl LoxError {
    pub fn scanning_error(err: ScannerError) {
        eprintln!("[line {}]\nScanning error: {}", err.line(), err.message());
    }

    pub fn parsing_error(err: ParserError) {
        eprintln!(
            "[line {}]\nParsing error on {:?}: {}",
            err.token().line,
            err.token().token_type,
            err.message()
        );
    }

    pub fn runtime_error(err: InterpreterError) {
        eprintln!(
            "[line {}]\nRuntime error on operand {:?}: {}",
            err.operator().line,
            err.operator().token_type,
            err.message()
        );
    }
}

pub fn run_file(filename: &str, interpreter: &mut Interpreter) {
    let error = run(
        fs::read_to_string(filename).expect("File not found"),
        interpreter,
    );
    match error {
        LoxError::Error => eprintln!("\nGeneric error while executing the program"),
        LoxError::ScanningError => eprintln!("\nError while scanning the program"),
        LoxError::ParsingError => eprintln!("\nError while parsing the program"),
        LoxError::RuntimeError => eprintln!("\nError while running the program"),
        _ => (),
    }
}

pub fn prompt(interpreter: &mut Interpreter) {
    let mut rl = Editor::<()>::new();
    if rl.load_history("history.txt").is_err() {}
    loop {
        let readline = rl.readline(">>> ");
        match readline {
            Ok(line) => {
                rl.add_history_entry(line.as_str());
                run(line, interpreter);
            }
            Err(err) => {
                println!("{:?}", err);
                break;
            }
        }
    }
    rl.save_history("history.txt").unwrap();
}

fn run(to_execute: String, interpreter: &mut Interpreter) -> LoxError {
    let mut scanner = Scanner::new(LoxError::NoError, to_execute);
    let tokens = scanner.scan_tokens();

    if scanner.had_error() {
        LoxError::ScanningError
    } else {
        let mut parser = Parser::new(tokens, LoxError::NoError);
        let exp = parser.parse();

        if parser.had_error() {
            LoxError::ParsingError
        } else {
            interpreter.interpret(exp);
            if interpreter.had_error() {
                LoxError::RuntimeError
            } else {
                LoxError::NoError
            }
        }
    }
}
