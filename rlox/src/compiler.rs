use crate::{
    chunk::{disassemble_chunk, OpCode},
    object::{Function, FunctionType, FunctionUpvalue, LoxString},
    scanner::*,
    types::{InterpretError, Precedence, Value},
};
use std::{collections::HashMap, mem};

type ParseFn<'s> = fn(&mut Parser<'s>, can_assign: bool) -> ();

#[derive(Clone)]
struct ParseRule<'s> {
    prefix: Option<ParseFn<'s>>,
    infix: Option<ParseFn<'s>>,
    precedence: Precedence,
}

pub struct Parser<'s> {
    current: Token<'s>,
    previous: Token<'s>,
    scanner: Scanner<'s>,
    had_error: bool,
    panic_mode: bool,
    parse_rules: HashMap<TokenType, ParseRule<'s>>,
    compiler: Box<Compiler<'s>>,
}

impl<'s> Parser<'s> {
    pub fn new(code: &'s str) -> Self {
        let mut parse_rules = HashMap::new();
        let mut rule = |kind, prefix, infix, precedence| {
            parse_rules.insert(
                kind,
                ParseRule {
                    prefix,
                    infix,
                    precedence,
                },
            )
        };
        rule(
            TokenType::LeftParen,
            Some(Parser::grouping),
            Some(Parser::call),
            Precedence::Call,
        );
        rule(TokenType::RightParen, None, None, Precedence::None);
        rule(TokenType::LeftBrace, None, None, Precedence::None);
        rule(TokenType::RightBrace, None, None, Precedence::None);
        rule(TokenType::Comma, None, None, Precedence::None);
        // rule(TokenType::Dot, None, Some(Parser::dot), Precedence::Call);
        rule(
            TokenType::Minus,
            Some(Parser::unary),
            Some(Parser::binary),
            Precedence::Term,
        );
        rule(
            TokenType::Plus,
            None,
            Some(Parser::binary),
            Precedence::Term,
        );
        rule(TokenType::Semicolon, None, None, Precedence::None);
        rule(
            TokenType::Slash,
            None,
            Some(Parser::binary),
            Precedence::Factor,
        );
        rule(
            TokenType::Star,
            None,
            Some(Parser::binary),
            Precedence::Factor,
        );
        rule(TokenType::Bang, Some(Parser::unary), None, Precedence::None);
        rule(
            TokenType::BangEqual,
            None,
            Some(Parser::binary),
            Precedence::Equality,
        );
        rule(TokenType::Equal, None, None, Precedence::None);
        rule(
            TokenType::EqualEqual,
            None,
            Some(Parser::binary),
            Precedence::Equality,
        );
        rule(
            TokenType::Greater,
            None,
            Some(Parser::binary),
            Precedence::Comparison,
        );
        rule(
            TokenType::GreaterEqual,
            None,
            Some(Parser::binary),
            Precedence::Comparison,
        );
        rule(
            TokenType::Less,
            None,
            Some(Parser::binary),
            Precedence::Comparison,
        );
        rule(
            TokenType::LessEqual,
            None,
            Some(Parser::binary),
            Precedence::Comparison,
        );
        rule(
            TokenType::Identifier,
            Some(Parser::variable),
            None,
            Precedence::None,
        );
        rule(
            TokenType::RString,
            Some(Parser::string),
            None,
            Precedence::None,
        );
        rule(
            TokenType::Number,
            Some(Parser::number),
            None,
            Precedence::None,
        );
        rule(TokenType::And, None, Some(Parser::and_op), Precedence::And);
        rule(TokenType::Class, None, None, Precedence::None);
        rule(TokenType::Else, None, None, Precedence::None);
        rule(
            TokenType::False,
            Some(Parser::literal),
            None,
            Precedence::None,
        );
        rule(TokenType::For, None, None, Precedence::None);
        rule(TokenType::Fun, None, None, Precedence::None);
        rule(TokenType::If, None, None, Precedence::None);
        rule(
            TokenType::Nil,
            Some(Parser::literal),
            None,
            Precedence::None,
        );
        rule(TokenType::Or, None, Some(Parser::or_op), Precedence::Or);
        rule(TokenType::Print, None, None, Precedence::None);
        rule(TokenType::Return, None, None, Precedence::None);
        // rule(TokenType::Super, Some(Parser::super_), None, Precedence::None);
        // rule(TokenType::This, Some(Parser::this), None, Precedence::None);
        rule(
            TokenType::True,
            Some(Parser::literal),
            None,
            Precedence::None,
        );
        rule(TokenType::Var, None, None, Precedence::None);
        rule(TokenType::While, None, None, Precedence::None);
        rule(TokenType::Error, None, None, Precedence::None);
        rule(TokenType::Eof, None, None, Precedence::None);

        let compiler = Compiler::new("script".to_string(), FunctionType::Script);

        Parser {
            current: Token::new(TokenType::None, "None", 4, 0),
            previous: Token::new(TokenType::None, "None", 4, 0),
            scanner: Scanner::new(code),
            had_error: false,
            panic_mode: false,
            parse_rules,
            compiler,
        }
    }

