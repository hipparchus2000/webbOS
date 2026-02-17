//! PWA Manifest Parser
//!
//! Parses manifest.json files into PwaManifest structures.
//! Supports standard Web App Manifest format with WebbOS extensions.

use super::{PwaManifest, DisplayMode, PwaResult, PwaError};
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::format;

/// Parse a manifest from JSON string
pub fn parse(json_str: &str) -> PwaResult<PwaManifest> {
    // Basic JSON parsing for no-std environment
    // This is a simplified parser that handles the essential manifest fields
    
    let mut manifest = PwaManifest::new("Unnamed App");
    
    // Extract string values
    if let Some(value) = extract_string_field(json_str, "name") {
        manifest.name = value.clone();
        manifest.short_name = value;
    }
    
    if let Some(value) = extract_string_field(json_str, "short_name") {
        manifest.short_name = value;
    }
    
    if let Some(value) = extract_string_field(json_str, "description") {
        manifest.description = value;
    }
    
    if let Some(value) = extract_string_field(json_str, "start_url") {
        manifest.start_url = value;
    }
    
    if let Some(value) = extract_string_field(json_str, "display") {
        manifest.display = DisplayMode::from_str(&value);
    }
    
    if let Some(value) = extract_string_field(json_str, "background_color") {
        manifest.background_color = parse_color(&value);
    }
    
    if let Some(value) = extract_string_field(json_str, "theme_color") {
        manifest.theme_color = parse_color(&value);
    }
    
    if let Some(value) = extract_string_field(json_str, "version") {
        manifest.version = value;
    }
    
    if let Some(value) = extract_string_field(json_str, "author") {
        manifest.author = Some(value);
    }
    
    // Extract icon - handle both simple string and icons array
    if let Some(icon) = extract_icon_path(json_str) {
        manifest.icon = Some(icon);
    }
    
    // Extract permissions array
    manifest.permissions = extract_string_array(json_str, "permissions");
    
    // Validate the manifest
    validate(&manifest)?;
    
    Ok(manifest)
}

/// Parse manifest from bytes
pub fn parse_bytes(bytes: &[u8]) -> PwaResult<PwaManifest> {
    let json_str = core::str::from_utf8(bytes)
        .map_err(|_| PwaError::invalid_manifest("Invalid UTF-8 in manifest"))?;
    parse(json_str)
}

/// Validate a manifest
fn validate(manifest: &PwaManifest) -> PwaResult<()> {
    if manifest.name.is_empty() {
        return Err(PwaError::invalid_manifest("App name cannot be empty"));
    }
    
    if manifest.start_url.is_empty() {
        return Err(PwaError::invalid_manifest("Start URL cannot be empty"));
    }
    
    // Check for valid color format (already parsed, so just ensure non-zero if specified)
    // Note: 0 is valid (transparent black), so we don't validate here
    
    Ok(())
}

/// Extract a string field from JSON
fn extract_string_field(json: &str, field: &str) -> Option<String> {
    let pattern = format!("\"{}\"", field);
    
    if let Some(pos) = json.find(&pattern) {
        // Find the colon after the field name
        let after_field = &json[pos + pattern.len()..];
        
        // Skip whitespace and colon
        let mut chars = after_field.chars();
        while let Some(c) = chars.next() {
            if c == ':' {
                break;
            }
        }
        
        // Skip whitespace
        let remaining: String = chars.collect();
        let trimmed = remaining.trim_start();
        
        // Extract the string value
        if trimmed.starts_with('"') {
            return extract_quoted_string(trimmed);
        }
    }
    
    None
}

/// Extract a quoted string from JSON
fn extract_quoted_string(s: &str) -> Option<String> {
    if !s.starts_with('"') {
        return None;
    }
    
    let mut result = String::new();
    let mut chars = s[1..].chars();
    let mut escaped = false;
    
    while let Some(c) = chars.next() {
        if escaped {
            match c {
                'n' => result.push('\n'),
                't' => result.push('\t'),
                'r' => result.push('\r'),
                '\\' => result.push('\\'),
                '"' => result.push('"'),
                _ => result.push(c),
            }
            escaped = false;
        } else if c == '\\' {
            escaped = true;
        } else if c == '"' {
            break;
        } else {
            result.push(c);
        }
    }
    
    Some(result)
}

/// Extract icon path from manifest
/// Handles both "icon": "path" and "icons": [{"src": "path"}]
fn extract_icon_path(json: &str) -> Option<String> {
    // First try simple "icon" field
    if let Some(icon) = extract_string_field(json, "icon") {
        return Some(icon);
    }
    
    // Try "icons" array and get the first icon's src
    if let Some(icons_section) = extract_array_section(json, "icons") {
        // Look for first "src" field in the array
        if let Some(src) = extract_string_field(&icons_section, "src") {
            return Some(src);
        }
    }
    
    None
}

