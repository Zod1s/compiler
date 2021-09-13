use crate::{
    chunk::{disassemble_instruction, disassemble_instruction_str, Chunk, OpCode},
    compiler,
    object::{Function, LoxString, NativeFn},
    types::{InterpretError, Value},
};
use cpu_time::ProcessTime;
use std::{collections::HashMap, fs};

pub struct Vm {
    debug: bool,
    stack: Vec<Value>,
    globals: HashMap<String, Value>,
    frames: Vec<CallFrame>,
    start_time: ProcessTime,
}

impl Vm {
    pub fn new(debug: bool) -> Self {
        let mut vm = Vm {
            debug,
            stack: Vec::new(),
            globals: HashMap::new(),
            frames: Vec::new(),
            start_time: ProcessTime::now(),
        };

        vm.define_native("clock".to_string(), NativeFn(clock));
        // vm.define_native("panic".to_string(), NativeFn(lox_panic));
        vm
    }

    pub fn interpret(&mut self, code: &str) -> Result<(), InterpretError> {
        let function = compiler::compile(code, false)?;
        self.stack.push(Value::Function(function.clone()));
        // self.frames.push(CallFrame::new(function, 0));
        self.call(function, 0)?;
        self.run()
    }

    pub fn dump(&mut self, code: &str, out_file: &str) -> Result<(), InterpretError> {
        let function = compiler::compile(code, false)?;
        let mut content = String::new();
        for (i, op) in function.chunk.code.iter().enumerate() {
            content.push_str(&disassemble_instruction_str(&function.chunk, *op, i));
        }
        fs::write(out_file, content).expect("Unable to write to file");
        Ok(())
    }

