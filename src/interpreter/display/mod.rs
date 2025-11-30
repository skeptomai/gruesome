// Core display trait - always available
pub mod display_trait;
pub use self::display_trait::*;

// Headless display - always available (no platform deps)
pub mod display_headless;
pub use self::display_headless::*;

// Native-only display implementations
#[cfg(feature = "native")]
pub mod display_crossterm;
#[cfg(feature = "native")]
pub mod display_logging;
#[cfg(feature = "native")]
pub mod display_manager;
#[cfg(feature = "native")]
pub mod display_ratatui;
#[cfg(feature = "native")]
pub mod display_v3;

#[cfg(feature = "native")]
pub use self::display_crossterm::*;
#[cfg(feature = "native")]
pub use self::display_logging::*;
#[cfg(feature = "native")]
pub use self::display_manager::*;
#[cfg(feature = "native")]
pub use self::display_ratatui::*;
#[cfg(feature = "native")]
pub use self::display_v3::*;

// WASM display implementation
#[cfg(feature = "wasm")]
pub mod display_wasm;
#[cfg(feature = "wasm")]
pub use self::display_wasm::*;
