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
    style::{Attribute, SetAttribute},
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
    current_style: u16,
    upper_window_has_content: bool,  // Track if upper window has styled content
    buffered_mode: bool,            // Track if we're in buffered mode
    lower_window_buffer: String,    // Buffer for lower window text when in buffered mode
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
            current_style: 0,
            upper_window_has_content: false,
            buffered_mode: true,  // Start in buffered mode (Z-Machine default)
            lower_window_buffer: String::new(),
        })
    }
    
    /// Flush the lower window buffer to the screen
    fn flush_lower_window_buffer(&mut self) -> Result<(), DisplayError> {
        if self.lower_window_buffer.is_empty() {
            return Ok(());
        }
        
        // Ensure cursor is positioned correctly for lower window
        if let Ok((_col, row)) = cursor::position() {
            if row < self.upper_window_lines {
                debug!("V4: Moving cursor from upper window area to lower window before flush");
                execute!(io::stdout(), MoveTo(0, self.upper_window_lines))?;
            }
        }
        
        // Print buffered content
        debug!("V4: About to flush buffer content: '{}'", 
               self.lower_window_buffer.chars().take(100).collect::<String>());
        print!("{}", self.lower_window_buffer);
        io::stdout().flush()?;
        debug!("V4: Flushed buffer content, clearing buffer");
        self.lower_window_buffer.clear();
        Ok(())
    }
    
    /// Refresh the upper window display
    fn refresh_upper_window(&mut self) -> Result<(), DisplayError> {
        if self.upper_window_lines == 0 {
            return Ok(());
        }
        
        debug!("V4: Refreshing upper window (lines={}, has_content={}, cursor=({},{}))", 
               self.upper_window_lines, self.upper_window_has_content,
               self.upper_cursor_x, self.upper_cursor_y);
        
        // Get current cursor position manually
        let current_pos = if self.current_window == 0 {
            // If we're in lower window, don't save cursor position
            // We'll position it correctly after refresh
            None
        } else {
            cursor::position().ok()
        };
        
        // Draw upper window
        for (i, line) in self.upper_window_buffer.iter().enumerate() {
            execute!(io::stdout(), MoveTo(0, i as u16))?;
            
            // Log what we're about to print
            if !line.trim().is_empty() {
                debug!("V4: Upper window line {}: '{}'", i, line.trim());
            }
            
            // Just print the line without any automatic styling
            print!("{}", line);
        }
        
        // Restore cursor position if we had one
        // But make sure we don't position cursor inside upper window if we're in lower window
        if let Some((col, row)) = current_pos {
            if self.current_window == 0 && row < self.upper_window_lines {
                // We're in lower window but cursor was in upper window area
                // Move cursor to start of lower window instead
                execute!(io::stdout(), MoveTo(0, self.upper_window_lines))?;
                debug!("V4: Adjusted cursor position from ({}, {}) to (0, {})", col, row, self.upper_window_lines);
            } else {
                execute!(io::stdout(), MoveTo(col, row))?;
            }
        } else if self.current_window == 0 {
            // We're in lower window and didn't save position
            // Position cursor at start of lower window
            execute!(io::stdout(), MoveTo(0, self.upper_window_lines))?;
            debug!("V4: Positioned cursor at start of lower window after refresh");
        }
        
        io::stdout().flush()?;
        debug!("V4: Upper window refresh complete");
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
            
            debug!("V4: Before print - line length: {}, content: '{}'", line.len(), line);
            
            // Ensure line is long enough
            if line.len() < col {
                line.push_str(&" ".repeat(col - line.len()));
                debug!("V4: Padded line to length {}", line.len());
            }
            
            // Build new line with text at cursor position
            let mut new_line = String::new();
            if col > 0 {
                // Add the content before the cursor position
                new_line.push_str(&line[..col.min(line.len())]);
            }
            
            // Add the text at cursor position
            new_line.push_str(text);
            
            // Debug log if we're writing at column 0 and might be overwriting
            if col == 0 && !line.trim().is_empty() {
                debug!("V4: WARNING - Writing '{}' at column 0, potentially overwriting '{}'", 
                       text, line.trim());
            }
            
            // Update cursor position
            self.upper_cursor_x = (col + text.len()).min(self.terminal_width as usize) as u16;
            
            // Keep rest of line if it extends beyond the new text
            let text_end = col + text.len();
            if text_end < line.len() {
                new_line.push_str(&line[text_end..]);
            }
            
            // Only pad to terminal width if we don't have existing content beyond the new text
            // This prevents overwriting existing text when printing single characters
            if new_line.len() < self.terminal_width as usize {
                // Only pad if the original line was also padded to terminal width
                // or if we're starting from an empty line
                if line.len() == self.terminal_width as usize || line.trim().is_empty() {
                    new_line.push_str(&" ".repeat(self.terminal_width as usize - new_line.len()));
                }
            } else if new_line.len() > self.terminal_width as usize {
                // Truncate if line is too long
                new_line.truncate(self.terminal_width as usize);
            }
            
            debug!("V4: After print - new line content: '{}'", new_line);
            *line = new_line;
            self.upper_window_dirty = true;
            
            // Track if we're printing styled content
            if self.current_style != 0 {
                self.upper_window_has_content = true;
            }
            
            // Debug: Log the exact buffer state after each print
            if log::log_enabled!(log::Level::Trace) {
                log::trace!("V4: Upper window buffer after print:");
                for (i, buf_line) in self.upper_window_buffer.iter().enumerate() {
                    if !buf_line.trim().is_empty() {
                        log::trace!("  Line {}: '{}'", i, buf_line);
                    }
                }
            }
        }
        
        Ok(())
    }
}

