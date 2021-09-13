use crate::chunk::Chunk;
use crate::types::Value;
use crate::vm::Vm;
use std::fmt;

// #[derive(Clone, PartialEq, Debug)]
// pub enum Object {
//     LoxString(LoxString),
//     Function(Function)
// }

// impl Object {
//     pub fn string(st: LoxString) -> Object {
//         Object::LoxString(st)
//     }
//     pub fn function(f: Function) -> Object {
//         Object::Function(f)
//     }
// }

// impl fmt::Display for Object {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         match self {
//             Object::LoxString(st) => write!(f, "{}", st),
//             Object::Function(f) => write!(f, "{}", f),
//         }
//     }
// }

#[derive(Clone, PartialEq, Debug, PartialOrd)]
pub struct LoxString {
    pub s: String,
}

impl LoxString {
    pub fn new(s: String) -> Self {
        LoxString { s }
    }
}

impl fmt::Display for LoxString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.s)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Function {
    pub arity: usize,
    pub chunk: Chunk,
    pub name: String,
}

impl Function {
    pub fn new(name: String) -> Self {
        Function {
            arity: 0,
            chunk: Chunk::new(),
            name,
        }
    }
}

impl fmt::Display for Function {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<fn {}>", self.name)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FunctionType {
    Function,
    // Initializer,
    // Method,
    Script,
}

#[derive(Clone, Copy)]
pub struct NativeFn(pub fn(&Vm, &[Value]) -> Value);

impl fmt::Debug for NativeFn {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "<fn>")
    }
}

impl PartialEq for NativeFn {
    fn eq(&self, other: &Self) -> bool {
        false
    }
}
