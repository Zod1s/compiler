use crate::token::{Literal, Token};

#[derive(Clone, Debug, PartialEq)]
pub enum Expr {
    Binary {
        left: Box<Expr>,
        operator: Token,
        right: Box<Expr>,
    },
    Grouping {
        expression: Box<Expr>,
    },
    Literal {
        value: Literal,
    },
    Unary {
        operator: Token,
        right: Box<Expr>,
    },
    Variable {
        name: Token,
    },
    Assign {
        name: Token,
        value: Box<Expr>,
    },
    Logical {
        left: Box<Expr>,
        operator: Token,
        right: Box<Expr>,
    },
    Call {
        callee: Box<Expr>,
        paren: Token,
        arguments: Vec<Expr>,
    },
    Null,
}

pub trait Visitor<T> {
    fn visit_binary_expr(&mut self, expr: &Expr) -> T;
    fn visit_grouping_expr(&mut self, expr: &Expr) -> T;
    fn visit_literal_expr(&mut self, expr: &Expr) -> T;
    fn visit_unary_expr(&mut self, expr: &Expr) -> T;
    fn visit_variable_expr(&mut self, expr: &Expr) -> T;
    fn visit_assign_expr(&mut self, expr: &Expr) -> T;
    fn visit_logical_expr(&mut self, expr: &Expr) -> T;
    fn visit_call_expr(&mut self, expr: &Expr) -> T;
}

impl Expr {
    pub fn accept<T>(&self, visitor: &mut impl Visitor<T>) -> T {
        match self {
            Expr::Binary { .. } => visitor.visit_binary_expr(self),
            Expr::Grouping { .. } => visitor.visit_grouping_expr(self),
            Expr::Literal { .. } => visitor.visit_literal_expr(self),
            Expr::Unary { .. } => visitor.visit_unary_expr(self),
            Expr::Variable { .. } => visitor.visit_variable_expr(self),
            Expr::Assign { .. } => visitor.visit_assign_expr(self),
            Expr::Logical { .. } => visitor.visit_logical_expr(self),
            Expr::Call { .. } => visitor.visit_call_expr(self),
            Expr::Null => panic!("calling visit on Expr::Null"),
        }
    }
    #[inline]
    pub fn binary(left: Box<Expr>, operator: Token, right: Box<Expr>) -> Expr {
        Expr::Binary {
            left,
            operator,
            right,
        }
    }
    #[inline]
    pub fn grouping(expression: Box<Expr>) -> Expr {
        Expr::Grouping { expression }
    }
    #[inline]
    pub fn literal(value: Literal) -> Expr {
        Expr::Literal { value }
    }
    #[inline]
    pub fn unary(operator: Token, right: Box<Expr>) -> Expr {
        Expr::Unary { operator, right }
    }
    #[inline]
    pub fn variable(name: Token) -> Expr {
        Expr::Variable { name }
    }
    #[inline]
    pub fn assign(name: Token, value: Box<Expr>) -> Expr {
        Expr::Assign { name, value }
    }
    #[inline]
    pub fn logical(left: Box<Expr>, operator: Token, right: Box<Expr>) -> Expr {
        Expr::Logical {
            left,
            operator,
            right,
        }
    }
    #[inline]
    pub fn call(callee: Box<Expr>, paren: Token, arguments: Vec<Expr>) -> Expr {
        Expr::Call {
            callee,
            paren,
            arguments,
        }
    }
}
