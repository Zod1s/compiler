use crate::{
    chunk::{Chunk, Disassembler, OpCode},
    gc::{Gc, GcRef},
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

struct Parser<'s> {
    current: Token<'s>,
    previous: Token<'s>,
    scanner: Scanner<'s>,
    gc: &'s mut Gc,
    had_error: bool,
    panic_mode: bool,
    parse_rules: HashMap<TokenType, ParseRule<'s>>,
    compiler: Box<Compiler<'s>>,
    class_compiler: Option<Box<ClassCompiler>>,
}

impl<'s> Parser<'s> {
    fn new(code: &'s str, gc: &'s mut Gc) -> Self {
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
        rule(TokenType::Dot, None, Some(Parser::dot), Precedence::Call);
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
        rule(
            TokenType::Rem,
            None,
            Some(Parser::binary),
            Precedence::Factor,
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
        rule(
            TokenType::Super,
            Some(Parser::super_),
            None,
            Precedence::None,
        );
        rule(TokenType::This, Some(Parser::this), None, Precedence::None);
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

        let compiler = Compiler::new(gc.intern("script".to_owned()), FunctionType::Script);

        Parser {
            current: Token::syntethic(""),
            previous: Token::syntethic(""),
            gc,
            scanner: Scanner::new(code),
            had_error: false,
            panic_mode: false,
            parse_rules,
            compiler,
            class_compiler: None,
        }
    }

    fn compile(mut self) -> Result<GcRef<Function>, InterpretError> {
        self.advance();
        while !self.match_token(TokenType::Eof) {
            self.declaration();
        }
        self.consume(TokenType::Eof, "Expect end of expression.");
        self.emit_return();
        if cfg!(feature = "debug_trace_execution") && !self.had_error {
            let disassembler = Disassembler::new(self.gc, &self.compiler.function.chunk, None);
            disassembler.disassemble("code");
        }
        if self.had_error {
            Err(InterpretError::Compile)
        } else {
            Ok(self.gc.alloc(self.compiler.function))
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
        }
    }

    fn consume(&mut self, ttype: TokenType, message: &str) {
        if self.current.token_type == ttype {
            self.advance();
        } else {
            self.error_at_current(message);
        }
    }

    fn parse_precedence(&mut self, precedence: Precedence) {
        self.advance();
        let prefix_rule = match self.get_rule(&self.previous.token_type).prefix {
            None => {
                self.error("Expect expression.");
                return;
            }
            Some(rule) => rule,
        };

        let can_assign = precedence <= Precedence::Assignment;
        prefix_rule(self, can_assign);

        while precedence <= self.get_rule(&self.current.token_type).precedence {
            self.advance();
            let infix_rule = self.get_rule(&self.previous.token_type).infix.unwrap();
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
        } else if self.match_token(TokenType::Class) {
            self.class_declaration();
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

        self.consume(
            TokenType::Semicolon,
            "Expect ';' after variable declaration.",
        );

        self.define_variable(global);
    }

    fn fun_declaration(&mut self) {
        let global = self.parse_variable("Expect function name.");
        self.mark_initialized();
        self.function(FunctionType::Function);
        self.define_variable(global);
    }

    fn class_declaration(&mut self) {
        self.consume(TokenType::Identifier, "Expect class name.");

        let class_name = self.previous;
        let name_constant = self.identifier_constant(self.previous);
        self.declare_varible();

        self.emit_opcode(OpCode::Class(name_constant));

        self.define_variable(name_constant);

        let old_class_compiler = self.class_compiler.take();
        let new_class_compiler = Box::new(ClassCompiler {
            enclosing: old_class_compiler,
            has_superclass: false,
        });
        self.class_compiler.replace(new_class_compiler);

        if self.match_token(TokenType::LessPipe) {
            self.consume(TokenType::Identifier, "Expect superclass name.");
            self.variable(false);

            if class_name.lexeme == self.previous.lexeme {
                self.error("A class can't inherit from itself.");
            }

            self.begin_scope();
            self.add_local(Token::syntethic("super"));
            self.define_variable(0);

            self.named_variable(class_name, false);
            self.emit_opcode(OpCode::Inherit);
            self.class_compiler.as_mut().unwrap().has_superclass = true;
        }

        self.named_variable(class_name, false);

        self.consume(TokenType::LeftBrace, "Expect '{' before class body.");

        while !(self.check(TokenType::RightBrace) || self.check(TokenType::Eof)) {
            self.method();
        }

        self.consume(TokenType::RightBrace, "Expect '}' after class body.");
        self.emit_pop();

        if self.class_compiler.as_ref().unwrap().has_superclass {
            self.end_scope();
        }

        match self.class_compiler.take() {
            Some(comp) => self.class_compiler = comp.enclosing,
            None => self.class_compiler = None,
        }
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
        self.consume(TokenType::Semicolon, "Expect ';' after value.");
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
                    self.emit_pop();
                }
                self.compiler.locals.pop();
            }
        }
    }

    fn if_statement(&mut self) {
        self.consume(TokenType::LeftParen, "Expect '(' after 'if'.");
        self.expression();
        self.consume(TokenType::RightParen, "Expect ')' after condition.");

        let then = self.emit_jump(OpCode::JumpIfFalse(0));
        self.emit_pop();
        self.statement();
        let else_jump = self.emit_jump(OpCode::Jump(0));
        self.patch_jump(then);
        self.emit_pop();
        if self.match_token(TokenType::Else) {
            self.statement();
        }
        self.patch_jump(else_jump);
    }

    fn while_statement(&mut self) {
        let loop_start = self.start_loop();
        self.consume(TokenType::LeftParen, "Expect '(' after 'while'.");
        self.expression();
        self.consume(TokenType::RightParen, "Expect ')' after condition.");

        let exit = self.emit_jump(OpCode::JumpIfFalse(0));
        self.emit_pop();
        self.statement();
        self.emit_loop(loop_start);
        self.patch_jump(exit);
        self.emit_pop();
    }

    fn for_statement(&mut self) {
        self.begin_scope();
        self.consume(TokenType::LeftParen, "Expect '(' after 'for'.");
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
            self.emit_pop();
        }

        if !self.match_token(TokenType::RightParen) {
            let body_jump = self.emit_jump(OpCode::Jump(0));
            let start = self.start_loop();
            self.expression();
            self.emit_pop();
            self.consume(TokenType::RightParen, "Expect ')' after for clauses.");
            self.emit_loop(loop_start);
            loop_start = start;
            self.patch_jump(body_jump);
        }

        self.statement();
        self.emit_loop(loop_start);

        if let Some(exit) = exit_jump {
            self.patch_jump(exit);
            self.emit_pop();
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
            if self.compiler.function_type == FunctionType::Initializer {
                self.error("Can't return a value from an initializer.");
            }
            self.expression();
            self.consume(TokenType::Semicolon, "Expect ';' after return value.");
            self.emit_opcode(OpCode::Return);
        }
    }

    fn expression_statement(&mut self) {
        self.expression();
        self.consume(TokenType::Semicolon, "Expect ';' after expression.");
        self.emit_pop();
    }

    fn variable(&mut self, can_assign: bool) {
        self.named_variable(self.previous, can_assign);
    }

    fn named_variable(&mut self, token: Token, can_assign: bool) {
        let (get_op, set_op);
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

    fn method(&mut self) {
        self.consume(TokenType::Identifier, "Expect method name.");
        let constant = self.identifier_constant(self.previous);
        let ftype = if self.previous.lexeme == "init" {
            FunctionType::Initializer
        } else {
            FunctionType::Method
        };

        self.function(ftype);
        self.emit_opcode(OpCode::Method(constant));
    }

    fn expression(&mut self) {
        self.parse_precedence(Precedence::Assignment);
    }

    fn number(&mut self, _can_assign: bool) {
        let value = self.previous.lexeme.parse::<f64>();
        match value {
            Ok(value) => self.emit_constant(Value::Number(value)),
            Err(_) => self.error_at_current("Expect number when converting string to number."),
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
        let rule = self.get_rule(&op_type).clone();
        self.parse_precedence(rule.precedence.next());
        match op_type {
            TokenType::Plus => self.emit_opcode(OpCode::Add),
            TokenType::Minus => self.emit_opcode(OpCode::Sub),
            TokenType::Rem => self.emit_opcode(OpCode::Rem),
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
        let string = self.gc.intern(value.to_string());
        self.emit_constant(Value::VString(string));
    }

    fn and_op(&mut self, _can_assign: bool) {
        let end = self.emit_jump(OpCode::JumpIfFalse(0));
        self.emit_pop();
        self.parse_precedence(Precedence::And);
        self.patch_jump(end);
    }

    fn or_op(&mut self, _can_assign: bool) {
        let else_jump = self.emit_jump(OpCode::JumpIfFalse(0));
        let end_jump = self.emit_jump(OpCode::Jump(0));
        self.patch_jump(else_jump);
        self.emit_pop();
        self.parse_precedence(Precedence::Or);
        self.patch_jump(end_jump);
    }

    fn call(&mut self, _can_assign: bool) {
        let arg_count = self.argument_list();
        self.emit_opcode(OpCode::Call(arg_count));
    }

    fn dot(&mut self, can_assign: bool) {
        self.consume(TokenType::Identifier, "Expect property name after '.'.");
        let name = self.identifier_constant(self.previous);
        if can_assign && self.match_token(TokenType::Equal) {
            self.expression();
            self.emit_opcode(OpCode::SetProperty(name));
        } else if self.match_token(TokenType::LeftParen) {
            let arg_count = self.argument_list();
            self.emit_opcode(OpCode::Invoke((name, arg_count)));
        } else {
            self.emit_opcode(OpCode::GetProperty(name));
        }
    }

    fn this(&mut self, _can_assign: bool) {
        if self.class_compiler.is_none() {
            self.error("Can't use 'this' outside of a class.");
            return;
        }
        self.variable(false);
    }

    fn super_(&mut self, _can_assign: bool) {
        if let Some(current_class) = self.class_compiler.as_ref() {
            if !current_class.has_superclass {
                self.error("Can't use 'super' in a class with no superclass.");
            }
        } else {
            self.error("Can't use 'super' outside of a class.");
        }
        self.consume(TokenType::Dot, "Expect '.' after 'super'.");
        self.consume(TokenType::Identifier, "Expect superclass method name.");
        let name = self.identifier_constant(self.previous);
        self.named_variable(Token::syntethic("this"), false);
        if self.match_token(TokenType::LeftParen) {
            let arg_count = self.argument_list();
            self.named_variable(Token::syntethic("super"), false);
            self.emit_opcode(OpCode::SuperInvoke((name, arg_count)));
        } else {
            self.named_variable(Token::syntethic("super"), false);
            self.emit_opcode(OpCode::GetSuper(name));
        }
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
            self.error("Already a variable with this name in this scope.");
        }
        self.add_local(name);
    }

    fn add_local(&mut self, name: Token<'s>) {
        self.compiler.locals.push(Local {
            name,
            depth: -1,
            is_captured: false,
        });
    }

    fn identifier_constant(&mut self, token: Token) -> usize {
        let string = self.gc.intern(token.lexeme.to_string());
        self.make_constant(Value::VString(string))
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
        let fn_id = self.gc.alloc(function);
        let index = self.make_constant(Value::Function(fn_id));
        self.emit_opcode(OpCode::Closure(index));
    }

    fn push_compiler(&mut self, function_type: FunctionType) {
        let name = self.gc.intern(self.previous.lexeme.to_owned());
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

    fn resolve_local(&mut self, name: Token) -> Option<usize> {
        let mut errors: Vec<&str> = Vec::new();
        let result = self.compiler.resolve_local(name, &mut errors);
        while let Some(err) = errors.pop() {
            self.error(err);
        }
        result
    }

    fn resolve_upvalue(&mut self, name: Token) -> Option<usize> {
        let mut errors: Vec<&str> = Vec::new();
        let result = self.compiler.resolve_upvalue(name, &mut errors);
        while let Some(err) = errors.pop() {
            self.error(err);
        }
        result
    }

    fn get_rule(&self, key: &TokenType) -> &ParseRule<'s> {
        self.parse_rules.get(key).unwrap()
    }

    // chunk manipulation

    fn emit_opcode(&mut self, opcode: OpCode) {
        self.compiler
            .function
            .chunk
            .write(opcode, self.previous.line);
    }

    fn emit_return(&mut self) {
        if self.compiler.function_type == FunctionType::Initializer {
            self.emit_opcode(OpCode::GetLocal(0));
            self.emit_opcode(OpCode::Return);
        } else {
            self.emit_opcode(OpCode::ReturnNil);
        }
    }

    fn emit_constant(&mut self, constant: Value) {
        let index = self.make_constant(constant);
        self.emit_opcode(OpCode::Constant(index));
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

    fn emit_pop(&mut self) {
        self.emit_opcode(OpCode::Pop);
    }

    // error handling

    fn error_at_current(&mut self, message: &str) {
        self.error_at(self.current, message);
    }

    fn error(&mut self, message: &str) {
        self.error_at(self.previous, message);
    }

    fn error_at(&mut self, token: Token, message: &str) {
        if self.panic_mode {
            return;
        }

        self.had_error = true;
        self.panic_mode = true;

        eprint!("[line {}] Error", token.line);

        if token.token_type == TokenType::Eof {
            eprint!(" at end");
        } else if token.token_type == TokenType::Error {
        } else {
            eprint!(" at '{}'", token.lexeme);
        }

        eprintln!(": {}", message);
    }
}

pub fn compile(code: &str, gc: &mut Gc) -> Result<GcRef<Function>, InterpretError> {
    let parser = Parser::new(code, gc);
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
    fn new(name: GcRef<LoxString>, function_type: FunctionType) -> Box<Self> {
        let mut compiler = Compiler {
            enclosing: None,
            scope_depth: 0,
            locals: Vec::new(),
            function: Function {
                arity: 0,
                chunk: Chunk::new(),
                name,
                upvalues: Vec::new(),
            },
            function_type,
        };
        let token = match function_type {
            FunctionType::Method | FunctionType::Initializer => Local {
                name: Token::syntethic("this"),
                depth: 0,
                is_captured: false,
            },
            _ => Local {
                name: Token::syntethic(""),
                depth: 0,
                is_captured: false,
            },
        };
        compiler.locals.push(token);
        Box::new(compiler)
    }

    fn is_local_defined(&self, name: Token) -> bool {
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

    fn resolve_local(&mut self, name: Token, errors: &mut Vec<&str>) -> Option<usize> {
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

    fn resolve_upvalue(&mut self, name: Token, errors: &mut Vec<&str>) -> Option<usize> {
        if let Some(env) = self.enclosing.as_mut() {
            if let Some(index) = env.resolve_local(name, errors) {
                env.locals[index].is_captured = true;
                return Some(self.add_upvalue(index, true));
            } else if let Some(index) = env.resolve_upvalue(name, errors) {
                return Some(self.add_upvalue(index, false));
            }
        }
        None
    }

    fn add_upvalue(&mut self, index: usize, is_local: bool) -> usize {
        for (i, upvalue) in self.function.upvalues.iter().enumerate() {
            if upvalue.index == index && is_local == upvalue.is_local {
                return i;
            }
        }
        let upvalue = FunctionUpvalue { index, is_local };
        self.function.upvalues.push(upvalue);
        self.function.upvalues.len() - 1
    }
}

struct Local<'a> {
    name: Token<'a>,
    depth: isize,
    is_captured: bool,
}

// impl<'a> Local<'a> {
//     fn new(name: Token<'a>, depth: isize) -> Self {
//         Self {
//             name,
//             depth,
//             is_captured: false,
//         }
//     }
// }

struct ClassCompiler {
    enclosing: Option<Box<ClassCompiler>>,
    has_superclass: bool,
}

// impl ClassCompiler {
//     fn new(enclosing: Option<Box<ClassCompiler>>) -> Box<Self> {
//         Box::new(Self {
//             enclosing,
//             has_superclass: false,
//         })
//     }
// }