    fn compile(&mut self) -> Result<Function, InterpretError> {
        self.advance();
        while !self.match_token(TokenType::Eof) {
            self.declaration();
        }
        self.consume(TokenType::Eof, "Expect end of expression.");
        self.emit_return();
        if cfg!(feature = "debug") && !self.had_error {
            disassemble_chunk(&self.compiler.function.chunk, "code");
        }
        if self.had_error {
            Err(InterpretError::Compile)
        } else {
            Ok(self.compiler.function.clone())
        }
    }

    fn advance(&mut self) {
        self.previous = self.current;

        loop {
            self.current = self.scanner.scan_token();
            if self.current.token_type != TokenType::Error {
                break;
            }

            self.error_at_current(self.current.lexeme);
            self.had_error = true;
            self.panic_mode = true;
        }
    }

    fn consume(&mut self, ttype: TokenType, message: &str) {
        if self.current.token_type == ttype {
            self.advance();
        } else {
            self.error_at_current(message);
            self.had_error = true;
            self.panic_mode = true;
        }
    }

    fn parse_precedence(&mut self, precedence: Precedence) {
        self.advance();
        let prefix_rule = match self
            .parse_rules
            .get(&self.previous.token_type)
            .unwrap_or_else(|| panic!("No parse rule found for {}.", self.previous.token_type))
            .prefix
        {
            None => {
                self.error("Expected expression.");
                return;
            }
            Some(rule) => rule,
        };

        let can_assign = precedence <= Precedence::Assignment;
        prefix_rule(self, can_assign);

        while precedence
            <= self
                .parse_rules
                .get(&self.current.token_type)
                .unwrap()
                .precedence
        {
            self.advance();
            let infix_rule = self
                .parse_rules
                .get(&self.previous.token_type)
                .unwrap()
                .infix
                .unwrap();
            infix_rule(self, can_assign);
        }

        if can_assign && self.match_token(TokenType::Equal) {
            self.error("Invalid assignment target.");
        }
    }

    fn declaration(&mut self) {
        if self.match_token(TokenType::Var) {
            self.var_declaration();
        } else if self.match_token(TokenType::Fun) {
            self.fun_declaration();
        } else {
            self.statement();
        }
        if self.panic_mode {
            self.synchronize();
        }
    }

    fn var_declaration(&mut self) {
        let global: usize = self.parse_variable("Expect variable name.");

        if self.match_token(TokenType::Equal) {
            self.expression();
        } else {
            self.emit_opcode(OpCode::Nil);
        }

        self.consume(TokenType::Semicolon, "Expected ';' after statement");

        self.define_variable(global);
    }

    fn fun_declaration(&mut self) {
        let global: usize = self.parse_variable("Expect variable name.");
        self.mark_initialized();
        self.function(FunctionType::Function);
        self.define_variable(global);
    }

    fn statement(&mut self) {
        if self.match_token(TokenType::Print) {
            self.print_statement();
        } else if self.match_token(TokenType::LeftBrace) {
            self.begin_scope();
            self.block();
            self.end_scope();
        } else if self.match_token(TokenType::If) {
            self.if_statement();
        } else if self.match_token(TokenType::While) {
            self.while_statement();
        } else if self.match_token(TokenType::For) {
            self.for_statement();
        } else if self.match_token(TokenType::Return) {
            self.return_statement();
        } else {
            self.expression_statement();
        }
    }

    fn print_statement(&mut self) {
        self.expression();
        self.consume(TokenType::Semicolon, "Expected ';' after statement");
        self.emit_opcode(OpCode::Print);
    }

    fn begin_scope(&mut self) {
        self.compiler.scope_depth += 1;
    }

