use crate::{
    gc::{Gc, GcTraceFormatter},
    types::Value,
};
// use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum OpCode {
    Add,
    Call(usize),
    Class(usize),
    CloseUpvalue,
    Closure(usize),
    Constant(usize),
    DefineGlobal(usize),
    Div,
    Equal,
    False,
    GetGlobal(usize),
    GetLocal(usize),
    GetProperty(usize),
    GetSuper(usize),
    GetUpvalue(usize),
    Greater,
    GreaterEqual,
    Inherit,
    Invoke((usize, usize)),
    Jump(usize),
    JumpIfFalse(usize),
    Less,
    LessEqual,
    Loop(usize),
    Method(usize),
    Mul,
    Negate,
    Nil,
    Not,
    NotEqual,
    Pop,
    Print,
    Return,
    ReturnNil,
    SetGlobal(usize),
    SuperInvoke((usize, usize)),
    SetLocal(usize),
    SetProperty(usize),
    SetUpvalue(usize),
    Sub,
    True,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Chunk {
    pub code: Vec<OpCode>,
    pub constants: Vec<Value>,
    pub lines: Vec<(usize, usize)>, // repetition, line
}

impl Default for Chunk {
    fn default() -> Self {
        Chunk::new()
    }
}

impl Chunk {
    pub fn new() -> Self {
        Chunk {
            code: Vec::new(),
            constants: Vec::new(),
            lines: Vec::new(),
        }
    }

    pub fn write(&mut self, opcode: OpCode, line: usize) {
        self.code.push(opcode);
        self.add_line(line);
    }

    #[inline]
    pub fn get_opcode(&self, index: usize) -> OpCode {
        self.code[index]
    }

    #[inline]
    pub fn get_constant(&self, index: usize) -> Value {
        self.constants[index]
    }

    pub fn add_constant(&mut self, value: Value) -> usize {
        if let Value::VString(_) = value {
            if self.constants.contains(&value) {
                self.constants.iter().position(|r| r == &value).unwrap()
            } else {
                self.constants.push(value);
                self.constants.len() - 1
            }
        } else {
            self.constants.push(value);
            self.constants.len() - 1
        }
    }

    pub fn get_line(&self, index: usize) -> usize {
        let mut ind = 0;
        let mut i = 0;
        let len = self.lines.len();
        while ind <= index && i < len {
            ind += self.lines[i].0;
            i += 1;
        }

        self.lines[i - 1].1
    }

    fn add_line(&mut self, line: usize) {
        let last = self.lines.len();
        if last > 0 && self.lines[last - 1].1 == line {
            self.lines[last - 1] = (self.lines[last - 1].0 + 1, line);
        } else {
            self.lines.push((1, line));
        }
    }
}

pub struct Disassembler<'s> {
    pub gc: &'s Gc,
    pub chunk: &'s Chunk,
    pub stack: Option<&'s Vec<Value>>,
}

impl<'s> Disassembler<'s> {
    pub fn new(gc: &'s Gc, chunk: &'s Chunk, stack: Option<&'s Vec<Value>>) -> Self {
        Disassembler { gc, chunk, stack }
    }

    pub fn disassemble_to_string(&self, name: &str) -> String {
        let mut content: Vec<String> = Vec::new();
        content.push(format!("=== BEGIN {} ===", name));
        for (i, op) in self.chunk.code.iter().enumerate() {
            content.push(self.disassemble_instruction_to_string(op, i));
        }
        content.push(format!("===  END {}  ===\n\n", name));
        content.join("\n")
    }

