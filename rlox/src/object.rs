use crate::{
    chunk::{Chunk, OpCode},
    gc::{Gc, GcRef, GcTrace},
    types::{Table, Value},
    vm::Vm,
};
use std::{any::Any, fmt, mem};

#[derive(Clone, PartialEq, Debug, PartialOrd)]
pub struct LoxString {
    pub s: String,
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

#[derive(Clone, Debug, PartialEq)]
pub struct Function {
    pub arity: usize,
    pub chunk: Chunk,
    pub name: GcRef<LoxString>,
    pub upvalues: Vec<FunctionUpvalue>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FunctionType {
    Function,
    Initializer,
    Method,
    Script,
}

#[derive(Clone, Copy)]
pub struct NativeFn(pub fn(&Vm, &[Value]) -> Result<Value, String>);

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

#[derive(Debug, Clone, PartialEq)]
pub struct Upvalue {
    pub location: usize,
    pub closed: Option<Value>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Closure {
    pub function: GcRef<Function>,
    pub upvalues: Vec<GcRef<Upvalue>>,
}

#[derive(Debug)]
pub struct Class {
    pub name: GcRef<LoxString>,
    pub methods: Table,
}

#[derive(Debug)]
pub struct Instance {
    pub class: GcRef<Class>,
    pub fields: Table,
}

#[derive(Debug)]
pub struct BoundMethod {
    pub receiver: Value,
    pub method: GcRef<Closure>,
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

impl GcTrace for Function {
    fn format(&self, f: &mut fmt::Formatter<'_>, gc: &Gc) -> fmt::Result {
        let name = &gc.deref(self.name).s;
        if name.is_empty() {
            write!(f, "<script>")
        } else {
            write!(f, "<fn {}>", name)
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

impl GcTrace for Class {
    fn format(&self, f: &mut fmt::Formatter<'_>, gc: &Gc) -> fmt::Result {
        let name = gc.deref(self.name);
        write!(f, "{}", name)
    }

    fn size(&self) -> usize {
        mem::size_of::<Class>()
    }

    fn trace(&self, gc: &mut Gc) {
        gc.mark_object(self.name);
        gc.mark_table(&self.methods);
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl GcTrace for Instance {
    fn format(&self, f: &mut fmt::Formatter<'_>, gc: &Gc) -> fmt::Result {
        let class = gc.deref(self.class);
        let name = gc.deref(class.name);
        write!(f, "{} instance", name)
    }

    fn size(&self) -> usize {
        mem::size_of::<Class>()
    }

    fn trace(&self, gc: &mut Gc) {
        gc.mark_object(self.class);
        gc.mark_table(&self.fields);
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl GcTrace for BoundMethod {
    fn format(&self, f: &mut fmt::Formatter<'_>, gc: &Gc) -> fmt::Result {
        let closure = gc.deref(self.method);
        closure.format(f, gc)
    }

    fn size(&self) -> usize {
        mem::size_of::<BoundMethod>()
    }

    fn trace(&self, gc: &mut Gc) {
        gc.mark_value(self.receiver);
        gc.mark_object(self.method);
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
