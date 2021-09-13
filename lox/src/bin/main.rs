// use lox::environment::Environment;
// use lox::interpreter;
// use lox::token::{LoxTypes, Literal};
use lox::*;
use std::{cmp::Ordering, env};
use lox::function::Function;

fn main() {
    let args = env::args().collect::<Vec<String>>();
    let mut globals = environment::Environment::new();
    globals.define(
        "clock".to_string(),
        token::LoxTypes::Object(token::Literal::Function(Function::clock())),
    );
    let mut interpreter = interpreter::Interpreter::new_with_env(LoxError::NoError, globals);
    match args.len().cmp(&2) {
        Ordering::Greater => panic!("Needs one argument, that is file name, or no arguments"),
        Ordering::Equal => run_file(&args[1], &mut interpreter),
        Ordering::Less => prompt(&mut interpreter),
    }
}