    fn disassemble_instruction_to_string(&self, instruction: &OpCode, offset: usize) -> String {
        let mut string = String::new();
        string = format!("{}{:04} ", string, offset);
        let line = self.chunk.get_line(offset);
        if offset > 0 && line == self.chunk.get_line(offset - 1) {
            string = format!("{}   | ", string);
        } else {
            string = format!("{}{:>4} ", string, line);
        }
        match instruction {
            OpCode::Constant(value) => format!(
                "{}{}",
                string,
                self.const_instruction_to_string("OP_CONSTANT", *value)
            ),
            OpCode::DefineGlobal(value) => format!(
                "{}{}",
                string,
                self.const_instruction_to_string("OP_DEFINE_GLOBAL", *value)
            ),
            OpCode::GetGlobal(value) => format!(
                "{}{}",
                string,
                self.const_instruction_to_string("OP_GET_GLOBAL", *value)
            ),
            OpCode::SetGlobal(value) => format!(
                "{}{}",
                string,
                self.const_instruction_to_string("OP_SET_GLOBAL", *value)
            ),
            OpCode::GetLocal(value) => format!(
                "{}{}",
                string,
                self.value_instruction_to_string("OP_GET_LOCAL", *value)
            ),
            OpCode::SetLocal(value) => format!(
                "{}{}",
                string,
                self.value_instruction_to_string("OP_SET_LOCAL", *value)
            ),
            OpCode::GetUpvalue(value) => format!(
                "{}{}",
                string,
                self.value_instruction_to_string("OP_GET_UPVALUE", *value)
            ),
            OpCode::SetUpvalue(value) => format!(
                "{}{}",
                string,
                self.value_instruction_to_string("OP_SET_UPVALUE", *value)
            ),
            OpCode::GetProperty(value) => format!(
                "{}{}",
                string,
                self.const_instruction_to_string("OP_GET_PROPERTY", *value)
            ),
            OpCode::SetProperty(value) => format!(
                "{}{}",
                string,
                self.const_instruction_to_string("OP_SET_PROPERTY", *value)
            ),
            OpCode::Method(value) => format!(
                "{}{}",
                string,
                self.const_instruction_to_string("OP_METHOD", *value)
            ),
            OpCode::JumpIfFalse(value) => format!(
                "{}{}",
                string,
                self.value_instruction_to_string("OP_JUMP_IF_FALSE", *value)
            ),
            OpCode::Jump(value) => {
                format!(
                    "{}{}",
                    string,
                    self.value_instruction_to_string("OP_JUMP", *value)
                )
            }
            OpCode::Loop(value) => {
                format!(
                    "{}{}",
                    string,
                    self.value_instruction_to_string("OP_LOOP", *value)
                )
            }
            OpCode::Call(value) => format!("{}{:<16} {:4}", string, "OP_CALL", *value),
            OpCode::Closure(value) => {
                format!(
                    "{}{}",
                    string,
                    self.const_instruction_to_string("OP_CLOSURE", *value)
                )
            }
            OpCode::Class(value) => {
                format!(
                    "{}{}",
                    string,
                    self.const_instruction_to_string("OP_CLASS", *value)
                )
            }
            OpCode::Invoke((name, args)) => format!(
                "{}{}",
                string,
                self.invoke_instruction_to_string("OP_INVOKE", *name, *args)
            ),
            OpCode::SuperInvoke((name, args)) => format!(
                "{}{}",
                string,
                self.invoke_instruction_to_string("OP_SUPER_INVOKE", *name, *args)
            ),
            OpCode::GetSuper(value) => {
                format!(
                    "{}{}",
                    string,
                    self.const_instruction_to_string("OP_GET_SUPER", *value)
                )
            }
            OpCode::Return => format!("{}{}", string, "OP_RETURN"),
            OpCode::ReturnNil => format!("{}{}", string, "OP_RETURN_NIL"),
            OpCode::Negate => format!("{}{}", string, "OP_NEGATE"),
            OpCode::Add => format!("{}{}", string, "OP_ADD"),
            OpCode::Sub => format!("{}{}", string, "OP_SUB"),
            OpCode::Mul => format!("{}{}", string, "OP_MUL"),
            OpCode::Div => format!("{}{}", string, "OP_DIV"),
            OpCode::True => format!("{}{}", string, "OP_TRUE"),
            OpCode::False => format!("{}{}", string, "OP_FALSE"),
            OpCode::Nil => format!("{}{}", string, "OP_NIL"),
            OpCode::Not => format!("{}{}", string, "OP_NOT"),
            OpCode::Equal => format!("{}{}", string, "OP_EQUAL"),
            OpCode::NotEqual => format!("{}{}", string, "OP_NOT_EQUAL"),
            OpCode::Greater => format!("{}{}", string, "OP_GREATER"),
            OpCode::GreaterEqual => format!("{}{}", string, "OP_GREATER_EQUAL"),
            OpCode::Less => format!("{}{}", string, "OP_LESS"),
            OpCode::LessEqual => format!("{}{}", string, "OP_LESS_EQUAL"),
            OpCode::Print => format!("{}{}", string, "OP_PRINT"),
            OpCode::Pop => format!("{}{}", string, "OP_POP"),
            OpCode::CloseUpvalue => format!("{}{}", string, "OP_CLOSE_UPVALUE"),
            OpCode::Inherit => format!("{}{}", string, "OP_INHERIT"),
        }
    }

    fn const_instruction_to_string(&self, instruction: &str, index: usize) -> String {
        let value = self.chunk.get_constant(index);
        format!(
            "{:<16} {:4} {}",
            instruction,
            index,
            GcTraceFormatter::new(value, self.gc)
        )
    }

    fn value_instruction_to_string(&self, instruction: &str, index: usize) -> String {
        format!("{:<16} {:4}", instruction, index)
    }

    fn invoke_instruction_to_string(
        &self,
        instruction: &str,
        constant_index: usize,
        args: usize,
    ) -> String {
        let value = self.chunk.constants[constant_index as usize];
        format!(
            "{:<16} {:4} ({}) {}",
            instruction,
            constant_index,
            crate::gc::GcTraceFormatter::new(value, self.gc),
            args
        )
    }

