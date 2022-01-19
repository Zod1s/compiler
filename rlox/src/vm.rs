use crate::{
    chunk::{Chunk, Disassembler, OpCode},
    compiler,
    gc::{Gc, GcRef, GcTrace, GcTraceFormatter},
    object::*,
    types::{InterpretError, Table, Value},
};
use cpu_time::ProcessTime;
use std::{fmt, fs, process};

pub struct Vm {
    debug: bool,
    repl: bool,
    gc: Gc,
    stack: Vec<Value>,
    globals: Table,
    frames: Vec<CallFrame>,
    open_upvalues: Vec<GcRef<Upvalue>>,
    start_time: ProcessTime,
    init_string: GcRef<String>,
}

impl Vm {
    // public interface

    pub fn new(repl: bool) -> Self {
        let mut gc = Gc::new();
        let init_string = gc.intern("init".to_owned());

        let mut vm = Vm {
            debug: false,
            repl,
            gc,
            stack: Vec::new(),
            globals: Table::new(),
            frames: Vec::new(),
            open_upvalues: Vec::new(),
            start_time: ProcessTime::now(),
            init_string,
        };

        // native function definition
        vm.define_native("clock", NativeFn(clock));
        vm.define_native("panic", NativeFn(lox_panic));
        // vm.define_native("sqrt", NativeFn(sqrt));
        // vm.define_native("pow", NativeFn(pow));
        // vm.define_native("square", NativeFn(square));
        // vm.define_native("abs", NativeFn(abs));
        vm.define_native("min", NativeFn(min));
        vm.define_native("max", NativeFn(max));
        // vm.define_native("floor", NativeFn(floor));
        // vm.define_native("ceil", NativeFn(ceil));
        vm.define_native("isBool", NativeFn(is_bool));
        vm.define_native("isClass", NativeFn(is_class));
        vm.define_native("isClosure", NativeFn(is_closure));
        vm.define_native("isFunction", NativeFn(is_function));
        vm.define_native("isInstance", NativeFn(is_instance));
        vm.define_native("isNil", NativeFn(is_nil));
        vm.define_native("isNumber", NativeFn(is_number));
        vm.define_native("isString", NativeFn(is_string));
        vm.define_native("instanceof", NativeFn(instance_of));
        // vm.define_native("float", NativeFn(float));

        vm
    }

    pub fn interpret(&mut self, code: &str) -> Result<(), InterpretError> {
        let function = compiler::compile(code, &mut self.gc)?;
        self.push(Value::Function(function))?;
        let closure = self.alloc(Closure::new(function));
        self.frames.push(CallFrame::new(closure, 0));
        self.run()
    }

    pub fn dump(&mut self, code: &str, file: &str) -> Result<(), InterpretError> {
        let function = compiler::compile(code, &mut self.gc)?;
        let function = self.gc.deref(function);
        let name = &self.gc.deref(function.name);
        let disassembler = Disassembler::new(&self.gc, &function.chunk, Some(&self.stack));
        let mut content = vec![disassembler.disassemble_to_string(name)];
        for gcref in self.gc.objects.iter().rev().flatten() {
            if let Some(fun) = gcref.object.as_any().downcast_ref::<Function>() {
                if fun.name != function.name {
                    let name = &self.gc.deref(fun.name);
                    let disassembler = Disassembler::new(&self.gc, &fun.chunk, Some(&self.stack));
                    content.push(disassembler.disassemble_to_string(name));
                }
            }
        }
        fs::write(file, content.join("")).expect("Couldn't write to file.");
        Ok(())
    }

    #[inline]
    pub fn set_debug(&mut self) {
        self.debug = true;
    }

    #[inline]
    pub fn unset_debug(&mut self) {
        self.debug = false;
    }

    // stack manipulation

    fn pop(&mut self) -> Value {
        if let Some(value) = self.stack.pop() {
            value
        } else {
            eprintln!("Error: popping a value from empty stack.");
            process::exit(65);
        }
    }

    fn pop_number(&mut self, msg: &str) -> Result<f64, InterpretError> {
        if let Value::Number(n) = self.pop() {
            Ok(n)
        } else {
            Err(self.runtime_error(&format!("Error: no number found on stack {}.", msg)))
        }
    }

    fn push(&mut self, value: Value) -> Result<(), InterpretError> {
        if self.stack.capacity() == isize::MAX as usize {
            Err(self.runtime_error("Stack full."))
        } else {
            self.stack.push(value);
            Ok(())
        }
    }

    fn push_number(&mut self, n: f64) -> Result<(), InterpretError> {
        self.push(Value::Number(n))
    }

    #[inline]
    fn peek(&self, index: usize) -> Value {
        self.stack[self.stack.len() - 1 - index]
    }

    // main function

