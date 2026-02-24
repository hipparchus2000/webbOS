//! Layout Engine
//!
//! Performs CSS box model layout on the DOM tree.

use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use crate::browser::BrowserError;
use crate::browser::html::{Document, Element, Node};
use crate::println;

/// Layout box
#[derive(Debug)]
pub struct LayoutBox {
    /// Position (x, y)
    pub x: f32,
    pub y: f32,
    /// Dimensions
    pub width: f32,
    pub height: f32,
    /// Padding
    pub padding: Edge,
    /// Border
    pub border: Edge,
    /// Margin
    pub margin: Edge,
    /// Content width
    pub content_width: f32,
    /// Content height
    pub content_height: f32,
    /// Box type
    pub box_type: BoxType,
    /// Children
    pub children: Vec<LayoutBox>,
    /// Text content
    pub text: Option<String>,
    /// Styles
    pub styles: LayoutStyles,
}

/// Box type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoxType {
    Block,
    Inline,
    InlineBlock,
    None,
}

/// Edge values (padding, border, margin)
#[derive(Debug, Clone, Copy)]
pub struct Edge {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

impl Edge {
    pub fn new() -> Self {
        Self {
            top: 0.0,
            right: 0.0,
            bottom: 0.0,
            left: 0.0,
        }
    }

    pub fn uniform(value: f32) -> Self {
        Self {
            top: value,
            right: value,
            bottom: value,
            left: value,
        }
    }

    pub fn horizontal(&self) -> f32 {
        self.left + self.right
    }

    pub fn vertical(&self) -> f32 {
        self.top + self.bottom
    }
}

/// Layout styles
#[derive(Debug, Clone)]
pub struct LayoutStyles {
    pub display: BoxType,
    pub background_color: Option<Color>,
    pub color: Option<Color>,
    pub font_size: f32,
    pub font_weight: FontWeight,
    pub text_align: TextAlign,
}

impl LayoutStyles {
    pub fn default() -> Self {
        Self {
            display: BoxType::Block,
            background_color: None,
            color: Some(Color { r: 0, g: 0, b: 0 }),
            font_size: 16.0,
            font_weight: FontWeight::Normal,
            text_align: TextAlign::Left,
        }
    }
}

/// Color
#[derive(Debug, Clone, Copy)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Color {
    pub fn black() -> Self {
        Self { r: 0, g: 0, b: 0 }
    }

    pub fn white() -> Self {
        Self { r: 255, g: 255, b: 255 }
    }

    pub fn gray() -> Self {
        Self { r: 128, g: 128, b: 128 }
    }
}

/// Font weight
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontWeight {
    Normal,
    Bold,
}

/// Text alignment
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextAlign {
    Left,
    Center,
    Right,
    Justify,
}

/// Layout tree
pub struct LayoutTree {
    pub root: LayoutBox,
    pub viewport_width: f32,
    pub viewport_height: f32,
}

/// Perform layout on document
pub fn layout(document: &Document, viewport_width: u32, viewport_height: u32) -> Result<LayoutTree, BrowserError> {
    let mut root_box = build_layout_tree(&document.root)?;
    
    // Calculate layout
    let containing_block = Dimensions {
        width: viewport_width as f32,
        height: viewport_height as f32,
    };
    
    calculate_layout(&mut root_box, &containing_block);
    
    Ok(LayoutTree {
        root: root_box,
        viewport_width: viewport_width as f32,
        viewport_height: viewport_height as f32,
    })
}

/// Dimensions for containing block
struct Dimensions {
    width: f32,
    height: f32,
}

/// Build layout tree from DOM element
fn build_layout_tree(element: &Element) -> Result<LayoutBox, BrowserError> {
    let box_type = determine_box_type(element);
    
    let styles = compute_styles(element);
    
    let mut layout_box = LayoutBox {
        x: 0.0,
        y: 0.0,
        width: 0.0,
        height: 0.0,
        padding: Edge::new(),
        border: Edge::new(),
        margin: Edge::new(),
        content_width: 0.0,
        content_height: 0.0,
        box_type,
        children: Vec::new(),
        text: None,
        styles,
    };

    // Build children
    for child in &element.children {
        match child {
            Node::Element(elem) => {
                let child_box = build_layout_tree(elem)?;
                if child_box.box_type != BoxType::None {
                    layout_box.children.push(child_box);
                }
            }
            Node::Text(text) => {
                if !text.trim().is_empty() {
                    let text_box = LayoutBox {
                        x: 0.0,
                        y: 0.0,
                        width: 0.0,
                        height: 0.0,
                        padding: Edge::new(),
                        border: Edge::new(),
                        margin: Edge::new(),
                        content_width: 0.0,
                        content_height: 0.0,
                        box_type: BoxType::Inline,
                        children: Vec::new(),
                        text: Some(text.clone()),
                        styles: layout_box.styles.clone(),
                    };
                    layout_box.children.push(text_box);
                }
            }
            _ => {}
        }
    }

    Ok(layout_box)
}

/// Determine box type from element
fn determine_box_type(element: &Element) -> BoxType {
    match element.tag.as_str() {
        "head" | "script" | "style" | "meta" | "link" => BoxType::None,
        "span" | "a" | "em" | "strong" | "code" | "b" | "i" | "u" => BoxType::Inline,
        "img" | "input" | "button" => BoxType::InlineBlock,
        _ => BoxType::Block,
    }
}