impl ZMachineDisplay for V4Display {
    fn clear_screen(&mut self) -> Result<(), DisplayError> {
        debug!("V4: clear_screen");
        execute!(io::stdout(), Clear(ClearType::All), MoveTo(0, 0))?;
        // Reset any active text styles
        execute!(io::stdout(), SetAttribute(Attribute::Reset))?;
        io::stdout().flush()?;
        Ok(())
    }
    
    fn split_window(&mut self, lines: u16) -> Result<(), DisplayError> {
        debug!("V4: split_window({}) - previous lines: {}, current window: {}", 
               lines, self.upper_window_lines, self.current_window);
        
        if lines != self.upper_window_lines {
            self.upper_window_lines = lines;
            
            // Initialize buffer for upper window
            self.upper_window_buffer.clear();
            for _ in 0..lines {
                self.upper_window_buffer.push(" ".repeat(self.terminal_width as usize));
            }
            
            debug!("V4: split_window - initialized {} lines of buffer", lines);
            
            // Clear and redraw the upper window
            self.refresh_upper_window()?;
            
            // Ensure cursor is positioned correctly for lower window
            // After splitting, cursor should be at start of lower window
            if self.current_window == 0 {
                execute!(io::stdout(), MoveTo(0, self.upper_window_lines))?;
                debug!("V4: Positioned cursor at start of lower window after split");
            }
        }
        
        Ok(())
    }
    
    fn set_window(&mut self, window: u8) -> Result<(), DisplayError> {
        debug!("V4: set_window({}) from {}, upper_window_lines={}, dirty={}, cursor=({},{})", 
               window, self.current_window, self.upper_window_lines, self.upper_window_dirty,
               self.upper_cursor_x, self.upper_cursor_y);
        
        // Refresh upper window when switching away from it
        if self.current_window == 1 && window == 0 && self.upper_window_dirty {
            debug!("V4: Refreshing dirty upper window on switch to lower (cursor was at ({},{}))",
                   self.upper_cursor_x, self.upper_cursor_y);
            self.refresh_upper_window()?;
            self.upper_window_dirty = false;
        }
        
        // Don't refresh when switching TO upper window - this causes duplicate printing
        // Upper window will be refreshed when we switch away from it or explicitly refresh
        
        self.current_window = window;
        Ok(())
    }
    
