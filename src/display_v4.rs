//! V4+ display implementation
//!
//! This implementation handles display for Z-Machine version 4+ games.
//! Key characteristics:
//! - Multi-line upper window support
//! - Buffered window content with deferred refresh
//! - Full cursor control
//! - Complex window management

use crate::display_trait::{DisplayError, ZMachineDisplay};
use crossterm::{
    cursor::{self, MoveTo},
    execute,
    style::{Attribute, Print, SetAttribute},
    terminal::{self, Clear, ClearType},
};
use log::debug;
use std::io::{self, Write};

pub struct V4Display {
    terminal_width: u16,
    terminal_height: u16,
    upper_window_lines: u16,
    upper_window_buffer: Vec<String>,
    upper_window_dirty: bool,
    current_window: u8,
    upper_cursor_x: u16,
    upper_cursor_y: u16,
}

impl V4Display {
    pub fn new() -> Result<Self, DisplayError> {
        let (width, height) = terminal::size().unwrap_or((80, 24));
        
        Ok(V4Display {
            terminal_width: width,
            terminal_height: height,
            upper_window_lines: 0,
            upper_window_buffer: Vec::new(),
            upper_window_dirty: false,
            current_window: 0,
            upper_cursor_x: 0,
            upper_cursor_y: 0,
        })
    }
    
    /// Refresh the upper window display
    fn refresh_upper_window(&mut self) -> Result<(), DisplayError> {
        if self.upper_window_lines == 0 {
            return Ok(());
        }
        
        debug!("V4: Refreshing upper window");
        
        // Get current cursor position manually
        let current_pos = cursor::position().ok();
        
        // Draw upper window with reverse video
        for (i, line) in self.upper_window_buffer.iter().enumerate() {
            execute!(
                io::stdout(),
                MoveTo(0, i as u16),
                SetAttribute(Attribute::Reverse),
                Print(line),
                SetAttribute(Attribute::Reset)
            )?;
        }
        
        // Restore cursor position if we had one
        if let Some((col, row)) = current_pos {
            execute!(io::stdout(), MoveTo(col, row))?;
        }
        
        io::stdout().flush()?;
        Ok(())
    }
    
    /// Print text to upper window at current cursor position
    fn print_to_upper_window(&mut self, text: &str) -> Result<(), DisplayError> {
        if self.upper_cursor_y >= self.upper_window_lines {
            debug!("V4: Cursor outside upper window bounds");
            return Ok(());
        }
        
        let line_idx = self.upper_cursor_y as usize;
        let col = self.upper_cursor_x as usize;
        
        debug!("V4: print_to_upper_window('{}') at ({}, {})", text, col, line_idx);
        
        if line_idx < self.upper_window_buffer.len() {
            let line = &mut self.upper_window_buffer[line_idx];
            
            // Ensure line is long enough
            if line.len() < col {
                line.push_str(&" ".repeat(col - line.len()));
            }
            
            // Build new line with text at cursor position
            let mut new_line = String::new();
            new_line.push_str(&line[..col]);
            new_line.push_str(text);
            
            // Update cursor position
            self.upper_cursor_x = (col + text.len()).min(self.terminal_width as usize) as u16;
            
            // Keep rest of line if it extends beyond the new text
            let text_end = col + text.len();
            if text_end < line.len() {
                new_line.push_str(&line[text_end..]);
            }
            
            // Ensure line is full width (prevents display artifacts)
            if new_line.len() < self.terminal_width as usize {
                new_line.push_str(&" ".repeat(self.terminal_width as usize - new_line.len()));
            }
            
            *line = new_line;
            self.upper_window_dirty = true;
        }
        
        Ok(())
    }
}

impl ZMachineDisplay for V4Display {
    fn clear_screen(&mut self) -> Result<(), DisplayError> {
        execute!(io::stdout(), Clear(ClearType::All), MoveTo(0, 0))?;
        io::stdout().flush()?;
        Ok(())
    }
    
