//! Quetzal save file format implementation
//!
//! Quetzal is the standard save game format for Z-Machine interpreters.
//! It uses the IFF (Interchange File Format) chunk structure.

pub mod chunks;
pub mod compressed_memory;
pub mod iff;
pub mod restore;
pub mod save;

pub use restore::RestoreGame;
pub use save::SaveGame;