    fn run(&mut self) -> Result<(), InterpretError> {
        loop {
            let instruction = self.current_chunk().get_opcode(self.current_frame().ip);
            if self.debug || cfg!(feature = "debug_trace_execution") {
                let disassembler =
                    Disassembler::new(&self.gc, self.current_chunk(), Some(&self.stack));
                disassembler.disassemble_instruction(&instruction, self.current_frame().ip);
                println!();
            }

            self.current_frame_mut().ip += 1;

            match instruction {
                OpCode::Add => match (self.pop(), self.pop()) {
                    (Value::Number(b), Value::Number(a)) => self.push(Value::Number(a + b))?,
                    (Value::VString(b), Value::VString(a)) => {
                        let a = self.gc.deref(a);
                        let b = self.gc.deref(b);
                        let new = format!("{}{}", a, b);
                        let string = self.intern(new);
                        self.push(Value::VString(string))?
                    }
                    (Value::VString(b), Value::Number(a)) => {
                        let b = self.gc.deref(b);
                        let new = format!("{}{}", a, b);
                        let string = self.intern(new);
                        self.push(Value::VString(string))?
                    }
                    (Value::Number(b), Value::VString(a)) => {
                        let a = self.gc.deref(a);
                        let new = format!("{}{}", a, b);
                        let string = self.intern(new);
                        self.push(Value::VString(string))?
                    }
                    (Value::Array(b), Value::Array(a)) => {
                        let a = self.gc.deref(a);
                        let b = self.gc.deref(b);
                        let mut c = a.clone();
                        c.extend(b);
                        let c = self.alloc(c);
                        self.push(Value::Array(c))?
                    }
                    _ => {
                        return Err(self.runtime_error(
                            "Arguments must be both numbers or at least one string.",
                        ))
                    }
                },
                OpCode::BuildList(length) => {
                    let mut vec: Vec<Value> = Vec::new();
                    for _ in 0..length {
                        vec.push(self.pop());
                    }
                    vec.reverse();
                    let vec = self.gc.alloc(vec);
                    self.push(Value::Array(vec))?
                }
                OpCode::Call(arg_count) => self.call_value(self.peek(arg_count), arg_count)?,
                OpCode::Class(value) => {
                    if let Value::VString(name) = self.current_chunk().constants[value] {
                        let class = Class::new(name);
                        let class = self.alloc(class);
                        self.push(Value::Class(class))?
                    } else {
                        return Err(self
                            .runtime_error("Error: Invalid identifier found for usage on stack."));
                    }
                }
                OpCode::CloseUpvalue => {
                    self.close_upvalue(self.stack.len() - 1);
                    self.pop();
                }
                OpCode::Closure(index) => match self.current_chunk().get_constant(index) {
                    Value::Function(function) => {
                        let upvalue_count = self.gc.deref(function).upvalues.len();
                        let mut closure = Closure::new(function);

                        for i in 0..upvalue_count {
                            let upvalue = self.gc.deref(function).upvalues[i];
                            let value = if upvalue.is_local {
                                self.capture_upvalue(self.current_frame().slot + upvalue.index)
                            } else {
                                self.current_closure().upvalues[upvalue.index]
                            };
                            closure.upvalues.push(value);
                        }
                        let closure = self.alloc(closure);
                        self.push(Value::Closure(closure))?
                    }
                    _ => return Err(self.runtime_error("Error: no function found.")),
                },
                OpCode::Constant(index) => self.push(self.current_chunk().get_constant(index))?,
                OpCode::DecrementGlobal(index) => {
                    if let Value::VString(string_ref) = self.current_chunk().get_constant(index) {
                        match self.globals.get(&string_ref) {
                            Some(&value) => {
                                if let Value::Number(v) = value {
                                    let val = Value::Number(v - 1.0);
                                    self.push(val)?;
                                    if self.globals.insert(string_ref, val).is_none() {
                                        self.globals.remove(&string_ref);
                                        return Err(self.runtime_error(&format!(
                                            "Undefined variable '{}'.",
                                            self.gc.deref(string_ref)
                                        )));
                                    }
                                } else {
                                    return Err(self.runtime_error(
                                        "Only numeric variables can be incremented.",
                                    ));
                                }
                            }
                            None => {
                                return Err(self.runtime_error(&format!(
                                    "Undefined variable '{}'.",
                                    self.gc.deref(string_ref)
                                )))
                            }
                        }
                    } else {
                        return Err(self
                            .runtime_error("Error: Invalid identifier found for usage on stack."));
                    }
                }
                OpCode::DecrementLocal(slot) => {
                    let index = slot + self.current_frame().slot;
                    if let Value::Number(value) = self.stack[index] {
                        let value = Value::Number(value - 1.0);
                        self.stack[index] = value;
                        self.push(value)?;
                    } else {
                        return Err(self.runtime_error("Only number can be incremented."));
                    }
                }
                OpCode::DecrementUpvalue(slot) => {
                    let upvalue = self.current_closure().upvalues[slot];
                    let value = {
                        let upvalue = self.gc.deref(upvalue);
                        let temp = if let Some(value) = upvalue.closed {
                            value
                        } else {
                            self.stack[upvalue.location]
                        };

                        if let Value::Number(val) = temp {
                            Value::Number(val - 1.0)
                        } else {
                            return Err(self.runtime_error("Only numbers can be incremented."));
                        }
                    };
                    let mut upvalue = self.gc.deref_mut(upvalue);
                    if upvalue.closed.is_none() {
                        self.stack[upvalue.location] = value;
                    } else {
                        upvalue.closed = Some(value);
                    }
                    self.push(value)?;
                }
                OpCode::DefineGlobal(index) => {
                    if let Value::VString(string_ref) = self.current_chunk().constants[index] {
                        self.globals.insert(string_ref, self.peek(0));
                        self.pop();
                    } else {
                        return Err(self.runtime_error(
                            "Error: Invalid identifier found for definition on stack.",
                        ));
                    }
                }
                OpCode::Div => self.bin_arith_op(|x, y| x / y, "when dividing")?,
                OpCode::Equal => self.bin_bool_op(|x, y| x == y)?,
                OpCode::False => {
                    self.push(Value::Bool(false))?;
                }
                OpCode::GetIndexArray => {
                    let index = self.pop_number("for indexing an array.")?;
                    if index.fract() != 0.0 {
                        return Err(
                            self.runtime_error("Can't index an array with a fractional number.")
                        );
                    }
                    if let Value::Array(array) = self.pop() {
                        let array = self.gc.deref(array);
                        let value = array[index as usize];
                        self.push(value)?
                    } else {
                        return Err(
                            self.runtime_error("No array found on stack when indexing an array.")
                        );
                    }
                }
                OpCode::GetGlobal(index) => {
                    if let Value::VString(string_ref) = self.current_chunk().get_constant(index) {
                        match self.globals.get(&string_ref) {
                            Some(&value) => self.push(value)?,
                            None => {
                                return Err(self.runtime_error(&format!(
                                    "Undefined variable '{}'.",
                                    self.gc.deref(string_ref)
                                )))
                            }
                        }
                    } else {
                        return Err(self
                            .runtime_error("Error: Invalid identifier found for usage on stack."));
                    }
                }
                OpCode::GetLocal(slot) => {
                    self.push(self.stack[slot + self.current_frame().slot])?;
                }
                OpCode::GetProperty(slot) => {
                    if let Value::Instance(instance) = self.peek(0) {
                        let instance = self.gc.deref(instance);
                        if let Value::VString(name) = self.current_chunk().get_constant(slot) {
                            let value = instance.fields.get(&name);
                            if let Some(&value) = value {
                                self.pop();
                                self.push(value)?
                            } else {
                                let class = instance.class;
                                self.bind_method(class, name)?;
                            }
                        } else {
                            return Err(self.runtime_error(
                                "Error: Invalid identifier found for usage on stack.",
                            ));
                        }
                    } else {
                        return Err(self.runtime_error("Only instances have properties."));
                    }
                }
                OpCode::GetSuper(slot) => {
                    if let Value::VString(name) = self.current_chunk().get_constant(slot) {
                        if let Value::Class(superclass) = self.pop() {
                            self.bind_method(superclass, name)?
                        } else {
                            return Err(self.runtime_error("No superclass found on the stack"));
                        }
                    } else {
                        return Err(self
                            .runtime_error("Error: Invalid identifier found for usage on stack."));
                    }
                }
                OpCode::GetUpvalue(slot) => {
                    let value = {
                        let upvalue = self.current_closure().upvalues[slot];
                        let upvalue = self.gc.deref(upvalue);
                        if let Some(value) = upvalue.closed {
                            value
                        } else {
                            self.stack[upvalue.location]
                        }
                    };
                    self.push(value)?
                }
                OpCode::Greater => match (self.pop(), self.pop()) {
                    (Value::Number(b), Value::Number(a)) => self.push(Value::Bool(a > b))?,
                    (Value::VString(b), Value::VString(a)) => {
                        let a = self.gc.deref(a);
                        let b = self.gc.deref(b);
                        let result = Value::Bool(a > b);
                        self.push(result)?
                    }
                    _ => {
                        return Err(
                            self.runtime_error("Arguments must be of same type and comparable.")
                        )
                    }
                },
                OpCode::GreaterEqual => match (self.pop(), self.pop()) {
                    (Value::Number(b), Value::Number(a)) => self.push(Value::Bool(a >= b))?,
                    (Value::VString(b), Value::VString(a)) => {
                        let a = self.gc.deref(a);
                        let b = self.gc.deref(b);
                        let result = Value::Bool(a >= b);
                        self.push(result)?
                    }
                    _ => {
                        return Err(
                            self.runtime_error("Arguments must be of same type and comparable.")
                        )
                    }
                },
                OpCode::IncrementGlobal(index) => {
                    if let Value::VString(string_ref) = self.current_chunk().get_constant(index) {
                        match self.globals.get(&string_ref) {
                            Some(&value) => {
                                if let Value::Number(v) = value {
                                    let val = Value::Number(v + 1.0);
                                    self.push(val)?;
                                    if self.globals.insert(string_ref, val).is_none() {
                                        self.globals.remove(&string_ref);
                                        return Err(self.runtime_error(&format!(
                                            "Undefined variable '{}'.",
                                            self.gc.deref(string_ref)
                                        )));
                                    }
                                } else {
                                    return Err(self.runtime_error(
                                        "Only numeric variables can be incremented.",
                                    ));
                                }
                            }
                            None => {
                                return Err(self.runtime_error(&format!(
                                    "Undefined variable '{}'.",
                                    self.gc.deref(string_ref)
                                )))
                            }
                        }
                    } else {
                        return Err(self
                            .runtime_error("Error: Invalid identifier found for usage on stack."));
                    }
                }
                OpCode::IncrementLocal(slot) => {
                    let index = slot + self.current_frame().slot;
                    if let Value::Number(value) = self.stack[index] {
                        let value = Value::Number(value + 1.0);
                        self.stack[index] = value;
                        self.push(value)?;
                    } else {
                        return Err(self.runtime_error("Only number can be incremented."));
                    }
                }
                OpCode::IncrementUpvalue(slot) => {
                    let upvalue = self.current_closure().upvalues[slot];
                    let value = {
                        let upvalue = self.gc.deref(upvalue);
                        let temp = if let Some(value) = upvalue.closed {
                            value
                        } else {
                            self.stack[upvalue.location]
                        };

                        if let Value::Number(val) = temp {
                            Value::Number(val + 1.0)
                        } else {
                            return Err(self.runtime_error("Only numbers can be incremented."));
                        }
                    };
                    let mut upvalue = self.gc.deref_mut(upvalue);
                    if upvalue.closed.is_none() {
                        self.stack[upvalue.location] = value;
                    } else {
                        upvalue.closed = Some(value);
                    }
                    self.push(value)?;
                }
                OpCode::Inherit => {
                    let pair = (self.peek(0), self.peek(1));
                    if let (Value::Class(class), Value::Class(superclass)) = pair {
                        let superclass = self.gc.deref(superclass);
                        let methods = superclass.methods.clone();
                        let class = self.gc.deref_mut(class);
                        class.methods = methods;
                        self.pop();
                    } else {
                        return Err(self.runtime_error("Superclass must be a class."));
                    }
                }
                OpCode::Invoke((name, count)) => {
                    if let Value::VString(name) = self.current_chunk().get_constant(name) {
                        self.invoke(name, count)?
                    } else {
                        return Err(self
                            .runtime_error("Error: Invalid identifier found for usage on stack."));
                    }
                }
                OpCode::Jump(offset) => {
                    self.current_frame_mut().ip += offset;
                }
                OpCode::JumpIfFalse(offset) => {
                    if self.peek(0).is_false() {
                        self.current_frame_mut().ip += offset;
                    }
                }
                OpCode::Less => match (self.pop(), self.pop()) {
                    (Value::Number(b), Value::Number(a)) => self.push(Value::Bool(a < b))?,
                    (Value::VString(b), Value::VString(a)) => {
                        let a = self.gc.deref(a);
                        let b = self.gc.deref(b);
                        let result = Value::Bool(a < b);
                        self.push(result)?
                    }
                    _ => {
                        return Err(
                            self.runtime_error("Arguments must be of same type and comparable.")
                        )
                    }
                },
                OpCode::LessEqual => match (self.pop(), self.pop()) {
                    (Value::Number(b), Value::Number(a)) => self.push(Value::Bool(a <= b))?,
                    (Value::VString(b), Value::VString(a)) => {
                        let a = self.gc.deref(a);
                        let b = self.gc.deref(b);
                        let result = Value::Bool(a <= b);
                        self.push(result)?
                    }
                    _ => {
                        return Err(
                            self.runtime_error("Arguments must be of same type and comparable.")
                        )
                    }
                },
                OpCode::Loop(offset) => {
                    self.current_frame_mut().ip -= offset + 1;
                }
                OpCode::Method(slot) => {
                    if let Value::VString(name) = self.current_chunk().get_constant(slot) {
                        self.define_method(name)?
                    } else {
                        return Err(self
                            .runtime_error("Error: Invalid identifier found for usage on stack."));
                    }
                }
                OpCode::Rem => {
                    let (b, a) = (
                        self.pop_number("as divisor in rem")?,
                        self.pop_number("as dividend in rem")?,
                    );
                    if b.fract() == 0.0 && a.fract() == 0.0 {
                        let a = a as usize;
                        let b = b as usize;
                        let rem = a % b;
                        self.push(Value::Number(rem as f64))?
                    }
                }
                OpCode::Mul => self.bin_arith_op(|x, y| x * y, "when multiplying")?,
                OpCode::Negate => {
                    let n = self.pop_number("to negate")?;
                    self.push(Value::Number(-n))?
                }
                OpCode::Nil => self.push(Value::Nil)?,
                OpCode::Not => {
                    let value = self.pop().is_false();
                    self.push(Value::Bool(value))?
                }
                OpCode::NotEqual => self.bin_bool_op(|x, y| x != y)?,
                OpCode::Pop => {
                    self.pop();
                }
                OpCode::Print => {
                    let value = self.pop();
                    if self.repl {
                        println!(">  {}", GcTraceFormatter::new(value, &self.gc));
                    } else {
                        println!("{}", GcTraceFormatter::new(value, &self.gc));
                    }
                }
                OpCode::Return => {
                    let frame = self.frames.pop().unwrap();
                    let result = self.pop();
                    self.close_upvalue(frame.slot);
                    if self.frames.is_empty() {
                        return Ok(());
                    } else {
                        self.stack.truncate(frame.slot);
                        self.push(result)?
                    }
                }
                OpCode::ReturnNil => {
                    let frame = self.frames.pop().unwrap();
                    self.close_upvalue(frame.slot);
                    if self.frames.is_empty() {
                        return Ok(());
                    } else {
                        self.stack.truncate(frame.slot);
                        self.push(Value::Nil)?
                    }
                }
                OpCode::SetIndexArray => {
                    let value = self.pop();
                    let index = self.pop_number("for indexing an array")?;
                    if let Value::Array(arrayref) = self.pop() {
                        let array = self.gc.deref_mut(arrayref);
                        array[index as usize] = value;
                        self.push(Value::Array(arrayref))?
                    } else {
                        return Err(self.runtime_error("No array found on stack when indexing."));
                    }
                }
                OpCode::SetGlobal(index) => {
                    if let Value::VString(string_ref) = self.current_chunk().constants[index] {
                        if self.globals.insert(string_ref, self.peek(0)).is_none() {
                            self.globals.remove(&string_ref);
                            return Err(self.runtime_error(&format!(
                                "Undefined variable '{}'.",
                                self.gc.deref(string_ref)
                            )));
                        }
                    } else {
                        return Err(self
                            .runtime_error("Error: Invalid identifier found for usage on stack."));
                    }
                }
                OpCode::SetLocal(slot) => {
                    let index = slot + self.current_frame().slot;
                    self.stack[index] = self.peek(0);
                }
                OpCode::SetProperty(slot) => {
                    if let Value::Instance(instance) = self.peek(1) {
                        if let Value::VString(name) = self.current_chunk().get_constant(slot) {
                            let value = self.pop();
                            let instance = self.gc.deref_mut(instance);
                            instance.fields.insert(name, value);
                            self.pop();
                            self.push(value)?
                        } else {
                            return Err(self.runtime_error(
                                "Error: Invalid identifier found for usage on stack.",
                            ));
                        }
                    } else {
                        return Err(self.runtime_error("Only instances have fields."));
                    }
                }
                OpCode::SetUpvalue(slot) => {
                    let upvalue = self.current_closure().upvalues[slot];
                    let value = self.peek(0);
                    let mut upvalue = self.gc.deref_mut(upvalue);
                    if upvalue.closed.is_none() {
                        self.stack[upvalue.location] = value;
                    } else {
                        upvalue.closed = Some(value);
                    }
                }
                OpCode::Sub => self.bin_arith_op(|x, y| x - y, "when subtracting")?,
                OpCode::SuperInvoke((name, count)) => {
                    if let Value::VString(name) = self.current_chunk().get_constant(name) {
                        if let Value::Class(class) = self.pop() {
                            self.invoke_from_class(class, name, count)?
                        } else {
                            return Err(self.runtime_error("No class found on the stack."));
                        }
                    } else {
                        return Err(self
                            .runtime_error("Error: Invalid identifier found for usage on stack."));
                    }
                }
                OpCode::True => self.push(Value::Bool(true))?,
            }
        }
    }

