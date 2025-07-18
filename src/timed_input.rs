//! Timed input handling for Z-Machine
//! 
//! This module provides non-blocking, interruptible input using crossterm's
//! event system. It uses OS-level event notification (epoll/kqueue/IOCP)
//! rather than polling.

use std::io::{self, Write};
use std::time::{Duration, Instant};
use crossterm::{
    terminal::{self, EnableLineWrap, DisableLineWrap},
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    cursor,
};
use log::{debug, info};

pub struct TimedInput {
    /// Whether we're currently in raw mode
    in_raw_mode: bool,
    /// Current input buffer being built
    buffer: String,
    /// Cursor position in buffer
    cursor_pos: usize,
}

impl TimedInput {
    pub fn new() -> Self {
        TimedInput {
            in_raw_mode: false,
            buffer: String::new(),
            cursor_pos: 0,
        }
    }
    
    /// Ensure we're not in raw mode when dropping
    fn cleanup(&mut self) {
        if self.in_raw_mode {
            let _ = terminal::disable_raw_mode();
            let _ = execute!(io::stdout(), EnableLineWrap);
            self.in_raw_mode = false;
        }
    }
    
    /// Read a line of input with optional timer support
    /// 
    /// This uses crossterm's event system for true non-blocking I/O.
    /// The OS notifies us when input is available (no polling required).
    /// 
    /// Returns: (input_string, was_terminated_by_timer)
    pub fn read_line_with_timer(
        &mut self,
        time_tenths: u16,
        routine_addr: u16,
    ) -> Result<(String, bool), String> {
        debug!("read_line_with_timer called: time={} tenths ({}s), routine=0x{:04x}", 
               time_tenths, time_tenths as f32 / 10.0, routine_addr);
        
        // Check if stdin is a terminal
        if !atty::is(atty::Stream::Stdin) {
            // Not a terminal - use standard blocking read
            debug!("Input is piped/redirected - using standard read");
            let input = self.read_line_standard()?;
            return Ok((input, false));
        }
        
        // Terminal input - use non-blocking event-driven input
        debug!("Terminal input detected - using non-blocking event system");
        
        // Use the real non-blocking implementation
        self.read_line_nonblocking(time_tenths, routine_addr)
    }
    
    /// Read a line without any timer support (basic mode)
    pub fn read_line_basic(&mut self) -> Result<String, String> {
        // Just use the timer version with no timeout
        let (input, _) = self.read_line_with_timer(0, 0)?;
        Ok(input)
    }
    
    /// Standard blocking line read (for non-terminal input)
    fn read_line_standard(&self) -> Result<String, String> {
        let mut input = String::new();
        io::stdin()
            .read_line(&mut input)
            .map_err(|e| format!("Failed to read input: {}", e))?;
        
        // Remove trailing newline
        if input.ends_with('\n') {
            input.pop();
            if input.ends_with('\r') {
                input.pop();
            }
        }
        
        Ok(input)
    }
    