    pub fn disassemble(&self, name: &str) {
        println!("=== BEGIN {} ===", name);
        for (i, op) in self.chunk.code.iter().enumerate() {
            self.disassemble_instruction(op, i);
        }
        println!("=== END {} ===\n", name);
    }

    pub fn disassemble_instruction(&self, instruction: &OpCode, offset: usize) {
        self.stack();
        print!("{:04} ", offset);
        let line = self.chunk.get_line(offset);
        if offset > 0 && line == self.chunk.get_line(offset - 1) {
            print!("   | ");
        } else {
            print!("{:>4} ", line);
        }
        match instruction {
            OpCode::Constant(value) => self.const_instruction("OP_CONSTANT", *value),
            OpCode::DefineGlobal(value) => self.const_instruction("OP_DEFINE_GLOBAL", *value),
            OpCode::GetGlobal(value) => self.const_instruction("OP_GET_GLOBAL", *value),
            OpCode::SetGlobal(value) => self.const_instruction("OP_SET_GLOBAL", *value),
            OpCode::GetLocal(value) => self.value_instruction("OP_GET_LOCAL", *value),
            OpCode::SetLocal(value) => self.value_instruction("OP_SET_LOCAL", *value),
            OpCode::GetUpvalue(value) => self.value_instruction("OP_GET_UPVALUE", *value),
            OpCode::SetUpvalue(value) => self.value_instruction("OP_SET_UPVALUE", *value),
            OpCode::GetProperty(value) => self.const_instruction("OP_GET_PROPERTY", *value),
            OpCode::SetProperty(value) => self.const_instruction("OP_SET_PROPERTY", *value),
            OpCode::Method(value) => self.const_instruction("OP_METHOD", *value),
            OpCode::JumpIfFalse(value) => self.value_instruction("OP_JUMP_IF_FALSE", *value),
            OpCode::Jump(value) => self.value_instruction("OP_JUMP", *value),
            OpCode::Loop(value) => self.value_instruction("OP_LOOP", *value),
            OpCode::Call(value) => println!("{:<16} {:4}", "OP_CALL", *value),
            OpCode::Closure(value) => self.const_instruction("OP_CLOSURE", *value),
            OpCode::Class(value) => self.const_instruction("OP_CLASS", *value),
            OpCode::Invoke((name, args)) => self.invoke_instruction("OP_INVOKE", *name, *args),
            OpCode::GetSuper(value) => self.const_instruction("OP_GET_SUPER", *value),
            OpCode::SuperInvoke((name, args)) => {
                self.invoke_instruction("OP_SUPER_INVOKE", *name, *args)
            }
            OpCode::Return => println!("OP_RETURN"),
            OpCode::ReturnNil => println!("OP_RETURN_NIL"),
            OpCode::Negate => println!("OP_NEGATE"),
            OpCode::Add => println!("OP_ADD"),
            OpCode::Sub => println!("OP_SUB"),
            OpCode::Mul => println!("OP_MUL"),
            OpCode::Div => println!("OP_DIV"),
            OpCode::True => println!("OP_TRUE"),
            OpCode::False => println!("OP_FALSE"),
            OpCode::Nil => println!("OP_NIL"),
            OpCode::Not => println!("OP_NOT"),
            OpCode::Equal => println!("OP_EQUAL"),
            OpCode::NotEqual => println!("OP_NOT_EQUAL"),
            OpCode::Greater => println!("OP_GREATER"),
            OpCode::GreaterEqual => println!("OP_GREATER_EQUAL"),
            OpCode::Less => println!("OP_LESS"),
            OpCode::LessEqual => println!("OP_LESS_EQUAL"),
            OpCode::Print => println!("OP_PRINT"),
            OpCode::Pop => println!("OP_POP"),
            OpCode::CloseUpvalue => println!("OP_CLOSE_UPVALUE"),
            OpCode::Inherit => println!("OP_INHERIT"),
        }
    }

    fn stack(&self) {
        if let Some(stack) = self.stack {
            print!("Stack: ");
            for &value in stack.iter() {
                print!("[{}]", crate::gc::GcTraceFormatter::new(value, self.gc));
            }
            println!();
        }
    }

    fn const_instruction(&self, instruction: &str, index: usize) {
        let value = self.chunk.get_constant(index);
        println!(
            "{:<16} {:4} {}",
            instruction,
            index,
            GcTraceFormatter::new(value, self.gc)
        )
    }

    fn value_instruction(&self, instruction: &str, index: usize) {
        println!("{:<16} {:4}", instruction, index)
    }

    fn invoke_instruction(&self, instruction: &str, constant_index: usize, args: usize) {
        let value = self.chunk.constants[constant_index as usize];
        println!(
            "{:<16} {:4} ({}) {}",
            instruction,
            constant_index,
            crate::gc::GcTraceFormatter::new(value, self.gc),
            args
        );
    }
}
