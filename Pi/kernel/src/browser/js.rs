//! JavaScript Engine
//!
//! A simple JavaScript interpreter for WebbOS.

#![allow(dead_code)]

use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::format;

use crate::browser::BrowserError;
use crate::println;

/// Binding pattern for destructuring
#[derive(Debug, Clone, PartialEq)]
pub enum BindingPattern {
    Identifier(String),
    Array(Vec<BindingPattern>),
    Object(Vec<(String, BindingPattern)>),
}

impl BindingPattern {
    /// Convert binding pattern to string (for simple identifier case)
    pub fn to_string(&self) -> String {
        match self {
            BindingPattern::Identifier(s) => s.clone(),
            _ => String::new(),
        }
    }
}

/// Promise state
#[derive(Debug, Clone, PartialEq)]
pub enum PromiseState {
    Pending,
    Fulfilled(Box<Value>),
    Rejected(Box<Value>),
}

/// JavaScript Promise
#[derive(Debug, Clone, PartialEq)]
pub struct Promise {
    pub state: Box<PromiseState>,
    pub on_fulfilled: Vec<Function>,
    pub on_rejected: Vec<Function>,
}

impl Promise {
    pub fn new() -> Self {
        Self {
            state: Box::new(PromiseState::Pending),
            on_fulfilled: Vec::new(),
            on_rejected: Vec::new(),
        }
    }

    pub fn resolve(value: Value) -> Self {
        Self {
            state: Box::new(PromiseState::Fulfilled(Box::new(value))),
            on_fulfilled: Vec::new(),
            on_rejected: Vec::new(),
        }
    }

    pub fn reject(reason: Value) -> Self {
        Self {
            state: Box::new(PromiseState::Rejected(Box::new(reason))),
            on_fulfilled: Vec::new(),
            on_rejected: Vec::new(),
        }
    }
}

/// JavaScript value types
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Undefined,
    Null,
    Boolean(bool),
    Number(f64),
    String(String),
    Object(Object),
    Array(Vec<Value>),
    Function(Function),
    Promise(Box<Promise>),
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
            Value::Promise(_) => String::from("[object Promise]"),
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
            Value::Promise(_) => true,
        }
    }

    /// Get property (for objects)
    pub fn get_property(&self, key: &str) -> Value {
        match self {
            Value::Object(obj) => obj.get(key),
            Value::Array(arr) => {
                // Handle array length property
                if key == "length" {
                    Value::Number(arr.len() as f64)
                } else if key == "map" {
                    // Return native map function
                    Value::Function(Function::new_native("map", 1, array_map))
                } else if key == "filter" {
                    Value::Function(Function::new_native("filter", 1, array_filter))
                } else if key == "reduce" {
                    Value::Function(Function::new_native("reduce", 2, array_reduce))
                } else if key == "find" {
                    Value::Function(Function::new_native("find", 1, array_find))
                } else if key == "includes" {
                    Value::Function(Function::new_native("includes", 1, array_includes))
                } else if key == "forEach" {
                    Value::Function(Function::new_native("forEach", 1, array_for_each))
                } else if key == "push" {
                    Value::Function(Function::new_native("push", 1, array_push))
                } else if key == "pop" {
                    Value::Function(Function::new_native("pop", 0, array_pop))
                } else if key == "join" {
                    Value::Function(Function::new_native("join", 1, array_join))
                } else if key == "indexOf" {
                    Value::Function(Function::new_native("indexOf", 1, array_index_of))
                } else if key == "slice" {
                    Value::Function(Function::new_native("slice", 2, array_slice))
                } else if key == "splice" {
                    Value::Function(Function::new_native("splice", 2, array_splice))
                } else {
                    Value::Undefined
                }
            }
            _ => Value::Undefined,
        }
    }

    /// Set property (for objects)
    pub fn set_property(&mut self, key: &str, value: Value) {
        if let Value::Object(obj) = self {
            obj.set(key, value);
        }
    }
}

/// JavaScript object
#[derive(Debug, Clone, PartialEq)]
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

/// Native function callback type (using function pointer for Clone support)
pub type NativeFn = fn(&mut Environment, Vec<Value>) -> Value;

/// JavaScript function
#[derive(Clone)]
pub struct Function {
    pub name: String,
    pub params: Vec<BindingPattern>,
    pub body: Vec<Statement>,
    pub native: Option<NativeFn>,
    pub is_arrow: bool,
    pub arrow_expr: Option<Box<Expr>>, // For arrow functions with expression body
    pub this_binding: Option<Box<Value>>, // For arrow function `this` binding
}

