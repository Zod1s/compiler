use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum Function {
    Clock {},
}

impl Function {
    pub fn clock() -> Function {
        Function::Clock {}
    }
}

impl fmt::Display for Function {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Function::Clock {} => write!(f, "<native function>"),
        }
    }
}
