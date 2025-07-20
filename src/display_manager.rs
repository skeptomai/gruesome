//! Display manager that wraps display implementations with robust fallback

use log::debug;

#[cfg(feature = "use-ratatui")]
use crate::display_ratatui::RatatuiDisplay;

use crate::display::Display as BasicDisplay;

/// Display mode selection
#[derive(Debug, Clone)]
pub enum DisplayMode {
    /// Try ratatui, fallback to basic, fallback to headless
    Auto,
    /// Force ratatui (fail if not available)  
    Ratatui,
    /// Force basic terminal
    Basic,
    /// No display output (for testing/CI)
    Headless,
}

impl Default for DisplayMode {
    fn default() -> Self {
        DisplayMode::Auto
    }
}

/// Display environment capabilities
#[derive(Debug)]
pub struct DisplayCapabilities {
    pub has_terminal: bool,
    pub has_color: bool, 
    pub has_unicode: bool,
    pub is_interactive: bool,
}

impl DisplayCapabilities {
    /// Detect current environment capabilities
    pub fn detect() -> Self {
        Self {
            has_terminal: atty::is(atty::Stream::Stdout),
            has_color: std::env::var("COLORTERM").is_ok() || std::env::var("TERM").map_or(false, |t| t.contains("color")),
            has_unicode: std::env::var("LANG").map_or(false, |lang| lang.contains("UTF-8")),
            is_interactive: atty::is(atty::Stream::Stdin) && atty::is(atty::Stream::Stdout),
        }
    }
    
    /// Check if ratatui is likely to work
    pub fn supports_ratatui(&self) -> bool {
        self.has_terminal && self.is_interactive
    }
}

/// Display manager trait
pub trait DisplayTrait {
    fn clear_screen(&mut self) -> Result<(), String>;
    fn split_window(&mut self, lines: u16) -> Result<(), String>;
    fn set_window(&mut self, window: u8) -> Result<(), String>;
    fn set_cursor(&mut self, line: u16, column: u16) -> Result<(), String>;
    fn print(&mut self, text: &str) -> Result<(), String>;
    fn print_char(&mut self, ch: char) -> Result<(), String>;
    fn erase_window(&mut self, window: i16) -> Result<(), String>;
    fn show_status(&mut self, location: &str, score: i16, moves: u16) -> Result<(), String>;
    fn handle_resize(&mut self, new_width: u16, new_height: u16);
    
    // v4+ display opcodes
    fn erase_line(&mut self) -> Result<(), String>;
    fn get_cursor(&mut self) -> Result<(u16, u16), String>;
    fn set_buffer_mode(&mut self, buffered: bool) -> Result<(), String>;
}

/// Headless display for testing/CI environments
#[derive(Debug)]
pub struct HeadlessDisplay {
    buffer: Vec<String>,
    cursor: (u16, u16),
    upper_window_lines: u16,
    current_window: u8,
}

impl HeadlessDisplay {
    pub fn new() -> Result<Self, String> {
        Ok(Self {
            buffer: Vec::new(),
            cursor: (1, 1),
            upper_window_lines: 0,
            current_window: 0,
        })
    }
    
    /// Get the current buffer content (for testing)
    pub fn get_buffer(&self) -> &[String] {
        &self.buffer
    }
}

impl DisplayTrait for HeadlessDisplay {
    fn clear_screen(&mut self) -> Result<(), String> {
        self.buffer.clear();
        Ok(())
    }
    
    fn split_window(&mut self, lines: u16) -> Result<(), String> {
        self.upper_window_lines = lines;
        Ok(())
    }
    
    fn set_window(&mut self, window: u8) -> Result<(), String> {
        self.current_window = window;
        Ok(())
    }
    
    fn set_cursor(&mut self, line: u16, column: u16) -> Result<(), String> {
        self.cursor = (line, column);
        Ok(())
    }
    
    fn print(&mut self, text: &str) -> Result<(), String> {
        self.buffer.push(text.to_string());
        Ok(())
    }
    
