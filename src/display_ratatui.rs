//! Ratatui-based display management for Z-Machine v4+ games
//!
//! This module provides advanced display capabilities required by v4+ Z-Machine games
//! like A Mind Forever Voyaging (AMFV). It features:
//!
//! ## Key Capabilities:
//! - **Split windowing**: Upper window (status/menus) + lower window (main text)
//! - **Absolute positioning**: Character-level cursor control in upper window  
//! - **Text flow management**: Proper streaming text in lower window with word wrap
//! - **Buffer mode support**: Z-Machine buffered vs unbuffered text output
//! - **Style support**: Reverse video, bold, italic text formatting
//!
//! ## Critical Text Flow Fix:
//! The display thread maintains `lower_current_line` for building text lines.
//! This buffer MUST be cleared during screen transitions (ERASE_WINDOW) to prevent
//! text concatenation issues like "[Hit any key to continue.]You hear..." appearing
//! on the same line instead of separate lines.
//!
//! ## Threading Architecture:
//! - Main thread: Handles Z-Machine execution and buffering
//! - Display thread: Manages terminal and renders content
//! - Communication via mpsc channels with DisplayCommand enum

use crossterm::{
    event::{self, Event},
    execute,
    terminal::{enable_raw_mode, EnterAlternateScreen},
};
use log::debug;
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Paragraph, Wrap},
    Terminal,
};
use std::io::{self, Stdout};
use std::process::Command;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::Duration;

/// Commands sent to the display thread
#[derive(Debug)]
pub enum DisplayCommand {
    SplitWindow(u16),
    SetWindow(u8),
    SetCursor(u16, u16),
    Print(String),
    PrintChar(char),
    EraseWindow(i16),
    ShowStatus(String, i16, u16),
    SetTextStyle(u16),
    ClearScreen,
    EraseLine, // v4+
    Quit,
}

/// Display manager using Ratatui
pub struct RatatuiDisplay {
    /// Channel to send commands to display thread
    tx: Sender<DisplayCommand>,
    /// Handle to display thread
    display_thread: Option<thread::JoinHandle<()>>,
    /// Current window (0 = lower, 1 = upper)
    current_window: u8,
}

/// Internal display state managed by the display thread
struct DisplayState {
    /// Terminal instance
    terminal: Terminal<CrosstermBackend<Stdout>>,
    /// Number of lines in upper window
    upper_window_lines: u16,
    /// Current window (0 = lower, 1 = upper)
    current_window: u8,
    /// Upper window content with style information
    upper_window_content: Vec<Vec<StyledChar>>,
    /// Lower window content as scrolling text lines
    lower_window_content: Vec<String>,
    /// Cursor position in upper window
    upper_cursor_x: u16,
    upper_cursor_y: u16,
    /// Current line being built in lower window
    lower_current_line: String,
    /// Current text style
    text_style: Style,
    /// Terminal dimensions
    terminal_width: u16,
    terminal_height: u16,
    /// Track if reverse video is currently active
    reverse_video_active: bool,
}

/// A character with associated styling
#[derive(Clone, Debug)]
struct StyledChar {
    ch: char,
    reverse_video: bool,
}

/// Get terminal size from environment variables or stty as fallback
fn get_terminal_size_fallback() -> Result<(u16, u16), std::io::Error> {
    // First try environment variables (most reliable for the user's setup)
    if let (Ok(cols), Ok(lines)) = (std::env::var("COLUMNS"), std::env::var("LINES")) {
        if let (Ok(width), Ok(height)) = (cols.parse::<u16>(), lines.parse::<u16>()) {
            if width > 0 && height > 0 {
                return Ok((width, height));
            }
        }
    }

    // Try stty as fallback
    let output = Command::new("stty").arg("size").output()?;

    if output.status.success() {
        let size_str = String::from_utf8_lossy(&output.stdout);
        let parts: Vec<&str> = size_str.split_whitespace().collect();
        if parts.len() == 2 {
            if let (Ok(height), Ok(width)) = (parts[0].parse::<u16>(), parts[1].parse::<u16>()) {
                return Ok((width, height));
            }
        }
    }

    Err(std::io::Error::new(
        std::io::ErrorKind::InvalidData,
        "Failed to get terminal size from environment or stty",
    ))
}

