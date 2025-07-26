//! Direct crossterm-based display for Z-Machine character-by-character printing
//!
//! This implementation follows the Z-Machine specification exactly:
//! - Character-by-character printing to cursor positions
//! - Automatic scrolling when cursor reaches bottom-right
//! - Direct cursor control for upper window positioning
//! - Immediate display updates (no batching)

use crossterm::{
    cursor::{Hide, MoveTo, Show},
    execute, queue,
    style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor},
    terminal::{
        self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen, ScrollUp,
    },
};
use log::debug;
use std::io::{self, Stdout, Write};

use crate::display_trait::{DisplayError, ZMachineDisplay};

/// Direct crossterm display implementation
pub struct CrosstermDisplay {
    /// Terminal output handle
    stdout: Stdout,
    /// Current cursor position (0-based)
    cursor_row: u16,
    cursor_col: u16,
    /// Terminal dimensions (actual visible area)
    terminal_width: u16,
    terminal_height: u16,
    /// Coordinate offset to compensate for crossterm's scrollback positioning
    coordinate_offset: u16,
    /// Upper window height (0 = no upper window)
    upper_window_lines: u16,
    /// Current window (0 = lower, 1 = upper)
    current_window: u8,
    /// Upper window cursor position (0-based)
    upper_cursor_row: u16,
    upper_cursor_col: u16,
    /// Upper window content buffer for absolute positioning
    upper_window_content: Vec<Vec<char>>,
    /// Current text style flags
    reverse_video: bool,
}

impl CrosstermDisplay {
    /// Create a new crossterm-based display
    pub fn new() -> Result<Self, String> {
        let mut stdout = io::stdout();
        
        // Initialize terminal without alternate screen to avoid coordinate issues
        execute!(
            stdout,
            Hide,
            Clear(ClearType::All),
            MoveTo(0, 0)
        ).map_err(|e| format!("Failed to initialize terminal: {}", e))?;
        
        // Enable raw mode for direct input handling
        terminal::enable_raw_mode()
            .map_err(|e| format!("Failed to enable raw mode: {}", e))?;
        
        // Get terminal size - crossterm may return default 80x24 instead of actual size
        let (width, reported_height) = terminal::size()
            .map_err(|e| format!("Failed to get terminal size: {}", e))?;
        
        // Try to get actual terminal size using stty as fallback
        let actual_size = std::process::Command::new("stty")
            .arg("size")
            .output()
            .ok()
            .and_then(|output| {
                if output.status.success() {
                    let size_str = String::from_utf8_lossy(&output.stdout);
                    let parts: Vec<&str> = size_str.split_whitespace().collect();
                    if parts.len() == 2 {
                        if let (Ok(height), Ok(width)) = (parts[0].parse::<u16>(), parts[1].parse::<u16>()) {
                            return Some((width, height));
                        }
                    }
                }
                None
            });
        
        let (actual_width, actual_height) = actual_size.unwrap_or((width, reported_height));
        
        // Calculate coordinate offset - if crossterm reports different size than stty
        let coordinate_offset = reported_height.saturating_sub(actual_height);
        
        debug!("CrosstermDisplay: Crossterm reports {}x{}, actual visible {}x{}, offset: {}", 
               width, reported_height, actual_width, actual_height, coordinate_offset);
        
        Ok(CrosstermDisplay {
            stdout,
            cursor_row: coordinate_offset, // Start at visible area, not scrollback
            cursor_col: 0,
            terminal_width: actual_width,
            terminal_height: actual_height,
            coordinate_offset,
            upper_window_lines: 0,
            current_window: 0, // Start in lower window
            upper_cursor_row: 0,
            upper_cursor_col: 0,
            upper_window_content: Vec::new(),
            reverse_video: false,
        })
    }
    
    /// Apply coordinate offset to move to visible area instead of scrollback
    fn move_to_visible(&mut self, col: u16, row: u16) -> Result<(), String> {
        let visible_row = row + self.coordinate_offset;
        queue!(self.stdout, MoveTo(col, visible_row))
            .map_err(|e| format!("Failed to move cursor to visible area: {}", e))
    }
    
    /// Print a character at the current cursor position with automatic scrolling
    fn print_char_at_cursor(&mut self, ch: char) -> Result<(), String> {
        if self.current_window == 0 {
            // Lower window: character-by-character with automatic scrolling
            self.print_char_lower_window(ch)
        } else {
            // Upper window: absolute positioning, no scrolling
            self.print_char_upper_window(ch)
        }
    }
    
