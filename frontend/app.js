// Gruesome Z-Machine Platform - Frontend Application
// Web-based Z-Machine game player using WASM interpreter
// Supports authentication, game library, and cloud save/load functionality

import init, { WasmInterpreter } from './gruesome.js';

// Configuration - use dev-config if available, otherwise default to production
const API_BASE = (typeof DEV_CONFIG !== 'undefined')
    ? DEV_CONFIG.getApiBase()
    : 'https://api.gruesome.skeptomai.com';

// Application State
let accessToken = null;        // JWT authentication token
let currentGame = null;        // Currently loaded game ID
let wasmInterpreter = null;    // WASM Z-Machine interpreter instance
let authMode = 'login';        // Authentication mode: 'login', 'signup', 'reset', or 'confirm-reset'
let isAdmin = false;           // Whether current user has admin role
let currentUploadData = null;  // Data for current file upload (for metadata form)
let editingGameId = null;      // Game ID currently being edited
let deletingGameId = null;     // Game ID pending deletion

// Visual Settings State - Retro terminal styling
let visualSettings = {
    theme: 'green',
    font: 'default',
    crtEnabled: false,
    blurLevel: 'medium',
    collapsed: false,
    controlsCollapsed: true  // Default to collapsed for cleaner initial view
};

// DOM Elements - initialized after DOM is ready
let loginSection, gameLibrary, gamePlayer, adminSection, authStatus, gamesList, gameOutput, gameInput, logoutButton, adminButton;
let emailInput, usernameInput, passwordInput, authSubmit, toggleAuthLink, forgotPasswordLink;
let resetCodeInput, newPasswordInput;

// Admin DOM Elements
let adminGamesList, adminSearchInput, adminTabButtons, adminTabContents;
let uploadDropzone, fileInput, uploadProgress, metadataForm;
let editGameModal, deleteGameModal;

// Flash Message Helper Functions
function showFlashMessage(message, type = 'error') {
    const container = document.getElementById('flash-messages');
    const flash = document.createElement('div');
    flash.className = `flash-message ${type}`;
    flash.textContent = message;

    container.appendChild(flash);

    // Auto-dismiss after 5 seconds
    setTimeout(() => {
        flash.style.opacity = '0';
        flash.style.transform = 'translateY(-20px)';
        flash.style.transition = 'all 0.3s ease-out';
        setTimeout(() => flash.remove(), 300);
    }, 5000);
}

// Visual Settings Functions
function loadVisualSettings() {
    const saved = localStorage.getItem('gruesome-visual-settings');
    if (saved) {
        try {
            visualSettings = JSON.parse(saved);
        } catch (e) {
            console.log('Failed to load visual settings, using defaults');
        }
    }
}

function saveVisualSettings() {
    localStorage.setItem('gruesome-visual-settings', JSON.stringify(visualSettings));
}

function applyVisualSettings() {
    if (!gameOutput) return;

    const crtContainer = gameOutput.parentElement?.classList.contains('crt-container')
        ? gameOutput.parentElement
        : null;

    // Remove existing theme/font/CRT classes from gameOutput
    gameOutput.className = gameOutput.className
        .split(' ')
        .filter(c => !c.startsWith('theme-') && !c.startsWith('font-') &&
                     !c.startsWith('crt-') && c !== 'effects-enabled')
        .join(' ');

    // Apply theme and font to gameOutput
    gameOutput.classList.add(`theme-${visualSettings.theme}`);
    gameOutput.classList.add(`font-${visualSettings.font}`);

    // Apply CRT effects to container (not scrolling element)
    if (crtContainer) {
        crtContainer.className = 'crt-container';
        if (visualSettings.crtEnabled) {
            crtContainer.classList.add('crt-enhanced');
            crtContainer.classList.add(`crt-blur-${visualSettings.blurLevel}`);
            crtContainer.classList.add('effects-enabled');
        }
    }
}

function updateTheme(theme) {
    visualSettings.theme = theme;
    saveVisualSettings();
    applyVisualSettings();
    renderVisualSettingsUI();
}

function updateFont(font) {
    visualSettings.font = font;
    saveVisualSettings();
    applyVisualSettings();
    renderVisualSettingsUI();
}

function updateCrtEnabled(enabled) {
    visualSettings.crtEnabled = enabled;
    saveVisualSettings();
    applyVisualSettings();
    renderVisualSettingsUI();
}

function updateBlurLevel(level) {
    visualSettings.blurLevel = level;
    saveVisualSettings();
    applyVisualSettings();
    renderVisualSettingsUI();
}

function toggleVisualSettings() {
    visualSettings.collapsed = !visualSettings.collapsed;
    saveVisualSettings();

    const settingsContent = document.getElementById('settings-content');
    const toggleIcon = document.getElementById('settings-toggle-icon');

    if (visualSettings.collapsed) {
        settingsContent.classList.add('collapsed');
        toggleIcon.classList.add('collapsed');
    } else {
        settingsContent.classList.remove('collapsed');
        toggleIcon.classList.remove('collapsed');
    }
}

function toggleControlPanels() {
    visualSettings.controlsCollapsed = !visualSettings.controlsCollapsed;
    saveVisualSettings();

    const controlsContent = document.getElementById('controls-content');
    const toggleIcon = document.getElementById('controls-toggle-icon');

    if (visualSettings.controlsCollapsed) {
        controlsContent.classList.add('collapsed');
        toggleIcon.classList.add('collapsed');
    } else {
        controlsContent.classList.remove('collapsed');
        toggleIcon.classList.remove('collapsed');
    }
}