    fn block(&mut self) {
        while !(self.check(TokenType::RightBrace) || self.check(TokenType::Eof)) {
            self.declaration();
        }

        self.consume(TokenType::RightBrace, "Expect '}' after block.");
    }

    fn end_scope(&mut self) {
        self.compiler.scope_depth -= 1;
        for i in (0..self.compiler.locals.len()).rev() {
            if self.compiler.locals[i].depth > self.compiler.scope_depth {
                if self.compiler.locals[i].is_captured {
                    self.emit_opcode(OpCode::CloseUpvalue);
                } else {
                    self.emit_opcode(OpCode::Pop);
                }
                self.compiler.locals.pop();
            } else {
                break;
            }
        }
    }

    fn if_statement(&mut self) {
        self.consume(TokenType::LeftParen, "Expect '(' after if.");
        self.expression();
        self.consume(TokenType::RightParen, "Expect ')' after condition.");

        let then = self.emit_jump(OpCode::JumpIfFalse(0));
        self.emit_opcode(OpCode::Pop);
        self.statement();
        let else_jump = self.emit_jump(OpCode::Jump(0));
        self.patch_jump(then);
        self.emit_opcode(OpCode::Pop);

        if self.match_token(TokenType::Else) {
            self.statement();
        }
        self.patch_jump(else_jump);
    }

    fn while_statement(&mut self) {
        let loop_start = self.start_loop();
        self.consume(TokenType::LeftParen, "Expect '(' after while.");
        self.expression();
        self.consume(TokenType::RightParen, "Expect ')' after condition.");

        let exit = self.emit_jump(OpCode::JumpIfFalse(0));
        self.emit_opcode(OpCode::Pop);
        self.statement();
        self.emit_loop(loop_start);
        self.patch_jump(exit);
        self.emit_opcode(OpCode::Pop);
    }

    fn for_statement(&mut self) {
        self.begin_scope();
        self.consume(TokenType::LeftParen, "Expect '(' after for.");
        if self.match_token(TokenType::Semicolon) {
        } else if self.match_token(TokenType::Var) {
            self.var_declaration();
        } else {
            self.expression_statement();
        }

        let mut loop_start = self.start_loop();
        let mut exit_jump: Option<usize> = None;

        if !self.match_token(TokenType::Semicolon) {
            self.expression();
            self.consume(TokenType::Semicolon, "Expect ';' after loop condition.");

            exit_jump = Some(self.emit_jump(OpCode::JumpIfFalse(0)));
            self.emit_opcode(OpCode::Pop);
        }

        if !self.match_token(TokenType::RightParen) {
            let body_jump = self.emit_jump(OpCode::Jump(0));
            let start = self.start_loop();
            self.expression();
            self.emit_opcode(OpCode::Pop);
            self.consume(TokenType::RightParen, "Expect ')' after for clauses.");
            self.emit_loop(loop_start);
            loop_start = start;
            self.patch_jump(body_jump);
        }

        self.statement();
        self.emit_loop(loop_start);

        if let Some(exit) = exit_jump {
            self.patch_jump(exit);
            self.emit_opcode(OpCode::Pop);
        }

        self.end_scope();
    }

    fn return_statement(&mut self) {
        if self.compiler.function_type == FunctionType::Script {
            self.error("Can't return from top-level code.");
        }

        if self.match_token(TokenType::Semicolon) {
            self.emit_return();
        } else {
            self.expression();
            self.consume(TokenType::Semicolon, "Expect ';' after return value.");
            self.emit_opcode(OpCode::Return);
        }
    }

    fn expression_statement(&mut self) {
        self.expression();
        self.consume(TokenType::Semicolon, "Expected ';' after expression");
        self.emit_opcode(OpCode::Pop);
    }

    fn variable(&mut self, can_assign: bool) {
        self.named_variable(self.previous, can_assign);
    }

    fn named_variable(&mut self, token: Token, can_assign: bool) {
        let (get_op, set_op): (OpCode, OpCode);
        if let Some(arg) = self.resolve_local(token) {
            set_op = OpCode::SetLocal(arg);
            get_op = OpCode::GetLocal(arg);
        } else if let Some(arg) = self.resolve_upvalue(token) {
            set_op = OpCode::SetUpvalue(arg);
            get_op = OpCode::GetUpvalue(arg);
        } else {
            let arg = self.identifier_constant(token);
            set_op = OpCode::SetGlobal(arg);
            get_op = OpCode::GetGlobal(arg);
        }

        if can_assign && self.match_token(TokenType::Equal) {
            self.expression();
            self.emit_opcode(set_op);
        } else {
            self.emit_opcode(get_op);
        }
    }

