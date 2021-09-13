use std::fmt;

pub struct Scanner<'s> {
    pub source: &'s str,
    pub start: usize,
    pub current: usize,
    pub line: usize,
}

impl<'s> Scanner<'s> {
    pub fn new(source: &'s str) -> Self {
        Scanner {
            source,
            start: 0,
            current: 0,
            line: 1,
        }
    }

    pub fn scan_token(&mut self) -> Token<'s> {
        self.skip_withespaces();
        self.start = self.current;

        if self.at_end() {
            self.make_token(TokenType::Eof)
        } else {
            let c = self.advance();

            match c {
                '(' => self.make_token(TokenType::LeftParen),
                ')' => self.make_token(TokenType::RightParen),
                '{' => self.make_token(TokenType::LeftBrace),
                '}' => self.make_token(TokenType::RightBrace),
                ';' => self.make_token(TokenType::Semicolon),
                ',' => self.make_token(TokenType::Comma),
                '.' => self.make_token(TokenType::Dot),
                '-' => self.make_token(TokenType::Minus),
                '+' => self.make_token(TokenType::Plus),
                '/' => self.make_token(TokenType::Slash),
                '*' => self.make_token(TokenType::Star),
                '!' => {
                    if self.match_char('=') {
                        self.make_token(TokenType::BangEqual)
                    } else {
                        self.make_token(TokenType::Bang)
                    }
                }
                '=' => {
                    if self.match_char('=') {
                        self.make_token(TokenType::EqualEqual)
                    } else {
                        self.make_token(TokenType::Equal)
                    }
                }
                '<' => {
                    if self.match_char('=') {
                        self.make_token(TokenType::LessEqual)
                    } else {
                        self.make_token(TokenType::Less)
                    }
                }
                '>' => {
                    if self.match_char('=') {
                        self.make_token(TokenType::GreaterEqual)
                    } else {
                        self.make_token(TokenType::Greater)
                    }
                }
                '"' => self.string(),
                '0'..='9' => self.number(),
                'a'..='z' | 'A'..='Z' | '_' => self.identifier(),
                _ => self.error_token("Unexpected character."),
            }
        }
    }

    fn at_end(&self) -> bool {
        self.source.chars().count() == self.current
    }

    fn make_token(&self, token_type: TokenType) -> Token<'s> {
        let lexeme = &self.source[self.start..self.current];
        Token::new(token_type, lexeme, self.current - self.start, self.line)
    }

    fn error_token(&self, message: &'s str) -> Token<'s> {
        let length = message.chars().count();
        Token::new(TokenType::Error, message, length, self.line)
    }

    fn advance(&mut self) -> char {
        self.current += 1;
        self.source
            .chars()
            .nth(self.current - 1)
            .expect("Error advancing on character")
    }

    fn match_char(&mut self, expected: char) -> bool {
        if self.at_end() || self.source.chars().nth(self.current).unwrap() != expected {
            false
        } else {
            self.current += 1;
            true
        }
    }

    fn skip_withespaces(&mut self) {
        loop {
            match self.peek() {
                ' ' | '\t' | '\r' => {
                    self.advance();
                }
                '\n' => {
                    self.line += 1;
                    self.advance();
                }
                '/' => {
                    if self.peek_next() == '/' {
                        while self.peek() != '\n' && !self.at_end() {
                            self.advance();
                        }
                    } else {
                        return;
                    }
                }
                _ => return,
            }
        }
    }

    fn peek(&mut self) -> char {
        self.source.chars().nth(self.current).unwrap_or('\0')
    }

    fn peek_next(&mut self) -> char {
        self.source.chars().nth(self.current + 1).unwrap_or('\0')
    }

    fn string(&mut self) -> Token<'s> {
        while self.peek() != '"' && !self.at_end() {
            if self.peek() == '\n' {
                self.line += 1;
            }
            self.advance();
        }

        if self.at_end() {
            self.error_token("Unterminated string.")
        } else {
            self.advance();
            self.make_token(TokenType::RString)
        }
    }

    fn number(&mut self) -> Token<'s> {
        while ('0'..='9').contains(&self.peek()) {
            self.advance();
        }

        if self.peek() == '.' && ('0'..='9').contains(&self.peek_next()) {
            self.advance();

            while ('0'..='9').contains(&self.peek()) {
                self.advance();
            }
        }

        self.make_token(TokenType::Number)
    }

    fn identifier(&mut self) -> Token<'s> {
        while ('a'..='z').contains(&self.peek())
            || ('A'..='Z').contains(&self.peek())
            || '_' == self.peek()
            || ('0'..='9').contains(&self.peek())
        {
            self.advance();
        }

        let ttype = self.identifier_type();
        self.make_token(ttype)
    }

    fn identifier_type(&mut self) -> TokenType {
        match self
            .source
            .chars()
            .nth(self.start)
            .expect("Error advancing on character")
        {
            'a' => self.check_keyword(1, 2, "nd", TokenType::And),
            'c' => self.check_keyword(1, 4, "lass", TokenType::Class),
            'e' => self.check_keyword(1, 3, "lse", TokenType::Else),
            'f' => {
                if self.current - self.start > 1 {
                    match self
                        .source
                        .chars()
                        .nth(self.start + 1)
                        .expect("Error advancing on character")
                    {
                        'a' => self.check_keyword(2, 3, "lse", TokenType::False),
                        'o' => self.check_keyword(2, 1, "r", TokenType::For),
                        'u' => self.check_keyword(2, 1, "n", TokenType::Fun),
                        _ => TokenType::Identifier,
                    }
                } else {
                    TokenType::Identifier
                }
            }
            'i' => self.check_keyword(1, 1, "f", TokenType::If),
            'n' => self.check_keyword(1, 2, "il", TokenType::Nil),
            'o' => self.check_keyword(1, 1, "r", TokenType::Or),
            'p' => self.check_keyword(1, 4, "rint", TokenType::Print),
            'r' => self.check_keyword(1, 5, "eturn", TokenType::Return),
            's' => self.check_keyword(1, 4, "uper", TokenType::Super),
            't' => {
                if self.current - self.start > 1 {
                    match self
                        .source
                        .chars()
                        .nth(self.start + 1)
                        .expect("Error advancing on character")
                    {
                        'h' => self.check_keyword(2, 2, "is", TokenType::This),
                        'r' => self.check_keyword(2, 2, "ue", TokenType::True),
                        _ => TokenType::Identifier,
                    }
                } else {
                    TokenType::Identifier
                }
            }
            'v' => self.check_keyword(1, 2, "ar", TokenType::Var),
            'w' => self.check_keyword(1, 4, "hile", TokenType::While),
            _ => TokenType::Identifier,
        }
    }

    fn check_keyword(
        &mut self,
        start: usize,
        length: usize,
        rest: &str,
        ttype: TokenType,
    ) -> TokenType {
        if start + length == self.current - self.start
            && rest == &self.source[self.start + start..self.current]
        {
            ttype
        } else {
            TokenType::Identifier
        }
    }
}

