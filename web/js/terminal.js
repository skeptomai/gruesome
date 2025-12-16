/**
 * Gruesome Z-Machine Terminal Component
 *
 * Preact component for rendering the terminal interface.
 * Handles output display, user input, and status bar.
 */

const { h, useState, useEffect, useRef, html } = window.preact;

/**
 * Status Bar Component
 * Displays location, score/moves (or time for some games)
 */
export function StatusBar({ location, score, moves, time, isTimeGame }) {
  return html`
    <div class="status-bar">
      <span class="location">${location || ''}</span>
      ${isTimeGame
        ? html`<span class="time">${time || ''}</span>`
        : html`<span class="score-moves">Score: ${score ?? 0}  Moves: ${moves ?? 0}</span>`
      }
    </div>
  `;
}

/**
 * Output Area Component
 * Displays game output with auto-scroll
 * Now includes the input area inline at the bottom for traditional terminal feel
 */
export function OutputArea({ lines, upperWindow, waitingForInput, onSubmit, prompt }) {
  const outputRef = useRef(null);

  // Auto-scroll to bottom when new content is added or input state changes
  useEffect(() => {
    if (outputRef.current) {
      outputRef.current.scrollTop = outputRef.current.scrollHeight;
    }
  }, [lines, waitingForInput]);

  return html`
    ${upperWindow && upperWindow.length > 0 && html`
      <div class="upper-window">
        ${upperWindow.map((line, i) => html`<pre key=${i}>${line}</pre>`)}
      </div>
    `}
    <div class="output-area" ref=${outputRef}>
      ${lines.map((line, i) => html`<pre key=${i}>${line}</pre>`)}
      ${waitingForInput && html`
        <${InputArea}
          onSubmit=${onSubmit}
          disabled=${!waitingForInput}
          prompt=${prompt}
        />
      `}
    </div>
  `;
}

/**
 * Input Area Component
 * Text input with command history support
 */
export function InputArea({ onSubmit, disabled, prompt }) {
  const [value, setValue] = useState('');
  const [history, setHistory] = useState([]);
  const [historyIndex, setHistoryIndex] = useState(-1);
  const inputRef = useRef(null);

  // Focus input on mount and when re-enabled
  useEffect(() => {
    if (!disabled && inputRef.current) {
      inputRef.current.focus();
    }
  }, [disabled]);

  const handleKeyDown = (e) => {
    if (e.key === 'Enter') {
      // Add non-empty commands to history
      if (value.trim()) {
        setHistory(prev => [...prev, value]);
      }
      setHistoryIndex(-1);

      // Submit command (including empty for "press Enter to continue")
      onSubmit(value);
      setValue('');
    } else if (e.key === 'ArrowUp') {
      // Navigate history backwards
      e.preventDefault();
      if (history.length > 0) {
        const newIndex = historyIndex === -1
          ? history.length - 1
          : Math.max(0, historyIndex - 1);
        setHistoryIndex(newIndex);
        setValue(history[newIndex]);
      }
    } else if (e.key === 'ArrowDown') {
      // Navigate history forwards
      e.preventDefault();
      if (historyIndex !== -1) {
        const newIndex = historyIndex + 1;
        if (newIndex >= history.length) {
          setHistoryIndex(-1);
          setValue('');
        } else {
          setHistoryIndex(newIndex);
          setValue(history[newIndex]);
        }
      }
    }
  };

  return html`
    <pre class="input-area">${prompt || '>'}${' '}<input
        ref=${inputRef}
        type="text"
        value=${value}
        onInput=${(e) => setValue(e.target.value)}
        onKeyDown=${handleKeyDown}
        disabled=${disabled}
        autocomplete="off"
        autocapitalize="off"
        spellcheck="false"
      /></pre>
  `;
}

/**
 * Loading Component
 * Shown while WASM loads
 */
export function Loading({ message }) {
  return html`
    <div class="terminal theme-green loading">
      <div class="spinner">*</div>
      <div class="message">${message || 'Loading...'}</div>
    </div>
  `;
}

/**
 * Error Component
 * Shown when something goes wrong
 */
export function ErrorDisplay({ title, message, details }) {
  return html`
    <div class="terminal theme-green">
      <div class="error">
        <h2>${title || 'Error'}</h2>
        <p>${message}</p>
        ${details && html`<pre>${details}</pre>`}
      </div>
    </div>
  `;
}

/**
 * Footer Component
 * Displays interpreter version at bottom of terminal
 *
 * Version is fetched from WASM on app load and automatically
 * updated when a new release is deployed (no manual updates needed).
 * Shows empty string if version hasn't loaded yet.
 */
export function Footer({ version }) {
  return html`
    <div class="footer">
      <span class="version">Gruesome ${version ? `v${version}` : ''}</span>
    </div>
  `;
}

/**
 * Main Terminal Component
 * Orchestrates the full terminal interface
 */
