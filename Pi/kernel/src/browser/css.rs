//! CSS Parser and Engine
//!
//! Parses CSS stylesheets and applies styles to DOM elements.

use alloc::string::String;
use alloc::vec::Vec;
use alloc::boxed::Box;

use crate::browser::{BrowserError, html::{Document, Element, Node}};
use crate::println;

/// CSS Stylesheet
pub struct Stylesheet {
    /// Style rules
    pub rules: Vec<Rule>,
}

/// CSS Rule
pub struct Rule {
    /// Selectors
    pub selectors: Vec<Selector>,
    /// Declarations
    pub declarations: Vec<Declaration>,
}

/// CSS Selector
#[derive(Debug, Clone)]
pub enum Selector {
    /// Universal selector (*)
    Universal,
    /// Type selector (tag name)
    Type(String),
    /// Class selector (.class)
    Class(String),
    /// ID selector (#id)
    Id(String),
    /// Attribute selector ([attr=value])
    Attribute(String, String),
    /// Descendant selector (ancestor descendant)
    Descendant(Box<Selector>, Box<Selector>),
    /// Child selector (parent > child)
    Child(Box<Selector>, Box<Selector>),
}

/// CSS Declaration
#[derive(Debug, Clone)]
pub struct Declaration {
    /// Property name
    pub property: String,
    /// Property value
    pub value: Value,
}

/// CSS Value
#[derive(Debug, Clone)]
pub enum Value {
    /// Keyword value
    Keyword(String),
    /// Length value (e.g., 10px, 5em)
    Length(f32, Unit),
    /// Color value
    Color(Color),
    /// Percentage
    Percentage(f32),
    /// String value
    String(String),
    /// Number
    Number(f32),
}

/// CSS Unit
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unit {
    Px,
    Em,
    Rem,
    Percent,
    Pt,
    Cm,
    Mm,
    In,
}

/// CSS Color
#[derive(Debug, Clone, Copy)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    /// Create color from RGB
    pub fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    /// Create color from RGBA
    pub fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    /// Parse color from string
    pub fn parse(s: &str) -> Option<Self> {
        // Named colors
        match s.to_ascii_lowercase().as_str() {
            "black" => return Some(Self::rgb(0, 0, 0)),
            "white" => return Some(Self::rgb(255, 255, 255)),
            "red" => return Some(Self::rgb(255, 0, 0)),
            "green" => return Some(Self::rgb(0, 128, 0)),
            "blue" => return Some(Self::rgb(0, 0, 255)),
            "yellow" => return Some(Self::rgb(255, 255, 0)),
            "cyan" => return Some(Self::rgb(0, 255, 255)),
            "magenta" => return Some(Self::rgb(255, 0, 255)),
            "silver" => return Some(Self::rgb(192, 192, 192)),
            "gray" | "grey" => return Some(Self::rgb(128, 128, 128)),
            "maroon" => return Some(Self::rgb(128, 0, 0)),
            "olive" => return Some(Self::rgb(128, 128, 0)),
            "lime" => return Some(Self::rgb(0, 255, 0)),
            "aqua" => return Some(Self::rgb(0, 255, 255)),
            "teal" => return Some(Self::rgb(0, 128, 128)),
            "navy" => return Some(Self::rgb(0, 0, 128)),
            "fuchsia" => return Some(Self::rgb(255, 0, 255)),
            "purple" => return Some(Self::rgb(128, 0, 128)),
            "orange" => return Some(Self::rgb(255, 165, 0)),
            "transparent" => return Some(Self::rgba(0, 0, 0, 0)),
            _ => {}
        }

        // Hex colors
        if s.starts_with('#') {
            let hex = &s[1..];
            if hex.len() == 6 {
                if let (Ok(r), Ok(g), Ok(b)) = (
                    u8::from_str_radix(&hex[0..2], 16),
                    u8::from_str_radix(&hex[2..4], 16),
                    u8::from_str_radix(&hex[4..6], 16),
                ) {
                    return Some(Self::rgb(r, g, b));
                }
            } else if hex.len() == 3 {
                if let (Ok(r), Ok(g), Ok(b)) = (
                    u8::from_str_radix(&hex[0..1], 16),
                    u8::from_str_radix(&hex[1..2], 16),
                    u8::from_str_radix(&hex[2..3], 16),
                ) {
                    return Some(Self::rgb(r * 16 + r, g * 16 + g, b * 16 + b));
                }
            }
        }

        // rgb() / rgba()
        if s.starts_with("rgb(") || s.starts_with("rgba(") {
            // Parse rgb(r, g, b) format
            let inner = s.trim_start_matches("rgb(").trim_start_matches("rgba(")
                .trim_end_matches(')');
            let parts: Vec<&str> = inner.split(',').collect();
            if parts.len() >= 3 {
                if let (Ok(r), Ok(g), Ok(b)) = (
                    parts[0].trim().parse::<u8>(),
                    parts[1].trim().parse::<u8>(),
                    parts[2].trim().parse::<u8>(),
                ) {
                    let a = if parts.len() >= 4 {
                        (parts[3].trim().parse::<f32>().unwrap_or(1.0) * 255.0) as u8
                    } else {
                        255
                    };
                    return Some(Self::rgba(r, g, b, a));
                }
            }
        }

        None
    }
}

