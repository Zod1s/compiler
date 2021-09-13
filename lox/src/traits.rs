use crate::expr::Expr;
use crate::function::Function;
use crate::interpreter::{Interpreter, InterpreterError, LoxRuntime};
use crate::token::{Literal, LoxTypes, Token};
use std::time::{SystemTime, UNIX_EPOCH};

pub trait LoxCallable {
    fn fcall(
        &self,
        interpreter: &Interpreter,
        arguments: Vec<LoxTypes>,
        paren: Token,
    ) -> LoxRuntime;
    fn arity(&self, paren: Token) -> Result<usize, InterpreterError>;
}

impl LoxCallable for Expr {
    fn fcall(
        &self,
        interpreter: &Interpreter,
        arguments: Vec<LoxTypes>,
        paren: Token,
    ) -> LoxRuntime {
        match self {
            _ => InterpreterError::error(paren, "can only call functions and classes.".to_string()),
        }
    }

    fn arity(&self, paren: Token) -> Result<usize, InterpreterError> {
        match self {
            _ => Err(InterpreterError::new(
                paren,
                "can only call functions and classes.".to_string(),
            )),
        }
    }
}

impl LoxCallable for Function {
    fn fcall(
        &self,
        interpreter: &Interpreter,
        arguments: Vec<LoxTypes>,
        paren: Token,
    ) -> LoxRuntime {
        match self {
            Function::Clock {} => Ok(LoxTypes::Object(Literal::Number(
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .expect("Time went backwards")
                    .as_millis() as f64
                    / 1000.0,
            ))),
            // _ => Err(InterpreterError::new(
            //     paren,
            //     "can only call functions and classes.".to_string(),
            // )),
        }
    }

    fn arity(&self, paren: Token) -> Result<usize, InterpreterError> {
        match self {
            Function::Clock {} => Ok(0),
            // _ => Err(InterpreterError::new(
            //     paren,
            //     "can only call functions and classes.".to_string(),
            // )),
        }
    }
}
