//! JavaScript Engine
//!
//! A simple JavaScript interpreter for WebbOS.

use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use alloc::boxed::Box;
use alloc::collections::BTreeMap;

use crate::browser::BrowserError;
use crate::println;

/// JavaScript value types
#[derive(Debug, Clone)]
pub enum Value {
    Undefined,
    Null,
    Boolean(bool),
    Number(f64),
    String(String),
    Object(Object),
    Array(Vec<Value>),
    Function(Function),
}

/// Simple float truncation (since f64::trunc is not available in no_std)
fn trunc_f64(n: f64) -> f64 {
    // Get integer part by casting
    let int_part = n as i64;
    if n >= 0.0 || n == int_part as f64 {
        int_part as f64
    } else {
        // For negative numbers, we need to subtract 1
        (int_part - 1) as f64
    }
}

/// Convert integer to string
fn int_to_string(n: i64) -> String {
    if n == 0 {
        return String::from("0");
    }
    
    let mut result = String::new();
    let mut num = n.abs();
    
    while num > 0 {
        let digit = (num % 10) as u8;
        result.insert(0, (b'0' + digit) as char);
        num /= 10;
    }
    
    if n < 0 {
        result.insert(0, '-');
    }
    
    result
}

impl Value {
    /// Convert to string
    pub fn to_string(&self) -> String {
        match self {
            Value::Undefined => String::from("undefined"),
            Value::Null => String::from("null"),
            Value::Boolean(b) => String::from(if *b { "true" } else { "false" }),
            Value::Number(n) => {
                // Simple float to string conversion
                if *n == trunc_f64(*n) {
                    // Integer
                    int_to_string(*n as i64)
                } else {
                    // Float - simplified
                    String::from("0.0")
                }
            }
            Value::String(s) => s.clone(),
            Value::Object(_) => String::from("[object Object]"),
            Value::Array(_) => String::from("[object Array]"),
            Value::Function(_) => String::from("[object Function]"),
        }
    }

    /// Check if truthy
    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Undefined | Value::Null => false,
            Value::Boolean(b) => *b,
            Value::Number(n) => *n != 0.0 && !n.is_nan(),
            Value::String(s) => !s.is_empty(),
            Value::Object(_) | Value::Array(_) | Value::Function(_) => true,
        }
    }
}

/// JavaScript object
#[derive(Debug, Clone)]
pub struct Object {
    pub properties: BTreeMap<String, Value>,
    pub prototype: Option<Box<Object>>,
}

impl Object {
    pub fn new() -> Self {
        Self {
            properties: BTreeMap::new(),
            prototype: None,
        }
    }

    pub fn get(&self, key: &str) -> Value {
        self.properties.get(key).cloned()
            .unwrap_or(Value::Undefined)
    }

    pub fn set(&mut self, key: &str, value: Value) {
        self.properties.insert(String::from(key), value);
    }
}

/// JavaScript function
#[derive(Debug, Clone)]
pub struct Function {
    pub name: String,
    pub params: Vec<String>,
    pub body: Vec<Statement>,
    pub native: Option<fn(&mut Environment, Vec<Value>) -> Value>,
}

/// Environment for variable scoping
pub struct Environment {
    /// Variable scopes
    scopes: Vec<BTreeMap<String, Value>>,
    /// Global object
    global: Object,
    /// Output buffer for console.log
    output: String,
}

impl Environment {
    pub fn new() -> Self {
        let mut env = Self {
            scopes: vec![BTreeMap::new()],
            global: Object::new(),
            output: String::new(),
        };

        // Add built-in functions
        env.global.set("console", Value::Object(Object::new()));
        
        env
    }

    /// Define variable in current scope
    pub fn define(&mut self, name: &str, value: Value) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(String::from(name), value);
        }
    }

    /// Get variable value
    pub fn get(&self, name: &str) -> Value {
        // Search from innermost to outermost scope
        for scope in self.scopes.iter().rev() {
            if let Some(value) = scope.get(name) {
                return value.clone();
            }
        }
        
        // Check global object
        if let Some(value) = self.global.properties.get(name) {
            return value.clone();
        }
        
        Value::Undefined
    }

    /// Set variable value
    pub fn set(&mut self, name: &str, value: Value) {
        for scope in self.scopes.iter_mut().rev() {
            if scope.contains_key(name) {
                scope.insert(String::from(name), value);
                return;
            }
        }
        
        // Define in current scope if not found
        self.define(name, value);
    }

    /// Push new scope
    pub fn push_scope(&mut self) {
        self.scopes.push(BTreeMap::new());
    }

    /// Pop scope
    pub fn pop_scope(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }

    /// Log output
    pub fn log(&mut self, msg: &str) {
        self.output.push_str(msg);
        self.output.push('\n');
        println!("[js] {}", msg);
    }

    /// Get output
    pub fn get_output(&self) -> &str {
        &self.output
    }
}