/// Extract an array section from JSON
fn extract_array_section(json: &str, field: &str) -> Option<String> {
    let pattern = format!("\"{}\"", field);
    
    if let Some(pos) = json.find(&pattern) {
        let after_field = &json[pos + pattern.len()..];
        
        // Find opening bracket
        if let Some(bracket_pos) = after_field.find('[') {
            let start = bracket_pos + 1;
            
            // Find closing bracket (accounting for nesting)
            let mut depth = 1;
            let chars: Vec<char> = after_field[start..].chars().collect();
            let mut end = start;
            
            for (i, c) in chars.iter().enumerate() {
                match c {
                    '[' => depth += 1,
                    ']' => {
                        depth -= 1;
                        if depth == 0 {
                            end = start + i;
                            break;
                        }
                    }
                    _ => {}
                }
            }
            
            return Some(after_field[start..end].to_string());
        }
    }
    
    None
}

/// Extract a string array from JSON
fn extract_string_array(json: &str, field: &str) -> Vec<String> {
    let mut result = Vec::new();
    
    if let Some(array_section) = extract_array_section(json, field) {
        // Parse array elements
        let mut in_string = false;
        let mut escaped = false;
        let mut current = String::new();
        
        for c in array_section.chars() {
            if escaped {
                if in_string {
                    match c {
                        'n' => current.push('\n'),
                        't' => current.push('\t'),
                        'r' => current.push('\r'),
                        '\\' => current.push('\\'),
                        '"' => current.push('"'),
                        _ => current.push(c),
                    }
                }
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                if in_string {
                    // End of string - add to result
                    if !current.is_empty() {
                        result.push(current.clone());
                        current.clear();
                    }
                }
                in_string = !in_string;
            } else if in_string {
                current.push(c);
            }
            // Skip commas, whitespace, brackets when not in string
        }
    }
    
    result
}

/// Parse color string to ARGB u32
/// Supports: #RGB, #RGBA, #RRGGBB, #RRGGBBAA
fn parse_color(color_str: &str) -> u32 {
    let s = color_str.trim();
    
    if !s.starts_with('#') {
        // Try named colors
        return parse_named_color(s);
    }
    
    let hex = &s[1..];
    
    match hex.len() {
        3 => {
            // #RGB
            let r = hex_char_to_u8(hex.as_bytes()[0]) * 17;
            let g = hex_char_to_u8(hex.as_bytes()[1]) * 17;
            let b = hex_char_to_u8(hex.as_bytes()[2]) * 17;
            0xFF000000 | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)
        }
        4 => {
            // #RGBA
            let r = hex_char_to_u8(hex.as_bytes()[0]) * 17;
            let g = hex_char_to_u8(hex.as_bytes()[1]) * 17;
            let b = hex_char_to_u8(hex.as_bytes()[2]) * 17;
            let a = hex_char_to_u8(hex.as_bytes()[3]) * 17;
            ((a as u32) << 24) | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)
        }
        6 => {
            // #RRGGBB
            let r = hex_byte(&hex[0..2]);
            let g = hex_byte(&hex[2..4]);
            let b = hex_byte(&hex[4..6]);
            0xFF000000 | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)
        }
        8 => {
            // #RRGGBBAA
            let r = hex_byte(&hex[0..2]);
            let g = hex_byte(&hex[2..4]);
            let b = hex_byte(&hex[4..6]);
            let a = hex_byte(&hex[6..8]);
            ((a as u32) << 24) | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)
        }
        _ => 0xFFFFFFFF, // Default to white
    }
}

/// Parse named CSS colors
fn parse_named_color(name: &str) -> u32 {
    match name.to_lowercase().as_str() {
        "black" => 0xFF000000,
        "white" => 0xFFFFFFFF,
        "red" => 0xFFFF0000,
        "green" => 0xFF00FF00,
        "blue" => 0xFF0000FF,
        "yellow" => 0xFFFFFF00,
        "cyan" => 0xFF00FFFF,
        "magenta" => 0xFFFF00FF,
        "silver" => 0xFFC0C0C0,
        "gray" => 0xFF808080,
        "grey" => 0xFF808080,
        "maroon" => 0xFF800000,
        "olive" => 0xFF808000,
        "lime" => 0xFF00FF00,
        "aqua" => 0xFF00FFFF,
        "teal" => 0xFF008080,
        "navy" => 0xFF000080,
        "fuchsia" => 0xFFFF00FF,
        "purple" => 0xFF800080,
        "orange" => 0xFFFFA500,
        "transparent" => 0x00000000,
        _ => 0xFFFFFFFF, // Default to white
    }
}

