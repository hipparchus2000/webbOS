//! DOM API Implementation
//!
//! Provides DOM operations that can be called from JavaScript.
//! This bridges the JavaScript engine with the HTML document tree.

use alloc::string::String;
use alloc::vec::Vec;
use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use spin::Mutex;
use lazy_static::lazy_static;

use crate::browser::html::{Document, Element, Node};
use crate::browser::BrowserError;
use crate::println;

/// Unique ID for DOM elements
pub type ElementId = u32;

/// Global element registry
lazy_static! {
    static ref ELEMENT_REGISTRY: Mutex<BTreeMap<ElementId, ElementHandle>> = Mutex::new(BTreeMap::new());
    static ref NEXT_ELEMENT_ID: Mutex<ElementId> = Mutex::new(1);
}

/// Handle to a DOM element
pub struct ElementHandle {
    pub id: ElementId,
    pub document_ptr: *mut Document,
    pub element_ptr: *mut Element,
}

// SAFETY: We ensure document/element pointers remain valid during element lifetime
unsafe impl Send for ElementHandle {}
unsafe impl Sync for ElementHandle {}

/// Register an element in the global registry
pub fn register_element(document: *mut Document, element: *mut Element) -> ElementId {
    let mut next_id = NEXT_ELEMENT_ID.lock();
    let id = *next_id;
    *next_id += 1;
    
    let handle = ElementHandle {
        id,
        document_ptr: document,
        element_ptr: element,
    };
    
    ELEMENT_REGISTRY.lock().insert(id, handle);
    id
}

/// Get element handle by ID
pub fn get_element(id: ElementId) -> Option<ElementHandle> {
    ELEMENT_REGISTRY.lock().get(&id).cloned()
}

/// Unregister an element
pub fn unregister_element(id: ElementId) {
    ELEMENT_REGISTRY.lock().remove(&id);
}

/// Clear all registrations (e.g., on page navigation)
pub fn clear_registry() {
    ELEMENT_REGISTRY.lock().clear();
    *NEXT_ELEMENT_ID.lock() = 1;
}

/// DOM API functions exposed to JavaScript
pub mod api {
    use super::*;
    
    /// Get element by ID
    pub fn get_element_by_id(document: *mut crate::browser::html::Document, id: &str) -> Option<ElementId> {
        unsafe {
            let doc = &*document;
            find_element_by_id_in_node(&Node::Element(doc.root.clone()), id, document)
        }
    }
    
    /// Query selector (simplified - supports only tag, class, id)
    pub fn query_selector(document: *mut crate::browser::html::Document, selector: &str) -> Option<ElementId> {
        let selector = selector.trim();
        
        unsafe {
            let doc = &*document;
            
            if selector.starts_with('#') {
                // ID selector
                get_element_by_id(document, &selector[1..])
            } else if selector.starts_with('.') {
                // Class selector
                find_element_by_class_in_node(&Node::Element(doc.root.clone()), &selector[1..], document)
            } else {
                // Tag selector
                find_element_by_tag_in_node(&Node::Element(doc.root.clone()), selector, document)
            }
        }
    }
    
    /// Query selector all
    pub fn query_selector_all(document: *mut crate::browser::html::Document, selector: &str) -> Vec<ElementId> {
        let selector = selector.trim();
        let mut results = Vec::new();
        
        unsafe {
            let doc = &*document;
            
            if selector.starts_with('#') {
                // ID selector - should only return one
                if let Some(id) = get_element_by_id(document, &selector[1..]) {
                    results.push(id);
                }
            } else if selector.starts_with('.') {
                // Class selector
                find_all_elements_by_class_in_node(&Node::Element(doc.root.clone()), &selector[1..], document, &mut results);
            } else {
                // Tag selector
                find_all_elements_by_tag_in_node(&Node::Element(doc.root.clone()), selector, document, &mut results);
            }
        }
        
        results
    }
    
    /// Get innerHTML
    pub fn get_inner_html(element_id: ElementId) -> String {
        let handle = match get_element(element_id) {
            Some(h) => h,
            None => return String::new(),
        };
        
        unsafe {
            let elem = &*handle.element_ptr;
            serialize_element_children(elem)
        }
    }
    
