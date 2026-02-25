//! CSS Parser and Engine
//!
//! Parses CSS stylesheets and applies styles to DOM elements.

#![allow(dead_code)]

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::boxed::Box;
use alloc::collections::BTreeMap;

use crate::browser::{BrowserError, html::{Document, Element, Node}};
use crate::println;

/// CSS Stylesheet
pub struct Stylesheet {
    /// Style rules
    pub rules: Vec<Rule>,
    /// Keyframes rules (@keyframes)
    pub keyframes: Vec<KeyframesRule>,
}

impl Stylesheet {
    pub fn new() -> Self {
        Self {
            rules: Vec::new(),
            keyframes: Vec::new(),
        }
    }

    /// Find a keyframes rule by name
    pub fn find_keyframes(&self, name: &str) -> Option<&KeyframesRule> {
        self.keyframes.iter().find(|k| k.name == name)
    }
}

/// CSS Rule
pub struct Rule {
    /// Selectors
    pub selectors: Vec<Selector>,
    /// Declarations
    pub declarations: Vec<Declaration>,
}

/// CSS Keyframes Rule (@keyframes)
#[derive(Debug, Clone)]
pub struct KeyframesRule {
    /// Animation name
    pub name: String,
    /// Keyframes (percentage -> declarations)
    pub keyframes: BTreeMap<u8, Vec<Declaration>>,
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
    /// Linear gradient
    LinearGradient(LinearGradient),
    /// Box shadow
    BoxShadow(BoxShadow),
    /// CSS Transform
    Transform(Transform),
    /// Border radius
    BorderRadius(BorderRadius),
    /// Animation value
    Animation(Animation),
    /// Transition value
    Transition(Transition),
}

/// CSS Animation
#[derive(Debug, Clone)]
pub struct Animation {
    /// Animation name (references @keyframes)
    pub name: String,
    /// Duration in seconds
    pub duration: f32,
    /// Timing function
    pub timing_function: TimingFunction,
    /// Delay in seconds
    pub delay: f32,
    /// Iteration count (0.0 = infinite)
    pub iteration_count: f32,
    /// Direction
    pub direction: AnimationDirection,
}

impl Animation {
    /// Parse animation shorthand: name duration timing-function delay iteration-count
    pub fn parse(value: &str) -> Option<Self> {
        let parts: Vec<&str> = value.split_whitespace().collect();
        if parts.is_empty() {
            return None;
        }

        let mut name = String::new();
        let mut duration = 1.0f32;
        let mut timing_function = TimingFunction::Ease;
        let mut delay = 0.0f32;
        let mut iteration_count = 1.0f32;
        let mut direction = AnimationDirection::Normal;

        for part in parts {
            // Check for timing function
            if let Some(tf) = TimingFunction::parse(part) {
                timing_function = tf;
                continue;
            }

            // Check for direction
            if let Some(dir) = AnimationDirection::parse(part) {
                direction = dir;
                continue;
            }

            // Check for iteration count (infinite or number)
            if part == "infinite" {
                iteration_count = 0.0; // 0.0 represents infinite
                continue;
            }

            // Check for time values (ends with 's' or 'ms')
            if part.ends_with("ms") {
                if let Ok(ms) = part[..part.len()-2].parse::<f32>() {
                    if duration == 1.0 && name.is_empty() {
                        // First time value is duration
                    } else {
                        delay = ms / 1000.0;
                    }
                }
                continue;
            }
            if part.ends_with('s') {
                if let Ok(s) = part[..part.len()-1].parse::<f32>() {
                    if duration == 1.0 && name.is_empty() {
                        duration = s;
                    } else {
                        delay = s;
                    }
                }
                continue;
            }

            // Check for number (iteration count)
            if let Ok(n) = part.parse::<f32>() {
                iteration_count = n;
                continue;
            }

            // Otherwise, it's the animation name
            if name.is_empty() {
                name = part.to_string();
            }
        }

        if name.is_empty() {
            return None;
        }

        Some(Self {
            name,
            duration,
            timing_function,
            delay,
            iteration_count,
            direction,
        })
    }
}

