use crate::token::{Literal, Token, TokenType};
use crate::LoxError;

pub struct ScannerError {
    line: usize,
    message: String,
}

impl ScannerError {
    pub fn new(line: usize, message: String) -> ScannerError {
        ScannerError { line, message }
    }

    pub fn message(&self) -> String {
        self.message.clone()
    }

    pub fn line(&self) -> usize {
        self.line
    }
}

pub struct Scanner {
    source: String,
    token_list: Vec<Token>,
    start: usize,
    current: usize,
    line: usize,
    error: LoxError,
}

impl Scanner {
    pub fn new(error: LoxError, source: String) -> Scanner {
        Scanner {
            source,
            token_list: Vec::new(),
            start: 0,
            current: 0,
            line: 1,
            error,
        }
    }

    pub fn scan_tokens(&mut self) -> Vec<Token> {
        while !self.is_at_end() {
            self.start = self.current;
            self.scan_token();
        }

        self.token_list.push(Token::new(
            TokenType::Eof,
            self.line,
            String::new(),
            Literal::Eof,
        ));
        self.token_list.clone()
    }

    pub fn had_error(&self) -> bool {
        self.error == LoxError::ScanningError
    }

    fn is_at_end(&self) -> bool {
        self.current >= self.source.len()
    }

    fn scan_token(&mut self) {
        let ch = self.advance();
        match ch {
            '(' => self.add_token(TokenType::LeftParen, Literal::Symbol),
            ')' => self.add_token(TokenType::RightParen, Literal::Symbol),
            '{' => self.add_token(TokenType::LeftBrace, Literal::Symbol),
            '}' => self.add_token(TokenType::RightBrace, Literal::Symbol),
            '+' => self.add_token(TokenType::Plus, Literal::Symbol),
            '-' => self.add_token(TokenType::Minus, Literal::Symbol),
            '*' => self.add_token(TokenType::Star, Literal::Symbol),
            '.' => self.add_token(TokenType::Dot, Literal::Symbol),
            ',' => self.add_token(TokenType::Comma, Literal::Symbol),
            ';' => self.add_token(TokenType::Semicolon, Literal::Symbol),
            '!' => {
                if self.match_char('=') {
                    self.add_token(TokenType::BangEqual, Literal::Symbol)
                } else {
                    self.add_token(TokenType::Bang, Literal::Symbol)
                }
            }
            '=' => {
                if self.match_char('=') {
                    self.add_token(TokenType::EqualEqual, Literal::Symbol)
                } else {
                    self.add_token(TokenType::Equal, Literal::Symbol)
                }
            }
            '<' => {
                if self.match_char('=') {
                    self.add_token(TokenType::LessEqual, Literal::Symbol)
                } else {
                    self.add_token(TokenType::Less, Literal::Symbol)
                }
            }
            '>' => {
                if self.match_char('=') {
                    self.add_token(TokenType::GreaterEqual, Literal::Symbol)
                } else {
                    self.add_token(TokenType::Greater, Literal::Symbol)
                }
            }
            '/' => {
                if self.match_char('/') {
                    while self.peek() != '\n' && !self.is_at_end() {
                        self.advance();
                    }
                } else if self.match_char('*') {
                    while self.peek() != '*' && self.peek_next() != '/' && !self.is_at_end() {
                        self.advance();
                    }
                    self.advance();
                    self.advance();
                } else {
                    self.add_token(TokenType::Slash, Literal::Symbol)
                }
            }
            ' ' | '\r' | '\t' => (),
            '\n' => {
                self.line += 1;
            }
            '"' => {
                self.string();
            }
            _ => {
                if Scanner::is_digit(ch) {
                    self.number()
                } else if Scanner::is_alpha(ch) {
                    self.identifier();
                } else {
                    LoxError::scanning_error(ScannerError::new(
                        self.line,
                        String::from("Unexpected character"),
                    ));
                    self.error = LoxError::ScanningError;
                }
            }
        }
    }

    fn advance(&mut self) -> char {
        self.current += 1;
        self.source
            .chars()
            .nth(self.current - 1)
            .expect("Error advancing on character")
    }

    fn add_token(&mut self, typ: TokenType, literal: Literal) {
        let st: String = self.source[self.start..self.current].to_string();
        self.token_list
            .push(Token::new(typ, self.line, st, literal));
    }

    fn match_char(&mut self, expected: char) -> bool {
        if self.is_at_end() || self.source.chars().nth(self.current).unwrap() != expected {
            false
        } else {
            self.current += 1;
            true
        }
    }