impl core::fmt::Debug for Function {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Function")
            .field("name", &self.name)
            .field("params", &self.params)
            .field("body", &self.body)
            .field("native", &self.native.is_some())
            .field("is_arrow", &self.is_arrow)
            .finish()
    }
}

impl PartialEq for Function {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
            && self.params == other.params
            && self.body == other.body
            && self.is_arrow == other.is_arrow
        // Note: native functions are compared by name only
    }
}

impl Function {
    pub fn new(name: String, params: Vec<String>, body: Vec<Statement>) -> Self {
        Self {
            name,
            params: params.into_iter().map(BindingPattern::Identifier).collect(),
            body,
            native: None,
            is_arrow: false,
            arrow_expr: None,
            this_binding: None,
        }
    }

    pub fn new_arrow(name: String, params: Vec<String>, body: Vec<Statement>) -> Self {
        Self {
            name,
            params: params.into_iter().map(BindingPattern::Identifier).collect(),
            body,
            native: None,
            is_arrow: true,
            arrow_expr: None,
            this_binding: None,
        }
    }

    pub fn new_arrow_expr(name: String, params: Vec<String>, expr: Expr) -> Self {
        Self {
            name,
            params: params.into_iter().map(BindingPattern::Identifier).collect(),
            body: Vec::new(),
            native: None,
            is_arrow: true,
            arrow_expr: Some(Box::new(expr)),
            this_binding: None,
        }
    }

    pub fn new_native(name: &str, arity: usize, func: fn(&mut Environment, Vec<Value>) -> Value) -> Self {
        Self {
            name: String::from(name),
            params: (0..arity).map(|i| BindingPattern::Identifier(format!("arg{}", i))).collect(),
            body: Vec::new(),
            native: Some(func),
            is_arrow: false,
            arrow_expr: None,
            this_binding: None,
        }
    }
}

/// Environment for variable scoping
pub struct Environment {
    /// Variable scopes
    scopes: Vec<BTreeMap<String, Value>>,
    /// Global object
    global: Object,
    /// Output buffer for console.log
    output: String,
    /// Current `this` binding
    this_binding: Value,
}