/// Animation direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnimationDirection {
    Normal,
    Reverse,
    Alternate,
    AlternateReverse,
}

impl AnimationDirection {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "normal" => Some(Self::Normal),
            "reverse" => Some(Self::Reverse),
            "alternate" => Some(Self::Alternate),
            "alternate-reverse" => Some(Self::AlternateReverse),
            _ => None,
        }
    }
}

/// CSS Transition
#[derive(Debug, Clone)]
pub struct Transition {
    /// Property name to transition
    pub property: String,
    /// Duration in seconds
    pub duration: f32,
    /// Timing function
    pub timing_function: TimingFunction,
    /// Delay in seconds
    pub delay: f32,
}

impl Transition {
    /// Parse transition shorthand: property duration timing-function delay
    pub fn parse(value: &str) -> Option<Self> {
        let parts: Vec<&str> = value.split_whitespace().collect();
        if parts.is_empty() {
            return None;
        }

        let mut property = String::from("all");
        let mut duration = 0.0f32;
        let mut timing_function = TimingFunction::Ease;
        let mut delay = 0.0f32;

        // First part is usually the property
        if !parts[0].ends_with("ms") && !parts[0].ends_with('s') && parts[0].parse::<f32>().is_err() {
            property = parts[0].to_string();
        }

        for part in parts {
            // Check for timing function
            if let Some(tf) = TimingFunction::parse(part) {
                timing_function = tf;
                continue;
            }

            // Check for time values
            if part.ends_with("ms") {
                if let Ok(ms) = part[..part.len()-2].parse::<f32>() {
                    let secs = ms / 1000.0;
                    if duration == 0.0 {
                        duration = secs;
                    } else {
                        delay = secs;
                    }
                }
                continue;
            }
            if part.ends_with('s') {
                if let Ok(s) = part[..part.len()-1].parse::<f32>() {
                    if duration == 0.0 {
                        duration = s;
                    } else {
                        delay = s;
                    }
                }
                continue;
            }
        }

        Some(Self {
            property,
            duration,
            timing_function,
            delay,
        })
    }
}

/// Timing function for animations and transitions
#[derive(Debug, Clone, Copy)]
pub enum TimingFunction {
    Linear,
    Ease,
    EaseIn,
    EaseOut,
    EaseInOut,
    /// Cubic bezier with control points (x1, y1, x2, y2)
    CubicBezier(f32, f32, f32, f32),
}

impl TimingFunction {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "linear" => Some(Self::Linear),
            "ease" => Some(Self::Ease),
            "ease-in" => Some(Self::EaseIn),
            "ease-out" => Some(Self::EaseOut),
            "ease-in-out" => Some(Self::EaseInOut),
            _ => {
                // Try to parse cubic-bezier(x1, y1, x2, y2)
                if s.starts_with("cubic-bezier(") && s.ends_with(')') {
                    let inner = &s[13..s.len()-1];
                    let values: Vec<&str> = inner.split(',').collect();
                    if values.len() == 4 {
                        if let (Ok(x1), Ok(y1), Ok(x2), Ok(y2)) = (
                            values[0].trim().parse::<f32>(),
                            values[1].trim().parse::<f32>(),
                            values[2].trim().parse::<f32>(),
                            values[3].trim().parse::<f32>(),
                        ) {
                            return Some(Self::CubicBezier(x1, y1, x2, y2));
                        }
                    }
                }
                None
            }
        }
    }

    /// Apply timing function to a progress value (0.0 to 1.0)
    pub fn apply(&self, t: f32) -> f32 {
        match self {
            Self::Linear => t,
            Self::Ease => ease(t),
            Self::EaseIn => ease_in(t),
            Self::EaseOut => ease_out(t),
            Self::EaseInOut => ease_in_out(t),
            Self::CubicBezier(x1, y1, x2, y2) => cubic_bezier(t, *x1, *y1, *x2, *y2),
        }
    }
}

