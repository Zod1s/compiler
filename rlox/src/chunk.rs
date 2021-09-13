use crate::types::Value;
use std::fmt;

#[derive(Clone, Copy, Debug)]
pub enum OpCode {
    Return,
    Constant(usize),
    Negate,
    Add,
    Sub,
    Mul,
    Div,
    True,
    False,
    Nil,
    Not,
    Equal,
    NotEqual,
    Greater,
    GreaterEqual,
    Less,
    LessEqual,
    Print,
    Pop,
    DefineGlobal(usize),
    GetGlobal(usize),
    SetGlobal(usize),
    GetLocal(usize),
    SetLocal(usize),
    JumpIfFalse(usize),
    Jump(usize),
    Loop(usize),
}

impl fmt::Display for OpCode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            OpCode::Return => write!(f, "OP_RETURN"),
            OpCode::Constant(i) => write!(f, "OP_CONSTANT {}", i),
            OpCode::Negate => write!(f, "OP_NEGATE"),
            OpCode::Add => write!(f, "OP_ADD"),
            OpCode::Sub => write!(f, "OP_SUB"),
            OpCode::Mul => write!(f, "OP_MUL"),
            OpCode::Div => write!(f, "OP_DIV"),
            OpCode::True => write!(f, "OP_TRUE"),
            OpCode::False => write!(f, "OP_FALSE"),
            OpCode::Nil => write!(f, "OP_NIL"),
            OpCode::Not => write!(f, "OP_NOT"),
            OpCode::Equal => write!(f, "OP_EQUAL"),
            OpCode::NotEqual => write!(f, "OP_NOT_EQUAL"),
            OpCode::Greater => write!(f, "OP_GREATER"),
            OpCode::GreaterEqual => write!(f, "OP_GREATER_EQUAL"),
            OpCode::Less => write!(f, "OP_LESS"),
            OpCode::LessEqual => write!(f, "OP_LESS_EQUAL"),
            OpCode::Print => write!(f, "OP_PRINT"),
            OpCode::Pop => write!(f, "OP_POP"),
            OpCode::DefineGlobal(i) => write!(f, "OP_DEFINE_GLOBAL {}", i),
            OpCode::GetGlobal(i) => write!(f, "OP_GET_GLOBAL {}", i),
            OpCode::SetGlobal(i) => write!(f, "OP_SET_GLOBAL {}", i),
            OpCode::GetLocal(i) => write!(f, "OP_GET_LOCAL {}", i),
            OpCode::SetLocal(i) => write!(f, "OP_SET_LOCAL {}", i),
            OpCode::JumpIfFalse(i) => write!(f, "OP_JUMP_IF_FALSE {}", i),
            OpCode::Jump(i) => write!(f, "OP_JUMP {}", i),
            OpCode::Loop(i) => write!(f, "OP_LOOP {}", i),
        }
    }
}