    // helpers for binary operations

    fn bin_arith_op(&mut self, f: fn(f64, f64) -> f64, msg: &str) -> Result<(), InterpretError> {
        let (b, a) = (
            self.pop_number(&format!("as second term {}", msg))?,
            self.pop_number(&format!("as second term {}", msg))?,
        );
        self.push_number(f(a, b))
    }

    fn bin_bool_op(&mut self, f: fn(Value, Value) -> bool) -> Result<(), InterpretError> {
        let (b, a) = (self.pop(), self.pop());
        self.push(Value::Bool(f(a, b)))
    }

    // error functions

    fn runtime_error(&mut self, message: &str) -> InterpretError {
        eprintln!("{}", message);

        for frame in self.frames.iter().rev() {
            let closure = self.gc.deref(frame.closure);
            let function = self.gc.deref(closure.function);
            let name = self.gc.deref(function.name);
            let name = if name.is_empty() { "<script>" } else { &name };
            let line = function.chunk.get_line(frame.ip - 1);
            eprintln!("[line {}] in {}", line, name);
        }

        self.stack.clear();
        InterpretError::Runtime
    }

    // current pointers

    #[inline]
    fn current_frame(&self) -> &CallFrame {
        self.frames.last().unwrap()
    }

    #[inline]
    fn current_closure(&self) -> &Closure {
        let closure = self.current_frame().closure;
        self.gc.deref(closure)
    }