/// Ease timing function approximation
fn ease(t: f32) -> f32 {
    cubic_bezier(t, 0.25, 0.1, 0.25, 1.0)
}

/// Ease-in timing function
fn ease_in(t: f32) -> f32 {
    t * t
}

/// Ease-out timing function
fn ease_out(t: f32) -> f32 {
    1.0 - (1.0 - t) * (1.0 - t)
}

/// Ease-in-out timing function
fn ease_in_out(t: f32) -> f32 {
    if t < 0.5 {
        2.0 * t * t
    } else {
        let v = -2.0 * t + 2.0;
        1.0 - (v * v) / 2.0
    }
}

/// Cubic bezier interpolation
fn cubic_bezier(t: f32, x1: f32, y1: f32, x2: f32, y2: f32) -> f32 {
    // Simplified approximation using De Casteljau's algorithm
    let cx = 3.0 * x1;
    let bx = 3.0 * (x2 - x1) - cx;
    let ax = 1.0 - cx - bx;

    let cy = 3.0 * y1;
    let by = 3.0 * (y2 - y1) - cy;
    let ay = 1.0 - cy - by;

    // Solve for x given t, then find y
    let _x = ((ax * t + bx) * t + cx) * t;
    let y = ((ay * t + by) * t + cy) * t;

    // For simplicity, use y directly (approximation)
    y
}

/// Linear gradient
#[derive(Debug, Clone)]
pub struct LinearGradient {
    pub angle: f32, // degrees
    pub stops: Vec<GradientStop>,
}

/// Gradient color stop
#[derive(Debug, Clone)]
pub struct GradientStop {
    pub color: Color,
    pub position: f32, // 0.0 to 1.0
}

/// Box shadow
#[derive(Debug, Clone)]
pub struct BoxShadow {
    pub offset_x: f32,
    pub offset_y: f32,
    pub blur_radius: f32,
    pub spread_radius: f32,
    pub color: Color,
    pub inset: bool,
}

/// CSS Transform
#[derive(Debug, Clone)]
pub enum Transform {
    Translate(f32, f32),
    Rotate(f32), // degrees
    Scale(f32, f32),
    Skew(f32, f32),
    None,
}

/// Border radius
#[derive(Debug, Clone)]
pub struct BorderRadius {
    pub top_left: f32,
    pub top_right: f32,
    pub bottom_right: f32,
    pub bottom_left: f32,
}

impl BorderRadius {
    pub fn uniform(radius: f32) -> Self {
        Self {
            top_left: radius,
            top_right: radius,
            bottom_right: radius,
            bottom_left: radius,
        }
    }
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

