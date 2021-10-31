// Preprocessor for rlox

// #include statements
// - it handles #include statements for importing other rlox programs into the current file
// - should scan the beginning of the program looking for #include
// | statements and recursively adding the content of the included program
// - #include statements must go at the beginning of the program, no #include are allowed after the first non-#include line
// - syntax: #include {program_name}
// - program_name can possibly contain a local path to another file
// - the whole line should be substituted by the content of {program_name}.lox after having preprocessed it

use std::fs;