    /// Read line using non-blocking event-driven I/O
    /// 
    /// This uses crossterm's event system which leverages OS-level
    /// event notification (epoll on Linux, kqueue on macOS, IOCP on Windows)
    fn read_line_nonblocking(
        &mut self,
        time_tenths: u16,
        routine_addr: u16,
    ) -> Result<(String, bool), String> {
        debug!("Entering non-blocking input mode");
        
        // Clear buffer for new input
        self.buffer.clear();
        self.cursor_pos = 0;
        
        // Enable raw mode for character-by-character input
        terminal::enable_raw_mode()
            .map_err(|e| format!("Failed to enable raw mode: {}", e))?;
        self.in_raw_mode = true;
        
        // Disable line wrap for cleaner display
        execute!(io::stdout(), DisableLineWrap)
            .map_err(|e| format!("Failed to disable line wrap: {}", e))?;
        
        // Calculate timeout if specified
        let timeout = if time_tenths > 0 {
            Some(Duration::from_millis((time_tenths as u64) * 100))
        } else {
            None
        };
        let start_time = Instant::now();
        
        info!("Non-blocking input active. Timeout: {:?}, routine: 0x{:04x}", timeout, routine_addr);
        
        let result = loop {
            // Check for timeout
            if let Some(timeout_duration) = timeout {
                if start_time.elapsed() >= timeout_duration {
                    debug!("Timer expired after {:?}", start_time.elapsed());
                    // In a real implementation, we would call the timer routine here
                    // For now, just return the partial input
                    break Ok((self.buffer.clone(), true));
                }
            }
            
            // Wait for next event with a small timeout to check timer
            // This is NOT polling - event::poll blocks until an event arrives
            // or the timeout expires. The OS wakes us up when input is ready.
            let poll_timeout = if timeout.is_some() {
                Duration::from_millis(100) // Check timer every 100ms
            } else {
                Duration::from_secs(3600) // Effectively infinite
            };
            
            if event::poll(poll_timeout).map_err(|e| format!("Event poll error: {}", e))? {
                // We have an event - process it
                match event::read().map_err(|e| format!("Event read error: {}", e))? {
                    Event::Key(key_event) => {
                        if let Some(result) = self.handle_key_event(key_event)? {
                            break Ok((result, false));
                        }
                    }
                    Event::Mouse(_) => {
                        // Ignore mouse events
                    }
                    Event::Resize(_, _) => {
                        // Handle terminal resize if needed
                    }
                    Event::FocusGained | Event::FocusLost => {
                        // Ignore focus events
                    }
                    Event::Paste(text) => {
                        // Handle pasted text
                        for ch in text.chars() {
                            self.buffer.insert(self.cursor_pos, ch);
                            self.cursor_pos += 1;
                        }
                        // Echo the pasted text
                        print!("{}", text);
                        io::stdout().flush().ok();
                    }
                }
            }
            // If poll times out, we loop back to check the timer
        };
        
        // Clean up
        self.cleanup();
        
        // Print newline after input
        println!();
        io::stdout().flush().ok();
        
        result
    }
    
    /// Handle a key event, returning Some(line) if input is complete
    fn handle_key_event(&mut self, key: KeyEvent) -> Result<Option<String>, String> {
        match key.code {
            KeyCode::Enter => {
                // Line complete
                debug!("Enter pressed, returning: '{}'", self.buffer);
                Ok(Some(self.buffer.clone()))
            }
            KeyCode::Char(c) => {
                // Handle Ctrl+C
                if key.modifiers.contains(KeyModifiers::CONTROL) && c == 'c' {
                    debug!("Ctrl+C pressed");
                    return Err("Interrupted by Ctrl+C".to_string());
                }
                
                // Add character to buffer
                self.buffer.insert(self.cursor_pos, c);
                self.cursor_pos += 1;
                
                // Echo the character
                print!("{}", c);
                io::stdout().flush().ok();
                
                Ok(None)
            }
            KeyCode::Backspace => {
                if self.cursor_pos > 0 {
                    self.cursor_pos -= 1;
                    self.buffer.remove(self.cursor_pos);
                    
                    // Move cursor back and clear to end of line
                    execute!(
                        io::stdout(),
                        cursor::MoveLeft(1),
                        crossterm::terminal::Clear(crossterm::terminal::ClearType::UntilNewLine)
                    ).ok();
                    
                    // Reprint the rest of the buffer
                    print!("{}", &self.buffer[self.cursor_pos..]);
                    
                    // Move cursor back to position
                    if self.buffer.len() > self.cursor_pos {
                        execute!(
                            io::stdout(),
                            cursor::MoveLeft((self.buffer.len() - self.cursor_pos) as u16)
                        ).ok();
                    }
                    
                    io::stdout().flush().ok();
                }
                Ok(None)
            }
            KeyCode::Left => {
                if self.cursor_pos > 0 {
                    self.cursor_pos -= 1;
                    execute!(io::stdout(), cursor::MoveLeft(1)).ok();
                }
                Ok(None)
            }
            KeyCode::Right => {
                if self.cursor_pos < self.buffer.len() {
                    self.cursor_pos += 1;
                    execute!(io::stdout(), cursor::MoveRight(1)).ok();
                }
                Ok(None)
            }
            _ => {
                // Ignore other keys
                Ok(None)
            }
        }
    }
    
    
    /// Read a single character with optional timeout
    /// 
    /// For future implementation of read_char opcode
    pub fn read_char_with_timeout(
        &mut self,
        _timeout_tenths: u16,
    ) -> Result<Option<char>, String> {
        // Placeholder for read_char implementation
        Err("read_char not implemented yet".to_string())
    }
}

impl Drop for TimedInput {
    fn drop(&mut self) {
        self.cleanup();
    }
}