    /// Interpolate between two colors
    pub fn interpolate(&self, other: &Color, t: f32) -> Color {
        let t = t.clamp(0.0, 1.0);
        Color {
            r: (self.r as f32 * (1.0 - t) + other.r as f32 * t) as u8,
            g: (self.g as f32 * (1.0 - t) + other.g as f32 * t) as u8,
            b: (self.b as f32 * (1.0 - t) + other.b as f32 * t) as u8,
            a: (self.a as f32 * (1.0 - t) + other.a as f32 * t) as u8,
        }
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
    Percent,
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
                b'%' => {
                    self.next();
                    tokens.push(Token::Percent);
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
    
    let mut stylesheet = Stylesheet::new();
    let mut pos = 0;

    while pos < tokens.len() {
        // Skip whitespace
        while pos < tokens.len() && matches!(tokens[pos], Token::Whitespace) {
            pos += 1;
        }

        if matches!(tokens[pos], Token::EOF) {
            break;
        }

        // Check for @-rules
        if let Token::AtKeyword(ref keyword) = tokens[pos] {
            let keyword = keyword.clone();
            pos += 1;

            match keyword.as_str() {
                "keyframes" => {
                    if let Some(keyframes_rule) = parse_keyframes_rule(&tokens, &mut pos)? {
                        stylesheet.keyframes.push(keyframes_rule);
                    }
                }
                _ => {
                    // Skip unknown @-rules
                    skip_unknown_at_rule(&tokens, &mut pos)?;
                }
            }
            continue;
        }

        // Parse regular selector rule
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

        stylesheet.rules.push(Rule {
            selectors,
            declarations,
        });
    }

    Ok(stylesheet)
}

/// Parse @keyframes rule
fn parse_keyframes_rule(tokens: &[Token], pos: &mut usize) -> Result<Option<KeyframesRule>, BrowserError> {
    // Skip whitespace
    while *pos < tokens.len() && matches!(tokens[*pos], Token::Whitespace) {
        *pos += 1;
    }

    // Get animation name
    let name = match &tokens[*pos] {
        Token::Ident(name) => {
            let n = name.clone();
            *pos += 1;
            n
        }
        _ => return Ok(None),
    };

    // Skip whitespace
    while *pos < tokens.len() && matches!(tokens[*pos], Token::Whitespace) {
        *pos += 1;
    }

    // Expect {
    if !matches!(tokens[*pos], Token::LBrace) {
        return Err(BrowserError::ParseError);
    }
    *pos += 1;

    let mut keyframes: BTreeMap<u8, Vec<Declaration>> = BTreeMap::new();

    // Parse keyframe blocks
    while *pos < tokens.len() {
        // Skip whitespace
        while *pos < tokens.len() && matches!(tokens[*pos], Token::Whitespace) {
            *pos += 1;
        }

        if matches!(tokens[*pos], Token::RBrace | Token::EOF) {
            break;
        }

        // Parse keyframe selector (percentage or keywords like from/to)
        let percentages = parse_keyframe_selectors(tokens, pos)?;

        // Skip whitespace
        while *pos < tokens.len() && matches!(tokens[*pos], Token::Whitespace) {
            *pos += 1;
        }

        // Expect {
        if !matches!(tokens[*pos], Token::LBrace) {
            return Err(BrowserError::ParseError);
        }
        *pos += 1;

        // Parse declarations
        let declarations = parse_declarations(tokens, pos)?;

        // Add declarations to each specified percentage
        for pct in percentages {
            keyframes.insert(pct, declarations.clone());
        }
    }

    // Consume }
    if matches!(tokens[*pos], Token::RBrace) {
        *pos += 1;
    }

    Ok(Some(KeyframesRule { name, keyframes }))
}

/// Parse keyframe selectors (e.g., "0%", "100%", "from", "to", "0%, 100%")
fn parse_keyframe_selectors(tokens: &[Token], pos: &mut usize) -> Result<Vec<u8>, BrowserError> {
    let mut percentages = Vec::new();

    while *pos < tokens.len() {
        // Skip whitespace
        while *pos < tokens.len() && matches!(tokens[*pos], Token::Whitespace) {
            *pos += 1;
        }

        let pct = match &tokens[*pos] {
            Token::Ident(keyword) => {
                match keyword.as_str() {
                    "from" => 0u8,
                    "to" => 100u8,
                    _ => return Err(BrowserError::ParseError),
                }
            }
            Token::Number(n) => {
                let num = *n;
                *pos += 1;
                // Check for %
                if matches!(tokens[*pos], Token::Percent) {
                    *pos += 1;
                }
                num as u8
            }
            _ => return Err(BrowserError::ParseError),
        };

        *pos += 1;
        percentages.push(pct);

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

    Ok(percentages)
}

/// Skip unknown @-rule
fn skip_unknown_at_rule(tokens: &[Token], pos: &mut usize) -> Result<(), BrowserError> {
    // Skip until we find a matching }
    let mut depth = 1;
    while *pos < tokens.len() {
        match &tokens[*pos] {
            Token::LBrace => depth += 1,
            Token::RBrace => {
                depth -= 1;
                if depth == 0 {
                    *pos += 1;
                    break;
                }
            }
            Token::EOF => break,
            _ => {}
        }
        *pos += 1;
    }
    Ok(())
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
    // Collect all tokens until semicolon or closing brace
    let mut value_parts = Vec::new();
    let start_pos = *pos;

    while *pos < tokens.len() {
        match &tokens[*pos] {
            Token::Semicolon | Token::RBrace => break,
            Token::Whitespace => {
                *pos += 1;
                continue;
            }
            _ => {
                value_parts.push(tokens[*pos].clone());
                *pos += 1;
            }
        }
    }

    // Try to parse as a combined value
    if let Some(value) = try_parse_complex_value(&value_parts, start_pos) {
        return Ok(value);
    }

    // Fall back to simple value parsing
    match &tokens[start_pos] {
        Token::Ident(ident) => {
            Ok(Value::Keyword(ident.clone()))
        }
        Token::Number(n) => {
            let num = *n;
            // Check for unit after number
            if *pos < tokens.len() {
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
                    return Ok(Value::Length(num, unit));
                }
            }
            Ok(Value::Number(num))
        }
        Token::Hash(hex) => {
            let mut color_str = String::from("#");
            color_str.push_str(hex);
            if let Some(color) = Color::parse(&color_str) {
                Ok(Value::Color(color))
            } else {
                Err(BrowserError::ParseError)
            }
        }
        _ => {
            Ok(Value::Keyword(String::from("inherit")))
        }
    }
}

/// Try to parse complex values (animation, transition, transform, etc.)
fn try_parse_complex_value(tokens: &[Token], _start_pos: usize) -> Option<Value> {
    if tokens.is_empty() {
        return None;
    }

    // Convert tokens to string for easier parsing
    let mut value_str = String::new();
    for token in tokens {
        match token {
            Token::Ident(s) => {
                if !value_str.is_empty() {
                    value_str.push(' ');
                }
                value_str.push_str(s);
            }
            Token::Number(n) => {
                if !value_str.is_empty() {
                    value_str.push(' ');
                }
                value_str.push_str(&alloc::format!("{}", n));
            }
            Token::Hash(h) => {
                if !value_str.is_empty() {
                    value_str.push(' ');
                }
                value_str.push('#');
                value_str.push_str(h);
            }
            Token::Delim(d) => {
                value_str.push(*d);
            }
            Token::LParen => value_str.push('('),
            Token::RParen => value_str.push(')'),
            Token::Comma => value_str.push(','),
            Token::Percent => value_str.push('%'),
            _ => {}
        }
    }

    // Try to parse as animation
    if let Some(anim) = Animation::parse(&value_str) {
        return Some(Value::Animation(anim));
    }

    // Try to parse as transition
    if let Some(trans) = Transition::parse(&value_str) {
        return Some(Value::Transition(trans));
    }

    // Try to parse as transform
    if value_str.starts_with("translate") || value_str.starts_with("rotate") || 
       value_str.starts_with("scale") || value_str.starts_with("skew") {
        if let Some(transform) = parse_transform(&value_str) {
            return Some(Value::Transform(transform));
        }
    }

    // Try to parse as color
    if let Some(color) = Color::parse(&value_str) {
        return Some(Value::Color(color));
    }

    None
}

/// Parse transform value
fn parse_transform(s: &str) -> Option<Transform> {
    let s = s.trim();
    
    if s.starts_with("translate(") && s.ends_with(')') {
        let inner = &s[10..s.len()-1];
        let parts: Vec<&str> = inner.split(',').collect();
        if parts.len() == 2 {
            let x = parse_length_value(parts[0].trim())?;
            let y = parse_length_value(parts[1].trim())?;
            return Some(Transform::Translate(x, y));
        }
    }
    
    if s.starts_with("translateX(") && s.ends_with(')') {
        let inner = &s[11..s.len()-1];
        let x = parse_length_value(inner.trim())?;
        return Some(Transform::Translate(x, 0.0));
    }
    
    if s.starts_with("translateY(") && s.ends_with(')') {
        let inner = &s[11..s.len()-1];
        let y = parse_length_value(inner.trim())?;
        return Some(Transform::Translate(0.0, y));
    }
    
    if s.starts_with("rotate(") && s.ends_with(')') {
        let inner = &s[7..s.len()-1];
        let deg = parse_angle_value(inner.trim())?;
        return Some(Transform::Rotate(deg));
    }
    
    if s.starts_with("scale(") && s.ends_with(')') {
        let inner = &s[6..s.len()-1];
        let parts: Vec<&str> = inner.split(',').collect();
        if parts.len() == 1 {
            let s = parts[0].trim().parse::<f32>().ok()?;
            return Some(Transform::Scale(s, s));
        } else if parts.len() == 2 {
            let sx = parts[0].trim().parse::<f32>().ok()?;
            let sy = parts[1].trim().parse::<f32>().ok()?;
            return Some(Transform::Scale(sx, sy));
        }
    }
    
    if s.starts_with("skew(") && s.ends_with(')') {
        let inner = &s[5..s.len()-1];
        let parts: Vec<&str> = inner.split(',').collect();
        if parts.len() == 2 {
            let x = parse_angle_value(parts[0].trim())?;
            let y = parse_angle_value(parts[1].trim())?;
            return Some(Transform::Skew(x, y));
        }
    }
    
    None
}

/// Parse length value (e.g., "10px", "5em", "50%")
fn parse_length_value(s: &str) -> Option<f32> {
    let s = s.trim();
    if s.ends_with("px") {
        s[..s.len()-2].parse().ok()
    } else if s.ends_with("%") {
        s[..s.len()-1].parse::<f32>().map(|v| v / 100.0).ok()
    } else if s.ends_with("em") {
        s[..s.len()-2].parse::<f32>().map(|v| v * 16.0).ok()
    } else if s.ends_with("rem") {
        s[..s.len()-3].parse::<f32>().map(|v| v * 16.0).ok()
    } else {
        s.parse().ok()
    }
}

/// Parse angle value (e.g., "45deg", "1rad")
fn parse_angle_value(s: &str) -> Option<f32> {
    let s = s.trim();
    if s.ends_with("deg") {
        s[..s.len()-3].parse().ok()
    } else if s.ends_with("rad") {
        s[..s.len()-3].parse::<f32>().map(|v| v * 180.0 / 3.14159).ok()
    } else {
        s.parse().ok()
    }
}

/// Apply styles to document
pub fn apply_styles(document: &mut Document) -> Result<(), BrowserError> {
    // Collect all stylesheets
    let mut stylesheet = Stylesheet::new();

    // Parse inline stylesheets
    for sheet_ref in &document.stylesheets {
        if let Ok(sheet) = parse(&sheet_ref.content) {
            stylesheet.rules.extend(sheet.rules);
            stylesheet.keyframes.extend(sheet.keyframes);
        }
    }

    // Apply rules to elements
    apply_rules_to_element(&stylesheet, &mut document.root);

    // Store the combined stylesheet in the document for later use
    document.computed_stylesheet = Some(stylesheet);

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
                        Value::LinearGradient(_) => String::from("linear-gradient"),
                        Value::BoxShadow(_) => String::from("box-shadow"),
                        Value::Transform(t) => {
                            match t {
                                Transform::Translate(x, y) => alloc::format!("translate({}px, {}px)", x, y),
                                Transform::Rotate(d) => alloc::format!("rotate({}deg)", d),
                                Transform::Scale(x, y) => alloc::format!("scale({}, {})", x, y),
                                Transform::Skew(x, y) => alloc::format!("skew({}deg, {}deg)", x, y),
                                Transform::None => String::from("none"),
                            }
                        }
                        Value::BorderRadius(_) => String::from("border-radius"),
                        Value::Animation(a) => {
                            alloc::format!("{} {}s", a.name, a.duration)
                        }
                        Value::Transition(t) => {
                            alloc::format!("{} {}s", t.property, t.duration)
                        }
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
    println!("[css] CSS engine initialized with animations support");
}

/// Predefined animations
pub mod predefined {
    use super::*;
    use alloc::vec;

    /// Get a predefined bounce animation
    pub fn bounce_keyframes() -> KeyframesRule {
        let mut keyframes: BTreeMap<u8, Vec<Declaration>> = BTreeMap::new();
        
        keyframes.insert(0, vec![
            Declaration { property: String::from("transform"), value: Value::Transform(Transform::Translate(0.0, 0.0)) }
        ]);
        
        keyframes.insert(25, vec![
            Declaration { property: String::from("transform"), value: Value::Transform(Transform::Translate(0.0, -20.0)) }
        ]);
        
        keyframes.insert(50, vec![
            Declaration { property: String::from("transform"), value: Value::Transform(Transform::Translate(0.0, -10.0)) }
        ]);
        
        keyframes.insert(75, vec![
            Declaration { property: String::from("transform"), value: Value::Transform(Transform::Translate(0.0, -20.0)) }
        ]);
        
        keyframes.insert(100, vec![
            Declaration { property: String::from("transform"), value: Value::Transform(Transform::Translate(0.0, 0.0)) }
        ]);

        KeyframesRule {
            name: String::from("bounce"),
            keyframes,
        }
    }

    /// Get a predefined pulse animation
    pub fn pulse_keyframes() -> KeyframesRule {
        let mut keyframes: BTreeMap<u8, Vec<Declaration>> = BTreeMap::new();
        
        keyframes.insert(0, vec![
            Declaration { property: String::from("transform"), value: Value::Transform(Transform::Scale(1.0, 1.0)) }
        ]);
        
        keyframes.insert(50, vec![
            Declaration { property: String::from("transform"), value: Value::Transform(Transform::Scale(1.1, 1.1)) }
        ]);
        
        keyframes.insert(100, vec![
            Declaration { property: String::from("transform"), value: Value::Transform(Transform::Scale(1.0, 1.0)) }
        ]);

        KeyframesRule {
            name: String::from("pulse"),
            keyframes,
        }
    }

    /// Get a predefined rotate animation
    pub fn rotate_keyframes() -> KeyframesRule {
        let mut keyframes: BTreeMap<u8, Vec<Declaration>> = BTreeMap::new();
        
        keyframes.insert(0, vec![
            Declaration { property: String::from("transform"), value: Value::Transform(Transform::Rotate(0.0)) }
        ]);
        
        keyframes.insert(100, vec![
            Declaration { property: String::from("transform"), value: Value::Transform(Transform::Rotate(360.0)) }
        ]);

        KeyframesRule {
            name: String::from("rotate"),
            keyframes,
        }
    }

    /// Get a predefined fade-in animation
    pub fn fade_in_keyframes() -> KeyframesRule {
        let mut keyframes: BTreeMap<u8, Vec<Declaration>> = BTreeMap::new();
        
        keyframes.insert(0, vec![
            Declaration { property: String::from("opacity"), value: Value::Number(0.0) }
        ]);
        
        keyframes.insert(100, vec![
            Declaration { property: String::from("opacity"), value: Value::Number(1.0) }
        ]);

        KeyframesRule {
            name: String::from("fade-in"),
            keyframes,
        }
    }

    /// Get a predefined slide-in animation
    pub fn slide_in_keyframes() -> KeyframesRule {
        let mut keyframes: BTreeMap<u8, Vec<Declaration>> = BTreeMap::new();
        
        keyframes.insert(0, vec![
            Declaration { property: String::from("transform"), value: Value::Transform(Transform::Translate(-100.0, 0.0)) }
        ]);
        
        keyframes.insert(100, vec![
            Declaration { property: String::from("transform"), value: Value::Transform(Transform::Translate(0.0, 0.0)) }
        ]);

        KeyframesRule {
            name: String::from("slide-in"),
            keyframes,
        }
    }

    /// Get all predefined keyframes
    pub fn all_keyframes() -> Vec<KeyframesRule> {
        vec![
            bounce_keyframes(),
            pulse_keyframes(),
            rotate_keyframes(),
            fade_in_keyframes(),
            slide_in_keyframes(),
        ]
    }
}