    /// Print character in lower window with Z-Machine scrolling behavior
    fn print_char_lower_window(&mut self, ch: char) -> Result<(), String> {
        match ch {
            '\n' => {
                // Newline: move to start of next line
                self.cursor_col = 0;
                self.cursor_row += 1;
                
                // Check if we need to scroll
                if self.cursor_row >= self.terminal_height - self.upper_window_lines {
                    self.scroll_lower_window_up(1)?;
                    self.cursor_row = self.terminal_height - self.upper_window_lines - 1;
                }
                
                self.move_to_visible(self.cursor_col, self.cursor_row)?;
            }
            '\x08' => {
                // Backspace: move cursor left
                if self.cursor_col > 0 {
                    self.cursor_col -= 1;
                    self.move_to_visible(self.cursor_col, self.cursor_row)?;
                    // Erase character at new position
                    queue!(self.stdout, Print(' '))
                        .map_err(|e| format!("Failed to print space: {}", e))?;
                    self.move_to_visible(self.cursor_col, self.cursor_row)?;
                }
            }
            _ => {
                // Regular character: print and advance cursor
                
                // Apply text style
                if self.reverse_video {
                    queue!(self.stdout, SetBackgroundColor(Color::White), SetForegroundColor(Color::Black))
                        .map_err(|e| format!("Failed to set reverse video: {}", e))?;
                } else {
                    queue!(self.stdout, SetBackgroundColor(Color::Black), SetForegroundColor(Color::White))
                        .map_err(|e| format!("Failed to set normal colors: {}", e))?;
                }
                
                self.move_to_visible(self.cursor_col, self.cursor_row)?;
                queue!(self.stdout, Print(ch))
                    .map_err(|e| format!("Failed to print character: {}", e))?;
                
                // Reset colors
                queue!(self.stdout, ResetColor)
                    .map_err(|e| format!("Failed to reset colors: {}", e))?;
                
                // Advance cursor
                self.cursor_col += 1;
                
                // Check for line wrap and scrolling
                if self.cursor_col >= self.terminal_width {
                    // Wrap to next line
                    self.cursor_col = 0;
                    self.cursor_row += 1;
                    
                    // Check if we need to scroll
                    if self.cursor_row >= self.terminal_height - self.upper_window_lines {
                        self.scroll_lower_window_up(1)?;
                        self.cursor_row = self.terminal_height - self.upper_window_lines - 1;
                    }
                }
            }
        }
        
        self.stdout.flush()
            .map_err(|e| format!("Failed to flush output: {}", e))?;
        
        Ok(())
    }
    
    /// Print character in upper window with absolute positioning
    fn print_char_upper_window(&mut self, ch: char) -> Result<(), String> {
        if self.upper_window_lines == 0 {
            debug!("Attempted to print '{}' to upper window but no upper window exists", ch);
            return Ok(()); // No upper window
        }
        
        debug!("Upper window: printing '{}' at ({}, {}) in {}-line upper window", 
               ch, self.upper_cursor_row, self.upper_cursor_col, self.upper_window_lines);
        
        match ch {
            '\n' => {
                // Move to start of next line in upper window
                self.upper_cursor_col = 0;
                if self.upper_cursor_row + 1 < self.upper_window_lines {
                    self.upper_cursor_row += 1;
                }
            }
            '\x08' => {
                // Backspace in upper window
                if self.upper_cursor_col > 0 {
                    self.upper_cursor_col -= 1;
                    // Update buffer and screen
                    if (self.upper_cursor_row as usize) < self.upper_window_content.len() {
                        let line = &mut self.upper_window_content[self.upper_cursor_row as usize];
                        if (self.upper_cursor_col as usize) < line.len() {
                            line[self.upper_cursor_col as usize] = ' ';
                        }
                    }
                    self.move_to_visible(self.upper_cursor_col, self.upper_cursor_row)?;
                    queue!(self.stdout, Print(' '))
                        .map_err(|e| format!("Failed to print space: {}", e))?;
                }
            }
            _ => {
                // Regular character in upper window
                
                // Ensure upper window buffer is large enough
                while self.upper_window_content.len() <= self.upper_cursor_row as usize {
                    self.upper_window_content.push(vec![' '; self.terminal_width as usize]);
                }
                
                // Update buffer
                let line = &mut self.upper_window_content[self.upper_cursor_row as usize];
                if (self.upper_cursor_col as usize) < line.len() {
                    line[self.upper_cursor_col as usize] = ch;
                }
                
                // Apply text style and print
                if self.reverse_video {
                    queue!(self.stdout, SetBackgroundColor(Color::White), SetForegroundColor(Color::Black))
                        .map_err(|e| format!("Failed to set reverse video: {}", e))?;
                } else {
                    queue!(self.stdout, SetBackgroundColor(Color::Black), SetForegroundColor(Color::White))
                        .map_err(|e| format!("Failed to set normal colors: {}", e))?;
                }
                
                self.move_to_visible(self.upper_cursor_col, self.upper_cursor_row)?;
                queue!(self.stdout, Print(ch))
                    .map_err(|e| format!("Failed to print character: {}", e))?;
                
                // Reset colors
                queue!(self.stdout, ResetColor)
                    .map_err(|e| format!("Failed to reset colors: {}", e))?;
                
                // Advance cursor (with bounds checking)
                self.upper_cursor_col += 1;
                if self.upper_cursor_col >= self.terminal_width {
                    // Don't auto-wrap in upper window per Z-Machine spec
                    self.upper_cursor_col = self.terminal_width - 1;
                }
            }
        }
        
        self.stdout.flush()
            .map_err(|e| format!("Failed to flush output: {}", e))?;
        
        Ok(())
    }
    