    fn set_cursor(&mut self, line: u16, column: u16) -> Result<(), DisplayError> {
        debug!("V4: set_cursor({}, {}) in window {}", line, column, self.current_window);
        
        if self.current_window == 1 {
            // Upper window - store position for buffered printing
            // Handle edge case where game might pass 0 (though it shouldn't per spec)
            let safe_line = if line == 0 { 1 } else { line };
            let safe_column = if column == 0 { 1 } else { column };
            
            self.upper_cursor_y = (safe_line - 1).min(self.upper_window_lines - 1);
            self.upper_cursor_x = (safe_column - 1).min(self.terminal_width - 1);
            
            if line == 0 || column == 0 {
                debug!("V4: WARNING - set_cursor called with 0 position: ({}, {})", line, column);
            }
        } else {
            // Lower window - per Z-Machine spec section 8.7.2.3.1:
            // "set_cursor can only set the position of the cursor in the upper window,
            // and has no effect when the lower window is selected."
            debug!("V4: set_cursor ignored in lower window (per spec)");
        }
        
        Ok(())
    }
    
    fn print(&mut self, text: &str) -> Result<(), DisplayError> {
        let preview = text.chars().take(30).collect::<String>();
        debug!("V4: print('{}') to window {} with style {}, buffered={}, upper_lines={}", 
               preview, self.current_window, self.current_style, self.buffered_mode, self.upper_window_lines);
        
        if self.current_window == 1 {
            if self.upper_window_lines > 0 {
                // Print to upper window buffer
                debug!("V4: Printing to upper window buffer");
                self.print_to_upper_window(text)?;
            } else {
                debug!("V4: WARNING - trying to print to upper window but no lines allocated!");
                // Should we print to lower window or ignore?
            }
        } else {
            // Lower window - check if we're in buffered mode
            if self.buffered_mode {
                // In buffered mode - accumulate text in buffer and flush on newlines
                debug!("V4: Buffering lower window text: '{}' (total buffer length: {})", preview, self.lower_window_buffer.len());
                self.lower_window_buffer.push_str(text);
                
                // Flush buffer on newlines (for word-wrapping as per Z-Machine spec)
                if text.contains('\n') {
                    debug!("V4: Flushing buffered content on newline (length={}): '{}'", 
                           self.lower_window_buffer.len(),
                           self.lower_window_buffer.chars().take(100).collect::<String>());
                    self.flush_lower_window_buffer()?;
                }
            } else {
                // Not buffered - print immediately
                // First ensure cursor is not in upper window area
                if let Ok((col, row)) = cursor::position() {
                    if row < self.upper_window_lines {
                        debug!("V4: WARNING - Cursor at ({}, {}) in upper window area while printing to lower window!", col, row);
                        // Move cursor to start of lower window
                        execute!(io::stdout(), MoveTo(0, self.upper_window_lines))?;
                    }
                }
                
                debug!("V4: Printing immediately to lower window: '{}'", preview);
                print!("{}", text);
                io::stdout().flush()?;
            }
        }
        Ok(())
    }
    
    fn print_char(&mut self, ch: char) -> Result<(), DisplayError> {
        debug!("V4: print_char('{}') to window {} at cursor ({}, {})", 
               ch, self.current_window, self.upper_cursor_x, self.upper_cursor_y);
        self.print(&ch.to_string())
    }
    
