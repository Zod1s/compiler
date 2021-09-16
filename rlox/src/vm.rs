use crate::{
    chunk::{disassemble_instruction, disassemble_instruction_str, Chunk, OpCode},
    compiler,
    object::{Closure, LoxString, NativeFn, Upvalue},
    types::{InterpretError, Value},
};
use cpu_time::ProcessTime;
use std::{collections::HashMap, fs, process}; //, rc::Rc, cell::{RefCell, Ref}};

// 25.4.4
// Use RefCell to enable internal mutability with references in Upvalue, using also Rc to count references

pub struct Vm {
    debug: bool,
    stack: Vec<Value>,
    globals: HashMap<String, Value>,
    frames: Vec<CallFrame>,
    open_upvalues: Vec<Upvalue>,
    start_time: ProcessTime,
}

impl Default for Vm {
    fn default() -> Self {
        Vm::new()
    }
}

impl Vm {
    pub fn new() -> Self {
        let mut vm = Vm {
            debug: false,
            stack: Vec::new(),
            globals: HashMap::new(),
            frames: Vec::new(),
            open_upvalues: Vec::new(),
            start_time: ProcessTime::now(),
        };

        vm.define_native("clock".to_string(), NativeFn(clock));
        // vm.define_native("panic".to_string(), NativeFn(lox_panic));
        vm
    }

    pub fn pop(&mut self) -> Value {
        if let Some(value) = self.stack.pop() {
            value
        } else {
            eprintln!("Error: popping a value from empty stack");
            process::exit(65);
        }
    }

    pub fn push(&mut self, value: Value) {
        self.stack.push(value);
    }

    pub fn set_at(&mut self, index: usize, value: Value) {
        if index < self.stack.len() {
            self.stack[index] = value;
        } else {
            eprintln!("Error: setting a value out of stack boundaries");
            process::exit(65);
        }
    }
    pub fn get_at(&self, index: usize, line: &str) -> Value {
        if index < self.stack.len() {
            self.stack[index].clone()
        } else {
            eprintln!("Error: getting a value out of stack boundaries at {}", line);
            process::exit(65);
        }
    }

    pub fn interpret(&mut self, code: &str) -> Result<(), InterpretError> {
        let function = compiler::compile(code)?;
        if cfg!(feature = "dump") {
            let mut content = String::new();
            for (i, op) in function.chunk.code.iter().enumerate() {
                content.push_str(&disassemble_instruction_str(&function.chunk, *op, i));
            }
            fs::write("./dump.txt", content).expect("Unable to write to file");
            Ok(())
        } else {
            self.push(Value::Function(function.clone()));
            let closure = Closure::new(function);
            self.frames.push(CallFrame::new(closure, 0));
            self.run()
        }
    }