impl RatatuiDisplay {
    /// Create a new Ratatui-based display
    pub fn new() -> Result<Self, String> {
        // More permissive TTY detection - only fail if we definitely can't work
        if std::env::var("TERM").is_err() && std::env::var("COLORTERM").is_err() {
            return Err("No terminal environment detected (TERM/COLORTERM not set)".to_string());
        }

        // Skip raw mode test - let the display thread handle terminal setup
        // If it fails there, the thread will report the error

        let (tx, rx) = mpsc::channel();

        // Spawn display thread
        let display_thread = thread::spawn(move || if let Err(_e) = run_display_thread(rx) {});

        Ok(RatatuiDisplay {
            tx,
            display_thread: Some(display_thread),
            current_window: 0, // Start in lower window
        })
    }

    /// Send a command to the display thread
    fn send_command(&self, cmd: DisplayCommand) -> Result<(), String> {
        self.tx
            .send(cmd)
            .map_err(|e| format!("Failed to send display command: {e}"))
    }

    /// Clear the entire screen
    pub fn clear_screen(&mut self) -> Result<(), String> {
        self.send_command(DisplayCommand::ClearScreen)
    }

    /// Split the screen into upper and lower windows
    pub fn split_window(&mut self, lines: u16) -> Result<(), String> {
        self.send_command(DisplayCommand::SplitWindow(lines))
    }

    /// Set the current window
    pub fn set_window(&mut self, window: u8) -> Result<(), String> {
        self.current_window = window;
        self.send_command(DisplayCommand::SetWindow(window))
    }

    /// Set cursor position (1-based)
    pub fn set_cursor(&mut self, line: u16, column: u16) -> Result<(), String> {
        self.send_command(DisplayCommand::SetCursor(line, column))
    }

    /// Print text to current window
    ///
    /// All text is sent immediately to the display thread for real-time rendering.
    /// Z-Machine "buffer mode" controls word-wrapping behavior, not display timing.
    pub fn print(&mut self, text: &str) -> Result<(), String> {
        self.send_command(DisplayCommand::Print(text.to_string()))
    }

    /// Print a character to current window
    pub fn print_char(&mut self, ch: char) -> Result<(), String> {
        self.print(&ch.to_string())
    }

    /// Erase a window
    pub fn erase_window(&mut self, window: i16) -> Result<(), String> {
        debug!("erase_window: {}", window);

        self.send_command(DisplayCommand::EraseWindow(window))
    }

    /// Show status line
    pub fn show_status(&mut self, location: &str, score: i16, moves: u16) -> Result<(), String> {
        self.show_status_with_version(location, score, moves, 3)
    }

    /// Show status line with version-specific behavior
    pub fn show_status_with_version(
        &mut self,
        location: &str,
        score: i16,
        moves: u16,
        _version: u8,
    ) -> Result<(), String> {
        self.send_command(DisplayCommand::ShowStatus(
            location.to_string(),
            score,
            moves,
        ))
    }

    /// Set text style
    pub fn set_text_style(&mut self, style: u16) -> Result<(), String> {
        self.send_command(DisplayCommand::SetTextStyle(style))
    }

    /// Handle terminal resize
    pub fn handle_resize(&mut self, _new_width: u16, _new_height: u16) {
        // Ratatui handles resize automatically
    }

    /// Erase from cursor to end of line (v4+)
    pub fn erase_line(&mut self) -> Result<(), String> {
        // Send erase line command to display thread
        self.tx
            .send(DisplayCommand::EraseLine)
            .map_err(|_| "Failed to send erase line command".to_string())
    }

    /// Get current cursor position (v4+)
    /// Returns (line, column) with 1-based indexing
    pub fn get_cursor(&mut self) -> Result<(u16, u16), String> {
        // We need to get the cursor position from the display state
        // For now, return a default since we don't have a way to query the display thread
        // In a real implementation, we'd need a request/response mechanism
        Ok((1, 1))
    }