    fn expression(&mut self) {
        self.parse_precedence(Precedence::Assignment);
    }

    fn number(&mut self, _can_assign: bool) {
        let value = self.previous.lexeme.parse::<f64>();
        match value {
            Ok(value) => self.emit_constant(Value::Number(value)),
            Err(_) => self.error_at_current("Expected number when converting string to number."),
        }
    }

    fn grouping(&mut self, _can_assign: bool) {
        self.expression();
        self.consume(TokenType::RightParen, "Expect ')' after expression.");
    }

    fn unary(&mut self, _can_assign: bool) {
        let op_type = self.previous.token_type;
        self.parse_precedence(Precedence::Unary);
        match op_type {
            TokenType::Minus => self.emit_opcode(OpCode::Negate),
            TokenType::Bang => self.emit_opcode(OpCode::Not),
            _ => (), // Unreachable.
        }
    }

    fn binary(&mut self, _can_assign: bool) {
        let op_type = self.previous.token_type;
        let rule = self.parse_rules.get(&op_type).cloned().unwrap();
        self.parse_precedence(rule.precedence.next());
        match op_type {
            TokenType::Plus => self.emit_opcode(OpCode::Add),
            TokenType::Minus => self.emit_opcode(OpCode::Sub),
            TokenType::Star => self.emit_opcode(OpCode::Mul),
            TokenType::Slash => self.emit_opcode(OpCode::Div),
            TokenType::EqualEqual => self.emit_opcode(OpCode::Equal),
            TokenType::BangEqual => self.emit_opcode(OpCode::NotEqual),
            TokenType::Greater => self.emit_opcode(OpCode::Greater),
            TokenType::GreaterEqual => self.emit_opcode(OpCode::GreaterEqual),
            TokenType::Less => self.emit_opcode(OpCode::Less),
            TokenType::LessEqual => self.emit_opcode(OpCode::LessEqual),
            _ => (), // Unreachable.
        }
    }

    fn literal(&mut self, _can_assign: bool) {
        match self.previous.token_type {
            TokenType::False => self.emit_opcode(OpCode::False),
            TokenType::True => self.emit_opcode(OpCode::True),
            TokenType::Nil => self.emit_opcode(OpCode::Nil),
            _ => (), // Unreachable.
        }
    }

    fn string(&mut self, _can_assign: bool) {
        let lexeme = self.previous.lexeme;
        let value = &lexeme[1..lexeme.chars().count() - 1];
        self.emit_constant(Value::VString(LoxString::new(value.to_string())));
    }

    fn and_op(&mut self, _can_assign: bool) {
        let end = self.emit_jump(OpCode::JumpIfFalse(0));
        self.emit_opcode(OpCode::Pop);
        self.parse_precedence(Precedence::And);
        self.patch_jump(end);
    }

    fn or_op(&mut self, _can_assign: bool) {
        let else_jump = self.emit_jump(OpCode::JumpIfFalse(0));
        let end_jump = self.emit_jump(OpCode::Jump(0));
        self.patch_jump(else_jump);
        self.emit_opcode(OpCode::Pop);
        self.parse_precedence(Precedence::Or);
        self.patch_jump(end_jump);
    }

    fn call(&mut self, _can_assign: bool) {
        let arg_count = self.argument_list();
        self.emit_opcode(OpCode::Call(arg_count));
    }

    // helpers

    fn match_token(&mut self, ttype: TokenType) -> bool {
        if !self.check(ttype) {
            false
        } else {
            self.advance();
            true
        }
    }

    #[inline]
    fn check(&self, ttype: TokenType) -> bool {
        self.current.token_type == ttype
    }

    fn synchronize(&mut self) {
        if cfg!(feature = "debug") {
            println!("Synchronizing...");
        }
        self.panic_mode = false;
        while self.current.token_type != TokenType::Eof {
            if self.previous.token_type == TokenType::Semicolon {
                return;
            } else {
                match self.current.token_type {
                    TokenType::Class
                    | TokenType::Fun
                    | TokenType::Var
                    | TokenType::For
                    | TokenType::If
                    | TokenType::While
                    | TokenType::Print
                    | TokenType::Return => return,
                    _ => (),
                }
                self.advance();
            }
        }
    }

