use crate::{
    chunk::{Chunk, OpCode},
    gc::{Gc, GcRef, GcTrace},
    types::{Table, Value},
    vm::Vm,
};
use std::{any::Any, fmt, mem};

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct FunctionUpvalue {
    pub index: usize,
    pub is_local: bool,
}

impl FunctionUpvalue {
    #[inline]
    pub fn new(index: usize, is_local: bool) -> Self {
        Self { index, is_local }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Function {
    pub arity: usize,
    pub chunk: Chunk,
    pub name: GcRef<String>,
    pub upvalues: Vec<FunctionUpvalue>,
}

impl Function {
    #[inline]
    pub fn new(name: GcRef<String>) -> Self {
        Self {
            arity: 0,
            chunk: Chunk::new(),
            name,
            upvalues: Vec::new(),
        }
    }
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
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "<fn>")
    }
}

impl PartialEq for NativeFn {
    #[inline]
    fn eq(&self, _: &Self) -> bool {
        false
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Upvalue {
    pub location: usize,
    pub closed: Option<Value>,
}

impl Upvalue {
    #[inline]
    pub fn new(location: usize) -> Self {
        Self {
            location,
            closed: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Closure {
    pub function: GcRef<Function>,
    pub upvalues: Vec<GcRef<Upvalue>>,
}

impl Closure {
    #[inline]
    pub fn new(function: GcRef<Function>) -> Self {
        Self {
            function,
            upvalues: Vec::new(),
        }
    }
}

#[derive(Debug)]
pub struct Class {
    pub name: GcRef<String>,
    pub methods: Table,
}

impl Class {
    #[inline]
    pub fn new(name: GcRef<String>) -> Self {
        Self {
            name,
            methods: Table::new(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Instance {
    pub class: GcRef<Class>,
    pub fields: Table,
}

impl Instance {
    #[inline]
    pub fn new(class: GcRef<Class>) -> Self {
        Self {
            class,
            fields: Table::new(),
        }
    }
}

#[derive(Debug)]
pub struct BoundMethod {
    pub receiver: Value,
    pub method: GcRef<Closure>,
}

impl BoundMethod {
    #[inline]
    pub fn new(receiver: Value, method: GcRef<Closure>) -> Self {
        Self { receiver, method }
    }
}

impl GcTrace for String {
    #[inline]
    fn format(&self, f: &mut fmt::Formatter<'_>, _gc: &Gc) -> fmt::Result {
        write!(f, "\"{}\"", self)
    }

    #[inline]
    fn size(&self) -> usize {
        mem::size_of::<String>() + self.as_bytes().len()
    }

    #[inline]
    fn trace(&self, _gc: &mut Gc) {}

    #[inline]
    fn as_any(&self) -> &dyn Any {
        self
    }

    #[inline]
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl GcTrace for Function {
    #[inline]
    fn format(&self, f: &mut fmt::Formatter<'_>, gc: &Gc) -> fmt::Result {
        let name = &gc.deref(self.name);
        if name.is_empty() {
            write!(f, "<script>")
        } else {
            write!(f, "<fn {}>", name)
        }
    }

    #[inline]
    fn size(&self) -> usize {
        mem::size_of::<Function>()
            + self.upvalues.capacity() * mem::size_of::<FunctionUpvalue>()
            + self.chunk.code.capacity() * mem::size_of::<OpCode>()
            + self.chunk.constants.capacity() * mem::size_of::<Value>()
            + self.chunk.constants.capacity() * mem::size_of::<usize>()
    }

    #[inline]
    fn trace(&self, gc: &mut Gc) {
        gc.mark_object(self.name);
        for &constant in &self.chunk.constants {
            gc.mark_value(constant);
        }
    }

    #[inline]
    fn as_any(&self) -> &dyn Any {
        self
    }

    #[inline]
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl GcTrace for Upvalue {
    #[inline]
    fn format(&self, f: &mut fmt::Formatter<'_>, _gc: &Gc) -> fmt::Result {
        write!(f, "upvalue")
    }

    #[inline]
    fn size(&self) -> usize {
        mem::size_of::<Upvalue>()
    }

    #[inline]
    fn trace(&self, gc: &mut Gc) {
        if let Some(obj) = self.closed {
            gc.mark_value(obj)
        }
    }

    #[inline]
    fn as_any(&self) -> &dyn Any {
        self
    }

    #[inline]
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl GcTrace for Closure {
    #[inline]
    fn format(&self, f: &mut fmt::Formatter<'_>, gc: &Gc) -> fmt::Result {
        gc.deref(self.function).format(f, gc)
    }

    #[inline]
    fn size(&self) -> usize {
        mem::size_of::<Closure>() + self.upvalues.capacity() * mem::size_of::<GcRef<Upvalue>>()
    }

    #[inline]
    fn trace(&self, gc: &mut Gc) {
        gc.mark_object(self.function);
        for &upvalue in &self.upvalues {
            gc.mark_object(upvalue);
        }
    }

    #[inline]
    fn as_any(&self) -> &dyn Any {
        self
    }

    #[inline]
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl GcTrace for Class {
    #[inline]
    fn format(&self, f: &mut fmt::Formatter<'_>, gc: &Gc) -> fmt::Result {
        let name = gc.deref(self.name);
        write!(f, "{}", name)
    }

    #[inline]
    fn size(&self) -> usize {
        mem::size_of::<Class>()
    }

    #[inline]
    fn trace(&self, gc: &mut Gc) {
        gc.mark_object(self.name);
        gc.mark_table(&self.methods);
    }

    #[inline]
    fn as_any(&self) -> &dyn Any {
        self
    }

    #[inline]
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl GcTrace for Instance {
    #[inline]
    fn format(&self, f: &mut fmt::Formatter<'_>, gc: &Gc) -> fmt::Result {
        let class = gc.deref(self.class);
        let name = gc.deref(class.name);
        write!(f, "{} instance", name)
    }

    #[inline]
    fn size(&self) -> usize {
        mem::size_of::<Class>()
    }

    #[inline]
    fn trace(&self, gc: &mut Gc) {
        gc.mark_object(self.class);
        gc.mark_table(&self.fields);
    }

    #[inline]
    fn as_any(&self) -> &dyn Any {
        self
    }

    #[inline]
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl GcTrace for BoundMethod {
    #[inline]
    fn format(&self, f: &mut fmt::Formatter<'_>, gc: &Gc) -> fmt::Result {
        let closure = gc.deref(self.method);
        closure.format(f, gc)
    }

    #[inline]
    fn size(&self) -> usize {
        mem::size_of::<BoundMethod>()
    }

    #[inline]
    fn trace(&self, gc: &mut Gc) {
        gc.mark_value(self.receiver);
        gc.mark_object(self.method);
    }

    #[inline]
    fn as_any(&self) -> &dyn Any {
        self
    }

    #[inline]
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl GcTrace for Vec<Value> {
    #[inline]
    fn format(&self, f: &mut fmt::Formatter<'_>, gc: &Gc) -> fmt::Result {
        write!(f, "[")?;
        for i in 0..self.len() {
            self[i].format(f, gc)?;
            if i != self.len() - 1 {
                write!(f, ", ")?;
            }
        }
        write!(f, "]")
    }

    #[inline]
    fn size(&self) -> usize {
        mem::size_of::<Vec<Value>>() + self.capacity() * mem::size_of::<Value>()
    }

    #[inline]
    fn trace(&self, gc: &mut Gc) {
        for &value in self {
            gc.mark_value(value);
        }
    }

    #[inline]
    fn as_any(&self) -> &dyn Any {
        self
    }

    #[inline]
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
