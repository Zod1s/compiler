// Preprocessor for rlox

use std::{fs, io::Result};

pub fn preprocessor(filename: &str) -> Result<String> {
    let mut program = fs::read_to_string(filename)?;
    let mut imported = vec![filename.to_string()];
    include_resolver(&mut program, &mut imported);
    // println!("{}", code);
    Ok(program)
}

/*
#include statements

- it handles #include statements for importing other rlox programs into the current file
- it scans the beginning of the program looking for #include
| statements and recursively adding the content of the included program
- #include statements must go at the beginning of the program, no #include are allowed
| after the first non-#include line
- syntax: #include {program_name}
- program_name must include file extension
- program_name can possibly contain a local path to another file (not yet implemented)
- the whole line is substituted by the content of {program_name} after having
| preprocessed it
*/

use lazy_static::lazy_static;
use regex::Regex;

const INCLUDE_HEADER: &str = "#include";
lazy_static! {
    static ref INCLUDE_PATH: Regex = Regex::new(r"#include ([a-zA-Z_]\w*.lox)").unwrap();
}

fn include_resolver(code: &mut String, imported: &mut Vec<String>) {
    let mut temp = code.split('\n').map(String::from).collect::<Vec<String>>();
    for line in temp.iter_mut() {
        if line.starts_with(INCLUDE_HEADER) {
            let matches = INCLUDE_PATH.captures(line);
            if let Some(mat) = matches {
                let file = mat[1].to_string();
                if !imported.contains(&file) {
                    imported.push(file.clone());
                    let mut import_file = fs::read_to_string(file).expect("File not found");
                    include_resolver(&mut import_file, imported);
                    *line = import_file;
                } else {
                    *line = String::new();
                }
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