    #[inline]
    fn current_frame_mut(&mut self) -> &mut CallFrame {
        self.frames.last_mut().unwrap()
    }

    #[inline]
    fn current_chunk(&self) -> &Chunk {
        let function = self.gc.deref(self.current_closure().function);
        &function.chunk
    }

    // helpers for calling a function

    fn call_value(&mut self, callee: Value, arg_count: usize) -> Result<(), InterpretError> {
        match callee {
            Value::NativeFn(fun) => {
                let left = self.stack.len() - arg_count;
                let result = match fun.0(self, &self.stack[left..]) {
                    Ok(res) => res,
                    Err(e) => return Err(self.runtime_error(&e)),
                };
                self.stack.truncate(left - 1);
                self.push(result)
            }
            Value::Closure(fun) => self.call(fun, arg_count),
            Value::Class(cls) => {
                let instance = Instance::new(cls);
                let instance = self.alloc(instance);
                let index = self.stack.len() - arg_count - 1;
                self.stack[index] = Value::Instance(instance);

                match self.gc.deref(cls).methods.get(&self.init_string) {
                    Some(&method) => {
                        if let Value::Closure(method) = method {
                            self.call(method, arg_count)
                        } else {
                            Err(self.runtime_error("Initializer is not closure"))
                        }
                    }
                    None => {
                        if arg_count != 0 {
                            let msg = format!("Expected 0 arguments but got {}.", arg_count);
                            Err(self.runtime_error(&msg))
                        } else {
                            Ok(())
                        }
                    }
                }
            }
            Value::BoundMethod(met) => {
                let bound_method = self.gc.deref(met);
                let method = bound_method.method;
                let receiver = bound_method.receiver;
                let index = self.stack.len() - 1 - arg_count;
                self.stack[index] = receiver;
                self.call(method, arg_count)
            }
            _ => Err(self.runtime_error("Can only call functions and classes.")),
        }
    }

