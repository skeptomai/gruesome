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
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Wrap},
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
}

/// Internal display state managed by the display thread
struct DisplayState {
    /// Terminal instance
    terminal: Terminal<CrosstermBackend<Stdout>>,
    /// Number of lines in upper window
    upper_window_lines: u16,
    /// Current window (0 = lower, 1 = upper)
    current_window: u8,
    /// Upper window content
    upper_window_content: Vec<String>,
    /// Lower window content
    lower_window_content: Vec<String>,
    /// Cursor position in upper window
    upper_cursor_x: u16,
    upper_cursor_y: u16,
    /// Current text style
    text_style: Style,
    /// Terminal dimensions
    terminal_width: u16,
    terminal_height: u16,
}

impl RatatuiDisplay {
    /// Create a new Ratatui-based display
    pub fn new() -> Result<Self, String> {
        let (tx, rx) = mpsc::channel();

        // Spawn display thread
        let display_thread = thread::spawn(move || {
            if let Err(e) = run_display_thread(rx) {
                eprintln!("Display thread error: {e}");
            }
        });

        Ok(RatatuiDisplay {
            tx,
            display_thread: Some(display_thread),
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
        self.send_command(DisplayCommand::SetWindow(window))
    }

    /// Set cursor position (1-based)
    pub fn set_cursor(&mut self, line: u16, column: u16) -> Result<(), String> {
        debug!("set_cursor: line={}, column={}", line, column);
        self.send_command(DisplayCommand::SetCursor(line, column))
    }

    /// Print text to current window
    pub fn print(&mut self, text: &str) -> Result<(), String> {
        self.send_command(DisplayCommand::Print(text.to_string()))
    }

    /// Print a character to current window
    pub fn print_char(&mut self, ch: char) -> Result<(), String> {
        self.send_command(DisplayCommand::PrintChar(ch))
    }

    /// Erase a window
    pub fn erase_window(&mut self, window: i16) -> Result<(), String> {
        debug!("erase_window: {}", window);
        self.send_command(DisplayCommand::EraseWindow(window))
    }

    /// Show status line
    pub fn show_status(&mut self, location: &str, score: i16, moves: u16) -> Result<(), String> {
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
    pub fn set_buffer_mode(&mut self, _buffered: bool) -> Result<(), String> {
        // Ratatui handles buffering internally
        Ok(())
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
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;

    // Create display state
    let mut state = DisplayState {
        terminal,
        upper_window_lines: 0,
        current_window: 0,
        upper_window_content: vec![],
        lower_window_content: vec![],
        upper_cursor_x: 0,
        upper_cursor_y: 0,
        text_style: Style::default(),
        terminal_width: 0,
        terminal_height: 0,
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

    // Cleanup
    disable_raw_mode()?;
    execute!(state.terminal.backend_mut(), LeaveAlternateScreen)?;

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
                state.upper_window_content.push(String::new());
            }
        }
        DisplayCommand::SetWindow(window) => {
            state.current_window = window;
        }
        DisplayCommand::SetCursor(line, column) => {
            if state.current_window == 1 {
                state.upper_cursor_y = (line - 1).min(state.upper_window_lines - 1);
                state.upper_cursor_x = (column - 1).min(state.terminal_width - 1);
            }
        }
        DisplayCommand::Print(text) => {
            if state.current_window == 1 && state.upper_window_lines > 0 {
                // Print to upper window
                let y = state.upper_cursor_y as usize;
                let x = state.upper_cursor_x as usize;

                if y < state.upper_window_content.len() {
                    let line = &mut state.upper_window_content[y];

                    // Ensure line is long enough
                    if line.len() < x {
                        line.push_str(&" ".repeat(x - line.len()));
                    }

                    // Replace text at cursor position
                    let mut new_line = String::new();
                    if x > 0 {
                        new_line.push_str(&line[..x]);
                    }
                    new_line.push_str(&text);

                    // Update cursor position
                    state.upper_cursor_x =
                        (x + text.len()).min(state.terminal_width as usize - 1) as u16;

                    // Keep rest of line if it fits
                    if x + text.len() < line.len() {
                        new_line.push_str(&line[x + text.len()..]);
                    }

                    *line = new_line;
                }
            } else {
                // Print to lower window
                state.lower_window_content.push(text);
                // Keep only last N lines based on window size
                let max_lines = (state.terminal_height - state.upper_window_lines - 1) as usize;
                if state.lower_window_content.len() > max_lines * 2 {
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
                    state.upper_window_content.clear();
                    state.lower_window_content.clear();
                    for _ in 0..state.upper_window_lines {
                        state.upper_window_content.push(String::new());
                    }
                }
                0 => {
                    // Clear lower window
                    state.lower_window_content.clear();
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
                state.upper_window_content[0] = status;
            }
        }
        DisplayCommand::SetTextStyle(style_bits) => {
            let mut style = Style::default();
            if style_bits & 1 != 0 {
                style = style.add_modifier(Modifier::REVERSED);
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
            for _ in 0..state.upper_window_lines {
                state.upper_window_content.push(String::new());
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
            }
            // For lower window, we don't track cursor position precisely
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
            let chunks = if self.upper_window_lines > 0 {
                Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(self.upper_window_lines),
                        Constraint::Min(1),
                    ])
                    .split(f.size())
            } else {
                vec![Rect::default(), f.size()].into()
            };

            // Render upper window if present
            if self.upper_window_lines > 0 {
                let upper_text: Vec<Line> = self
                    .upper_window_content
                    .iter()
                    .map(|s| {
                        Line::from(vec![Span::styled(s, Style::default().bg(Color::DarkGray))])
                    })
                    .collect();

                let upper_paragraph = Paragraph::new(upper_text).wrap(Wrap { trim: false });

                f.render_widget(upper_paragraph, chunks[0]);
            }

            // Render lower window
            let lower_text: Vec<Line> = self
                .lower_window_content
                .iter()
                .map(|s| Line::from(s.as_str()))
                .collect();

            let lower_paragraph = Paragraph::new(lower_text)
                .wrap(Wrap { trim: true })
                .scroll((
                    self.lower_window_content
                        .len()
                        .saturating_sub(chunks[1].height as usize) as u16,
                    0,
                ));

            f.render_widget(lower_paragraph, chunks[1]);
        })?;

        Ok(())
    }
}