    /// Set buffer mode (v4+)
    ///
    /// Z-Machine buffer mode controls word-wrapping to prevent words from splitting
    /// across lines. It does NOT control display timing - all text appears immediately.
    /// Our ratatui implementation handles word-wrapping automatically.
    pub fn set_buffer_mode(&mut self, _buffered: bool) -> Result<(), String> {
        Ok(())
    }

    /// Get terminal size
    pub fn get_terminal_size(&self) -> (u16, u16) {
        // Try crossterm first
        match crossterm::terminal::size() {
            Ok(size) => {
                // If crossterm returns default 80x24, try fallback methods
                if size == (80, 24) {
                    if let Ok(fallback_size) = get_terminal_size_fallback() {
                        return fallback_size;
                    }
                }
                size
            }
            Err(_) => get_terminal_size_fallback().unwrap_or((80, 24)),
        }
    }

    /// Get effective screen size for Z-Machine header
    /// This accounts for typical upper window usage to prevent coordinate mismatches
    pub fn get_effective_screen_size(&self) -> (u16, u16) {
        let (width, height) = self.get_terminal_size();

        // Reserve space for typical upper window usage (1-2 lines for status/menus)
        // This ensures Z-Machine header size matches actual usable lower window space
        let effective_height = if height > 2 {
            height - 1 // Reserve 1 line for typical status line
        } else {
            height // Don't go below minimum
        };

        (width, effective_height)
    }

    /// Force refresh
    pub fn force_refresh(&mut self) -> Result<(), String> {
        // Ratatui handles refresh automatically
        Ok(())
    }
}

use crate::display_trait::{DisplayError, ZMachineDisplay};

impl ZMachineDisplay for RatatuiDisplay {
    fn clear_screen(&mut self) -> Result<(), DisplayError> {
        self.clear_screen().map_err(DisplayError::new)
    }

    fn split_window(&mut self, lines: u16) -> Result<(), DisplayError> {
        self.split_window(lines).map_err(DisplayError::new)
    }

    fn set_window(&mut self, window: u8) -> Result<(), DisplayError> {
        self.set_window(window).map_err(DisplayError::new)
    }

    fn set_cursor(&mut self, line: u16, column: u16) -> Result<(), DisplayError> {
        self.set_cursor(line, column).map_err(DisplayError::new)
    }

    fn print(&mut self, text: &str) -> Result<(), DisplayError> {
        self.print(text).map_err(DisplayError::new)
    }

    fn print_char(&mut self, ch: char) -> Result<(), DisplayError> {
        self.print_char(ch).map_err(DisplayError::new)
    }

    fn erase_window(&mut self, window: i16) -> Result<(), DisplayError> {
        self.erase_window(window).map_err(DisplayError::new)
    }

    fn handle_resize(&mut self, width: u16, height: u16) {
        self.handle_resize(width, height);
    }

    fn show_status(&mut self, location: &str, score: i16, moves: u16) -> Result<(), DisplayError> {
        self.show_status(location, score, moves)
            .map_err(DisplayError::new)
    }

    fn erase_line(&mut self) -> Result<(), DisplayError> {
        self.erase_line().map_err(DisplayError::new)
    }

    fn get_cursor(&mut self) -> Result<(u16, u16), DisplayError> {
        self.get_cursor().map_err(DisplayError::new)
    }

    fn set_buffer_mode(&mut self, buffered: bool) -> Result<(), DisplayError> {
        self.set_buffer_mode(buffered).map_err(DisplayError::new)
    }

    fn get_terminal_size(&self) -> (u16, u16) {
        self.get_terminal_size()
    }

    fn get_effective_screen_size(&self) -> (u16, u16) {
        self.get_effective_screen_size()
    }

    fn force_refresh(&mut self) -> Result<(), DisplayError> {
        self.force_refresh().map_err(DisplayError::new)
    }

