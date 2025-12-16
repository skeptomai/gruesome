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
  DISCLAIMER: 'disclaimer',
  LOADING: 'loading',
  LOADING_GAME: 'loading_game',
  DEMO: 'demo',
  PLAYING: 'playing',
  ERROR: 'error',
};

/**
 * Theme Toggle Component
 */
function ThemeToggle({ theme, onToggle }) {
  return html`
    <div class="theme-toggle">
      <button
        class="theme-button ${theme === 'green' ? 'active' : ''}"
        onClick=${() => onToggle('green')}
      >Green</button>
      <button
        class="theme-button ${theme === 'amber' ? 'active' : ''}"
        onClick=${() => onToggle('amber')}
      >Amber</button>
    </div>
  `;
}

/**
 * Font Toggle Component
 */
function FontToggle({ font, onToggle }) {
  return html`
    <div class="font-toggle">
      <button
        class="font-button ${font === 'default' ? 'active' : ''}"
        onClick=${() => onToggle('default')}
      >Default</button>
      <button
        class="font-button ${font === 'vt323' ? 'active' : ''}"
        onClick=${() => onToggle('vt323')}
      >VT323</button>
      <button
        class="font-button ${font === 'ibm3270' ? 'active' : ''}"
        onClick=${() => onToggle('ibm3270')}
      >IBM 3270</button>
      <button
        class="font-button ${font === 'sharetech' ? 'active' : ''}"
        onClick=${() => onToggle('sharetech')}
      >Share Tech</button>
    </div>
  `;
}

/**
 * CRT Effects Toggle Component
 */
function CrtToggle({ crtEnabled, onToggle }) {
  return html`
    <div class="crt-toggle">
      <button
        class="theme-button ${!crtEnabled ? 'active' : ''}"
        onClick=${() => onToggle(false)}
      >CRT Off</button>
      <button
        class="theme-button ${crtEnabled ? 'active' : ''}"
        onClick=${() => onToggle(true)}
      >CRT On</button>
    </div>
  `;
}

/**
 * Blur Intensity Toggle Component
 * Controls the amount of phosphor blur in CRT mode
 */
function BlurToggle({ blurLevel, onToggle, disabled }) {
  const levels = [
    { id: 'none', label: 'Sharp' },
    { id: 'light', label: 'Light' },
    { id: 'medium', label: 'Medium' },
    { id: 'heavy', label: 'Heavy' },
  ];

  return html`
    <div class="blur-toggle ${disabled ? 'disabled' : ''}">
      <span class="blur-label">Blur:</span>
      ${levels.map(level => html`
        <button
          class="blur-button ${blurLevel === level.id ? 'active' : ''}"
          onClick=${() => !disabled && onToggle(level.id)}
          disabled=${disabled}
        >${level.label}</button>
      `)}
    </div>
  `;
}

/**
 * Disclaimer Component
 * Shown before loading the game with legal notices
 */
