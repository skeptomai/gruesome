//! Display manager that wraps either the basic or ratatui display implementation

use log::debug;

#[cfg(feature = "use-ratatui")]
use crate::display_ratatui::RatatuiDisplay;

#[cfg(not(feature = "use-ratatui"))]
use crate::display::Display as BasicDisplay;

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
}

/// Wrapper for display implementations
pub enum DisplayManager {
    #[cfg(feature = "use-ratatui")]
    Ratatui(RatatuiDisplay),
    #[cfg(not(feature = "use-ratatui"))]
    Basic(BasicDisplay),
}

impl DisplayManager {
    /// Create a new display manager
    pub fn new() -> Result<Self, String> {
        #[cfg(feature = "use-ratatui")]
        {
            debug!("Initializing Ratatui display");
            Ok(DisplayManager::Ratatui(RatatuiDisplay::new()?))
        }
        
        #[cfg(not(feature = "use-ratatui"))]
        {
            debug!("Initializing basic display");
            Ok(DisplayManager::Basic(BasicDisplay::new()?))
        }
    }
}

impl DisplayTrait for DisplayManager {
    fn clear_screen(&mut self) -> Result<(), String> {
        match self {
            #[cfg(feature = "use-ratatui")]
            DisplayManager::Ratatui(d) => d.clear_screen(),
            #[cfg(not(feature = "use-ratatui"))]
            DisplayManager::Basic(d) => d.clear_screen(),
        }
    }
    
    fn split_window(&mut self, lines: u16) -> Result<(), String> {
        match self {
            #[cfg(feature = "use-ratatui")]
            DisplayManager::Ratatui(d) => d.split_window(lines),
            #[cfg(not(feature = "use-ratatui"))]
            DisplayManager::Basic(d) => d.split_window(lines),
        }
    }
    
    fn set_window(&mut self, window: u8) -> Result<(), String> {
        match self {
            #[cfg(feature = "use-ratatui")]
            DisplayManager::Ratatui(d) => d.set_window(window),
            #[cfg(not(feature = "use-ratatui"))]
            DisplayManager::Basic(d) => d.set_window(window),
        }
    }
    
    fn set_cursor(&mut self, line: u16, column: u16) -> Result<(), String> {
        match self {
            #[cfg(feature = "use-ratatui")]
            DisplayManager::Ratatui(d) => d.set_cursor(line, column),
            #[cfg(not(feature = "use-ratatui"))]
            DisplayManager::Basic(d) => d.set_cursor(line, column),
        }
    }
    
    fn print(&mut self, text: &str) -> Result<(), String> {
        match self {
            #[cfg(feature = "use-ratatui")]
            DisplayManager::Ratatui(d) => d.print(text),
            #[cfg(not(feature = "use-ratatui"))]
            DisplayManager::Basic(d) => d.print(text),
        }
    }
    
    fn print_char(&mut self, ch: char) -> Result<(), String> {
        match self {
            #[cfg(feature = "use-ratatui")]
            DisplayManager::Ratatui(d) => d.print_char(ch),
            #[cfg(not(feature = "use-ratatui"))]
            DisplayManager::Basic(d) => d.print_char(ch),
        }
    }
    
    fn erase_window(&mut self, window: i16) -> Result<(), String> {
        match self {
            #[cfg(feature = "use-ratatui")]
            DisplayManager::Ratatui(d) => d.erase_window(window),
            #[cfg(not(feature = "use-ratatui"))]
            DisplayManager::Basic(d) => d.erase_window(window),
        }
    }
    
    fn show_status(&mut self, location: &str, score: i16, moves: u16) -> Result<(), String> {
        match self {
            #[cfg(feature = "use-ratatui")]
            DisplayManager::Ratatui(d) => d.show_status(location, score, moves),
            #[cfg(not(feature = "use-ratatui"))]
            DisplayManager::Basic(d) => d.show_status(location, score, moves),
        }
    }
    
    fn handle_resize(&mut self, new_width: u16, new_height: u16) {
        match self {
            #[cfg(feature = "use-ratatui")]
            DisplayManager::Ratatui(d) => d.handle_resize(new_width, new_height),
            #[cfg(not(feature = "use-ratatui"))]
            DisplayManager::Basic(d) => d.handle_resize(new_width, new_height),
        }
    }
}