    fn set_text_style(&mut self, style: u16) -> Result<(), DisplayError> {
        self.set_text_style(style).map_err(DisplayError::new)
    }

    fn print_input_echo(&mut self, text: &str) -> Result<(), DisplayError> {
        // Input echo uses standard print - display thread handles timing
        self.print(text).map_err(DisplayError::new)
    }
}

impl Drop for RatatuiDisplay {
    fn drop(&mut self) {
        // Send quit command and wait for thread to finish
        let _ = self.send_command(DisplayCommand::Quit);
        if let Some(thread) = self.display_thread.take() {
            let _ = thread.join();
        }
    }
}

/// Run the display thread
fn run_display_thread(rx: Receiver<DisplayCommand>) -> Result<(), Box<dyn std::error::Error>> {
    // Try to setup terminal with fallback handling
    if let Err(_e) = enable_raw_mode() {
        // Don't fail immediately - some terminals work without raw mode
        // We'll try to continue and see if basic terminal access works
    }
    let mut stdout = io::stdout();
    if let Err(_e) = execute!(stdout, EnterAlternateScreen) {
        // Continue without alternate screen - may work in some environments
    }
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend).map_err(|e| format!("Failed to create terminal: {e}"))?;

    // Create display state
    let mut state = DisplayState {
        terminal,
        upper_window_lines: 0,
        current_window: 0,
        upper_window_content: vec![],
        lower_window_content: vec![],
        upper_cursor_x: 0,
        upper_cursor_y: 0,
        lower_current_line: String::new(),
        text_style: Style::default(),
        terminal_width: 0,
        terminal_height: 0,
        reverse_video_active: false,
    };

    // Get initial terminal size with fallback
    let ratatui_size = state.terminal.size()?;

    // If ratatui reports default 80x24, try fallback methods
    let final_size = if ratatui_size.width == 80 && ratatui_size.height == 24 {
        if let Ok(fallback_size) = get_terminal_size_fallback() {
            (fallback_size.0, fallback_size.1)
        } else {
            (ratatui_size.width, ratatui_size.height)
        }
    } else {
        (ratatui_size.width, ratatui_size.height)
    };

    state.terminal_width = final_size.0;
    state.terminal_height = final_size.1;

    // Initial render
    state.render()?;

    // Main event loop
    loop {
        let mut should_render = false;

        // Process all available commands before rendering
        loop {
            match rx.recv_timeout(Duration::from_millis(16)) {
                Ok(cmd) => match cmd {
                    DisplayCommand::Quit => return Ok(()),
                    _ => {
                        handle_command(&mut state, cmd)?;
                        should_render = true;
                    }
                },
                Err(mpsc::RecvTimeoutError::Timeout) => break,
                Err(mpsc::RecvTimeoutError::Disconnected) => return Ok(()),
            }
        }

        // Render only if we processed commands
        if should_render {
            state.render()?;
        } else {
            // Check for terminal resize events if no commands were processed
            if event::poll(Duration::from_millis(0))? {
                if let Event::Resize(width, height) = event::read()? {
                    state.terminal_width = width;
                    state.terminal_height = height;
                    state.render()?;
                }
            }
        }
    }
}

