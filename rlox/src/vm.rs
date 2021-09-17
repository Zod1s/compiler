use crate::{
    chunk::{disassemble_instruction, disassemble_instruction_str, Chunk, OpCode},
    compiler,
    object::{Closure, LoxString, NativeFn, Upvalue},
    types::{new_mutref, InterpretError, MutRef, Value},
};
use cpu_time::ProcessTime;
use std::{collections::HashMap, fs, process, rc::Rc};

pub struct Vm {
    debug: bool,
    stack: Vec<Value>,
    globals: HashMap<String, Value>,
    frames: Vec<CallFrame>,
    open_upvalues: Vec<MutRef<Upvalue>>,
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
                    self.push(self.stack[slot + self.current_frame().slot].clone());
                }
                OpCode::SetLocal(slot) => {
                    let index = slot + self.current_frame().slot;
                    self.stack[index] = self.peek(0);
                }
                OpCode::GetUpvalue(slot) => {
                    let value = {
                        let upvalue = self.current_closure().upvalues[slot].borrow();
                        if let Some(value) = upvalue.closed.clone() {
                            value
                        } else {
                            self.stack[upvalue.location].clone()
                        }
                    };
                    self.push(value);
                }
                OpCode::SetUpvalue(slot) => {
                    let upvalue = Rc::clone(&self.current_closure().upvalues[slot]);
                    let value = self.peek(0);
                    if upvalue.borrow().closed.is_none() {
                        self.stack[upvalue.borrow().location] = value;
                    } else {
                        upvalue.borrow_mut().closed = Some(value);
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
                                Rc::clone(&self.current_closure().upvalues[upvalue.index])
                            };
                            closure.upvalues.push(value);
                        }

                        self.push(Value::Closure(closure));
                    }
                    _ => self.runtime_error("Error: no function found.")?,
                },
                OpCode::CloseUpvalue => {
                    let top = self.stack.len() - 1;
                    self.close_upvalue(top);
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
        self.stack[self.stack.len() - 1 - index].clone()
    }

    fn current_frame(&self) -> &CallFrame {
        self.frames.last().unwrap()
    }

    fn current_closure(&self) -> &Closure {
        &self.current_frame().closure
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
                let result = fun.0(self, &self.stack[left..]);
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

    fn capture_upvalue(&mut self, index: usize) -> MutRef<Upvalue> {
        for upvalue in &self.open_upvalues {
            if upvalue.borrow().location == index {
                return Rc::clone(upvalue);
            }
        }
        let upvalue = new_mutref(Upvalue::new(index));
        self.open_upvalues.push(Rc::clone(&upvalue));
        upvalue
    }

    fn close_upvalue(&mut self, last: usize) {
        let mut i = 0;
        while i != self.open_upvalues.len() {
            let upvalue = Rc::clone(&self.open_upvalues[i]);
            let location = upvalue.borrow().location;
            if location >= last {
                self.open_upvalues.remove(i);
                upvalue.borrow_mut().closed = Some(self.stack[location].clone());
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
