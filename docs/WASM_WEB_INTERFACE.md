# WASM Web Interface

This document describes the WebAssembly (WASM) build target for Gruesome, enabling in-browser gameplay of Z-Machine games.

## Overview

The WASM interface allows users to play Z-Machine games directly in a web browser without installing any software. The Rust interpreter compiles to WebAssembly and runs client-side.

## Architecture

### Feature Flags

The codebase uses Cargo feature flags to support both native and WASM builds:

```toml
[features]
default = ["native"]
native = ["dep:crossterm", "dep:ratatui", "dep:atty", "dep:env_logger"]
wasm = ["dep:wasm-bindgen", "dep:web-sys", "dep:js-sys", "dep:console_log",
        "dep:console_error_panic_hook", "getrandom/js"]
```

### Conditional Compilation

Modules that use platform-specific functionality are conditionally compiled:

- `src/interpreter/core/interpreter.rs` - Native only (uses std::io)
- `src/interpreter/input/` - Native only
- `src/interpreter/display/display_crossterm.rs` - Native only
- `src/interpreter/display/display_wasm.rs` - WASM only
- `src/interpreter/quetzal/{save,restore,iff}.rs` - Native only (file I/O)

### WASM-Specific Files

- **`src/wasm.rs`** - Main WASM entry point, exports `WasmInterpreter` class
- **`src/interpreter/display/display_wasm.rs`** - Display implementation that buffers output for JavaScript consumption

## Build Commands

### Native Build (default)
```bash
cargo build
cargo build --release
```

### WASM Build
```bash
wasm-pack build --target web --out-dir web/pkg --no-default-features --features wasm
```

This generates:
- `web/pkg/gruesome.js` - JavaScript bindings
- `web/pkg/gruesome_bg.wasm` - Compiled WASM module (~145KB)
- `web/pkg/gruesome.d.ts` - TypeScript definitions

## Web Frontend

### Directory Structure

```
web/
├── index.html              # Entry point, loads Preact from CDN
├── css/
│   └── terminal.css        # Retro terminal styling
├── js/
│   ├── main.js             # App logic, WASM integration, disclaimer page
│   └── terminal.js         # Preact UI components
├── pkg/                    # Generated WASM package (built by CI, gitignored locally)
│   ├── gruesome.js
│   ├── gruesome_bg.wasm
│   └── ...
└── games/
    └── zork1.z3            # Default game file (Zork I)
```

### Technology Stack

- **Preact + HTM** - Lightweight React alternative loaded from CDN (no build step)
- **wasm-bindgen** - Rust/JavaScript interop
- **Pure CSS** - Retro terminal styling with phosphor color themes

### UI Features

- Green phosphor terminal theme (amber and white also available)
- Status bar with location, score, and moves
- Command history (up/down arrow keys)
- Auto-scrolling output
- File picker for loading game files
- CRT scanline effects (optional)
- Disclaimer/landing page with legal notices

## JavaScript API

### WasmInterpreter Class

```typescript
class WasmInterpreter {
  constructor(game_data: Uint8Array);
  provide_input(input: string): void;
  step(): StepResult;
  readonly version: number;
}
```

### StepResult Class

```typescript
class StepResult {
  readonly needs_input: boolean;
  readonly status_moves: number;
  readonly status_score: number;
  readonly status_location: string;
  readonly quit: boolean;
  readonly error: string | undefined;
  readonly output: string;
  free(): void;
}
```

### Usage Pattern

```javascript
// Initialize WASM
const wasm = await import('./pkg/gruesome.js');
await wasm.default();
wasm.init(); // Set up panic hook

// Load game
const response = await fetch('./games/zork1.z3');
const gameData = new Uint8Array(await response.arrayBuffer());
const interpreter = new wasm.WasmInterpreter(gameData);

// Run until input needed
let result = interpreter.step();
console.log(result.output);

// Provide input and continue
interpreter.provide_input("open mailbox");
result = interpreter.step();
console.log(result.output);

// Clean up
result.free();
```

## File Loading Flow

### On the Hosting Server

```
web/
├── index.html
├── pkg/gruesome_bg.wasm    # WASM interpreter
└── games/zork1.z3          # Default game file
```

### Loading Sequence

1. **Browser requests `index.html`** from server
2. **JavaScript loads**, imports `pkg/gruesome.js`
3. **WASM module fetched** (`gruesome_bg.wasm`) and instantiated
4. **Disclaimer page shown** with legal notices and options
5. **User clicks "Play Zork I"** or "Load Your Own Game"
6. **`tryLoadDefaultGame()` runs** (if playing Zork I):
   - Fetches `./games/zork1.z3` via HTTP from server
   - If 200 OK: game auto-loads
   - If 404: file picker shown
