# UX Tasks: Mahjong.html → xmahjongg.png Appearance Matching

## Critical Visual Differences

### 1. Header/Score Display
- [ ] **Score Numbers Position & Style**
  - Current: "1 4 2" in top-left, large colored numbers
  - Target: Score circles at top center with yellow coin-like appearance showing game count
  - Action: Replace "1 4 2" with row of circular yellow coin indicators at top-center

### 2. Control Buttons (Top Right)
- [ ] **Button Iconography**
  - Current: Simple symbols (↶, ?, 🔄) on beige rounded rectangles
  - Target: Detailed icon buttons with visual imagery (lightbulb, clock, undo arrow, stop sign, skull)
  - Action: Replace text/emoji buttons with proper icon graphics matching xmahjongg style

- [ ] **Button Count & Functions**
  - Current: 3 buttons (Undo, Hint, New Game)
  - Target: 5 icon buttons visible (lightbulb/hint, clock, undo, stop/quit, skull/reset)
  - Action: Add missing buttons and implement proper icon rendering

### 3. Tile Visual Design
- [ ] **Tile Shading & Borders**
  - Current: Flat gradient with thick black borders
  - Target: Beveled 3D appearance with lighter top-left edges, darker bottom-right
  - Action: Implement proper 3D bevel effect using CSS gradients and border styling

- [ ] **Tile Shadow**
  - Current: Subtle drop shadow
  - Target: Prominent dark shadow on bottom and right edges creating strong depth
  - Action: Increase shadow intensity and offset for xmahjongg-style depth

- [ ] **Tile Size & Proportions**
  - Current: 44x58px tiles
  - Target: Tiles appear slightly larger with more pronounced vertical orientation
  - Action: Adjust tile dimensions to match xmahjongg proportions (estimate: 52x68px)

### 4. Tile Symbols & Typography
- [ ] **Symbol Size**
  - Current: Font-size 1.8rem (smaller symbols)
  - Target: Larger, bolder symbols filling more of tile face
  - Action: Increase font-size to approximately 2.4-2.8rem

- [ ] **Symbol Rendering**
  - Current: Unicode Mahjong glyphs (clean, thin lines)
  - Target: More ornate, traditional appearance with richer detail
  - Note: May be limited by Unicode font rendering; consider if custom SVG tiles needed

### 5. Board Background & Frame
- [ ] **Board Surface**
  - Current: Olive-green gradient with rounded corners
  - Target: Darker, more muted green with subtle texture
  - Action: Adjust background color to darker olive (#6b7447 range) and add texture overlay

- [ ] **Board Border/Frame**
  - Current: Soft rounded rectangle with box-shadow
  - Target: More defined border with stronger edge definition
  - Action: Add visible border stroke and adjust shadow for harder edge

### 6. Body Background Pattern
- [ ] **Background Texture**
  - Current: Diagonal crosshatch pattern (khaki/yellow)
  - Target: More organic, mottled texture with green/yellow variations
  - Action: Replace gradient pattern with more organic repeating texture or noise pattern

- [ ] **Background Color Palette**
  - Current: Yellow-brown tones (#9b8b4f, #a09557)
  - Target: More greenish-yellow with organic variation
  - Action: Adjust to warmer green-yellow (#a0975f, #8a8550) with texture overlay

### 7. Status Bar (Bottom Left)
- [ ] **Text Styling**
  - Current: White text with simple display
  - Target: Same positioning, verify if font/size matches
  - Action: Review and adjust font family and size if needed

- [ ] **Checkbox Styling**
  - Current: Standard checkbox with label
  - Target: Checkbox appears integrated with status text
  - Action: Style checkbox to match xmahjongg appearance

### 8. Tile Layer Visualization
- [ ] **3D Stacking Effect**
  - Current: Minimal offset between layers
  - Target: More pronounced shadow and offset creating stronger 3D pyramid effect
  - Action: Increase z-offset and shadow for each layer level

- [ ] **Top Tile Highlight**
  - Current: Selected tiles have golden background
  - Target: Verify selection state appearance matches
  - Action: Ensure selection highlighting matches xmahjongg visual feedback

### 9. Tile Content Variations
- [ ] **Season/Flower Tiles**
  - Current: Unicode glyphs showing seasons/flowers
  - Target: Tiles show text labels like "AUTUMN", "SPRING", "CHRYSAN" with decorative images
  - Action: Replace Unicode season/flower symbols with text labels + images matching xmahjongg

### 10. Overall Layout & Spacing
- [ ] **Board Centering**
  - Current: Board centered with equal margins
  - Target: Appears properly centered with balanced spacing
  - Status: Verify alignment matches

- [ ] **Element Positioning**
  - Current: Absolute positioning for controls, status, header
  - Target: Confirm all elements positioned identically
  - Action: Fine-tune positioning to pixel-perfect match

## Priority Order
1. **High**: Tile appearance (borders, shadows, 3D effect, size)
2. **High**: Button icons and count
3. **High**: Season/Flower tile labels
4. **Medium**: Score display style
5. **Medium**: Background texture
6. **Medium**: Board surface color/texture
7. **Low**: Fine-tuning spacing and alignment

## Testing Checklist
- [ ] Side-by-side visual comparison at 100% zoom
- [ ] Verify tile click/selection states match target
- [ ] Confirm all buttons functional with proper icons
- [ ] Validate color accuracy using color picker
- [ ] Check tile shadow renders consistently across layers
- [ ] Verify text remains readable at all zoom levels