    fn print_char(&mut self, ch: char) -> Result<(), String> {
        self.buffer.push(ch.to_string());
        Ok(())
    }
    
    fn erase_window(&mut self, _window: i16) -> Result<(), String> {
        self.buffer.clear();
        Ok(())
    }
    
    fn show_status(&mut self, location: &str, score: i16, moves: u16) -> Result<(), String> {
        let status = format!("{} Score: {} Moves: {}", location, score, moves);
        self.buffer.push(format!("[STATUS: {}]", status));
        Ok(())
    }
    
    fn handle_resize(&mut self, _new_width: u16, _new_height: u16) {
        // Nothing to do in headless mode
    }
    
    fn erase_line(&mut self) -> Result<(), String> {
        // Just track that it happened
        self.buffer.push("[ERASE_LINE]".to_string());
        Ok(())
    }
    
    fn get_cursor(&mut self) -> Result<(u16, u16), String> {
        Ok(self.cursor)
    }
    
    fn set_buffer_mode(&mut self, _buffered: bool) -> Result<(), String> {
        // Nothing to do in headless mode
        Ok(())
    }
}

/// Wrapper for display implementations
pub enum DisplayManager {
    #[cfg(feature = "use-ratatui")]
    Ratatui(RatatuiDisplay),
    Basic(BasicDisplay),
    Headless(HeadlessDisplay),
}

impl DisplayManager {
    /// Create a new display manager with auto mode (smart fallback)
    pub fn new() -> Result<Self, String> {
        Self::new_with_mode(DisplayMode::Auto)
    }
    
    /// Create a display manager with specific mode
    pub fn new_with_mode(mode: DisplayMode) -> Result<Self, String> {
        let caps = DisplayCapabilities::detect();
        debug!("Display capabilities: {:?}", caps);
        
        match mode {
            DisplayMode::Auto => {
                // Smart fallback: try ratatui -> basic -> headless
                #[cfg(feature = "use-ratatui")]
                if caps.supports_ratatui() {
                    match RatatuiDisplay::new() {
                        Ok(display) => {
                            debug!("Using Ratatui display (auto mode)");
                            return Ok(DisplayManager::Ratatui(display));
                        }
                        Err(e) => {
                            debug!("Ratatui failed ({}), falling back to basic display", e);
                        }
                    }
                }
                
                // Try basic display  
                if caps.has_terminal {
                    match BasicDisplay::new() {
                        Ok(display) => {
                            debug!("Using basic display (auto mode)");
                            return Ok(DisplayManager::Basic(display));
                        }
                        Err(e) => {
                            debug!("Basic display failed ({}), falling back to headless", e);
                        }
                    }
                }
                
                // Final fallback: headless
                debug!("Using headless display (auto mode fallback)");
                Ok(DisplayManager::Headless(HeadlessDisplay::new()?))
            }
            
            DisplayMode::Ratatui => {
                #[cfg(feature = "use-ratatui")]
                {
                    debug!("Forcing Ratatui display");
                    Ok(DisplayManager::Ratatui(RatatuiDisplay::new()?))
                }
                #[cfg(not(feature = "use-ratatui"))]
                {
                    Err("Ratatui not available (feature disabled)".to_string())
                }
            }
            
            DisplayMode::Basic => {
                debug!("Forcing basic display");
                Ok(DisplayManager::Basic(BasicDisplay::new()?))
            }
            
            DisplayMode::Headless => {
                debug!("Using headless display");
                Ok(DisplayManager::Headless(HeadlessDisplay::new()?))
            }
        }
    }
}

impl DisplayTrait for DisplayManager {
    fn clear_screen(&mut self) -> Result<(), String> {
        match self {
            #[cfg(feature = "use-ratatui")]
            DisplayManager::Ratatui(d) => d.clear_screen(),
            DisplayManager::Basic(d) => d.clear_screen(),
            DisplayManager::Headless(d) => d.clear_screen(),
        }
    }