7. **Game data transferred** to browser memory as `Uint8Array`
8. **`WasmInterpreter` created** with game data (all in browser RAM)
9. **Gameplay begins** - all execution happens client-side

### Key Point

The `fetch('./games/zork1.z3')` is an HTTP request to the hosting server. The path is relative to the browser's URL, not any local filesystem. Users can also load local files via the file picker, which reads directly from their computer without uploading to the server.

## GitHub Pages Deployment

### Automated Deployment via GitHub Actions

The repository includes a GitHub Actions workflow (`.github/workflows/deploy-pages.yml`) that automatically:

1. Triggers on push to `main` branch
2. Installs Rust toolchain and wasm-pack
3. Builds WASM module
4. Deploys `web/` directory to GitHub Pages

### Workflow Configuration

```yaml
name: Deploy to GitHub Pages

on:
  push:
    branches: [main]
  workflow_dispatch:  # Allow manual trigger

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-action@stable
      - run: cargo install wasm-pack
      - run: wasm-pack build --target web --out-dir web/pkg --no-default-features --features wasm
      - uses: actions/configure-pages@v4
      - uses: actions/upload-pages-artifact@v3
        with:
          path: ./web

  deploy:
    runs-on: ubuntu-latest
    needs: build
    steps:
      - uses: actions/deploy-pages@v4
```

### Enable GitHub Pages

1. Go to repository Settings → Pages
2. Source: "GitHub Actions"
3. Site will be available at `https://<username>.github.io/<repo>/`

### Local Testing

To test locally before pushing:

```bash
# Build WASM
wasm-pack build --target web --out-dir web/pkg --no-default-features --features wasm

# Serve locally
cd web && python3 -m http.server 8080

# Open http://localhost:8080
```

## Disclaimer Page

The web interface includes a disclaimer/landing page shown before gameplay:

### Content

- **Project description**: Educational Z-Machine interpreter in Rust/WASM
- **Legal notice**: Zork I copyright belongs to Activision
- **Links to acquire legally**:
  - GOG.com - The Zork Anthology
  - Steam - Zork Anthology
  - Infocom-IF.org - Game Information
- **Free alternatives**: Link to Interactive Fiction Archive
- **Action buttons**: "Play Zork I" (prominent) and "Load Your Own Game" (secondary)
- **Acknowledgment footer**: User accepts responsibility for copyrighted material use

### Purpose

Provides legal cover for hosting abandonware by:
1. Clearly stating copyright ownership
2. Providing links to purchase legally
3. Offering alternatives
4. Requiring user acknowledgment before proceeding

## Legal Considerations

Z-Machine game files (like Zork I) are copyrighted by Activision Publishing, Inc.

### Options for Deployment

1. **Safest**: Deploy interpreter only, let users supply their own game files via file picker
2. **Gray area**: Include abandonware games with disclaimer (current approach)
3. **Proper**: Obtain permission from rights holders

### Risk Mitigation

- Disclaimer page with legal notice
- Links to legitimate purchase options
- User acknowledgment required
- File picker allows users to supply their own legally-obtained games

## Known Limitations

### Save/Restore Not Implemented

The WASM interpreter does not currently support the `save` and `restore` opcodes. Attempting to save will show an error:

```
Unimplemented opcode: save (0x05) at <address>
```

**Why**: The native interpreter uses file I/O (Quetzal format) which isn't available in WASM/browser environment.

**Future options**:
1. **Browser localStorage** - Save game state as a blob, persists across sessions
2. **IndexedDB** - More storage space, also persists
3. **Download file** - Generate a save file the user downloads, upload to restore

### Other Limitations

- Some advanced opcodes may not be implemented
- No sound support (V5+ games)
- No graphics support (V6 games)

## Implementation Notes

### Why a Separate WASM Interpreter?

The native interpreter (`src/interpreter/core/interpreter.rs`) uses:
- `std::io` for input/output
- Blocking reads for user input
- Direct terminal manipulation via crossterm

These don't work in WASM. The WASM interpreter (`src/wasm.rs`):
- Buffers output as strings
- Returns control to JavaScript when input needed
- Uses message-passing pattern instead of blocking I/O

### Opcodes Implemented in WASM

The WASM interpreter implements a subset of Z-Machine opcodes sufficient for most games. Unimplemented opcodes return an error message. The native interpreter remains the reference implementation.

## Future Enhancements

Potential improvements:
- Save/restore via browser localStorage or IndexedDB
- Multiple color themes (UI to switch between green/amber/white)
- Sound support (for games that use it)
- Transcript download
- Mobile-friendly touch controls
- Keyboard shortcuts for common commands
