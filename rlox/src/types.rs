use crate::object::{Function, LoxString, NativeFn};
use std::fmt;

#[derive(Clone, PartialEq, Debug)]
pub enum Value {
    Number(f64),
    Bool(bool),
    Nil,
    VString(LoxString),
    Function(Function),
    NativeFn(NativeFn)
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Value::Number(n) => write!(f, "{}", n),
            Value::Bool(b) => write!(f, "{}", b),
            Value::Nil => write!(f, "Nil"),
            Value::VString(s) => write!(f, "\"{}\"", s),
            Value::Function(fun) => write!(f, "{}", fun),
            Value::NativeFn(fun) => write!(f, "{:?}", fun),
        }
    }
}

impl Value {
    // pub fn is_number(&self) -> bool {
    //     matches!(self, Value::Number(_))
    // }

    // pub fn is_bool(&self) -> bool {
    //     matches!(self, Value::Bool(_))
    // }

    // pub fn is_nil(&self) -> bool {
    //     matches!(self, Value::Nil)
    // }

    pub fn is_false(&self) -> bool {
        match self {
            Value::Bool(b) => !b,
            Value::Nil => true,
            _ => false,
        }
    }
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum InterpretError {
    Compile,
    Runtime,
}

#[derive(Copy, Clone, PartialEq, Debug, PartialOrd)]
pub enum Precedence {
    None,
    Assignment, // =
    Or,         // or
    And,        // and
    Equality,   // == !=
    Comparison, // < > <= >=
    Term,       // + -
    Factor,     // * /
    Unary,      // ! -
    Call,       // . ()
    Primary,
}

impl Precedence {
    pub fn next(&self) -> Self {
        match self {
            Precedence::None => Precedence::Assignment,
            Precedence::Assignment => Precedence::Or,
            Precedence::Or => Precedence::And,
            Precedence::And => Precedence::Equality,
            Precedence::Equality => Precedence::Comparison,
            Precedence::Comparison => Precedence::Term,
            Precedence::Term => Precedence::Factor,
            Precedence::Factor => Precedence::Unary,
            Precedence::Unary => Precedence::Call,
            Precedence::Call => Precedence::Primary,
            Precedence::Primary => Precedence::None,
        }
    }
}