    /// Redraw the entire upper window from buffer
    fn redraw_upper_window(&mut self) -> Result<(), String> {
        if self.upper_window_lines == 0 {
            return Ok(());
        }
        
        debug!("Redrawing upper window ({} lines)", self.upper_window_lines);
        
        let lines_to_draw = self.upper_window_lines.min(self.upper_window_content.len() as u16);
        for row_idx in 0..lines_to_draw {
            self.move_to_visible(0, row_idx)?;
            
            // Set reverse video for the entire upper window (status line)
            queue!(self.stdout, SetBackgroundColor(Color::White), SetForegroundColor(Color::Black))
                .map_err(|e| format!("Failed to set reverse video: {}", e))?;
            
            if let Some(line) = self.upper_window_content.get(row_idx as usize) {
                for &ch in line.iter() {
                    queue!(self.stdout, Print(ch))
                        .map_err(|e| format!("Failed to print character: {}", e))?;
                }
            }
            
            // Reset colors at end of line
            queue!(self.stdout, ResetColor)
                .map_err(|e| format!("Failed to reset colors: {}", e))?;
        }
        
        self.stdout.flush()
            .map_err(|e| format!("Failed to flush upper window redraw: {}", e))?;
        
        Ok(())
    }
    
    /// Scroll the lower window up by the specified number of lines
    fn scroll_lower_window_up(&mut self, lines: u16) -> Result<(), String> {
        // Calculate the region to scroll (lower window only)
        let _scroll_top = self.upper_window_lines;
        let scroll_bottom = self.terminal_height - 1;
        
        // Use crossterm's scroll functionality
        for _ in 0..lines {
            // Move to the scroll region and scroll up
            self.move_to_visible(0, scroll_bottom)?;
            queue!(self.stdout, ScrollUp(1))
                .map_err(|e| format!("Failed to scroll up: {}", e))?;
        }
        
        self.stdout.flush()
            .map_err(|e| format!("Failed to flush after scroll: {}", e))?;
        
        Ok(())
    }
}

impl ZMachineDisplay for CrosstermDisplay {
    fn clear_screen(&mut self) -> Result<(), DisplayError> {
        execute!(self.stdout, Clear(ClearType::All))
            .map_err(|e| DisplayError::new(format!("Failed to clear screen: {}", e)))?;
        
        // Reset all window state
        self.upper_window_lines = 0;
        self.upper_window_content.clear();
        self.upper_cursor_row = 0;
        self.upper_cursor_col = 0;
        self.current_window = 0; // Switch to lower window
        
        // Reset cursor to top-left for now (debug coordinate issue)
        self.cursor_row = 0;
        self.cursor_col = 0;
        
        self.move_to_visible(self.cursor_col, self.cursor_row)
            .map_err(|e| DisplayError::new(format!("Failed to move cursor: {}", e)))?;
        
        debug!("Clear screen: reset all window state, cursor at bottom-left");
        
        Ok(())
    }
    