/// Token types
#[derive(Debug, Clone)]
enum Token {
    Identifier(String),
    Number(f64),
    String(String),
    Keyword(String),
    Operator(String),
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Semicolon,
    Comma,
    Dot,
    Colon,
    EOF,
}

/// JavaScript keywords
const KEYWORDS: &[&str] = &[
    "var", "let", "const", "function", "return", "if", "else", "while",
    "for", "break", "continue", "true", "false", "null", "undefined",
    "new", "this", "typeof", "instanceof", "in", "of",
];

/// Tokenizer
struct Tokenizer<'a> {
    input: &'a [u8],
    pos: usize,
}

impl<'a> Tokenizer<'a> {
    fn new(input: &'a [u8]) -> Self {
        Self { input, pos: 0 }
    }

    fn peek(&self) -> Option<u8> {
        self.input.get(self.pos).copied()
    }

    fn next(&mut self) -> Option<u8> {
        let ch = self.peek()?;
        self.pos += 1;
        Some(ch)
    }

    fn skip_whitespace(&mut self) {
        while let Some(ch) = self.peek() {
            if ch.is_ascii_whitespace() {
                self.next();
            } else {
                break;
            }
        }
    }

    fn read_identifier(&mut self) -> String {
        let mut ident = String::new();
        while let Some(ch) = self.peek() {
            if ch.is_ascii_alphanumeric() || ch == b'_' || ch == b'$' {
                ident.push(ch as char);
                self.next();
            } else {
                break;
            }
        }
        ident
    }

    fn read_number(&mut self) -> f64 {
        let mut num = String::new();
        let mut has_dot = false;

        while let Some(ch) = self.peek() {
            if ch.is_ascii_digit() {
                num.push(ch as char);
                self.next();
            } else if ch == b'.' && !has_dot {
                has_dot = true;
                num.push(ch as char);
                self.next();
            } else {
                break;
            }
        }

        num.parse().unwrap_or(0.0)
    }

    fn read_string(&mut self, quote: u8) -> String {
        let mut s = String::new();
        self.next(); // consume opening quote

        while let Some(ch) = self.peek() {
            if ch == quote {
                self.next(); // consume closing quote
                break;
            }
            if ch == b'\\' {
                self.next();
                if let Some(escaped) = self.next() {
                    match escaped {
                        b'n' => s.push('\n'),
                        b't' => s.push('\t'),
                        b'r' => s.push('\r'),
                        b'\\' => s.push('\\'),
                        b'"' => s.push('"'),
                        b'\'' => s.push('\''),
                        _ => s.push(escaped as char),
                    }
                }
            } else {
                s.push(ch as char);
                self.next();
            }
        }

        s
    }