/// CSS Token
#[derive(Debug, Clone)]
enum Token {
    Ident(String),
    String(String),
    Number(f32),
    Hash(String),
    AtKeyword(String),
    Delim(char),
    LBrace,
    RBrace,
    LParen,
    RParen,
    LBracket,
    RBracket,
    Colon,
    Semicolon,
    Comma,
    Whitespace,
    EOF,
}

/// CSS Tokenizer
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

    fn consume_whitespace(&mut self) {
        while let Some(ch) = self.peek() {
            if !ch.is_ascii_whitespace() {
                break;
            }
            self.next();
        }
    }

    fn consume_ident(&mut self) -> String {
        let mut ident = String::new();
        while let Some(ch) = self.peek() {
            if ch.is_ascii_alphanumeric() || ch == b'-' || ch == b'_' {
                ident.push(ch as char);
                self.next();
            } else {
                break;
            }
        }
        ident
    }

    fn consume_number(&mut self) -> f32 {
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

    fn consume_string(&mut self, quote: u8) -> String {
        let mut s = String::new();
        self.next(); // consume opening quote

        while let Some(ch) = self.peek() {
            if ch == quote {
                self.next(); // consume closing quote
                break;
            }
            s.push(ch as char);
            self.next();
        }

        s
    }

    fn tokenize(&mut self) -> Vec<Token> {
        let mut tokens = Vec::new();

        while let Some(ch) = self.peek() {
            match ch {
                b' ' | b'\t' | b'\n' | b'\r' => {
                    self.consume_whitespace();
                    tokens.push(Token::Whitespace);
                }
                b'{' => {
                    self.next();
                    tokens.push(Token::LBrace);
                }
                b'}' => {
                    self.next();
                    tokens.push(Token::RBrace);
                }
                b'(' => {
                    self.next();
                    tokens.push(Token::LParen);
                }
                b')' => {
                    self.next();
                    tokens.push(Token::RParen);
                }
                b'[' => {
                    self.next();
                    tokens.push(Token::LBracket);
                }
                b']' => {
                    self.next();
                    tokens.push(Token::RBracket);
                }
                b':' => {
                    self.next();
                    tokens.push(Token::Colon);
                }
                b';' => {
                    self.next();
                    tokens.push(Token::Semicolon);
                }
                b',' => {
                    self.next();
                    tokens.push(Token::Comma);
                }
                b'#' => {
                    self.next();
                    let hash = self.consume_ident();
                    tokens.push(Token::Hash(hash));
                }
                b'@' => {
                    self.next();
                    let kw = self.consume_ident();
                    tokens.push(Token::AtKeyword(kw));
                }
                b'"' | b'\'' => {
                    let s = self.consume_string(ch);
                    tokens.push(Token::String(s));
                }
                _ if ch.is_ascii_digit() => {
                    let num = self.consume_number();
                    tokens.push(Token::Number(num));
                }
                _ if ch.is_ascii_alphabetic() || ch == b'-' || ch == b'_' => {
                    let ident = self.consume_ident();
                    tokens.push(Token::Ident(ident));
                }
                _ => {
                    self.next();
                    tokens.push(Token::Delim(ch as char));
                }
            }
        }

        tokens.push(Token::EOF);
        tokens
    }
}