function renderVisualSettingsUI() {
    // Theme toggle
    const themeToggle = document.getElementById('theme-toggle');
    if (themeToggle) {
        themeToggle.innerHTML = ['green', 'amber', 'white']
            .map(theme => `
                <button
                    class="theme-button ${visualSettings.theme === theme ? 'active' : ''}"
                    onclick="updateTheme('${theme}')"
                >${theme.charAt(0).toUpperCase() + theme.slice(1)}</button>
            `).join('');
    }

    // Font toggle
    const fontToggle = document.getElementById('font-toggle');
    if (fontToggle) {
        const fonts = [
            { id: 'default', label: 'Default' },
            { id: 'vt323', label: 'VT323' },
            { id: 'ibm3270', label: 'IBM 3270' },
            { id: 'sharetech', label: 'Share Tech' }
        ];
        fontToggle.innerHTML = fonts
            .map(font => `
                <button
                    class="font-button ${visualSettings.font === font.id ? 'active' : ''}"
                    onclick="updateFont('${font.id}')"
                >${font.label}</button>
            `).join('');
    }

    // CRT toggle
    const crtToggle = document.getElementById('crt-toggle');
    if (crtToggle) {
        crtToggle.innerHTML = `
            <button
                class="theme-button ${!visualSettings.crtEnabled ? 'active' : ''}"
                onclick="updateCrtEnabled(false)"
            >Off</button>
            <button
                class="theme-button ${visualSettings.crtEnabled ? 'active' : ''}"
                onclick="updateCrtEnabled(true)"
            >On</button>
        `;
    }

    // Blur toggle
    const blurToggle = document.getElementById('blur-toggle');
    if (blurToggle) {
        const levels = [
            { id: 'none', label: 'Sharp' },
            { id: 'light', label: 'Light' },
            { id: 'medium', label: 'Medium' },
            { id: 'heavy', label: 'Heavy' }
        ];
        blurToggle.className = `blur-toggle ${!visualSettings.crtEnabled ? 'disabled' : ''}`;
        blurToggle.innerHTML = levels
            .map(level => `
                <button
                    class="blur-button ${visualSettings.blurLevel === level.id ? 'active' : ''}"
                    onclick="updateBlurLevel('${level.id}')"
                    ${!visualSettings.crtEnabled ? 'disabled' : ''}
                >${level.label}</button>
            `).join('');
    }
}

// Make visual settings functions global so onclick handlers work
window.updateTheme = updateTheme;
window.updateFont = updateFont;
window.updateCrtEnabled = updateCrtEnabled;
window.updateBlurLevel = updateBlurLevel;
window.toggleVisualSettings = toggleVisualSettings;
window.toggleControlPanels = toggleControlPanels;

// Application Initialization
// Sets up DOM references, event handlers, and checks for existing auth session
async function initApp() {
    // Get DOM elements after DOM is ready
    loginSection = document.getElementById('login-section');
    gameLibrary = document.getElementById('game-library');
    gamePlayer = document.getElementById('game-player');
    adminSection = document.getElementById('admin-section');
    authStatus = document.getElementById('auth-status');
    gamesList = document.getElementById('games-list');
    gameOutput = document.getElementById('game-output');
    gameInput = null;  // Will be created dynamically when game loads
    logoutButton = document.getElementById('logout-button');
    adminButton = document.getElementById('admin-button');

    // Auth form elements
    emailInput = document.getElementById('email');
    usernameInput = document.getElementById('username');
    passwordInput = document.getElementById('password');
    resetCodeInput = document.getElementById('reset-code');
    newPasswordInput = document.getElementById('new-password');
    authSubmit = document.getElementById('auth-submit');
    toggleAuthLink = document.getElementById('toggle-auth-mode');
    forgotPasswordLink = document.getElementById('forgot-password-link');

    // Initialize visual settings
    loadVisualSettings();
    renderVisualSettingsUI();

    // Apply collapsed state if saved
    if (visualSettings.collapsed) {
        const settingsContent = document.getElementById('settings-content');
        const toggleIcon = document.getElementById('settings-toggle-icon');
        if (settingsContent) settingsContent.classList.add('collapsed');
        if (toggleIcon) toggleIcon.classList.add('collapsed');
    }
    if (visualSettings.controlsCollapsed) {
        const controlsContent = document.getElementById('controls-content');
        const controlsIcon = document.getElementById('controls-toggle-icon');
        if (controlsContent) controlsContent.classList.add('collapsed');
        if (controlsIcon) controlsIcon.classList.add('collapsed');
    }

    // Set up logout button handler
    if (logoutButton) {
        logoutButton.addEventListener('click', handleLogout);
    } else {
        console.error('ERROR: logout button element not found in DOM!');
    }

    // Set up auth form handler (login/signup)
    const authForm = document.getElementById('auth-form');
    if (authForm) {
        authForm.addEventListener('submit', handleAuthSubmit);
    }

    // Set up toggle auth mode handler
    if (toggleAuthLink) {
        toggleAuthLink.addEventListener('click', toggleAuthMode);
    }

    // Set up forgot password handler
    if (forgotPasswordLink) {
        forgotPasswordLink.addEventListener('click', handleForgotPasswordClick);
    }

    // Note: game input handler is set up dynamically in createInputArea() when game loads

    // Set up back button
    const backButton = document.getElementById('back-button');
    if (backButton) {
        backButton.addEventListener('click', handleBackToLibrary);
    }

    // Set up save button
    const saveButton = document.getElementById('save-button');
    if (saveButton) {
        saveButton.addEventListener('click', handleSaveGame);
    }

    // Set up load button
    const loadButton = document.getElementById('load-button');
    if (loadButton) {
        loadButton.addEventListener('click', handleLoadGame);
    }

    // Admin elements
    adminGamesList = document.getElementById('admin-games-list');
    adminSearchInput = document.getElementById('admin-search');
    adminTabButtons = document.querySelectorAll('.admin-tab');
    adminTabContents = document.querySelectorAll('.admin-tab-content');
    uploadDropzone = document.getElementById('upload-dropzone');
    fileInput = document.getElementById('game-file-input');
    uploadProgress = document.getElementById('upload-progress');
    metadataForm = document.getElementById('metadata-form');
    editGameModal = document.getElementById('edit-game-modal');
    deleteGameModal = document.getElementById('delete-game-modal');

    // Set up admin button
    if (adminButton) {
        adminButton.addEventListener('click', showAdminPanel);
    }

    // Set up admin back button
    const adminBackButton = document.getElementById('admin-back-button');
    if (adminBackButton) {
        adminBackButton.addEventListener('click', handleAdminBackToLibrary);
    }

    // Set up admin tab switching
    adminTabButtons.forEach(btn => {
        btn.addEventListener('click', () => switchAdminTab(btn.dataset.tab));
    });

    // Set up file upload dropzone
    if (uploadDropzone && fileInput) {
        // Click to browse
        uploadDropzone.addEventListener('click', () => fileInput.click());

        // File input change
        fileInput.addEventListener('change', (e) => {
            if (e.target.files.length > 0) {
                handleFileUpload(e.target.files[0]);
            }
        });

        // Drag and drop
        uploadDropzone.addEventListener('dragover', (e) => {
            e.preventDefault();
            uploadDropzone.classList.add('dragover');
        });

        uploadDropzone.addEventListener('dragleave', () => {
            uploadDropzone.classList.remove('dragover');
        });

        uploadDropzone.addEventListener('drop', (e) => {
            e.preventDefault();
            uploadDropzone.classList.remove('dragover');
            if (e.dataTransfer.files.length > 0) {
                handleFileUpload(e.dataTransfer.files[0]);
            }
        });
    }

    // Set up upload form buttons
    const cancelUploadBtn = document.getElementById('cancel-upload');
    if (cancelUploadBtn) {
        cancelUploadBtn.addEventListener('click', resetUploadForm);
    }

    const publishGameBtn = document.getElementById('publish-game');
    if (publishGameBtn) {
        publishGameBtn.addEventListener('click', handlePublishGame);
    }

    // Set up edit modal
    const closeEditModal = document.getElementById('close-edit-modal');
    if (closeEditModal) {
        closeEditModal.addEventListener('click', () => {
            editGameModal.style.display = 'none';
            editingGameId = null;
        });
    }

    const cancelEditBtn = document.getElementById('cancel-edit');
    if (cancelEditBtn) {
        cancelEditBtn.addEventListener('click', () => {
            editGameModal.style.display = 'none';
            editingGameId = null;
        });
    }

    const saveEditBtn = document.getElementById('save-edit');
    if (saveEditBtn) {
        saveEditBtn.addEventListener('click', handleSaveEdit);
    }

    // Set up delete modal
    const closeDeleteModal = document.getElementById('close-delete-modal');
    if (closeDeleteModal) {
        closeDeleteModal.addEventListener('click', () => {
            deleteGameModal.style.display = 'none';
            deletingGameId = null;
        });
    }

    const cancelDeleteBtn = document.getElementById('cancel-delete');
    if (cancelDeleteBtn) {
        cancelDeleteBtn.addEventListener('click', () => {
            deleteGameModal.style.display = 'none';
            deletingGameId = null;
        });
    }

    const confirmDeleteBtn = document.getElementById('confirm-delete');
    if (confirmDeleteBtn) {
        confirmDeleteBtn.addEventListener('click', handleConfirmDelete);
    }

    // Initialize WASM
    await init();

    // Check for existing token
    const savedToken = localStorage.getItem('accessToken');
    if (savedToken) {
        accessToken = savedToken;
        await loadGameLibrary();
        await checkAdminRole();  // Check if user is admin
    }
}

