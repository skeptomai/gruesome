//! Quetzal save file format implementation
//!
//! Quetzal is the standard save game format for Z-Machine interpreters.
//! It uses the IFF (Interchange File Format) chunk structure.

// Core chunks and compression - always available
pub mod chunks;
pub mod compressed_memory;

// Byte-based save/restore - works in both native and WASM
pub mod save_bytes;
pub use save_bytes::{restore_from_bytes, save_to_bytes};

// File I/O based save/restore - native only
#[cfg(feature = "native")]
pub mod iff;
#[cfg(feature = "native")]
pub mod restore;
#[cfg(feature = "native")]
pub mod save;

#[cfg(feature = "native")]
pub use restore::RestoreGame;
#[cfg(feature = "native")]
pub use save::SaveGame;