/// Parse CSS stylesheet
pub fn parse(input: &str) -> Result<Stylesheet, BrowserError> {
    let mut tokenizer = Tokenizer::new(input.as_bytes());
    let tokens = tokenizer.tokenize();
    
    let mut rules = Vec::new();
    let mut pos = 0;

    while pos < tokens.len() {
        // Skip whitespace
        while pos < tokens.len() && matches!(tokens[pos], Token::Whitespace) {
            pos += 1;
        }

        if matches!(tokens[pos], Token::EOF) {
            break;
        }

        // Parse selector
        let selectors = parse_selectors(&tokens, &mut pos)?;

        // Skip whitespace
        while pos < tokens.len() && matches!(tokens[pos], Token::Whitespace) {
            pos += 1;
        }

        // Expect {
        if !matches!(tokens[pos], Token::LBrace) {
            return Err(BrowserError::ParseError);
        }
        pos += 1;

        // Parse declarations
        let declarations = parse_declarations(&tokens, &mut pos)?;

        rules.push(Rule {
            selectors,
            declarations,
        });
    }

    Ok(Stylesheet { rules })
}

/// Parse selectors
fn parse_selectors(tokens: &[Token], pos: &mut usize) -> Result<Vec<Selector>, BrowserError> {
    let mut selectors = Vec::new();

    while *pos < tokens.len() {
        // Skip whitespace
        while *pos < tokens.len() && matches!(tokens[*pos], Token::Whitespace) {
            *pos += 1;
        }

        let selector = match &tokens[*pos] {
            Token::Ident(tag) => {
                let tag = tag.clone();
                *pos += 1;
                Selector::Type(tag)
            }
            Token::Hash(id) => {
                let id = id.clone();
                *pos += 1;
                Selector::Id(id)
            }
            Token::Delim('.') => {
                *pos += 1;
                if let Token::Ident(class) = &tokens[*pos] {
                    let class = class.clone();
                    *pos += 1;
                    Selector::Class(class)
                } else {
                    return Err(BrowserError::ParseError);
                }
            }
            Token::Delim('*') => {
                *pos += 1;
                Selector::Universal
            }
            _ => break,
        };

        selectors.push(selector);

        // Skip whitespace
        while *pos < tokens.len() && matches!(tokens[*pos], Token::Whitespace) {
            *pos += 1;
        }

        // Check for comma (multiple selectors)
        if matches!(tokens[*pos], Token::Comma) {
            *pos += 1;
            continue;
        }

        // If next is {, we're done
        if matches!(tokens[*pos], Token::LBrace) {
            break;
        }
    }

    Ok(selectors)
}

/// Parse declarations
fn parse_declarations(tokens: &[Token], pos: &mut usize) -> Result<Vec<Declaration>, BrowserError> {
    let mut declarations = Vec::new();

    while *pos < tokens.len() {
        // Skip whitespace
        while *pos < tokens.len() && matches!(tokens[*pos], Token::Whitespace) {
            *pos += 1;
        }

        if matches!(tokens[*pos], Token::RBrace | Token::EOF) {
            break;
        }

        // Parse property
        let property = if let Token::Ident(prop) = &tokens[*pos] {
            prop.clone()
        } else {
            break;
        };
        *pos += 1;

        // Skip whitespace
        while *pos < tokens.len() && matches!(tokens[*pos], Token::Whitespace) {
            *pos += 1;
        }

        // Expect :
        if !matches!(tokens[*pos], Token::Colon) {
            return Err(BrowserError::ParseError);
        }
        *pos += 1;

        // Skip whitespace
        while *pos < tokens.len() && matches!(tokens[*pos], Token::Whitespace) {
            *pos += 1;
        }

        // Parse value
        let value = parse_value(tokens, pos)?;

        declarations.push(Declaration { property, value });

        // Skip whitespace
        while *pos < tokens.len() && matches!(tokens[*pos], Token::Whitespace) {
            *pos += 1;
        }

        // Optional semicolon
        if matches!(tokens[*pos], Token::Semicolon) {
            *pos += 1;
        }
    }

    // Consume }
    if matches!(tokens[*pos], Token::RBrace) {
        *pos += 1;
    }

    Ok(declarations)
}

