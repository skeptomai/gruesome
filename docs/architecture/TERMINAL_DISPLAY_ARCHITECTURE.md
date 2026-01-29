# Terminal Display Architecture: Z-Machine vs Modern Terminals

This document explains the fundamental architectural challenges encountered when implementing Z-Machine display functionality on modern terminal emulators, and why certain implementation choices were made.

## The Core Problem: 1980s vs 2020s Display Models

### 1. Can Modern Terminal Emulators Act Like 80s Dumb Terminals?

**Short answer**: Not really, not reliably.

#### Fundamental Differences

**80s terminals**:
- Direct memory-mapped video buffers
- Immediate character placement at absolute coordinates
- Fixed viewport with no scrollback
- Direct hardware access to cursor positioning

**Modern terminal emulators**:
- Complex software with scrollback buffers
- Shell integration and window management
- Layered architecture with multiple coordinate systems
- Abstract display models with buffering

#### What We Discovered During Implementation

During development of the crossterm-based display system, we encountered several critical issues:

- **Coordinate system mismatch**: Crossterm reports terminal size including scrollback buffer (e.g., 35 lines when only 29 are visible)
- **Invisible viewport**: Coordinate (0,0) maps to scrollback history, not visible top-left corner
- **Unreliable size detection**: No consistent way to detect "true" viewport boundaries
- **Environment dependencies**: LINES/COLUMNS environment variables often not set
- **Execution context issues**: Even `stty` fails in some execution contexts (e.g., when run through cargo)

#### The Fundamental Assumption Break

The Z-Machine was designed assuming you could write a character to coordinate (x,y) and it would appear exactly there on screen. Modern terminals break this assumption with their layered architecture, making "direct" screen access an illusion.

### 2. Z-Machine Display Commands vs ANSI Escape Sequences

**Correct!** The Z-Machine specification defines **logical display opcodes**, not ANSI sequences.

#### Z-Machine Display Opcodes

The Z-Machine specification defines these logical display commands:

- `set_cursor` - Move cursor to absolute position (1-based coordinates)
- `erase_window` - Clear specific window (-1=all, 0=lower, 1=upper)  
- `split_window` - Create upper window with N lines
- `set_window` - Switch between upper/lower windows
- `set_text_style` - Set reverse video, bold, italic
- `erase_line` - Clear from cursor to end of line (v4+)
- `get_cursor` - Get current cursor position (v4+)

#### Implementation Responsibility

The Z-Machine interpreter must translate these logical commands into whatever the target platform uses:

- **1980s platforms**: Direct hardware access to video memory
- **Modern terminals**: ANSI escape sequences via libraries like crossterm
- **GUI systems**: Custom windowing code with native APIs
- **Web browsers**: HTML/CSS/JavaScript rendering
- **Game consoles**: Platform-specific graphics APIs

#### The Abstraction Layer Challenge

This is why ratatui actually works better than raw crossterm for Z-Machine implementation:

- **Ratatui**: Provides a proper abstraction layer that handles terminal compatibility quirks
- **Raw crossterm**: Exposes terminal inconsistencies that break Z-Machine assumptions
- **Result**: Modern UI library better emulates 1980s direct-screen-access than "lower-level" approaches

## Our Implementation Solution

### Why We Chose Ratatui Over Crossterm

After extensive testing, we implemented the following display strategy:

1. **v3 games**: Simple terminal display for maximum compatibility
2. **v4+ games**: Ratatui display for advanced windowing features
3. **Fallback chain**: Graceful degradation to headless mode if displays fail

### The Scrolling Fix

The critical breakthrough was fixing ratatui's scroll calculation to account for word-wrapped lines:

```rust
// Calculate actual display lines after word wrapping
let mut total_display_lines = 0;
for line in &lower_lines {
    if line.is_empty() {
        total_display_lines += 1;
    } else {
        // Calculate wrapped line count
        let wrapped_lines = (line.len() + available_width - 1) / available_width.max(1);
        total_display_lines += wrapped_lines.max(1);
    }
}

let scroll_offset = total_display_lines.saturating_sub(available_lines);
```

This ensures input prompts remain visible when content fills the terminal, resolving the core gameplay issue in games like AMFV.

## Historical Context

### Why Original Interpreters Were Platform-Specific

This architectural analysis explains why original Z-Machine interpreters were often platform-specific. Each platform required custom display code to properly implement the logical display model:

- **Apple II**: Direct memory access to text/graphics pages
- **Commodore 64**: Custom character ROM and color memory
- **PC DOS**: Direct video buffer access or BIOS calls
- **Unix terminals**: Terminal-specific escape sequences
- **Macintosh**: QuickDraw graphics primitives

### The Modern Challenge

Today's interpreters face the challenge of implementing a direct-access display model on top of abstracted, buffered, windowed systems. The Z-Machine's elegant simplicity becomes a complexity when the underlying assumptions no longer hold.

## Key Takeaways

1. **Modern terminals are not dumb terminals**: They have complex internal state that interferes with direct positioning
2. **Abstraction layers help**: Libraries like ratatui solve compatibility issues that raw terminal access exposes
3. **The Z-Machine model is still valid**: The logical display commands work well, but require careful implementation
4. **Platform adaptation is still necessary**: Different environments require different strategies, just as in the 1980s