    fn tokenize(&mut self) -> Vec<Token> {
        let mut tokens = Vec::new();

        loop {
            self.skip_whitespace();

            match self.peek() {
                None => break,
                Some(b'(') => { tokens.push(Token::LParen); self.next(); }
                Some(b')') => { tokens.push(Token::RParen); self.next(); }
                Some(b'{') => { tokens.push(Token::LBrace); self.next(); }
                Some(b'}') => { tokens.push(Token::RBrace); self.next(); }
                Some(b'[') => { tokens.push(Token::LBracket); self.next(); }
                Some(b']') => { tokens.push(Token::RBracket); self.next(); }
                Some(b';') => { tokens.push(Token::Semicolon); self.next(); }
                Some(b',') => { tokens.push(Token::Comma); self.next(); }
                Some(b'.') => { tokens.push(Token::Dot); self.next(); }
                Some(b':') => { tokens.push(Token::Colon); self.next(); }
                Some(b'"') | Some(b'\'') => {
                    let quote = self.peek().unwrap();
                    let s = self.read_string(quote);
                    tokens.push(Token::String(s));
                }
                Some(ch) if ch.is_ascii_digit() => {
                    let n = self.read_number();
                    tokens.push(Token::Number(n));
                }
                Some(ch) if ch.is_ascii_alphabetic() || ch == b'_' || ch == b'$' => {
                    let ident = self.read_identifier();
                    if KEYWORDS.contains(&ident.as_str()) {
                        tokens.push(Token::Keyword(ident));
                    } else {
                        tokens.push(Token::Identifier(ident));
                    }
                }
                Some(ch) => {
                    // Operators
                    let mut op = String::new();
                    op.push(ch as char);
                    self.next();
                    
                    // Check for two-character operators
                    if let Some(next) = self.peek() {
                        let two = [op.as_bytes()[0], next];
                        let two_str = core::str::from_utf8(&two).unwrap_or("");
                        if ["==", "!=", "<=", ">=", "&&", "||", "++", "--", "+=", "-=", "*=", "/="].contains(&two_str) {
                            op.push(next as char);
                            self.next();
                        }
                    }
                    
                    tokens.push(Token::Operator(op));
                }
            }
        }

        tokens.push(Token::EOF);
        tokens
    }
}

/// Statement types
#[derive(Debug, Clone)]
enum Statement {
    VarDecl(String, Option<Expr>),
    LetDecl(String, Option<Expr>),
    ConstDecl(String, Expr),
    Expr(Expr),
    Return(Option<Expr>),
    If(Expr, Box<Statement>, Option<Box<Statement>>),
    While(Expr, Box<Statement>),
    Block(Vec<Statement>),
    FunctionDecl(String, Vec<String>, Vec<Statement>),
}

/// Expression types
#[derive(Debug, Clone)]
enum Expr {
    Identifier(String),
    Number(f64),
    String(String),
    Boolean(bool),
    Null,
    Undefined,
    Binary(String, Box<Expr>, Box<Expr>),
    Unary(String, Box<Expr>),
    Call(Box<Expr>, Vec<Expr>),
    Member(Box<Expr>, String),
    Array(Vec<Expr>),
    Object(Vec<(String, Expr)>),
    Assign(Box<Expr>, Box<Expr>),
}

