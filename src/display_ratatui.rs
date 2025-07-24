//! Ratatui-based display management for Z-Machine
//!
//! Provides efficient window management with proper double-buffering
//! and flicker-free updates for games like Seastalker.

use crossterm::{
    event::{self, Event},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use log::debug;
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    Terminal,
};
use std::io::{self, Stdout};
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
    EraseLine,  // v4+
    Quit,
}

/// Display manager using Ratatui
pub struct RatatuiDisplay {
    /// Channel to send commands to display thread
    tx: Sender<DisplayCommand>,
    /// Handle to display thread
    display_thread: Option<thread::JoinHandle<()>>,
    /// Buffer mode state
    buffered_mode: bool,
    /// Buffered content for lower window
    lower_window_buffer: String,
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
    /// Lower window content with style information (absolute character grid)
    lower_window_content: Vec<Vec<StyledChar>>,
    /// Cursor position in upper window
    upper_cursor_x: u16,
    upper_cursor_y: u16,
    /// Cursor position in lower window
    lower_cursor_x: u16,
    lower_cursor_y: u16,
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
        let display_thread = thread::spawn(move || {
            if let Err(e) = run_display_thread(rx) {
                debug!("Display thread error: {e}");
            }
        });

        Ok(RatatuiDisplay {
            tx,
            display_thread: Some(display_thread),
            buffered_mode: true,  // Z-Machine v4+ starts in buffered mode
            lower_window_buffer: String::new(),
            current_window: 0,    // Start in lower window
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
        debug!("split_window: {} lines", lines);
        self.send_command(DisplayCommand::SplitWindow(lines))
    }

    /// Set the current window
    pub fn set_window(&mut self, window: u8) -> Result<(), String> {
        debug!("set_window: {}", window);
        self.current_window = window;
        self.send_command(DisplayCommand::SetWindow(window))
    }

    /// Set cursor position (1-based)
    pub fn set_cursor(&mut self, line: u16, column: u16) -> Result<(), String> {
        debug!("set_cursor: line={}, column={}", line, column);
        self.send_command(DisplayCommand::SetCursor(line, column))
    }

    /// Print text to current window
    pub fn print(&mut self, text: &str) -> Result<(), String> {
        debug!("Ratatui: print('{}') to window {} (buffered={})", 
               text.chars().take(30).collect::<String>(), self.current_window, self.buffered_mode);
        
        if self.current_window == 0 && self.buffered_mode {
            // Lower window in buffered mode - accumulate in buffer
            debug!("Ratatui: Buffering lower window text: '{}' (buffer length: {})", 
                   text.chars().take(50).collect::<String>(), self.lower_window_buffer.len());
            self.lower_window_buffer.push_str(text);
            
            // Flush on newlines (Z-Machine buffering behavior)
            if text.contains('\n') {
                debug!("Ratatui: Flushing buffer on newline (length={}): '{}'", 
                       self.lower_window_buffer.len(),
                       self.lower_window_buffer.chars().take(100).collect::<String>());
                self.flush_lower_window_buffer()?;
            }
            Ok(())
        } else {
            // Upper window or unbuffered - send immediately
            self.send_command(DisplayCommand::Print(text.to_string()))
        }
    }

    /// Print a character to current window
    pub fn print_char(&mut self, ch: char) -> Result<(), String> {
        self.print(&ch.to_string())
    }
    
    /// Flush the lower window buffer to the display
    fn flush_lower_window_buffer(&mut self) -> Result<(), String> {
        if !self.lower_window_buffer.is_empty() {
            debug!("Ratatui: Flushing buffer content: '{}'", 
                   self.lower_window_buffer.chars().take(100).collect::<String>());
            self.send_command(DisplayCommand::Print(self.lower_window_buffer.clone()))?;
            self.lower_window_buffer.clear();
        }
        Ok(())
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
    pub fn show_status_with_version(&mut self, location: &str, score: i16, moves: u16, _version: u8) -> Result<(), String> {
        debug!(
            "show_status: location='{}', score={}, moves={}",
            location, score, moves
        );
        self.send_command(DisplayCommand::ShowStatus(
            location.to_string(),
            score,
            moves,
        ))
    }

    /// Set text style
    pub fn set_text_style(&mut self, style: u16) -> Result<(), String> {
        debug!("set_text_style: {}", style);
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
    /// Ratatui already buffers appropriately
    pub fn set_buffer_mode(&mut self, buffered: bool) -> Result<(), String> {
        debug!("Ratatui: set_buffer_mode({}) - was {}, buffer has: '{}'", 
               buffered, self.buffered_mode, 
               self.lower_window_buffer.chars().take(50).collect::<String>());
        
        if self.buffered_mode && !buffered {
            // Switching from buffered to unbuffered - flush buffer
            debug!("Ratatui: Switching from buffered to unbuffered mode - flushing buffer");
            self.flush_lower_window_buffer()?;
        } else if !self.buffered_mode && buffered {
            // Switching from unbuffered to buffered - also flush any existing content
            debug!("Ratatui: Switching from unbuffered to buffered mode - flushing buffer");
            self.flush_lower_window_buffer()?;
        }
        
        self.buffered_mode = buffered;
        Ok(())
    }

    /// Get terminal size
    pub fn get_terminal_size(&self) -> (u16, u16) {
        // Default size - ratatui will handle actual terminal size
        (80, 24)
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
        self.clear_screen().map_err(|e| DisplayError::new(e))
    }
    
    fn split_window(&mut self, lines: u16) -> Result<(), DisplayError> {
        self.split_window(lines).map_err(|e| DisplayError::new(e))
    }
    
    fn set_window(&mut self, window: u8) -> Result<(), DisplayError> {
        self.set_window(window).map_err(|e| DisplayError::new(e))
    }
    
    fn set_cursor(&mut self, line: u16, column: u16) -> Result<(), DisplayError> {
        self.set_cursor(line, column).map_err(|e| DisplayError::new(e))
    }
    
    fn print(&mut self, text: &str) -> Result<(), DisplayError> {
        self.print(text).map_err(|e| DisplayError::new(e))
    }
    
    fn print_char(&mut self, ch: char) -> Result<(), DisplayError> {
        self.print_char(ch).map_err(|e| DisplayError::new(e))
    }
    
    fn erase_window(&mut self, window: i16) -> Result<(), DisplayError> {
        self.erase_window(window).map_err(|e| DisplayError::new(e))
    }
    
    fn handle_resize(&mut self, width: u16, height: u16) {
        self.handle_resize(width, height);
    }
    
    fn show_status(&mut self, location: &str, score: i16, moves: u16) -> Result<(), DisplayError> {
        self.show_status(location, score, moves).map_err(|e| DisplayError::new(e))
    }
    
    fn erase_line(&mut self) -> Result<(), DisplayError> {
        self.erase_line().map_err(|e| DisplayError::new(e))
    }
    
    fn get_cursor(&mut self) -> Result<(u16, u16), DisplayError> {
        self.get_cursor().map_err(|e| DisplayError::new(e))
    }
    
    fn set_buffer_mode(&mut self, buffered: bool) -> Result<(), DisplayError> {
        self.set_buffer_mode(buffered).map_err(|e| DisplayError::new(e))
    }
    
    fn get_terminal_size(&self) -> (u16, u16) {
        self.get_terminal_size()
    }
    
    fn force_refresh(&mut self) -> Result<(), DisplayError> {
        self.force_refresh().map_err(|e| DisplayError::new(e))
    }
    
    fn set_text_style(&mut self, style: u16) -> Result<(), DisplayError> {
        self.set_text_style(style).map_err(|e| DisplayError::new(e))
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
    debug!("Ratatui: attempting to enable raw mode");
    if let Err(e) = enable_raw_mode() {
        debug!("Ratatui: raw mode failed: {}, trying to continue anyway", e);
        // Don't fail immediately - some terminals work without raw mode
        // We'll try to continue and see if basic terminal access works
    }
    let mut stdout = io::stdout();
    if let Err(e) = execute!(stdout, EnterAlternateScreen) {
        debug!("Ratatui: failed to enter alternate screen: {}, continuing without it", e);
        // Continue without alternate screen - may work in some environments
    }
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend).map_err(|e| format!("Failed to create terminal: {}", e))?;

    // Create display state
    let mut state = DisplayState {
        terminal,
        upper_window_lines: 0,
        current_window: 0,
        upper_window_content: vec![],
        lower_window_content: vec![],
        upper_cursor_x: 0,
        upper_cursor_y: 0,
        lower_cursor_x: 0,
        lower_cursor_y: 0,
        text_style: Style::default(),
        terminal_width: 0,
        terminal_height: 0,
        reverse_video_active: false,
    };

    // Get initial terminal size
    let size = state.terminal.size()?;
    state.terminal_width = size.width;
    state.terminal_height = size.height;

    // Initial render
    state.render()?;

    // Main event loop
    loop {
        // Check for display commands with timeout
        match rx.recv_timeout(Duration::from_millis(50)) {
            Ok(cmd) => {
                match cmd {
                    DisplayCommand::Quit => break,
                    _ => handle_command(&mut state, cmd)?,
                }
                state.render()?;
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                // Check for terminal resize events
                if event::poll(Duration::from_millis(0))? {
                    if let Event::Resize(width, height) = event::read()? {
                        state.terminal_width = width;
                        state.terminal_height = height;
                        state.render()?;
                    }
                }
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }

    // Cleanup - be resilient to failures
    if let Err(e) = disable_raw_mode() {
        debug!("Ratatui: failed to disable raw mode during cleanup: {}", e);
    }
    if let Err(e) = execute!(state.terminal.backend_mut(), LeaveAlternateScreen) {
        debug!("Ratatui: failed to leave alternate screen during cleanup: {}", e);
    }

    Ok(())
}

/// Handle a display command
fn handle_command(
    state: &mut DisplayState,
    cmd: DisplayCommand,
) -> Result<(), Box<dyn std::error::Error>> {
    match cmd {
        DisplayCommand::SplitWindow(lines) => {
            state.upper_window_lines = lines;
            // Initialize upper window content
            state.upper_window_content.clear();
            for _ in 0..lines {
                state.upper_window_content.push(Vec::new());
            }
        }
        DisplayCommand::SetWindow(window) => {
            state.current_window = window;
        }
        DisplayCommand::SetCursor(line, column) => {
            if state.current_window == 1 {
                state.upper_cursor_y = (line - 1).min(state.upper_window_lines - 1);
                state.upper_cursor_x = (column - 1).min(state.terminal_width - 1);
            } else {
                // Lower window cursor positioning
                state.lower_cursor_y = (line - 1).min((state.terminal_height - state.upper_window_lines) - 1);
                state.lower_cursor_x = (column - 1).min(state.terminal_width - 1);
                debug!("Lower window cursor set to ({}, {})", state.lower_cursor_x, state.lower_cursor_y);
            }
        }
        DisplayCommand::Print(text) => {
            debug!("Print: '{}' (reverse_video_active: {})", text, state.reverse_video_active);
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
                        if current_y >= state.upper_window_content.len() {
                            break; // Don't go beyond allocated lines
                        }
                    } else {
                        // Regular character
                        if current_y < state.upper_window_content.len() {
                            let line = &mut state.upper_window_content[current_y];

                            // Ensure line is long enough with spaces
                            while line.len() <= current_x {
                                line.push(StyledChar { ch: ' ', reverse_video: false });
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

                debug!("Upper window: cursor now at ({}, {})", state.upper_cursor_x, state.upper_cursor_y);
            } else {
                // Print to lower window using absolute character positioning
                debug!("Lower window: placing '{}' at cursor ({}, {})", text, state.lower_cursor_x, state.lower_cursor_y);
                
                let mut current_y = state.lower_cursor_y as usize;
                let mut current_x = state.lower_cursor_x as usize;
                let max_lines = (state.terminal_height - state.upper_window_lines) as usize;

                // Ensure we have enough lines in the lower window content
                while state.lower_window_content.len() <= current_y {
                    state.lower_window_content.push(Vec::new());
                }

                // Handle text with potential newlines
                for ch in text.chars() {
                    if ch == '\n' {
                        // Move to next line
                        current_y += 1;
                        current_x = 0;
                        
                        // Ensure we have enough lines
                        while state.lower_window_content.len() <= current_y {
                            state.lower_window_content.push(Vec::new());
                        }
                        
                        // Scroll if we're beyond the available window space
                        if current_y >= max_lines {
                            // Remove the first line and adjust current_y
                            if !state.lower_window_content.is_empty() {
                                state.lower_window_content.remove(0);
                                current_y = max_lines - 1;
                            }
                        }
                    } else {
                        // Regular character - place at absolute position
                        if current_y < state.lower_window_content.len() {
                            let line = &mut state.lower_window_content[current_y];

                            // Ensure line is long enough with spaces
                            while line.len() <= current_x {
                                line.push(StyledChar { ch: ' ', reverse_video: false });
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
                        }
                    }
                }

                // Update cursor position
                state.lower_cursor_y = current_y as u16;
                state.lower_cursor_x = current_x as u16;
                
                debug!("Lower window: cursor now at ({}, {})", state.lower_cursor_x, state.lower_cursor_y);
            }
        }
        DisplayCommand::PrintChar(ch) => {
            handle_command(state, DisplayCommand::Print(ch.to_string()))?;
        }
        DisplayCommand::EraseWindow(window) => {
            match window {
                -1 => {
                    // Clear entire screen
                    state.upper_window_content.clear();
                    state.lower_window_content.clear();
                    for _ in 0..state.upper_window_lines {
                        state.upper_window_content.push(Vec::new());
                    }
                }
                0 => {
                    // Clear lower window
                    state.lower_window_content.clear();
                    state.lower_cursor_x = 0;
                    state.lower_cursor_y = 0;
                }
                1 => {
                    // Clear upper window
                    for line in &mut state.upper_window_content {
                        line.clear();
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
                let styled_chars: Vec<StyledChar> = status.chars()
                    .map(|ch| StyledChar { ch, reverse_video: false })
                    .collect();
                state.upper_window_content[0] = styled_chars;
            }
        }
        DisplayCommand::SetTextStyle(style_bits) => {
            debug!("SetTextStyle called with bits: 0x{:04x}", style_bits);
            let mut style = Style::default();
            if style_bits & 1 != 0 {
                style = style.add_modifier(Modifier::REVERSED);
                state.reverse_video_active = true;
                debug!("Reverse video ENABLED");
            } else {
                state.reverse_video_active = false;
                debug!("Reverse video disabled");
            }
            if style_bits & 2 != 0 {
                style = style.add_modifier(Modifier::BOLD);
                debug!("Bold enabled");
            }
            if style_bits & 4 != 0 {
                style = style.add_modifier(Modifier::ITALIC);
                debug!("Italic enabled");
            }
            state.text_style = style;
        }
        DisplayCommand::ClearScreen => {
            state.upper_window_content.clear();
            state.lower_window_content.clear();
            for _ in 0..state.upper_window_lines {
                state.upper_window_content.push(Vec::new());
            }
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
            } else if state.current_window == 0 {
                // Erase from cursor to end of line in lower window
                let line_idx = state.lower_cursor_y as usize;
                if line_idx < state.lower_window_content.len() {
                    let line = &mut state.lower_window_content[line_idx];
                    let cursor_pos = state.lower_cursor_x as usize;
                    if cursor_pos < line.len() {
                        line.truncate(cursor_pos);
                    }
                }
            }
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

impl DisplayState {
    /// Render the current state to the terminal
    fn render(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.terminal.draw(|f| {
            // Clear the entire screen with black background
            f.render_widget(
                ratatui::widgets::Clear,
                f.size()
            );
            let chunks = if self.upper_window_lines > 0 {
                Layout::default()
                    .direction(Direction::Vertical)
                    .margin(0)  // No margin - use full screen
                    .constraints([
                        Constraint::Length(self.upper_window_lines),
                        Constraint::Min(0),  // Allow zero height if needed
                    ])
                    .split(f.size())
            } else {
                vec![f.size(), f.size()].into()  // Use full screen for lower window
            };

            // Render upper window if present - treat as absolute character grid
            if self.upper_window_lines > 0 {
                debug!("RATATUI rendering upper window with {} lines", self.upper_window_lines);
                let _upper_text: Vec<Line> = self
                    .upper_window_content
                    .iter()
                    .enumerate()
                    .map(|(line_idx, styled_line)| {
                        debug!("Rendering upper window line {}: {} styled chars", line_idx, styled_line.len());
                        
                        // Build a fixed-width line that represents the exact character grid
                        // Pad to full terminal width to preserve spacing
                        let mut grid_line = vec![StyledChar { ch: ' ', reverse_video: false }; self.terminal_width as usize];
                        
                        // Copy the actual characters from the styled line
                        for (col_idx, styled_char) in styled_line.iter().enumerate() {
                            if col_idx < grid_line.len() {
                                grid_line[col_idx] = styled_char.clone();
                            }
                        }
                        
                        // Convert to spans, preserving ALL characters including spaces
                        let mut spans = Vec::new();
                        let mut current_span_text = String::new();
                        let mut current_reverse = false;
                        
                        for styled_char in grid_line {
                            if styled_char.reverse_video != current_reverse {
                                // Style change - push current span if it has content
                                if !current_span_text.is_empty() {
                                    let style = if current_reverse {
                                        Style::default().add_modifier(Modifier::REVERSED)
                                    } else {
                                        Style::default().fg(Color::White)
                                    };
                                    spans.push(Span::styled(current_span_text, style));
                                    current_span_text = String::new();
                                }
                                current_reverse = styled_char.reverse_video;
                            }
                            current_span_text.push(styled_char.ch);
                        }
                        
                        // Push final span
                        if !current_span_text.is_empty() {
                            let style = if current_reverse {
                                Style::default().add_modifier(Modifier::REVERSED)
                            } else {
                                Style::default().fg(Color::White)
                            };
                            spans.push(Span::styled(current_span_text, style));
                        }
                        
                        if spans.is_empty() {
                            spans.push(Span::raw(""));
                        }
                        
                        Line::from(spans)
                    })
                    .collect();

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
                                    Style::default().fg(Color::White).bg(Color::Black)
                                };
                                f.buffer_mut().get_mut(x, y).set_char(styled_char.ch).set_style(style);
                            }
                        }
                    }
                }
            }

            // Render lower window using absolute character positioning
            for (line_idx, styled_line) in self.lower_window_content.iter().enumerate() {
                if line_idx < chunks[1].height as usize {
                    let y = chunks[1].y + line_idx as u16;
                    for (col_idx, styled_char) in styled_line.iter().enumerate() {
                        if col_idx < chunks[1].width as usize {
                            let x = chunks[1].x + col_idx as u16;
                            let style = if styled_char.reverse_video {
                                Style::default().add_modifier(Modifier::REVERSED)
                            } else {
                                Style::default().fg(Color::White).bg(Color::Black)
                            };
                            f.buffer_mut().get_mut(x, y).set_char(styled_char.ch).set_style(style);
                        }
                    }
                }
            }
        })?;

        Ok(())
    }
}
