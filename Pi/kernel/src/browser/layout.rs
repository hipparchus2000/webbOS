//! Layout Engine
//!
//! Performs CSS box model layout on the DOM tree.

#![allow(dead_code)]

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crate::browser::BrowserError;
use crate::browser::css::{KeyframesRule, Animation as CssAnimation, Transition as CssTransition, TimingFunction, Transform as CssTransform};
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
    /// Animation state
    pub animation_state: Option<AnimationState>,
    /// Transition state
    pub transition_state: Option<TransitionState>,
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
    // Extended CSS3 features
    pub border_radius: Option<BorderRadius>,
    pub box_shadow: Option<Vec<BoxShadow>>,
    pub transform: Option<Transform>,
    pub background_gradient: Option<LinearGradient>,
    pub opacity: f32,
    pub backdrop_filter: Option<BackdropFilter>,
    // Animation and transition
    pub animation: Option<Animation>,
    pub transition: Option<Transition>,
}

impl LayoutStyles {
    pub fn default() -> Self {
        Self {
            display: BoxType::Block,
            background_color: None,
            color: Some(Color { r: 0, g: 0, b: 0, a: 255 }),
            font_size: 16.0,
            font_weight: FontWeight::Normal,
            text_align: TextAlign::Left,
            border_radius: None,
            box_shadow: None,
            transform: None,
            background_gradient: None,
            opacity: 1.0,
            backdrop_filter: None,
            animation: None,
            transition: None,
        }
    }
}

/// Animation configuration
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnimationDirection {
    Normal,
    Reverse,
    Alternate,
    AlternateReverse,
}

impl AnimationDirection {
    pub fn from_css(css_dir: crate::browser::css::AnimationDirection) -> Self {
        use crate::browser::css::AnimationDirection as CssDir;
        match css_dir {
            CssDir::Normal => Self::Normal,
            CssDir::Reverse => Self::Reverse,
            CssDir::Alternate => Self::Alternate,
            CssDir::AlternateReverse => Self::AlternateReverse,
        }
    }
}

impl Animation {
    /// Create from CSS Animation
    pub fn from_css(css_anim: &CssAnimation) -> Self {
        Self {
            name: css_anim.name.clone(),
            duration: css_anim.duration,
            timing_function: css_anim.timing_function,
            delay: css_anim.delay,
            iteration_count: css_anim.iteration_count,
            direction: AnimationDirection::from_css(css_anim.direction),
        }
    }

    /// Parse from CSS string
    pub fn parse(value: &str) -> Option<Self> {
        CssAnimation::parse(value).map(|a| Self::from_css(&a))
    }
}

/// Transition configuration
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
    /// Create from CSS Transition
    pub fn from_css(css_trans: &CssTransition) -> Self {
        Self {
            property: css_trans.property.clone(),
            duration: css_trans.duration,
            timing_function: css_trans.timing_function,
            delay: css_trans.delay,
        }
    }

    /// Parse from CSS string
    pub fn parse(value: &str) -> Option<Self> {
        CssTransition::parse(value).map(|t| Self::from_css(&t))
    }
}

/// Animation state (tracks current animation progress)
#[derive(Debug, Clone)]
pub struct AnimationState {
    /// Reference to keyframes rule
    pub keyframes_name: String,
    /// Animation start time (in milliseconds)
    pub start_time: u64,
    /// Current iteration
    pub current_iteration: u32,
    /// Whether animation is paused
    pub paused: bool,
    /// Pause start time
    pub pause_start: Option<u64>,
    /// Total paused duration
    pub total_paused: u64,
}

impl AnimationState {
    pub fn new(keyframes_name: String, current_time: u64, delay_ms: u64) -> Self {
        Self {
            keyframes_name,
            start_time: current_time + delay_ms,
            current_iteration: 0,
            paused: false,
            pause_start: None,
            total_paused: 0,
        }
    }

