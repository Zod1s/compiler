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
    init_string: GcRef<LoxString>,
}

impl Vm {
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

        vm.define_native("clock", NativeFn(clock));
        vm.define_native("panic", NativeFn(lox_panic));
        vm
    }

    fn pop(&mut self) -> Value {
        if let Some(value) = self.stack.pop() {
            value
        } else {
            eprintln!("Error: popping a value from empty stack");
            process::exit(65);
        }
    }

    fn push(&mut self, value: Value) {
        self.stack.push(value);
    }

    pub fn interpret(&mut self, code: &str) -> Result<(), InterpretError> {
        let function = compiler::compile(code, &mut self.gc)?;
        self.push(Value::Function(function));
        let closure = self.alloc(Closure::new(function));
        self.frames.push(CallFrame::new(closure, 0));
        self.run()
    }

    pub fn dump(&mut self, code: &str, file: &str) -> Result<(), InterpretError> {
        let function = compiler::compile(code, &mut self.gc)?;
        let function = self.gc.deref(function);
        let name = &self.gc.deref(function.name).s;
        let disassembler = Disassembler::new(&self.gc, &function.chunk, Some(&self.stack));
        let mut content = disassembler.disassemble_to_string(name);
        for gcref in self.gc.objects.iter().rev() {
            if let Some(gcref) = gcref {
                if let Some(fun) = gcref.object.as_any().downcast_ref::<Function>() {
                    if fun.name != function.name {
                        let name = &self.gc.deref(fun.name).s;
                        let disassembler =
                            Disassembler::new(&self.gc, &fun.chunk, Some(&self.stack));
                        content =
                            format!("{}{}", content, disassembler.disassemble_to_string(name));
                    }
                }
            }
        }
        fs::write(file, content).expect("Couldn't write to file");
        Ok(())
    }

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
                    (Value::Number(b), Value::Number(a)) => {
                        self.push(Value::Number(a + b));
                    }
                    (Value::VString(b), Value::VString(a)) => {
                        let a = self.gc.deref(a);
                        let b = self.gc.deref(b);
                        let new = format!("{}{}", a, b);
                        let string = self.intern(new);
                        self.push(Value::VString(string));
                    }
                    (Value::VString(b), Value::Number(a)) => {
                        let b = self.gc.deref(b);
                        let new = format!("{}{}", a, b);
                        let string = self.intern(new);
                        self.push(Value::VString(string));
                    }
                    (Value::Number(b), Value::VString(a)) => {
                        let a = self.gc.deref(a);
                        let new = format!("{}{}", a, b);
                        let string = self.intern(new);
                        self.push(Value::VString(string));
                    }
                    _ => self
                        .runtime_error("Arguments must be both numbers or at least one string.")?,
                },
                OpCode::Constant(index) => self.push(self.current_chunk().get_constant(index)),
                OpCode::Return => {
                    let frame = self.frames.pop().unwrap();
                    let result = self.pop();
                    self.close_upvalue(frame.slot);
                    if self.frames.is_empty() {
                        return Ok(());
                    } else {
                        self.stack.truncate(frame.slot);
                        self.push(result);
                    }
                }
                OpCode::Negate => match self.pop() {
                    Value::Number(n) => {
                        self.push(Value::Number(-n));
                    }
                    _ => {
                        self.runtime_error("No number found on stack.")?;
                    }
                },
                
                OpCode::Sub => {
                    self.bin_arith_op(|x, y| x - y, "when subtracting")?;
                }
                OpCode::Mul => {
                    self.bin_arith_op(|x, y| x * y, "when multiplying")?;
                }
                OpCode::Div => {
                    self.bin_arith_op(|x, y| x / y, "when dividing")?;
                }
                OpCode::False => self.push(Value::Bool(false)),
                OpCode::True => self.push(Value::Bool(true)),
                OpCode::Nil => self.push(Value::Nil),
                OpCode::Not => {
                    let value = self.pop().is_false();
                    self.push(Value::Bool(value));
                }
                OpCode::Equal => {
                    self.bin_op(|x, y| x == y)?;
                }
                OpCode::NotEqual => {
                    self.bin_op(|x, y| x != y)?;
                }
                OpCode::Greater => match (self.pop(), self.pop()) {
                    (Value::Number(b), Value::Number(a)) => {
                        self.push(Value::Bool(a > b));
                    }
                    (Value::VString(b), Value::VString(a)) => {
                        let a = self.gc.deref(a);
                        let b = self.gc.deref(b);
                        let result = Value::Bool(a > b);
                        self.push(result);
                    }
                    _ => self.runtime_error("Arguments must be of same type and comparable.")?,
                },
                OpCode::GreaterEqual => match (self.pop(), self.pop()) {
                    (Value::Number(b), Value::Number(a)) => {
                        self.push(Value::Bool(a >= b));
                    }
                    (Value::VString(b), Value::VString(a)) => {
                        let a = self.gc.deref(a);
                        let b = self.gc.deref(b);
                        let result = Value::Bool(a >= b);
                        self.push(result);
                    }
                    _ => self.runtime_error("Arguments must be of same type and comparable.")?,
                },
                OpCode::Less => match (self.pop(), self.pop()) {
                    (Value::Number(b), Value::Number(a)) => {
                        self.push(Value::Bool(a < b));
                    }
                    (Value::VString(b), Value::VString(a)) => {
                        let a = self.gc.deref(a);
                        let b = self.gc.deref(b);
                        let result = Value::Bool(a < b);
                        self.push(result);
                    }
                    _ => self.runtime_error("Arguments must be of same type and comparable.")?,
                },
                OpCode::LessEqual => match (self.pop(), self.pop()) {
                    (Value::Number(b), Value::Number(a)) => {
                        self.push(Value::Bool(a <= b));
                    }
                    (Value::VString(b), Value::VString(a)) => {
                        let a = self.gc.deref(a);
                        let b = self.gc.deref(b);
                        let result = Value::Bool(a <= b);
                        self.push(result);
                    }
                    _ => self.runtime_error("Arguments must be of same type and comparable.")?,
                },
                OpCode::Print => {
                    let value = self.pop();
                    if self.repl {
                        println!(">  {}", GcTraceFormatter::new(value, &self.gc));
                    } else {
                        println!("{}", GcTraceFormatter::new(value, &self.gc));
                    }
                }
                OpCode::Pop => {
                    self.pop();
                }
                OpCode::DefineGlobal(index) => {
                    if let Value::VString(string_ref) = self.current_chunk().constants[index] {
                        self.globals.insert(string_ref, self.peek(0));
                        self.pop();
                    } else {
                        self.runtime_error(
                            "Error: Invalid identifier found for definition on stack.",
                        )?
                    }
                }
                OpCode::GetGlobal(index) => {
                    if let Value::VString(string_ref) = self.current_chunk().get_constant(index) {
                        match self.globals.get(&string_ref) {
                            Some(&value) => self.push(value),
                            None => self.runtime_error(&format!(
                                "Undefined variable '{}'.",
                                self.gc.deref(string_ref)
                            ))?,
                        }
                    } else {
                        self.runtime_error("Error: Invalid identifier found for usage on stack.")?
                    }
                }
                OpCode::SetGlobal(index) => {
                    if let Value::VString(string_ref) = self.current_chunk().constants[index] {
                        if self.globals.insert(string_ref, self.peek(0)).is_none() {
                            self.globals.remove(&string_ref);
                            self.runtime_error(&format!(
                                "Undefined variable '{}'.",
                                self.gc.deref(string_ref)
                            ))?
                        }
                    } else {
                        self.runtime_error("Error: Invalid identifier found for usage on stack.")?
                    }
                }
                OpCode::GetLocal(slot) => {
                    self.push(self.stack[slot + self.current_frame().slot]);
                }
                OpCode::SetLocal(slot) => {
                    let index = slot + self.current_frame().slot;
                    self.stack[index] = self.peek(0);
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
                    self.push(value);
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
                OpCode::JumpIfFalse(offset) => {
                    if self.peek(0).is_false() {
                        self.current_frame_mut().ip += offset;
                    }
                }
                OpCode::Jump(offset) => {
                    self.current_frame_mut().ip += offset;
                }
                OpCode::Loop(offset) => {
                    self.current_frame_mut().ip -= offset + 1;
                }
                OpCode::Call(arg_count) => {
                    self.call_value(self.peek(arg_count), arg_count)?;
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
                        self.push(Value::Closure(closure));
                    }
                    _ => self.runtime_error("Error: no function found.")?,
                },
                OpCode::CloseUpvalue => {
                    let top = self.stack.len() - 1;
                    self.close_upvalue(top);
                    self.pop();
                }
                OpCode::Class(value) => {
                    if let Value::VString(name) = self.current_chunk().constants[value] {
                        let class = Class::new(name);
                        let class = self.alloc(class);
                        self.push(Value::Class(class));
                    } else {
                        self.runtime_error("Error: Invalid identifier found for usage on stack.")?
                    }
                }
                OpCode::GetProperty(slot) => {
                    if let Value::Instance(instance) = self.peek(0) {
                        let instance = self.gc.deref(instance);
                        if let Value::VString(name) = self.current_chunk().get_constant(slot) {
                            let value = instance.fields.get(&name);
                            if let Some(&value) = value {
                                self.pop();
                                self.push(value);
                            } else {
                                let class = instance.class;
                                self.bind_method(class, name)?;
                            }
                        } else {
                            self.runtime_error(
                                "Error: Invalid identifier found for usage on stack.",
                            )?
                        }
                    } else {
                        self.runtime_error("Only instances have properties.")?
                    }
                }
                OpCode::SetProperty(slot) => {
                    if let Value::Instance(instance) = self.peek(1) {
                        if let Value::VString(name) = self.current_chunk().get_constant(slot) {
                            let value = self.pop();
                            let instance = self.gc.deref_mut(instance);
                            instance.fields.insert(name, value);
                            self.pop();
                            self.push(value);
                        } else {
                            self.runtime_error(
                                "Error: Invalid identifier found for usage on stack.",
                            )?
                        }
                    } else {
                        self.runtime_error("Only instances have properties.")?
                    }
                }
                OpCode::Method(slot) => {
                    if let Value::VString(name) = self.current_chunk().get_constant(slot) {
                        self.define_method(name);
                    } else {
                        self.runtime_error("Error: Invalid identifier found for usage on stack.")?
                    }
                }
                OpCode::Invoke((name, count)) => {
                    if let Value::VString(name) = self.current_chunk().get_constant(name) {
                        self.invoke(name, count)?
                    } else {
                        self.runtime_error("Error: Invalid identifier found for usage on stack.")?
                    }
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
                        self.runtime_error("Error: Invalid identifier found for usage on stack.")?
                    }
                }
                OpCode::GetSuper(slot) => {
                    if let Value::VString(name) = self.current_chunk().get_constant(slot) {
                        if let Value::Class(superclass) = self.pop() {
                            self.bind_method(superclass, name)?
                        } else {
                            self.runtime_error("No superclass found on the stack")?
                        }
                    } else {
                        self.runtime_error("Error: Invalid identifier found for usage on stack.")?
                    }
                }
                OpCode::SuperInvoke((name, count)) => {
                    if let Value::VString(name) = self.current_chunk().get_constant(name) {
                        if let Value::Class(class) = self.pop() {
                            self.invoke_from_class(class, name, count)?
                        } else {
                            self.runtime_error("No class found on the stack")?
                        }
                    } else {
                        self.runtime_error("Error: Invalid identifier found for usage on stack.")?
                    }
                }
                OpCode::ReturnNil => {
                    let frame = self.frames.pop().unwrap();
                    self.close_upvalue(frame.slot);
                    if self.frames.is_empty() {
                        return Ok(());
                    } else {
                        self.stack.truncate(frame.slot);
                        self.push(Value::Nil);
                    }
                }
            }
        }
    }

    fn bin_arith_op(
        &mut self,
        f: fn(f64, f64) -> f64,
        message: &str,
    ) -> Result<(), InterpretError> {
        match (self.pop(), self.pop()) {
            (Value::Number(b), Value::Number(a)) => {
                self.push(Value::Number(f(a, b)));
                Ok(())
            }
            (_, Value::Number(_)) => {
                self.runtime_error(&format!("Second argument must be a number {}.", message))
            }
            (Value::Number(_), _) => {
                self.runtime_error(&format!("First argument must be a number {}.", message))
            }
            _ => self.runtime_error(&format!("Both arguments must be numbers {}.", message)),
        }
    }

    fn bin_op(&mut self, f: fn(Value, Value) -> bool) -> Result<(), InterpretError> {
        let (b, a) = (self.pop(), self.pop());
        self.push(Value::Bool(f(a, b)));
        Ok(())
    }

    fn runtime_error(&mut self, message: &str) -> Result<(), InterpretError> {
        eprintln!("{}", message);

        for frame in self.frames.iter().rev() {
            let closure = self.gc.deref(frame.closure);
            let function = self.gc.deref(closure.function);
            let name = self.gc.deref(function.name);
            let name = if name.s.is_empty() {
                "<script>"
            } else {
                &name.s
            };
            let line = function.chunk.get_line(frame.ip - 1);
            eprintln!("[line {}] in {}", line, name);
        }

        self.stack.clear();
        Err(InterpretError::Runtime)
    }

    pub fn set_debug(&mut self) {
        self.debug = true;
    }

    pub fn unset_debug(&mut self) {
        self.debug = false;
    }

    fn peek(&self, index: usize) -> Value {
        self.stack[self.stack.len() - 1 - index]
    }

    fn current_frame(&self) -> &CallFrame {
        self.frames.last().unwrap()
    }

    fn current_closure(&self) -> &Closure {
        let closure = self.current_frame().closure;
        let closure = self.gc.deref(closure);
        &closure
    }

    fn current_frame_mut(&mut self) -> &mut CallFrame {
        self.frames.last_mut().unwrap()
    }

    fn current_chunk(&self) -> &Chunk {
        let function = self.gc.deref(self.current_closure().function);
        &function.chunk
    }

    fn call_value(&mut self, callee: Value, arg_count: usize) -> Result<(), InterpretError> {
        match callee {
            Value::NativeFn(fun) => {
                let left = self.stack.len() - arg_count;
                let result = fun.0(self, &self.stack[left..]);
                self.stack.truncate(left - 1);
                self.push(result);
                Ok(())
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
                            self.runtime_error("Initializer is not closure")
                        }
                    }
                    None => {
                        if arg_count != 0 {
                            let msg = format!("Expected 0 arguments but got {}.", arg_count);
                            self.runtime_error(&msg)
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
            _ => self.runtime_error("Can only call functions and classes."),
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
            self.runtime_error(&msg)
        } else {
            let frame = CallFrame::new(callee, self.stack.len() - arg_count - 1);
            self.frames.push(frame);
            Ok(())
        }
    }

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

    fn define_method(&mut self, name: GcRef<LoxString>) {
        let method = self.peek(0);
        if let Value::Class(class) = self.peek(1) {
            let class = self.gc.deref_mut(class);
            class.methods.insert(name, method);
            self.pop();
        } else {
            panic!("Cannot define a method on non class.");
        }
    }

    fn bind_method(
        &mut self,
        class: GcRef<Class>,
        name: GcRef<LoxString>,
    ) -> Result<(), InterpretError> {
        let class = self.gc.deref(class);
        if let Some(method) = class.methods.get(&name) {
            let receiver = self.peek(0);
            let method = match method {
                Value::Closure(cl) => cl,
                _ => panic!("No method found"),
            };
            let bound = BoundMethod::new(receiver, *method);
            let bound = self.alloc(bound);
            self.pop();
            self.push(Value::BoundMethod(bound));
            Ok(())
        } else {
            let name = &self.gc.deref(name).s;
            let message = format!("Undefined property '{}'.", name);
            self.runtime_error(&message)
        }
    }

    fn invoke(&mut self, name: GcRef<LoxString>, arg_count: usize) -> Result<(), InterpretError> {
        let receiver = self.peek(arg_count);
        if let Value::Instance(instance) = receiver {
            let instance = self.gc.deref(instance);
            if let Some(&value) = instance.fields.get(&name) {
                let pos = self.stack.len() - 1 - arg_count;
                self.stack[pos] = value;
                self.call_value(value, arg_count)
            } else {
                let class = instance.class;
                self.invoke_from_class(class, name, arg_count)
            }
        } else {
            self.runtime_error("Only instances have methods.")
        }
    }

    fn invoke_from_class(
        &mut self,
        class: GcRef<Class>,
        name: GcRef<LoxString>,
        arg_count: usize,
    ) -> Result<(), InterpretError> {
        let class = self.gc.deref(class);
        if let Some(&method) = class.methods.get(&name) {
            if let Value::Closure(closure) = method {
                self.call(closure, arg_count)
            } else {
                panic!("Got method that is not closure!")
            }
        } else {
            let name = &self.gc.deref(name).s;
            let message = format!("No method with name: {} found in class.", name);
            self.runtime_error(&message)
        }
    }

    // gc

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

    fn alloc<T: GcTrace + 'static + fmt::Debug>(&mut self, object: T) -> GcRef<T> {
        self.collect_garbage();
        self.gc.alloc(object)
    }

    fn intern(&mut self, string: String) -> GcRef<LoxString> {
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

fn clock(vm: &Vm, _args: &[Value]) -> Value {
    let time = vm.start_time.elapsed().as_secs_f64();
    Value::Number(time)
}

fn lox_panic(vm: &Vm, args: &[Value]) -> Value {
    let mut terms: Vec<String> = vec![];

    for &arg in args.iter() {
        let formatter = GcTraceFormatter::new(arg, &vm.gc);
        let term = format!("{}", formatter);
        terms.push(term);
    }

    panic!("panic: {}", terms.join(", "))
}
