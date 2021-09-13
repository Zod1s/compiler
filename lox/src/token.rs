use std::fmt;
use crate::function;

#[derive(Debug, Clone, PartialEq)]
pub enum TokenType {
    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,
    Plus,
    Minus,
    Star,
    Slash,
    Dot,
    Comma,
    Semicolon,

    // One or two character tokens.
    Bang,
    BangEqual,
    Equal,
    EqualEqual,
    Greater,
    GreaterEqual,
    Less,
    LessEqual,

    // Literals.
    Identifier,
    TString,
    Number,

    // Keywords.
    And,
    Or,
    Class,
    If,
    Else,
    True,
    False,
    Fun,
    While,
    For,
    Nil,
    Print,
    Return,
    Super,
    This,
    Var,

    Eof,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Number(f64),
    LString(String),
    Boolean(bool),
    Function(function::Function),
    Identifier,
    Keyword,
    Eof,
    Symbol,
    Null,
}

impl Literal {
    #[inline(always)]
    pub fn lit_true() -> Literal {
        Literal::Boolean(true)
    }

    #[inline(always)]
    pub fn lit_false() -> Literal {
        Literal::Boolean(false)
    }

    pub fn literal_type(&self) -> String {
        match self {
            Literal::Number(_) => "number".to_string(),
            Literal::LString(_) => "string".to_string(),
            Literal::Boolean(_) => "boolean".to_string(),
            Literal::Function(_) => "function".to_string(),
            Literal::Identifier => "identifier".to_string(),
            Literal::Keyword => "keyword".to_string(),
            Literal::Eof => "end of file marker".to_string(),
            Literal::Symbol => "symbol".to_string(),
            Literal::Null => "nil".to_string(),
        }
    }
}

impl fmt::Display for Literal {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Literal::Number(n) => write!(f, "(Number {})", n),
            Literal::LString(s) => write!(f, "(LString \"{}\")", s),
            Literal::Boolean(b) => write!(f, "(Boolean {})", b),
            Literal::Function(fun) => write!(f, "(Function {})", fun),
            Literal::Identifier => write!(f, "Identifier"),
            Literal::Keyword => write!(f, "Keyword"),
            Literal::Eof => write!(f, "EOF"),
            Literal::Symbol => write!(f, "Symbol"),
            Literal::Null => write!(f, "Null"),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Token {
    pub token_type: TokenType,
    pub line: usize,
    pub lexeme: String,
    pub literal: Literal,
}

impl Token {
    pub fn new(token_type: TokenType, line: usize, lexeme: String, literal: Literal) -> Token {
        Token {
            token_type,
            line,
            lexeme,
            literal,
        }
    }
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?} {} {}", self.token_type, self.lexeme, self.literal)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum LoxTypes {
    Object(Literal),
}

impl fmt::Display for LoxTypes {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            LoxTypes::Object(l) => write!(f, "{}", l),
            // LoxTypes::Function(fun) => write!(f, "{}", fun)
        }
    }
}