/// Parser
struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.pos]
    }

    fn next(&mut self) -> Token {
        let tok = self.tokens[self.pos].clone();
        if self.pos < self.tokens.len() - 1 {
            self.pos += 1;
        }
        tok
    }

    fn expect(&mut self, expected: Token) -> Result<(), BrowserError> {
        if core::mem::discriminant(self.peek()) == core::mem::discriminant(&expected) {
            self.next();
            Ok(())
        } else {
            Err(BrowserError::JsError)
        }
    }

    fn parse(&mut self) -> Result<Vec<Statement>, BrowserError> {
        let mut stmts = Vec::new();
        while !matches!(self.peek(), Token::EOF) {
            stmts.push(self.parse_statement()?);
        }
        Ok(stmts)
    }

    fn parse_statement(&mut self) -> Result<Statement, BrowserError> {
        match self.peek() {
            Token::Keyword(kw) => {
                match kw.as_str() {
                    "var" => self.parse_var_decl(),
                    "let" => self.parse_let_decl(),
                    "const" => self.parse_const_decl(),
                    "function" => self.parse_function_decl(),
                    "return" => self.parse_return(),
                    "if" => self.parse_if(),
                    "while" => self.parse_while(),
                    _ => Err(BrowserError::JsError),
                }
            }
            Token::LBrace => self.parse_block(),
            _ => {
                let expr = self.parse_expr()?;
                Ok(Statement::Expr(expr))
            }
        }
    }

    fn parse_var_decl(&mut self) -> Result<Statement, BrowserError> {
        self.next(); // consume 'var'
        let name = match self.next() {
            Token::Identifier(n) => n,
            _ => return Err(BrowserError::JsError),
        };

        let init = if matches!(self.peek(), Token::Operator(op) if op == "=") {
            self.next(); // consume '='
            Some(self.parse_expr()?)
        } else {
            None
        };

        if matches!(self.peek(), Token::Semicolon) {
            self.next();
        }

        Ok(Statement::VarDecl(name, init))
    }

    fn parse_let_decl(&mut self) -> Result<Statement, BrowserError> {
        self.next(); // consume 'let'
        let name = match self.next() {
            Token::Identifier(n) => n,
            _ => return Err(BrowserError::JsError),
        };

        let init = if matches!(self.peek(), Token::Operator(op) if op == "=") {
            self.next(); // consume '='
            Some(self.parse_expr()?)
        } else {
            None
        };

        if matches!(self.peek(), Token::Semicolon) {
            self.next();
        }

        Ok(Statement::LetDecl(name, init))
    }

    fn parse_const_decl(&mut self) -> Result<Statement, BrowserError> {
        self.next(); // consume 'const'
        let name = match self.next() {
            Token::Identifier(n) => n,
            _ => return Err(BrowserError::JsError),
        };

        self.expect(Token::Operator(String::from("=")))?;
        let init = self.parse_expr()?;

        if matches!(self.peek(), Token::Semicolon) {
            self.next();
        }

        Ok(Statement::ConstDecl(name, init))
    }

    fn parse_function_decl(&mut self) -> Result<Statement, BrowserError> {
        self.next(); // consume 'function'
        let name = match self.next() {
            Token::Identifier(n) => n,
            _ => return Err(BrowserError::JsError),
        };

        self.expect(Token::LParen)?;
        let params = self.parse_params()?;
        self.expect(Token::RParen)?;

        let body = self.parse_block_body()?;

        Ok(Statement::FunctionDecl(name, params, body))
    }

    fn parse_params(&mut self) -> Result<Vec<String>, BrowserError> {
        let mut params = Vec::new();
        
        while !matches!(self.peek(), Token::RParen) {
            match self.next() {
                Token::Identifier(n) => params.push(n),
                _ => return Err(BrowserError::JsError),
            }

            if matches!(self.peek(), Token::Comma) {
                self.next();
            } else {
                break;
            }
        }

        Ok(params)
    }

    fn parse_return(&mut self) -> Result<Statement, BrowserError> {
        self.next(); // consume 'return'
        
        let expr = if matches!(self.peek(), Token::Semicolon) {
            None
        } else {
            Some(self.parse_expr()?)
        };

        if matches!(self.peek(), Token::Semicolon) {
            self.next();
        }

        Ok(Statement::Return(expr))
    }

    fn parse_if(&mut self) -> Result<Statement, BrowserError> {
        self.next(); // consume 'if'
        self.expect(Token::LParen)?;
        let cond = self.parse_expr()?;
        self.expect(Token::RParen)?;
        let then_branch = Box::new(self.parse_statement()?);
        
        let else_branch = if matches!(self.peek(), Token::Keyword(kw) if kw == "else") {
            self.next();
            Some(Box::new(self.parse_statement()?))
        } else {
            None
        };

        Ok(Statement::If(cond, then_branch, else_branch))
    }

    fn parse_while(&mut self) -> Result<Statement, BrowserError> {
        self.next(); // consume 'while'
        self.expect(Token::LParen)?;
        let cond = self.parse_expr()?;
        self.expect(Token::RParen)?;
        let body = Box::new(self.parse_statement()?);

        Ok(Statement::While(cond, body))
    }

    fn parse_block(&mut self) -> Result<Statement, BrowserError> {
        self.expect(Token::LBrace)?;
        let body = self.parse_block_body()?;
        Ok(Statement::Block(body))
    }

    fn parse_block_body(&mut self) -> Result<Vec<Statement>, BrowserError> {
        let mut stmts = Vec::new();
        
        while !matches!(self.peek(), Token::RBrace | Token::EOF) {
            stmts.push(self.parse_statement()?);
        }

        self.expect(Token::RBrace)?;
        Ok(stmts)
    }

    fn parse_expr(&mut self) -> Result<Expr, BrowserError> {
        self.parse_assignment()
    }

    fn parse_assignment(&mut self) -> Result<Expr, BrowserError> {
        let left = self.parse_equality()?;

        if matches!(self.peek(), Token::Operator(op) if op == "=") {
            self.next();
            let right = self.parse_assignment()?;
            return Ok(Expr::Assign(Box::new(left), Box::new(right)));
        }

        Ok(left)
    }

    fn parse_equality(&mut self) -> Result<Expr, BrowserError> {
        let mut left = self.parse_comparison()?;

        while let Token::Operator(op) = self.peek() {
            if op == "==" || op == "!=" {
                let op = op.clone();
                self.next();
                let right = self.parse_comparison()?;
                left = Expr::Binary(op, Box::new(left), Box::new(right));
            } else {
                break;
            }
        }

        Ok(left)
    }

    fn parse_comparison(&mut self) -> Result<Expr, BrowserError> {
        let mut left = self.parse_term()?;

        while let Token::Operator(op) = self.peek() {
            if op == "<" || op == ">" || op == "<=" || op == ">=" {
                let op = op.clone();
                self.next();
                let right = self.parse_term()?;
                left = Expr::Binary(op, Box::new(left), Box::new(right));
            } else {
                break;
            }
        }

        Ok(left)
    }

    fn parse_term(&mut self) -> Result<Expr, BrowserError> {
        let mut left = self.parse_factor()?;

        while let Token::Operator(op) = self.peek() {
            if op == "+" || op == "-" {
                let op = op.clone();
                self.next();
                let right = self.parse_factor()?;
                left = Expr::Binary(op, Box::new(left), Box::new(right));
            } else {
                break;
            }
        }

        Ok(left)
    }

    fn parse_factor(&mut self) -> Result<Expr, BrowserError> {
        let mut left = self.parse_unary()?;

        while let Token::Operator(op) = self.peek() {
            if op == "*" || op == "/" || op == "%" {
                let op = op.clone();
                self.next();
                let right = self.parse_unary()?;
                left = Expr::Binary(op, Box::new(left), Box::new(right));
            } else {
                break;
            }
        }

        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Expr, BrowserError> {
        if let Token::Operator(op) = self.peek() {
            if op == "-" || op == "!" {
                let op = op.clone();
                self.next();
                let operand = self.parse_unary()?;
                return Ok(Expr::Unary(op, Box::new(operand)));
            }
        }

        self.parse_call()
    }

    fn parse_call(&mut self) -> Result<Expr, BrowserError> {
        let mut expr = self.parse_primary()?;

        loop {
            match self.peek() {
                Token::LParen => {
                    self.next();
                    let args = self.parse_args()?;
                    self.expect(Token::RParen)?;
                    expr = Expr::Call(Box::new(expr), args);
                }
                Token::Dot => {
                    self.next();
                    match self.next() {
                        Token::Identifier(name) => {
                            expr = Expr::Member(Box::new(expr), name);
                        }
                        _ => return Err(BrowserError::JsError),
                    }
                }
                _ => break,
            }
        }

        Ok(expr)
    }

    fn parse_args(&mut self) -> Result<Vec<Expr>, BrowserError> {
        let mut args = Vec::new();

        while !matches!(self.peek(), Token::RParen) {
            args.push(self.parse_expr()?);
            if matches!(self.peek(), Token::Comma) {
                self.next();
            } else {
                break;
            }
        }

        Ok(args)
    }

    fn parse_primary(&mut self) -> Result<Expr, BrowserError> {
        match self.next() {
            Token::Number(n) => Ok(Expr::Number(n)),
            Token::String(s) => Ok(Expr::String(s)),
            Token::Keyword(kw) => {
                match kw.as_str() {
                    "true" => Ok(Expr::Boolean(true)),
                    "false" => Ok(Expr::Boolean(false)),
                    "null" => Ok(Expr::Null),
                    "undefined" => Ok(Expr::Undefined),
                    _ => Err(BrowserError::JsError),
                }
            }
            Token::Identifier(name) => Ok(Expr::Identifier(name)),
            Token::LParen => {
                let expr = self.parse_expr()?;
                self.expect(Token::RParen)?;
                Ok(expr)
            }
            Token::LBracket => {
                let mut elements = Vec::new();
                while !matches!(self.peek(), Token::RBracket) {
                    elements.push(self.parse_expr()?);
                    if matches!(self.peek(), Token::Comma) {
                        self.next();
                    } else {
                        break;
                    }
                }
                self.expect(Token::RBracket)?;
                Ok(Expr::Array(elements))
            }
            Token::LBrace => {
                let mut props = Vec::new();
                while !matches!(self.peek(), Token::RBrace) {
                    let key = match self.next() {
                        Token::Identifier(n) | Token::String(n) => n,
                        _ => return Err(BrowserError::JsError),
                    };
                    self.expect(Token::Colon)?;
                    let value = self.parse_expr()?;
                    props.push((key, value));
                    if matches!(self.peek(), Token::Comma) {
                        self.next();
                    } else {
                        break;
                    }
                }
                self.expect(Token::RBrace)?;
                Ok(Expr::Object(props))
            }
            _ => Err(BrowserError::JsError),
        }
    }
}