    fn call(&mut self, callee: GcRef<Closure>, arg_count: usize) -> Result<(), InterpretError> {
        let closure = self.gc.deref(callee);
        let function = self.gc.deref(closure.function);
        if function.arity != arg_count {
            let msg = format!(
                "Expected {} arguments but got {}.",
                function.arity, arg_count
            );
            Err(self.runtime_error(&msg))
        } else {
            let frame = CallFrame::new(callee, self.stack.len() - arg_count - 1);
            self.frames.push(frame);
            Ok(())
        }
    }

    #[inline]
    fn define_native(&mut self, name: &str, function: NativeFn) {
        let name = self.intern(name.to_owned());
        self.globals.insert(name, Value::NativeFn(function));
    }

    fn capture_upvalue(&mut self, index: usize) -> GcRef<Upvalue> {
        for &upvalue in &self.open_upvalues {
            if self.gc.deref(upvalue).location == index {
                return upvalue;
            }
        }
        let upvalue = Upvalue::new(index);
        let upvalue = self.alloc(upvalue);
        self.open_upvalues.push(upvalue);
        upvalue
    }

    fn close_upvalue(&mut self, last: usize) {
        let mut i = 0;
        while i != self.open_upvalues.len() {
            let upvalue = self.open_upvalues[i];
            let upvalue = self.gc.deref_mut(upvalue);
            if upvalue.location >= last {
                self.open_upvalues.remove(i);
                upvalue.closed = Some(self.stack[upvalue.location]);
            } else {
                i += 1;
            }
        }
    }

