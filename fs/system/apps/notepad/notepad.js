let currentFilename = 'Untitled';
let isModified = false;
let savedFiles = [];
let selectedFile = null;

const editor = document.getElementById('editor');
const filenameDisplay = document.getElementById('filename');
const unsavedIndicator = document.getElementById('unsaved');
const savedIndicator = document.getElementById('saved');

// Initialize
function init() {
    loadSavedFilesList();
    updateStatus();
    
    // Check for saved session
    const savedSession = localStorage.getItem('notepad_current_session');
    if (savedSession) {
        const session = JSON.parse(savedSession);
        editor.value = session.content;
        currentFilename = session.filename || 'Untitled';
        filenameDisplay.textContent = currentFilename;
        updateStatus();
    }

    // Auto-save session every 30 seconds
    setInterval(autoSaveSession, 30000);
}

// Event listeners
editor.addEventListener('input', () => {
    isModified = true;
    updateSaveIndicator();
    updateStatus();
});

editor.addEventListener('keydown', (e) => {
    // Ctrl/Cmd + S to save
    if ((e.ctrlKey || e.metaKey) && e.key === 's') {
        e.preventDefault();
        saveFile();
    }
    
    // Ctrl/Cmd + N for new
    if ((e.ctrlKey || e.metaKey) && e.key === 'n') {
        e.preventDefault();
        newFile();
    }
    
    // Ctrl/Cmd + O to open
    if ((e.ctrlKey || e.metaKey) && e.key === 'o') {
        e.preventDefault();
        openFileDialog();
    }
    
    updateCursorPosition();
});

editor.addEventListener('click', updateCursorPosition);
editor.addEventListener('keyup', updateCursorPosition);

function updateCursorPosition() {
    const pos = editor.selectionStart;
    const text = editor.value.substring(0, pos);
    const lines = text.split('\n');
    const currentLine = lines.length;
    const currentCol = lines[lines.length - 1].length + 1;
    document.getElementById('cursorPos').textContent = `Ln ${currentLine}, Col ${currentCol}`;
}

function updateStatus() {
    const text = editor.value;
    const words = text.trim() ? text.trim().split(/\s+/).length : 0;
    const chars = text.length;
    
    document.getElementById('wordCount').textContent = `${words} word${words !== 1 ? 's' : ''}`;
    document.getElementById('charCount').textContent = `${chars} char${chars !== 1 ? 's' : ''}`;
}

function updateSaveIndicator() {
    if (isModified) {
        unsavedIndicator.classList.add('show');
        savedIndicator.classList.remove('show');
    } else {
        unsavedIndicator.classList.remove('show');
    }
}

function showSavedIndicator() {
    savedIndicator.classList.add('show');
    unsavedIndicator.classList.remove('show');
    setTimeout(() => {
        savedIndicator.classList.remove('show');
    }, 2000);
}

function newFile() {
    if (isModified) {
        if (!confirm('You have unsaved changes. Create new file anyway?')) {
            return;
        }
    }
    editor.value = '';
    currentFilename = 'Untitled';
    filenameDisplay.textContent = currentFilename;
    isModified = false;
    updateSaveIndicator();
    updateStatus();
    localStorage.removeItem('notepad_current_session');
}

function saveFile() {
    if (currentFilename === 'Untitled') {
        openSaveDialog();
    } else {
        confirmSave();
    }
}

function openSaveDialog() {
    document.getElementById('saveFilename').value = currentFilename === 'Untitled' ? '' : currentFilename;
    document.getElementById('saveDialog').classList.add('show');
    document.getElementById('saveFilename').focus();
}

function closeDialog(dialogId) {
    document.getElementById(dialogId).classList.remove('show');
}

function confirmSave() {
    const filenameInput = document.getElementById('saveFilename');
    let filename = filenameInput.value.trim();
    
    if (!filename) {
        filename = 'untitled.txt';
    }
    
    // Add .txt extension if no extension provided
    if (!filename.includes('.')) {
        filename += '.txt';
    }
    
    const content = editor.value;
    const timestamp = new Date().toISOString();
    
    // Save to localStorage
    const fileData = {
        name: filename,
        content: content,
        modified: timestamp
    };
    
    localStorage.setItem(`notepad_file_${filename}`, JSON.stringify(fileData));
    
    // Update current file
    currentFilename = filename;
    filenameDisplay.textContent = filename;
    isModified = false;
    updateSaveIndicator();
    showSavedIndicator();
    
    // Update saved files list
    loadSavedFilesList();
    
    // Save session
    autoSaveSession();
    
    closeDialog('saveDialog');
}

function loadSavedFilesList() {
    savedFiles = [];
    for (let i = 0; i < localStorage.length; i++) {
        const key = localStorage.key(i);
        if (key && key.startsWith('notepad_file_')) {
            const filename = key.replace('notepad_file_', '');
            const data = JSON.parse(localStorage.getItem(key));
            savedFiles.push({
                name: filename,
                modified: data.modified,
                content: data.content
            });
        }
    }
    
    // Sort by modified date, newest first
    savedFiles.sort((a, b) => new Date(b.modified) - new Date(a.modified));
}