    fn split_window(&mut self, lines: u16) -> Result<(), DisplayError> {
        debug!("V4: split_window({})", lines);
        
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
    
    fn set_window(&mut self, window: u8) -> Result<(), DisplayError> {
        debug!("V4: set_window({}) from {}", window, self.current_window);
        
        // Refresh upper window when switching away from it
        if self.current_window == 1 && window == 0 && self.upper_window_dirty {
            self.refresh_upper_window()?;
            self.upper_window_dirty = false;
        }
        
        // Also refresh when switching TO upper window if it's dirty
        // This helps games that don't follow strict patterns
        if self.current_window == 0 && window == 1 && self.upper_window_dirty {
            self.refresh_upper_window()?;
            self.upper_window_dirty = false;
        }
        
        self.current_window = window;
        Ok(())
    }
    
    fn set_cursor(&mut self, line: u16, column: u16) -> Result<(), DisplayError> {
        debug!("V4: set_cursor({}, {}) in window {}", line, column, self.current_window);
        
        if self.current_window == 1 {
            // Upper window - store position for buffered printing
            self.upper_cursor_y = (line - 1).min(self.upper_window_lines - 1);
            self.upper_cursor_x = (column - 1).min(self.terminal_width - 1);
        } else {
            // Lower window - move cursor directly
            let actual_line = if self.upper_window_lines > 0 {
                self.upper_window_lines + line - 1
            } else {
                line - 1
            };
            execute!(io::stdout(), MoveTo(column - 1, actual_line))?;
        }
        
        Ok(())
    }
    
    fn print(&mut self, text: &str) -> Result<(), DisplayError> {
        if self.current_window == 1 && self.upper_window_lines > 0 {
            // Print to upper window buffer
            self.print_to_upper_window(text)?;
        } else {
            // Print to lower window directly
            print!("{}", text);
            io::stdout().flush()?;
        }
        Ok(())
    }
    
    fn print_char(&mut self, ch: char) -> Result<(), DisplayError> {
        self.print(&ch.to_string())
    }
    
    fn erase_window(&mut self, window: i16) -> Result<(), DisplayError> {
        debug!("V4: erase_window({})", window);
        
        match window {
            -1 => {
                // Erase whole screen
                execute!(io::stdout(), Clear(ClearType::All))?;
            }
            0 => {
                // Erase lower window
                if self.upper_window_lines > 0 {
                    execute!(
                        io::stdout(),
                        MoveTo(0, self.upper_window_lines),
                        Clear(ClearType::FromCursorDown)
                    )?;
                }
            }
            1 => {
                // Erase upper window
                for line in &mut self.upper_window_buffer {
                    *line = " ".repeat(self.terminal_width as usize);
                }
                self.upper_cursor_x = 0;
                self.upper_cursor_y = 0;
                self.upper_window_dirty = true;
                self.refresh_upper_window()?;
            }
            _ => {}
        }
        
        Ok(())
    }
    
    fn handle_resize(&mut self, width: u16, height: u16) {
        self.terminal_width = width;
        self.terminal_height = height;
        
        // Resize upper window buffer
        for line in &mut self.upper_window_buffer {
            if line.len() > width as usize {
                line.truncate(width as usize);
            } else {
                line.push_str(&" ".repeat(width as usize - line.len()));
            }
        }
    }
    
    fn show_status(&mut self, _location: &str, _score: i16, _moves: u16) -> Result<(), DisplayError> {
        // V4+ games don't use show_status - they manage their own headers
        debug!("V4: show_status called but ignored (v4+ games manage their own)");
        Ok(())
    }
    
    // V4+ specific operations
    
    fn erase_line(&mut self) -> Result<(), DisplayError> {
        execute!(io::stdout(), Clear(ClearType::UntilNewLine))?;
        Ok(())
    }
    
    fn get_cursor(&mut self) -> Result<(u16, u16), DisplayError> {
        if self.current_window == 1 {
            // Upper window - return our tracked position
            Ok((self.upper_cursor_y + 1, self.upper_cursor_x + 1))
        } else {
            // Lower window - get from terminal
            if let Ok((col, row)) = cursor::position() {
                Ok((row + 1, col + 1))
            } else {
                Ok((1, 1))
            }
        }
    }
    
    fn set_buffer_mode(&mut self, _buffered: bool) -> Result<(), DisplayError> {
        // We don't implement true buffering, but flush on unbuffer
        if !_buffered {
            io::stdout().flush()?;
        }
        Ok(())
    }
    
    fn get_terminal_size(&self) -> (u16, u16) {
        (self.terminal_width, self.terminal_height)
    }
    
    fn force_refresh(&mut self) -> Result<(), DisplayError> {
        if self.upper_window_dirty {
            self.refresh_upper_window()?;
            self.upper_window_dirty = false;
        }
        io::stdout().flush()?;
        Ok(())
    }
}

impl Drop for V4Display {
    fn drop(&mut self) {
        // Reset terminal attributes on exit
        let _ = execute!(
            io::stdout(),
            SetAttribute(Attribute::Reset),
            crossterm::cursor::Show
        );
    }
}