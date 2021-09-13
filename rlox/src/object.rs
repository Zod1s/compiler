use std::fmt;

// #[derive(Clone, PartialEq, Debug)]
// pub enum Object {
//     LoxString(LoxString),
// }

// impl Object {
//     pub fn string(st: LoxString) -> Object {
//         Object::LoxString(st)
//     }
// }

// impl fmt::Display for Object {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         match self {
//             Object::LoxString(st) => write!(f, "{}", st),
//         }
//     }
// }

#[derive(Clone, PartialEq, Debug, PartialOrd)]
pub struct LoxString {
    pub s: String,
}

impl LoxString {
    pub fn new(s: String) -> Self {
        LoxString { s }
    }
}

impl fmt::Display for LoxString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.s)
    }
}