/// Parse value
fn parse_value(tokens: &[Token], pos: &mut usize) -> Result<Value, BrowserError> {
    match &tokens[*pos] {
        Token::Ident(ident) => {
            let val = ident.clone();
            *pos += 1;
            Ok(Value::Keyword(val))
        }
        Token::Number(n) => {
            let num = *n;
            *pos += 1;
            
            // Check for unit
            if let Token::Ident(unit) = &tokens[*pos] {
                let unit = match unit.as_str() {
                    "px" => Unit::Px,
                    "em" => Unit::Em,
                    "rem" => Unit::Rem,
                    "%" => Unit::Percent,
                    "pt" => Unit::Pt,
                    "cm" => Unit::Cm,
                    "mm" => Unit::Mm,
                    "in" => Unit::In,
                    _ => return Ok(Value::Number(num)),
                };
                *pos += 1;
                Ok(Value::Length(num, unit))
            } else {
                Ok(Value::Number(num))
            }
        }
        Token::Hash(hex) => {
            let mut color_str = String::from("#");
            color_str.push_str(hex);
            if let Some(color) = Color::parse(&color_str) {
                *pos += 1;
                Ok(Value::Color(color))
            } else {
                Err(BrowserError::ParseError)
            }
        }
        _ => {
            *pos += 1;
            Ok(Value::Keyword(String::from("inherit")))
        }
    }
}

/// Apply styles to document
pub fn apply_styles(document: &mut Document) -> Result<(), BrowserError> {
    // Collect all stylesheets
    let mut stylesheet = Stylesheet { rules: Vec::new() };

    // Parse inline stylesheets
    for sheet_ref in &document.stylesheets {
        if let Ok(sheet) = parse(&sheet_ref.content) {
            stylesheet.rules.extend(sheet.rules);
        }
    }

    // Apply rules to elements
    apply_rules_to_element(&stylesheet, &mut document.root);

    Ok(())
}

/// Apply rules to element and children
fn apply_rules_to_element(sheet: &Stylesheet, element: &mut Element) {
    // Find matching rules
    for rule in &sheet.rules {
        for selector in &rule.selectors {
            if matches_selector(selector, element) {
                for decl in &rule.declarations {
                    let value_str = match &decl.value {
                        Value::Keyword(s) => s.clone(),
                        Value::Length(n, u) => {
                            let mut s = int_to_string(*n as i64);
                            match u {
                                Unit::Px => s.push_str("px"),
                                Unit::Em => s.push_str("em"),
                                Unit::Rem => s.push_str("rem"),
                                Unit::Percent => s.push_str("%"),
                                Unit::Pt => s.push_str("pt"),
                                Unit::Cm => s.push_str("cm"),
                                Unit::Mm => s.push_str("mm"),
                                Unit::In => s.push_str("in"),
                            }
                            s
                        }
                        Value::Color(_) => String::from("color"),
                        Value::Percentage(n) => {
                            let mut s = int_to_string(*n as i64);
                            s.push('%');
                            s
                        }
                        Value::String(s) => s.clone(),
                        Value::Number(n) => int_to_string(*n as i64),
                    };
                    element.computed_styles.push((
                        decl.property.clone(),
                        value_str,
                    ));
                }
            }
        }
    }

    // Apply to children
    for child in &mut element.children {
        if let Node::Element(ref mut elem) = child {
            apply_rules_to_element(sheet, elem);
        }
    }
}

/// Check if element matches selector
fn matches_selector(selector: &Selector, element: &Element) -> bool {
    match selector {
        Selector::Universal => true,
        Selector::Type(tag) => element.tag == *tag,
        Selector::Class(class) => {
            element.get_attr("class")
                .map(|c| c.split_whitespace().any(|p| p == class))
                .unwrap_or(false)
        }
        Selector::Id(id) => element.get_attr("id") == Some(id),
        _ => false, // Other selectors not implemented yet
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

/// Initialize CSS engine
pub fn init() {
    println!("[css] CSS engine initialized");
}
