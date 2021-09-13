use crate::environment::Environment;
use crate::expr::{self as ex, Expr};
use crate::stmt::{self, Stmt};
use crate::token::{Literal, LoxTypes, Token, TokenType::*};
use crate::traits::LoxCallable;
use crate::LoxError;
// use lazy_static::lazy_static;

// 10.2.1

#[derive(Debug)]
pub struct InterpreterError {
    operator: Token,
    message: String,
}

impl InterpreterError {
    pub fn new(operator: Token, message: String) -> InterpreterError {
        InterpreterError { operator, message }
    }

    #[inline]
    pub fn error(token: Token, message: String) -> LoxRuntime {
        Err(InterpreterError::new(token, message))
    }

    pub fn message(&self) -> String {
        self.message.clone()
    }

    pub fn operator(&self) -> Token {
        self.operator.clone()
    }
}

pub type LoxRuntime = Result<LoxTypes, InterpreterError>;

#[derive(Clone, Debug)]
pub struct Interpreter {
    error: LoxError,
    environment: Environment,
}

impl Interpreter {
    pub fn new(error: LoxError) -> Interpreter {
        Interpreter {
            error,
            environment: Environment::new(),
        }
    }

    pub fn new_with_env(error: LoxError, environment: Environment) -> Interpreter {
        Interpreter { error, environment }
    }

    pub fn interpret(&mut self, stmts: Vec<Stmt>) {
        for stmt in stmts {
            self.execute(stmt);
            if self.error == LoxError::RuntimeError {
                break;
            }
        }
    }

    pub fn had_error(&self) -> bool {
        self.error == LoxError::RuntimeError
    }

    fn evaluate(&mut self, expr: &Expr) -> LoxRuntime {
        expr.accept(self)
    }

    fn execute(&mut self, stmt: Stmt) {
        stmt.accept(self);
    }

    fn is_true(obj: Literal) -> bool {
        match obj {
            Literal::Boolean(b) => b,
            Literal::Null => false,
            _ => true,
        }
    }

    fn is_true_object(obj: LoxTypes) -> bool {
        match obj {
            LoxTypes::Object(obj) => Interpreter::is_true(obj),
        }
    }

    fn logic_negate(obj: Literal) -> Literal {
        match obj {
            Literal::Boolean(b) => Literal::Boolean(!b),
            Literal::Null => Literal::lit_true(),
            _ => Literal::lit_false(),
        }
    }

    fn execute_block(&mut self, stmts: Vec<Stmt>, env: Environment) {
        let previous = self.environment.clone();
        self.environment = env;
        for stmt in stmts {
            self.execute(stmt);
            if self.error == LoxError::RuntimeError {
                break;
            }
        }
        self.environment.enclosing().extend(previous.values());
        self.environment = self.environment.clone().upper_env();
    }
}

impl ex::Visitor<LoxRuntime> for Interpreter {
    fn visit_literal_expr(&mut self, expr: &Expr) -> LoxRuntime {
        match expr {
            Expr::Literal { value } => Ok(LoxTypes::Object(value.clone())),
            _ => panic!("Unexpected value in interpreting literal expression."), // should be unreachable
        }
    }

    fn visit_grouping_expr(&mut self, expr: &Expr) -> LoxRuntime {
        match expr {
            Expr::Grouping { expression } => self.evaluate(expression),
            _ => panic!("Unexpected value in interpreting grouping expression."), // should be unreachable
        }
    }

    fn visit_unary_expr(&mut self, expr: &Expr) -> LoxRuntime {
        match expr {
            Expr::Unary { operator, right } => {
                let LoxTypes::Object(r) = self.evaluate(right)?;
                match operator.token_type {
                    Minus => {
                        if let Literal::Number(n) = r {
                            Ok(LoxTypes::Object(Literal::Number(-n)))
                        } else {
                            InterpreterError::error(
                                operator.clone(),
                                format!("operand must be a number, found {}.", r.literal_type()),
                            )
                        }
                    }
                    Bang => Ok(LoxTypes::Object(Interpreter::logic_negate(r))),
                    _ => panic!("Non-unary operator found in unary expression."), // should be unreachable
                }
            }
            _ => panic!("Unexpected value in interpreting unary expression."), // should be unreachable
        }
    }

