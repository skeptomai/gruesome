//! Display manager that creates appropriate display implementations
//!
//! This module is responsible for:
//! - Creating the right display implementation based on Z-Machine version
//! - Handling fallback between ratatui/terminal/headless modes
//! - Providing a unified interface to the interpreter

use crate::display_trait::{DisplayError, ZMachineDisplay};
use crate::display_v3::V3Display;
use crate::display_headless::HeadlessDisplay;
use crate::display_logging::LoggingDisplay;
use crate::display_ratatui::RatatuiDisplay;

use log::debug;

/// Display mode selection
#[derive(Debug, Clone)]
pub enum DisplayMode {
    /// Try ratatui, fallback to terminal, fallback to headless
    Auto,
    /// Force ratatui (fail if not available)  
    Ratatui,
    /// Force terminal-based display
    Terminal,
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
            has_color: std::env::var("COLORTERM").is_ok() 
                || std::env::var("TERM").map_or(false, |t| t.contains("color")),
            has_unicode: std::env::var("LANG").map_or(false, |lang| lang.contains("UTF-8")),
            is_interactive: atty::is(atty::Stream::Stdin) && atty::is(atty::Stream::Stdout),
        }
    }
    
    /// Check if ratatui is likely to work
    pub fn supports_ratatui(&self) -> bool {
        self.has_terminal && self.is_interactive
    }
    
    /// Check if terminal display is likely to work
    pub fn supports_terminal(&self) -> bool {
        // Be more permissive - try terminal display if we have any output
        // Even if stdout isn't detected as a terminal, we might still be able to use it
        true
    }
}

/// Create a display implementation based on version and mode
pub fn create_display(version: u8, mode: DisplayMode) -> Result<Box<dyn ZMachineDisplay>, DisplayError> {
    let caps = DisplayCapabilities::detect();
    debug!("Display capabilities: {:?}", caps);
    debug!("Creating display for Z-Machine version {} with mode {:?}", version, mode);
    
    let mut display: Box<dyn ZMachineDisplay> = match mode {
        DisplayMode::Auto => {
            // Try in order: ratatui -> v3-only fallback -> headless
            match create_ratatui_display(version) {
                Ok(display) => {
                    debug!("Using Ratatui display for version {}", version);
                    display
                }
                Err(e) => {
                    debug!("Ratatui failed ({}), trying fallback", e);
                    if version <= 3 {
                        match V3Display::new() {
                            Ok(v3_display) => {
                                debug!("Using V3 terminal fallback for version {}", version);
                                Box::new(v3_display)
                            }
                            Err(e) => {
                                debug!("V3 display failed ({}), falling back to headless", e);
                                Box::new(HeadlessDisplay::new()?)
                            }
                        }
                    } else {
                        debug!("No fallback available for v4+ games, using headless");
                        Box::new(HeadlessDisplay::new()?)
                    }
                }
            }
        }
        
        DisplayMode::Ratatui => {
            debug!("Forcing Ratatui display for version {}", version);
            create_ratatui_display(version)?
        }
        
        DisplayMode::Terminal => {
            if version <= 3 {
                debug!("Using V3 terminal display for version {}", version);
                Box::new(V3Display::new()?)
            } else {
                return Err(DisplayError::new("V4+ games require RatatuiDisplay - V4Display removed due to limitations"));
            }
        }
        
        DisplayMode::Headless => {
            debug!("Using headless display");
            Box::new(HeadlessDisplay::new()?)
        }
    };
    
    // Check if we should wrap with logging
    if std::env::var("DISPLAY_LOG").is_ok() {
        debug!("Wrapping display with logging");
        display = Box::new(LoggingDisplay::new(display));
    }
    
    Ok(display)
}

/// Create a ratatui-based display for the given version
fn create_ratatui_display(_version: u8) -> Result<Box<dyn ZMachineDisplay>, DisplayError> {
    debug!("Creating RatatuiDisplay for version {}", _version);
    let display = RatatuiDisplay::new()
        .map_err(|e| DisplayError::new(format!("Failed to create RatatuiDisplay: {}", e)))?;
    Ok(Box::new(display))
}