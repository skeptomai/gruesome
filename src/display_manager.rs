//! Display manager that creates appropriate display implementations
//!
//! This module provides version-aware display system selection for the Z-Machine interpreter.
//! It automatically chooses the optimal display system based on game version and environment:
//!
//! ## Display Strategy:
//! - **v3 games** (Zork I): Use simple terminal display for maximum compatibility
//! - **v4+ games** (AMFV): Use ratatui display for advanced windowing features
//! - **Fallbacks**: Graceful degradation to headless mode if displays fail
//!
//! ## Key Features:
//! - Automatic version detection and appropriate display selection
//! - Environment capability detection (TTY, color support, etc.)
//! - Comprehensive fallback chain for maximum compatibility
//! - Optional logging wrapper for debugging display operations

use crate::display_trait::{DisplayError, ZMachineDisplay};
use crate::display_v3::V3Display;
use crate::display_headless::HeadlessDisplay;
use crate::display_logging::LoggingDisplay;

#[cfg(feature = "use-ratatui")]
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
            // Version-aware display selection
            if version <= 3 {
                // v3 games work best with simple terminal display
                debug!("Auto mode: Using terminal display for v3 game");
                match create_terminal_display(version) {
                    Ok(display) => {
                        debug!("Using terminal display for v3");
                        display
                    }
                    Err(e) => {
                        debug!("Terminal display failed ({}), falling back to headless", e);
                        Box::new(HeadlessDisplay::new()?)
                    }
                }
            } else {
                // v4+ games need ratatui for advanced features
                #[cfg(feature = "use-ratatui")]
                {
                    debug!("Auto mode: Using ratatui display for v4+ game");
                    match create_ratatui_display(version) {
                        Ok(display) => {
                            debug!("Using Ratatui display for v4+");
                            display
                        }
                        Err(e) => {
                            debug!("Ratatui failed ({}), falling back to terminal for v4+", e);
                            match create_terminal_display(version) {
                                Ok(display) => {
                                    debug!("Using terminal display fallback for v4+");
                                    display
                                }
                                Err(e) => {
                                    debug!("Terminal display failed ({}), falling back to headless", e);
                                    Box::new(HeadlessDisplay::new()?)
                                }
                            }
                        }
                    }
                }
                #[cfg(not(feature = "use-ratatui"))]
                {
                    debug!("Auto mode: Ratatui not available, using terminal display for v4+ game");
                    match create_terminal_display(version) {
                        Ok(display) => {
                            debug!("Using terminal display for v4+ (no ratatui)");
                            display
                        }
                        Err(e) => {
                            debug!("Terminal display failed ({}), falling back to headless", e);
                            Box::new(HeadlessDisplay::new()?)
                        }
                    }
                }
            }
        }
        
        DisplayMode::Ratatui => {
            #[cfg(feature = "use-ratatui")]
            {
                debug!("Forcing Ratatui display for version {}", version);
                create_ratatui_display(version)?
            }
            #[cfg(not(feature = "use-ratatui"))]
            {
                return Err(DisplayError::new("Ratatui not available (feature disabled)"));
            }
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

/// Create a terminal-based display for the given version
fn create_terminal_display(version: u8) -> Result<Box<dyn ZMachineDisplay>, DisplayError> {
    if version <= 3 {
        debug!("Creating V3 terminal display");
        Ok(Box::new(V3Display::new()?))
    } else {
        debug!("V4+ games require ratatui - terminal display removed");
        Err(DisplayError::new("V4+ games require ratatui display"))
    }
}

/// Create a ratatui-based display for the given version
#[cfg(feature = "use-ratatui")]
fn create_ratatui_display(_version: u8) -> Result<Box<dyn ZMachineDisplay>, DisplayError> {
    debug!("Creating RatatuiDisplay for version {}", _version);
    let display = RatatuiDisplay::new()
        .map_err(|e| DisplayError::new(format!("Failed to create RatatuiDisplay: {}", e)))?;
    Ok(Box::new(display))
}