/// Convert hex char to u8
fn hex_char_to_u8(c: u8) -> u8 {
    match c {
        b'0'..=b'9' => c - b'0',
        b'A'..=b'F' => c - b'A' + 10,
        b'a'..=b'f' => c - b'a' + 10,
        _ => 0,
    }
}

/// Convert hex byte string to u8
fn hex_byte(s: &str) -> u8 {
    if s.len() != 2 {
        return 0;
    }
    let bytes = s.as_bytes();
    (hex_char_to_u8(bytes[0]) << 4) | hex_char_to_u8(bytes[1])
}

/// Generate a manifest JSON string from a PwaManifest
pub fn generate_json(manifest: &PwaManifest) -> String {
    let mut json = String::from("{\n");
    
    json.push_str(&format!("  \"name\": \"{}\",\n", escape_json(&manifest.name)));
    json.push_str(&format!("  \"short_name\": \"{}\",\n", escape_json(&manifest.short_name)));
    json.push_str(&format!("  \"description\": \"{}\",\n", escape_json(&manifest.description)));
    json.push_str(&format!("  \"start_url\": \"{}\",\n", escape_json(&manifest.start_url)));
    json.push_str(&format!("  \"display\": \"{}\",\n", manifest.display.to_str()));
    json.push_str(&format!("  \"background_color\": \"{}\",\n", color_to_hex(manifest.background_color)));
    json.push_str(&format!("  \"theme_color\": \"{}\",\n", color_to_hex(manifest.theme_color)));
    json.push_str(&format!("  \"version\": \"{}\",\n", escape_json(&manifest.version)));
    
    if let Some(ref author) = manifest.author {
        json.push_str(&format!("  \"author\": \"{}\",\n", escape_json(author)));
    }
    
    if let Some(ref icon) = manifest.icon {
        json.push_str(&format!("  \"icon\": \"{}\"", escape_json(icon)));
    } else {
        json.push_str("  \"icon\": \"icon.png\"");
    }
    
    if !manifest.permissions.is_empty() {
        json.push_str(",\n  \"permissions\": [\n");
        for (i, perm) in manifest.permissions.iter().enumerate() {
            json.push_str(&format!("    \"{}\"", escape_json(perm)));
            if i < manifest.permissions.len() - 1 {
                json.push_str(",");
            }
            json.push('\n');
        }
        json.push_str("  ]");
    }
    
    if manifest.is_system {
        json.push_str(",\n  \"is_system\": true");
    }
    
    json.push('\n');
    json.push('}');
    
    json
}

/// Escape a string for JSON
fn escape_json(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    
    for c in s.chars() {
        match c {
            '"' => result.push_str("\\\""),
            '\\' => result.push_str("\\\\"),
            '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\r"),
            '\t' => result.push_str("\\t"),
            c => result.push(c),
        }
    }
    
    result
}

/// Convert ARGB color to hex string
fn color_to_hex(color: u32) -> String {
    let a = ((color >> 24) & 0xFF) as u8;
    let r = ((color >> 16) & 0xFF) as u8;
    let g = ((color >> 8) & 0xFF) as u8;
    let b = (color & 0xFF) as u8;
    
    if a == 255 {
        format!("#{:02X}{:02X}{:02X}", r, g, b)
    } else {
        format!("#{:02X}{:02X}{:02X}{:02X}", r, g, b, a)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_basic_manifest() {
        let json = r#"{
            "name": "Test App",
            "short_name": "Test",
            "description": "A test app",
            "start_url": "app.html",
            "display": "standalone"
        }"#;
        
        let manifest = parse(json).unwrap();
        assert_eq!(manifest.name, "Test App");
        assert_eq!(manifest.short_name, "Test");
        assert_eq!(manifest.description, "A test app");
        assert_eq!(manifest.start_url, "app.html");
        assert_eq!(manifest.display, DisplayMode::Standalone);
    }
    
    #[test]
    fn test_parse_color() {
        assert_eq!(parse_color("#FFFFFF"), 0xFFFFFFFF);
        assert_eq!(parse_color("#000000"), 0xFF000000);
        assert_eq!(parse_color("#FF0000"), 0xFFFF0000);
        assert_eq!(parse_color("red"), 0xFFFF0000);
        assert_eq!(parse_color("blue"), 0xFF0000FF);
    }
    
    #[test]
    fn test_extract_string_field() {
        let json = r#"{"name": "Test App", "version": "1.0"}"#;
        assert_eq!(extract_string_field(json, "name"), Some("Test App".to_string()));
        assert_eq!(extract_string_field(json, "version"), Some("1.0".to_string()));
        assert_eq!(extract_string_field(json, "missing"), None);
    }
}
