use crate::{
    gc::{Gc, GcRef, GcTrace},
    object::*,
};
use std::{any::Any, collections::HashMap, fmt};

#[derive(Clone, PartialEq, Debug, Copy)]
pub enum Value {
    Number(f64),
    Bool(bool),
    Nil,
    VString(GcRef<LoxString>),
    Function(GcRef<Function>),
    NativeFn(NativeFn),
    Closure(GcRef<Closure>),
    Class(GcRef<Class>),
    Instance(GcRef<Instance>),
}

impl GcTrace for Value {
    fn format(&self, f: &mut fmt::Formatter, gc: &Gc) -> fmt::Result {
        match self {
            Value::Number(value) => write!(f, "{}", value),
            Value::Bool(value) => write!(f, "{}", value),
            Value::Nil => write!(f, "nil"),
            Value::VString(value) => gc.deref(*value).format(f, gc),
            Value::Function(value) => gc.deref(*value).format(f, gc),
            Value::NativeFn(_) => write!(f, "<native fn>"),
            Value::Closure(value) => gc.deref(*value).format(f, gc),
            Value::Class(value) => gc.deref(*value).format(f, gc),
            Value::Instance(value) => gc.deref(*value).format(f, gc),
        }
    }

    fn size(&self) -> usize {
        0
    }

    fn trace(&self, gc: &mut Gc) {
        match self {
            Value::Function(value) => gc.mark_object(*value),
            Value::Closure(value) => gc.mark_object(*value),
            Value::VString(value) => gc.mark_object(*value),
            _ => (),
        }
    }

    fn as_any(&self) -> &dyn Any {
        panic!("Value should not be allocated")
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        panic!("Value should not be allocated")
    }
}

impl Value {
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

pub type Table = HashMap<GcRef<LoxString>, Value>;
