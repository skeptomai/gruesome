// Gruesome Z-Machine Platform - Frontend Application
// Web-based Z-Machine game player using WASM interpreter
// Supports authentication, game library, and cloud save/load functionality

import init, { WasmInterpreter } from './gruesome.js';

// Configuration
const API_BASE = 'https://api.gruesome.skeptomai.com';

// Application State
let accessToken = null;        // JWT authentication token
let currentGame = null;        // Currently loaded game ID
let wasmInterpreter = null;    // WASM Z-Machine interpreter instance
let authMode = 'login';        // Authentication mode: 'login', 'signup', 'reset', or 'confirm-reset'

// DOM Elements - initialized after DOM is ready
let loginSection, gameLibrary, gamePlayer, authStatus, gamesList, gameOutput, gameInput, logoutButton;
let emailInput, usernameInput, passwordInput, authSubmit, toggleAuthLink, forgotPasswordLink;
let resetCodeInput, newPasswordInput;

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
    gameInput = document.getElementById('game-input');
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

    // Set up game input handler
    if (gameInput) {
        gameInput.addEventListener('keypress', handleGameInput);
    }

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
    // Hide all optional fields first
    emailInput.style.display = 'none';
    resetCodeInput.style.display = 'none';
    newPasswordInput.style.display = 'none';
    emailInput.required = false;
    resetCodeInput.required = false;
    newPasswordInput.required = false;

    if (authMode === 'signup') {
        // Signup mode: show email
        emailInput.style.display = 'block';
        emailInput.required = true;
        passwordInput.style.display = 'block';
        passwordInput.required = true;
        usernameInput.style.display = 'block';
        usernameInput.required = true;
        authSubmit.textContent = 'Create Account';
        toggleAuthLink.textContent = 'Already have an account? Login';
        forgotPasswordLink.style.display = 'inline';
    } else if (authMode === 'reset') {
        // Password reset step 1: username only
        usernameInput.style.display = 'block';
        usernameInput.required = true;
        passwordInput.style.display = 'none';
        passwordInput.required = false;
        authSubmit.textContent = 'Send Reset Code';
        toggleAuthLink.textContent = 'Back to Login';
        forgotPasswordLink.style.display = 'none';
    } else if (authMode === 'confirm-reset') {
        // Password reset step 2: username, code, new password
        usernameInput.style.display = 'block';
        usernameInput.required = true;
        passwordInput.style.display = 'none';
        passwordInput.required = false;
        resetCodeInput.style.display = 'block';
        resetCodeInput.required = true;
        newPasswordInput.style.display = 'block';
        newPasswordInput.required = true;
        authSubmit.textContent = 'Reset Password';
        toggleAuthLink.textContent = 'Back to Login';
        forgotPasswordLink.style.display = 'none';
    } else {
        // Login mode
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
        alert('Account created successfully! Logging you in...');
        await handleLogin();
    } catch (error) {
        // Check if it's a "user already exists" error
        if (error.message && error.message.toLowerCase().includes('already exists')) {
            alert('This username or email is already registered. Try logging in instead.');
            // Switch to login mode
            authMode = 'login';
            updateAuthUI();
        } else {
            alert('Signup failed: ' + error.message);
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
        alert('Password reset code sent to your email. Please check your inbox.');
        authMode = 'confirm-reset';
        updateAuthUI();
    } catch (error) {
        alert('Failed to send reset code: ' + error.message);
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
        alert('Password reset successfully! Please login with your new password.');
        authMode = 'login';
        resetCodeInput.value = '';
        newPasswordInput.value = '';
        updateAuthUI();
    } catch (error) {
        alert('Failed to reset password: ' + error.message);
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
        alert('Login failed: ' + error.message);
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

        // Show game player
        gameLibrary.style.display = 'none';
        gamePlayer.style.display = 'block';

        // Start game - step until it needs input
        runUntilInput();
        gameInput.focus();

    } catch (error) {
        alert('Failed to load game: ' + error.message);
    }
};

// Run interpreter until it needs input
function runUntilInput() {
    let result;
    do {
        result = wasmInterpreter.step();
        if (result.output) {
            gameOutput.textContent += result.output;
            gameOutput.scrollTop = gameOutput.scrollHeight;
        }
        if (result.error) {
            console.error('Game error:', result.error);
        }
        if (result.quit) {
            gameOutput.textContent += '\n\n[Game Over]';
            break;
        }
    } while (!result.needs_input && !result.quit);
}

// Game Input
function handleGameInput(e) {
    if (e.key === 'Enter' && gameInput.value.trim()) {
        const command = gameInput.value.trim();
        gameInput.value = '';

        // Display command
        gameOutput.textContent += `\n> ${command}\n\n`;
        gameOutput.scrollTop = gameOutput.scrollHeight;

        // Provide input to interpreter
        wasmInterpreter.provide_input(command);

        // Run until next input needed
        runUntilInput();
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

        alert('Game saved successfully!');
    } catch (error) {
        alert('Failed to save game: ' + error.message);
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
            alert('No saves found for this game');
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

        // Run until input needed to show current game state
        runUntilInput();

    } catch (error) {
        alert('Failed to load save: ' + error.message);
    }
}

// Start the app
initApp();