    fn erase_window(&mut self, window: i16) -> Result<(), DisplayError> {
        debug!("V4: erase_window({}) - current_window={}, upper_lines={}, has_content={}", 
               window, self.current_window, self.upper_window_lines, self.upper_window_has_content);
        
        match window {
            -1 => {
                // Erase whole screen - per spec section 8.7.3.3:
                // - Clear screen to background colour
                // - Collapse upper window to height 0
                // - Move cursor to bottom left (v4) or top left (v5+)
                // - Select lower window
                execute!(io::stdout(), Clear(ClearType::All))?;
                
                // Collapse upper window
                self.upper_window_lines = 0;
                self.upper_window_buffer.clear();
                self.upper_window_dirty = false;
                self.upper_window_has_content = false;
                
                // Select lower window
                self.current_window = 0;
                
                // Position cursor at bottom left for v4
                // (In v5+ it would be top left, but this is v4)
                execute!(io::stdout(), MoveTo(0, self.terminal_height - 1))?;
            }
            0 => {
                // Erase lower window
                debug!("V4: Erasing lower window only");
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
                debug!("V4: erase_window(1) - clearing upper window buffer and content flag");
                for (i, line) in self.upper_window_buffer.iter_mut().enumerate() {
                    debug!("V4: Clearing upper window line {} (was: '{}')", i, line.trim());
                    *line = " ".repeat(self.terminal_width as usize);
                }
                self.upper_cursor_x = 0;
                self.upper_cursor_y = 0;
                self.upper_window_dirty = true;
                self.upper_window_has_content = false;
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
        debug!("V4: erase_line called in window {} at cursor ({},{})", 
               self.current_window, self.upper_cursor_x, self.upper_cursor_y);
        
        if self.current_window == 1 {
            // In upper window, we need to update the buffer
            let line_idx = self.upper_cursor_y as usize;
            let col = self.upper_cursor_x as usize;
            
            if line_idx < self.upper_window_buffer.len() {
                let line = &mut self.upper_window_buffer[line_idx];
                // Clear from cursor to end of line
                if col < line.len() {
                    line.truncate(col);
                    // Pad to terminal width
                    line.push_str(&" ".repeat(self.terminal_width as usize - line.len()));
                }
                self.upper_window_dirty = true;
                debug!("V4: Erased from column {} in upper window line {}", col, line_idx);
            }
        } else {
            // In lower window, use terminal's erase line
            execute!(io::stdout(), Clear(ClearType::UntilNewLine))?;
        }
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
    
    fn set_buffer_mode(&mut self, buffered: bool) -> Result<(), DisplayError> {
        debug!("V4: set_buffer_mode({}) - was {}, buffer has: '{}'", 
               buffered, self.buffered_mode, 
               self.lower_window_buffer.chars().take(50).collect::<String>());
        
        if self.buffered_mode && !buffered {
            // Switching from buffered to unbuffered - flush any buffered content
            debug!("V4: Switching from buffered to unbuffered mode - flushing buffer");
            self.flush_lower_window_buffer()?;
        } else if !self.buffered_mode && buffered {
            // Switching from unbuffered to buffered - also flush any existing content
            debug!("V4: Switching from unbuffered to buffered mode - flushing buffer");
            self.flush_lower_window_buffer()?;
        }
        
        self.buffered_mode = buffered;
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
    
    fn set_text_style(&mut self, style: u16) -> Result<(), DisplayError> {
        debug!("V4: set_text_style({}) in window {}", style, self.current_window);
        self.current_style = style;
        
        // If we're in the lower window, apply style immediately
        if self.current_window == 0 {
            let style_str = if style == 0 {
                "\x1b[0m" // Reset to normal
            } else if style & 1 != 0 {
                "\x1b[7m" // Reverse video
            } else if style & 2 != 0 {
                "\x1b[1m" // Bold
            } else if style & 4 != 0 {
                "\x1b[3m" // Italic
            } else {
                ""
            };
            
            if !style_str.is_empty() {
                print!("{}", style_str);
                io::stdout().flush()?;
            }
        }
        // For upper window, styles need to be embedded in the text buffer
        // This is handled when text is printed
        
        Ok(())
    }
}

impl Drop for V4Display {
    fn drop(&mut self) {
        // Comprehensive terminal cleanup on exit
        let _ = execute!(
            io::stdout(),
            SetAttribute(Attribute::Reset),    // Reset all text attributes
            crossterm::cursor::Show,           // Show cursor
            MoveTo(0, self.terminal_height.saturating_sub(1)), // Move to bottom of screen
            Clear(ClearType::UntilNewLine),    // Clear to end of line
        );
        
        // Flush to ensure all cleanup is applied
        let _ = io::stdout().flush();
        
        debug!("V4Display: Terminal cleanup completed");
    }
}