    fn split_window(&mut self, lines: u16) -> Result<(), DisplayError> {
        self.upper_window_lines = lines;
        
        // Clear the upper window area on screen
        for row in 0..lines {
            self.move_to_visible(0, row)
                .map_err(|e| DisplayError::new(format!("Failed to move cursor: {}", e)))?;
            queue!(self.stdout, Clear(ClearType::CurrentLine))
                .map_err(|e| DisplayError::new(format!("Failed to clear line: {}", e)))?;
        }
        
        // Initialize upper window content buffer
        self.upper_window_content.clear();
        for _ in 0..lines {
            self.upper_window_content.push(vec![' '; self.terminal_width as usize]);
        }
        
        // Reset upper window cursor
        self.upper_cursor_row = 0;
        self.upper_cursor_col = 0;
        
        // Adjust lower window cursor if it's now in the upper window area
        let lower_window_top = self.upper_window_lines;
        if self.cursor_row < lower_window_top {
            self.cursor_row = lower_window_top;
            self.cursor_col = 0;
        }
        
        self.stdout.flush()
            .map_err(|e| DisplayError::new(format!("Failed to flush: {}", e)))?;
        
        debug!("Split window: {} lines, cleared upper area, lower window starts at row {}", lines, lower_window_top);
        
        Ok(())
    }
    
    fn set_window(&mut self, window: u8) -> Result<(), DisplayError> {
        let old_window = self.current_window;
        self.current_window = window;
        debug!("Set window: {} -> {} (upper_window_lines={})", old_window, window, self.upper_window_lines);
        
        // When switching from upper window back to lower window, redraw upper window
        // to ensure it remains visible
        if old_window == 1 && window == 0 && self.upper_window_lines > 0 {
            debug!("Switching from upper to lower window - redrawing upper window");
            self.redraw_upper_window()
                .map_err(|e| DisplayError::new(format!("Failed to redraw upper window: {}", e)))?;
        }
        
        Ok(())
    }
    
    fn set_cursor(&mut self, line: u16, column: u16) -> Result<(), DisplayError> {
        if self.current_window == 1 {
            // Upper window: 1-based to 0-based conversion
            self.upper_cursor_row = (line - 1).min(self.upper_window_lines - 1);
            self.upper_cursor_col = (column - 1).min(self.terminal_width - 1);
            debug!("Set upper window cursor: ({}, {})", self.upper_cursor_row, self.upper_cursor_col);
        } else {
            // Lower window cursor positioning not typically used in Z-Machine
            debug!("Lower window cursor positioning not supported");
        }
        Ok(())
    }
    
    fn print(&mut self, text: &str) -> Result<(), DisplayError> {
        debug!("Print to window {}: '{}'", self.current_window, text.replace('\n', "\\n"));
        for ch in text.chars() {
            self.print_char_at_cursor(ch)
                .map_err(|e| DisplayError::new(e))?;
        }
        Ok(())
    }
    
    fn print_char(&mut self, ch: char) -> Result<(), DisplayError> {
        self.print_char_at_cursor(ch)
            .map_err(|e| DisplayError::new(e))
    }
    
    fn erase_window(&mut self, window: i16) -> Result<(), DisplayError> {
        match window {
            -1 => {
                // Clear entire screen but preserve window structure
                debug!("Erase window -1: clearing entire screen content but preserving window structure");
                
                // Clear the screen
                execute!(self.stdout, Clear(ClearType::All))
                    .map_err(|e| DisplayError::new(format!("Failed to clear screen: {}", e)))?;
                
                // Clear upper window buffer but keep the window structure
                for line in &mut self.upper_window_content {
                    line.fill(' ');
                }
                
                // Reset cursors but preserve window lines
                self.upper_cursor_row = 0;
                self.upper_cursor_col = 0;
                self.cursor_row = self.terminal_height - 1;
                self.cursor_col = 0;
                self.current_window = 0; // Switch to lower window
                
                execute!(self.stdout, MoveTo(self.cursor_col, self.cursor_row))
                    .map_err(|e| DisplayError::new(format!("Failed to move cursor: {}", e)))?;
            }
            0 => {
                // Clear lower window
                let lower_start = self.upper_window_lines;
                debug!("Erase window 0: clearing lower window from row {} to {}", lower_start, self.terminal_height - 1);
                for row in lower_start..self.terminal_height {
                    self.move_to_visible(0, row)
                        .map_err(|e| DisplayError::new(format!("Failed to move cursor: {}", e)))?;
                    queue!(self.stdout, Clear(ClearType::CurrentLine))
                        .map_err(|e| DisplayError::new(format!("Failed to clear line: {}", e)))?;
                }
                
                // Reset lower window cursor to bottom of lower window area
                self.cursor_row = self.terminal_height - 1;
                self.cursor_col = 0;
                
                self.stdout.flush()
                    .map_err(|e| DisplayError::new(format!("Failed to flush: {}", e)))?;
            }
            1 => {
                // Clear upper window
                for row in 0..self.upper_window_lines {
                    self.move_to_visible(0, row)
                        .map_err(|e| DisplayError::new(format!("Failed to move cursor: {}", e)))?;
                    queue!(self.stdout, Clear(ClearType::CurrentLine))
                        .map_err(|e| DisplayError::new(format!("Failed to clear line: {}", e)))?;
                }
                
                // Clear upper window buffer
                for line in &mut self.upper_window_content {
                    line.fill(' ');
                }
                
                // Reset upper window cursor
                self.upper_cursor_row = 0;
                self.upper_cursor_col = 0;
                
                self.stdout.flush()
                    .map_err(|e| DisplayError::new(format!("Failed to flush: {}", e)))?;
            }
            _ => {}
        }
        
        debug!("Erase window: {}", window);
        Ok(())
    }
    
