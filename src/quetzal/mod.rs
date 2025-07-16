//! Quetzal save file format implementation
//! 
//! Quetzal is the standard save game format for Z-Machine interpreters.
//! It uses the IFF (Interchange File Format) chunk structure.

pub mod iff;
pub mod chunks;
pub mod compressed_memory;
pub mod save;
pub mod restore;

pub use save::SaveGame;
pub use restore::RestoreGame;