function Disclaimer({ onContinue, onLoadOwn, theme, onThemeChange, font, onFontChange, crtEnabled, onCrtChange, blurLevel, onBlurChange, version }) {
  const crtClass = crtEnabled ? `crt-enhanced crt-blur-${blurLevel || 'medium'}` : '';
  return html`
    <div class="terminal theme-${theme} font-${font} ${crtClass}">
      <div class="disclaimer">
        <h1 class="disclaimer-title">GRUESOME</h1>
        <p class="disclaimer-subtitle">Z-Machine Interpreter${version ? ` v${version}` : ''}</p>

        <${ThemeToggle} theme=${theme} onToggle=${onThemeChange} />
        <${FontToggle} font=${font} onToggle=${onFontChange} />
        <${CrtToggle} crtEnabled=${crtEnabled} onToggle=${onCrtChange} />
        <${BlurToggle} blurLevel=${blurLevel || 'medium'} onToggle=${onBlurChange} disabled=${!crtEnabled} />

        <div class="disclaimer-section">
          <h2>About This Project</h2>
          <p>
            This is an educational project demonstrating Z-Machine interpreter
            implementation in Rust, compiled to WebAssembly for in-browser gameplay.
          </p>
        </div>

        <div class="disclaimer-section">
          <h2>Legal Notice</h2>
          <p>
            Zork I and other Infocom games are copyrighted by Activision Publishing, Inc.
            This site is not affiliated with or endorsed by Activision.
          </p>
          <p>
            This interpreter is provided for educational and preservation purposes.
          </p>
        </div>

        <div class="disclaimer-section">
          <h2>Acquire Zork Legally</h2>
          <ul class="disclaimer-links">
            <li><a href="https://www.gog.com/en/game/the_zork_anthology" target="_blank" rel="noopener">GOG.com - The Zork Anthology</a></li>
            <li><a href="https://store.steampowered.com/app/570580/Zork_Anthology/" target="_blank" rel="noopener">Steam - Zork Anthology</a></li>
            <li><a href="https://www.infocom-if.org/games/games.html" target="_blank" rel="noopener">Infocom-IF.org - Game Information</a></li>
          </ul>
        </div>

        <div class="disclaimer-section">
          <h2>Free Alternatives</h2>
          <p>
            The <a href="https://www.ifarchive.org/" target="_blank" rel="noopener">Interactive Fiction Archive</a> hosts
            hundreds of free Z-Machine games you can play legally.
          </p>
        </div>

        <div class="disclaimer-cta">
          <button class="play-button" onClick=${onContinue}>
            Play Zork I
          </button>
          <p class="play-hint">Click to start the classic text adventure</p>
        </div>

        <div class="disclaimer-alt">
          <button class="disclaimer-button secondary" onClick=${onLoadOwn}>
            Or Load Your Own Game File
          </button>
        </div>

        <p class="disclaimer-footer">
          By continuing, you acknowledge this notice and accept responsibility
          for your use of any copyrighted material.
        </p>

        <p class="disclaimer-footer">
          <a href="https://github.com/skeptomai/gruesome" target="_blank" rel="noopener">View source on GitHub</a>
        </p>
      </div>
    </div>
  `;
}

/**
 * Main Application Component
 */
