use crate::expr::Expr;
use crate::stmt::Stmt;
use crate::token::{Literal, Token, TokenType, TokenType::*};
use crate::LoxError;

/** Full grammar
 * program        → declaration* EOF ;
 * declaration    → varDecl
 *                | statement ;
 * varDecl        → "var" IDENTIFIER ( "=" expression )? ";" ;
 * statement      → ifStmt
 *                | block
 *                | printStmt
 *                | whileStmt
 *                | forStmt
 *                | exprStmt ;
 * ifStmt         → "if" "(" expression ")" statement
 *                ( "else" statement )? ;
 * block          → "{" declaration* "}" ;
 * exprStmt       → expression ";" ;
 * printStmt      → "print" expression ";" ;
 * whileStmt      → "while" "(" expression ")" statement ;
 * forStmt        → "for" "(" ( varDecl | exprStmt | ";" ) ")" expression? ";" expression? ")" statement ;
 *
 * expression     → assignment ;
 * assignment     → IDENTIFIER "=" assignment
 *                | logic_or ;
 * logic_or       → logic_and ( "or" logic_and )* ;
 * logic_and      → equality ( "and" equality )* ;
 * equality       → comparison ( ( "!=" | "==" ) comparison )* ;
 * comparison     → term ( ( ">" | ">=" | "<" | "<=" ) term )* ;
 * term           → factor ( ( "-" | "+" ) factor )* ;
 * factor         → unary ( ( "/" | "*" ) unary )* ;
 * unary          → ( "!" | "-" ) unary
 *                | call ;
 * call           → primary ( "(" arguments? ")" ) *;
 * arguments      → expression ( "," expression ) *;
 * primary        → "true" | "false" | "nil"
 *                | "(" expression ")" ;
 *                | NUMBER | STRING
 *                | IDENTIFIER
*/

#[derive(Debug, Clone)]
pub struct ParserError {
    token: Token,
    message: String,
}

impl ParserError {
    pub fn new(token: Token, message: String) -> ParserError {
        let err = ParserError { token, message };
        LoxError::parsing_error(err.clone());
        err
    }

    pub fn message(&self) -> String {
        self.message.clone()
    }

    pub fn token(&self) -> Token {
        self.token.clone()
    }
}

type ExprError = Result<Expr, ParserError>;
type StmtError = Result<Stmt, ParserError>;
type TokenError = Result<Token, ParserError>;

#[derive(Debug)]
pub struct Parser {
    tokens: Vec<Token>,
    current: usize,
    error: LoxError,
}

impl Parser {
    pub fn new(tokens: Vec<Token>, error: LoxError) -> Parser {
        Parser {
            tokens,
            current: 0,
            error,
        }
    }

    pub fn parse(&mut self) -> Vec<Stmt> {
        let mut statements: Vec<Stmt> = Vec::new();

        while !self.is_at_end() {
            match self.declaration() {
                Ok(stmt) => statements.push(stmt),
                Err(_) => {
                    // LoxError::parsing_error(err);
                    break;
                }
            }
        }

        statements
    }

    pub fn had_error(&self) -> bool {
        self.error == LoxError::ParsingError
    }

    // grammar rules

    fn declaration(&mut self) -> StmtError {
        if self.match_token(Var) {
            self.var_declaration()
        } else {
            match self.statement() {
                id @ Ok(_) => id,
                Err(_) => {
                    self.synchronize();
                    Ok(Stmt::Null)
                }
            }
        }
    }

    fn var_declaration(&mut self) -> StmtError {
        let name = self.consume(Identifier, "expected identifier.".to_string())?;
        let initializer;
        if self.match_token(Equal) {
            initializer = self.expression()?;
        } else {
            initializer = Expr::Null;
        }

        self.consume(Semicolon, "expected ';' after the value.".to_string())?;

        Ok(Stmt::var(name, Box::new(initializer)))
    }

    fn statement(&mut self) -> StmtError {
        if self.match_token(Print) {
            self.print_statement()
        } else if self.match_token(LeftBrace) {
            let bl = self.block()?;
            Ok(Stmt::block(bl))
        } else if self.match_token(If) {
            self.if_statement()
        } else if self.match_token(While) {
            self.while_statement()
        } else if self.match_token(For) {
            self.for_statement()
        } else {
            self.expression_statement()
        }
    }