/// Execute JavaScript code
pub fn execute(code: &[u8]) -> Result<(), BrowserError> {
    // Tokenize
    let mut tokenizer = Tokenizer::new(code);
    let tokens = tokenizer.tokenize();

    // Parse
    let mut parser = Parser::new(tokens);
    let stmts = parser.parse()?;

    // Execute
    let mut env = Environment::new();
    
    // Set up console.log
    env.define("console", Value::Object(Object::new()));

    for stmt in stmts {
        evaluate_statement(&mut env, &stmt)?;
    }

    Ok(())
}

/// Evaluate statement
fn evaluate_statement(env: &mut Environment, stmt: &Statement) -> Result<Value, BrowserError> {
    match stmt {
        Statement::VarDecl(name, init) => {
            let value = if let Some(expr) = init {
                evaluate_expr(env, expr)?
            } else {
                Value::Undefined
            };
            env.define(name, value);
            Ok(Value::Undefined)
        }
        Statement::LetDecl(name, init) => {
            let value = if let Some(expr) = init {
                evaluate_expr(env, expr)?
            } else {
                Value::Undefined
            };
            env.define(name, value);
            Ok(Value::Undefined)
        }
        Statement::ConstDecl(name, init) => {
            let value = evaluate_expr(env, init)?;
            env.define(name, value);
            Ok(Value::Undefined)
        }
        Statement::Expr(expr) => {
            evaluate_expr(env, expr)
        }
        Statement::Return(expr) => {
            if let Some(expr) = expr {
                evaluate_expr(env, expr)
            } else {
                Ok(Value::Undefined)
            }
        }
        Statement::If(cond, then_branch, else_branch) => {
            let cond_value = evaluate_expr(env, cond)?;
            if cond_value.is_truthy() {
                evaluate_statement(env, then_branch)
            } else if let Some(else_stmt) = else_branch {
                evaluate_statement(env, else_stmt)
            } else {
                Ok(Value::Undefined)
            }
        }
        Statement::While(cond, body) => {
            loop {
                let cond_value = evaluate_expr(env, cond)?;
                if !cond_value.is_truthy() {
                    break;
                }
                evaluate_statement(env, body)?;
            }
            Ok(Value::Undefined)
        }
        Statement::Block(stmts) => {
            env.push_scope();
            let mut result = Value::Undefined;
            for stmt in stmts {
                result = evaluate_statement(env, stmt)?;
            }
            env.pop_scope();
            Ok(result)
        }
        Statement::FunctionDecl(name, params, body) => {
            let func = Value::Function(Function {
                name: name.clone(),
                params: params.clone(),
                body: body.clone(),
                native: None,
            });
            env.define(name, func);
            Ok(Value::Undefined)
        }
    }
}

