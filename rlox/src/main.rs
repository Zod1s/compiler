// #![allow(
//     dead_code,
//     unused_variables,
//     unreachable_patterns,
//     irrefutable_let_patterns,
//     clippy::map_entry,
//     clippy::enum_variant_names
// )]
#![deny(clippy::all)]

mod chunk;
mod compiler;
mod gc;
mod object;
mod preprocessor;
mod scanner;
mod types;
mod vm;

use preprocessor::preprocessor;
use rustyline::Editor;
use std::{env::args, process::exit};
use types::InterpretError;
use vm::Vm;

fn main() {
    let args = args().collect::<Vec<String>>();
    match args.len() {
        1 => repl(Vm::new(true)),
        2 => run_file(&args[1], Vm::new(false)),
        3 => dump(&args[1], Vm::new(false), &args[2]),
        _ => {
            eprintln!("Needs one argument, that is file name, or no arguments");
            exit(1);
        }
    }
}

pub fn run_file(filename: &str, mut vm: Vm) {
    let program = preprocessor(filename).expect("File not found");
    match vm.interpret(&program) {
        Err(InterpretError::Runtime) => {
            drop(vm);
            exit(70);
        }
        Err(InterpretError::Compile) => {
            drop(vm);
            exit(65);
        }
        _ => (),
    }
}

pub fn repl(mut vm: Vm) {
    let mut rl = Editor::<()>::new();
    if rl.load_history("history.txt").is_err() {}
    loop {
        let readline = rl.readline(">> ");
        match readline {
            Ok(line) => {
                rl.add_history_entry(line.as_str());
                if line == ":set debug" {
                    println!("> debug flag set");
                    vm.set_debug();
                } else if line == ":unset debug" {
                    println!("> debug flag unset");
                    vm.unset_debug();
                } else if line == ":quit" || line == ":q" {
                    println!("> quitting");
                    break;
                } else if line.starts_with(":load") {
                    let file = if let Ok(f) = preprocessor(line.trim_start_matches(":load ")) {
                        f
                    } else {
                        eprintln!(
                            "No file with name {} found.",
                            line.trim_start_matches(":load ")
                        );
                        continue;
                    };
                    let _ = vm.interpret(&file);
                } else {
                    let _ = vm.interpret(&line);
                }
            }
            Err(err) => {
                println!("{:?}", err);
                break;
            }
        }
    }
    rl.save_history("history.txt").unwrap();
}

pub fn dump(filename: &str, mut vm: Vm, to_dump: &str) {
    let program = preprocessor(filename).expect("File not found");
    match vm.dump(&program, to_dump) {
        Err(InterpretError::Runtime) => {
            println!("Error while running.");
            drop(vm);
            exit(70);
        }
        Err(InterpretError::Compile) => {
            println!("Error while compiling.");
            drop(vm);
            exit(65);
        }
        _ => (),
    }
}