    /// Set innerHTML (simplified - just text content for now)
    pub fn set_inner_html(element_id: ElementId, html: &str) {
        let handle = match get_element(element_id) {
            Some(h) => h,
            None => return,
        };
        
        unsafe {
            let elem = &mut *handle.element_ptr;
            // Clear existing children
            elem.children.clear();
            // Add text node
            elem.children.push(Node::Text(String::from(html)));
        }
    }
    
    /// Get textContent
    pub fn get_text_content(element_id: ElementId) -> String {
        let handle = match get_element(element_id) {
            Some(h) => h,
            None => return String::new(),
        };
        
        unsafe {
            let elem = &*handle.element_ptr;
            extract_text_content(elem)
        }
    }
    
    /// Set textContent
    pub fn set_text_content(element_id: ElementId, text: &str) {
        let handle = match get_element(element_id) {
            Some(h) => h,
            None => return,
        };
        
        unsafe {
            let elem = &mut *handle.element_ptr;
            elem.children.clear();
            elem.children.push(Node::Text(String::from(text)));
        }
    }
    
    /// Get attribute
    pub fn get_attribute(element_id: ElementId, name: &str) -> Option<String> {
        let handle = get_element(element_id)?;
        
        unsafe {
            let elem = &*handle.element_ptr;
            elem.get_attr(name).map(String::from)
        }
    }
    
    /// Set attribute
    pub fn set_attribute(element_id: ElementId, name: &str, value: &str) {
        let handle = match get_element(element_id) {
            Some(h) => h,
            None => return,
        };
        
        unsafe {
            let elem = &mut *handle.element_ptr;
            // Remove existing attribute if present
            elem.attributes.retain(|(k, _)| k != name);
            // Add new attribute
            elem.attributes.push((String::from(name), String::from(value)));
        }
    }
    
    /// Get element id
    pub fn get_id(element_id: ElementId) -> String {
        get_attribute(element_id, "id").unwrap_or_default()
    }
    
    /// Set element id
    pub fn set_id(element_id: ElementId, id: &str) {
        set_attribute(element_id, "id", id);
    }
    
    /// Get className
    pub fn get_class_name(element_id: ElementId) -> String {
        get_attribute(element_id, "class").unwrap_or_default()
    }
    
    /// Set className
    pub fn set_class_name(element_id: ElementId, class_name: &str) {
        set_attribute(element_id, "class", class_name);
    }
    
    /// Get tag name
    pub fn get_tag_name(element_id: ElementId) -> String {
        let handle = match get_element(element_id) {
            Some(h) => h,
            None => return String::new(),
        };
        
        unsafe {
            let elem = &*handle.element_ptr;
            elem.tag.clone()
        }
    }
    
    /// Append child
    pub fn append_child(parent_id: ElementId, child_id: ElementId) {
        let parent_handle = match get_element(parent_id) {
            Some(h) => h,
            None => return,
        };
        
        let child_handle = match get_element(child_id) {
            Some(h) => h,
            None => return,
        };
        
        unsafe {
            let parent = &mut *parent_handle.element_ptr;
            let child = &*child_handle.element_ptr;
            
            // Clone the child element and add it
            // Note: We need to clone children manually since Node doesn't impl Clone
            let cloned_children: Vec<_> = child.children.iter().map(|node| {
                match node {
                    Node::Text(s) => Node::Text(s.clone()),
                    Node::Comment(s) => Node::Comment(s.clone()),
                    Node::Element(e) => Node::Element(Element {
                        tag: e.tag.clone(),
                        attributes: e.attributes.clone(),
                        children: Vec::new(), // Simplified - don't recursively clone
                        computed_styles: e.computed_styles.clone(),
                    }),
                }
            }).collect();
            
            let child_node = Node::Element(Element {
                tag: child.tag.clone(),
                attributes: child.attributes.clone(),
                children: cloned_children,
                computed_styles: child.computed_styles.clone(),
            });
            
            parent.children.push(child_node);
        }
    }
    
    /// Create element
    pub fn create_element(document: *mut crate::browser::html::Document, tag_name: &str) -> ElementId {
        let element = Box::new(Element::new(tag_name));
        let element_ptr = Box::into_raw(element);
        
        register_element(document, element_ptr)
    }
    
