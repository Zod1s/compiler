use crate::expr::Expr;
use crate::token::Token;

#[derive(Clone, Debug, PartialEq)]
pub enum Stmt {
    Expression {
        expr: Expr,
    },
    Print {
        expr: Expr,
    },
    Var {
        name: Token,
        initializer: Box<Expr>,
    },
    Block {
        statements: Vec<Stmt>,
    },
    IfStmt {
        condition: Expr,
        then_branch: Box<Stmt>,
        else_branch: Box<Stmt>,
    },
    WhileStmt {
        condition: Expr,
        body: Box<Stmt>,
    },
    Null,
}

pub trait Visitor<T> {
    fn visit_expression_stmt(&mut self, stmt: &Stmt) -> T;
    fn visit_print_stmt(&mut self, stmt: &Stmt) -> T;
    fn visit_var_stmt(&mut self, stmt: &Stmt) -> T;
    fn visit_block_stmt(&mut self, stmt: &Stmt) -> T;
    fn visit_ifstmt_stmt(&mut self, stmt: &Stmt) -> T;
    fn visit_whilestmt_stmt(&mut self, stmt: &Stmt) -> T;
}

impl Stmt {
    pub fn accept<T>(&self, visitor: &mut impl Visitor<T>) -> T {
        match self {
            Stmt::Expression { .. } => visitor.visit_expression_stmt(self),
            Stmt::Print { .. } => visitor.visit_print_stmt(self),
            Stmt::Var { .. } => visitor.visit_var_stmt(self),
            Stmt::Block { .. } => visitor.visit_block_stmt(self),
            Stmt::IfStmt { .. } => visitor.visit_ifstmt_stmt(self),
            Stmt::WhileStmt { .. } => visitor.visit_whilestmt_stmt(self),
            Stmt::Null => panic!("calling visit on Stmt::Null"),
        }
    }
    #[inline]
    pub fn expression(expr: Expr) -> Stmt {
        Stmt::Expression { expr }
    }
    #[inline]
    pub fn print(expr: Expr) -> Stmt {
        Stmt::Print { expr }
    }
    #[inline]
    pub fn var(name: Token, initializer: Box<Expr>) -> Stmt {
        Stmt::Var { name, initializer }
    }
    #[inline]
    pub fn block(statements: Vec<Stmt>) -> Stmt {
        Stmt::Block { statements }
    }
    #[inline]
    pub fn ifstmt(condition: Expr, then_branch: Box<Stmt>, else_branch: Box<Stmt>) -> Stmt {
        Stmt::IfStmt {
            condition,
            then_branch,
            else_branch,
        }
    }
    #[inline]
    pub fn whilestmt(condition: Expr, body: Box<Stmt>) -> Stmt {
        Stmt::WhileStmt { condition, body }
    }
}
