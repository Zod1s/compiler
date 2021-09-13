use crate::expr::{Expr, Visitor};
use crate::token::{Literal, Token, TokenType};

#[derive(Clone, Copy)]
pub struct ASTPrinter {}

impl ASTPrinter {
    pub fn new() -> ASTPrinter {
        ASTPrinter {}
    }

    pub fn print(&self, expr: Expr) -> String {
        expr.accept(Box::new(*self))
    }

    fn parenthesize(&self, name: String, exprs: Vec<Expr>) -> String {
        let mut string = String::new();
        string.push('(');
        string.push_str(&name);
        for expr in exprs {
            string.push(' ');
            string.push_str(&expr.accept(Box::new(*self)));
        }
        string.push(')');

        string
    }
}

impl Visitor<String> for ASTPrinter {
    fn visit_binary_expr(&self, expr: &Expr) -> String {
        match expr {
            Expr::Binary {
                left,
                operator,
                right,
            } => self.parenthesize(operator.lexeme.clone(), vec![*left.clone(), *right.clone()]),
            _ => String::new(),
        }
    }
    fn visit_grouping_expr(&self, expr: &Expr) -> String {
        match expr {
            Expr::Grouping { expression } => {
                self.parenthesize("group".to_string(), vec![*expression.clone()])
            }
            _ => String::new(),
        }
    }
    fn visit_literal_expr(&self, expr: &Expr) -> String {
        match expr {
            Expr::Literal { value } => {
                if *value == Literal::Null {
                    String::from("Nil")
                } else {
                    value.to_string()
                }
            }
            _ => String::new(),
        }
    }
    fn visit_unary_expr(&self, expr: &Expr) -> String {
        match expr {
            Expr::Unary { operator, right } => {
                self.parenthesize(operator.lexeme.clone(), vec![*right.clone()])
            }
            _ => String::new(),
        }
    }
}

pub fn test() {
    let expr = Expr::binary(
        Box::new(Expr::unary(
            Token::new(TokenType::Minus, 1, "-".to_string(), Literal::Symbol),
            Box::new(Expr::literal(Literal::Number(123.0))),
        )),
        Token::new(TokenType::Star, 1, "*".to_string(), Literal::Symbol),
        Box::new(Expr::grouping(Box::new(Expr::literal(Literal::Number(
            45.67,
        ))))),
    );
    let ast = ASTPrinter::new();
    println!("{}", ast.print(expr));
}
