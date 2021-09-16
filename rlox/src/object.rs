use crate::{chunk::Chunk, types::Value, vm::Vm};
use std::fmt; //, rc::Rc, cell::{RefCell, Ref}};

// #[derive(Clone, PartialEq, Debug)]
// pub enum Object {
//     LoxString(LoxString),
//     Function(Function)
//     Closure(Closure)
// }

// impl Object {
//     pub fn string(st: LoxString) -> Object {
//         Object::LoxString(st)
//     }
//     pub fn function(f: Function) -> Object {
//         Object::Function(f)
//     }
//     pub fn closure(c: Closure) -> Object {
//         Object::Closure(c)
//     }
// }

// impl fmt::Display for Object {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         match self {
//             Object::LoxString(st) => write!(f, "{}", st),
//             Object::Function(fun) => write!(f, "{}", fun),
//             Object::Closure(c) => write!(f, "{}", c),
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

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct FunctionUpvalue {
    pub index: usize,
    pub is_local: bool,
}

impl FunctionUpvalue {
    pub fn new(index: usize, is_local: bool) -> Self {
        FunctionUpvalue { index, is_local }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Function {
    pub arity: usize,
    pub chunk: Chunk,
    pub name: String,
    pub upvalues: Vec<FunctionUpvalue>,
}

impl Function {
    pub fn new(name: String) -> Self {
        Function {
            arity: 0,
            chunk: Chunk::new(),
            name,
            upvalues: Vec::new(),
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
    fn eq(&self, _: &Self) -> bool {
        false
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Closure {
    pub function: Function,
    pub upvalues: Vec<Upvalue>,
}

impl Closure {
    pub fn new(function: Function) -> Self {
        Closure {
            function,
            upvalues: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Upvalue {
    pub location: usize,
    pub closed: Option<Value>,
}

impl Upvalue {
    pub fn new(location: usize) -> Self {
        Upvalue {
            location,
            closed: None,
        }
    }
}
