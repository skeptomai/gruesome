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

// Visual Settings State - Retro terminal styling
let visualSettings = {
    theme: 'green',
    font: 'default',
    crtEnabled: false,
    blurLevel: 'medium',
    collapsed: false,
    controlsCollapsed: false
};

// DOM Elements - initialized after DOM is ready
let loginSection, gameLibrary, gamePlayer, authStatus, gamesList, gameOutput, gameInput, logoutButton;
let emailInput, usernameInput, passwordInput, authSubmit, toggleAuthLink, forgotPasswordLink;
let resetCodeInput, newPasswordInput;

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
    authStatus = document.getElementById('auth-status');
    gamesList = document.getElementById('games-list');
    gameOutput = document.getElementById('game-output');
    gameInput = null;  // Will be created dynamically when game loads
    logoutButton = document.getElementById('logout-button');

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

    // Initialize WASM
    await init();

    // Check for existing token
    const savedToken = localStorage.getItem('accessToken');
    if (savedToken) {
        accessToken = savedToken;
        await loadGameLibrary();
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
    } else if (authMode === 'reset') {
        // Password reset step 1: username only
        usernameInput.style.display = 'block';
        usernameInput.required = true;
        authSubmit.textContent = 'Send Reset Code';
        toggleAuthLink.textContent = 'Back to Login';
        forgotPasswordLink.style.display = 'none';
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
    } else {
        // Login mode: username + password
        usernameInput.style.display = 'block';
        usernameInput.required = true;
        passwordInput.style.display = 'block';
        passwordInput.required = true;
        authSubmit.textContent = 'Login';
        toggleAuthLink.textContent = 'Need an account? Sign up';
        forgotPasswordLink.style.display = 'inline';
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
        // Prompt stays visible - no need to hide/show
    }
}

// Load and Start Game
window.loadGame = async function(gameId) {
    try {
        // Get download URL
        const response = await fetch(`${API_BASE}/api/games/${gameId}/file`);
        const data = await response.json();

        // Download game file
        const gameResponse = await fetch(data.download_url);
        const gameData = await gameResponse.arrayBuffer();

        // Initialize WASM interpreter with game data
        wasmInterpreter = new WasmInterpreter(new Uint8Array(gameData));
        currentGame = gameId;

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
        runUntilInput();

        // Create input area after game text
        createInputArea();

        // Focus input
        if (gameInput) gameInput.focus();

    } catch (error) {
        showFlashMessage('Failed to load game: ' + error.message, 'error');
    }
};

// Run interpreter until it needs input
function runUntilInput() {
    let result;
    do {
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
            // Disable Save and Load buttons when game ends
            const saveButton = document.getElementById('save-button');
            const loadButton = document.getElementById('load-button');
            if (saveButton) saveButton.disabled = true;
            if (loadButton) loadButton.disabled = true;
            break;
        }
    } while (!result.needs_input && !result.quit);
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

        // Upload save file to S3
        await fetch(data.upload_url, {
            method: 'PUT',
            body: saveData
        });

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

        // Download save file
        const saveResponse = await fetch(downloadData.download_url);
        const saveData = new Uint8Array(await saveResponse.arrayBuffer());

        // Restore save state
        wasmInterpreter.restore_game(saveData);
        gameOutput.textContent = '\n[Save loaded successfully!]\n\n';

        // Enable Save and Load buttons (in case they were disabled from previous game quit)
        const saveButton = document.getElementById('save-button');
        const loadButton = document.getElementById('load-button');
        if (saveButton) saveButton.disabled = false;
        if (loadButton) loadButton.disabled = false;

        // Run until input needed to show current game state
        runUntilInput();

        // Recreate input area after restore
        createInputArea();

        // Focus input
        if (gameInput) gameInput.focus();

    } catch (error) {
        showFlashMessage('Failed to load save: ' + error.message, 'error');
    }
}

// Start the app
initApp();