This architectural understanding informed our final implementation choice and explains why certain approaches succeeded while others failed.

## Our Ratatui Implementation Details

### How We Use Ratatui

#### 1. Fullscreen Alternate Screen Mode

```rust
// We enter alternate screen mode for fullscreen control
if let Err(_e) = execute!(stdout, EnterAlternateScreen) {
    // Continue without alternate screen - may work in some environments
}
```

This puts the terminal into fullscreen mode, taking over the entire terminal viewport (similar to vim/emacs), avoiding interference from shell prompts and history.

#### 2. Frame-Based Coordinate System (Not Raw Terminal)

```rust
// We get size from ratatui's terminal, not crossterm directly
let ratatui_size = state.terminal.size()?;

// In render function, we use frame's abstracted size
f.render_widget(ratatui::widgets::Clear, f.size());
let screen_rect = f.size();
```

**This is crucial**: We use **ratatui's abstracted size** (`f.size()`) rather than crossterm's raw `terminal::size()`. This avoids the scrollback buffer coordinate issues we encountered with direct crossterm positioning.

#### 3. Our Hybrid Screen Drawing Process

Our `render()` method (lines 770-856 in `display_ratatui.rs`) implements a sophisticated hybrid approach:

##### Step 1: Dynamic Layout Creation
```rust
let chunks = if self.upper_window_lines > 0 {
    Layout::default()
        .direction(Direction::Vertical)
        .margin(0) // No margin - use full screen
        .constraints([
            Constraint::Length(self.upper_window_lines),
            Constraint::Min(0), // Allow zero height if needed
        ])
        .split(screen_rect)
} else {
    vec![f.size(), f.size()].into() // Use full screen for lower window
};
```

This creates a **dynamic vertical split** with:
- **Upper chunk**: Fixed height (N lines for status/menus)
- **Lower chunk**: Remaining space (for main game text)

##### Step 2: Upper Window - Direct Buffer Manipulation
```rust
// Direct character-by-character placement for upper window
for (line_idx, styled_line) in self.upper_window_content.iter().enumerate() {
    let y = chunks[0].y + line_idx as u16;
    for (col_idx, styled_char) in styled_line.iter().enumerate() {
        let x = chunks[0].x + col_idx as u16;
        f.buffer_mut()
            .get_mut(x, y)
            .set_char(styled_char.ch)
            .set_style(style);
    }
}
```

**Critical technique**: We bypass ratatui's widget system entirely and directly manipulate the character buffer for precise Z-Machine positioning in the upper window. This gives us exact 1980s-style character placement.

##### Step 3: Lower Window - Modern Widget System
```rust
let lower_paragraph = Paragraph::new(lower_text)
    .wrap(Wrap { trim: false }) // Don't trim - preserve spaces!
    .style(Style::default().bg(Color::Black).fg(Color::White))
    .scroll((scroll_offset as u16, 0));

f.render_widget(lower_paragraph, chunks[1]);
```

For the lower window, we use ratatui's `Paragraph` widget with automatic word wrapping and our custom scroll calculation.

### Why This Hybrid Approach Works Perfectly

#### Upper Window: "1980s Style" Direct Placement
- **Z-Machine compliant**: Exact character positioning for status lines, menus
- **No interference**: Bypasses ratatui's layout system for precise control
- **Perfect for**: "* PART I *" boxes, status lines, absolute cursor positioning
- **Implementation**: Direct buffer manipulation via `f.buffer_mut().get_mut(x, y)`

#### Lower Window: "Modern Style" Text Flow
- **Automatic word wrap**: Handles varying terminal widths gracefully
- **Smart scrolling**: Built-in scroll management with our word-wrap-aware calculation
- **Natural text flow**: Proper paragraph rendering for game narrative
- **Implementation**: Ratatui `Paragraph` widget with custom scroll offset

#### Coordinate System: Frame-Based Abstraction
- **`f.size()`**: Uses ratatui's abstracted viewport, not raw terminal dimensions
- **Layout chunks**: Automatic space division without manual coordinate calculations
- **Resize handling**: Ratatui handles terminal resize events transparently
- **No scrollback issues**: Frame coordinates map to visible viewport, not terminal history

### The Technical Breakthrough

Our implementation uses **two different rendering paradigms in one system**:

1. **Upper window rendering**: Direct character buffer access for exact positioning
2. **Lower window rendering**: Widget-based text flow for robust content display
3. **Coordinate abstraction**: Frame-based positioning eliminates terminal quirks

This hybrid approach solves the fundamental architecture mismatch:
- We get Z-Machine's direct positioning model where required (upper window)
- We get modern terminal compatibility where beneficial (lower window)
- We avoid raw terminal coordinate system problems entirely (frame abstraction)

### Code Location Summary

The key implementation files:
- **`src/display_ratatui.rs`**: Main ratatui implementation
  - Lines 770-856: Core `render()` method with hybrid approach
  - Lines 791-810: Direct buffer manipulation for upper window
  - Lines 847-852: Widget-based lower window rendering
- **`src/display_manager.rs`**: Display system selection logic
- **`src/display_trait.rs`**: Common interface abstraction

This technical approach explains why our ratatui implementation succeeded where pure crossterm failed - we combined the best aspects of both direct hardware access (for Z-Machine compliance) and modern terminal abstraction (for compatibility) in a single, cohesive system.