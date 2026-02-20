# Sheet.html Test Suite

## Test Categories

### 1. Basic Cell Operations
- [ ] Test 1.1: Select a single cell (A1)
- [ ] Test 1.2: Edit a cell and enter text value
- [ ] Test 1.3: Edit a cell and enter numeric value
- [ ] Test 1.4: Edit a cell and enter formula (=A1+1)
- [ ] Test 1.5: Navigate between cells using arrow keys
- [ ] Test 1.6: Delete cell content using Delete key
- [ ] Test 1.7: Delete cell content using Backspace key
- [ ] Test 1.8: Formula bar updates when cell is selected
- [ ] Test 1.9: Cell reference displays correctly in formula bar

### 2. Range Selection
- [ ] Test 2.1: Select range using Shift+Click
- [ ] Test 2.2: Range highlighting appears correctly
- [ ] Test 2.3: Range reference displays in formula bar (e.g., A1:B5)
- [ ] Test 2.4: Clear range selection by clicking single cell
- [ ] Test 2.5: Select entire row
- [ ] Test 2.6: Select entire column

### 3. Formulas and Calculations
- [ ] Test 3.1: Simple addition formula (=A1+B1)
- [ ] Test 3.2: Simple subtraction formula (=A1-B1)
- [ ] Test 3.3: Simple multiplication formula (=A1*B1)
- [ ] Test 3.4: Simple division formula (=A1/B1)
- [ ] Test 3.5: SUM function with range (=SUM(A1:A5))
- [ ] Test 3.6: AVG function with range (=AVG(A1:A5))
- [ ] Test 3.7: MIN function with range (=MIN(A1:A5))
- [ ] Test 3.8: MAX function with range (=MAX(A1:A5))
- [ ] Test 3.9: Formula with cell references updates when referenced cells change
- [ ] Test 3.10: Error handling for invalid formulas

### 4. Copy and Paste
- [ ] Test 4.1: Copy single cell
- [ ] Test 4.2: Paste single cell
- [ ] Test 4.3: Copy range of cells
- [ ] Test 4.4: Paste range of cells
- [ ] Test 4.5: Paste preserves cell formatting
- [ ] Test 4.6: Paste overwrites existing cells correctly

### 4a. Right-Click Context Menu
- [ ] Test 4a.1: Right-click on cell shows context menu
- [ ] Test 4a.2: Context menu appears at cursor position
- [ ] Test 4a.3: Context menu Copy option works
- [ ] Test 4a.4: Context menu Paste option works when clipboard has data
- [ ] Test 4a.5: Context menu Paste option is disabled when clipboard is empty
- [ ] Test 4a.6: Context menu Delete option works for single cell
- [ ] Test 4a.7: Context menu Delete option works for range
- [ ] Test 4a.8: Context menu closes after selecting an option
- [ ] Test 4a.9: Context menu closes when clicking outside
- [ ] Test 4a.10: Context menu stays within viewport boundaries
- [ ] Test 4a.11: Right-click selects the cell before showing menu
- [ ] Test 4a.12: Context menu works with range selection

### 5. Cell Formatting
- [ ] Test 5.1: Change font family for single cell
- [ ] Test 5.2: Change font size for single cell
- [ ] Test 5.3: Apply bold formatting to single cell
- [ ] Test 5.4: Apply italic formatting to single cell
- [ ] Test 5.5: Apply underline formatting to single cell
- [ ] Test 5.6: Change text color for single cell
- [ ] Test 5.7: Change background color for single cell
- [ ] Test 5.8: Toggle formatting off (bold, italic, underline) for single cell

### 5a. Range Formatting
- [ ] Test 5a.1: Change font family for selected range
- [ ] Test 5a.2: Change font size for selected range
- [ ] Test 5a.3: Apply bold formatting to selected range
- [ ] Test 5a.4: Apply italic formatting to selected range
- [ ] Test 5a.5: Apply underline formatting to selected range
- [ ] Test 5a.6: Change text color for selected range
- [ ] Test 5a.7: Change background color for selected range
- [ ] Test 5a.8: Toggle bold formatting off for entire range
- [ ] Test 5a.9: Toggle italic formatting off for entire range
- [ ] Test 5a.10: Toggle underline formatting off for entire range
- [ ] Test 5a.11: Format large range (e.g., A1:Z50)
- [ ] Test 5a.12: Format rectangular range (e.g., B2:D5)
- [ ] Test 5a.13: Format single row range (e.g., A1:Z1)
- [ ] Test 5a.14: Format single column range (e.g., A1:A50)
- [ ] Test 5a.15: Multiple formatting properties applied to range
- [ ] Test 5a.16: Formatting persists after range selection cleared

### 6. Pie Chart Functionality
- [ ] Test 6.1: Create pie chart with 2 columns (labels + values)
- [ ] Test 6.2: Create pie chart with single column (values only)
- [ ] Test 6.3: Auto-fill range when cells are selected
- [ ] Test 6.4: Pie chart displays with correct slices
- [ ] Test 6.5: Pie chart shows percentage labels on slices
- [ ] Test 6.6: Pie chart legend displays correctly
- [ ] Test 6.7: Pie chart title displays when provided
- [ ] Test 6.8: Pie chart works without title (no title shown)
- [ ] Test 6.9: Error handling for empty range
- [ ] Test 6.10: Error handling for invalid range format
- [ ] Test 6.11: Pie chart handles zero values correctly
- [ ] Test 6.12: Pie chart handles negative values correctly