// Logout
function handleLogout() {
    console.log('handleLogout called');
    // Clear token
    localStorage.removeItem('accessToken');
    accessToken = null;

    // Reset UI
    loginSection.style.display = 'block';
    gameLibrary.style.display = 'none';
    gamePlayer.style.display = 'none';
    authStatus.textContent = '';
    logoutButton.style.display = 'none';

    // Clear form
    emailInput.value = '';
    usernameInput.value = '';
    passwordInput.value = '';

    // Reset to login mode
    authMode = 'login';
    updateAuthUI();

    // Reset game state
    currentGame = null;
    wasmInterpreter = null;
}

// Toggle between login and signup modes (or back to login from reset modes)
function toggleAuthMode(e) {
    e.preventDefault();
    // If in reset mode, "Back to Login" should go to login
    if (authMode === 'reset' || authMode === 'confirm-reset') {
        authMode = 'login';
    } else {
        // Normal toggle between login and signup
        authMode = authMode === 'login' ? 'signup' : 'login';
    }
    updateAuthUI();
}

// Handle forgot password link click
function handleForgotPasswordClick(e) {
    e.preventDefault();
    authMode = 'reset';
    updateAuthUI();
}

// Update UI based on current auth mode
function updateAuthUI() {
    // Reset all fields to hidden and not required
    emailInput.style.display = 'none';
    emailInput.required = false;
    passwordInput.style.display = 'none';
    passwordInput.required = false;
    usernameInput.style.display = 'none';
    usernameInput.required = false;
    resetCodeInput.style.display = 'none';
    resetCodeInput.required = false;
    newPasswordInput.style.display = 'none';
    newPasswordInput.required = false;

    // Get password requirements element
    const passwordRequirements = document.getElementById('password-requirements');

    if (authMode === 'signup') {
        // Signup mode: email + username + password
        emailInput.style.display = 'block';
        emailInput.required = true;
        usernameInput.style.display = 'block';
        usernameInput.required = true;
        passwordInput.style.display = 'block';
        passwordInput.required = true;
        authSubmit.textContent = 'Create Account';
        toggleAuthLink.textContent = 'Already have an account? Login';
        forgotPasswordLink.style.display = 'inline';
        // Show password requirements for signup
        if (passwordRequirements) passwordRequirements.style.display = 'block';
    } else if (authMode === 'reset') {
        // Password reset step 1: username only
        usernameInput.style.display = 'block';
        usernameInput.required = true;
        authSubmit.textContent = 'Send Reset Code';
        toggleAuthLink.textContent = 'Back to Login';
        forgotPasswordLink.style.display = 'none';
        // Hide password requirements
        if (passwordRequirements) passwordRequirements.style.display = 'none';
    } else if (authMode === 'confirm-reset') {
        // Password reset step 2: username + code + new password
        usernameInput.style.display = 'block';
        usernameInput.required = true;
        resetCodeInput.style.display = 'block';
        resetCodeInput.required = true;
        newPasswordInput.style.display = 'block';
        newPasswordInput.required = true;
        authSubmit.textContent = 'Reset Password';
        toggleAuthLink.textContent = 'Back to Login';
        forgotPasswordLink.style.display = 'none';
        // Show password requirements for new password
        if (passwordRequirements) passwordRequirements.style.display = 'block';
    } else {
        // Login mode: username + password
        usernameInput.style.display = 'block';
        usernameInput.required = true;
        passwordInput.style.display = 'block';
        passwordInput.required = true;
        authSubmit.textContent = 'Login';
        toggleAuthLink.textContent = 'Need an account? Sign up';
        forgotPasswordLink.style.display = 'inline';
        // Hide password requirements for login
        if (passwordRequirements) passwordRequirements.style.display = 'none';
    }
}

// Handle form submission - route based on auth mode
async function handleAuthSubmit(e) {
    e.preventDefault();
    if (authMode === 'signup') {
        await handleSignup();
    } else if (authMode === 'reset') {
        await handleForgotPassword();
    } else if (authMode === 'confirm-reset') {
        await handleConfirmReset();
    } else {
        await handleLogin();
    }
}

