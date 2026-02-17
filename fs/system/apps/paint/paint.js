const canvas = document.getElementById('canvas');
const ctx = canvas.getContext('2d');
const statusText = document.getElementById('statusText');
const coordsDisplay = document.getElementById('coords');
const colorPicker = document.getElementById('colorPicker');
const sizeSlider = document.getElementById('brushSize');
const sizeValue = document.getElementById('sizeValue');

let isDrawing = false;
let currentTool = 'brush';
let currentColor = '#000000';
let brushSize = 5;
let startX, startY;
let snapshot;

// Undo/Redo history
let history = [];
let historyStep = -1;
const MAX_HISTORY = 50;

// Initialize
function init() {
    // Set white background
    ctx.fillStyle = '#ffffff';
    ctx.fillRect(0, 0, canvas.width, canvas.height);
    
    // Save initial state
    saveState();
    
    // Setup canvas size based on container
    resizeCanvas();
    window.addEventListener('resize', resizeCanvas);
    
    // Setup event listeners
    setupEventListeners();
    
    // Load saved canvas if exists
    loadCanvas();
}

function resizeCanvas() {
    const wrapper = document.getElementById('canvasWrapper');
    const maxWidth = wrapper.clientWidth - 40;
    const maxHeight = wrapper.clientHeight - 40;
    
    // Keep aspect ratio or fit to container
    if (canvas.width > maxWidth || canvas.height > maxHeight) {
        const scale = Math.min(maxWidth / canvas.width, maxHeight / canvas.height);
        canvas.style.width = (canvas.width * scale) + 'px';
        canvas.style.height = (canvas.height * scale) + 'px';
    }
}

function setupEventListeners() {
    // Mouse events
    canvas.addEventListener('mousedown', startDrawing);
    canvas.addEventListener('mousemove', draw);
    canvas.addEventListener('mouseup', stopDrawing);
    canvas.addEventListener('mouseout', stopDrawing);
    
    // Touch events for mobile
    canvas.addEventListener('touchstart', handleTouch);
    canvas.addEventListener('touchmove', handleTouch);
    canvas.addEventListener('touchend', stopDrawing);
    
    // Prevent scrolling while drawing
    canvas.addEventListener('touchstart', (e) => e.preventDefault(), { passive: false });
    canvas.addEventListener('touchmove', (e) => e.preventDefault(), { passive: false });
}

function getCoordinates(e) {
    const rect = canvas.getBoundingClientRect();
    const scaleX = canvas.width / rect.width;
    const scaleY = canvas.height / rect.height;
    
    let clientX, clientY;
    
    if (e.touches && e.touches.length > 0) {
        clientX = e.touches[0].clientX;
        clientY = e.touches[0].clientY;
    } else {
        clientX = e.clientX;
        clientY = e.clientY;
    }
    
    return {
        x: (clientX - rect.left) * scaleX,
        y: (clientY - rect.top) * scaleY
    };
}

function handleTouch(e) {
    e.preventDefault();
    const touch = e.touches[0];
    const mouseEvent = new MouseEvent(e.type === 'touchstart' ? 'mousedown' : 'mousemove', {
        clientX: touch.clientX,
        clientY: touch.clientY
    });
    canvas.dispatchEvent(mouseEvent);
}

function startDrawing(e) {
    isDrawing = true;
    const coords = getCoordinates(e);
    startX = coords.x;
    startY = coords.y;
    
    // Save snapshot for shapes
    snapshot = ctx.getImageData(0, 0, canvas.width, canvas.height);
    
    ctx.beginPath();
    ctx.moveTo(startX, startY);
    
    if (currentTool === 'brush' || currentTool === 'eraser') {
        ctx.lineTo(startX, startY);
        ctx.stroke();
    }
    
    updateStatus();
}

function draw(e) {
    const coords = getCoordinates(e);
    coordsDisplay.textContent = `X: ${Math.round(coords.x)}, Y: ${Math.round(coords.y)}`;
    
    if (!isDrawing) return;
    
    ctx.lineWidth = brushSize;
    ctx.lineCap = 'round';
    ctx.lineJoin = 'round';
    
    if (currentTool === 'eraser') {
        ctx.globalCompositeOperation = 'destination-out';
        ctx.strokeStyle = 'rgba(0,0,0,1)';
    } else {
        ctx.globalCompositeOperation = 'source-over';
        ctx.strokeStyle = currentColor;
    }
    
    switch (currentTool) {
        case 'brush':
        case 'eraser':
            ctx.lineTo(coords.x, coords.y);
            ctx.stroke();
            break;
            
        case 'line':
            ctx.putImageData(snapshot, 0, 0);
            ctx.beginPath();
            ctx.moveTo(startX, startY);
            ctx.lineTo(coords.x, coords.y);
            ctx.stroke();
            break;
            
        case 'rect':
            ctx.putImageData(snapshot, 0, 0);
            ctx.beginPath();
            ctx.rect(startX, startY, coords.x - startX, coords.y - startY);
            ctx.stroke();
            break;
            
        case 'circle':
            ctx.putImageData(snapshot, 0, 0);
            ctx.beginPath();
            const radius = Math.sqrt(
                Math.pow(coords.x - startX, 2) + Math.pow(coords.y - startY, 2)
            );
            ctx.arc(startX, startY, radius, 0, 2 * Math.PI);
            ctx.stroke();
            break;
    }
}

