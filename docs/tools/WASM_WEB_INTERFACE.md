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
  provide_restore_data(data: Uint8Array): void;
  cancel_restore(): void;
  step(): StepResult;
  readonly version: number;
}
```

### StepResult Class

```typescript
class StepResult {
  readonly needs_input: boolean;
  readonly needs_restore_data: boolean;
  readonly status_moves: number;
  readonly status_score: number;
  readonly status_location: string;
  readonly quit: boolean;
  readonly error: string | undefined;
  readonly output: string;
  readonly save_data: Uint8Array | undefined;
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

## Save/Restore Support

The WASM interpreter supports save and restore using the standard **Quetzal format** (`.qzl` files). This provides cross-interpreter compatibility - save files created in the web interface can be loaded in native interpreters and vice versa.

### How It Works

**Save:**
1. User types "save" command in game
2. Interpreter serializes game state to Quetzal format
3. Browser triggers download of `gruesome_save.qzl` file
4. Game continues (branch taken on success)

**Restore:**
1. User types "restore" command in game
2. Browser shows file picker dialog
3. User selects a `.qzl` or `.sav` file
4. Interpreter deserializes state from file
5. Game resumes from saved position

### Save File Format

Uses standard Quetzal IFF format with chunks:
- **IFhd** - Game identification (release, serial, checksum)
- **CMem** - Compressed dynamic memory (XOR-RLE compression)
- **Stks** - Call stack frames
- **IntD** - Interpreter identification ("RUST")

### JavaScript API for Save/Restore

```javascript
// After calling step(), check for save/restore needs:
const result = interpreter.step();

// If save data is available, trigger download
if (result.save_data && result.save_data.length > 0) {
  const blob = new Blob([result.save_data], { type: 'application/octet-stream' });
  const url = URL.createObjectURL(blob);
  // Trigger download...
}

// If restore data is needed, show file picker
if (result.needs_restore_data) {
  // User selects file, then:
  const data = new Uint8Array(await file.arrayBuffer());
  interpreter.provide_restore_data(data);
  interpreter.step(); // Continue execution
}

// If user cancels restore:
interpreter.cancel_restore();
interpreter.step();
```

## Known Limitations

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
- ~~Multiple color themes (UI to switch between green/amber/white)~~ ✅ Implemented (green/amber toggle)
- Sound support (for games that use it)
- Transcript download
- Mobile-friendly touch controls
- Keyboard shortcuts for common commands
- Browser localStorage for auto-save persistence
- Enhanced CRT visual effects (see below)

## CRT Visual Enhancement Options

The current implementation uses CSS-only effects (scanlines, text glow). Here are options for more authentic CRT aesthetics, ranging from simple to complex.

### Specialty Terminal Fonts

**Authentic Terminal Fonts (Google Fonts, easy to add):**
- **VT323** - Based on DEC VT320 terminal, very authentic
- **IBM 3270** - IBM mainframe terminal look, excellent for the era
- **Share Tech Mono** - Clean terminal aesthetic

**Pixel/Bitmap Style:**
- **Press Start 2P** - 8-bit game aesthetic
- **Commodore 64** fonts - Home computer feel
- **Perfect DOS VGA** - Classic DOS look

### CSS Enhancement Libraries

**crt.css** (https://github.com/mskocik/crt.css)
- Lightweight CSS-only CRT effects
- Adds: flicker, glow, color separation
- **Improvement over current**: Moderate - adds subtle flicker and chromatic aberration we don't have
- **Complexity**: Low - just include CSS file
- **Drawback**: Effects are subtle, not dramatically different from our current implementation

**Our current CSS** already implements:
- Scanlines overlay
- Text glow via text-shadow
- Phosphor color themes

**Additional CSS-only effects we could add:**
- Screen curvature (barrel distortion via CSS border-radius tricks)
- Vignette (darkened corners via radial-gradient overlay)
- Subtle flicker animation
- Interlacing effect

### JavaScript/Canvas Libraries

**Retro.js** and similar
- **Improvement over current**: Moderate - adds animated noise, better flicker
- **Complexity**: Medium - requires JS integration
- **Drawback**: May impact performance on low-end devices

### WebGL Shader Approaches (Most Authentic)

**Three.js + postprocessing**
- Full shader pipeline with FilmPass, UnrealBloomPass
- **Improvement over current**: Dramatic - phosphor persistence, true bloom, barrel distortion
- **Complexity**: High - significant code addition, WebGL dependency
- **Drawback**: Won't work on older browsers/devices, adds ~100KB+ to bundle

**Custom CRT Fragment Shaders** (via ShaderToy patterns)
- Phosphor persistence/ghosting (text leaves trails)
- True bloom effect (light bleeds realistically)
- Barrel distortion (curved screen effect)
- RGB shadow mask pattern (visible subpixels)
- Chromatic aberration (color fringing at edges)

**cool-retro-term** style effects
- The gold standard for CRT emulation (used by the cool-retro-term Linux app)
- Would require porting GLSL shaders to WebGL
- **Improvement over current**: Transformative - looks like actual CRT monitor
- **Complexity**: Very high

### Recommendation by Effort/Impact

| Approach | Effort | Visual Impact | Compatibility |
|----------|--------|---------------|---------------|
| Add VT323/IBM 3270 font | Low | Medium | Excellent |
| Add vignette + curvature CSS | Low | Low-Medium | Excellent |
| Include crt.css | Low | Low | Excellent |
| Custom flicker/noise CSS | Medium | Medium | Excellent |
| WebGL CRT shader | High | Very High | Good (modern browsers) |

**Quick wins**: Adding an authentic terminal font (VT323 or IBM 3270) would have the highest impact-to-effort ratio. The font choice significantly affects perceived authenticity.

**Maximum authenticity**: A WebGL shader approach would give the most dramatic "old CRT monitor" feel but represents a significant complexity increase and potential compatibility concerns.