    /// Create text node
    pub fn create_text_node(document: *mut crate::browser::html::Document, text: &str) -> ElementId {
        // For simplicity, we treat text nodes as elements with tag "#text"
        let mut element = Element::new("#text");
        element.children.push(Node::Text(String::from(text)));
        
        let element_ptr = Box::into_raw(Box::new(element));
        register_element(document, element_ptr)
    }
}

// Helper functions

unsafe fn find_element_by_id_in_node(node: &Node, id: &str, document: *mut crate::browser::html::Document) -> Option<ElementId> {
    if let Node::Element(elem) = node {
        if let Some(elem_id) = elem.get_attr("id") {
            if elem_id == id {
                // Found it - register and return
                return Some(register_element(document, elem as *const _ as *mut _));
            }
        }
        
        // Search children
        for child in &elem.children {
            if let Some(found) = find_element_by_id_in_node(child, id, document) {
                return Some(found);
            }
        }
    }
    
    None
}

unsafe fn find_element_by_class_in_node(node: &Node, class: &str, document: *mut crate::browser::html::Document) -> Option<ElementId> {
    if let Node::Element(elem) = node {
        if let Some(elem_class) = elem.get_attr("class") {
            if elem_class.split_whitespace().any(|c| c == class) {
                return Some(register_element(document, elem as *const _ as *mut _));
            }
        }
        
        for child in &elem.children {
            if let Some(found) = find_element_by_class_in_node(child, class, document) {
                return Some(found);
            }
        }
    }
    
    None
}

unsafe fn find_all_elements_by_class_in_node(
    node: &Node, 
    class: &str, 
    document: *mut crate::browser::html::Document, 
    results: &mut Vec<ElementId>
) {
    if let Node::Element(elem) = node {
        if let Some(elem_class) = elem.get_attr("class") {
            if elem_class.split_whitespace().any(|c| c == class) {
                results.push(register_element(document, elem as *const _ as *mut _));
            }
        }
        
        for child in &elem.children {
            find_all_elements_by_class_in_node(child, class, document, results);
        }
    }
}

unsafe fn find_element_by_tag_in_node(node: &Node, tag: &str, document: *mut crate::browser::html::Document) -> Option<ElementId> {
    if let Node::Element(elem) = node {
        if elem.tag.eq_ignore_ascii_case(tag) {
            return Some(register_element(document, elem as *const _ as *mut _));
        }
        
        for child in &elem.children {
            if let Some(found) = find_element_by_tag_in_node(child, tag, document) {
                return Some(found);
            }
        }
    }
    
    None
}

unsafe fn find_all_elements_by_tag_in_node(
    node: &Node, 
    tag: &str, 
    document: *mut crate::browser::html::Document, 
    results: &mut Vec<ElementId>
) {
    if let Node::Element(elem) = node {
        if elem.tag.eq_ignore_ascii_case(tag) {
            results.push(register_element(document, elem as *const _ as *mut _));
        }
        
        for child in &elem.children {
            find_all_elements_by_tag_in_node(child, tag, document, results);
        }
    }
}

fn serialize_element_children(elem: &Element) -> String {
    let mut result = String::new();
    
    for child in &elem.children {
        match child {
            Node::Text(text) => result.push_str(text),
            Node::Element(child_elem) => {
                // Simple serialization
                result.push('<');
                result.push_str(&child_elem.tag);
                
                for (name, value) in &child_elem.attributes {
                    result.push(' ');
                    result.push_str(name);
                    result.push_str("=\"");
                    result.push_str(value);
                    result.push('"');
                }
                
                if child_elem.children.is_empty() {
                    result.push_str(" />");
                } else {
                    result.push('>');
                    result.push_str(&serialize_element_children(child_elem));
                    result.push('<');
                    result.push('/');
                    result.push_str(&child_elem.tag);
                    result.push('>');
                }
            }
            Node::Comment(_) => {}
        }
    }
    
    result
}

fn extract_text_content(elem: &Element) -> String {
    let mut result = String::new();
    
    for child in &elem.children {
        match child {
            Node::Text(text) => result.push_str(text),
            Node::Element(child_elem) => result.push_str(&extract_text_content(child_elem)),
            Node::Comment(_) => {}
        }
    }
    
    result
}

impl Clone for ElementHandle {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            document_ptr: self.document_ptr,
            element_ptr: self.element_ptr,
        }
    }
}

/// Initialize DOM API
pub fn init() {
    println!("[dom_api] DOM API initialized");
}