// Signup - Create new account
async function handleSignup() {
    const email = emailInput.value;
    const username = usernameInput.value;
    const password = passwordInput.value;

    try {
        const response = await fetch(`${API_BASE}/api/auth/signup`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ email, username, password })
        });

        const data = await response.json();

        if (!response.ok) {
            // Show detailed error message from API
            const errorMsg = data.message || data.error || 'Signup failed';
            const errorDetails = data.details ? ` (${data.details})` : '';
            throw new Error(errorMsg + errorDetails);
        }

        // Success! Auto-login the user
        showFlashMessage('Account created successfully! Logging you in...', 'success');
        await handleLogin();
    } catch (error) {
        // Check if it's a "user already exists" error
        if (error.message && error.message.toLowerCase().includes('already exists')) {
            showFlashMessage('This username or email is already registered. Try logging in instead.', 'info');
            // Switch to login mode
            authMode = 'login';
            updateAuthUI();
        } else {
            showFlashMessage('Signup failed: ' + error.message, 'error');
        }
    }
}

// Forgot Password - Request reset code
async function handleForgotPassword() {
    const username = usernameInput.value;

    try {
        const response = await fetch(`${API_BASE}/api/auth/forgot-password`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ username })
        });

        const data = await response.json();

        if (!response.ok) {
            throw new Error(data.message || 'Failed to send reset code');
        }

        // Success! Move to confirmation step
        showFlashMessage('Password reset code sent to your email. Please check your inbox.', 'success');
        authMode = 'confirm-reset';
        updateAuthUI();
    } catch (error) {
        showFlashMessage('Failed to send reset code: ' + error.message, 'error');
    }
}

// Confirm Reset - Complete password reset with code
async function handleConfirmReset() {
    const username = usernameInput.value;
    const confirmation_code = resetCodeInput.value;
    const new_password = newPasswordInput.value;

    try {
        const response = await fetch(`${API_BASE}/api/auth/confirm-forgot-password`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ username, confirmation_code, new_password })
        });

        const data = await response.json();

        if (!response.ok) {
            throw new Error(data.message || 'Failed to reset password');
        }

        // Success! Back to login
        showFlashMessage('Password reset successfully! Please login with your new password.', 'success');
        authMode = 'login';
        resetCodeInput.value = '';
        newPasswordInput.value = '';
        updateAuthUI();
    } catch (error) {
        showFlashMessage('Failed to reset password: ' + error.message, 'error');
    }
}

// Login - Authenticate existing user
async function handleLogin() {
    const username = usernameInput.value;
    const password = passwordInput.value;

    try {
        const response = await fetch(`${API_BASE}/api/auth/login`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ username, password })
        });

        const data = await response.json();

        if (!response.ok) {
            throw new Error(data.message || 'Login failed');
        }

        if (data.access_token) {
            accessToken = data.access_token;
            localStorage.setItem('accessToken', accessToken);
            await loadGameLibrary();
        }
    } catch (error) {
        showFlashMessage('Login failed: ' + error.message, 'error');
    }
}

// Load Game Library
// Displays the game library section and fetches available games from the API
async function loadGameLibrary() {
    // Re-get the logout button element in case reference was lost
    logoutButton = document.getElementById('logout-button');

    // Show game library, hide login
    loginSection.style.display = 'none';
    gameLibrary.style.display = 'block';

    // Show logout button (don't clear authStatus.textContent as it removes the logout button!)
    if (logoutButton) {
        logoutButton.style.display = 'inline-block';
    } else {
        console.error('Logout button element not found in DOM');
    }

    try {
        const response = await fetch(`${API_BASE}/api/games`);
        const data = await response.json();

        gamesList.innerHTML = data.games.map(game => `
            <div class="game-card" onclick="loadGame('${game.game_id}')">
                <h3>${game.title}</h3>
                <p>${game.description}</p>
                <p>Version: ${game.version} | Size: ${(game.file_size / 1024).toFixed(1)}KB</p>
            </div>
        `).join('');

        // Check if user has admin role (and show/hide admin button)
        await checkAdminRole();
    } catch (error) {
        gamesList.innerHTML = `<div class="error">Failed to load games: ${error.message}</div>`;
    }
}

// Create and append input area to game output
function createInputArea() {
    // Remove existing input area if present
    const existingInput = gameOutput.querySelector('.input-area');
    if (existingInput) {
        existingInput.remove();
    }

    // Create input area with prompt
    const inputArea = document.createElement('div');
    inputArea.className = 'input-area';
    inputArea.innerHTML = '<span class="prompt" id="input-prompt">&gt;</span><input type="text" id="game-input" placeholder="" autocomplete="off">';

    // Append to game output
    gameOutput.appendChild(inputArea);

    // Update gameInput reference
    gameInput = document.getElementById('game-input');

    // Set up event listeners
    if (gameInput) {
        gameInput.addEventListener('keypress', handleGameInput);
        // Auto-collapse controls when input is focused (for more screen space)
        gameInput.addEventListener('focus', () => {
            if (!visualSettings.controlsCollapsed) {
                toggleControlPanels();
            }
        });
        // Prompt stays visible - no need to hide/show
    }
}

// Load and Start Game
window.loadGame = async function(gameId) {
    try {
        console.log(`loadGame: Starting to load game ${gameId}`);

        // Get download URL
        console.log(`loadGame: Fetching download URL for ${gameId}`);
        const response = await fetch(`${API_BASE}/api/games/${gameId}/file`);
        const data = await response.json();
        console.log(`loadGame: Got download URL`);

        // Download game file
        console.log(`loadGame: Downloading game file from S3`);
        const gameResponse = await fetch(data.download_url);
        const gameData = await gameResponse.arrayBuffer();
        console.log(`loadGame: Downloaded ${gameData.byteLength} bytes`);

        // Initialize WASM interpreter with game data
        console.log(`loadGame: Initializing WASM interpreter`);
        wasmInterpreter = new WasmInterpreter(new Uint8Array(gameData));
        currentGame = gameId;
        console.log(`loadGame: WASM interpreter initialized`);

        // Clear game output
        gameOutput.textContent = '';

        // Show game player
        gameLibrary.style.display = 'none';
        gamePlayer.style.display = 'block';

        // Enable Save and Load buttons
        const saveButton = document.getElementById('save-button');
        const loadButton = document.getElementById('load-button');
        if (saveButton) saveButton.disabled = false;
        if (loadButton) loadButton.disabled = false;

        // Wrap game output in CRT container if CRT is enabled
        if (!gameOutput.parentElement.classList.contains('crt-container')) {
            const crtContainer = document.createElement('div');
            crtContainer.className = 'crt-container';
            gameOutput.parentElement.insertBefore(crtContainer, gameOutput);
            crtContainer.appendChild(gameOutput);
        }

        // Apply visual settings to game output
        applyVisualSettings();

        // Start game - step until it needs input
        console.log(`loadGame: Starting game execution`);
        runUntilInput();
        console.log(`loadGame: Game ready for input`);

        // Create input area after game text
        createInputArea();

        // Focus input
        if (gameInput) gameInput.focus();

    } catch (error) {
        console.error(`loadGame: Error loading game ${gameId}:`, error);
        showFlashMessage('Failed to load game: ' + error.message, 'error');
    }
};

