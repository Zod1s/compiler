use crate::{
    gc::{Gc, GcRef, GcTrace},
    object::*,
};
use std::{any::Any, collections::HashMap, fmt};

#[derive(Clone, PartialEq, Debug, Copy)]
pub enum Value {
    Array(GcRef<Vec<Value>>),
    Bool(bool),
    BoundMethod(GcRef<BoundMethod>),
    Class(GcRef<Class>),
    Closure(GcRef<Closure>),
    Function(GcRef<Function>),
    Instance(GcRef<Instance>),
    NativeFn(NativeFn),
    Nil,
    Number(f64),
    VString(GcRef<String>),
}

impl Value {
    pub fn is_false(&self) -> bool {
        match self {
            Value::Bool(b) => !b,
            Value::Nil => true,
            _ => false,
        }
    }

    pub fn type_of(&self) -> &str {
        match self {
            Value::Array(_) => "array",
            Value::Bool(_) => "bool",
            Value::BoundMethod(_) => "bound method",
            Value::Class(_) => "class",
            Value::Closure(_) => "closure",
            Value::Function(_) => "function",
            Value::Instance(_) => "instance",
            Value::NativeFn(_) => "native function",
            Value::Nil => "nil",
            Value::Number(_) => "number",
            Value::VString(_) => "string",
        }
    }

    // pub fn is_number(&self) -> bool {
    //     matches!(self, Value::Number(_))
    // }
}

impl GcTrace for Value {
    fn format(&self, f: &mut fmt::Formatter, gc: &Gc) -> fmt::Result {
        match self {
            Value::Array(value) => gc.deref(*value).format(f, gc),
            Value::Bool(value) => write!(f, "{}", value),
            Value::BoundMethod(value) => gc.deref(*value).format(f, gc),
            Value::Class(value) => gc.deref(*value).format(f, gc),
            Value::Closure(value) => gc.deref(*value).format(f, gc),
            Value::Function(value) => gc.deref(*value).format(f, gc),
            Value::Instance(value) => gc.deref(*value).format(f, gc),
            Value::NativeFn(_) => write!(f, "<native fn>"),
            Value::Nil => write!(f, "nil"),
            Value::Number(value) => write!(f, "{}", value),
            Value::VString(value) => gc.deref(*value).format(f, gc),
        }
    }

    #[inline]
    fn size(&self) -> usize {
        0
    }

    fn trace(&self, gc: &mut Gc) {
        match self {
            Value::Closure(value) => gc.mark_object(*value),
            Value::Function(value) => gc.mark_object(*value),
            Value::VString(value) => gc.mark_object(*value),
            Value::BoundMethod(value) => gc.mark_object(*value),
            Value::Class(value) => gc.mark_object(*value),
            Value::Instance(value) => gc.mark_object(*value),
            _ => (),
        }
    }

    #[inline]
    fn as_any(&self) -> &dyn Any {
        panic!("Value should not be allocated")
    }

    #[inline]
    fn as_any_mut(&mut self) -> &mut dyn Any {
        panic!("Value should not be allocated")
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

pub type Table = HashMap<GcRef<String>, Value>;