    fn parse_variable(&mut self, message: &str) -> usize {
        self.consume(TokenType::Identifier, message);

        self.declare_varible();
        if self.compiler.scope_depth > 0 {
            return 0;
        }

        self.identifier_constant(self.previous)
    }

    fn define_variable(&mut self, var: usize) {
        if self.compiler.scope_depth > 0 {
            self.mark_initialized();
            return;
        }
        self.emit_opcode(OpCode::DefineGlobal(var))
    }

    fn declare_varible(&mut self) {
        if self.compiler.scope_depth == 0 {
            return;
        }

        let name = self.previous;
        if self.compiler.is_local_defined(name) {
            self.error(&format!(
                "Already a variable with name {} in this scope.",
                name.lexeme
            ));
        }
        self.add_local(name);
    }

    fn add_local(&mut self, name: Token<'s>) {
        self.compiler.locals.push(Local::new(name, -1));
    }

    fn identifier_constant(&mut self, token: Token) -> usize {
        self.make_constant(Value::VString(LoxString::new(token.lexeme.to_string())))
    }

    fn mark_initialized(&mut self) {
        if self.compiler.scope_depth == 0 {
            return;
        }
        let i = self.compiler.locals.len() - 1;
        self.compiler.locals[i].depth = self.compiler.scope_depth;
    }

    fn patch_jump(&mut self, then: usize) {
        let offset = self.compiler.function.chunk.code.len() - then - 1;
        let instr = self.compiler.function.chunk.code[then];
        self.compiler.function.chunk.code[then] = match instr {
            OpCode::JumpIfFalse(_) => OpCode::JumpIfFalse(offset),
            OpCode::Jump(_) => OpCode::Jump(offset),
            _ => panic!("No jump instruction found"),
        };
    }

    fn start_loop(&self) -> usize {
        self.compiler.function.chunk.code.len()
    }

    fn code_len(&self) -> usize {
        self.compiler.function.chunk.code.len()
    }

    fn function(&mut self, function_type: FunctionType) {
        self.push_compiler(function_type);
        self.begin_scope();
        self.consume(TokenType::LeftParen, "Expect '(' after function name.");

        if !self.check(TokenType::RightParen) {
            loop {
                self.compiler.function.arity += 1;
                if self.compiler.function.arity > 255 {
                    self.error_at_current("Can't have more than 255 parameters.");
                }
                let constant = self.parse_variable("Expect parameter name.");
                self.define_variable(constant);
                if !self.match_token(TokenType::Comma) {
                    break;
                }
            }
        }

        self.consume(TokenType::RightParen, "Expect ')' after parameters.");
        self.consume(TokenType::LeftBrace, "Expect '{' before function body.");
        self.block();
        let function = self.pop_compiler();
        let index = self.make_constant(Value::Function(function));
        self.emit_opcode(OpCode::Closure(index));
    }

    fn push_compiler(&mut self, function_type: FunctionType) {
        let name = if function_type != FunctionType::Script {
            self.previous.lexeme.to_string()
        } else {
            String::from("<script>")
        };
        let new_compiler = Compiler::new(name, function_type);
        let old_compiler = mem::replace(&mut self.compiler, new_compiler);
        self.compiler.enclosing = Some(old_compiler);
    }

    fn pop_compiler(&mut self) -> Function {
        self.emit_return();
        match self.compiler.enclosing.take() {
            Some(enclosing) => {
                let compiler = mem::replace(&mut self.compiler, enclosing);
                compiler.function
            }
            None => panic!("Didn't find an enclosing compiler"),
        }
    }

    fn argument_list(&mut self) -> usize {
        let mut arg_count = 0;
        if !self.check(TokenType::RightParen) {
            loop {
                self.expression();
                if arg_count == 255 {
                    self.error("Can't have more than 255 arguments.");
                }
                arg_count += 1;
                if !self.match_token(TokenType::Comma) {
                    break;
                }
            }
        }
        self.consume(TokenType::RightParen, "Expect ')' after arguments.");
        arg_count
    }

    pub fn resolve_local(&mut self, name: Token) -> Option<usize> {
        let mut errors: Vec<&str> = Vec::new();
        let result = self.compiler.resolve_local(name, &mut errors);
        while let Some(err) = errors.pop() {
            self.error(err);
        }
        result
    }