    fn define_method(&mut self, name: GcRef<String>) -> Result<(), InterpretError> {
        let method = self.peek(0);
        if let Value::Class(class) = self.peek(1) {
            let class = self.gc.deref_mut(class);
            class.methods.insert(name, method);
            self.pop();
            Ok(())
        } else {
            Err(self.runtime_error("Cannot define a method on non class."))
        }
    }

    fn bind_method(
        &mut self,
        class: GcRef<Class>,
        name: GcRef<String>,
    ) -> Result<(), InterpretError> {
        let class = self.gc.deref(class);
        if let Some(method) = class.methods.get(&name) {
            let receiver = self.peek(0);
            let method = match method {
                Value::Closure(cl) => cl,
                _ => return Err(self.runtime_error("No method found")),
            };
            let bound = BoundMethod::new(receiver, *method);
            let bound = self.alloc(bound);
            self.pop();
            self.push(Value::BoundMethod(bound))
        } else {
            let name = &self.gc.deref(name);
            let message = format!("Undefined property '{}'.", name);
            Err(self.runtime_error(&message))
        }
    }

    fn invoke(&mut self, name: GcRef<String>, arg_count: usize) -> Result<(), InterpretError> {
        let receiver = self.peek(arg_count);
        let method_name = self.gc.deref(name).clone();
        if method_name == "copy" {
            if arg_count != 0 {
                Err(self.runtime_error("Copy requires only one argument."))
            } else {
                let to_push = match receiver {
                    Value::Array(value) => {
                        let new = self.gc.deref(value).clone();
                        let new = self.alloc(new);
                        Value::Array(new)
                    }
                    Value::Instance(value) => {
                        let new = self.gc.deref(value).clone();
                        let new = self.alloc(new);
                        Value::Instance(new)
                    }
                    _ => {
                        return Err(self.runtime_error(&format!(
                            "Function copy is not defined for {}",
                            receiver.type_of()
                        )))
                    }
                };
                self.pop();
                self.push(to_push)
            }
        } else if let Value::VString(string) = receiver {
            match &*method_name {
                "isAlpha" => {
                    if arg_count != 0 {
                        Err(self.runtime_error("isAlpha requires no arguments."))
                    } else {
                        self.pop();
                        self.push(Value::Bool(
                            self.gc.deref(string).chars().all(char::is_alphabetic),
                        ))
                    }
                }
                "isAlphaNumeric" => {
                    if arg_count != 0 {
                        Err(self.runtime_error("isAlphaNumeric requires no arguments."))
                    } else {
                        self.pop();
                        self.push(Value::Bool(
                            self.gc.deref(string).chars().all(char::is_alphanumeric),
                        ))
                    }
                }
                "isDigit" => {
                    if arg_count != 0 {
                        Err(self.runtime_error("isDigit requires no arguments."))
                    } else {
                        self.pop();
                        self.push(Value::Bool(
                            self.gc.deref(string).chars().all(char::is_numeric),
                        ))
                    }
                }

                "float" => {
                    if arg_count == 1 {
                        let top = self.pop();
                        if let Value::VString(string) = top {
                            match self.gc.deref(string).parse() {
                                Ok(n) => {
                                    self.pop();
                                    self.push_number(n)
                                }
                                _ => Err(self.runtime_error("couldn't read number from string")),
                            }
                        } else {
                            Err(self.runtime_error(&format!(
                                "float needs a number as an argument, found {}",
                                top.type_of()
                            )))
                        }
                    } else {
                        Err(self.runtime_error("float needs one argument"))
                    }
                }
                "length" => {
                    if arg_count != 0 {
                        Err(self.runtime_error("length requires no arguments."))
                    } else {
                        self.pop();
                        self.push_number(self.gc.deref(string).len() as f64)
                    }
                }
                "ord" => {
                    if arg_count != 0 {
                        Err(self.runtime_error("ord requires no arguments."))
                    } else if self.gc.deref(string).chars().count() == 1 {
                        let c = self.gc.deref(string).chars().next().unwrap();
                        self.push_number((c as u32) as f64)
                    } else {
                        Err(self.runtime_error("ord can be called on one-char strings only."))
                    }
                }
                _ => {
                    Err(self
                        .runtime_error(&format!("String doesn't have {} as method.", method_name)))
                }
            }
        } else if let Value::Instance(instance) = receiver {
            let instance = self.gc.deref(instance);
            if let Some(&value) = instance.fields.get(&name) {
                let pos = self.stack.len() - 1 - arg_count;
                self.stack[pos] = value;
                self.call_value(value, arg_count)
            } else {
                let class = instance.class;
                self.invoke_from_class(class, name, arg_count)
            }
        } else if let Value::Array(array) = receiver {
            match &*method_name {
                "all" => {
                    if arg_count != 0 {
                        Err(self.runtime_error("all requires no arguments."))
                    } else {
                        self.pop();
                        self.push(Value::Bool(
                            !self.gc.deref(array).iter().any(|&x| x.is_false()),
                        ))
                    }
                }
                "any" => {
                    if arg_count != 0 {
                        Err(self.runtime_error("any requires no arguments."))
                    } else {
                        self.pop();
                        self.push(Value::Bool(
                            self.gc.deref(array).iter().any(|&x| !x.is_false()),
                        ))
                    }
                }
                "extend" => {
                    if arg_count != 1 {
                        Err(self.runtime_error("extend requires only one argument."))
                    } else if let Value::Array(array_ref) = self.pop() {
                        let mut new_array = self.gc.deref(array_ref).clone();
                        self.gc.deref_mut(array).append(&mut new_array);
                        self.pop();
                        self.push(Value::Nil)
                    } else {
                        Err(self.runtime_error("extend needs an array as argument"))
                    }
                }
                "length" => {
                    if arg_count != 0 {
                        Err(self.runtime_error("length requires no arguments."))
                    } else {
                        self.pop();
                        self.push_number(self.gc.deref(array).len() as f64)
                    }
                }
                "pop" => {
                    if arg_count != 0 {
                        Err(self.runtime_error("pop requires no arguments."))
                    } else if let Some(value) = self.gc.deref_mut(array).pop() {
                        self.pop();
                        self.push(value)
                    } else {
                        Err(self.runtime_error("No element in array when popping from it."))
                    }
                }
                "push" => {
                    if arg_count == 0 {
                        Err(self.runtime_error("No arguments given to function push."))
                    } else {
                        let mut temp = Vec::new();
                        for _ in 0..arg_count {
                            temp.push(self.pop());
                        }
                        temp.reverse();
                        self.gc.deref_mut(array).append(&mut temp);
                        self.pop();
                        self.push(Value::Nil)
                    }
                }
                "reverse" => {
                    if arg_count != 0 {
                        Err(self.runtime_error("reverse requires only one argument."))
                    } else {
                        self.gc.deref_mut(array).reverse();
                        self.pop();
                        self.push(Value::Nil)
                    }
                }
                "sort" => {
                    let array = self.gc.deref_mut(array);
                    if array.iter().all(|&x| matches!(x, Value::Number(_))) {
                        array.sort_by(|a, b| {
                            if let (Value::Number(a), Value::Number(b)) = (a, b) {
                                a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Less)
                            } else {
                                panic!();
                            }
                        });
                        self.pop();
                        self.push(Value::Nil)
                    } else {
                        Err(self.runtime_error("Cannot sort an array with not-number elements"))
                    }
                }
                _ => {
                    Err(self
                        .runtime_error(&format!("Array doesn't have {} as method.", method_name)))
                }
            }
        } else if let Value::Number(n) = receiver {
            match &*method_name {
                "abs" => match arg_count {
                    0 => {
                        if let Value::Number(n) = self.pop() {
                            self.push_number(n.abs())
                        } else {
                            Err(self.runtime_error("abs needs numeric argument."))
                        }
                    }
                    _ => Err(self.runtime_error("abs expects only one argument.")),
                },
                "ceil" => match arg_count {
                    0 => {
                        if let Value::Number(n) = self.pop() {
                            self.push_number(n.ceil())
                        } else {
                            Err(self.runtime_error("ceil needs numeric argument."))
                        }
                    }
                    _ => Err(self.runtime_error("ceil needs one argument.")),
                },
                "chr" => {
                    if arg_count != 0 {
                        Err(self.runtime_error("chr requires no arguments."))
                    } else {
                        self.pop();
                        let n = if n.fract() == 0.0 {
                            n as u32
                        } else {
                            return Err(self.runtime_error("chr needs an integer argument."));
                        };
                        let s = match char::from_u32(n) {
                            Some(c) => self.intern(c.to_string()),
                            None => {
                                return Err(self.runtime_error("chr couldn't read number to char"))
                            }
                        };
                        self.push(Value::VString(s))
                    }
                }
                "floor" => {
                    if arg_count == 0 {
                        self.pop();
                        self.push_number(n.floor())
                    } else {
                        Err(self.runtime_error("floor needs one argument."))
                    }
                }
                "pow" => {
                    if arg_count == 1 {
                        if let Value::Number(n1) = self.pop() {
                            self.push_number(n.powf(n1))
                        } else {
                            Err(self.runtime_error("sqrt needs numeric argument"))
                        }
                    } else {
                        Err(self.runtime_error("sqrt expects only one argument"))
                    }
                }
                "sqrt" => {
                    if arg_count == 0 {
                        self.pop();
                        self.push_number(n.sqrt())
                    } else {
                        Err(self.runtime_error("sqrt expects only one argument"))
                    }
                }
                "square" => {
                    if arg_count == 0 {
                        self.pop();
                        self.push_number(n * n)
                    } else {
                        Err(self.runtime_error("square expects only one argument"))
                    }
                }
                _ => {
                    Err(self
                        .runtime_error(&format!("Float doesn't have {} as method.", method_name)))
                }
            }
        } else if method_name == "toString" {
            if arg_count != 0 {
                Err(self.runtime_error("toString requires no arguments"))
            } else {
                let string = format!("{}", GcTraceFormatter::new(receiver, &self.gc));
                let string = self.alloc(string);
                self.pop();
                self.push(Value::VString(string))
            }
        } else {
            Err(self.runtime_error("Only instances have methods."))
        }
    }

    fn invoke_from_class(
        &mut self,
        class: GcRef<Class>,
        name: GcRef<String>,
        count: usize,
    ) -> Result<(), InterpretError> {
        let class = self.gc.deref(class);
        if let Some(&method) = class.methods.get(&name) {
            if let Value::Closure(closure) = method {
                self.call(closure, count)
            } else {
                Err(self.runtime_error("Got method that is not closure!"))
            }
        } else if self.gc.deref(name) == "toString" {
            if count != 0 {
                Err(self.runtime_error("toString requires no arguments"))
            } else {
                let name = class.name;
                self.pop();
                self.push(Value::VString(name))
            }
        } else {
            let name = &self.gc.deref(name);
            let message = format!("Undefined property '{}'.", name);
            Err(self.runtime_error(&message))
        }
    }

    // garbage collection helpers

    fn collect_garbage(&mut self) {
        if self.gc.should_gc() {
            #[cfg(feature = "debug_gc_log")]
            eprintln!("\n-- gc start");
            self.mark_roots();
            self.gc.collect_garbage();
            #[cfg(feature = "debug_gc_log")]
            eprintln!("-- gc end\n");
        }
    }

    #[inline]
    fn alloc<T: GcTrace + 'static + fmt::Debug>(&mut self, object: T) -> GcRef<T> {
        self.collect_garbage();
        self.gc.alloc(object)
    }

    #[inline]
    fn intern(&mut self, string: String) -> GcRef<String> {
        self.collect_garbage();
        self.gc.intern(string)
    }

    fn mark_roots(&mut self) {
        for &value in &self.stack {
            self.gc.mark_value(value);
        }

        for frame in &self.frames {
            self.gc.mark_object(frame.closure);
        }

        for &upvalue in &self.open_upvalues {
            self.gc.mark_object(upvalue);
        }

        self.gc.mark_table(&self.globals);
        self.gc.mark_object(self.init_string);
    }
}