export function Terminal({
  // Game state
  status = {},
  outputLines = [],
  upperWindow = [],
  waitingForInput = false,

  // Callbacks
  onCommand,

  // Settings
  theme = 'green',
  font = 'default',
  crtEnabled = false,
  effectsEnabled = false,
  blurLevel = 'medium',
  prompt = '>',
  interpreterVersion = null,
}) {
  const themeClass = `theme-${theme}`;
  const fontClass = `font-${font}`;
  const effectsClass = effectsEnabled ? 'effects-enabled' : '';
  const crtClass = crtEnabled ? `crt-enhanced crt-blur-${blurLevel}` : '';

  const handleCommand = (command) => {
    if (onCommand) {
      onCommand(command);
    }
  };

  // Click anywhere to focus input
  const handleClick = () => {
    const input = document.querySelector('.input-area input');
    if (input) {
      input.focus();
    }
  };

  return html`
    <div class="terminal ${themeClass} ${fontClass} ${effectsClass} ${crtClass}" onClick=${handleClick}>
      <${StatusBar}
        location=${status.location}
        score=${status.score}
        moves=${status.moves}
        time=${status.time}
        isTimeGame=${status.isTimeGame}
      />
      <${OutputArea}
        lines=${outputLines}
        upperWindow=${upperWindow}
        waitingForInput=${waitingForInput}
        onSubmit=${handleCommand}
        prompt=${prompt}
      />
      <${Footer} version=${interpreterVersion} />
    </div>
  `;
}

/**
 * Demo Terminal Component
 * For testing the UI without WASM backend
 */
export function DemoTerminal() {
  const [outputLines, setOutputLines] = useState([
    'ZORK I: The Great Underground Empire',
    'Copyright (c) 1981, 1982, 1983 Infocom, Inc. All rights reserved.',
    'ZORK is a registered trademark of Infocom, Inc.',
    'Revision 88 / Serial number 840726',
    '',
    'West of House',
    'You are standing in an open field west of a white house, with a boarded',
    'front door.',
    'There is a small mailbox here.',
    '',
  ]);

  const [status, setStatus] = useState({
    location: 'West of House',
    score: 0,
    moves: 0,
  });

  const [theme, setTheme] = useState('green');
  const [effectsEnabled, setEffectsEnabled] = useState(true);

  const handleCommand = (command) => {
    // Echo the command
    setOutputLines(prev => [...prev, `>${command}`]);

    // Demo responses
    const lowerCommand = command.toLowerCase().trim();

    if (lowerCommand === 'look' || lowerCommand === 'l') {
      setOutputLines(prev => [...prev,
        '',
        'West of House',
        'You are standing in an open field west of a white house, with a boarded',
        'front door.',
        'There is a small mailbox here.',
        '',
      ]);
    } else if (lowerCommand === 'open mailbox') {
      setOutputLines(prev => [...prev, 'Opening the small mailbox reveals a leaflet.', '']);
      setStatus(prev => ({ ...prev, moves: prev.moves + 1 }));
    } else if (lowerCommand === 'read leaflet' || lowerCommand === 'examine leaflet') {
      setOutputLines(prev => [...prev,
        '"WELCOME TO ZORK!',
        '',
        'ZORK is a game of adventure, danger, and low cunning. In it you will',
        'explore some of the most amazing territory ever seen by mortals. No',
        'computer should be without one!"',
        '',
      ]);
      setStatus(prev => ({ ...prev, moves: prev.moves + 1 }));
    } else if (lowerCommand === 'theme green') {
      setTheme('green');
      setOutputLines(prev => [...prev, '[Theme changed to green phosphor]', '']);
    } else if (lowerCommand === 'theme amber') {
      setTheme('amber');
      setOutputLines(prev => [...prev, '[Theme changed to amber phosphor]', '']);
    } else if (lowerCommand === 'theme white') {
      setTheme('white');
      setOutputLines(prev => [...prev, '[Theme changed to white phosphor]', '']);
    } else if (lowerCommand === 'effects on') {
      setEffectsEnabled(true);
      setOutputLines(prev => [...prev, '[CRT effects enabled]', '']);
    } else if (lowerCommand === 'effects off') {
      setEffectsEnabled(false);
      setOutputLines(prev => [...prev, '[CRT effects disabled]', '']);
    } else if (lowerCommand === 'help') {
      setOutputLines(prev => [...prev,
        'Demo commands:',
        '  look          - Look around',
        '  open mailbox  - Open the mailbox',
        '  read leaflet  - Read the leaflet',
        '  theme green   - Green phosphor theme',
        '  theme amber   - Amber phosphor theme',
        '  theme white   - White phosphor theme',
        '  effects on    - Enable CRT effects',
        '  effects off   - Disable CRT effects',
        '',
      ]);
    } else {
      setOutputLines(prev => [...prev, `I don't understand "${command}".`, '']);
    }

    setStatus(prev => ({ ...prev, moves: prev.moves + 1 }));
  };

  return html`
    <${Terminal}
      status=${status}
      outputLines=${outputLines}
      waitingForInput=${true}
      onCommand=${handleCommand}
      theme=${theme}
      effectsEnabled=${effectsEnabled}
    />
  `;
}

// Export for use in main.js
window.GruesomeTerminal = {
  Terminal,
  DemoTerminal,
  Loading,
  ErrorDisplay,
  StatusBar,
  OutputArea,
  InputArea,
  Footer,
};