// Run interpreter until it needs input
function runUntilInput() {
    let result;
    let stepCount = 0;
    const MAX_STEPS = 100000; // Allow for long game intros like Enchanter (1116 chars)

    do {
        stepCount++;
        if (stepCount > MAX_STEPS) {
            console.error('runUntilInput: Maximum steps exceeded, possible infinite loop');
            gameOutput.textContent += '\n\n[ERROR: Game initialization failed - too many steps]';
            throw new Error('Game initialization exceeded maximum steps');
        }

        result = wasmInterpreter.step();

        if (result.output) {
            // Filter out duplicate ">" prompt at end of output
            let output = result.output;
            // Handle various prompt patterns: ">", "> ", "\n>", "\n> "
            if (output.endsWith('> ')) {
                output = output.slice(0, -2);
            } else if (output.endsWith('>')) {
                output = output.slice(0, -1);
            }
            gameOutput.textContent += output;
            gameOutput.scrollTop = gameOutput.scrollHeight;
        }
        if (result.error) {
            console.error('Game error:', result.error);
        }
        if (result.quit) {
            gameOutput.textContent += '\n\n[Game Over]';
            // Disable Save button when game ends (Load remains enabled to restart from save)
            const saveButton = document.getElementById('save-button');
            if (saveButton) saveButton.disabled = true;
            break;
        }
        // Handle in-game save command
        if (result.save_data) {
            const blob = new Blob([result.save_data], { type: 'application/octet-stream' });
            const url = URL.createObjectURL(blob);
            const a = document.createElement('a');
            a.href = url;
            a.download = 'save.qzl';
            a.click();
            URL.revokeObjectURL(url);
        }
        // Handle in-game restore command
        if (result.needs_restore_data) {
            handleInGameRestore();
            break;
        }
    } while (!result.needs_input && !result.quit && !result.needs_restore_data);

    console.log(`runUntilInput completed after ${stepCount} steps`);
}

// Handle in-game restore command (when user types "restore")
async function handleInGameRestore() {
    // Create file input for restore
    const input = document.createElement('input');
    input.type = 'file';
    input.accept = '.qzl,.sav';

    input.onchange = async (e) => {
        const file = e.target.files[0];
        if (file) {
            const data = new Uint8Array(await file.arrayBuffer());
            wasmInterpreter.provide_restore_data(data);

            // Clear output and continue game
            gameOutput.textContent = '';
            runUntilInput();

            // Recreate input area
            createInputArea();
            if (gameInput) gameInput.focus();
        } else {
            // User cancelled - tell interpreter to cancel restore
            wasmInterpreter.cancel_restore();
            runUntilInput();
            createInputArea();
            if (gameInput) gameInput.focus();
        }
    };

    // Show file picker
    input.click();
}

// Game Input
function handleGameInput(e) {
    if (e.key === 'Enter' && gameInput.value.trim()) {
        const command = gameInput.value.trim();

        // Add command to output as text before removing input area
        const inputArea = gameOutput.querySelector('.input-area');
        if (inputArea) {
            // Convert input area to plain text to preserve the command line
            const commandText = '> ' + command + '\n';
            const textNode = document.createTextNode(commandText);
            inputArea.replaceWith(textNode);
        }

        // Provide input to interpreter (game handles echo)
        wasmInterpreter.provide_input(command);

        // Run until next input needed
        runUntilInput();

        // Re-create input area for next command
        createInputArea();

        // Restore focus to input
        if (gameInput) {
            gameInput.focus();
        }

        // Scroll to bottom
        gameOutput.scrollTop = gameOutput.scrollHeight;
    }
}

// Back to Library
function handleBackToLibrary() {
    gamePlayer.style.display = 'none';
    gameLibrary.style.display = 'block';
    currentGame = null;
    wasmInterpreter = null;
}

// Save Game
async function handleSaveGame() {
    if (!wasmInterpreter) return;

    const saveName = prompt('Enter save name:');
    if (!saveName) return;

    try {
        // Get save state from WASM
        const saveData = wasmInterpreter.save_game();

        // Get upload URL
        const response = await fetch(`${API_BASE}/api/saves/${currentGame}/${saveName}`, {
            method: 'POST',
            headers: {
                'Authorization': `Bearer ${accessToken}`,
                'Content-Type': 'application/json'
            },
            body: JSON.stringify({ file_size: saveData.length })
        });

        const data = await response.json();

        // Check for API errors
        if (!response.ok || !data.upload_url) {
            throw new Error(data.message || data.error || 'Failed to get upload URL');
        }

        // Upload save file to S3
        const uploadResponse = await fetch(data.upload_url, {
            method: 'PUT',
            body: saveData
        });

        // Verify upload succeeded
        if (!uploadResponse.ok) {
            throw new Error('Failed to upload save file to storage. Please try again.');
        }

        showFlashMessage('Game saved successfully!', 'success');
    } catch (error) {
        showFlashMessage('Failed to save game: ' + error.message, 'error');
    }
}

