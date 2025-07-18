//! Display management for Z-Machine
//! 
//! Handles screen splitting, cursor positioning, and window management
//! using crossterm for cross-platform terminal control.

use crossterm::{
    cursor::{self, MoveTo},
    execute,
    style::{self, Print},
    terminal::{self, Clear, ClearType},
};
use std::io::{self, Write};
use log::debug;

/// Display manager for Z-Machine screen handling
pub struct Display {
    /// Number of lines in the upper window (status line)
    upper_window_lines: u16,
    /// Current window (0 = lower/main, 1 = upper/status)
    current_window: u8,
    /// Terminal size
    terminal_width: u16,
    terminal_height: u16,
    /// Current cursor position in upper window
    upper_cursor_x: u16,
    upper_cursor_y: u16,
    /// Buffer for upper window content
    upper_window_buffer: Vec<String>,
}

impl Display {
    /// Create a new display manager
    pub fn new() -> Result<Self, String> {
        let (width, height) = terminal::size()
            .map_err(|e| format!("Failed to get terminal size: {}", e))?;
        
        Ok(Display {
            upper_window_lines: 0,
            current_window: 0,
            terminal_width: width,
            terminal_height: height,
            upper_cursor_x: 0,
            upper_cursor_y: 0,
            upper_window_buffer: Vec::new(),
        })
    }
    
    /// Split the screen into upper and lower windows
    /// 
    /// The upper window is used for the status line in v3 games
    pub fn split_window(&mut self, lines: u16) -> Result<(), String> {
        debug!("split_window: {} lines", lines);
        
        if lines != self.upper_window_lines {
            self.upper_window_lines = lines;
            
            // Initialize buffer for upper window
            self.upper_window_buffer.clear();
            for _ in 0..lines {
                self.upper_window_buffer.push(" ".repeat(self.terminal_width as usize));
            }
            
            // Clear and redraw the upper window
            self.refresh_upper_window()?;
        }
        
        Ok(())
    }
    
    /// Set the current window (0 = lower, 1 = upper)
    pub fn set_window(&mut self, window: u8) -> Result<(), String> {
        debug!("set_window: {}", window);
        self.current_window = window;
        Ok(())
    }
    
    /// Set cursor position (1-based coordinates)
    /// 
    /// In v3, this is only used for the upper window
    pub fn set_cursor(&mut self, line: u16, column: u16) -> Result<(), String> {
        debug!("set_cursor: line={}, column={}", line, column);
        
        if self.current_window == 1 {
            // Upper window - store position
            self.upper_cursor_y = (line - 1).min(self.upper_window_lines - 1);
            self.upper_cursor_x = (column - 1).min(self.terminal_width - 1);
        }
        
        Ok(())
    }
    
    /// Print text to the current window
    pub fn print(&mut self, text: &str) -> Result<(), String> {
        if self.current_window == 1 && self.upper_window_lines > 0 {
            // Print to upper window
            self.print_to_upper_window(text)?;
        } else {
            // Print to lower window (normal output)
            print!("{}", text);
            io::stdout().flush().ok();
        }
        Ok(())
    }
    
    /// Print a character to the current window
    pub fn print_char(&mut self, ch: char) -> Result<(), String> {
        self.print(&ch.to_string())
    }
    
    /// Erase the current window
    pub fn erase_window(&mut self, window: i16) -> Result<(), String> {
        debug!("erase_window: {}", window);
        
        match window {
            -1 => {
                // Erase whole screen
                execute!(io::stdout(), Clear(ClearType::All))
                    .map_err(|e| format!("Failed to clear screen: {}", e))?;
            }
            0 => {
                // Erase lower window
                if self.upper_window_lines > 0 {
                    // Move to start of lower window and clear from there
                    execute!(
                        io::stdout(),
                        MoveTo(0, self.upper_window_lines),
                        Clear(ClearType::FromCursorDown)
                    ).map_err(|e| format!("Failed to clear lower window: {}", e))?;
                }
            }
            1 => {
                // Erase upper window
                self.clear_upper_window()?;
            }
            _ => {}
        }
        
        Ok(())
    }
    