    pub fn resolve_upvalue(&mut self, name: Token) -> Option<usize> {
        let mut errors: Vec<&str> = Vec::new();
        let result = self.compiler.resolve_upvalue(name, &mut errors);
        while let Some(err) = errors.pop() {
            self.error(err);
        }
        result
    }

    // chunk manipulation

    fn emit_opcode(&mut self, opcode: OpCode) {
        self.compiler
            .function
            .chunk
            .write(opcode, self.previous.line);
    }

    fn emit_return(&mut self) {
        self.compiler
            .function
            .chunk
            .write(OpCode::Nil, self.previous.line);
        self.compiler
            .function
            .chunk
            .write(OpCode::Return, self.previous.line);
    }

    fn emit_constant(&mut self, constant: Value) {
        let index = self.make_constant(constant);
        self.compiler
            .function
            .chunk
            .write(OpCode::Constant(index), self.previous.line);
    }

    fn make_constant(&mut self, constant: Value) -> usize {
        self.compiler.function.chunk.add_constant(constant)
    }

    fn emit_jump(&mut self, jump: OpCode) -> usize {
        self.emit_opcode(jump);
        self.compiler.function.chunk.code.len() - 1
    }

    fn emit_loop(&mut self, start: usize) {
        self.emit_opcode(OpCode::Loop(self.code_len() - start));
    }

    // error handling

    fn error_at_current(&self, message: &str) {
        self.error_at(&self.current, message);
    }

    fn error(&self, message: &str) {
        self.error_at(&self.previous, message);
    }

    fn error_at(&self, token: &Token, message: &str) {
        if self.panic_mode {
            return;
        }
        print!("[line {}] Error", token.line);

        if token.token_type == TokenType::Eof {
            print!(" at end");
        } else if token.token_type == TokenType::Error {
        } else {
            print!(" at '{}'", token.lexeme);
        }

        println!(": {}", message);
    }
}

pub fn compile(code: &str) -> Result<Function, InterpretError> {
    let mut parser = Parser::new(code);
    parser.compile()
}

struct Compiler<'a> {
    enclosing: Option<Box<Compiler<'a>>>,
    scope_depth: isize,
    locals: Vec<Local<'a>>,
    function: Function,
    function_type: FunctionType,
}

impl<'a> Compiler<'a> {
    pub fn new(name: String, function_type: FunctionType) -> Box<Self> {
        let mut compiler = Compiler {
            enclosing: None,
            scope_depth: 0,
            locals: Vec::new(),
            function: Function::new(name),
            function_type,
        };
        compiler.locals.push(Local::new(Token::syntethic(""), 0));
        Box::new(compiler)
    }

    pub fn is_local_defined(&self, name: Token) -> bool {
        for local in self.locals.iter().rev() {
            if local.depth != -1 && local.depth < self.scope_depth {
                return false;
            }
            if local.name.lexeme == name.lexeme {
                return true;
            }
        }
        false
    }

    pub fn resolve_local(&mut self, name: Token, errors: &mut Vec<&str>) -> Option<usize> {
        for (i, local) in self.locals.iter().enumerate().rev() {
            if name.lexeme == local.name.lexeme {
                if local.depth == -1 {
                    errors.push("Can't read local variable in its own initializer.");
                }
                return Some(i);
            }
        }
        None
    }

    pub fn resolve_upvalue(&mut self, name: Token, errors: &mut Vec<&str>) -> Option<usize> {
        if let Some(env) = self.enclosing.as_mut() {
            if let Some(index) = env.resolve_local(name, errors) {
                env.locals[index].is_captured = true;
                return Some(self.add_upvalue(index, true));
            }
            if let Some(index) = env.resolve_upvalue(name, errors) {
                return Some(self.add_upvalue(index, false));
            }
        }
        None
    }

    pub fn add_upvalue(&mut self, index: usize, is_local: bool) -> usize {
        for (i, upvalue) in self.function.upvalues.iter().enumerate() {
            if upvalue.index == index && is_local == upvalue.is_local {
                return i;
            }
        }
        let upvalue = FunctionUpvalue::new(index, is_local);
        self.function.upvalues.push(upvalue);
        self.function.upvalues.len() - 1
    }
}

struct Local<'a> {
    name: Token<'a>,
    depth: isize,
    is_captured: bool,
}

impl<'a> Local<'a> {
    pub fn new(name: Token<'a>, depth: isize) -> Self {
        Local {
            name,
            depth,
            is_captured: false,
        }
    }
}