// Load Game
async function handleLoadGame() {
    if (!currentGame) return;

    try {
        // List saves for this game
        const response = await fetch(`${API_BASE}/api/saves/${currentGame}`, {
            headers: { 'Authorization': `Bearer ${accessToken}` }
        });

        const data = await response.json();

        // Check for API errors
        if (!response.ok || !data.saves) {
            throw new Error(data.message || data.error || 'Failed to load saves');
        }

        if (data.saves.length === 0) {
            showFlashMessage('No saves found for this game', 'info');
            return;
        }

        // Simple selection (could be improved with UI)
        const saveNames = data.saves.map(s => s.save_name).join(', ');
        const saveName = prompt(`Available saves: ${saveNames}\n\nEnter save name to load:`);

        if (!saveName) return;

        // Get download URL
        const downloadResponse = await fetch(`${API_BASE}/api/saves/${currentGame}/${saveName}`, {
            headers: { 'Authorization': `Bearer ${accessToken}` }
        });

        const downloadData = await downloadResponse.json();

        // Check for API errors (orphaned save detection)
        if (!downloadResponse.ok || !downloadData.download_url) {
            throw new Error(downloadData.message || downloadData.error || 'Failed to get save file');
        }

        // Download save file
        const saveResponse = await fetch(downloadData.download_url);
        if (!saveResponse.ok) {
            throw new Error('Failed to download save file from storage');
        }
        const saveData = new Uint8Array(await saveResponse.arrayBuffer());

        // Restore save state
        wasmInterpreter.restore_game(saveData);

        // Clear output and show restore message
        gameOutput.textContent = '';

        // Enable Save and Load buttons (in case they were disabled from previous game quit)
        const saveButton = document.getElementById('save-button');
        const loadButton = document.getElementById('load-button');
        if (saveButton) saveButton.disabled = false;
        if (loadButton) loadButton.disabled = false;

        // After restore, let game output current state
        runUntilInput();

        // Recreate input area after restore
        createInputArea();

        // Focus input
        if (gameInput) gameInput.focus();

    } catch (error) {
        showFlashMessage('Failed to load save: ' + error.message, 'error');
    }
}

// ===================================================================
// ADMIN FUNCTIONALITY
// ===================================================================

// Check if user has admin role
async function checkAdminRole() {
    try {
        const response = await fetch(`${API_BASE}/api/auth/me`, {
            headers: { 'Authorization': `Bearer ${accessToken}` }
        });

        if (!response.ok) {
            isAdmin = false;
            return;
        }

        const data = await response.json();
        isAdmin = data.profile && data.profile.role === 'admin';

        // Show/hide admin button based on role
        if (adminButton) {
            adminButton.style.display = isAdmin ? 'inline-block' : 'none';
        }
    } catch (error) {
        console.error('Failed to check admin role:', error);
        isAdmin = false;
    }
}

// Admin API: Get all games
async function adminGetAllGames() {
    const response = await fetch(`${API_BASE}/api/admin/games`, {
        headers: { 'Authorization': `Bearer ${accessToken}` }
    });

    if (!response.ok) {
        const error = await response.json();
        throw new Error(error.message || 'Failed to fetch games');
    }

    return await response.json();
}

// Admin API: Get presigned upload URL
async function adminGetUploadUrl(filename) {
    const response = await fetch(`${API_BASE}/api/admin/games/upload-url`, {
        method: 'POST',
        headers: {
            'Content-Type': 'application/json',
            'Authorization': `Bearer ${accessToken}`
        },
        body: JSON.stringify({ filename })
    });

    if (!response.ok) {
        const error = await response.json();
        throw new Error(error.message || 'Failed to get upload URL');
    }

    return await response.json();
}

// Admin API: Create game metadata
async function adminCreateGame(gameData) {
    const response = await fetch(`${API_BASE}/api/admin/games`, {
        method: 'POST',
        headers: {
            'Content-Type': 'application/json',
            'Authorization': `Bearer ${accessToken}`
        },
        body: JSON.stringify(gameData)
    });

    if (!response.ok) {
        const error = await response.json();
        throw new Error(error.message || 'Failed to create game');
    }

    return await response.json();
}

// Admin API: Update game metadata
async function adminUpdateGame(gameId, gameData) {
    const response = await fetch(`${API_BASE}/api/admin/games/${gameId}`, {
        method: 'PUT',
        headers: {
            'Content-Type': 'application/json',
            'Authorization': `Bearer ${accessToken}`
        },
        body: JSON.stringify(gameData)
    });

    if (!response.ok) {
        const error = await response.json();
        throw new Error(error.message || 'Failed to update game');
    }

    return await response.json();
}

// Admin API: Delete game
async function adminDeleteGame(gameId) {
    const response = await fetch(`${API_BASE}/api/admin/games/${gameId}`, {
        method: 'DELETE',
        headers: { 'Authorization': `Bearer ${accessToken}` }
    });

    if (!response.ok) {
        const error = await response.json();
        throw new Error(error.message || 'Failed to delete game');
    }

    return await response.json();
}

// Extract Z-Machine metadata from file bytes
function extractZMachineMetadata(bytes) {
    if (bytes.length < 64) {
        throw new Error('File too small to be valid Z-Machine file');
    }

    const version = bytes[0];
    if (![3, 4, 5, 8].includes(version)) {
        throw new Error(`Unsupported Z-Machine version: ${version}`);
    }

    const release = (bytes[2] << 8) | bytes[3];
    const serial = String.fromCharCode(...bytes.slice(18, 24));
    const checksum = ((bytes[28] << 8) | bytes[29]).toString(16).padStart(4, '0');
    const fileSize = bytes.length;

    return { version, release, serial, checksum, fileSize };
}

// Generate game ID from filename
function generateGameId(filename) {
    return filename
        .toLowerCase()
        .replace(/\.(z3|z4|z5|z8)$/i, '')
        .replace(/[^a-z0-9]/g, '-')
        .replace(/-+/g, '-')
        .replace(/^-|-$/g, '');
}

// Navigate to admin panel
function showAdminPanel() {
    if (!isAdmin) {
        showFlashMessage('Admin access required', 'error');
        return;
    }

    loginSection.style.display = 'none';
    gameLibrary.style.display = 'none';
    gamePlayer.style.display = 'none';
    adminSection.style.display = 'block';

    // Load admin games list
    loadAdminGames();
}

// Navigate back to library from admin
function handleAdminBackToLibrary() {
    adminSection.style.display = 'none';
    gameLibrary.style.display = 'block';
    loadGameLibrary();
}

// Switch between admin tabs
function switchAdminTab(tabName) {
    // Update tab buttons
    adminTabButtons.forEach(btn => {
        if (btn.dataset.tab === tabName) {
            btn.classList.add('active');
        } else {
            btn.classList.remove('active');
        }
    });

    // Update tab content
    adminTabContents.forEach(content => {
        if (content.id === `admin-${tabName}-tab`) {
            content.classList.add('active');
            content.style.display = 'block';
        } else {
            content.classList.remove('active');
            content.style.display = 'none';
        }
    });

    // Reset upload form if switching away from upload tab
    if (tabName !== 'upload') {
        resetUploadForm();
    }
}

// Load and render admin games list
async function loadAdminGames() {
    try {
        const data = await adminGetAllGames();
        renderAdminGamesList(data.games);
    } catch (error) {
        showFlashMessage('Failed to load games: ' + error.message, 'error');
    }
}