function App() {
  const { useState, useEffect, useRef } = window.preact;

  const [appState, setAppState] = useState(AppState.DISCLAIMER);
  const [loadingMessage, setLoadingMessage] = useState('Loading Gruesome Z-Machine...');
  const [error, setError] = useState(null);
  const [loadOwnGame, setLoadOwnGame] = useState(false);

  // WASM interpreter instance
  const interpreterRef = useRef(null);

  // Interpreter version - fetch from WASM on load
  const [interpreterVersion, setInterpreterVersion] = useState(null);

  // Game state (when playing with real WASM)
  const [gameState, setGameState] = useState({
    status: { location: '', score: 0, moves: 0 },
    outputLines: [],
    waitingForInput: false,
    quit: false,
  });

  // Load WASM and get version on app startup
  useEffect(() => {
    async function loadVersion() {
      try {
        const response = await fetch('./pkg/gruesome.js', { method: 'HEAD' });
        if (response.ok) {
          const wasm = await import('../pkg/gruesome.js');
          await wasm.default();
          const version = wasm.get_interpreter_version();
          setInterpreterVersion(version);
          console.log('Gruesome version:', version);
        }
      } catch (err) {
        console.log('Could not load version:', err);
      }
    }
    loadVersion();
  }, []);

  // Settings - load from localStorage if available
  const [settings, setSettings] = useState(() => {
    const saved = localStorage.getItem('gruesome-settings');
    if (saved) {
      try {
        return JSON.parse(saved);
      } catch (e) {
        // Ignore parse errors
      }
    }
    return { theme: 'green', font: 'default', crtEnabled: false, effectsEnabled: true, blurLevel: 'medium' };
  });

  // Save settings to localStorage when they change
  function updateSettings(newSettings) {
    setSettings(newSettings);
    localStorage.setItem('gruesome-settings', JSON.stringify(newSettings));
  }

  function handleThemeChange(newTheme) {
    updateSettings({ ...settings, theme: newTheme });
  }

  function handleFontChange(newFont) {
    updateSettings({ ...settings, font: newFont });
  }

  function handleCrtChange(enabled) {
    updateSettings({ ...settings, crtEnabled: enabled });
  }

  function handleBlurChange(level) {
    updateSettings({ ...settings, blurLevel: level });
  }

  async function handleDisclaimerContinue() {
    setLoadOwnGame(false);
    setAppState(AppState.LOADING);
    await initializeApp(false);
  }

  async function handleLoadOwnGame() {
    setLoadOwnGame(true);
    setAppState(AppState.LOADING);
    await initializeApp(true);
  }

  async function initializeApp(skipDefaultGame = false) {
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

        if (skipDefaultGame) {
          // Go directly to file picker
          setAppState(AppState.LOADING_GAME);
          setLoadingMessage('Select a game file to play...');
        } else {
          // Try to load the default Zork I game
          setLoadingMessage('Loading Zork I...');
          const gameData = await tryLoadDefaultGame();
          if (gameData) {
            await startGame(wasm, gameData);
          } else {
            // No default game, show file picker
            setAppState(AppState.LOADING_GAME);
            setLoadingMessage('Zork I not found. Select a game file...');
          }
        }
      } else {
        // Fall back to demo mode
        console.log('WASM not available, running in demo mode');
        setAppState(AppState.DEMO);
      }
    } catch (err) {
      console.error('Failed to initialize:', err);
      setError({ message: 'Failed to initialize: ' + err.message });
      setAppState(AppState.ERROR);
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
      // Try to load the default Zork I game file
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
        waitingForInput: result.needs_input || result.needs_restore_data,
        quit: result.quit,
      }));

      // Handle save data - trigger download
      const saveData = result.save_data;
      if (saveData && saveData.length > 0) {
        downloadSaveFile(saveData);
      }

      // Handle restore request - show file picker
      if (result.needs_restore_data) {
        showRestoreFilePicker();
      }

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

  // Download save file to user's computer
  function downloadSaveFile(data) {
    const blob = new Blob([data], { type: 'application/octet-stream' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = 'gruesome_save.qzl';
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
  }

  // Show file picker for restore
  function showRestoreFilePicker() {
    const input = document.createElement('input');
    input.type = 'file';
    input.accept = '.qzl,.sav';
    input.onchange = async (e) => {
      const file = e.target.files[0];
      if (file) {
        try {
          const arrayBuffer = await file.arrayBuffer();
          const data = new Uint8Array(arrayBuffer);
          interpreterRef.current.provide_restore_data(data);
          runInterpreterStep();
        } catch (err) {
          console.error('Failed to load save file:', err);
          interpreterRef.current.cancel_restore();
          runInterpreterStep();
        }
      } else {
        // User cancelled
        interpreterRef.current.cancel_restore();
        runInterpreterStep();
      }
    };
    input.click();
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
      setAppState(AppState.LOADING);
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
    case AppState.DISCLAIMER:
      return html`
        <${Disclaimer}
          onContinue=${handleDisclaimerContinue}
          onLoadOwn=${handleLoadOwnGame}
          theme=${settings.theme}
          onThemeChange=${handleThemeChange}
          font=${settings.font || 'default'}
          onFontChange=${handleFontChange}
          crtEnabled=${settings.crtEnabled || false}
          onCrtChange=${handleCrtChange}
          blurLevel=${settings.blurLevel || 'medium'}
          onBlurChange=${handleBlurChange}
          version=${interpreterVersion}
        />
      `;

    case AppState.LOADING:
      return html`<${Loading} message=${loadingMessage} />`;

    case AppState.LOADING_GAME:
      const crtClass = settings.crtEnabled ? `crt-enhanced crt-blur-${settings.blurLevel || 'medium'}` : '';
      return html`
        <div class="terminal theme-${settings.theme} font-${settings.font || 'default'} ${crtClass}">
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
          font=${settings.font || 'default'}
          crtEnabled=${settings.crtEnabled || false}
          effectsEnabled=${settings.effectsEnabled}
          blurLevel=${settings.blurLevel || 'medium'}
          interpreterVersion=${interpreterVersion}
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