    fn visit_binary_expr(&mut self, expr: &Expr) -> LoxRuntime {
        match expr {
            Expr::Binary {
                left,
                operator,
                right,
            } => {
                let LoxTypes::Object(l) = self.evaluate(left)?;
                let LoxTypes::Object(r) = self.evaluate(right)?;

                match operator.token_type {
                    Minus => {
                        if let Literal::Number(ln) = l {
                            if let Literal::Number(rn) = r {
                                Ok(LoxTypes::Object(Literal::Number(ln - rn)))
                            } else {
                                InterpreterError::error(
                                    operator.clone(),
                                    format!(
                                        "both operands must be numbers, instead found {} and {}.",
                                        l.literal_type(),
                                        r.literal_type()
                                    ),
                                )
                            }
                        } else {
                            InterpreterError::error(
                                operator.clone(),
                                format!(
                                    "both operands must be numbers, instead found {} and {}.",
                                    l.literal_type(),
                                    r.literal_type()
                                ),
                            )
                        }
                    }
                    Slash => {
                        if let Literal::Number(ln) = l {
                            if let Literal::Number(rn) = r {
                                Ok(LoxTypes::Object(Literal::Number(ln / rn)))
                            } else {
                                InterpreterError::error(
                                    operator.clone(),
                                    format!(
                                        "both operands must be numbers, instead found {} and {}.",
                                        l.literal_type(),
                                        r.literal_type()
                                    ),
                                )
                            }
                        } else {
                            InterpreterError::error(
                                operator.clone(),
                                format!(
                                    "both operands must be numbers, instead found {} and {}.",
                                    l.literal_type(),
                                    r.literal_type()
                                ),
                            )
                        }
                    }
                    Star => {
                        if let Literal::Number(ln) = l {
                            if let Literal::Number(rn) = r {
                                Ok(LoxTypes::Object(Literal::Number(ln * rn)))
                            } else {
                                InterpreterError::error(
                                    operator.clone(),
                                    format!(
                                        "both operands must be numbers, instead found {} and {}.",
                                        l.literal_type(),
                                        r.literal_type()
                                    ),
                                )
                            }
                        } else {
                            InterpreterError::error(
                                operator.clone(),
                                format!(
                                    "both operands must be numbers, instead found {} and {}.",
                                    l.literal_type(),
                                    r.literal_type()
                                ),
                            )
                        }
                    }
                    Plus => {
                        if let Literal::Number(ln) = l {
                            if let Literal::Number(rn) = r {
                                Ok(LoxTypes::Object(Literal::Number(ln + rn)))
                            } else if let Literal::LString(rs) = r {
                                Ok(LoxTypes::Object(Literal::LString(format!("{}{}", ln, rs))))
                            } else {
                                InterpreterError::error(
                                    operator.clone(),
                                    format!(
                                        "plus sign operands must be numbers or strings, found {} on left and {} on right.",
                                        l.literal_type(),
                                        r.literal_type()
                                    ),
                                )
                            }
                        } else if let Literal::LString(ls) = l.clone() {
                            if let Literal::LString(rs) = r {
                                Ok(LoxTypes::Object(Literal::LString(format!("{}{}", ls, rs))))
                            } else if let Literal::Number(rn) = r {
                                Ok(LoxTypes::Object(Literal::LString(format!("{}{}", ls, rn))))
                            } else {
                                InterpreterError::error(
                                    operator.clone(),
                                    format!(
                                        "plus sign operands must be numbers or strings, found {} on left and {} on right.",
                                        l.literal_type(),
                                        r.literal_type()
                                    ),
                                )
                            }
                        } else {
                            InterpreterError::error(
                                operator.clone(),
                                format!(
                                    "plus sign operands must be numbers or strings, found {} on left and {} on right.",
                                    l.literal_type(),
                                    r.literal_type()
                                ),
                            )
                        }
                    }
                    Greater => {
                        if let Literal::Number(ln) = l {
                            if let Literal::Number(rn) = r {
                                Ok(LoxTypes::Object(Literal::Boolean(ln > rn)))
                            } else {
                                InterpreterError::error(
                                    operator.clone(),
                                    format!(
                                        "both operands must be of the same type, found {} and {}.",
                                        l.literal_type(),
                                        r.literal_type()
                                    ),
                                )
                            }
                        } else if let Literal::LString(ls) = l.clone() {
                            if let Literal::LString(rs) = r {
                                Ok(LoxTypes::Object(Literal::Boolean(ls > rs)))
                            } else {
                                InterpreterError::error(
                                    operator.clone(),
                                    format!(
                                        "both operands must be of the same type, found {} and {}.",
                                        l.literal_type(),
                                        r.literal_type()
                                    ),
                                )
                            }
                        } else {
                            InterpreterError::error(
                                operator.clone(),
                                format!(
                                    "can compare only numbers with numbers or strings with strings, found {} and {}.",
                                    l.literal_type(),
                                    r.literal_type()
                                ),
                            )
                        }
                    }
                    Less => {
                        if let Literal::Number(ln) = l {
                            if let Literal::Number(rn) = r {
                                Ok(LoxTypes::Object(Literal::Boolean(ln < rn)))
                            } else {
                                InterpreterError::error(
                                    operator.clone(),
                                    format!(
                                        "both operands must be of the same type, found {} and {}.",
                                        l.literal_type(),
                                        r.literal_type()
                                    ),
                                )
                            }
                        } else if let Literal::LString(ls) = l.clone() {
                            if let Literal::LString(rs) = r {
                                Ok(LoxTypes::Object(Literal::Boolean(ls < rs)))
                            } else {
                                InterpreterError::error(
                                    operator.clone(),
                                    format!(
                                        "both operands must be of the same type, found {} and {}.",
                                        l.literal_type(),
                                        r.literal_type()
                                    ),
                                )
                            }
                        } else {
                            InterpreterError::error(
                                operator.clone(),
                                format!(
                                    "can compare only numbers with numbers or strings with strings, found {} and {}.",
                                    l.literal_type(),
                                    r.literal_type()
                                ),
                            )
                        }
                    }
                    GreaterEqual => {
                        if let Literal::Number(ln) = l {
                            if let Literal::Number(rn) = r {
                                Ok(LoxTypes::Object(Literal::Boolean(ln >= rn)))
                            } else {
                                InterpreterError::error(
                                    operator.clone(),
                                    format!(
                                        "both operands must be of the same type, found {} and {}.",
                                        l.literal_type(),
                                        r.literal_type()
                                    ),
                                )
                            }
                        } else if let Literal::LString(ls) = l.clone() {
                            if let Literal::LString(rs) = r {
                                Ok(LoxTypes::Object(Literal::Boolean(ls >= rs)))
                            } else {
                                InterpreterError::error(
                                    operator.clone(),
                                    format!(
                                        "both operands must be of the same type, found {} and {}.",
                                        l.literal_type(),
                                        r.literal_type()
                                    ),
                                )
                            }
                        } else {
                            InterpreterError::error(
                                operator.clone(),
                                format!(
                                    "can compare only numbers with numbers or strings with strings, found {} and {}.",
                                    l.literal_type(),
                                    r.literal_type()
                                ),
                            )
                        }
                    }
                    LessEqual => {
                        if let Literal::Number(ln) = l {
                            if let Literal::Number(rn) = r {
                                Ok(LoxTypes::Object(Literal::Boolean(ln <= rn)))
                            } else {
                                InterpreterError::error(
                                    operator.clone(),
                                    format!(
                                        "both operands must be of the same type, found {} and {}.",
                                        l.literal_type(),
                                        r.literal_type()
                                    ),
                                )
                            }
                        } else if let Literal::LString(ls) = l.clone() {
                            if let Literal::LString(rs) = r {
                                Ok(LoxTypes::Object(Literal::Boolean(ls <= rs)))
                            } else {
                                InterpreterError::error(
                                    operator.clone(),
                                    format!(
                                        "both operands must be of the same type, found {} and {}.",
                                        l.literal_type(),
                                        r.literal_type()
                                    ),
                                )
                            }
                        } else {
                            InterpreterError::error(
                                operator.clone(),
                                format!(
                                    "can compare only numbers with numbers or strings with strings, found {} and {}.",
                                    l.literal_type(),
                                    r.literal_type()
                                ),
                            )
                        }
                    }
                    BangEqual => Ok(LoxTypes::Object(Literal::Boolean(l != r))),
                    EqualEqual => Ok(LoxTypes::Object(Literal::Boolean(l == r))),
                    _ => panic!("Non-binary operator found in binary expression."), // should be unreachable
                }
            }
            _ => panic!("Unexpected value in interpreting binary expression."), // should be unreachable
        }
    }