function stopDrawing() {
    if (isDrawing) {
        isDrawing = false;
        ctx.beginPath();
        saveState();
        autoSave();
    }
}

function setTool(tool) {
    currentTool = tool;
    
    // Update button states
    document.querySelectorAll('.toolbar-btn').forEach(btn => {
        btn.classList.remove('active');
    });
    document.getElementById(`btn-${tool}`).classList.add('active');
    
    updateStatus();
}

function setColor(color) {
    currentColor = color;
    colorPicker.value = color;
    
    // Update preset color selection
    document.querySelectorAll('.preset-color').forEach(preset => {
        preset.classList.toggle('active', preset.style.backgroundColor === color || 
            rgbToHex(preset.style.backgroundColor) === color);
    });
}

function rgbToHex(rgb) {
    if (!rgb || rgb.indexOf('rgb') !== 0) return rgb;
    const values = rgb.match(/\d+/g);
    if (!values) return rgb;
    return '#' + values.map(x => {
        const hex = parseInt(x).toString(16);
        return hex.length === 1 ? '0' + hex : hex;
    }).join('');
}

function setBrushSize(size) {
    brushSize = parseInt(size);
    sizeValue.textContent = size + 'px';
}

function clearCanvas() {
    if (confirm('Are you sure you want to clear the canvas?')) {
        ctx.fillStyle = '#ffffff';
        ctx.fillRect(0, 0, canvas.width, canvas.height);
        saveState();
        autoSave();
        statusText.textContent = 'Canvas cleared';
    }
}

function saveImage() {
    const link = document.createElement('a');
    link.download = 'painting_' + new Date().getTime() + '.png';
    link.href = canvas.toDataURL();
    link.click();
    statusText.textContent = 'Image saved';
}

// Undo/Redo functionality
function saveState() {
    historyStep++;
    
    // Remove future history if we're not at the end
    if (historyStep < history.length) {
        history.length = historyStep;
    }
    
    // Add new state
    history.push(canvas.toDataURL());
    
    // Limit history size
    if (history.length > MAX_HISTORY) {
        history.shift();
        historyStep--;
    }
}

function undo() {
    if (historyStep > 0) {
        historyStep--;
        restoreState();
        statusText.textContent = 'Undo';
    }
}

function redo() {
    if (historyStep < history.length - 1) {
        historyStep++;
        restoreState();
        statusText.textContent = 'Redo';
    }
}

function restoreState() {
    const img = new Image();
    img.src = history[historyStep];
    img.onload = () => {
        ctx.clearRect(0, 0, canvas.width, canvas.height);
        ctx.drawImage(img, 0, 0);
    };
}

function updateStatus() {
    const toolNames = {
        'brush': 'Brush',
        'eraser': 'Eraser',
        'line': 'Line Tool',
        'rect': 'Rectangle Tool',
        'circle': 'Circle Tool'
    };
    statusText.textContent = `${toolNames[currentTool]} | Size: ${brushSize}px`;
}

// Auto-save to localStorage
function autoSave() {
    try {
        localStorage.setItem('paint_canvas_backup', canvas.toDataURL());
        localStorage.setItem('paint_canvas_timestamp', new Date().toISOString());
    } catch (e) {
        console.log('Could not auto-save canvas');
    }
}

function loadCanvas() {
    const saved = localStorage.getItem('paint_canvas_backup');
    if (saved) {
        const img = new Image();
        img.src = saved;
        img.onload = () => {
            ctx.drawImage(img, 0, 0);
        };
    }
}

// Keyboard shortcuts
document.addEventListener('keydown', (e) => {
    if (e.ctrlKey || e.metaKey) {
        switch (e.key) {
            case 'z':
                e.preventDefault();
                if (e.shiftKey) {
                    redo();
                } else {
                    undo();
                }
                break;
            case 'y':
                e.preventDefault();
                redo();
                break;
            case 's':
                e.preventDefault();
                saveImage();
                break;
        }
    }
});

// Initialize
init();

// Service Worker Registration
if ('serviceWorker' in navigator) {
    window.addEventListener('load', () => {
        navigator.serviceWorker.register('./sw.js')
            .then((registration) => {
                console.log('Paint SW registered:', registration);
            })
            .catch((error) => {
                console.log('Paint SW registration failed:', error);
            });
    });
}
