//! V3 display implementation
//!
//! This implementation handles display for Z-Machine version 3 games.
//! Key characteristics:
//! - Simple status line (1 line)
//! - Immediate refresh on show_status
//! - No complex window buffering
//! - Direct terminal output

use crate::display_trait::{DisplayError, ZMachineDisplay};
use crossterm::{
    cursor::MoveTo,
    execute,
    style::{Attribute, Print, SetAttribute},
    terminal::{self, Clear, ClearType},
};
use log::debug;
use std::io::{self, Write};

pub struct V3Display {
    terminal_width: u16,
    terminal_height: u16,
    has_status_line: bool,
    current_window: u8,
}

impl V3Display {
    pub fn new() -> Result<Self, DisplayError> {
        let (width, height) = terminal::size().unwrap_or((120, 36));
        
        Ok(V3Display {
            terminal_width: width,
            terminal_height: height,
            has_status_line: false,
            current_window: 0,
        })
    }
    
    /// Draw the status line with the given content
    fn draw_status_line(&self, content: &str) -> Result<(), DisplayError> {
        // Save cursor position
        execute!(
            io::stdout(),
            crossterm::cursor::SavePosition,
            MoveTo(0, 0),
            SetAttribute(Attribute::Reverse),
            Print(content),
            SetAttribute(Attribute::Reset),
            crossterm::cursor::RestorePosition,
        )?;
        
        io::stdout().flush()?;
        Ok(())
    }
}

impl ZMachineDisplay for V3Display {
    fn clear_screen(&mut self) -> Result<(), DisplayError> {
        execute!(io::stdout(), Clear(ClearType::All), MoveTo(0, 0))?;
        io::stdout().flush()?;
        Ok(())
    }
    
    fn split_window(&mut self, lines: u16) -> Result<(), DisplayError> {
        debug!("V3: split_window({})", lines);
        
        if lines > 0 {
            self.has_status_line = true;
            // V3 only supports 1-line status window
            if lines != 1 {
                debug!("V3: Warning: requested {} lines but v3 only supports 1", lines);
            }
        } else {
            self.has_status_line = false;
        }
        
        Ok(())
    }
    
    fn set_window(&mut self, window: u8) -> Result<(), DisplayError> {
        debug!("V3: set_window({})", window);
        self.current_window = window;
        
        // V3 games rarely use set_window, but if they do, we handle it simply
        if window == 0 && self.has_status_line {
            // Make sure cursor is below status line
            execute!(io::stdout(), MoveTo(0, 1))?;
        }
        
        Ok(())
    }
    
    fn set_cursor(&mut self, line: u16, column: u16) -> Result<(), DisplayError> {
        debug!("V3: set_cursor({}, {})", line, column);
        
        // V3 games rarely use this, but Seastalker does for its sonar display
        if self.current_window == 1 {
            // For upper window, position is relative to top of screen
            execute!(io::stdout(), MoveTo(column.saturating_sub(1), line.saturating_sub(1)))?;
        }
        
        Ok(())
    }
    
    fn print(&mut self, text: &str) -> Result<(), DisplayError> {
        // V3 printing is simple - just output to terminal
        print!("{}", text);
        io::stdout().flush()?;
        Ok(())
    }
    
    fn print_char(&mut self, ch: char) -> Result<(), DisplayError> {
        print!("{}", ch);
        io::stdout().flush()?;
        Ok(())
    }
    
    fn erase_window(&mut self, window: i16) -> Result<(), DisplayError> {
        debug!("V3: erase_window({})", window);
        
        match window {
            -1 => {
                // Erase whole screen
                execute!(io::stdout(), Clear(ClearType::All), MoveTo(0, 0))?;
            }
            0 => {
                // Erase lower window (everything below status line)
                if self.has_status_line {
                    execute!(
                        io::stdout(),
                        MoveTo(0, 1),
                        Clear(ClearType::FromCursorDown)
                    )?;
                } else {
                    execute!(io::stdout(), Clear(ClearType::All))?;
                }
            }
            1 => {
                // Erase upper window (status line)
                if self.has_status_line {
                    let blank_line = " ".repeat(self.terminal_width as usize);
                    self.draw_status_line(&blank_line)?;
                }
            }
            _ => {}
        }
        
        io::stdout().flush()?;
        Ok(())
    }
    
    fn handle_resize(&mut self, width: u16, height: u16) {
        self.terminal_width = width;
        self.terminal_height = height;
    }
    
    fn show_status(&mut self, location: &str, score: i16, moves: u16) -> Result<(), DisplayError> {
        if !self.has_status_line {
            return Ok(());
        }
        
        debug!("V3: show_status('{}', {}, {})", location, score, moves);
        
        // Format the status line
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
        let status_line = format!(
            "{}{:padding$}{}",
            location_display,
            "",
            right_text,
            padding = padding_len
        );
        
        // Draw it immediately - this is the key difference from v4!
        self.draw_status_line(&status_line)?;
        
        Ok(())
    }
    
    // V4+ operations - not supported in v3
    
    fn erase_line(&mut self) -> Result<(), DisplayError> {
        Err(DisplayError::new("erase_line not supported in v3"))
    }
    
    fn get_cursor(&mut self) -> Result<(u16, u16), DisplayError> {
        Err(DisplayError::new("get_cursor not supported in v3"))
    }
    
    fn set_buffer_mode(&mut self, _buffered: bool) -> Result<(), DisplayError> {
        // V3 doesn't support buffer mode, just ignore
        Ok(())
    }
    
    fn get_terminal_size(&self) -> (u16, u16) {
        (self.terminal_width, self.terminal_height)
    }
    
    fn force_refresh(&mut self) -> Result<(), DisplayError> {
        io::stdout().flush()?;
        Ok(())
    }
    
    fn set_text_style(&mut self, style: u16) -> Result<(), DisplayError> {
        // V3 games use ANSI codes for text styling, primarily for status lines
        let style_str = if style == 0 {
            "\x1b[0m" // Reset to normal
        } else if style & 1 != 0 {
            "\x1b[7m" // Reverse video
        } else if style & 2 != 0 {
            "\x1b[1m" // Bold
        } else {
            ""
        };
        
        if !style_str.is_empty() {
            print!("{}", style_str);
            io::stdout().flush()?;
        }
        
        Ok(())
    }
}

impl Drop for V3Display {
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
        
        debug!("V3Display: Terminal cleanup completed");
    }
}