// Render admin games list as table
function renderAdminGamesList(games) {
    if (!adminGamesList) return;

    if (games.length === 0) {
        adminGamesList.innerHTML = '<p class="empty-message">No games found</p>';
        return;
    }

    // Games are already sorted by display_order from backend (nulls at end, then by title)

    // Add reorder controls
    const controls = document.createElement('div');
    controls.className = 'reorder-controls';
    controls.innerHTML = `
        <button id="save-order-btn" class="primary" style="display:none;">Save New Order</button>
        <span id="reorder-hint" class="hint-text">Drag games to reorder</span>
    `;

    const table = document.createElement('table');
    table.className = 'admin-games-table';
    table.id = 'games-reorder-table';

    // Table header
    const thead = document.createElement('thead');
    thead.innerHTML = `
        <tr>
            <th class="drag-handle-col">⋮⋮</th>
            <th>Title</th>
            <th>Author</th>
            <th>Version</th>
            <th>Size</th>
            <th>Status</th>
            <th>Actions</th>
        </tr>
    `;
    table.appendChild(thead);

    // Table body
    const tbody = document.createElement('tbody');
    tbody.id = 'games-tbody';
    games.forEach((game, index) => {
        const row = document.createElement('tr');
        row.draggable = true;
        row.dataset.gameId = game.game_id;
        row.dataset.originalOrder = index;
        row.innerHTML = `
            <td class="drag-handle">⋮⋮</td>
            <td><strong>${escapeHtml(game.title)}</strong></td>
            <td>${escapeHtml(game.author)}</td>
            <td>v${game.version}.${game.release}</td>
            <td>${formatFileSize(game.file_size)}</td>
            <td>${game.archived ? '<span class="archived-badge">Archived</span>' : '<span class="active-badge">Active</span>'}</td>
            <td class="actions">
                <button class="btn-small btn-edit" data-game-id="${game.game_id}">Edit</button>
                <button class="btn-small btn-delete" data-game-id="${game.game_id}">Delete</button>
            </td>
        `;
        tbody.appendChild(row);
    });
    table.appendChild(tbody);

    adminGamesList.innerHTML = '';
    adminGamesList.appendChild(controls);
    adminGamesList.appendChild(table);

    // Setup drag-and-drop reordering
    setupDragAndDrop(tbody, games);

    // Add event listeners to action buttons
    adminGamesList.querySelectorAll('.btn-edit').forEach(btn => {
        btn.addEventListener('click', () => handleEditGame(btn.dataset.gameId));
    });
    adminGamesList.querySelectorAll('.btn-delete').forEach(btn => {
        btn.addEventListener('click', () => handleDeleteGameClick(btn.dataset.gameId));
    });
}

// Setup drag-and-drop for game reordering
function setupDragAndDrop(tbody, games) {
    let draggedRow = null;
    let orderChanged = false;

    tbody.querySelectorAll('tr').forEach(row => {
        row.addEventListener('dragstart', (e) => {
            draggedRow = row;
            row.classList.add('dragging');
            e.dataTransfer.effectAllowed = 'move';
        });

        row.addEventListener('dragend', (e) => {
            row.classList.remove('dragging');
            tbody.querySelectorAll('tr').forEach(r => r.classList.remove('drag-over'));
        });

        row.addEventListener('dragover', (e) => {
            e.preventDefault();
            e.dataTransfer.dropEffect = 'move';

            if (draggedRow && row !== draggedRow) {
                const rect = row.getBoundingClientRect();
                const midpoint = rect.top + rect.height / 2;

                if (e.clientY < midpoint) {
                    row.parentNode.insertBefore(draggedRow, row);
                } else {
                    row.parentNode.insertBefore(draggedRow, row.nextSibling);
                }
            }
        });

        row.addEventListener('drop', (e) => {
            e.preventDefault();
            orderChanged = true;
            const saveBtn = document.getElementById('save-order-btn');
            if (saveBtn) saveBtn.style.display = 'inline-block';
        });
    });

    // Save order button handler - attach after a brief delay to ensure DOM is ready
    setTimeout(() => {
        const saveBtn = document.getElementById('save-order-btn');
        if (saveBtn) {
            saveBtn.addEventListener('click', async () => {
                await saveNewOrder(tbody, games);
            });
        }
    }, 0);
}

// Save new game order
async function saveNewOrder(tbody, games) {
    try {
        const rows = Array.from(tbody.querySelectorAll('tr'));
        const updates = [];

        // Build update requests for all games with new display_order
        for (let i = 0; i < rows.length; i++) {
            const gameId = rows[i].dataset.gameId;
            const game = games.find(g => g.game_id === gameId);

            if (game) {
                updates.push({
                    game_id: gameId,
                    display_order: i,
                    title: game.title,
                    author: game.author,
                    description: game.description,
                    category: game.category || null,
                    year: game.year || null
                });
            }
        }

        // Show progress
        const saveBtn = document.getElementById('save-order-btn');
        const originalText = saveBtn.textContent;
        saveBtn.textContent = 'Saving...';
        saveBtn.disabled = true;

        // Send updates sequentially
        for (const update of updates) {
            await adminUpdateGame(update.game_id, update);
        }

        // Success
        saveBtn.style.display = 'none';
        saveBtn.disabled = false;
        saveBtn.textContent = originalText;
        showFlashMessage('Game order saved successfully!', 'success');

        // Reload games to reflect new order
        await loadAdminGames();

    } catch (error) {
        showFlashMessage('Failed to save order: ' + error.message, 'error');
        document.getElementById('save-order-btn').disabled = false;
    }
}