    /// Show status line (v3 specific)
    /// 
    /// This is typically called automatically by the game
    pub fn show_status(&mut self, location: &str, score: i16, moves: u16) -> Result<(), String> {
        if self.upper_window_lines == 0 {
            // No status line allocated
            return Ok(());
        }
        
        debug!("show_status: location='{}', score={}, moves={}", location, score, moves);
        
        // Format status line
        let right_text = format!("Score: {} Moves: {}", score, moves);
        let available_width = self.terminal_width as usize;
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
        let status_line = format!("{}{:padding$}{}", 
            location_display, 
            "", 
            right_text, 
            padding = padding_len
        );
        
        // Update the first line of upper window
        if !self.upper_window_buffer.is_empty() {
            self.upper_window_buffer[0] = status_line;
            self.refresh_upper_window()?;
        }
        
        Ok(())
    }
    
    /// Print text to upper window at current cursor position
    fn print_to_upper_window(&mut self, text: &str) -> Result<(), String> {
        if self.upper_cursor_y >= self.upper_window_lines {
            return Ok(());
        }
        
        let line_idx = self.upper_cursor_y as usize;
        let col = self.upper_cursor_x as usize;
        
        // Get current line
        if line_idx < self.upper_window_buffer.len() {
            let line = &mut self.upper_window_buffer[line_idx];
            
            // Ensure line is long enough
            if line.len() < col {
                line.push_str(&" ".repeat(col - line.len()));
            }
            
            // Replace characters at cursor position
            let mut new_line = String::new();
            new_line.push_str(&line[..col]);
            new_line.push_str(text);
            
            // Update cursor position
            self.upper_cursor_x = (col + text.len()).min(self.terminal_width as usize - 1) as u16;
            
            // Keep rest of line if it fits
            if col + text.len() < line.len() {
                new_line.push_str(&line[col + text.len()..]);
            }
            
            *line = new_line;
            self.refresh_upper_window()?;
        }
        
        Ok(())
    }
    
    /// Clear the upper window
    fn clear_upper_window(&mut self) -> Result<(), String> {
        for line in &mut self.upper_window_buffer {
            *line = " ".repeat(self.terminal_width as usize);
        }
        self.upper_cursor_x = 0;
        self.upper_cursor_y = 0;
        self.refresh_upper_window()?;
        Ok(())
    }
    
    /// Refresh the upper window display
    fn refresh_upper_window(&mut self) -> Result<(), String> {
        if self.upper_window_lines == 0 {
            return Ok(());
        }
        
        // Save current cursor position
        execute!(io::stdout(), cursor::SavePosition)
            .map_err(|e| format!("Failed to save cursor: {}", e))?;
        
        // Draw upper window with reverse video
        for (i, line) in self.upper_window_buffer.iter().enumerate() {
            execute!(
                io::stdout(),
                MoveTo(0, i as u16),
                style::SetAttribute(style::Attribute::Reverse),
                Print(line),
                style::SetAttribute(style::Attribute::Reset)
            ).map_err(|e| format!("Failed to draw upper window line: {}", e))?;
        }
        
        // Draw separator line if there's room
        if self.upper_window_lines > 0 {
            execute!(
                io::stdout(),
                MoveTo(0, self.upper_window_lines),
                style::SetAttribute(style::Attribute::Reset)
            ).map_err(|e| format!("Failed to reset attributes: {}", e))?;
        }
        
        // Restore cursor position
        execute!(io::stdout(), cursor::RestorePosition)
            .map_err(|e| format!("Failed to restore cursor: {}", e))?;
        
        io::stdout().flush().ok();
        Ok(())
    }
    
    /// Handle terminal resize
    pub fn handle_resize(&mut self, new_width: u16, new_height: u16) {
        self.terminal_width = new_width;
        self.terminal_height = new_height;
        
        // Resize upper window buffer
        for line in &mut self.upper_window_buffer {
            if line.len() > new_width as usize {
                line.truncate(new_width as usize);
            } else {
                line.push_str(&" ".repeat(new_width as usize - line.len()));
            }
        }
    }
}

impl Drop for Display {
    fn drop(&mut self) {
        // Reset terminal attributes on exit
        let _ = execute!(
            io::stdout(),
            style::SetAttribute(style::Attribute::Reset),
            cursor::Show
        );
    }
}