    fn handle_resize(&mut self, width: u16, height: u16) {
        self.terminal_width = width;
        self.terminal_height = height;
        
        // Adjust cursor positions if they're now out of bounds
        self.cursor_col = self.cursor_col.min(width - 1);
        self.cursor_row = self.cursor_row.min(height - 1);
        self.upper_cursor_col = self.upper_cursor_col.min(width - 1);
        self.upper_cursor_row = self.upper_cursor_row.min(self.upper_window_lines.saturating_sub(1));
        
        debug!("Terminal resized: {}x{}", width, height);
    }
    
    fn show_status(&mut self, location: &str, score: i16, moves: u16) -> Result<(), DisplayError> {
        // Status line is typically handled by the game itself in v4+ games
        // For v3 compatibility, we could implement it here, but AMFV doesn't need it
        debug!("Status: {} - Score: {}, Moves: {}", location, score, moves);
        Ok(())
    }
    
    fn erase_line(&mut self) -> Result<(), DisplayError> {
        if self.current_window == 1 {
            // Erase from cursor to end of line in upper window
            self.move_to_visible(self.upper_cursor_col, self.upper_cursor_row)
                .map_err(|e| DisplayError::new(format!("Failed to move cursor: {}", e)))?;
            queue!(self.stdout, Clear(ClearType::UntilNewLine))
                .map_err(|e| DisplayError::new(format!("Failed to clear line: {}", e)))?;
            
            // Update buffer
            if (self.upper_cursor_row as usize) < self.upper_window_content.len() {
                let line = &mut self.upper_window_content[self.upper_cursor_row as usize];
                for col in self.upper_cursor_col as usize..line.len() {
                    line[col] = ' ';
                }
            }
            
            self.stdout.flush()
                .map_err(|e| DisplayError::new(format!("Failed to flush: {}", e)))?;
        }
        
        Ok(())
    }
    
    fn get_cursor(&mut self) -> Result<(u16, u16), DisplayError> {
        if self.current_window == 1 {
            // Return 1-based coordinates for upper window
            Ok((self.upper_cursor_row + 1, self.upper_cursor_col + 1))
        } else {
            // Lower window cursor (1-based)
            Ok((self.cursor_row + 1, self.cursor_col + 1))
        }
    }
    
    fn set_buffer_mode(&mut self, _buffered: bool) -> Result<(), DisplayError> {
        // With character-by-character printing, buffering is handled by flush timing
        // We always flush immediately for real-time display
        Ok(())
    }
    
    fn get_terminal_size(&self) -> (u16, u16) {
        (self.terminal_width, self.terminal_height)
    }
    
    fn force_refresh(&mut self) -> Result<(), DisplayError> {
        self.stdout.flush()
            .map_err(|e| DisplayError::new(format!("Failed to flush: {}", e)))?;
        Ok(())
    }
    
    fn set_text_style(&mut self, style: u16) -> Result<(), DisplayError> {
        // Bit 0: Reverse video
        self.reverse_video = (style & 1) != 0;
        
        // Other styles (bold, italic) could be implemented here
        debug!("Set text style: 0x{:04x} (reverse: {})", style, self.reverse_video);
        Ok(())
    }
    
    fn print_input_echo(&mut self, text: &str) -> Result<(), DisplayError> {
        // Input echo uses the same character-by-character printing
        self.print(text)
    }
}

impl Drop for CrosstermDisplay {
    fn drop(&mut self) {
        // Clean up terminal state
        let _ = execute!(
            self.stdout,
            Show,
            ResetColor
        );
        let _ = terminal::disable_raw_mode();
    }
}