struct CallFrame {
    closure: GcRef<Closure>,
    ip: usize,
    slot: usize,
}

impl CallFrame {
    fn new(closure: GcRef<Closure>, slot: usize) -> Self {
        CallFrame {
            closure,
            ip: 0,
            slot,
        }
    }
}

// native functions

fn clock(vm: &Vm, _args: &[Value]) -> Result<Value, String> {
    let time = vm.start_time.elapsed().as_secs_f64();
    Ok(Value::Number(time))
}

fn instance_of(vm: &Vm, args: &[Value]) -> Result<Value, String> {
    match args.len() {
        2 => {
            if let (Value::Instance(instance), Value::Class(class)) = (args[0], args[1]) {
                let class_ref = vm.gc.deref(instance).class;
                Ok(Value::Bool(class_ref == class))
            } else {
                Err(format!(
                    "instanceof needs an instance and a class, found {} {}",
                    args[0].type_of(),
                    args[1].type_of()
                ))
            }
        }
        _ => Err("instanceof needs two arguments".to_owned()),
    }
}

fn is_bool(_vm: &Vm, args: &[Value]) -> Result<Value, String> {
    match args.len() {
        1 => {
            if let Value::Bool(_) = args[0] {
                Ok(Value::Bool(true))
            } else {
                Ok(Value::Bool(false))
            }
        }
        _ => Err("isBool needs one argument".to_owned()),
    }
}