    fn peek(&self) -> char {
        if self.is_at_end() {
            '\0'
        } else {
            self.source.chars().nth(self.current).unwrap()
        }
    }

    fn peek_next(&self) -> char {
        if self.current + 1 >= self.source.len() {
            '\0'
        } else {
            self.source.chars().nth(self.current + 1).unwrap()
        }
    }

    fn string(&mut self) {
        while !self.is_at_end() && self.peek() != '"' {
            if self.peek() == '\n' {
                self.line += 1;
            }
            self.advance();
        }

        if self.is_at_end() {
            LoxError::scanning_error(ScannerError::new(
                self.line,
                String::from("Unterminated string"),
            ));
            self.error = LoxError::ScanningError;
        } else {
            self.advance();
            self.add_token(
                TokenType::TString,
                Literal::LString(self.source[self.start + 1..self.current - 1].to_string()),
            );
        }
    }

    fn is_digit(c: char) -> bool {
        ('0'..='9').contains(&c)
    }

    fn number(&mut self) {
        while Scanner::is_digit(self.peek()) {
            self.advance();
        }

        if self.peek() == '.' && Scanner::is_digit(self.peek_next()) {
            self.advance();
            while Scanner::is_digit(self.peek()) {
                self.advance();
            }
        }

        self.add_token(
            TokenType::Number,
            Literal::Number(
                self.source[self.start..self.current]
                    .parse::<f64>()
                    .unwrap(),
            ),
        )
    }

    fn is_alpha(c: char) -> bool {
        ('a'..='z').contains(&c) || ('A'..='Z').contains(&c) || c == '_'
    }

    fn is_alphanumeric(c: char) -> bool {
        Scanner::is_alpha(c) || Scanner::is_digit(c)
    }

    // fn identifier(&mut self) {
    //     while Scanner::is_alphanumeric(self.peek()) {
    //         self.advance();
    //     }

    //     let text = self.source[self.start..self.current].to_string();
    //     if let Some(t) = KEYWORDS.get(&text) {
    //         self.add_token(t.clone(), Literal::Keyword);
    //     } else {
    //         self.add_token(TokenType::Identifier, Literal::Identifier);
    //     }
    // }

    fn identifier(&mut self) {
        while Scanner::is_alphanumeric(self.peek()) {
            self.advance();
        }

        let text = &self.source[self.start..self.current];
        if let Some(t) = Scanner::match_keyword(text) {
            self.add_token(t, Literal::Keyword);
        } else {
            self.add_token(TokenType::Identifier, Literal::Identifier);
        }
    }

    fn match_keyword(s: &str) -> Option<TokenType> {
        match s {
            "and" => Some(TokenType::And),
            "class" => Some(TokenType::Class),
            "else" => Some(TokenType::Else),
            "false" => Some(TokenType::False),
            "for" => Some(TokenType::For),
            "fun" => Some(TokenType::Fun),
            "if" => Some(TokenType::If),
            "nil" => Some(TokenType::Nil),
            "or" => Some(TokenType::Or),
            "print" => Some(TokenType::Print),
            "return" => Some(TokenType::Return),
            "super" => Some(TokenType::Super),
            "this" => Some(TokenType::This),
            "true" => Some(TokenType::True),
            "var" => Some(TokenType::Var),
            "while" => Some(TokenType::While),
            _ => None,
        }
    }
}

// use lazy_static::lazy_static;
// use std::collections::HashMap;

// lazy_static! {
//     static ref KEYWORDS: HashMap<String, TokenType> = {
//         let mut hashmap: HashMap<String, TokenType> = HashMap::new();
//         hashmap.insert("and".to_string(), TokenType::And);
//         hashmap.insert("class".to_string(), TokenType::Class);
//         hashmap.insert("else".to_string(), TokenType::Else);
//         hashmap.insert("false".to_string(), TokenType::False);
//         hashmap.insert("for".to_string(), TokenType::For);
//         hashmap.insert("fun".to_string(), TokenType::Fun);
//         hashmap.insert("if".to_string(), TokenType::If);
//         hashmap.insert("nil".to_string(), TokenType::Nil);
//         hashmap.insert("or".to_string(), TokenType::Or);
//         hashmap.insert("print".to_string(), TokenType::Print);
//         hashmap.insert("return".to_string(), TokenType::Return);
//         hashmap.insert("super".to_string(), TokenType::Super);
//         hashmap.insert("this".to_string(), TokenType::This);
//         hashmap.insert("true".to_string(), TokenType::True);
//         hashmap.insert("var".to_string(), TokenType::Var);
//         hashmap.insert("while".to_string(), TokenType::While);
//         hashmap
//     };
// }