#[derive(Clone, Copy)]
pub struct Token<'a> {
    pub token_type: TokenType,
    pub lexeme: &'a str,
    pub length: usize,
    pub line: usize,
}

impl<'a> Token<'a> {
    pub fn new(token_type: TokenType, lexeme: &'a str, length: usize, line: usize) -> Self {
        Token {
            token_type,
            lexeme,
            length,
            line,
        }
    }
}

#[derive(Clone, PartialEq, Debug, Eq, Hash, Copy)]
pub enum TokenType {
    // Single-character tokens.
    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,
    Comma,
    Dot,
    Minus,
    Plus,
    Semicolon,
    Slash,
    Star,
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
    RString,
    Number,
    // Keywords.
    And,
    Class,
    Else,
    False,
    For,
    Fun,
    If,
    Nil,
    Or,
    Print,
    Return,
    Super,
    This,
    True,
    Var,
    While,

    Error,
    Eof,
    None,
}

impl fmt::Display for TokenType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            TokenType::LeftParen => write!(f, "   LeftParen"),
            TokenType::RightParen => write!(f, "  RightParen"),
            TokenType::LeftBrace => write!(f, "   LeftBrace"),
            TokenType::RightBrace => write!(f, "  RightBrace"),
            TokenType::Comma => write!(f, "       Comma"),
            TokenType::Dot => write!(f, "         Dot"),
            TokenType::Minus => write!(f, "       Minus"),
            TokenType::Plus => write!(f, "        Plus"),
            TokenType::Semicolon => write!(f, "   Semicolon"),
            TokenType::Slash => write!(f, "       Slash"),
            TokenType::Star => write!(f, "        Star"),
            TokenType::Bang => write!(f, "        Bang"),
            TokenType::BangEqual => write!(f, "   BangEqual"),
            TokenType::Equal => write!(f, "       Equal"),
            TokenType::EqualEqual => write!(f, "  EqualEqual"),
            TokenType::Greater => write!(f, "     Greater"),
            TokenType::GreaterEqual => write!(f, "GreaterEqual"),
            TokenType::Less => write!(f, "        Less"),
            TokenType::LessEqual => write!(f, "   LessEqual"),

            TokenType::Identifier => write!(f, "  Identifier"),
            TokenType::RString => write!(f, "     String"),
            TokenType::Number => write!(f, "      Number"),

            TokenType::And => write!(f, "         And"),
            TokenType::Class => write!(f, "       Class"),
            TokenType::Else => write!(f, "        Else"),
            TokenType::False => write!(f, "       False"),
            TokenType::For => write!(f, "         For"),
            TokenType::Fun => write!(f, "         Fun"),
            TokenType::If => write!(f, "          If"),
            TokenType::Nil => write!(f, "         Nil"),
            TokenType::Or => write!(f, "          Or"),
            TokenType::Print => write!(f, "       Print"),
            TokenType::Return => write!(f, "      Return"),
            TokenType::Super => write!(f, "       Super"),
            TokenType::This => write!(f, "        This"),
            TokenType::True => write!(f, "        True"),
            TokenType::Var => write!(f, "         Var"),
            TokenType::While => write!(f, "       While"),
            TokenType::Error => write!(f, "       Error"),
            TokenType::Eof => write!(f, "         Eof"),
            TokenType::None => write!(f, "        None"),
        }
    }
}
