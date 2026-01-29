//! HTML Parser
//!
//! Parses HTML documents into a DOM tree.

use alloc::string::String;
use alloc::vec::Vec;
use alloc::boxed::Box;

use crate::browser::BrowserError;
use crate::println;

/// HTML Document
pub struct Document {
    /// Document type
    pub doctype: Option<String>,
    /// Root element (<html>)
    pub root: Element,
    /// Document scripts
    pub scripts: Vec<Script>,
    /// Document stylesheets
    pub stylesheets: Vec<StylesheetRef>,
}

impl Document {
    /// Get total element count
    pub fn element_count(&self) -> usize {
        self.root.count_descendants()
    }
}

/// HTML Element
pub struct Element {
    /// Tag name
    pub tag: String,
    /// Attributes
    pub attributes: Vec<(String, String)>,
    /// Child nodes
    pub children: Vec<Node>,
    /// Computed styles (filled by CSS engine)
    pub computed_styles: Vec<(String, String)>,
}

impl Element {
    /// Create new element
    pub fn new(tag: &str) -> Self {
        Self {
            tag: String::from(tag),
            attributes: Vec::new(),
            children: Vec::new(),
            computed_styles: Vec::new(),
        }
    }

    /// Get attribute value
    pub fn get_attr(&self, name: &str) -> Option<&str> {
        for (k, v) in &self.attributes {
            if k == name {
                return Some(v);
            }
        }
        None
    }

    /// Count all descendant elements
    pub fn count_descendants(&self) -> usize {
        let mut count = 1; // Self
        for child in &self.children {
            if let Node::Element(ref elem) = child {
                count += elem.count_descendants();
            }
        }
        count
    }
}

/// DOM Node
pub enum Node {
    Element(Element),
    Text(String),
    Comment(String),
}

/// Script element
pub struct Script {
    /// Script source URL (if external)
    pub src: Option<String>,
    /// Script content (if inline)
    pub content: Vec<u8>,
    /// Async loading
    pub async_: bool,
    /// Deferred loading
    pub defer: bool,
}

/// Stylesheet reference
pub struct StylesheetRef {
    /// URL (if external)
    pub href: Option<String>,
    /// Inline content
    pub content: String,
}

/// HTML Token
#[derive(Debug, Clone)]
enum Token {
    Doctype(String),
    StartTag(String, Vec<(String, String)>),
    EndTag(String),
    Text(String),
    Comment(String),
    EOF,
}

/// Tokenize HTML
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

    fn consume_until(&mut self, target: u8) -> String {
        let mut result = String::new();
        while let Some(ch) = self.peek() {
            if ch == target {
                break;
            }
            result.push(ch as char);
            self.next();
        }
        result
    }

    fn tokenize(&mut self) -> Vec<Token> {
        let mut tokens = Vec::new();

        while self.pos < self.input.len() {
            if let Some(ch) = self.peek() {
                if ch == b'<' {
                    // Parse tag
                    self.next(); // consume '<'
                    
                    if self.peek() == Some(b'!') {
                        // Doctype or comment
                        self.next(); // consume '!'
                        if self.peek() == Some(b'-') && self.input.get(self.pos + 1) == Some(&b'-') {
                            // Comment
                            self.pos += 2; // skip '--'
                            let comment = self.consume_until(b'-');
                            self.pos += 2; // skip '-->'
                            tokens.push(Token::Comment(comment));
                        } else {
                            // Doctype
                            let content = self.consume_until(b'>');
                            tokens.push(Token::Doctype(content));
                        }
                    } else if self.peek() == Some(b'/') {
                        // End tag
                        self.next(); // consume '/'
                        let tag = self.parse_tag_name();
                        self.consume_until(b'>');
                        self.next(); // consume '>'
                        tokens.push(Token::EndTag(tag));
                    } else {
                        // Start tag
                        let (tag, attrs) = self.parse_start_tag();
                        tokens.push(Token::StartTag(tag, attrs));
                    }
                } else {
                    // Text content
                    let text = self.consume_until(b'<');
                    if !text.trim().is_empty() {
                        tokens.push(Token::Text(text));
                    }
                }
            } else {
                break;
            }
        }

        tokens.push(Token::EOF);
        tokens
    }

    fn parse_tag_name(&mut self) -> String {
        self.consume_whitespace();
        let mut name = String::new();
        
        while let Some(ch) = self.peek() {
            if ch.is_ascii_alphanumeric() || ch == b'-' {
                name.push(ch.to_ascii_lowercase() as char);
                self.next();
            } else {
                break;
            }
        }
        
        name
    }

    fn parse_start_tag(&mut self) -> (String, Vec<(String, String)>) {
        let tag = self.parse_tag_name();
        let mut attrs = Vec::new();

        // Parse attributes
        loop {
            self.consume_whitespace();
            
            if self.peek() == Some(b'>') || self.peek() == Some(b'/') {
                break;
            }

            let name = self.parse_attr_name();
            let value = if self.peek() == Some(b'=') {
                self.next(); // consume '='
                self.parse_attr_value()
            } else {
                String::new()
            };

            attrs.push((name, value));
        }

        // Consume self-closing marker if present
        if self.peek() == Some(b'/') {
            self.next();
        }
        
        // Consume '>'
        if self.peek() == Some(b'>') {
            self.next();
        }

        (tag, attrs)
    }

    fn parse_attr_name(&mut self) -> String {
        let mut name = String::new();
        
        while let Some(ch) = self.peek() {
            if ch.is_ascii_alphanumeric() || ch == b'-' || ch == b'_' || ch == b':' {
                name.push(ch.to_ascii_lowercase() as char);
                self.next();
            } else {
                break;
            }
        }
        
        name
    }

    fn parse_attr_value(&mut self) -> String {
        self.consume_whitespace();
        
        let quote = self.peek();
        if quote == Some(b'"') || quote == Some(b'\'') {
            self.next(); // consume quote
            let value = self.consume_until(quote.unwrap());
            self.next(); // consume closing quote
            value
        } else {
            let mut value = String::new();
            while let Some(ch) = self.peek() {
                if ch.is_ascii_whitespace() || ch == b'>' || ch == b'/' {
                    break;
                }
                value.push(ch as char);
                self.next();
            }
            value
        }
    }
}