fn is_class(_vm: &Vm, args: &[Value]) -> Result<Value, String> {
    match args.len() {
        1 => {
            if let Value::Class(_) = args[0] {
                Ok(Value::Bool(true))
            } else {
                Ok(Value::Bool(false))
            }
        }
        _ => Err("isClass needs one argument".to_owned()),
    }
}

fn is_closure(_vm: &Vm, args: &[Value]) -> Result<Value, String> {
    match args.len() {
        1 => {
            if let Value::Closure(_) = args[0] {
                Ok(Value::Bool(true))
            } else {
                Ok(Value::Bool(false))
            }
        }
        _ => Err("isClosure needs one argument".to_owned()),
    }
}

fn is_function(_vm: &Vm, args: &[Value]) -> Result<Value, String> {
    match args.len() {
        1 => {
            if let Value::Function(_) = args[0] {
                Ok(Value::Bool(true))
            } else {
                Ok(Value::Bool(false))
            }
        }
        _ => Err("isFunction needs one argument".to_owned()),
    }
}

fn is_instance(_vm: &Vm, args: &[Value]) -> Result<Value, String> {
    match args.len() {
        1 => {
            if let Value::Instance(_) = args[0] {
                Ok(Value::Bool(true))
            } else {
                Ok(Value::Bool(false))
            }
        }
        _ => Err("isInstance needs one argument".to_owned()),
    }
}

fn is_nil(_vm: &Vm, args: &[Value]) -> Result<Value, String> {
    match args.len() {
        1 => {
            if let Value::Nil = args[0] {
                Ok(Value::Bool(true))
            } else {
                Ok(Value::Bool(false))
            }
        }
        _ => Err("isNil needs one argument".to_owned()),
    }
}

fn is_number(_vm: &Vm, args: &[Value]) -> Result<Value, String> {
    match args.len() {
        1 => {
            if let Value::Number(_) = args[0] {
                Ok(Value::Bool(true))
            } else {
                Ok(Value::Bool(false))
            }
        }
        _ => Err("isNumber needs one argument".to_owned()),
    }
}

fn is_string(_vm: &Vm, args: &[Value]) -> Result<Value, String> {
    match args.len() {
        1 => {
            if let Value::VString(_) = args[0] {
                Ok(Value::Bool(true))
            } else {
                Ok(Value::Bool(false))
            }
        }
        _ => Err("isString needs one argument".to_owned()),
    }
}

fn lox_panic(vm: &Vm, args: &[Value]) -> Result<Value, String> {
    let mut terms: Vec<String> = vec![];

    for &arg in args.iter() {
        let formatter = GcTraceFormatter::new(arg, &vm.gc);
        let term = format!("{}", formatter);
        terms.push(term);
    }

    panic!("panic: {}", terms.join(", "))
}

fn max(_vm: &Vm, args: &[Value]) -> Result<Value, String> {
    match args.len() {
        0 | 1 => Err("max expects more than 1 argument".to_owned()),
        _ => {
            let mut max = -f64::INFINITY;
            for &arg in args.iter() {
                if let Value::Number(n) = arg {
                    max = max.max(n);
                } else {
                    return Err("max needs numeric argument".to_owned());
                }
            }
            Ok(Value::Number(max))
        }
    }
}

fn min(_vm: &Vm, args: &[Value]) -> Result<Value, String> {
    match args.len() {
        0 | 1 => Err("min expects more than 1 argument".to_owned()),
        _ => {
            let mut min = f64::INFINITY;
            for &arg in args.iter() {
                if let Value::Number(n) = arg {
                    min = min.min(n);
                } else {
                    return Err("min needs numeric argument".to_owned());
                }
            }
            Ok(Value::Number(min))
        }
    }
}
