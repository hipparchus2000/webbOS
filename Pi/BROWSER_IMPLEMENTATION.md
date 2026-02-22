# WebbOS Browser Engine Implementation Plan

## Overview
Building a complete HTML5/CSS/JS browser engine from scratch for WebbOS.

## Phase 1: Core DOM API & Event Handling (Critical - Apps Need This)

### 1.1 DOM Core API
- [ ] `document` global object
- [ ] `document.getElementById(id)`
- [ ] `document.querySelector(selector)`
- [ ] `document.querySelectorAll(selector)`
- [ ] `document.createElement(tagName)`
- [ ] `document.createTextNode(text)`
- [ ] `element.innerHTML` (getter/setter)
- [ ] `element.textContent`
- [ ] `element.id`
- [ ] `element.className` / `element.classList`
- [ ] `element.style` (CSSStyleDeclaration)
- [ ] `element.appendChild(node)`
- [ ] `element.removeChild(node)`
- [ ] `element.insertBefore(newNode, referenceNode)`
- [ ] `element.setAttribute(name, value)`
- [ ] `element.getAttribute(name)`
- [ ] `element.addEventListener(type, listener)`
- [ ] `element.removeEventListener(type, listener)`
- [ ] `element.children` / `element.childNodes`
- [ ] `element.parentNode`
- [ ] `element.tagName`
- [ ] Node types: ELEMENT_NODE, TEXT_NODE

### 1.2 Event System
- [ ] Event object with `target`, `type`, `preventDefault()`, `stopPropagation()`
- [ ] Event bubbling/capturing
- [ ] Mouse events: click, dblclick, mousedown, mouseup, mousemove, mouseenter, mouseleave
- [ ] Keyboard events: keydown, keyup, keypress
- [ ] Form events: submit, change, input, focus, blur
- [ ] Window events: load, resize, scroll
- [ ] Event dispatch mechanism from kernel to JS

### 1.3 Window Object
- [ ] `window` global object
- [ ] `window.document`
- [ ] `window.location` (href, pathname, reload)
- [ ] `window.innerWidth` / `window.innerHeight`
- [ ] `window.alert()` / `window.confirm()` / `window.prompt()` (stubs)
- [ ] `window.postMessage()` (for iframe communication)

## Phase 2: Network APIs

### 2.1 Fetch API
- [ ] `fetch(url, options)`
- [ ] Response object with `text()`, `json()`, `ok`, `status`
- [ ] Request headers
- [ ] POST/PUT/DELETE methods
- [ ] Basic CORS handling

### 2.2 XMLHttpRequest (Legacy)
- [ ] `new XMLHttpRequest()`
- [ ] `open(method, url)`
- [ ] `send(body)`
- [ ] `onload`, `onerror`, `onprogress` callbacks
- [ ] `responseText`, `status`

## Phase 3: Storage APIs

### 3.1 Local Storage
- [ ] `localStorage.getItem(key)`
- [ ] `localStorage.setItem(key, value)`
- [ ] `localStorage.removeItem(key)`
- [ ] `localStorage.clear()`
- [ ] `localStorage.length` / `key(index)`
- [ ] Persistence to disk (FAT32)

### 3.2 Session Storage
- [ ] Same API as localStorage but memory-only

### 3.3 Cookies (Basic)
- [ ] `document.cookie` (getter/setter)
- [ ] Basic cookie parsing

## Phase 4: Advanced CSS

### 4.1 Layout
- [ ] Flexbox (full implementation)
- [ ] CSS Grid
- [ ] Proper float support
- [ ] Position: sticky

### 4.2 Animations
- [ ] CSS transitions
- [ ] CSS @keyframes animations
- [ ] `requestAnimationFrame()`

### 4.3 Media Queries
- [ ] `window.matchMedia()`
- [ ] Responsive layout engine

### 4.4 Advanced Selectors
- [ ] Pseudo-classes: :hover, :focus, :active, :checked, :nth-child, etc.
- [ ] Pseudo-elements: ::before, ::after

## Phase 5: ES6+ JavaScript

### 5.1 Classes
- [ ] `class` syntax
- [ ] `extends` / `super`
- [ ] Getters/setters
- [ ] Static methods

### 5.2 Modern Syntax
- [ ] Arrow functions
- [ ] Template literals
- [ ] Destructuring
- [ ] Spread/rest operators
- [ ] Default parameters
- [ ] for-of loops

### 5.3 Modules
- [ ] `import` / `export`
- [ ] Dynamic `import()`

### 5.4 Async
- [ ] Promises
- [ ] async/await

### 5.5 Built-in Objects
- [ ] Map, Set, WeakMap, WeakSet
- [ ] Array methods: map, filter, reduce, forEach, etc.
- [ ] String methods: startsWith, endsWith, includes, template handling
- [ ] Object methods: assign, keys, values, entries
- [ ] JSON.parse / JSON.stringify

## Phase 6: HTML5 Features

### 6.1 Forms
- [ ] All input types: text, password, email, number, checkbox, radio, file, etc.
- [ ] Form validation API
- [ ] FormData object

### 6.2 Media
- [ ] `<canvas>` 2D context
- [ ] `<video>` / `<audio>` (basic)
- [ ] `<img>` proper loading

### 6.3 Other Elements
- [ ] `<iframe>` (sandboxed)
- [ ] `<svg>` (basic)
- [ ] Semantic elements: header, nav, main, article, section, footer, aside

## Implementation Strategy

Each phase builds on the previous. The JS engine needs to expose native functions to JavaScript through bindings.

### DOM-JS Bridge Architecture
```
┌─────────────────┐
│   JavaScript    │
│   Engine        │
└────────┬────────┘
         │
    ┌────┴────┐
    │  Bindings│  (rust code exposing DOM APIs)
    └────┬────┘
         │
┌────────┴────────┐
│   DOM Tree      │  (Rust structs)
│   (html.rs)     │
└────────┬────────┘
         │
┌────────┴────────┐
│   Layout Engine │
│   (layout.rs)   │
└────────┬────────┘
         │
┌────────┴────────┐
│   Renderer      │
│   (render.rs)   │
└─────────────────┘
```

### Key Files to Modify
- `kernel/src/browser/js.rs` - Add DOM bindings
- `kernel/src/browser/dom_api.rs` - New file for DOM API implementation
- `kernel/src/browser/event.rs` - New file for event system
- `kernel/src/browser/mod.rs` - Integrate DOM with browser
- `kernel/src/browser/window.rs` - New file for window object