/// Build DOM from tokens
struct DomBuilder {
    stack: Vec<Element>,
    scripts: Vec<Script>,
    stylesheets: Vec<StylesheetRef>,
}

impl DomBuilder {
    fn new() -> Self {
        Self {
            stack: Vec::new(),
            scripts: Vec::new(),
            stylesheets: Vec::new(),
        }
    }

    fn build(mut self, tokens: &[Token]) -> Result<Document, BrowserError> {
        let mut doctype = None;

        for token in tokens {
            match token {
                Token::Doctype(dt) => {
                    doctype = Some(dt.clone());
                }
                Token::StartTag(tag, attrs) => {
                    let mut elem = Element::new(tag);
                    elem.attributes = attrs.clone();

                    // Handle special elements
                    match tag.as_str() {
                        "script" => {
                            // Extract script info
                            let src = elem.get_attr("src").map(String::from);
                            let async_ = elem.get_attr("async").is_some();
                            let defer = elem.get_attr("defer").is_some();
                            
                            self.scripts.push(Script {
                                src,
                                content: Vec::new(),
                                async_,
                                defer,
                            });
                        }
                        "link" => {
                            if elem.get_attr("rel") == Some("stylesheet") {
                                let href = elem.get_attr("href").map(String::from);
                                self.stylesheets.push(StylesheetRef {
                                    href,
                                    content: String::new(),
                                });
                            }
                        }
                        "style" => {
                            // Inline stylesheet - content will be in text child
                            self.stylesheets.push(StylesheetRef {
                                href: None,
                                content: String::new(),
                            });
                        }
                        _ => {}
                    }

                    self.stack.push(elem);
                }
                Token::EndTag(tag) => {
                    if let Some(mut elem) = self.stack.pop() {
                        // Capture inline script/style content
                        match tag.as_str() {
                            "script" => {
                                if let Some(last) = self.scripts.last_mut() {
                                    if last.src.is_none() {
                                        // Inline script - get text content from children
                                        let mut content = Vec::new();
                                        for child in &elem.children {
                                            if let Node::Text(text) = child {
                                                content.extend_from_slice(text.as_bytes());
                                            }
                                        }
                                        last.content = content;
                                    }
                                }
                            }
                            "style" => {
                                if let Some(last) = self.stylesheets.last_mut() {
                                    if last.href.is_none() {
                                        // Inline stylesheet
                                        let mut content = String::new();
                                        for child in &elem.children {
                                            if let Node::Text(text) = child {
                                                content.push_str(text);
                                            }
                                        }
                                        last.content = content;
                                    }
                                }
                            }
                            _ => {}
                        }

                        // Add to parent if exists
                        if let Some(parent) = self.stack.last_mut() {
                            parent.children.push(Node::Element(elem));
                        } else if tag == "html" {
                            // Root element
                            self.stack.push(elem);
                        }
                    }
                }
                Token::Text(text) => {
                    if let Some(parent) = self.stack.last_mut() {
                        parent.children.push(Node::Text(text.clone()));
                    }
                }
                Token::Comment(_) => {
                    // Ignore comments for now
                }
                Token::EOF => break,
            }
        }

        // Get root element
        let root = if self.stack.len() == 1 {
            self.stack.pop().unwrap()
        } else {
            Element::new("html")
        };

        Ok(Document {
            doctype,
            root,
            scripts: self.scripts,
            stylesheets: self.stylesheets,
        })
    }
}

/// Parse HTML document
pub fn parse(input: &[u8]) -> Result<Document, BrowserError> {
    let mut tokenizer = Tokenizer::new(input);
    let tokens = tokenizer.tokenize();
    
    let builder = DomBuilder::new();
    builder.build(&tokens)
}

/// Initialize HTML parser
pub fn init() {
    println!("[html] HTML parser initialized");
}

/// Create a simple test document
pub fn create_test_document() -> Document {
    let mut html = Element::new("html");
    let mut head = Element::new("head");
    let mut body = Element::new("body");
    
    // Add title
    let mut title = Element::new("title");
    title.children.push(Node::Text(String::from("WebbOS Browser")));
    head.children.push(Node::Element(title));
    
    // Add heading
    let mut h1 = Element::new("h1");
    h1.children.push(Node::Text(String::from("Welcome to WebbOS!")));
    body.children.push(Node::Element(h1));
    
    // Add paragraph
    let mut p = Element::new("p");
    p.children.push(Node::Text(String::from("This is a test page.")));
    body.children.push(Node::Element(p));
    
    html.children.push(Node::Element(head));
    html.children.push(Node::Element(body));
    
    Document {
        doctype: Some(String::from("html")),
        root: html,
        scripts: Vec::new(),
        stylesheets: Vec::new(),
    }
}