    fn split_window(&mut self, lines: u16) -> Result<(), String> {
        match self {
            #[cfg(feature = "use-ratatui")]
            DisplayManager::Ratatui(d) => d.split_window(lines),
            DisplayManager::Basic(d) => d.split_window(lines),
            DisplayManager::Headless(d) => d.split_window(lines),
        }
    }

    fn set_window(&mut self, window: u8) -> Result<(), String> {
        match self {
            #[cfg(feature = "use-ratatui")]
            DisplayManager::Ratatui(d) => d.set_window(window),
            DisplayManager::Basic(d) => d.set_window(window),
            DisplayManager::Headless(d) => d.set_window(window),
        }
    }

    fn set_cursor(&mut self, line: u16, column: u16) -> Result<(), String> {
        match self {
            #[cfg(feature = "use-ratatui")]
            DisplayManager::Ratatui(d) => d.set_cursor(line, column),
            DisplayManager::Basic(d) => d.set_cursor(line, column),
            DisplayManager::Headless(d) => d.set_cursor(line, column),
        }
    }

    fn print(&mut self, text: &str) -> Result<(), String> {
        match self {
            #[cfg(feature = "use-ratatui")]
            DisplayManager::Ratatui(d) => d.print(text),
            DisplayManager::Basic(d) => d.print(text),
            DisplayManager::Headless(d) => d.print(text),
        }
    }

    fn print_char(&mut self, ch: char) -> Result<(), String> {
        match self {
            #[cfg(feature = "use-ratatui")]
            DisplayManager::Ratatui(d) => d.print_char(ch),
            DisplayManager::Basic(d) => d.print_char(ch),
            DisplayManager::Headless(d) => d.print_char(ch),
        }
    }

    fn erase_window(&mut self, window: i16) -> Result<(), String> {
        match self {
            #[cfg(feature = "use-ratatui")]
            DisplayManager::Ratatui(d) => d.erase_window(window),
            DisplayManager::Basic(d) => d.erase_window(window),
            DisplayManager::Headless(d) => d.erase_window(window),
        }
    }

    fn show_status(&mut self, location: &str, score: i16, moves: u16) -> Result<(), String> {
        match self {
            #[cfg(feature = "use-ratatui")]
            DisplayManager::Ratatui(d) => d.show_status(location, score, moves),
            DisplayManager::Basic(d) => d.show_status(location, score, moves),
            DisplayManager::Headless(d) => d.show_status(location, score, moves),
        }
    }

    fn handle_resize(&mut self, new_width: u16, new_height: u16) {
        match self {
            #[cfg(feature = "use-ratatui")]
            DisplayManager::Ratatui(d) => d.handle_resize(new_width, new_height),
            DisplayManager::Basic(d) => d.handle_resize(new_width, new_height),
            DisplayManager::Headless(d) => d.handle_resize(new_width, new_height),
        }
    }
    
    fn erase_line(&mut self) -> Result<(), String> {
        match self {
            #[cfg(feature = "use-ratatui")]
            DisplayManager::Ratatui(d) => d.erase_line(),
            DisplayManager::Basic(d) => d.erase_line(),
            DisplayManager::Headless(d) => d.erase_line(),
        }
    }
    
    fn get_cursor(&mut self) -> Result<(u16, u16), String> {
        match self {
            #[cfg(feature = "use-ratatui")]
            DisplayManager::Ratatui(d) => d.get_cursor(),
            DisplayManager::Basic(d) => d.get_cursor(),
            DisplayManager::Headless(d) => d.get_cursor(),
        }
    }
    
    fn set_buffer_mode(&mut self, buffered: bool) -> Result<(), String> {
        match self {
            #[cfg(feature = "use-ratatui")]
            DisplayManager::Ratatui(d) => d.set_buffer_mode(buffered),
            DisplayManager::Basic(d) => d.set_buffer_mode(buffered),
            DisplayManager::Headless(d) => d.set_buffer_mode(buffered),
        }
    }
}