/// Evaluate expression
fn evaluate_expr(env: &mut Environment, expr: &Expr) -> Result<Value, BrowserError> {
    match expr {
        Expr::Identifier(name) => Ok(env.get(name)),
        Expr::Number(n) => Ok(Value::Number(*n)),
        Expr::String(s) => Ok(Value::String(s.clone())),
        Expr::Boolean(b) => Ok(Value::Boolean(*b)),
        Expr::Null => Ok(Value::Null),
        Expr::Undefined => Ok(Value::Undefined),
        Expr::Binary(op, left, right) => {
            let left_val = evaluate_expr(env, left)?;
            let right_val = evaluate_expr(env, right)?;
            
            match op.as_str() {
                "+" => match (&left_val, &right_val) {
                    (Value::Number(a), Value::Number(b)) => Ok(Value::Number(a + b)),
                    _ => {
                        let mut result = left_val.to_string();
                        result.push_str(&right_val.to_string());
                        Ok(Value::String(result))
                    }
                }
                "-" => match (&left_val, &right_val) {
                    (Value::Number(a), Value::Number(b)) => Ok(Value::Number(a - b)),
                    _ => Ok(Value::Number(f64::NAN)),
                }
                "*" => match (&left_val, &right_val) {
                    (Value::Number(a), Value::Number(b)) => Ok(Value::Number(a * b)),
                    _ => Ok(Value::Number(f64::NAN)),
                }
                "/" => match (&left_val, &right_val) {
                    (Value::Number(a), Value::Number(b)) => {
                        if *b == 0.0 {
                            Ok(Value::Number(f64::INFINITY))
                        } else {
                            Ok(Value::Number(a / b))
                        }
                    }
                    _ => Ok(Value::Number(f64::NAN)),
                }
                "%" => match (&left_val, &right_val) {
                    (Value::Number(a), Value::Number(b)) => Ok(Value::Number(a % b)),
                    _ => Ok(Value::Number(f64::NAN)),
                }
                "==" => Ok(Value::Boolean(left_val.to_string() == right_val.to_string())),
                "!=" => Ok(Value::Boolean(left_val.to_string() != right_val.to_string())),
                "<" => match (&left_val, &right_val) {
                    (Value::Number(a), Value::Number(b)) => Ok(Value::Boolean(a < b)),
                    _ => Ok(Value::Boolean(left_val.to_string() < right_val.to_string())),
                }
                ">" => match (&left_val, &right_val) {
                    (Value::Number(a), Value::Number(b)) => Ok(Value::Boolean(a > b)),
                    _ => Ok(Value::Boolean(left_val.to_string() > right_val.to_string())),
                }
                "<=" => match (&left_val, &right_val) {
                    (Value::Number(a), Value::Number(b)) => Ok(Value::Boolean(a <= b)),
                    _ => Ok(Value::Boolean(left_val.to_string() <= right_val.to_string())),
                }
                ">=" => match (&left_val, &right_val) {
                    (Value::Number(a), Value::Number(b)) => Ok(Value::Boolean(a >= b)),
                    _ => Ok(Value::Boolean(left_val.to_string() >= right_val.to_string())),
                }
                "&&" => Ok(Value::Boolean(left_val.is_truthy() && right_val.is_truthy())),
                "||" => Ok(Value::Boolean(left_val.is_truthy() || right_val.is_truthy())),
                _ => Ok(Value::Undefined),
            }
        }
        Expr::Unary(op, operand) => {
            let val = evaluate_expr(env, operand)?;
            match op.as_str() {
                "-" => match val {
                    Value::Number(n) => Ok(Value::Number(-n)),
                    _ => Ok(Value::Number(f64::NAN)),
                }
                "!" => Ok(Value::Boolean(!val.is_truthy())),
                _ => Ok(Value::Undefined),
            }
        }
        Expr::Call(callee, args) => {
            let func_val = evaluate_expr(env, callee)?;
            
            let arg_values: Vec<Value> = args.iter()
                .map(|arg| evaluate_expr(env, arg).unwrap_or(Value::Undefined))
                .collect();

            match func_val {
                Value::Function(func) => {
                    if let Some(native) = func.native {
                        Ok(native(env, arg_values))
                    } else {
                        // User-defined function
                        env.push_scope();
                        
                        // Bind parameters
                        for (i, param) in func.params.iter().enumerate() {
                            let value = arg_values.get(i).cloned().unwrap_or(Value::Undefined);
                            env.define(param, value);
                        }

                        // Execute body
                        let mut result = Value::Undefined;
                        for stmt in &func.body {
                            result = evaluate_statement(env, stmt)?;
                        }

                        env.pop_scope();
                        Ok(result)
                    }
                }
                _ => Ok(Value::Undefined),
            }
        }
        Expr::Member(obj, prop) => {
            let obj_val = evaluate_expr(env, obj)?;
            match obj_val {
                Value::Object(o) => Ok(o.get(prop)),
                _ => Ok(Value::Undefined),
            }
        }
        Expr::Array(elements) => {
            let values: Vec<Value> = elements.iter()
                .map(|e| evaluate_expr(env, e).unwrap_or(Value::Undefined))
                .collect();
            Ok(Value::Array(values))
        }
        Expr::Object(props) => {
            let mut obj = Object::new();
            for (key, val_expr) in props {
                let val = evaluate_expr(env, val_expr)?;
                obj.set(key, val);
            }
            Ok(Value::Object(obj))
        }
        Expr::Assign(left, right) => {
            let value = evaluate_expr(env, right)?;
            if let Expr::Identifier(name) = left.as_ref() {
                env.set(name, value.clone());
            }
            Ok(value)
        }
    }
}

/// Initialize JavaScript engine
pub fn init() {
    println!("[js] JavaScript engine initialized");
}