/// Handle a display command
fn handle_command(
    state: &mut DisplayState,
    cmd: DisplayCommand,
) -> Result<(), Box<dyn std::error::Error>> {
    match cmd {
        DisplayCommand::SplitWindow(lines) => {
            state.upper_window_lines = lines;
            // Initialize upper window content with spaces to properly separate windows
            state.upper_window_content.clear();
            for _line_idx in 0..state.upper_window_lines {
                // Fill each line with spaces to ensure proper window separation
                let mut line = Vec::new();
                for _col_idx in 0..state.terminal_width {
                    line.push(StyledChar {
                        ch: ' ',
                        reverse_video: false,
                    }); // Use space character
                }
                state.upper_window_content.push(line);
            }
        }
        DisplayCommand::SetWindow(window) => {
            state.current_window = window;
        }
        DisplayCommand::SetCursor(line, column) => {
            if state.current_window == 1 {
                let target_line = (line - 1) as usize; // Convert to 0-based

                // Auto-expand upper window if cursor positioned beyond bounds (error recovery)
                while target_line >= state.upper_window_content.len() {
                    let mut new_line = Vec::new();
                    for _ in 0..state.terminal_width {
                        new_line.push(StyledChar {
                            ch: ' ',
                            reverse_video: false,
                        });
                    }
                    state.upper_window_content.push(new_line);
                    state.upper_window_lines += 1;
                }

                state.upper_cursor_y = target_line as u16;
                state.upper_cursor_x = (column - 1).min(state.terminal_width - 1);
            }
            // Lower window doesn't support cursor positioning per Z-Machine spec
        }
        DisplayCommand::Print(text) => {
            if state.current_window == 1 && state.upper_window_lines > 0 {
                // Print to upper window with style information, handling newlines
                let mut current_y = state.upper_cursor_y as usize;
                let mut current_x = state.upper_cursor_x as usize;

                // Handle text with potential newlines
                for ch in text.chars() {
                    if ch == '\n' {
                        // Move to next line
                        current_y += 1;
                        current_x = 0;

                        // Auto-expand upper window if needed (error recovery per Z-Machine spec)
                        while current_y >= state.upper_window_content.len() {
                            let mut new_line = Vec::new();
                            for _ in 0..state.terminal_width {
                                new_line.push(StyledChar {
                                    ch: ' ',
                                    reverse_video: false,
                                }); // Use space character
                            }
                            state.upper_window_content.push(new_line);
                            state.upper_window_lines += 1;
                        }
                    } else if ch == '\x08' {
                        // Backspace - move cursor left
                        current_x = current_x.saturating_sub(1);
                    } else {
                        // Auto-expand upper window if needed (error recovery per Z-Machine spec)
                        while current_y >= state.upper_window_content.len() {
                            let mut new_line = Vec::new();
                            for _ in 0..state.terminal_width {
                                new_line.push(StyledChar {
                                    ch: ' ',
                                    reverse_video: false,
                                }); // Use space character
                            }
                            state.upper_window_content.push(new_line);
                            state.upper_window_lines += 1;
                        }

                        // Regular character
                        if current_y < state.upper_window_content.len() {
                            let line = &mut state.upper_window_content[current_y];

                            // Ensure line is long enough with spaces
                            while line.len() <= current_x {
                                line.push(StyledChar {
                                    ch: ' ',
                                    reverse_video: false,
                                });
                            }

                            // Place styled character at cursor position
                            let styled_char = StyledChar {
                                ch,
                                reverse_video: state.reverse_video_active,
                            };

                            if current_x < line.len() {
                                line[current_x] = styled_char;
                            } else {
                                line.push(styled_char);
                            }

                            current_x += 1;
                            // Don't auto-wrap - let the Z-Machine handle line breaking
                        }
                    }
                }

                // Update cursor position
                state.upper_cursor_y = (current_y as u16).min(state.upper_window_lines - 1);
                state.upper_cursor_x = (current_x as u16).min(state.terminal_width - 1);
            } else {
                // Print to lower window with proper text flow
                debug!("Lower window: adding text '{}'", text);

                // Handle newlines and control characters in text
                if text.contains('\n') {
                    let parts: Vec<&str> = text.split('\n').collect();

                    // Add first part to current line with backspace processing
                    if !parts.is_empty() {
                        process_text_with_backspace(&mut state.lower_current_line, parts[0]);
                    }

                    // For each newline, finish current line and start new ones
                    for part in parts.iter().skip(1) {
                        // Finish current line and add to content
                        state
                            .lower_window_content
                            .push(state.lower_current_line.clone());
                        state.lower_current_line.clear();

                        // Start new line with this part (with backspace processing)
                        process_text_with_backspace(&mut state.lower_current_line, part);
                    }
                } else {
                    // No newlines - add to current line with backspace processing
                    process_text_with_backspace(&mut state.lower_current_line, &text);
                }

                // Keep scrolling buffer reasonable
                let max_lines = (state.terminal_height - state.upper_window_lines) as usize;
                if state.lower_window_content.len() > max_lines * 3 {
                    state.lower_window_content.drain(0..max_lines);
                }
            }
        }
        DisplayCommand::PrintChar(ch) => {
            handle_command(state, DisplayCommand::Print(ch.to_string()))?;
        }
        DisplayCommand::EraseWindow(window) => {
            match window {
                -1 => {
                    // Clear entire screen
                    // CRITICAL: Must clear lower_current_line to prevent text concatenation
                    // Without this, "[Hit any key to continue.]" gets concatenated with
                    // subsequent text like "You hear..." on the same line
                    state.upper_window_content.clear();
                    state.lower_window_content.clear();
                    state.lower_current_line.clear(); // <- This line prevents holdover text
                    for _ in 0..state.upper_window_lines {
                        state.upper_window_content.push(Vec::new());
                    }
                }
                0 => {
                    // Clear lower window - this should completely reset the text flow
                    state.lower_window_content.clear();
                    state.lower_current_line.clear();
                    debug!(
                        "Lower window cleared - removed {} lines and current line",
                        state.lower_window_content.len()
                    );
                }
                1 => {
                    // Clear upper window - refill with spaces
                    for line in state.upper_window_content.iter_mut() {
                        line.clear();
                        for _ in 0..state.terminal_width {
                            line.push(StyledChar {
                                ch: ' ',
                                reverse_video: false,
                            });
                        }
                    }
                    state.upper_cursor_x = 0;
                    state.upper_cursor_y = 0;
                }
                _ => {}
            }
        }
        DisplayCommand::ShowStatus(location, score, moves) => {
            if !state.upper_window_content.is_empty() {
                let status = format_status_line(&location, score, moves, state.terminal_width);
                // Convert string to styled chars (status line is not reversed)
                let styled_chars: Vec<StyledChar> = status
                    .chars()
                    .map(|ch| StyledChar {
                        ch,
                        reverse_video: false,
                    })
                    .collect();
                state.upper_window_content[0] = styled_chars;
            }
        }
        DisplayCommand::SetTextStyle(style_bits) => {
            let mut style = Style::default();
            if style_bits & 1 != 0 {
                style = style.add_modifier(Modifier::REVERSED);
                state.reverse_video_active = true;
            } else {
                state.reverse_video_active = false;
            }
            if style_bits & 2 != 0 {
                style = style.add_modifier(Modifier::BOLD);
            }
            if style_bits & 4 != 0 {
                style = style.add_modifier(Modifier::ITALIC);
            }
            state.text_style = style;
        }
        DisplayCommand::ClearScreen => {
            state.upper_window_content.clear();
            state.lower_window_content.clear();
            state.lower_current_line.clear();
            // Don't restore upper window lines here - split_window will create the correct number
        }
        DisplayCommand::EraseLine => {
            // Erase from cursor to end of line in current window
            if state.current_window == 1 && state.upper_cursor_y < state.upper_window_lines {
                let line_idx = state.upper_cursor_y as usize;
                if line_idx < state.upper_window_content.len() {
                    let line = &mut state.upper_window_content[line_idx];
                    let cursor_pos = state.upper_cursor_x as usize;
                    if cursor_pos < line.len() {
                        line.truncate(cursor_pos);
                    }
                }
            }
            // Lower window uses streaming - no cursor-based line erasing
        }
        _ => {}
    }
    Ok(())
}