impl Environment {
    pub fn new() -> Self {
        let mut env = Self {
            scopes: vec![BTreeMap::new()],
            global: Object::new(),
            output: String::new(),
            this_binding: Value::Undefined,
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

    /// Get current `this` binding
    pub fn get_this(&self) -> Value {
        self.this_binding.clone()
    }

    /// Set `this` binding
    pub fn set_this(&mut self, value: Value) {
        self.this_binding = value;
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
#[derive(Debug, Clone, PartialEq)]
enum Token {
    Identifier(String),
    Number(f64),
    String(String),
    Template(String), // Template literal part (backtick string without ${...})
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
    Arrow,      // =>
    Backtick,   // `
    Ellipsis,   // ...
    EOF,
}

/// JavaScript keywords
const KEYWORDS: &[&str] = &[
    "var", "let", "const", "function", "return", "if", "else", "while",
    "for", "break", "continue", "true", "false", "null", "undefined",
    "new", "this", "typeof", "instanceof", "in", "of", "class",
    "constructor", "extends", "super", "static", "get", "set",
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

    fn read_template(&mut self) -> (String, bool) {
        let mut s = String::new();
        let mut has_expr = false;

        while let Some(ch) = self.peek() {
            if ch == b'`' {
                self.next(); // consume closing backtick
                break;
            }
            if ch == b'$' {
                self.next();
                if let Some(b'{') = self.peek() {
                    // Found ${ - end of template part
                    self.next(); // consume {
                    has_expr = true;
                    break;
                } else {
                    // Just a $ character
                    s.push('$');
                }
            } else if ch == b'\\' {
                self.next();
                if let Some(escaped) = self.next() {
                    match escaped {
                        b'n' => s.push('\n'),
                        b't' => s.push('\t'),
                        b'r' => s.push('\r'),
                        b'\\' => s.push('\\'),
                        b'`' => s.push('`'),
                        b'$' => s.push('$'),
                        _ => s.push(escaped as char),
                    }
                }
            } else {
                s.push(ch as char);
                self.next();
            }
        }

        (s, has_expr)
    }

    fn tokenize(&mut self) -> Vec<Token> {
        let mut tokens = Vec::new();
        let mut in_template = false;

        loop {
            self.skip_whitespace();

            if in_template {
                // We're inside a template literal, read until ${ or `
                let (s, has_expr) = self.read_template();
                if !s.is_empty() {
                    tokens.push(Token::Template(s));
                }
                if has_expr {
                    // Continue to parse the expression
                    in_template = true;
                    tokens.push(Token::LBrace); // ${ was already consumed
                    continue;
                } else {
                    // Template ended
                    in_template = false;
                    tokens.push(Token::Backtick);
                    continue;
                }
            }

            match self.peek() {
                None => break,
                Some(b'(') => { tokens.push(Token::LParen); self.next(); }
                Some(b')') => { tokens.push(Token::RParen); self.next(); }
                Some(b'{') => { tokens.push(Token::LBrace); self.next(); }
                Some(b'}') => { 
                    tokens.push(Token::RBrace); 
                    self.next();
                    // Check if we're closing a template expression
                    if in_template {
                        // Continue reading the template
                        let (s, has_expr) = self.read_template();
                        if !s.is_empty() {
                            tokens.push(Token::Template(s));
                        }
                        if has_expr {
                            tokens.push(Token::LBrace);
                        } else {
                            in_template = false;
                            tokens.push(Token::Backtick);
                        }
                    }
                }
                Some(b'[') => { tokens.push(Token::LBracket); self.next(); }
                Some(b']') => { tokens.push(Token::RBracket); self.next(); }
                Some(b';') => { tokens.push(Token::Semicolon); self.next(); }
                Some(b',') => { tokens.push(Token::Comma); self.next(); }
                Some(b':') => { tokens.push(Token::Colon); self.next(); }
                Some(b'`') => {
                    self.next(); // consume opening backtick
                    tokens.push(Token::Backtick);
                    in_template = true;
                }
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
                    // Operators and special tokens
                    let mut op = String::new();
                    op.push(ch as char);
                    self.next();
                    
                    // Check for special tokens
                    if ch == b'=' {
                        if let Some(b'>') = self.peek() {
                            self.next();
                            tokens.push(Token::Arrow);
                            continue;
                        }
                    } else if ch == b'.' {
                        // Check for ellipsis (...)
                        if self.peek() == Some(b'.') {
                            self.next();
                            if self.peek() == Some(b'.') {
                                self.next();
                                tokens.push(Token::Ellipsis);
                                continue;
                            } else {
                                // Two dots - push first as dot, second will be handled next
                                tokens.push(Token::Dot);
                                tokens.push(Token::Dot);
                                continue;
                            }
                        }
                    }
                    
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
#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    VarDecl(String, Option<Expr>),
    LetDecl(String, Option<Expr>),
    ConstDecl(String, Expr),
    Expr(Expr),
    Return(Option<Expr>),
    If(Expr, Box<Statement>, Option<Box<Statement>>),
    While(Expr, Box<Statement>),
    For(Box<Statement>, Option<Expr>, Option<Expr>, Box<Statement>),
    Block(Vec<Statement>),
    FunctionDecl(String, Vec<String>, Vec<Statement>),
    ClassDecl(String, Option<String>, Vec<ClassMember>), // name, extends, members
}

/// Class member types
#[derive(Debug, Clone, PartialEq)]
pub enum ClassMember {
    Constructor(Vec<String>, Vec<Statement>),
    Method(String, Vec<String>, Vec<Statement>),
    StaticMethod(String, Vec<String>, Vec<Statement>),
    Property(String, Expr),
}

/// Expression types
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
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
    ArrowFunction(Vec<String>, Box<Expr>), // params, body (expression or block)
    ArrowFunctionBlock(Vec<String>, Vec<Statement>), // params, statements
    TemplateLiteral(Vec<TemplatePart>),
    Spread(Box<Expr>),
    New(Box<Expr>, Vec<Expr>),
}

/// Template literal part
#[derive(Debug, Clone, PartialEq)]
pub enum TemplatePart {
    String(String),
    Expression(Expr),
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
                    "for" => self.parse_for(),
                    "class" => self.parse_class_decl(),
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

    fn parse_class_decl(&mut self) -> Result<Statement, BrowserError> {
        self.next(); // consume 'class'
        
        let name = match self.next() {
            Token::Identifier(n) => n,
            _ => return Err(BrowserError::JsError),
        };

        // Check for extends
        let extends = if matches!(self.peek(), Token::Keyword(kw) if kw == "extends") {
            self.next(); // consume 'extends'
            match self.next() {
                Token::Identifier(n) => Some(n),
                _ => return Err(BrowserError::JsError),
            }
        } else {
            None
        };

        self.expect(Token::LBrace)?;
        
        let mut members = Vec::new();
        
        while !matches!(self.peek(), Token::RBrace | Token::EOF) {
            let is_static = if matches!(self.peek(), Token::Keyword(kw) if kw == "static") {
                self.next();
                true
            } else {
                false
            };

            let member_name = match self.next() {
                Token::Identifier(n) | Token::Keyword(n) => n,
                _ => return Err(BrowserError::JsError),
            };

            if member_name == "constructor" && !is_static {
                self.expect(Token::LParen)?;
                let params = self.parse_params()?;
                self.expect(Token::RParen)?;
                let body = self.parse_block_body()?;
                members.push(ClassMember::Constructor(params, body));
            } else {
                self.expect(Token::LParen)?;
                let params = self.parse_params()?;
                self.expect(Token::RParen)?;
                let body = self.parse_block_body()?;
                if is_static {
                    members.push(ClassMember::StaticMethod(member_name, params, body));
                } else {
                    members.push(ClassMember::Method(member_name, params, body));
                }
            }
        }

        self.expect(Token::RBrace)?;

        Ok(Statement::ClassDecl(name, extends, members))
    }

    fn parse_params(&mut self) -> Result<Vec<String>, BrowserError> {
        let mut params = Vec::new();
        
        while !matches!(self.peek(), Token::RParen) {
            // Handle rest parameter
            if matches!(self.peek(), Token::Ellipsis) {
                self.next(); // consume ...
                let name = match self.next() {
                    Token::Identifier(n) => n,
                    _ => return Err(BrowserError::JsError),
                };
                params.push(format!("...{}", name));
                break;
            }

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

    fn parse_for(&mut self) -> Result<Statement, BrowserError> {
        self.next(); // consume 'for'
        self.expect(Token::LParen)?;
        
        // Parse init (can be var/let/const or expression)
        let init = match self.peek() {
            Token::Keyword(kw) if kw == "var" || kw == "let" || kw == "const" => {
                match kw.as_str() {
                    "var" => self.parse_var_decl()?,
                    "let" => self.parse_let_decl()?,
                    "const" => self.parse_const_decl()?,
                    _ => return Err(BrowserError::JsError),
                }
            }
            _ => {
                let expr = self.parse_expr()?;
                Statement::Expr(expr)
            }
        };

        if matches!(self.peek(), Token::Keyword(kw) if kw == "in" || kw == "of") {
            // For-in or for-of loop (simplified)
            self.next(); // consume in/of
            let _iterable = self.parse_expr()?;
            self.expect(Token::RParen)?;
            let body = Box::new(self.parse_statement()?);
            // Return as regular while loop for now
            return Ok(Statement::For(Box::new(init), None, None, body));
        }

        self.expect(Token::Semicolon)?;
        
        let cond = if matches!(self.peek(), Token::Semicolon) {
            None
        } else {
            Some(self.parse_expr()?)
        };
        
        self.expect(Token::Semicolon)?;
        
        let update = if matches!(self.peek(), Token::RParen) {
            None
        } else {
            Some(self.parse_expr()?)
        };
        
        self.expect(Token::RParen)?;
        let body = Box::new(self.parse_statement()?);

        Ok(Statement::For(Box::new(init), cond, update, body))
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
        let left = self.parse_arrow_function()?;

        if matches!(self.peek(), Token::Operator(op) if op == "=") {
            self.next();
            let right = self.parse_assignment()?;
            return Ok(Expr::Assign(Box::new(left), Box::new(right)));
        }

        Ok(left)
    }

    fn parse_arrow_function(&mut self) -> Result<Expr, BrowserError> {
        // Check for arrow function: (params) => expr or (params) => { body }
        // Also: param => expr (single parameter without parens)
        
        let saved_pos = self.pos;
        
        // Try to parse params
        let params = if matches!(self.peek(), Token::LParen) {
            self.next(); // consume '('
            let params = self.parse_params()?;
            if !matches!(self.next(), Token::RParen) {
                self.pos = saved_pos;
                return self.parse_spread();
            }
            params
        } else if let Token::Identifier(name) = self.peek() {
            // Single parameter without parens
            let name = name.clone();
            self.next();
            vec![name]
        } else {
            return self.parse_spread();
        };

        // Check for arrow
        if matches!(self.peek(), Token::Arrow) {
            self.next(); // consume '=>'
            
            // Check for block body
            if matches!(self.peek(), Token::LBrace) {
                let body = self.parse_block_body()?;
                Ok(Expr::ArrowFunctionBlock(params, body))
            } else {
                // Expression body
                let expr = self.parse_expr()?;
                Ok(Expr::ArrowFunction(params, Box::new(expr)))
            }
        } else {
            // Not an arrow function, restore position
            self.pos = saved_pos;
            self.parse_spread()
        }
    }

    fn parse_spread(&mut self) -> Result<Expr, BrowserError> {
        if matches!(self.peek(), Token::Ellipsis) {
            self.next(); // consume ...
            let expr = self.parse_unary()?;
            return Ok(Expr::Spread(Box::new(expr)));
        }
        self.parse_or()
    }

    fn parse_or(&mut self) -> Result<Expr, BrowserError> {
        let mut left = self.parse_and()?;

        while let Token::Operator(op) = self.peek() {
            if op == "||" {
                let op = op.clone();
                self.next();
                let right = self.parse_and()?;
                left = Expr::Binary(op, Box::new(left), Box::new(right));
            } else {
                break;
            }
        }

        Ok(left)
    }

    fn parse_and(&mut self) -> Result<Expr, BrowserError> {
        let mut left = self.parse_equality()?;

        while let Token::Operator(op) = self.peek() {
            if op == "&&" {
                let op = op.clone();
                self.next();
                let right = self.parse_equality()?;
                left = Expr::Binary(op, Box::new(left), Box::new(right));
            } else {
                break;
            }
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
            if op == "-" || op == "!" || op == "typeof" {
                let op = op.clone();
                self.next();
                let operand = self.parse_unary()?;
                return Ok(Expr::Unary(op, Box::new(operand)));
            }
        }

        if matches!(self.peek(), Token::Keyword(kw) if kw == "new") {
            self.next(); // consume 'new'
            let ctor = self.parse_call()?;
            let args = if matches!(self.peek(), Token::LParen) {
                self.next();
                let args = self.parse_args()?;
                self.expect(Token::RParen)?;
                args
            } else {
                Vec::new()
            };
            return Ok(Expr::New(Box::new(ctor), args));
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
                Token::LBracket => {
                    // Dynamic property access: obj[prop]
                    self.next();
                    let index = self.parse_expr()?;
                    self.expect(Token::RBracket)?;
                    // Represent as member with string conversion
                    expr = Expr::Call(
                        Box::new(Expr::Member(Box::new(expr), String::from("__get"))),
                        vec![index]
                    );
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
                    "this" => Ok(Expr::Identifier(String::from("this"))),
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
                        Token::Keyword(n) => n,
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
            Token::Backtick => {
                self.parse_template_literal()
            }
            _ => Err(BrowserError::JsError),
        }
    }

    fn parse_template_literal(&mut self) -> Result<Expr, BrowserError> {
        let mut parts = Vec::new();
        
        // First Backtick token was already consumed by parse_primary
        // Now we should see either Template parts or expressions
        
        loop {
            match self.peek() {
                Token::Template(s) => {
                    parts.push(TemplatePart::String(s.clone()));
                    self.next();
                }
                Token::LBrace => {
                    // This is ${...} expression - the LBrace represents the {
                    self.next(); // consume {
                    let expr = self.parse_expr()?;
                    parts.push(TemplatePart::Expression(expr));
                    // Expect the closing }
                    if !matches!(self.peek(), Token::RBrace) {
                        return Err(BrowserError::JsError);
                    }
                    self.next(); // consume }
                }
                Token::RBrace => {
                    // Closing brace after template expression - just consume it
                    self.next();
                }
                Token::Backtick => {
                    // End of template
                    self.next();
                    break;
                }
                Token::EOF => break,
                _ => {
                    // Unexpected token, but try to continue
                    self.next();
                }
            }
        }
        
        Ok(Expr::TemplateLiteral(parts))
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
        Statement::For(init, cond, update, body) => {
            env.push_scope();
            
            // Execute init
            if let Statement::VarDecl(name, init_expr) = init.as_ref() {
                let value = if let Some(expr) = init_expr {
                    evaluate_expr(env, expr)?
                } else {
                    Value::Undefined
                };
                env.define(name, value);
            } else {
                evaluate_statement(env, init)?;
            }
            
            // Loop
            loop {
                // Check condition
                if let Some(cond_expr) = cond {
                    let cond_value = evaluate_expr(env, cond_expr)?;
                    if !cond_value.is_truthy() {
                        break;
                    }
                }
                
                // Execute body
                evaluate_statement(env, body)?;
                
                // Update
                if let Some(update_expr) = update {
                    evaluate_expr(env, update_expr)?;
                }
            }
            
            env.pop_scope();
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
            let func = Value::Function(Function::new(name.clone(), params.clone(), body.clone()));
            env.define(name, func);
            Ok(Value::Undefined)
        }
        Statement::ClassDecl(name, _extends, members) => {
            // Create constructor function
            let mut constructor_params = Vec::new();
            let mut constructor_body = Vec::new();
            
            for member in members {
                match member {
                    ClassMember::Constructor(params, body) => {
                        constructor_params = params.clone();
                        constructor_body = body.clone();
                    }
                    ClassMember::Method(_, _, _) => {}
                    ClassMember::StaticMethod(_, _, _) => {}
                    ClassMember::Property(_, _) => {}
                }
            }
            
            // Create the constructor function
            let _ = _extends;
            let constructor = Value::Function(Function::new(
                name.clone(),
                constructor_params,
                constructor_body,
            ));
            
            // Store class
            env.define(name, constructor);
            
            Ok(Value::Undefined)
        }
    }
}

/// Evaluate expression
fn evaluate_expr(env: &mut Environment, expr: &Expr) -> Result<Value, BrowserError> {
    match expr {
        Expr::Identifier(name) => {
            if name == "this" {
                Ok(env.get_this())
            } else {
                Ok(env.get(name))
            }
        }
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
                "typeof" => {
                    let type_str = match val {
                        Value::Undefined => "undefined",
                        Value::Null => "object",
                        Value::Boolean(_) => "boolean",
                        Value::Number(_) => "number",
                        Value::String(_) => "string",
                        Value::Object(_) => "object",
                        Value::Array(_) => "object",
                        Value::Function(_) => "function",
                        Value::Promise(_) => "object",
                    };
                    Ok(Value::String(String::from(type_str)))
                }
                _ => Ok(Value::Undefined),
            }
        }
        Expr::Call(callee, args) => {
            let func_val = evaluate_expr(env, callee)?;
            
            // Handle spread operator in arguments
            let mut arg_values: Vec<Value> = Vec::new();
            for arg in args {
                match arg {
                    Expr::Spread(expr) => {
                        let spread_val = evaluate_expr(env, expr)?;
                        if let Value::Array(arr) = spread_val {
                            arg_values.extend(arr);
                        }
                    }
                    _ => {
                        arg_values.push(evaluate_expr(env, arg)?);
                    }
                }
            }

            match func_val {
                Value::Function(func) => {
                    if let Some(native) = func.native {
                        Ok(native(env, arg_values))
                    } else if func.is_arrow {
                        // Arrow function - capture `this` at definition time
                        let saved_this = env.get_this();
                        
                        env.push_scope();
                        
                        // Bind parameters
                        for (i, param) in func.params.iter().enumerate() {
                            let param_name = param.to_string();
                            if param_name.starts_with("...") {
                                // Rest parameter
                                let rest_name = &param_name[3..];
                                let rest_values: Vec<Value> = arg_values[i..].to_vec();
                                env.define(rest_name, Value::Array(rest_values));
                                break;
                            } else {
                                let value = arg_values.get(i).cloned().unwrap_or(Value::Undefined);
                                env.define(&param_name, value);
                            }
                        }

                        // Execute body
                        let result = if let Some(arrow_expr) = &func.arrow_expr {
                            // Expression body
                            evaluate_expr(env, arrow_expr)?
                        } else {
                            // Block body
                            let mut result = Value::Undefined;
                            for stmt in &func.body {
                                result = evaluate_statement(env, stmt)?;
                            }
                            result
                        };

                        env.pop_scope();
                        
                        // Restore `this` binding
                        env.set_this(saved_this);
                        
                        Ok(result)
                    } else {
                        // Regular function
                        env.push_scope();
                        
                        // Bind parameters
                        for (i, param) in func.params.iter().enumerate() {
                            let param_name = param.to_string();
                            if param_name.starts_with("...") {
                                // Rest parameter
                                let rest_name = &param_name[3..];
                                let rest_values: Vec<Value> = arg_values[i..].to_vec();
                                env.define(rest_name, Value::Array(rest_values));
                                break;
                            } else {
                                let value = arg_values.get(i).cloned().unwrap_or(Value::Undefined);
                                env.define(&param_name, value);
                            }
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
                Value::Array(arr) => {
                    if prop == "length" {
                        Ok(Value::Number(arr.len() as f64))
                    } else {
                        Ok(Value::Undefined)
                    }
                }
                _ => Ok(Value::Undefined),
            }
        }
        Expr::Array(elements) => {
            let mut values: Vec<Value> = Vec::new();
            for e in elements {
                match e {
                    Expr::Spread(expr) => {
                        let spread_val = evaluate_expr(env, expr)?;
                        if let Value::Array(arr) = spread_val {
                            values.extend(arr);
                        }
                    }
                    _ => {
                        values.push(evaluate_expr(env, e).unwrap_or(Value::Undefined));
                    }
                }
            }
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
        Expr::ArrowFunction(params, body) => {
            // Expression arrow function: (a, b) => a + b
            let func = Function::new_arrow_expr(
                String::from(""),
                params.clone(),
                *body.clone(),
            );
            Ok(Value::Function(func))
        }
        Expr::ArrowFunctionBlock(params, body) => {
            // Block arrow function: (a, b) => { return a + b; }
            let func = Function::new_arrow(
                String::from(""),
                params.clone(),
                body.clone(),
            );
            Ok(Value::Function(func))
        }
        Expr::TemplateLiteral(parts) => {
            let mut result = String::new();
            for part in parts {
                match part {
                    TemplatePart::String(s) => result.push_str(s),
                    TemplatePart::Expression(expr) => {
                        let val = evaluate_expr(env, expr)?;
                        result.push_str(&val.to_string());
                    }
                }
            }
            Ok(Value::String(result))
        }
        Expr::Spread(expr) => {
            // Spread in expression context returns an array
            evaluate_expr(env, expr)
        }
        Expr::New(ctor, args) => {
            let ctor_val = evaluate_expr(env, ctor)?;
            let arg_values: Vec<Value> = args.iter()
                .map(|arg| evaluate_expr(env, arg).unwrap_or(Value::Undefined))
                .collect();
            
            match ctor_val {
                Value::Function(func) => {
                    // Create new object
                    let new_obj = Object::new();
                    
                    env.push_scope();
                    
                    // Bind `this`
                    env.set_this(Value::Object(new_obj.clone()));
                    
                    // Bind parameters and execute constructor
                    for (i, param) in func.params.iter().enumerate() {
                        let value = arg_values.get(i).cloned().unwrap_or(Value::Undefined);
                        let param_name = param.to_string();
                        env.define(&param_name, value);
                    }
                    
                    for stmt in &func.body {
                        evaluate_statement(env, stmt)?;
                    }
                    
                    env.pop_scope();
                    
                    Ok(Value::Object(new_obj))
                }
                _ => Ok(Value::Undefined),
            }
        }
    }
}

/// Array prototype methods

fn array_map(env: &mut Environment, args: Vec<Value>) -> Value {
    let this_arr = match env.get_this() {
        Value::Array(arr) => arr,
        _ => return Value::Undefined,
    };
    
    let callback = match args.get(0) {
        Some(Value::Function(f)) => f,
        _ => return Value::Array(this_arr.clone()),
    };
    
    let mut result = Vec::new();
    for (i, item) in this_arr.iter().enumerate() {
        // Call callback with item
        let call_args = vec![item.clone(), Value::Number(i as f64)];
        if let Some(native) = callback.native {
            result.push(native(env, call_args));
        } else {
            // For non-native functions, we'd need to evaluate
            // Simplified: just push the item
            result.push(item.clone());
        }
    }
    Value::Array(result)
}

fn array_filter(env: &mut Environment, args: Vec<Value>) -> Value {
    let this_arr = match env.get_this() {
        Value::Array(arr) => arr,
        _ => return Value::Undefined,
    };
    
    let callback = match args.get(0) {
        Some(Value::Function(f)) => f,
        _ => return Value::Array(this_arr.clone()),
    };
    
    let mut result = Vec::new();
    for (i, item) in this_arr.iter().enumerate() {
        let call_args = vec![item.clone(), Value::Number(i as f64)];
        let keep = if let Some(native) = callback.native {
            native(env, call_args).is_truthy()
        } else {
            true
        };
        if keep {
            result.push(item.clone());
        }
    }
    Value::Array(result)
}

fn array_reduce(env: &mut Environment, args: Vec<Value>) -> Value {
    let this_arr = match env.get_this() {
        Value::Array(arr) => arr,
        _ => return Value::Undefined,
    };
    
    let callback = match args.get(0) {
        Some(Value::Function(f)) => f,
        _ => return Value::Undefined,
    };
    
    let initial = args.get(1).cloned().unwrap_or(Value::Undefined);
    let mut accumulator = initial;
    
    for (i, item) in this_arr.iter().enumerate() {
        let call_args = vec![accumulator.clone(), item.clone(), Value::Number(i as f64)];
        accumulator = if let Some(native) = callback.native {
            native(env, call_args)
        } else {
            item.clone()
        };
    }
    accumulator
}

fn array_find(env: &mut Environment, args: Vec<Value>) -> Value {
    let this_arr = match env.get_this() {
        Value::Array(arr) => arr,
        _ => return Value::Undefined,
    };
    
    let callback = match args.get(0) {
        Some(Value::Function(f)) => f,
        _ => return Value::Undefined,
    };
    
    for (i, item) in this_arr.iter().enumerate() {
        let call_args = vec![item.clone(), Value::Number(i as f64)];
        let found = if let Some(native) = callback.native {
            native(env, call_args).is_truthy()
        } else {
            false
        };
        if found {
            return item.clone();
        }
    }
    Value::Undefined
}

fn array_includes(_env: &mut Environment, args: Vec<Value>) -> Value {
    let this_arr = match _env.get_this() {
        Value::Array(arr) => arr,
        _ => return Value::Boolean(false),
    };
    
    let search = args.get(0).cloned().unwrap_or(Value::Undefined);
    for item in this_arr.iter() {
        if item == &search {
            return Value::Boolean(true);
        }
    }
    Value::Boolean(false)
}

fn array_for_each(env: &mut Environment, args: Vec<Value>) -> Value {
    let _ = array_map(env, args); // Execute for side effects
    Value::Undefined
}

fn array_push(_env: &mut Environment, args: Vec<Value>) -> Value {
    // Note: This doesn't actually modify the array since we can't mutate through get_this
    // Would need to pass mutable reference
    let this_arr = match _env.get_this() {
        Value::Array(arr) => arr,
        _ => return Value::Number(0.0),
    };
    
    Value::Number((this_arr.len() + args.len()) as f64)
}

fn array_pop(_env: &mut Environment, _args: Vec<Value>) -> Value {
    let this_arr = match _env.get_this() {
        Value::Array(arr) => arr,
        _ => return Value::Undefined,
    };
    
    this_arr.last().cloned().unwrap_or(Value::Undefined)
}

fn array_join(_env: &mut Environment, args: Vec<Value>) -> Value {
    let this_arr = match _env.get_this() {
        Value::Array(arr) => arr,
        _ => return Value::String(String::new()),
    };
    
    let separator = match args.get(0) {
        Some(Value::String(s)) => s.clone(),
        _ => String::from(","),
    };
    
    let mut result = String::new();
    for (i, item) in this_arr.iter().enumerate() {
        if i > 0 {
            result.push_str(&separator);
        }
        result.push_str(&item.to_string());
    }
    Value::String(result)
}

fn array_index_of(_env: &mut Environment, args: Vec<Value>) -> Value {
    let this_arr = match _env.get_this() {
        Value::Array(arr) => arr,
        _ => return Value::Number(-1.0),
    };
    
    let search = args.get(0).cloned().unwrap_or(Value::Undefined);
    for (i, item) in this_arr.iter().enumerate() {
        if item == &search {
            return Value::Number(i as f64);
        }
    }
    Value::Number(-1.0)
}

fn array_slice(_env: &mut Environment, args: Vec<Value>) -> Value {
    let this_arr = match _env.get_this() {
        Value::Array(arr) => arr,
        _ => return Value::Array(Vec::new()),
    };
    
    let start = match args.get(0) {
        Some(Value::Number(n)) => *n as usize,
        _ => 0,
    };
    let end = match args.get(1) {
        Some(Value::Number(n)) => *n as usize,
        _ => this_arr.len(),
    };
    
    let start = start.min(this_arr.len());
    let end = end.min(this_arr.len());
    
    Value::Array(this_arr[start..end].to_vec())
}

fn array_splice(_env: &mut Environment, args: Vec<Value>) -> Value {
    let this_arr = match _env.get_this() {
        Value::Array(arr) => arr,
        _ => return Value::Array(Vec::new()),
    };
    
    let start = match args.get(0) {
        Some(Value::Number(n)) => *n as usize,
        _ => 0,
    };
    let delete_count = match args.get(1) {
        Some(Value::Number(n)) => *n as usize,
        _ => 0,
    };
    
    let start = start.min(this_arr.len());
    let end = (start + delete_count).min(this_arr.len());
    
    Value::Array(this_arr[start..end].to_vec())
}

/// Initialize JavaScript engine
pub fn init() {
    println!("[js] JavaScript engine initialized");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arrow_function_expression() {
        let code = b"const add = (a, b) => a + b; add(1, 2);";
        assert!(execute(code).is_ok());
    }

    #[test]
    fn test_arrow_function_block() {
        let code = b"const add = (a, b) => { return a + b; }; add(1, 2);";
        assert!(execute(code).is_ok());
    }

    #[test]
    fn test_class_declaration() {
        let code = b"class Person { constructor(name) { this.name = name; } greet() { return \"Hello \" + this.name; } } new Person(\"World\");";
        assert!(execute(code).is_ok());
    }

    #[test]
    fn test_template_literal() {
        let code = b"const name = \"World\"; const msg = `Hello ${name}!`;";
        assert!(execute(code).is_ok());
    }

    #[test]
    fn test_spread_array() {
        let code = b"const arr1 = [1, 2]; const arr2 = [...arr1, 3];";
        assert!(execute(code).is_ok());
    }
}
