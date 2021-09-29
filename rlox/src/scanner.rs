// doesn't handle unicode characters

pub struct Scanner<'s> {
    source: &'s str,
    start: usize,
    current: usize,
    line: usize,
}

impl<'s> Scanner<'s> {
    pub fn new(source: &'s str) -> Self {
        Self {
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
            self.make_token(Eof)
        } else {
            let c = self.advance();

            match c {
                '(' => self.make_token(LeftParen),
                ')' => self.make_token(RightParen),
                '[' => self.make_token(LeftBracket),
                ']' => self.make_token(RightBracket),
                '{' => self.make_token(LeftBrace),
                '}' => self.make_token(RightBrace),
                ';' => self.make_token(Semicolon),
                ',' => self.make_token(Comma),
                '.' => self.make_token(Dot),
                '-' => {
                    if self.match_char('=') {
                        self.make_token(MinusEqual)
                    } else if self.match_char('-') {
                        self.make_token(MinusMinus)
                    } else {
                        self.make_token(Minus)
                    }
                }
                '+' => {
                    if self.match_char('=') {
                        self.make_token(PlusEqual)
                    } else if self.match_char('+') {
                        self.make_token(PlusPlus)
                    } else {
                        self.make_token(Plus)
                    }
                }
                '/' => {
                    if self.match_char('=') {
                        self.make_token(SlashEqual)
                    } else {
                        self.make_token(Slash)
                    }
                }
                '*' => {
                    if self.match_char('=') {
                        self.make_token(StarEqual)
                    } else {
                        self.make_token(Star)
                    }
                }
                '!' => {
                    if self.match_char('=') {
                        self.make_token(BangEqual)
                    } else {
                        self.make_token(Bang)
                    }
                }
                '=' => {
                    if self.match_char('=') {
                        self.make_token(EqualEqual)
                    } else {
                        self.make_token(Equal)
                    }
                }
                '<' => {
                    if self.match_char('=') {
                        self.make_token(LessEqual)
                    } else if self.match_char('|') {
                        self.make_token(LessPipe)
                    } else {
                        self.make_token(Less)
                    }
                }
                '>' => {
                    if self.match_char('=') {
                        self.make_token(GreaterEqual)
                    } else {
                        self.make_token(Greater)
                    }
                }
                '%' => self.make_token(Rem),
                '"' => self.string(),
                '0'..='9' => self.number(),
                'a'..='z' | 'A'..='Z' | '_' => self.identifier(),
                _ => self.error_token("Unexpected character."),
            }
        }
    }

    #[inline]
    fn at_end(&self) -> bool {
        self.source.chars().count() == self.current
    }

    #[inline]
    fn make_token(&self, token_type: TokenType) -> Token<'s> {
        let lexeme = &self.source[self.start..self.current];
        Token::new(token_type, lexeme, self.line)
    }

    #[inline]
    fn error_token(&self, message: &'s str) -> Token<'s> {
        Token::new(Error, message, self.line)
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

    #[inline]
    fn peek(&mut self) -> char {
        self.source.chars().nth(self.current).unwrap_or('\0')
    }

    #[inline]
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
            self.make_token(RString)
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

        self.make_token(Number)
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
            'a' => self.check_keyword(1, 2, "nd", And),
            'c' => self.check_keyword(1, 4, "lass", Class),
            'e' => self.check_keyword(1, 3, "lse", Else),
            'f' => {
                if self.current - self.start > 1 {
                    match self
                        .source
                        .chars()
                        .nth(self.start + 1)
                        .expect("Error advancing on character")
                    {
                        'a' => self.check_keyword(2, 3, "lse", False),
                        'o' => self.check_keyword(2, 1, "r", For),
                        'u' => self.check_keyword(2, 1, "n", Fun),
                        _ => Identifier,
                    }
                } else {
                    Identifier
                }
            }
            'i' => self.check_keyword(1, 1, "f", If),
            'n' => self.check_keyword(1, 2, "il", Nil),
            'o' => self.check_keyword(1, 1, "r", Or),
            'p' => self.check_keyword(1, 4, "rint", Print),
            'r' => self.check_keyword(1, 5, "eturn", Return),
            's' => self.check_keyword(1, 4, "uper", Super),
            't' => {
                if self.current - self.start > 1 {
                    match self
                        .source
                        .chars()
                        .nth(self.start + 1)
                        .expect("Error advancing on character")
                    {
                        'h' => self.check_keyword(2, 2, "is", This),
                        'r' => self.check_keyword(2, 2, "ue", True),
                        _ => Identifier,
                    }
                } else {
                    Identifier
                }
            }
            'v' => self.check_keyword(1, 2, "ar", Var),
            'w' => self.check_keyword(1, 4, "hile", While),
            _ => Identifier,
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
            Identifier
        }
    }
}

#[derive(Clone, Copy)]
pub struct Token<'a> {
    pub token_type: TokenType,
    pub lexeme: &'a str,
    pub line: usize,
}

impl<'a> Token<'a> {
    pub fn new(token_type: TokenType, lexeme: &'a str, line: usize) -> Self {
        Self {
            token_type,
            lexeme,
            line,
        }
    }

    pub fn syntethic(lexeme: &'a str) -> Self {
        Self {
            token_type: Error,
            lexeme,
            line: 0,
        }
    }
}

use self::TokenType::*;

#[derive(Clone, PartialEq, Debug, Eq, Hash, Copy)]
pub enum TokenType {
    // Single-character tokens.
    Comma,
    Dot,
    LeftBrace,
    LeftBracket,
    LeftParen,
    Minus,
    Rem,
    Plus,
    RightBrace,
    RightBracket,
    RightParen,
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
    LessPipe,
    MinusEqual,
    MinusMinus,
    PlusEqual,
    PlusPlus,
    SlashEqual,
    StarEqual,
    // Literals.
    Identifier,
    Number,
    RString,
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
    // Signal tokens
    Eof,
    Error,
}