/// Format the status line
fn format_status_line(location: &str, score: i16, moves: u16, width: u16) -> String {
    let right_text = format!("Score: {score} Moves: {moves}");
    let available_width = width as usize;
    let right_len = right_text.len();

    // Truncate location if needed
    let location_max_len = available_width.saturating_sub(right_len + 2);
    let location_display = if location.len() > location_max_len {
        &location[..location_max_len]
    } else {
        location
    };

    // Build status line with padding
    let padding_len = available_width
        .saturating_sub(location_display.len())
        .saturating_sub(right_len);

    format!(
        "{}{:padding$}{}",
        location_display,
        "",
        right_text,
        padding = padding_len
    )
}

/// Process text with backspace characters, properly removing characters
/// This handles the backspace sequence "\x08 \x08" sent by input handlers
fn process_text_with_backspace(buffer: &mut String, text: &str) {
    for ch in text.chars() {
        if ch == '\x08' {
            // Backspace - remove last character
            buffer.pop();
        } else {
            // Regular character - add to buffer
            buffer.push(ch);
        }
    }
}

impl DisplayState {
    /// Render the current state to the terminal
    fn render(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.terminal.draw(|f| {
            // Clear the entire screen with black background
            f.render_widget(ratatui::widgets::Clear, f.size());
            let chunks = if self.upper_window_lines > 0 {
                let screen_rect = f.size();

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

            // Render upper window if present - treat as absolute character grid
            if self.upper_window_lines > 0 {
                // Render character grid with individual character placement
                for (line_idx, styled_line) in self.upper_window_content.iter().enumerate() {
                    if line_idx < chunks[0].height as usize {
                        let y = chunks[0].y + line_idx as u16;
                        for (col_idx, styled_char) in styled_line.iter().enumerate() {
                            if col_idx < chunks[0].width as usize {
                                let x = chunks[0].x + col_idx as u16;
                                let style = if styled_char.reverse_video {
                                    Style::default().add_modifier(Modifier::REVERSED)
                                } else {
                                    // Use normal colors for all characters
                                    Style::default().fg(Color::White).bg(Color::Black)
                                };
                                f.buffer_mut()
                                    .get_mut(x, y)
                                    .set_char(styled_char.ch)
                                    .set_style(style);
                            }
                        }
                    }
                }
            }

            // Render lower window as scrolling text
            let mut lower_lines = self.lower_window_content.clone();

            // Add current line being built (if any)
            if !self.lower_current_line.is_empty() {
                lower_lines.push(self.lower_current_line.clone());
            }

            let lower_text: Vec<Line> =
                lower_lines.iter().map(|s| Line::from(s.as_str())).collect();

            // Calculate scroll to keep all content visible, accounting for word wrapping
            // CRITICAL FIX (v0.5.0): This calculation now accounts for ratatui's automatic
            // word wrapping, which can cause logical lines to span multiple display lines.
            // Previous versions only counted logical lines, causing input prompts to be
            // lost below the viewport in games like AMFV when content filled small terminals.
            // Note: The prompt is part of the game content, not a separate UI element
            let available_lines = chunks[1].height as usize;
            let available_width = chunks[1].width as usize;

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

            let lower_paragraph = Paragraph::new(lower_text)
                .wrap(Wrap { trim: false }) // Don't trim - preserve spaces!
                .style(Style::default().bg(Color::Black).fg(Color::White))
                .scroll((scroll_offset as u16, 0));

            f.render_widget(lower_paragraph, chunks[1]);
        })?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_text_with_backspace() {
        let mut buffer = String::new();

        // Test normal text
        process_text_with_backspace(&mut buffer, "Hello");
        assert_eq!(buffer, "Hello");

        // Test backspace removing character
        process_text_with_backspace(&mut buffer, "\x08");
        assert_eq!(buffer, "Hell");

        // Test backspace sequence like input handlers send: "\x08 \x08"
        process_text_with_backspace(&mut buffer, "\x08 \x08");
        assert_eq!(buffer, "Hel"); // First \x08 removes 'l', space adds ' ', second \x08 removes ' '

        // Test backspace on empty buffer (should be safe)
        buffer.clear();
        process_text_with_backspace(&mut buffer, "\x08");
        assert_eq!(buffer, "");

        // Test mixed text and backspaces
        buffer.clear();
        process_text_with_backspace(&mut buffer, "AB\x08C");
        assert_eq!(buffer, "AC"); // AB, backspace removes B, C is added
    }
}