    fn visit_variable_expr(&mut self, expr: &Expr) -> LoxRuntime {
        match expr {
            Expr::Variable { name } => self.environment.get(name.clone()),
            _ => panic!("Unexpected value in interpreting variable expression."), // should be unreachable
        }
    }

    fn visit_assign_expr(&mut self, expr: &Expr) -> LoxRuntime {
        match expr {
            Expr::Assign { name, value } => {
                let new_value = self.evaluate(value)?;
                self.environment.assign(name.clone(), new_value.clone())?;
                Ok(new_value)
            }
            _ => panic!("Unexpected value in interpreting assign expression."), // should be unreachable
        }
    }

    fn visit_logical_expr(&mut self, expr: &Expr) -> LoxRuntime {
        match expr {
            Expr::Logical {
                left,
                operator,
                right,
            } => {
                let l = self.evaluate(left)?;
                if (operator.token_type == Or && Interpreter::is_true_object(l.clone()))
                    || !Interpreter::is_true_object(l.clone())
                {
                    return Ok(l);
                }
                self.evaluate(right)
            }
            _ => panic!("Unexpected value in interpreting logical expression."), // should be unreachable
        }
    }

    fn visit_call_expr(&mut self, expr: &Expr) -> LoxRuntime {
        match expr {
            Expr::Call {
                callee,
                paren,
                arguments,
            } => {
                let call = self.evaluate(callee)?;

                let mut args: Vec<LoxTypes> = Vec::new();

                for arg in arguments {
                    args.push(self.evaluate(arg)?);
                }

                if args.len() != callee.arity(paren.clone())? {
                    return InterpreterError::error(
                        paren.clone(),
                        format!(
                            "expected {} arguments, found {}.",
                            callee.arity(paren.clone())?,
                            args.len()
                        ),
                    );
                }

                callee.fcall(self, args, paren.clone())
            }
            _ => panic!("Unexpected value in interpreting call expression."), // should be unreachable
        }
    }
}

