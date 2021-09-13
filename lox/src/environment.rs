use crate::interpreter::{InterpreterError, LoxRuntime};
use crate::token::{Literal, LoxTypes, Token};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Environment {
    values: HashMap<String, LoxTypes>,
    enclosing: Option<Box<Environment>>,
}

impl Environment {
    pub fn new() -> Environment {
        Environment {
            values: HashMap::new(),
            enclosing: None,
        }
    }

    pub fn new_with_enclosing(enclosing: Box<Environment>) -> Environment {
        Environment {
            values: HashMap::new(),
            enclosing: Some(enclosing),
        }
    }

    pub fn values(&self) -> HashMap<String, LoxTypes> {
        self.values.clone()
    }

    pub fn enclosing(&self) -> HashMap<String, LoxTypes> {
        match &self.enclosing {
            Some(encl) => encl.values(),
            None => panic!("No enclosing environment found."),
        }
    }

    pub fn upper_env(self) -> Environment {
        match self.enclosing {
            Some(encl) => *encl,
            None => self,
        }
    }

    pub fn define(&mut self, name: String, value: LoxTypes) {
        self.values.insert(name, value);
    }

    pub fn get(&self, name: Token) -> LoxRuntime {
        match self.values.get(&name.lexeme) {
            Some(value) => Ok(value.clone()),
            None => match &self.enclosing {
                Some(encl) => encl.get(name),
                None => InterpreterError::error(
                    name.clone(),
                    format!("undefined variable '{}'.", name.lexeme),
                ),
            },
        }
    }

    pub fn get_global(&self, name: Token) -> LoxRuntime {
        match &self.enclosing {
            Some(encl) => encl.get_global(name),
            None => self.get(name)
        }
    }

    pub fn assign(&mut self, name: Token, value: LoxTypes) -> LoxRuntime {
        if self.values.contains_key(&name.lexeme) {
            self.values.insert(name.lexeme, value);
            Ok(LoxTypes::Object(Literal::Null))
        } else {
            match &mut self.enclosing {
                Some(encl) => encl.assign(name, value),
                None => InterpreterError::error(
                    name.clone(),
                    format!("undefined variable '{}'.", name.lexeme),
                ),
            }
        }
    }
}