    pub fn run(&mut self) -> Result<(), InterpretError> {
        loop {
            if self.current_frame().ip >= self.current_chunk().code.len() {
                self.runtime_error("Instruction pointer out of bounds.")?;
            }
            let instruction = self.current_chunk().get_opcode(self.current_frame().ip);
            if self.debug || cfg!(feature = "debug") {
                println!("==== Stack content ====");
                if self.stack.is_empty() {
                    println!("[empty stack]");
                } else {
                    for elem in &self.stack {
                        println!("[{}]", elem);
                    }
                }
                println!("\nCurrent instruction: ");
                disassemble_instruction(self.current_chunk(), instruction, self.current_frame().ip);
                println!();
            }
            self.current_frame_mut().ip += 1;
            match instruction {
                OpCode::Constant(index) => self.push(self.current_chunk().get_constant(index)),
                OpCode::Return => {
                    let result = self.pop();
                    let frame = self.frames.pop().unwrap();
                    self.close_upvalue(frame.slot, "116");
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
                OpCode::Add => match (self.pop(), self.pop()) {
                    (Value::Number(b), Value::Number(a)) => {
                        self.push(Value::Number(a + b));
                    }
                    (Value::VString(b), Value::VString(a)) => {
                        self.push(Value::VString(LoxString::new(format!("{}{}", a, b))));
                    }
                    (Value::VString(b), Value::Number(a)) => {
                        self.push(Value::VString(LoxString::new(format!("{}{}", a, b))));
                    }
                    (Value::Number(b), Value::VString(a)) => {
                        self.push(Value::VString(LoxString::new(format!("{}{}", a, b))));
                    }
                    _ => self
                        .runtime_error("Arguments must be both numbers or at least one string.")?,
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
                        self.push(Value::Bool(a > b));
                    }
                    _ => self.runtime_error("Arguments must be of same type and comparable.")?,
                },
                OpCode::GreaterEqual => match (self.pop(), self.pop()) {
                    (Value::Number(b), Value::Number(a)) => {
                        self.push(Value::Bool(a >= b));
                    }
                    (Value::VString(b), Value::VString(a)) => {
                        self.push(Value::Bool(a >= b));
                    }
                    _ => self.runtime_error("Arguments must be of same type and comparable.")?,
                },
                OpCode::Less => match (self.pop(), self.pop()) {
                    (Value::Number(b), Value::Number(a)) => {
                        self.push(Value::Bool(a < b));
                    }
                    (Value::VString(b), Value::VString(a)) => {
                        self.push(Value::Bool(a < b));
                    }
                    _ => self.runtime_error("Arguments must be of same type and comparable.")?,
                },
                OpCode::LessEqual => match (self.pop(), self.pop()) {
                    (Value::Number(b), Value::Number(a)) => {
                        self.push(Value::Bool(a <= b));
                    }
                    (Value::VString(b), Value::VString(a)) => {
                        self.push(Value::Bool(a <= b));
                    }
                    _ => self.runtime_error("Arguments must be of same type and comparable.")?,
                },
                OpCode::Print => {
                    println!(">  {}", self.pop());
                }
                OpCode::Pop => {
                    self.pop();
                }
                OpCode::DefineGlobal(index) => {
                    if let Value::VString(LoxString { s: name }) =
                        self.current_chunk().constants[index].clone()
                    {
                        self.globals.insert(name, self.peek(0));
                        self.pop();
                    } else {
                        self.runtime_error(
                            "Error: Invalid identifier found for definition on stack.",
                        )?
                    }
                }
                OpCode::GetGlobal(index) => {
                    if let Value::VString(LoxString { s: name }) =
                        self.current_chunk().constants[index].clone()
                    {
                        match self.globals.get(&name) {
                            Some(value) => {
                                let value = value.clone();
                                self.push(value)
                            }
                            None => {
                                self.runtime_error(&format!("Undefined variable '{}'.", name))?
                            }
                        }
                    } else {
                        self.runtime_error("Error: Invalid identifier found for usage on stack.")?
                    }
                }
                OpCode::SetGlobal(index) => {
                    if let Value::VString(LoxString { s: name }) =
                        self.current_chunk().constants[index].clone()
                    {
                        if self.globals.insert(name.clone(), self.peek(0)).is_none() {
                            self.globals.remove(&name);
                            self.runtime_error(&format!("Undefined variable '{}'.", name))?
                        }
                    } else {
                        self.runtime_error("Error: Invalid identifier found for usage on stack.")?
                    }
                }
                OpCode::GetLocal(slot) => {
                    self.push(self.get_at(slot + self.current_frame().slot, "255"));
                }
                OpCode::SetLocal(slot) => {
                    let index = slot + self.current_frame().slot;
                    self.set_at(index, self.peek(0));
                }
                OpCode::GetUpvalue(slot) => {
                    let value = {
                        let upvalue = self.current_closure().upvalues[slot].clone();
                        if let Some(value) = upvalue.closed {
                            value
                        } else {
                            self.get_at(upvalue.location, "267")
                        }
                    };
                    self.push(value);
                }
                OpCode::SetUpvalue(slot) => {
                    let is_closed = self.current_closure().upvalues[slot].closed.is_none();
                    let value = self.peek(0);
                    if is_closed {
                        let location = self.current_closure().upvalues[slot].location;
                        self.stack[location] = value;
                    } else {
                        self.current_closure_mut().upvalues[slot].closed = Some(value);
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
                        let upvalue_count = function.upvalues.len();
                        let mut closure = Closure::new(function);

                        for i in 0..upvalue_count {
                            let upvalue = closure.function.upvalues[i];
                            let value = if upvalue.is_local {
                                self.capture_upvalue(self.current_frame().slot + upvalue.index)
                            } else {
                                self.current_closure().upvalues[upvalue.index].clone()
                            };
                            closure.upvalues.push(value);
                        }

                        self.push(Value::Closure(closure));
                    }
                    _ => self.runtime_error("Error: no function found.")?,
                },
                OpCode::CloseUpvalue => {
                    let top = self.stack.len() - 1;
                    self.close_upvalue(top, "318");
                    self.pop();
                } // _ => (),
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
            (_, Value::Number(_)) => self.runtime_error(
                &format!("Second argument must be a number {}.", message).to_string(),
            ),
            (Value::Number(_), _) => self.runtime_error(
                &format!("First argument must be a number {}.", message).to_string(),
            ),
            _ => self
                .runtime_error(&format!("Both arguments must be numbers {}.", message).to_string()),
        }
    }

    fn bin_op(&mut self, f: fn(Value, Value) -> bool) -> Result<(), InterpretError> {
        let (b, a) = (self.pop(), self.pop());
        self.push(Value::Bool(f(a, b)));
        Ok(())
    }

    fn runtime_error(&mut self, message: &str) -> Result<(), InterpretError> {
        let frame = self.current_frame();
        eprintln!("{}", message);
        let chunk = self.current_chunk();
        let line = chunk.get_line(frame.ip - 1);
        eprintln!("[line {}] in script", line);

        for frame in self.frames.iter().rev() {
            eprintln!(
                "[line {}] in {}",
                frame.closure.function.chunk.get_line(frame.ip - 1),
                frame.closure.function.name
            );
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
        self.get_at(self.stack.len() - 1 - index, "380")
    }

    fn current_frame(&self) -> &CallFrame {
        self.frames.last().unwrap()
    }

    fn current_closure(&self) -> &Closure {
        &self.current_frame().closure
    }

    fn current_closure_mut(&mut self) -> &mut Closure {
        &mut self.current_frame_mut().closure
    }

    fn current_frame_mut(&mut self) -> &mut CallFrame {
        self.frames.last_mut().unwrap()
    }

    fn current_chunk(&self) -> &Chunk {
        &self.current_frame().closure.function.chunk
    }

    fn call_value(&mut self, callee: Value, arg_count: usize) -> Result<(), InterpretError> {
        match callee {
            Value::NativeFn(fun) => {
                let left = self.stack.len() - arg_count;
                let result = fun.0(&self, &self.stack[left..]);
                self.stack.truncate(left - 1);
                self.push(result);
                Ok(())
            }
            Value::Closure(fun) => self.call(fun, arg_count),
            _ => self.runtime_error("Can only call functions and classes."),
        }
    }

    fn call(&mut self, callee: Closure, arg_count: usize) -> Result<(), InterpretError> {
        if callee.function.arity != arg_count {
            let msg = format!(
                "Expected {} arguments but got {}.",
                callee.function.arity, arg_count
            );
            self.runtime_error(&msg)
        } else {
            let frame = CallFrame::new(callee, self.stack.len() - arg_count - 1);
            self.frames.push(frame);
            Ok(())
        }
    }

    fn define_native(&mut self, name: String, function: NativeFn) {
        self.globals.insert(name, Value::NativeFn(function));
    }

    fn capture_upvalue(&mut self, index: usize) -> Upvalue {
        for upvalue in &self.open_upvalues {
            if upvalue.location == index {
                return upvalue.clone();
            }
        }
        let upvalue = Upvalue::new(index);
        self.open_upvalues.push(upvalue.clone());
        upvalue
    }

    fn close_upvalue(&mut self, last: usize, line: &str) {
        let mut i = 0;
        while i < self.open_upvalues.len() {
            if self.open_upvalues[i].location >= last {
                let mut upvalue = self.open_upvalues.remove(i);
                upvalue.closed = Some(self.get_at(i, &format!("450, {}", line)));
            } else {
                i += 1;
            }
        }
    }
}

struct CallFrame {
    closure: Closure,
    ip: usize,
    slot: usize,
}

impl CallFrame {
    pub fn new(closure: Closure, slot: usize) -> Self {
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

// fn lox_panic(vm: &Vm, args: &[Value]) -> Value {
//     let mut terms: Vec<String> = vec![];

//     for &arg in args.iter() {
//         let formatter = GcTraceFormatter::new(arg, &vm.gc);
//         let term = format!("{}", formatter);
//         terms.push(term);
//     }

//     panic!("panic: {}", terms.join(", "))
// }