/// Compute layout styles from element
fn compute_styles(element: &Element) -> LayoutStyles {
    let mut styles = LayoutStyles::default();

    // Check for display: none
    for (prop, val) in &element.computed_styles {
        match prop.as_str() {
            "display" => {
                styles.display = match val.as_str() {
                    "none" => BoxType::None,
                    "inline" => BoxType::Inline,
                    "inline-block" => BoxType::InlineBlock,
                    _ => BoxType::Block,
                };
            }
            "background-color" => {
                styles.background_color = parse_color(val);
            }
            "color" => {
                styles.color = parse_color(val);
            }
            "font-size" => {
                if let Some(size) = parse_length(val) {
                    styles.font_size = size;
                }
            }
            "font-weight" => {
                if val == "bold" || val == "700" {
                    styles.font_weight = FontWeight::Bold;
                }
            }
            "text-align" => {
                styles.text_align = match val.as_str() {
                    "center" => TextAlign::Center,
                    "right" => TextAlign::Right,
                    "justify" => TextAlign::Justify,
                    _ => TextAlign::Left,
                };
            }
            _ => {}
        }
    }

    styles
}

/// Parse color value
fn parse_color(s: &str) -> Option<Color> {
    // Named colors
    match s.to_ascii_lowercase().as_str() {
        "black" => return Some(Color::black()),
        "white" => return Some(Color::white()),
        "gray" | "grey" => return Some(Color::gray()),
        "red" => return Some(Color { r: 255, g: 0, b: 0 }),
        "green" => return Some(Color { r: 0, g: 128, b: 0 }),
        "blue" => return Some(Color { r: 0, g: 0, b: 255 }),
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
                return Some(Color { r, g, b });
            }
        }
    }

    None
}

/// Parse length value
fn parse_length(s: &str) -> Option<f32> {
    if s.ends_with("px") {
        s[..s.len()-2].parse().ok()
    } else if s.ends_with("em") {
        s[..s.len()-2].parse::<f32>().map(|v| v * 16.0).ok()
    } else if s.ends_with("rem") {
        s[..s.len()-3].parse::<f32>().map(|v| v * 16.0).ok()
    } else if s.ends_with("pt") {
        s[..s.len()-2].parse::<f32>().map(|v| v * 1.33).ok()
    } else {
        s.parse().ok()
    }
}

/// Calculate layout dimensions
fn calculate_layout(layout_box: &mut LayoutBox, containing_block: &Dimensions) {
    match layout_box.box_type {
        BoxType::Block => calculate_block_layout(layout_box, containing_block),
        BoxType::Inline => calculate_inline_layout(layout_box, containing_block),
        BoxType::InlineBlock => calculate_inline_block_layout(layout_box, containing_block),
        BoxType::None => {}
    }
}

/// Calculate block-level layout
fn calculate_block_layout(layout_box: &mut LayoutBox, containing_block: &Dimensions) {
    // Calculate width
    layout_box.width = containing_block.width;
    layout_box.content_width = layout_box.width - layout_box.padding.horizontal() - layout_box.border.horizontal() - layout_box.margin.horizontal();

    // Calculate children
    let mut current_y = layout_box.padding.top + layout_box.border.top + layout_box.margin.top;
    
    for child in &mut layout_box.children {
        child.x = layout_box.padding.left + layout_box.border.left;
        child.y = current_y;
        
        let child_containing = Dimensions {
            width: layout_box.content_width,
            height: containing_block.height,
        };
        calculate_layout(child, &child_containing);
        
        current_y += child.height;
    }

    // Calculate height
    layout_box.content_height = current_y;
    layout_box.height = layout_box.content_height + layout_box.padding.vertical() + layout_box.border.vertical() + layout_box.margin.vertical();
}

/// Calculate inline layout
fn calculate_inline_layout(layout_box: &mut LayoutBox, containing_block: &Dimensions) {
    // Simple inline layout - just estimate text size
    if let Some(ref text) = layout_box.text {
        // Rough estimate: 8 pixels per character
        let char_width = layout_box.styles.font_size * 0.6;
        let char_height = layout_box.styles.font_size * 1.2;
        
        layout_box.content_width = text.len() as f32 * char_width;
        layout_box.content_height = char_height;
    } else {
        layout_box.content_width = 0.0;
        layout_box.content_height = layout_box.styles.font_size;
    }

    layout_box.width = layout_box.content_width + layout_box.padding.horizontal() + layout_box.border.horizontal();
    layout_box.height = layout_box.content_height + layout_box.padding.vertical() + layout_box.border.vertical();
}

/// Calculate inline-block layout
fn calculate_inline_block_layout(layout_box: &mut LayoutBox, containing_block: &Dimensions) {
    // Similar to block but with natural width
    if layout_box.width == 0.0 {
        layout_box.width = 100.0; // Default width
    }
    
    layout_box.content_width = layout_box.width - layout_box.padding.horizontal() - layout_box.border.horizontal();
    layout_box.content_height = layout_box.styles.font_size * 1.2;
    layout_box.height = layout_box.content_height + layout_box.padding.vertical() + layout_box.border.vertical();
}

/// Initialize layout engine
pub fn init() {
    println!("[layout] Layout engine initialized");
}