function openFileDialog() {
    loadSavedFilesList();
    const fileList = document.getElementById('fileList');
    fileList.innerHTML = '';
    
    if (savedFiles.length === 0) {
        fileList.innerHTML = '<p style="text-align: center; color: #666; padding: 20px;">No saved files yet.</p>';
    } else {
        savedFiles.forEach(file => {
            const fileItem = document.createElement('div');
            fileItem.className = 'file-item';
            fileItem.dataset.filename = file.name;
            fileItem.onclick = () => selectFile(file.name);
            
            const date = new Date(file.modified);
            const dateStr = date.toLocaleDateString() + ' ' + date.toLocaleTimeString();
            
            fileItem.innerHTML = `
                <span class="file-name">📄 ${escapeHtml(file.name)}</span>
                <span class="file-date">${dateStr}</span>
            `;
            
            fileList.appendChild(fileItem);
        });
    }
    
    selectedFile = null;
    document.getElementById('openDialog').classList.add('show');
}

function selectFile(filename) {
    selectedFile = filename;
    document.querySelectorAll('.file-item').forEach(item => {
        item.classList.toggle('selected', item.dataset.filename === filename);
    });
}

function confirmOpen() {
    if (!selectedFile) {
        alert('Please select a file to open.');
        return;
    }
    
    if (isModified) {
        if (!confirm('You have unsaved changes. Open file anyway?')) {
            return;
        }
    }
    
    const fileData = JSON.parse(localStorage.getItem(`notepad_file_${selectedFile}`));
    if (fileData) {
        editor.value = fileData.content;
        currentFilename = fileData.name;
        filenameDisplay.textContent = currentFilename;
        isModified = false;
        updateSaveIndicator();
        updateStatus();
        autoSaveSession();
    }
    
    closeDialog('openDialog');
}

function deleteSelectedFile() {
    if (!selectedFile) {
        alert('Please select a file to delete.');
        return;
    }
    
    if (!confirm(`Are you sure you want to delete "${selectedFile}"?`)) {
        return;
    }
    
    localStorage.removeItem(`notepad_file_${selectedFile}`);
    
    if (currentFilename === selectedFile) {
        currentFilename = 'Untitled';
        filenameDisplay.textContent = currentFilename;
    }
    
    loadSavedFilesList();
    openFileDialog(); // Refresh the dialog
}

function clearAll() {
    if (editor.value && !confirm('Are you sure you want to clear all text?')) {
        return;
    }
    editor.value = '';
    isModified = true;
    updateSaveIndicator();
    updateStatus();
}

function downloadFile() {
    const content = editor.value;
    const blob = new Blob([content], { type: 'text/plain' });
    const url = URL.createObjectURL(blob);
    
    const a = document.createElement('a');
    a.href = url;
    a.download = currentFilename === 'Untitled' ? 'download.txt' : currentFilename;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    
    URL.revokeObjectURL(url);
}

function uploadFile() {
    document.getElementById('fileInput').click();
}

function handleFileUpload(event) {
    const file = event.target.files[0];
    if (!file) return;
    
    if (isModified) {
        if (!confirm('You have unsaved changes. Upload file anyway?')) {
            event.target.value = '';
            return;
        }
    }
    
    const reader = new FileReader();
    reader.onload = (e) => {
        editor.value = e.target.result;
        currentFilename = file.name;
        filenameDisplay.textContent = currentFilename;
        isModified = true;
        updateSaveIndicator();
        updateStatus();
        autoSaveSession();
    };
    reader.readAsText(file);
    
    event.target.value = '';
}

function autoSaveSession() {
    const session = {
        filename: currentFilename,
        content: editor.value,
        timestamp: new Date().toISOString()
    };
    localStorage.setItem('notepad_current_session', JSON.stringify(session));
}

function escapeHtml(text) {
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
}

// Close dialog on overlay click
document.querySelectorAll('.dialog-overlay').forEach(overlay => {
    overlay.addEventListener('click', (e) => {
        if (e.target === overlay) {
            overlay.classList.remove('show');
        }
    });
});

// Enter key in save dialog
document.getElementById('saveFilename').addEventListener('keydown', (e) => {
    if (e.key === 'Enter') {
        confirmSave();
    }
});

// Warn before leaving with unsaved changes
window.addEventListener('beforeunload', (e) => {
    if (isModified) {
        e.preventDefault();
        e.returnValue = '';
    }
});

// Initialize on load
init();

// Service Worker Registration
if ('serviceWorker' in navigator) {
    window.addEventListener('load', () => {
        navigator.serviceWorker.register('./sw.js')
            .then((registration) => {
                console.log('Notepad SW registered:', registration);
            })
            .catch((error) => {
                console.log('Notepad SW registration failed:', error);
            });
    });
}