    fn print_statement(&mut self) -> StmtError {
        let value = self.expression()?;
        self.consume(Semicolon, "expected ';' after the value.".to_string())?;
        // Ok(Stmt::print(Box::new(value)))
        Ok(Stmt::print(value))
    }

    fn expression_statement(&mut self) -> StmtError {
        let value = self.expression()?;
        self.consume(Semicolon, "expected ';' after the value.".to_string())?;
        // Ok(Stmt::expression(Box::new(value)))
        Ok(Stmt::expression(value))
    }

    fn block(&mut self) -> Result<Vec<Stmt>, ParserError> {
        let mut stmt: Vec<Stmt> = Vec::new();

        while !self.check(RightBrace) && !self.is_at_end() {
            stmt.push(self.declaration()?);
        }

        self.consume(RightBrace, "expecterd '}' after block.".to_string())?;

        Ok(stmt)
    }

    fn if_statement(&mut self) -> StmtError {
        self.consume(LeftParen, "expected '(' after 'if'.".to_string())?;
        let condition = self.expression()?;
        self.consume(RightParen, "expected ')' after if condition.".to_string())?;

        let then_branch = self.statement()?;
        let else_branch;

        if self.match_token(Else) {
            else_branch = self.statement()?;
        } else {
            else_branch = Stmt::Null;
        }

        Ok(Stmt::ifstmt(
            condition,
            Box::new(then_branch),
            Box::new(else_branch),
        ))
    }

    fn while_statement(&mut self) -> StmtError {
        self.consume(LeftParen, "expected '(' after 'while'.".to_string())?;
        let condition = self.expression()?;
        self.consume(
            RightParen,
            "expected ')' after while condition.".to_string(),
        )?;

        let body = self.statement()?;

        Ok(Stmt::whilestmt(condition, Box::new(body)))
    }

    fn for_statement(&mut self) -> StmtError {
        // converts a for loop in a while loop
        self.consume(LeftParen, "expected '(' after 'for'.".to_string())?;

        let initializer;
        if self.match_token(Semicolon) {
            initializer = Stmt::Null;
        } else if self.match_token(Var) {
            initializer = self.var_declaration()?;
        } else {
            initializer = self.expression_statement()?;
        }

        let condition;
        if !self.check(Semicolon) {
            condition = self.expression()?;
        } else {
            condition = Expr::literal(Literal::Boolean(true));
        }
        self.consume(
            Semicolon,
            "expected ';' after for loop condition.".to_string(),
        )?;

        let increment;
        if !self.check(RightParen) {
            increment = self.expression()?;
        } else {
            increment = Expr::Null;
        }
        self.consume(RightParen, "expected ')' after for clauses.".to_string())?;

        let mut body = self.statement()?;

        if increment != Expr::Null {
            body = Stmt::block(vec![body, Stmt::expression(increment)]);
        }

        body = Stmt::whilestmt(condition, Box::new(body));

        if initializer != Stmt::Null {
            body = Stmt::block(vec![initializer, body]);
        }

        Ok(body)
    }

    fn expression(&mut self) -> ExprError {
        self.assignment()
    }

    fn assignment(&mut self) -> ExprError {
        let exp = self.logic_or()?;

        if self.match_token(Equal) {
            let equals = self.previous();
            let value = self.assignment()?;
            match exp {
                Expr::Variable { name } => {
                    return Ok(Expr::assign(name, Box::new(value)));
                }
                _ => {
                    ParserError::new(equals, "invalid assign target.".to_string());
                }
            }
        }
        Ok(exp)
    }

    fn logic_or(&mut self) -> ExprError {
        let mut and = self.logic_and()?;

        while self.match_token(Or) {
            let or = self.previous();
            let rest = self.logic_and()?;
            and = Expr::logical(Box::new(and), or, Box::new(rest));
        }

        Ok(and)
    }

    fn logic_and(&mut self) -> ExprError {
        let mut eq = self.equality()?;

        while self.match_token(And) {
            let and = self.previous();
            let rest = self.equality()?;
            eq = Expr::logical(Box::new(eq), and, Box::new(rest));
        }

        Ok(eq)
    }

    fn equality(&mut self) -> ExprError {
        let mut expr = self.comparison()?;

        while self.match_tokens(vec![BangEqual, EqualEqual]) {
            let operator = self.previous();
            let right = self.comparison()?;

            expr = Expr::binary(Box::new(expr), operator, Box::new(right));
        }

        Ok(expr)
    }