    /// Calculate current animation progress (0.0 to 1.0)
    pub fn progress(&self, current_time: u64, duration_ms: u64, iteration_count: f32) -> Option<f32> {
        if self.paused {
            return None;
        }

        let elapsed = current_time.saturating_sub(self.start_time).saturating_sub(self.total_paused);
        
        if iteration_count > 0.0 {
            let total_duration = (duration_ms as f32 * iteration_count) as u64;
            if elapsed >= total_duration {
                return Some(1.0); // Animation complete
            }
        }

        let iteration_duration = duration_ms;
        let iteration_elapsed = elapsed % iteration_duration.max(1);
        let progress = iteration_elapsed as f32 / iteration_duration as f32;
        
        Some(progress.min(1.0))
    }

    /// Pause the animation
    pub fn pause(&mut self, current_time: u64) {
        if !self.paused {
            self.paused = true;
            self.pause_start = Some(current_time);
        }
    }

    /// Resume the animation
    pub fn resume(&mut self, current_time: u64) {
        if self.paused {
            self.paused = false;
            if let Some(start) = self.pause_start {
                self.total_paused += current_time.saturating_sub(start);
            }
            self.pause_start = None;
        }
    }
}

/// Transition state (tracks property transitions)
#[derive(Debug, Clone)]
pub struct TransitionState {
    /// Property being transitioned
    pub property: String,
    /// Start value
    pub start_value: f32,
    /// End value
    pub end_value: f32,
    /// Transition start time (in milliseconds)
    pub start_time: u64,
    /// Duration in milliseconds
    pub duration_ms: u64,
    /// Timing function
    pub timing_function: TimingFunction,
}

impl TransitionState {
    pub fn new(property: String, start_value: f32, end_value: f32, current_time: u64, duration_ms: u64, timing_function: TimingFunction) -> Self {
        Self {
            property,
            start_value,
            end_value,
            start_time: current_time,
            duration_ms,
            timing_function,
        }
    }

    /// Calculate current transition value
    pub fn current_value(&self, current_time: u64) -> f32 {
        let elapsed = current_time.saturating_sub(self.start_time);
        if elapsed >= self.duration_ms {
            return self.end_value;
        }

        let progress = elapsed as f32 / self.duration_ms as f32;
        let eased = self.timing_function.apply(progress);
        
        self.start_value + (self.end_value - self.start_value) * eased
    }

    /// Check if transition is complete
    pub fn is_complete(&self, current_time: u64) -> bool {
        current_time.saturating_sub(self.start_time) >= self.duration_ms
    }
}

/// Border radius
#[derive(Debug, Clone)]
pub struct BorderRadius {
    pub top_left: f32,
    pub top_right: f32,
    pub bottom_right: f32,
    pub bottom_left: f32,
}

/// Box shadow
#[derive(Debug, Clone)]
pub struct BoxShadow {
    pub offset_x: f32,
    pub offset_y: f32,
    pub blur_radius: f32,
    pub spread_radius: f32,
    pub color: Color,
}

/// CSS Transform
#[derive(Debug, Clone)]
pub enum Transform {
    Translate(f32, f32),
    Rotate(f32),
    Scale(f32, f32),
    Skew(f32, f32),
    Multiple(Vec<Transform>),
}

impl Transform {
    /// Interpolate between two transforms
    pub fn interpolate(&self, other: &Transform, t: f32) -> Transform {
        let t = t.clamp(0.0, 1.0);
        
        match (self, other) {
            (Transform::Translate(x1, y1), Transform::Translate(x2, y2)) => {
                Transform::Translate(
                    x1 + (x2 - x1) * t,
                    y1 + (y2 - y1) * t,
                )
            }
            (Transform::Rotate(r1), Transform::Rotate(r2)) => {
                Transform::Rotate(r1 + (r2 - r1) * t)
            }
            (Transform::Scale(x1, y1), Transform::Scale(x2, y2)) => {
                Transform::Scale(
                    x1 + (x2 - x1) * t,
                    y1 + (y2 - y1) * t,
                )
            }
            (Transform::Skew(x1, y1), Transform::Skew(x2, y2)) => {
                Transform::Skew(
                    x1 + (x2 - x1) * t,
                    y1 + (y2 - y1) * t,
                )
            }
            _ => other.clone(),
        }
    }