### 7. Other Chart Types
- [ ] Test 7.1: Create bar chart
- [ ] Test 7.2: Create line chart
- [ ] Test 7.3: Charts display with correct data
- [ ] Test 7.4: Chart titles work for all chart types

### 8. Save and Load
- [ ] Test 8.1: Save spreadsheet to JSON file with custom filename
- [ ] Test 8.2: Save spreadsheet prompts for filename
- [ ] Test 8.3: Save includes charts in saved data
- [ ] Test 8.4: Load spreadsheet from JSON file
- [ ] Test 8.5: Loaded spreadsheet preserves cell values
- [ ] Test 8.6: Loaded spreadsheet preserves formulas
- [ ] Test 8.7: Loaded spreadsheet preserves formatting
- [ ] Test 8.8: Loaded spreadsheet recalculates formulas
- [ ] Test 8.9: Loaded spreadsheet restores charts
- [ ] Test 8.10: Loaded spreadsheet recreates chart titles
- [ ] Test 8.11: Save/load works with multiple charts
- [ ] Test 8.12: Legacy file format (cells only) still loads correctly

### 9. Keyboard Navigation
- [ ] Test 9.1: Arrow keys navigate between cells
- [ ] Test 9.2: Shift+ArrowUp extends selection upward
- [ ] Test 9.3: Shift+ArrowDown extends selection downward
- [ ] Test 9.4: Shift+ArrowLeft extends selection leftward
- [ ] Test 9.5: Shift+ArrowRight extends selection rightward
- [ ] Test 9.6: Shift+Arrow keys create range selection
- [ ] Test 9.7: Range selection shows correct range in formula bar
- [ ] Test 9.8: Enter key starts editing cell
- [ ] Test 9.9: Tab key moves to next cell
- [ ] Test 9.10: Escape cancels cell editing
- [ ] Test 9.11: Typing starts editing cell
- [ ] Test 9.12: Shift+Arrow maintains selectionStart point

### 10. Mouse Selection
- [ ] Test 10.1: Click single cell selects it
- [ ] Test 10.2: Shift+Click extends selection from start point
- [ ] Test 10.3: Mouse drag selects range of cells
- [ ] Test 10.4: Mouse drag from A1 to C3 selects A1:C3
- [ ] Test 10.5: Mouse drag works in all directions
- [ ] Test 10.6: Mouse drag selection shows correct range
- [ ] Test 10.7: Mouse drag doesn't select text
- [ ] Test 10.8: Mouse drag from any cell works correctly
- [ ] Test 10.9: Mouse drag maintains selectionStart point
- [ ] Test 10.10: Range selection persists after mouseup (doesn't get deselected by click event)
- [ ] Test 10.11: Click after drag doesn't clear the range selection
- [ ] Test 10.12: Single click (no drag) still works correctly

### 11. Edge Cases
- [ ] Test 11.1: Handle empty cells in formulas
- [ ] Test 11.2: Handle division by zero
- [ ] Test 11.3: Handle very large numbers
- [ ] Test 11.4: Handle very small numbers
- [ ] Test 11.5: Handle text in numeric formulas
- [ ] Test 11.6: Select cells at boundaries (A1, Z50)
- [ ] Test 11.7: Create chart with single data point
- [ ] Test 11.8: Create chart with many data points
- [ ] Test 11.9: Save with no charts works correctly
- [ ] Test 11.10: Load file with no charts works correctly

## Test Execution Results

Run the test suite using `sheet.test.html` in a browser.

## Bugs Found and Fixed

### 1. Range Bounds Validation (Fixed)
- **Issue**: When a single cell reference was provided for chart data, the code would create an end position that could exceed spreadsheet bounds (ROWS=50, COLS=26)
- **Fix**: Added bounds checking using `Math.min()` to ensure endPos stays within valid range
- **Location**: `getChartData()` function

### 2. Missing Bounds Validation in Loops (Fixed)
- **Issue**: Data collection loops didn't check if row/col indices were within bounds before accessing cells
- **Fix**: Added bounds checks in loops: `r < ROWS` and `col < COLS`
- **Location**: `getChartData()` function loops

### 3. Division by Zero in Line Chart (Fixed)
- **Issue**: When chart had only 1 data point, `pointSpacing` calculation would divide by zero: `(width - padding * 2) / (data.values.length - 1)`
- **Fix**: Added check for single data point case and handle it separately
- **Location**: `drawLineChart()` function

### 4. Empty Data Handling in Charts (Fixed)
- **Issue**: Bar and line charts didn't handle empty data arrays or all-zero values gracefully
- **Fix**: Added checks for empty arrays and zero/negative max values with appropriate error messages
- **Location**: `drawBarChart()` and `drawLineChart()` functions

### 5. Cell Reference Parsing Validation (Fixed)
- **Issue**: `parseCellRef()` didn't validate that parsed references were within spreadsheet bounds
- **Fix**: Added bounds validation and support check for multi-letter columns (currently only A-Z supported)
- **Location**: `parseCellRef()` function

### 6. Range Validation (Fixed)
- **Issue**: No validation that parsed ranges were within spreadsheet bounds before using them
- **Fix**: Added comprehensive bounds checking after parsing range
- **Location**: `getChartData()` function

