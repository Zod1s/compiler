use crate::{
    chunk::{Chunk, OpCode},
    gc::{Gc, GcRef, GcTrace},
    types::Value,
    vm::Vm,
};
use std::{any::Any, fmt, mem};

#[derive(Clone, PartialEq, Debug, PartialOrd)]
pub struct LoxString {
    pub s: String,
}

impl LoxString {
    pub fn new(s: String) -> Self {
        LoxString { s }
    }
}

impl GcTrace for LoxString {
    fn format(&self, f: &mut fmt::Formatter<'_>, _gc: &Gc) -> fmt::Result {
        write!(f, "{}", self)
    }

    fn size(&self) -> usize {
        mem::size_of::<String>() + self.s.as_bytes().len()
    }

    fn trace(&self, _gc: &mut Gc) {}

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl fmt::Display for LoxString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "\"{}\"", self.s)
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

impl fmt::Display for FunctionUpvalue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "index: {}, local: {}", self.index, self.is_local)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Function {
    pub arity: usize,
    pub chunk: Chunk,
    pub name: GcRef<LoxString>,
    pub upvalues: Vec<FunctionUpvalue>,
}

impl Function {
    pub fn new(name: GcRef<LoxString>) -> Self {
        Function {
            arity: 0,
            chunk: Chunk::new(),
            name,
            upvalues: Vec::new(),
        }
    }
}

impl GcTrace for Function {
    fn format(&self, f: &mut fmt::Formatter<'_>, gc: &Gc) -> fmt::Result {
        let name = &gc.deref(self.name).s;
        if name.is_empty() {
            write!(f, "<script>")
        } else {
            write!(f, "{}", name)
        }
    }

    fn size(&self) -> usize {
        mem::size_of::<Function>()
            + self.upvalues.capacity() * mem::size_of::<FunctionUpvalue>()
            + self.chunk.code.capacity() * mem::size_of::<OpCode>()
            + self.chunk.constants.capacity() * mem::size_of::<Value>()
            + self.chunk.constants.capacity() * mem::size_of::<usize>()
    }

    fn trace(&self, gc: &mut Gc) {
        gc.mark_object(self.name);
        for &constant in &self.chunk.constants {
            gc.mark_value(constant);
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl fmt::Display for Function {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<fn {:?}>", self.name)
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
    pub function: GcRef<Function>,
    pub upvalues: Vec<GcRef<Upvalue>>,
}

impl Closure {
    pub fn new(function: GcRef<Function>) -> Self {
        Closure {
            function,
            upvalues: Vec::new(),
        }
    }
}

impl GcTrace for Closure {
    fn format(&self, f: &mut fmt::Formatter<'_>, gc: &Gc) -> fmt::Result {
        gc.deref(self.function).format(f, gc)
    }

    fn size(&self) -> usize {
        mem::size_of::<Closure>() + self.upvalues.capacity() * mem::size_of::<GcRef<Upvalue>>()
    }

    fn trace(&self, gc: &mut Gc) {
        gc.mark_object(self.function);
        for &upvalue in &self.upvalues {
            gc.mark_object(upvalue);
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
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

impl GcTrace for Upvalue {
    fn format(&self, f: &mut fmt::Formatter<'_>, _gc: &Gc) -> fmt::Result {
        write!(f, "upvalue")
    }

    fn size(&self) -> usize {
        mem::size_of::<Upvalue>()
    }

    fn trace(&self, gc: &mut Gc) {
        if let Some(obj) = self.closed {
            gc.mark_value(obj)
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