    fn comparison(&mut self) -> ExprError {
        let mut expr = self.term()?;

        while self.match_tokens(vec![Less, LessEqual, Greater, GreaterEqual]) {
            let operator = self.previous();
            let right = self.term()?;

            expr = Expr::binary(Box::new(expr), operator, Box::new(right));
        }

        Ok(expr)
    }

    fn term(&mut self) -> ExprError {
        let mut expr = self.factor()?;

        while self.match_tokens(vec![Minus, Plus]) {
            let operator = self.previous();
            let right = self.factor()?;

            expr = Expr::binary(Box::new(expr), operator, Box::new(right));
        }

        Ok(expr)
    }

    fn factor(&mut self) -> ExprError {
        let mut expr = self.unary()?;

        while self.match_tokens(vec![Star, Slash]) {
            let operator = self.previous();
            let right = self.unary()?;

            expr = Expr::binary(Box::new(expr), operator, Box::new(right));
        }

        Ok(expr)
    }

    fn unary(&mut self) -> ExprError {
        if self.match_tokens(vec![Bang, Minus]) {
            let operator = self.previous();
            let right = self.unary()?;

            Ok(Expr::unary(operator, Box::new(right)))
        } else {
            self.call()
        }
    }

    fn call(&mut self) -> ExprError {
        let mut exp = self.primary()?;

        loop {
            if self.match_token(LeftParen) {
                exp = self.finish_call(exp)?;
            } else {
                break;
            }
        }

        Ok(exp)
    }

    fn finish_call(&mut self, callee: Expr) -> ExprError {
        let mut args: Vec<Expr> = Vec::new();

        if !self.check(RightParen) {
            loop {
                if args.len() > 255 {
                    return Err(ParserError::new(
                        self.peek(),
                        "can't have more than 255 arguments.".to_string(),
                    ));
                }
                args.push(self.expression()?);
                if self.match_token(Comma) {
                    break;
                }
            }
        }

        let paren = self.consume(RightParen, "expected ')' after arguments.".to_string())?;

        Ok(Expr::call(Box::new(callee), paren, args))
    }

    fn primary(&mut self) -> ExprError {
        if self.match_token(False) {
            Ok(Expr::literal(Literal::Boolean(false)))
        } else if self.match_token(True) {
            Ok(Expr::literal(Literal::Boolean(true)))
        } else if self.match_token(Nil) {
            Ok(Expr::literal(Literal::Null))
        } else if self.match_tokens(vec![Number, TString]) {
            Ok(Expr::literal(self.previous().literal))
        } else if self.match_token(LeftParen) {
            let expr = self.expression()?;
            self.consume(RightParen, "expected ')' after expression;".to_string())?;
            Ok(Expr::grouping(Box::new(expr)))
        } else if self.match_token(Identifier) {
            Ok(Expr::variable(self.previous()))
        } else {
            self.error = LoxError::ParsingError;
            Err(ParserError::new(
                self.peek(),
                "expected expression.".to_string(),
            ))
        }
    }

    // aux

    fn match_token(&mut self, token_type: TokenType) -> bool {
        if self.check(token_type) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn match_tokens(&mut self, token_types: Vec<TokenType>) -> bool {
        for token_type in token_types {
            if self.check(token_type) {
                self.advance();
                return true;
            }
        }

        false
    }

    fn previous(&self) -> Token {
        self.tokens[self.current - 1].clone()
    }

    fn check(&self, token_type: TokenType) -> bool {
        if self.is_at_end() {
            false
        } else {
            self.peek().token_type == token_type
        }
    }

    fn advance(&mut self) -> Token {
        if !self.is_at_end() {
            self.current += 1;
        }
        self.previous()
    }

    fn is_at_end(&self) -> bool {
        self.peek().token_type == TokenType::Eof
    }

    fn peek(&self) -> Token {
        self.tokens[self.current].clone()
    }

    fn consume(&mut self, token_type: TokenType, error_mex: String) -> TokenError {
        if self.check(token_type) {
            Ok(self.advance())
        } else {
            self.error = LoxError::ParsingError;
            Err(ParserError::new(self.peek(), error_mex))
        }
    }

    fn synchronize(&mut self) {
        self.advance();

        while !self.is_at_end() {
            if self.previous().token_type == Semicolon {
                return;
            }

            match self.peek().token_type {
                Class | Fun | Var | For | If | While | Print | Return => return,
                _ => {
                    self.advance();
                }
            }
        }
    }
}