    /// Create from CSS Transform
    pub fn from_css(css_transform: &CssTransform) -> Option<Self> {
        match css_transform {
            CssTransform::Translate(x, y) => Some(Transform::Translate(*x, *y)),
            CssTransform::Rotate(r) => Some(Transform::Rotate(*r)),
            CssTransform::Scale(x, y) => Some(Transform::Scale(*x, *y)),
            CssTransform::Skew(x, y) => Some(Transform::Skew(*x, *y)),
            CssTransform::None => None,
        }
    }
}

/// Linear gradient
#[derive(Debug, Clone)]
pub struct LinearGradient {
    pub angle: f32,
    pub stops: Vec<GradientStop>,
}

/// Gradient color stop
#[derive(Debug, Clone)]
pub struct GradientStop {
    pub color: Color,
    pub position: f32,
}

/// Backdrop filter (for glassmorphism)
#[derive(Debug, Clone)]
pub enum BackdropFilter {
    Blur(f32),
    Brightness(f32),
    Contrast(f32),
}

/// Color with alpha
#[derive(Debug, Clone, Copy)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub fn black() -> Self {
        Self { r: 0, g: 0, b: 0, a: 255 }
    }

    pub fn white() -> Self {
        Self { r: 255, g: 255, b: 255, a: 255 }
    }

    pub fn gray() -> Self {
        Self { r: 128, g: 128, b: 128, a: 255 }
    }
    
    pub fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }
    
    /// Convert to ARGB u32
    pub fn to_u32(&self) -> u32 {
        ((self.a as u32) << 24) | ((self.r as u32) << 16) | ((self.g as u32) << 8) | (self.b as u32)
    }
    
    /// Blend this color over another (for transparency)
    pub fn blend_over(&self, background: Color) -> Color {
        if self.a == 255 {
            return *self;
        }
        if self.a == 0 {
            return background;
        }
        
        let alpha = self.a as f32 / 255.0;
        let inv_alpha = 1.0 - alpha;
        
        Color {
            r: (self.r as f32 * alpha + background.r as f32 * inv_alpha) as u8,
            g: (self.g as f32 * alpha + background.g as f32 * inv_alpha) as u8,
            b: (self.b as f32 * alpha + background.b as f32 * inv_alpha) as u8,
            a: 255,
        }
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
    let _containing_block = Dimensions {
        width: viewport_width as f32,
        height: viewport_height as f32,
    };
    
    calculate_layout(&mut root_box, &_containing_block);
    
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
        animation_state: None,
        transition_state: None,
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
                        animation_state: None,
                        transition_state: None,
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
            "opacity" => {
                if let Some(opacity) = parse_number(val) {
                    styles.opacity = opacity.clamp(0.0, 1.0);
                }
            }
            "transform" => {
                styles.transform = parse_transform(val);
            }
            "animation" | "animation-name" => {
                // Try parsing as shorthand first
                if let Some(anim) = Animation::parse(val) {
                    styles.animation = Some(anim);
                } else if !val.is_empty() && val != "none" {
                    // Simple name-only parsing
                    styles.animation = Some(Animation {
                        name: val.to_string(),
                        duration: 1.0,
                        timing_function: TimingFunction::Ease,
                        delay: 0.0,
                        iteration_count: 1.0,
                        direction: AnimationDirection::Normal,
                    });
                }
            }
            "transition" | "transition-property" => {
                if let Some(trans) = Transition::parse(val) {
                    styles.transition = Some(trans);
                }
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
        "red" => return Some(Color { r: 255, g: 0, b: 0, a: 255 }),
        "green" => return Some(Color { r: 0, g: 128, b: 0, a: 255 }),
        "blue" => return Some(Color { r: 0, g: 0, b: 255, a: 255 }),
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
                return Some(Color { r, g, b, a: 255 });
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

/// Parse number value
fn parse_number(s: &str) -> Option<f32> {
    s.parse().ok()
}

/// Parse transform value
fn parse_transform(s: &str) -> Option<Transform> {
    let s = s.trim();
    
    if s.starts_with("translate(") && s.ends_with(')') {
        let inner = &s[10..s.len()-1];
        let parts: Vec<&str> = inner.split(',').collect();
        if parts.len() == 2 {
            let x = parse_length(parts[0].trim())?;
            let y = parse_length(parts[1].trim())?;
            return Some(Transform::Translate(x, y));
        }
    }
    
    if s.starts_with("rotate(") && s.ends_with(')') {
        let inner = &s[7..s.len()-1];
        if inner.ends_with("deg") {
            if let Ok(deg) = inner[..inner.len()-3].parse::<f32>() {
                return Some(Transform::Rotate(deg));
            }
        }
    }
    
    if s.starts_with("scale(") && s.ends_with(')') {
        let inner = &s[6..s.len()-1];
        let parts: Vec<&str> = inner.split(',').collect();
        if parts.len() == 1 {
            if let Ok(s) = parts[0].trim().parse::<f32>() {
                return Some(Transform::Scale(s, s));
            }
        } else if parts.len() == 2 {
            if let (Ok(x), Ok(y)) = (parts[0].trim().parse::<f32>(), parts[1].trim().parse::<f32>()) {
                return Some(Transform::Scale(x, y));
            }
        }
    }
    
    if s == "none" {
        return None;
    }
    
    None
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

fn calculate_inline_layout(layout_box: &mut LayoutBox, _containing_block: &Dimensions) {
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

fn calculate_inline_block_layout(layout_box: &mut LayoutBox, _containing_block: &Dimensions) {
    // Similar to block but with natural width
    if layout_box.width == 0.0 {
        layout_box.width = 100.0; // Default width
    }
    
    layout_box.content_width = layout_box.width - layout_box.padding.horizontal() - layout_box.border.horizontal();
    layout_box.content_height = layout_box.styles.font_size * 1.2;
    layout_box.height = layout_box.content_height + layout_box.padding.vertical() + layout_box.border.vertical();
}

/// Initialize animation state for a layout box
pub fn init_animation_state(layout_box: &mut LayoutBox, current_time: u64) {
    if let Some(ref anim) = layout_box.styles.animation {
        if layout_box.animation_state.is_none() {
            layout_box.animation_state = Some(AnimationState::new(
                anim.name.clone(),
                current_time,
                (anim.delay * 1000.0) as u64,
            ));
        }
    }

    // Initialize for children
    for child in &mut layout_box.children {
        init_animation_state(child, current_time);
    }
}

/// Update animations for a layout tree
pub fn update_animations(layout_box: &mut LayoutBox, current_time: u64, keyframes: &[KeyframesRule]) {
    // Update this box's animation
    if let Some(ref mut state) = layout_box.animation_state {
        if let Some(ref anim) = layout_box.styles.animation {
            // Check if animation is complete
            let progress = state.progress(
                current_time,
                (anim.duration * 1000.0) as u64,
                anim.iteration_count,
            );
            
            if progress.is_none() {
                // Animation is paused
            } else if let Some(p) = progress {
                if p >= 1.0 && anim.iteration_count > 0.0 {
                    // Animation complete - keep at final state
                }
            }
        }
    }

    // Update children
    for child in &mut layout_box.children {
        update_animations(child, current_time, keyframes);
    }
}

/// Get interpolated transform based on animation progress
pub fn get_animated_transform(
    layout_box: &LayoutBox,
    current_time: u64,
    keyframes: &[KeyframesRule],
) -> Option<Transform> {
    let anim_state = layout_box.animation_state.as_ref()?;
    let anim = layout_box.styles.animation.as_ref()?;
    
    // Find the keyframes rule
    let keyframes_rule = keyframes.iter().find(|k| k.name == anim.name)?;
    
    // Calculate progress
    let progress = anim_state.progress(
        current_time,
        (anim.duration * 1000.0) as u64,
        anim.iteration_count,
    )?;
    
    let progress = anim.timing_function.apply(progress);
    
    // Apply direction
    let adjusted_progress = match anim.direction {
        AnimationDirection::Normal => progress,
        AnimationDirection::Reverse => 1.0 - progress,
        AnimationDirection::Alternate => {
            if anim_state.current_iteration % 2 == 0 {
                progress
            } else {
                1.0 - progress
            }
        }
        AnimationDirection::AlternateReverse => {
            if anim_state.current_iteration % 2 == 0 {
                1.0 - progress
            } else {
                progress
            }
        }
    };
    
    // Find the two keyframes we're between
    let percentage = (adjusted_progress * 100.0) as u8;
    
    let mut prev_pct: Option<u8> = None;
    let mut next_pct: Option<u8> = None;
    
    for (&pct, _) in &keyframes_rule.keyframes {
        if pct <= percentage {
            prev_pct = Some(pct);
        }
        if pct >= percentage && next_pct.is_none() {
            next_pct = Some(pct);
            break;
        }
    }
    
    let prev_pct = prev_pct?;
    let next_pct = next_pct.unwrap_or(prev_pct);
    
    // Get transform values from keyframes
    let prev_transform = get_transform_from_keyframe(keyframes_rule, prev_pct);
    let next_transform = get_transform_from_keyframe(keyframes_rule, next_pct);
    
    // Calculate local progress between keyframes
    let local_progress = if next_pct == prev_pct {
        0.0
    } else {
        (percentage - prev_pct) as f32 / (next_pct - prev_pct) as f32
    };
    
    // Interpolate
    match (prev_transform, next_transform) {
        (Some(prev), Some(next)) => Some(prev.interpolate(&next, local_progress)),
        (Some(prev), None) => Some(prev),
        (None, Some(next)) => Some(next),
        (None, None) => layout_box.styles.transform.clone(),
    }
}

/// Get transform from a keyframe at a specific percentage
fn get_transform_from_keyframe(keyframes_rule: &KeyframesRule, percentage: u8) -> Option<Transform> {
    let declarations = keyframes_rule.keyframes.get(&percentage)?;
    
    for decl in declarations {
        if decl.property == "transform" {
            match &decl.value {
                crate::browser::css::Value::Transform(t) => {
                    return Transform::from_css(t);
                }
                _ => {}
            }
        }
    }
    
    None
}

/// Get animated opacity
pub fn get_animated_opacity(
    layout_box: &LayoutBox,
    current_time: u64,
    keyframes: &[KeyframesRule],
) -> f32 {
    let base_opacity = layout_box.styles.opacity;
    
    let anim_state = match layout_box.animation_state.as_ref() {
        Some(s) => s,
        None => return base_opacity,
    };
    
    let anim = match layout_box.styles.animation.as_ref() {
        Some(a) => a,
        None => return base_opacity,
    };
    
    // Find the keyframes rule
    let keyframes_rule = match keyframes.iter().find(|k| k.name == anim.name) {
        Some(k) => k,
        None => return base_opacity,
    };
    
    // Calculate progress
    let progress = match anim_state.progress(
        current_time,
        (anim.duration * 1000.0) as u64,
        anim.iteration_count,
    ) {
        Some(p) => p,
        None => return base_opacity,
    };
    
    let progress = anim.timing_function.apply(progress);
    
    // Check if keyframes define opacity
    let percentage = (progress * 100.0) as u8;
    
    let mut prev_pct: Option<u8> = None;
    let mut next_pct: Option<u8> = None;
    
    for (&pct, _) in &keyframes_rule.keyframes {
        if pct <= percentage {
            prev_pct = Some(pct);
        }
        if pct >= percentage && next_pct.is_none() {
            next_pct = Some(pct);
            break;
        }
    }
    
    if let (Some(prev), Some(next)) = (prev_pct, next_pct) {
        let prev_opacity = get_opacity_from_keyframe(keyframes_rule, prev).unwrap_or(base_opacity);
        let next_opacity = get_opacity_from_keyframe(keyframes_rule, next).unwrap_or(base_opacity);
        
        if next == prev {
            return prev_opacity;
        }
        
        let local_progress = (percentage - prev) as f32 / (next - prev) as f32;
        return prev_opacity + (next_opacity - prev_opacity) * local_progress;
    }
    
    base_opacity
}

/// Get opacity from a keyframe at a specific percentage
fn get_opacity_from_keyframe(keyframes_rule: &KeyframesRule, percentage: u8) -> Option<f32> {
    let declarations = keyframes_rule.keyframes.get(&percentage)?;
    
    for decl in declarations {
        if decl.property == "opacity" {
            match &decl.value {
                crate::browser::css::Value::Number(n) => return Some(*n),
                _ => {}
            }
        }
    }
    
    None
}

/// Initialize layout engine
pub fn init() {
    println!("[layout] Layout engine initialized with animation support");
}