// Handle file upload
async function handleFileUpload(file) {
    try {
        // Validate file
        if (!file.name.match(/\.(z3|z4|z5|z8)$/i)) {
            throw new Error('Invalid file type. Only .z3, .z4, .z5, .z8 files are supported.');
        }

        if (file.size > 512 * 1024) {
            throw new Error('File too large. Maximum size is 512 KB.');
        }

        // Show progress
        uploadDropzone.style.display = 'none';
        uploadProgress.style.display = 'block';
        document.getElementById('upload-status').textContent = 'Reading file...';

        // Read file
        const arrayBuffer = await file.arrayBuffer();
        const bytes = new Uint8Array(arrayBuffer);

        // Extract metadata
        const metadata = extractZMachineMetadata(bytes);
        const gameId = generateGameId(file.name);

        // Update progress
        document.getElementById('upload-status').textContent = 'Getting upload URL...';

        // Get presigned URL
        const uploadData = await adminGetUploadUrl(file.name);

        // Update progress
        document.getElementById('upload-status').textContent = 'Uploading file...';

        // Upload to S3
        const uploadResponse = await fetch(uploadData.upload_url, {
            method: 'PUT',
            body: file,
            headers: {
                'Content-Type': 'application/octet-stream'
            }
        });

        if (!uploadResponse.ok) {
            throw new Error('Failed to upload file to S3');
        }

        // Hide progress, show metadata form
        uploadProgress.style.display = 'none';
        metadataForm.style.display = 'block';

        // Store upload data for later
        currentUploadData = {
            gameId,
            s3Key: uploadData.s3_key,
            ...metadata
        };

        // Pre-fill form
        document.getElementById('game-id').value = gameId;
        document.getElementById('meta-version').textContent = metadata.version;
        document.getElementById('meta-release').textContent = metadata.release;
        document.getElementById('meta-serial').textContent = metadata.serial;
        document.getElementById('meta-checksum').textContent = metadata.checksum;
        document.getElementById('meta-filesize').textContent = formatFileSize(metadata.fileSize);

        // Clear editable fields
        document.getElementById('game-title').value = '';
        document.getElementById('game-author').value = '';
        document.getElementById('game-description').value = '';
        document.getElementById('game-category').value = '';
        document.getElementById('game-year').value = '';

        showFlashMessage('File uploaded successfully. Please enter game metadata.', 'success');

    } catch (error) {
        uploadProgress.style.display = 'none';
        uploadDropzone.style.display = 'block';
        showFlashMessage('Upload failed: ' + error.message, 'error');
    }
}

// Handle publish game
async function handlePublishGame() {
    if (!currentUploadData) {
        showFlashMessage('No file uploaded', 'error');
        return;
    }

    try {
        // Get form data
        const title = document.getElementById('game-title').value.trim();
        const author = document.getElementById('game-author').value.trim();
        const description = document.getElementById('game-description').value.trim();
        const category = document.getElementById('game-category').value || null;
        const year = document.getElementById('game-year').value ? parseInt(document.getElementById('game-year').value) : null;

        // Validate
        if (!title || !author || !description) {
            showFlashMessage('Please fill in all required fields', 'error');
            return;
        }

        // Create game
        const gameData = {
            game_id: currentUploadData.gameId,
            title,
            author,
            description,
            category,
            year_published: year,
            s3_key: currentUploadData.s3Key,
            file_size: currentUploadData.fileSize,
            version: currentUploadData.version,
            release: currentUploadData.release,
            serial: currentUploadData.serial,
            checksum: currentUploadData.checksum
        };

        await adminCreateGame(gameData);

        showFlashMessage('Game published successfully!', 'success');
        resetUploadForm();
        switchAdminTab('games');
        loadAdminGames();

    } catch (error) {
        showFlashMessage('Failed to publish game: ' + error.message, 'error');
    }
}

// Reset upload form
function resetUploadForm() {
    currentUploadData = null;
    uploadDropzone.style.display = 'block';
    uploadProgress.style.display = 'none';
    metadataForm.style.display = 'none';
    fileInput.value = '';

    // Clear form
    ['game-title', 'game-author', 'game-description', 'game-category', 'game-year'].forEach(id => {
        const el = document.getElementById(id);
        if (el) el.value = '';
    });
}

// Handle edit game
async function handleEditGame(gameId) {
    try {
        editingGameId = gameId;

        // Fetch game data
        const response = await fetch(`${API_BASE}/api/admin/games/${gameId}`, {
            headers: { 'Authorization': `Bearer ${accessToken}` }
        });

        if (!response.ok) {
            throw new Error('Failed to fetch game data');
        }

        const game = await response.json();

        // Pre-fill edit form
        document.getElementById('edit-game-title').value = game.title;
        document.getElementById('edit-game-author').value = game.author;
        document.getElementById('edit-game-description').value = game.description;
        document.getElementById('edit-game-category').value = game.category || '';
        document.getElementById('edit-game-year').value = game.year || '';
        document.getElementById('edit-game-display-order').value = game.display_order !== null && game.display_order !== undefined ? game.display_order : '';

        // Show modal
        editGameModal.style.display = 'flex';

    } catch (error) {
        showFlashMessage('Failed to load game: ' + error.message, 'error');
    }
}

// Handle save edit
async function handleSaveEdit() {
    if (!editingGameId) return;

    try {
        const title = document.getElementById('edit-game-title').value.trim();
        const author = document.getElementById('edit-game-author').value.trim();
        const description = document.getElementById('edit-game-description').value.trim();
        const category = document.getElementById('edit-game-category').value || null;
        const year = document.getElementById('edit-game-year').value ? parseInt(document.getElementById('edit-game-year').value) : null;
        const displayOrderValue = document.getElementById('edit-game-display-order').value.trim();
        const display_order = displayOrderValue ? parseInt(displayOrderValue) : null;

        if (!title || !author || !description) {
            showFlashMessage('Please fill in all required fields', 'error');
            return;
        }

        const gameData = {
            title,
            author,
            description,
            category,
            year,
            display_order
        };

        await adminUpdateGame(editingGameId, gameData);

        showFlashMessage('Game updated successfully!', 'success');
        editGameModal.style.display = 'none';
        editingGameId = null;
        loadAdminGames();

    } catch (error) {
        showFlashMessage('Failed to update game: ' + error.message, 'error');
    }
}

// Handle delete game click
function handleDeleteGameClick(gameId) {
    deletingGameId = gameId;

    // Find game title
    const gameTitle = adminGamesList.querySelector(`[data-game-id="${gameId}"]`)?.closest('tr')?.querySelector('strong')?.textContent || gameId;

    document.getElementById('delete-game-name').textContent = gameTitle;
    deleteGameModal.style.display = 'flex';
}

// Handle confirm delete
async function handleConfirmDelete() {
    if (!deletingGameId) return;

    try {
        await adminDeleteGame(deletingGameId);

        showFlashMessage('Game deleted successfully!', 'success');
        deleteGameModal.style.display = 'none';
        deletingGameId = null;
        loadAdminGames();

    } catch (error) {
        showFlashMessage('Failed to delete game: ' + error.message, 'error');
    }
}

// Utility: Format file size
function formatFileSize(bytes) {
    if (bytes < 1024) return bytes + ' B';
    if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(1) + ' KB';
    return (bytes / (1024 * 1024)).toFixed(1) + ' MB';
}

// Utility: Escape HTML
function escapeHtml(text) {
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
}

// Start the app
initApp();
