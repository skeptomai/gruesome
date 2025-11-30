/**
 * Gruesome Z-Machine Web Interface
 *
 * Main entry point for the web application.
 * Handles WASM loading and application initialization.
 */

const { render, html } = window.preact;
const { DemoTerminal, Loading, ErrorDisplay, Terminal } = window.GruesomeTerminal;

/**
 * Application State
 */
const AppState = {
  LOADING: 'loading',
  LOADING_GAME: 'loading_game',
  DEMO: 'demo',
  PLAYING: 'playing',
  ERROR: 'error',
};

/**
 * Main Application Component
 */
function App() {
  const { useState, useEffect, useRef } = window.preact;

  const [appState, setAppState] = useState(AppState.LOADING);
  const [loadingMessage, setLoadingMessage] = useState('Loading Gruesome Z-Machine...');
  const [error, setError] = useState(null);

  // WASM interpreter instance
  const interpreterRef = useRef(null);

  // Game state (when playing with real WASM)
  const [gameState, setGameState] = useState({
    status: { location: '', score: 0, moves: 0 },
    outputLines: [],
    waitingForInput: false,
    quit: false,
  });

  // Settings
  const [settings, setSettings] = useState({
    theme: 'green',
    effectsEnabled: true,
  });

  useEffect(() => {
    initializeApp();
  }, []);

  async function initializeApp() {
    try {
      // Check if WASM module is available
      const wasmAvailable = await checkWasmAvailable();

      if (wasmAvailable) {
        setLoadingMessage('Loading WASM module...');
        // Load WASM module
        const wasm = await loadWasmModule();

        // Initialize panic hook for better error messages
        wasm.init();

        // Store wasm module reference
        window.gruesomeWasm = wasm;

        // Now we need game data - check for a default game or prompt user
        setAppState(AppState.LOADING_GAME);
        setLoadingMessage('Select a game file to play...');

        // Try to load a default game file
        const gameData = await tryLoadDefaultGame();
        if (gameData) {
          await startGame(wasm, gameData);
        } else {
          // Stay in LOADING_GAME state to show file picker
          console.log('No default game found, showing file picker');
          // State is already LOADING_GAME, which shows the file picker
        }
      } else {
        // Fall back to demo mode
        console.log('WASM not available, running in demo mode');
        setAppState(AppState.DEMO);
      }
    } catch (err) {
      console.error('Failed to initialize:', err);
      // Fall back to demo mode on error
      setAppState(AppState.DEMO);
    }
  }

  async function checkWasmAvailable() {
    try {
      // Check if the WASM package exists
      const response = await fetch('./pkg/gruesome.js', { method: 'HEAD' });
      return response.ok;
    } catch {
      return false;
    }
  }

  async function loadWasmModule() {
    // Dynamic import of the WASM module
    // Path is relative to the HTML file, not this JS file
    const wasm = await import('../pkg/gruesome.js');
    await wasm.default(); // Initialize WASM
    return wasm;
  }

  async function tryLoadDefaultGame() {
    try {
      // Try to load a default game file from the games directory
      const response = await fetch('./games/zork1.z3');
      if (response.ok) {
        const arrayBuffer = await response.arrayBuffer();
        return new Uint8Array(arrayBuffer);
      }
    } catch {
      // No default game available
    }
    return null;
  }

  async function startGame(wasm, gameData) {
    try {
      setLoadingMessage('Starting game...');

      // Create interpreter instance
      const interpreter = new wasm.WasmInterpreter(gameData);
      interpreterRef.current = interpreter;

      console.log('Game loaded, version:', interpreter.version);

      // Run initial step to get opening text
      setAppState(AppState.PLAYING);
      runInterpreterStep();

    } catch (err) {
      console.error('Failed to start game:', err);
      setError({ message: 'Failed to start game: ' + err.message });
      setAppState(AppState.ERROR);
    }
  }

  function runInterpreterStep() {
    const interpreter = interpreterRef.current;
    if (!interpreter) return;

    try {
      // Run interpreter until it needs input or finishes
      const result = interpreter.step();

      // Parse output - split by newlines
      const newOutput = result.output ? result.output.split('\n') : [];

      // Update game state from result
      setGameState(prev => ({
        status: {
          location: result.status_location || prev.status.location,
          score: result.status_score,
          moves: result.status_moves,
        },
        outputLines: [...prev.outputLines, ...newOutput],
        waitingForInput: result.needs_input,
        quit: result.quit,
      }));

      // Check for errors
      if (result.error) {
        console.error('Interpreter error:', result.error);
        setError({ message: result.error });
        setAppState(AppState.ERROR);
      }

      if (result.quit) {
        // Game ended
        setGameState(prev => ({
          ...prev,
          outputLines: [...prev.outputLines, '', '[Game ended]'],
          waitingForInput: false,
        }));
      }

      // Free the result object
      result.free();

    } catch (err) {
      console.error('Interpreter error:', err);
      setError({ message: err.message });
      setAppState(AppState.ERROR);
    }
  }

  function handleCommand(command) {
    if (appState !== AppState.PLAYING) return;

    const interpreter = interpreterRef.current;
    if (!interpreter) return;

    try {
      // Send command to WASM interpreter
      interpreter.provide_input(command);

      // Run interpreter to process command
      runInterpreterStep();

    } catch (err) {
      console.error('Error executing command:', err);
      setError({ message: err.message });
      setAppState(AppState.ERROR);
    }
  }

  // Handle file upload
  async function handleFileSelect(event) {
    const file = event.target.files[0];
    if (!file) return;

    try {
      setAppState(AppState.LOADING_GAME);
      setLoadingMessage('Loading ' + file.name + '...');

      const arrayBuffer = await file.arrayBuffer();
      const gameData = new Uint8Array(arrayBuffer);

      // Reset game state
      setGameState({
        status: { location: '', score: 0, moves: 0 },
        outputLines: [],
        waitingForInput: false,
        quit: false,
      });

      await startGame(window.gruesomeWasm, gameData);

    } catch (err) {
      console.error('Failed to load file:', err);
      setError({ message: 'Failed to load game file: ' + err.message });
      setAppState(AppState.ERROR);
    }
  }

  // Render based on current state
  switch (appState) {
    case AppState.LOADING:
      return html`<${Loading} message=${loadingMessage} />`;

    case AppState.LOADING_GAME:
      return html`
        <div class="terminal theme-green">
          <div class="loading">
            <div class="message">${loadingMessage}</div>
            <div class="file-upload-container">
              <label class="file-upload-label">
                <input
                  type="file"
                  accept=".z1,.z2,.z3,.z4,.z5,.z6,.z7,.z8,.dat,.DAT"
                  onChange=${handleFileSelect}
                />
                <span class="file-upload-button">Choose Game File</span>
              </label>
              <p class="file-upload-hint">Load a Z-Machine game file (.z3, .z5, etc.)</p>
            </div>
          </div>
        </div>
      `;

    case AppState.ERROR:
      return html`
        <${ErrorDisplay}
          title="Error"
          message=${error?.message || 'An unexpected error occurred'}
          details=${error?.details}
        />
      `;

    case AppState.DEMO:
      return html`<${DemoTerminal} />`;

    case AppState.PLAYING:
      return html`
        <${Terminal}
          status=${gameState.status}
          outputLines=${gameState.outputLines}
          waitingForInput=${gameState.waitingForInput}
          onCommand=${handleCommand}
          theme=${settings.theme}
          effectsEnabled=${settings.effectsEnabled}
        />
      `;

    default:
      return html`<${Loading} />`;
  }
}

/**
 * Initialize the application when DOM is ready
 */
function init() {
  const appRoot = document.getElementById('app');

  if (!appRoot) {
    console.error('Could not find #app element');
    return;
  }

  render(html`<${App} />`, appRoot);

  console.log('Gruesome Z-Machine initialized');
}

// Start the app
if (document.readyState === 'loading') {
  document.addEventListener('DOMContentLoaded', init);
} else {
  init();
}

// Export for debugging
window.GruesomeApp = {
  AppState,
  init,
};
