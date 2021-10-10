use crate::{
    gc::{Gc, GcTraceFormatter},
    types::Value,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum OpCode {
    Add,
    BuildList(usize),
    Call(usize),
    Class(usize),
    CloseUpvalue,
    Closure(usize),
    Constant(usize),
    // Decrement,
    DecrementGlobal(usize),
    DecrementLocal(usize),
    DecrementUpvalue(usize),
    DefineGlobal(usize),
    Div,
    Equal,
    False,
    GetIndexArray,
    GetGlobal(usize),
    GetLocal(usize),
    GetProperty(usize),
    GetSuper(usize),
    GetUpvalue(usize),
    Greater,
    GreaterEqual,
    // Increment,
    IncrementGlobal(usize),
    IncrementLocal(usize),
    IncrementUpvalue(usize),
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
    Rem,
    Return,
    ReturnNil,
    SetIndexArray,
    SetGlobal(usize),
    SetLocal(usize),
    SetProperty(usize),
    SetUpvalue(usize),
    Sub,
    SuperInvoke((usize, usize)),
    True,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Chunk {
    pub code: Vec<OpCode>,
    pub constants: Vec<Value>,
    lines: Vec<(usize, usize)>, // repetition, line
}

impl Chunk {
    pub fn new() -> Self {
        Chunk {
            code: Vec::new(),
            constants: Vec::new(),
            lines: Vec::new(),
        }
    }

    #[inline]
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
        self.constants.push(value);
        self.constants.len() - 1
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
        let mut content = vec![String::new()];
        let mut length = 0;
        for (i, op) in self.chunk.code.iter().enumerate() {
            let line = self.disassemble_instruction_to_string(op, i);
            length = length.max(line.len());
            content.push(line);
        }
        length -= 8 + name.len();
        let half = length / 2;
        let begin_space = "=".repeat(half);
        let end_space = "=".repeat(length - half);
        content[0] = format!("{} BEGIN {} {}", begin_space, name, end_space);
        content.push(format!("{}  END {}  {}\n\n", begin_space, name, end_space));
        content.join("\n")
    }

    fn disassemble_instruction_to_string(&self, instruction: &OpCode, offset: usize) -> String {
        let mut content = vec![format!("{:04} ", offset)];
        let line = self.chunk.get_line(offset);
        if offset > 0 && line == self.chunk.get_line(offset - 1) {
            content.push("   | ".to_owned());
        } else {
            content.push(format!("{:>4} ", line));
        }
        let instr = match instruction {
            OpCode::BuildList(value) => self.const_instruction_to_string("OP_BUILD_LIST", *value),
            OpCode::Constant(value) => self.const_instruction_to_string("OP_CONSTANT", *value),
            OpCode::DefineGlobal(value) => {
                self.const_instruction_to_string("OP_DEFINE_GLOBAL", *value)
            }
            OpCode::GetGlobal(value) => self.const_instruction_to_string("OP_GET_GLOBAL", *value),
            OpCode::SetGlobal(value) => self.const_instruction_to_string("OP_SET_GLOBAL", *value),
            OpCode::GetLocal(value) => self.value_instruction_to_string("OP_GET_LOCAL", *value),
            OpCode::SetLocal(value) => self.value_instruction_to_string("OP_SET_LOCAL", *value),
            OpCode::GetUpvalue(value) => self.value_instruction_to_string("OP_GET_UPVALUE", *value),
            OpCode::SetUpvalue(value) => self.value_instruction_to_string("OP_SET_UPVALUE", *value),
            OpCode::GetProperty(value) => {
                self.const_instruction_to_string("OP_GET_PROPERTY", *value)
            }
            OpCode::SetProperty(value) => {
                self.const_instruction_to_string("OP_SET_PROPERTY", *value)
            }
            OpCode::Method(value) => self.const_instruction_to_string("OP_METHOD", *value),
            OpCode::JumpIfFalse(value) => {
                self.value_instruction_to_string("OP_JUMP_IF_FALSE", *value)
            }
            OpCode::Jump(value) => self.value_instruction_to_string("OP_JUMP", *value),
            OpCode::Loop(value) => self.value_instruction_to_string("OP_LOOP", *value),
            OpCode::Call(value) => format!("{:<16} {:4}", "OP_CALL", *value),
            OpCode::Closure(value) => self.const_instruction_to_string("OP_CLOSURE", *value),
            OpCode::Class(value) => self.const_instruction_to_string("OP_CLASS", *value),
            OpCode::Invoke((name, args)) => {
                self.invoke_instruction_to_string("OP_INVOKE", *name, *args)
            }
            OpCode::SuperInvoke((name, args)) => {
                self.invoke_instruction_to_string("OP_SUPER_INVOKE", *name, *args)
            }
            OpCode::GetSuper(value) => self.const_instruction_to_string("OP_GET_SUPER", *value),
            OpCode::IncrementGlobal(value) => {
                self.value_instruction_to_string("OP_INCREMENT_GLOBAL", *value)
            }
            OpCode::IncrementLocal(value) => {
                self.value_instruction_to_string("OP_INCREMENT_LOCAL", *value)
            }
            OpCode::IncrementUpvalue(value) => {
                self.value_instruction_to_string("OP_INCREMENT_UPVALUE", *value)
            }
            OpCode::DecrementGlobal(value) => {
                self.value_instruction_to_string("OP_DECREMENT_GLOBAL", *value)
            }
            OpCode::DecrementLocal(value) => {
                self.value_instruction_to_string("OP_DECREMENT_LOCAL", *value)
            }
            OpCode::DecrementUpvalue(value) => {
                self.value_instruction_to_string("OP_DECREMENT_UPVALUE", *value)
            }

            OpCode::Return => String::from("OP_RETURN"),
            // OpCode::Increment => String::from("OP_INCREMENT"),
            // OpCode::Decrement => String::from("OP_DECREMENT"),
            OpCode::ReturnNil => String::from("OP_RETURN_NIL"),
            OpCode::Negate => String::from("OP_NEGATE"),
            OpCode::Add => String::from("OP_ADD"),
            OpCode::Sub => String::from("OP_SUB"),
            OpCode::Rem => String::from("OP_REM"),
            OpCode::Mul => String::from("OP_MUL"),
            OpCode::Div => String::from("OP_DIV"),
            OpCode::True => String::from("OP_TRUE"),
            OpCode::False => String::from("OP_FALSE"),
            OpCode::Nil => String::from("OP_NIL"),
            OpCode::Not => String::from("OP_NOT"),
            OpCode::Equal => String::from("OP_EQUAL"),
            OpCode::NotEqual => String::from("OP_NOT_EQUAL"),
            OpCode::Greater => String::from("OP_GREATER"),
            OpCode::GreaterEqual => String::from("OP_GREATER_EQUAL"),
            OpCode::Less => String::from("OP_LESS"),
            OpCode::LessEqual => String::from("OP_LESS_EQUAL"),
            OpCode::Print => String::from("OP_PRINT"),
            OpCode::Pop => String::from("OP_POP"),
            OpCode::CloseUpvalue => String::from("OP_CLOSE_UPVALUE"),
            OpCode::Inherit => String::from("OP_INHERIT"),
            OpCode::GetIndexArray => String::from("OP_GET_INDEX_ARRAY"),
            OpCode::SetIndexArray => String::from("OP_SET_INDEX_ARRAY"),
        };
        content.push(instr);
        content.join(" ")
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

    fn invoke_instruction_to_string(&self, instr: &str, index: usize, args: usize) -> String {
        let value = self.chunk.constants[index as usize];
        format!(
            "{:<16} {:4} ({}) {}",
            instr,
            index,
            crate::gc::GcTraceFormatter::new(value, self.gc),
            args
        )
    }

    pub fn disassemble(&self, name: &str) {
        println!("{}", self.disassemble_to_string(name));
    }

    pub fn disassemble_instruction(&self, instruction: &OpCode, offset: usize) {
        self.stack();
        println!(
            "{}",
            self.disassemble_instruction_to_string(instruction, offset)
        );
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
}