    pub fn run(&mut self) -> Result<(), InterpretError> {
        loop {
            if self.current_frame().ip >= self.current_chunk().code.len() {
                self.runtime_error("Instruction pointer out of bounds.")?;
            }
            let instruction = self.current_chunk().get_opcode(self.current_frame().ip);
            if self.debug {
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
                OpCode::Constant(index) => {
                    self.stack.push(self.current_chunk().get_constant(index))
                }
                OpCode::Return => {
                    if let Some(result) = self.stack.pop() {
                        let frame = self.frames.pop().unwrap();
                        if self.frames.len() == 0 {
                            self.stack.pop();
                            return Ok(());
                        } else {
                            self.stack.truncate(frame.slot);
                            self.stack.push(result);
                        }
                    } else {
                        self.runtime_error("Error: empty stack.")?;
                    }
                }
                OpCode::Negate => match self.stack.pop() {
                    Some(Value::Number(n)) => {
                        self.stack.push(Value::Number(-n));
                    }
                    Some(_) => {
                        self.runtime_error("No number found on stack.")?;
                    }
                    _ => {
                        self.runtime_error("Error: empty stack.")?;
                    }
                },
                OpCode::Add => match (self.stack.pop(), self.stack.pop()) {
                    (Some(Value::Number(b)), Some(Value::Number(a))) => {
                        self.stack.push(Value::Number(a + b));
                    }
                    (Some(Value::VString(b)), Some(Value::VString(a))) => {
                        self.stack
                            .push(Value::VString(LoxString::new(format!("{}{}", a, b))));
                    }
                    (Some(Value::VString(b)), Some(Value::Number(a))) => {
                        self.stack
                            .push(Value::VString(LoxString::new(format!("{}{}", a, b))));
                    }
                    (Some(Value::Number(b)), Some(Value::VString(a))) => {
                        self.stack
                            .push(Value::VString(LoxString::new(format!("{}{}", a, b))));
                    }
                    (Some(_), Some(_)) => self
                        .runtime_error("Arguments must be both numbers or at least one string.")?,
                    _ => self.runtime_error("Error: not enough numbers on stack when summing.")?,
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
                OpCode::False => self.stack.push(Value::Bool(false)),
                OpCode::True => self.stack.push(Value::Bool(true)),
                OpCode::Nil => self.stack.push(Value::Nil),
                OpCode::Not => {
                    if let Some(value) = self.stack.pop() {
                        self.stack.push(Value::Bool(value.is_false()));
                    } else {
                        return self.runtime_error("Empty stack.");
                    }
                }
                OpCode::Equal => {
                    self.bin_op(|x, y| x == y)?;
                }
                OpCode::NotEqual => {
                    self.bin_op(|x, y| x != y)?;
                }
                OpCode::Greater => match (self.stack.pop(), self.stack.pop()) {
                    (Some(Value::Number(b)), Some(Value::Number(a))) => {
                        self.stack.push(Value::Bool(a > b));
                    }
                    (Some(Value::VString(b)), Some(Value::VString(a))) => {
                        self.stack.push(Value::Bool(a > b));
                    }
                    (Some(_), Some(_)) => {
                        self.runtime_error("Arguments must be of same type and comparable.")?
                    }
                    _ => {
                        self.runtime_error("Error: not enough numbers on stack when comparing >.")?
                    }
                },
                OpCode::GreaterEqual => match (self.stack.pop(), self.stack.pop()) {
                    (Some(Value::Number(b)), Some(Value::Number(a))) => {
                        self.stack.push(Value::Bool(a >= b));
                    }
                    (Some(Value::VString(b)), Some(Value::VString(a))) => {
                        self.stack.push(Value::Bool(a >= b));
                    }
                    (Some(_), Some(_)) => {
                        self.runtime_error("Arguments must be of same type and comparable.")?
                    }
                    _ => {
                        self.runtime_error("Error: not enough numbers on stack when comparing >=.")?
                    }
                },
                OpCode::Less => match (self.stack.pop(), self.stack.pop()) {
                    (Some(Value::Number(b)), Some(Value::Number(a))) => {
                        self.stack.push(Value::Bool(a < b));
                    }
                    (Some(Value::VString(b)), Some(Value::VString(a))) => {
                        self.stack.push(Value::Bool(a < b));
                    }
                    (Some(_), Some(_)) => {
                        self.runtime_error("Arguments must be of same type and comparable.")?
                    }
                    _ => {
                        self.runtime_error("Error: not enough numbers on stack when comparing <.")?
                    }
                },
                OpCode::LessEqual => match (self.stack.pop(), self.stack.pop()) {
                    (Some(Value::Number(b)), Some(Value::Number(a))) => {
                        self.stack.push(Value::Bool(a <= b));
                    }
                    (Some(Value::VString(b)), Some(Value::VString(a))) => {
                        self.stack.push(Value::Bool(a <= b));
                    }
                    (Some(_), Some(_)) => {
                        self.runtime_error("Arguments must be of same type and comparable.")?
                    }
                    _ => {
                        self.runtime_error("Error: not enough numbers on stack when comparing <=.")?
                    }
                },
                OpCode::Print => {
                    if let Some(v) = self.stack.pop() {
                        println!(">  {}", v);
                    } else {
                        self.runtime_error("Error: not enough numbers on stack when printing.")?
                    }
                }
                OpCode::Pop => {
                    self.stack.pop();
                }
                OpCode::DefineGlobal(index) => {
                    if let Value::VString(LoxString { s: name }) =
                        self.current_chunk().constants[index].clone()
                    {
                        self.globals.insert(name, self.peek(0));
                        self.stack.pop();
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
                            Some(value) => self.stack.push(value.clone()),
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
                    self.stack
                        .push(self.stack[slot + self.current_frame().slot].clone());
                }
                OpCode::SetLocal(slot) => {
                    let index = slot + self.current_frame().slot;
                    self.stack[index] = self.peek(0);
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
                } // _ => (),
            }
        }
    }

    fn bin_arith_op(
        &mut self,
        f: fn(f64, f64) -> f64,
        message: &str,
    ) -> Result<(), InterpretError> {
        match (self.stack.pop(), self.stack.pop()) {
            (Some(Value::Number(b)), Some(Value::Number(a))) => {
                self.stack.push(Value::Number(f(a, b)));
                Ok(())
            }
            (Some(_), Some(Value::Number(_))) => {
                self.runtime_error("Second argument must be a number.")
            }
            (Some(Value::Number(_)), Some(_)) => {
                self.runtime_error("First argument must be a number.")
            }
            (Some(_), Some(_)) => self.runtime_error("Both arguments must be numbers."),
            _ => self.runtime_error(&format!("Error: not enough numbers on stack {}.", message)),
        }
    }

    fn bin_op(&mut self, f: fn(Value, Value) -> bool) -> Result<(), InterpretError> {
        match (self.stack.pop(), self.stack.pop()) {
            (Some(b), Some(a)) => {
                self.stack.push(Value::Bool(f(a, b)));
                Ok(())
            }
            _ => self.runtime_error("Error: not enough values on stack."),
        }
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
                frame.function.chunk.get_line(frame.ip - 1),
                frame.function.name
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

    fn current_frame_mut(&mut self) -> &mut CallFrame {
        self.frames.last_mut().unwrap()
    }

    fn current_chunk(&self) -> &Chunk {
        &self.current_frame().function.chunk
    }

    fn call_value(&mut self, callee: Value, arg_count: usize) -> Result<(), InterpretError> {
        match callee {
            Value::Function(fun) => self.call(fun, arg_count),
            Value::NativeFn(fun) => {
                let left = self.stack.len() - arg_count;
                let result = fun.0(&self, &self.stack[left..]);
                self.stack.truncate(left - 1);
                self.stack.push(result);
                Ok(())
            }
            _ => self.runtime_error("Can only call functions and classes."),
        }
    }

    fn call(&mut self, callee: Function, arg_count: usize) -> Result<(), InterpretError> {
        if callee.arity != arg_count {
            let msg = format!("Expected {} arguments but got {}.", callee.arity, arg_count);
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
}

struct CallFrame {
    function: Function,
    ip: usize,
    slot: usize,
}

impl CallFrame {
    pub fn new(function: Function, slot: usize) -> Self {
        CallFrame {
            function,
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