impl stmt::Visitor<()> for Interpreter {
    fn visit_expression_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Expression { expr } => match self.evaluate(expr) {
                Ok(_) => (),
                Err(err) => {
                    LoxError::runtime_error(err);
                    self.error = LoxError::RuntimeError;
                }
            },
            _ => panic!("Unexpected value in evaluating expression statement."), // should be unreachable
        }
    }

    fn visit_print_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Print { expr } => match self.evaluate(expr) {
                Ok(value) => println!("{}", value),
                Err(err) => {
                    LoxError::runtime_error(err);
                    self.error = LoxError::RuntimeError;
                }
            },
            _ => panic!("Unexpected value in evaluating print statement."), // should be unreachable
        }
    }

    fn visit_var_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Var { name, initializer } => {
                let mut value = LoxTypes::Object(Literal::Null);
                if **initializer != Expr::Null {
                    match self.evaluate(initializer) {
                        Ok(val) => value = val,
                        Err(err) => {
                            LoxError::runtime_error(err);
                            self.error = LoxError::RuntimeError;
                        }
                    }
                }

                self.environment.define(name.lexeme.clone(), value);
            }
            _ => panic!("Unexpected value in evaluating var statement."), // should be unreachable
        }
    }

    fn visit_block_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Block { statements } => {
                self.execute_block(
                    statements.clone(),
                    Environment::new_with_enclosing(Box::new(self.environment.clone())),
                );
            }
            _ => panic!("Unexpected value in evaluating var statement."), // should be unreachable
        }
    }

    fn visit_ifstmt_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::IfStmt {
                condition,
                then_branch,
                else_branch,
            } => match self.evaluate(condition) {
                Ok(val) => {
                    if Interpreter::is_true_object(val) {
                        self.execute(*then_branch.clone());
                    } else if **else_branch != Stmt::Null {
                        self.execute(*else_branch.clone());
                    }
                }
                Err(err) => {
                    LoxError::runtime_error(err);
                    self.error = LoxError::RuntimeError;
                }
            },
            _ => panic!("Unexpected value in evaluating if statement."), // should be unreachable
        }
    }

    fn visit_whilestmt_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::WhileStmt { condition, body } => loop {
                match self.evaluate(condition) {
                    Ok(val) => {
                        if !Interpreter::is_true_object(val) {
                            break;
                        }
                        self.execute(*body.clone())
                    }
                    Err(err) => {
                        LoxError::runtime_error(err);
                        self.error = LoxError::RuntimeError;
                    }
                }
            },
            _ => panic!("Unexpected value in evaluating while statement."), // should be unreachable
        }
    }
}