#[derive(Clone, Debug)]
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
    pub fn code(&self) -> Vec<OpCode> {
        self.code.clone()
    }

    #[inline]
    pub fn get_opcode(&self, index: usize) -> OpCode {
        self.code[index]
    }

    #[inline]
    pub fn lines(&self) -> Vec<(usize, usize)> {
        self.lines.clone()
    }

    #[inline]
    pub fn get_constant(&self, index: usize) -> Value {
        self.constants[index].clone()
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

pub fn disassemble_chunk(ch: &Chunk, name: &str) {
    println!("======== {} ========", name);

    for (i, c) in ch.code().iter().enumerate() {
        disassemble_instruction(ch, *c, i);
    }
    println!();
}

pub fn disassemble_instruction(ch: &Chunk, op: OpCode, index: usize) {
    print!("{:04} ", index);
    if index > 0 && ch.get_line(index) == ch.get_line(index - 1) {
        print!("   | ");
    } else {
        print!("{:4} ", ch.get_line(index));
    }
    match op {
        OpCode::Constant(i) => constant("OP_CONSTANT", ch, i),
        OpCode::DefineGlobal(i) => constant("OP_DEFINE_GLOBAL", ch, i),
        OpCode::GetGlobal(i) => constant("OP_GET_GLOBAL", ch, i),
        OpCode::SetGlobal(i) => constant("OP_SET_GLOBAL", ch, i),
        OpCode::GetLocal(i) => local("OP_GET_LOCAL", i),
        OpCode::SetLocal(i) => local("OP_SET_LOCAL", i),
        OpCode::JumpIfFalse(i) => local("OP_JUMP_IF_FALSE", i),
        OpCode::Jump(i) => local("OP_JUMP", i),
        OpCode::Loop(i) => local("OP_LOOP", i),
        // OpCode::Return => println!("{}", op),
        // OpCode::Negate => println!("{}", op),
        // OpCode::Add => println!("{}", op),
        // OpCode::Sub => println!("{}", op),
        // OpCode::Mul => println!("{}", op),
        // OpCode::Div => println!("{}", op),
        // OpCode::True => println!("OP_TRUE"),
        // OpCode::False => println!("OP_FALSE"),
        // OpCode::Nil => println!("OP_NIL"),
        // OpCode::Not => println!("OP_NOT"),
        // OpCode::Equal => println!("OP_EQUAL"),
        // OpCode::NotEqual => println!("OP_NOT_EQUAL"),
        // OpCode::Greater => println!("OP_GREATER"),
        // OpCode::GreaterEqual => println!("OP_GREATER_EQUAL"),
        // OpCode::Less => println!("OP_LESS"),
        // OpCode::LessEqual => println!("OP_LESS_EQUAL"),
        // OpCode::Print => println!("OP_PRINT"),
        _ => println!("{}", op), // _ => println!("Unknown opcode, {}", op),
    }
}

fn constant(name: &str, ch: &Chunk, index: usize) {
    println!("{:<16} {:4} '{}'", name, index, ch.get_constant(index));
}

fn local(name: &str, index: usize) {
    println!("{:<16} {:4}\n", name, index)
}

pub fn disassemble_instruction_str(ch: &Chunk, op: OpCode, index: usize) -> String {
    let mut content = format!("{:04} ", index);
    if index > 0 && ch.get_line(index) == ch.get_line(index - 1) {
        content = format!("{}   | ", content);
    } else if ch.get_line(index) > 1 {
        content = format!("\n{}{:4} ", content, ch.get_line(index));
    } else {
        content = format!("{}{:4} ", content, ch.get_line(index));
    }
    match op {
        OpCode::Constant(i) => format!("{}{}", content, constant_str("OP_CONSTANT", ch, i)),
        OpCode::DefineGlobal(i) => {
            format!("{}{}", content, constant_str("OP_DEFINE_GLOBAL", ch, i))
        }
        OpCode::GetGlobal(i) => format!("{}{}", content, constant_str("OP_GET_GLOBAL", ch, i)),
        OpCode::SetGlobal(i) => format!("{}{}", content, constant_str("OP_SET_GLOBAL", ch, i)),
        OpCode::GetLocal(i) => format!("{}{}", content, local_str("OP_GET_LOCAL", i)),
        OpCode::SetLocal(i) => format!("{}{}", content, local_str("OP_SET_LOCAL", i)),
        OpCode::JumpIfFalse(i) => format!("{}{}", content, local_str("OP_JUMP_IF_FALSE", i)),
        OpCode::Jump(i) => format!("{}{}", content, local_str("OP_JUMP", i)),
        OpCode::Loop(i) => format!("{}{}", content, local_str("OP_LOOP", i)),

        _ => format!("{}{}\n", content, op),
    }
}

fn constant_str(name: &str, ch: &Chunk, index: usize) -> String {
    format!("{:<16} {:4} '{}'\n", name, index, ch.get_constant(index))
}

fn local_str(name: &str, index: usize) -> String {
    format!("{:<16} {:4}\n", name, index)
}
