// Preprocessor for rlox

pub fn preprocessor(code: &mut String) {
    include_resolver(code);
    // println!("{}", code);
}

// #include statements
// - it handles #include statements for importing other rlox programs into the current file
// - should scan the beginning of the program looking for #include
//  |statements and recursively adding the content of the included program
// - #include statements must go at the beginning of the program, no #include are allowed after the first non-#include line
// - syntax: #include {program_name}
// - program_name can possibly contain a local path to another file (not yet implemented)
// - the whole line should be substituted by the content of {program_name}.lox after having preprocessed it
// KNOWN ISSUES:
// - can import multiple times the same file if different imports use it
// - infinite import loop

use lazy_static::lazy_static;
use regex::Regex;
use std::fs;

const INCLUDE_HEADER: &str = "#include";
lazy_static! {
    static ref INCLUDE_PATH: Regex = Regex::new(r"#include ([a-zA-Z_]\w*.lox)").unwrap();
}

fn include_resolver(code: &mut String) {
    let mut temp = code.split('\n').map(String::from).collect::<Vec<String>>();
    for line in temp.iter_mut() {
        if line.starts_with(INCLUDE_HEADER) {
            let matches = INCLUDE_PATH.captures(&line);
            if let Some(mat) = matches {
                let mut import_file = fs::read_to_string(&mat[1]).expect("File not found");
                include_resolver(&mut import_file);
                *line = import_file;
            } else {
                panic!("INCLUDE Error: expected filename, found {}", line);
            }
        } else if line.is_empty() {
            continue;
        } else {
            break;
        }
    }
    *code = temp.join("\n");
}
