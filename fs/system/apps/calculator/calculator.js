let display = '0';
let firstOperand = null;
let operator = null;
let waitingForSecondOperand = false;
let shouldResetDisplay = false;

const displayElement = document.getElementById('display');

function updateDisplay() {
    // Format display for readability (add commas for thousands)
    if (!display.includes('e') && !isNaN(parseFloat(display))) {
        const parts = display.split('.');
        parts[0] = parts[0].replace(/\B(?=(\d{3})+(?!\d))/g, ',');
        displayElement.textContent = parts.join('.');
    } else {
        displayElement.textContent = display;
    }
    
    // Adjust font size for long numbers
    if (display.length > 10) {
        displayElement.style.fontSize = '32px';
    } else if (display.length > 8) {
        displayElement.style.fontSize = '40px';
    } else {
        displayElement.style.fontSize = '48px';
    }
}

function inputDigit(digit) {
    if (shouldResetDisplay) {
        display = digit;
        shouldResetDisplay = false;
    } else if (display === '0') {
        display = digit;
    } else if (display.length < 15) {
        display += digit;
    }
    waitingForSecondOperand = false;
    updateDisplay();
}

function inputDecimal() {
    if (shouldResetDisplay) {
        display = '0.';
        shouldResetDisplay = false;
    } else if (!display.includes('.')) {
        if (display.length < 14) {
            display += '.';
        }
    }
    updateDisplay();
}

function clearAll() {
    display = '0';
    firstOperand = null;
    operator = null;
    waitingForSecondOperand = false;
    shouldResetDisplay = false;
    updateDisplay();
}

function clearEntry() {
    display = '0';
    updateDisplay();
}

function toggleSign() {
    const value = parseFloat(display);
    if (value !== 0) {
        display = (value * -1).toString();
    }
    updateDisplay();
}

function percentage() {
    const value = parseFloat(display);
    display = (value / 100).toString();
    updateDisplay();
}

function setOperator(op) {
    const inputValue = parseFloat(display);
    
    if (operator && waitingForSecondOperand) {
        operator = op;
        return;
    }
    
    if (firstOperand === null) {
        firstOperand = inputValue;
    } else if (operator) {
        const result = performCalculation(operator, firstOperand, inputValue);
        display = String(result);
        firstOperand = result;
        updateDisplay();
    }
    
    operator = op;
    waitingForSecondOperand = true;
    shouldResetDisplay = true;
}

function performCalculation(op, a, b) {
    switch (op) {
        case '+':
            return a + b;
        case '-':
            return a - b;
        case '*':
            return a * b;
        case '/':
            if (b === 0) {
                return 'Error';
            }
            return a / b;
        default:
            return b;
    }
}

function calculate() {
    if (!operator || firstOperand === null) {
        return;
    }
    
    const inputValue = parseFloat(display);
    const result = performCalculation(operator, firstOperand, inputValue);
    
    // Handle result formatting
    if (result === 'Error') {
        display = 'Error';
    } else {
        // Limit decimal places for clean display
        const resultStr = String(result);
        if (resultStr.length > 15) {
            display = result.toExponential(9);
        } else {
            display = resultStr;
        }
    }
    
    firstOperand = null;
    operator = null;
    waitingForSecondOperand = false;
    shouldResetDisplay = true;
    updateDisplay();
}

// Keyboard support
document.addEventListener('keydown', (event) => {
    const key = event.key;
    
    if (key >= '0' && key <= '9') {
        inputDigit(key);
    } else if (key === '.') {
        inputDecimal();
    } else if (key === '+' || key === '-' || key === '*' || key === '/') {
        setOperator(key);
    } else if (key === 'Enter' || key === '=') {
        event.preventDefault();
        calculate();
    } else if (key === 'Escape' || key === 'c' || key === 'C') {
        clearAll();
    } else if (key === 'Backspace') {
        if (display.length > 1 && !shouldResetDisplay) {
            display = display.slice(0, -1);
        } else {
            display = '0';
        }
        updateDisplay();
    } else if (key === '%') {
        percentage();
    }
});

// Prevent zoom on double tap for mobile
document.addEventListener('touchstart', (event) => {
    if (event.touches.length > 1) {
        event.preventDefault();
    }
}, { passive: false });

// Service Worker Registration
if ('serviceWorker' in navigator) {
    window.addEventListener('load', () => {
        navigator.serviceWorker.register('./sw.js')
            .then((registration) => {
                console.log('Calculator SW registered:', registration);
            })
            .catch((error) => {
                console.log('Calculator SW registration failed:', error);
            });
    });
}
