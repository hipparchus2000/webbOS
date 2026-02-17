// Default settings
const defaultSettings = {
    theme: 'system',
    accentColor: '#667eea',
    animations: true,
    fontSize: 16,
    reduceMotion: false,
    notifications: true,
    sound: false
};

let currentSettings = { ...defaultSettings };

// Initialize
function init() {
    loadSettings();
    updateUI();
    updateStorageInfo();
    updateSystemInfo();
    
    // Listen for online/offline events
    window.addEventListener('online', updateOnlineStatus);
    window.addEventListener('offline', updateOnlineStatus);
}

function loadSettings() {
    const saved = localStorage.getItem('webbos_settings');
    if (saved) {
        try {
            currentSettings = { ...defaultSettings, ...JSON.parse(saved) };
        } catch (e) {
            console.error('Failed to load settings:', e);
        }
    }
    applySettings();
}

function saveSettings() {
    localStorage.setItem('webbos_settings', JSON.stringify(currentSettings));
    applySettings();
}

function applySettings() {
    // Apply theme
    if (currentSettings.theme === 'dark') {
        document.documentElement.style.colorScheme = 'dark';
    } else if (currentSettings.theme === 'light') {
        document.documentElement.style.colorScheme = 'light';
    } else {
        document.documentElement.style.colorScheme = 'light dark';
    }
    
    // Apply accent color
    document.documentElement.style.setProperty('--accent-color', currentSettings.accentColor);
    document.documentElement.style.setProperty('--accent-hover', adjustColor(currentSettings.accentColor, -20));
    
    // Apply font size
    document.documentElement.style.fontSize = currentSettings.fontSize + 'px';
    
    // Apply reduce motion
    if (currentSettings.reduceMotion) {
        document.documentElement.style.setProperty('--transition-duration', '0s');
    } else {
        document.documentElement.style.setProperty('--transition-duration', '0.3s');
    }
    
    // Store global settings for other apps to use
    localStorage.setItem('webbos_global_theme', currentSettings.theme);
    localStorage.setItem('webbos_global_accent', currentSettings.accentColor);
    localStorage.setItem('webbos_global_fontsize', currentSettings.fontSize);
}

function updateUI() {
    document.getElementById('themeSelect').value = currentSettings.theme;
    document.getElementById('accentColor').value = currentSettings.accentColor;
    document.getElementById('animationsToggle').checked = currentSettings.animations;
    document.getElementById('fontSize').value = currentSettings.fontSize;
    document.getElementById('fontSizeValue').textContent = currentSettings.fontSize + 'px';
    document.getElementById('reduceMotion').checked = currentSettings.reduceMotion;
    document.getElementById('notificationsToggle').checked = currentSettings.notifications;
    document.getElementById('soundToggle').checked = currentSettings.sound;
}

// Setting handlers
function setTheme(theme) {
    currentSettings.theme = theme;
    saveSettings();
    showToast('Theme updated');
}

function setAccentColor(color) {
    currentSettings.accentColor = color;
    saveSettings();
    showToast('Accent color updated');
}

function toggleAnimations(enabled) {
    currentSettings.animations = enabled;
    saveSettings();
    showToast(enabled ? 'Animations enabled' : 'Animations disabled');
}

function setFontSize(size) {
    currentSettings.fontSize = parseInt(size);
    document.getElementById('fontSizeValue').textContent = size + 'px';
    saveSettings();
}

function toggleReduceMotion(enabled) {
    currentSettings.reduceMotion = enabled;
    saveSettings();
    showToast(enabled ? 'Reduced motion enabled' : 'Reduced motion disabled');
}

function toggleNotifications(enabled) {
    currentSettings.notifications = enabled;
    saveSettings();
    
    if (enabled) {
        // Request notification permission
        if ('Notification' in window && Notification.permission === 'default') {
            Notification.requestPermission();
        }
    }
    
    showToast(enabled ? 'Notifications enabled' : 'Notifications disabled');
}

function toggleSound(enabled) {
    currentSettings.sound = enabled;
    saveSettings();
    showToast(enabled ? 'Sound effects enabled' : 'Sound effects disabled');
}

// Helper to darken/lighten color
function adjustColor(color, amount) {
    const hex = color.replace('#', '');
    const r = Math.max(0, Math.min(255, parseInt(hex.substr(0, 2), 16) + amount));
    const g = Math.max(0, Math.min(255, parseInt(hex.substr(2, 2), 16) + amount));
    const b = Math.max(0, Math.min(255, parseInt(hex.substr(4, 2), 16) + amount));
    return `#${r.toString(16).padStart(2, '0')}${g.toString(16).padStart(2, '0')}${b.toString(16).padStart(2, '0')}`;
}

// Storage info
function updateStorageInfo() {
    let totalSize = 0;
    
    for (let i = 0; i < localStorage.length; i++) {
        const key = localStorage.key(i);
        if (key) {
            const value = localStorage.getItem(key);
            if (value) {
                totalSize += key.length + value.length;
            }
        }
    }
    
    // Convert to KB (2 bytes per character)
    const usedKB = Math.round(totalSize * 2 / 1024);
    const totalKB = 5120; // Approximate 5MB limit
    const percentUsed = Math.min(100, (usedKB / totalKB) * 100);
    
    document.getElementById('storageBar').style.width = percentUsed + '%';
    document.getElementById('storageUsed').textContent = usedKB + ' KB used';
}

// System info
function updateSystemInfo() {
    document.getElementById('userAgent').textContent = navigator.userAgent;
    document.getElementById('screenRes').textContent = `${window.screen.width} x ${window.screen.height}`;
    updateOnlineStatus();
}

function updateOnlineStatus() {
    const statusEl = document.getElementById('onlineStatus');
    if (navigator.onLine) {
        statusEl.textContent = 'Online ✅';
        statusEl.style.color = '#2ecc71';
    } else {
        statusEl.textContent = 'Offline ❌';
        statusEl.style.color = '#e74c3c';
    }
}

// Data management
function clearAllData() {
    if (!confirm('Are you sure you want to clear ALL data? This will delete all saved files, settings, and app data. This cannot be undone.')) {
        return;
    }
    
    // Keep only settings
    const settings = localStorage.getItem('webbos_settings');
    localStorage.clear();
    if (settings) {
        localStorage.setItem('webbos_settings', settings);
    }
    
    updateStorageInfo();
    showToast('All data cleared', 'success');
}

function resetSettings() {
    if (!confirm('Are you sure you want to reset all settings to defaults?')) {
        return;
    }
    
    currentSettings = { ...defaultSettings };
    saveSettings();
    updateUI();
    showToast('Settings reset to defaults', 'success');
}

// Toast notification
function showToast(message, type = '') {
    const toast = document.getElementById('toast');
    toast.textContent = message;
    toast.className = 'toast show' + (type ? ' ' + type : '');
    
    setTimeout(() => {
        toast.classList.remove('show');
    }, 3000);
}

// Request notification permission on load
if ('Notification' in window && Notification.permission === 'default') {
    Notification.requestPermission();
}

// Initialize
init();

// Service Worker Registration
if ('serviceWorker' in navigator) {
    window.addEventListener('load', () => {
        navigator.serviceWorker.register('./sw.js')
            .then((registration) => {
                console.log('Settings SW registered:', registration);
            })
            .catch((error) => {
                console.log('Settings SW registration failed:', error);
            });
    });
}
