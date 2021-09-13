use crate::{
    chunk::{disassemble_instruction, Chunk, OpCode},
    compiler,
    object::LoxString,
    types::{InterpretError, Value},
};
use std::collections::HashMap;

pub struct Vm {
    chunk: Chunk,
    ip: usize,
    debug: bool,
    stack: Vec<Value>,
    globals: HashMap<String, Value>,
}

impl Vm {
    pub fn new(debug: bool) -> Self {
        Vm {
            chunk: Chunk::new(),
            ip: 0,
            debug,
            stack: Vec::new(),
            globals: HashMap::new(),
        }
    }

    pub fn init(&mut self, chunk: Chunk) {
        self.chunk = chunk;
    }

    pub fn interpret(&mut self, code: &str) -> Result<(), InterpretError> {
        let mut chunk = Chunk::new();
        if !compiler::compile(code, &mut chunk, self.debug) {
            Err(InterpretError::Compile)
        } else {
            self.chunk = chunk;
            self.ip = 0;
            self.run()
        }
    }

    pub fn run(&mut self) -> Result<(), InterpretError> {
        loop {
            if self.ip >= self.chunk.code.len() {
                self.runtime_error("Instruction pointer out of bounds.")?;
            }
            let instruction = self.chunk.get_opcode(self.ip);
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
                disassemble_instruction(&self.chunk, instruction, self.ip);
                println!();
            }
            self.ip += 1;
            match instruction {
                OpCode::Constant(i) => self.stack.push(self.chunk.get_constant(i)),
                OpCode::Return => {
                    return Ok(());
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
                    _ => self.runtime_error("Error: not enough numbers on stack.")?,
                },
                OpCode::Sub => {
                    self.bin_arith_op(|x, y| x - y)?;
                }
                OpCode::Mul => {
                    self.bin_arith_op(|x, y| x * y)?;
                }
                OpCode::Div => {
                    self.bin_arith_op(|x, y| x / y)?;
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
                    _ => self.runtime_error("Error: not enough numbers on stack.")?,
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
                    _ => self.runtime_error("Error: not enough numbers on stack.")?,
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
                    _ => self.runtime_error("Error: not enough numbers on stack.")?,
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
                    _ => self.runtime_error("Error: not enough numbers on stack.")?,
                },
                OpCode::Print => {
                    if let Some(v) = self.stack.pop() {
                        println!("{}", v);
                    } else {
                        self.runtime_error("Error: not enough numbers on stack.")?
                    }
                }
                OpCode::Pop => {
                    self.stack.pop();
                }
                OpCode::DefineGlobal(c) => {
                    if let Value::VString(LoxString { s: name }) = self.chunk.constants[c].clone() {
                        self.globals.insert(name, self.peek(0));
                        self.stack.pop();
                    } else {
                        self.runtime_error(
                            "Error: Invalid identifier found for definition on stack.",
                        )?
                    }
                }
                OpCode::GetGlobal(c) => {
                    if let Value::VString(LoxString { s: name }) = self.chunk.constants[c].clone() {
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
                OpCode::SetGlobal(c) => {
                    if let Value::VString(LoxString { s: name }) = self.chunk.constants[c].clone() {
                        if self.globals.insert(name.clone(), self.peek(0)).is_none() {
                            self.globals.remove(&name);
                            self.runtime_error(&format!("Undefined variable '{}'.", name))?
                        }
                    } else {
                        self.runtime_error("Error: Invalid identifier found for usage on stack.")?
                    }
                }
                OpCode::GetLocal(slot) => {
                    self.stack.push(self.stack[slot].clone());
                }
                OpCode::SetLocal(slot) => {
                    self.stack[slot] = self.peek(0);
                } // _ => (),
            }
        }
    }

    fn bin_arith_op(&mut self, f: fn(f64, f64) -> f64) -> Result<(), InterpretError> {
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
            _ => self.runtime_error("Error: not enough numbers on stack."),
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
        eprintln!("{}", message);
        let line = self.chunk.get_line(self.ip - 1);
        eprintln!("[line {}] in script", line);
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
}
