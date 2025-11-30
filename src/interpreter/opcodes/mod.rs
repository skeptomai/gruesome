// Opcode tables - always available for instruction decoding
pub mod opcode_tables;
pub use self::opcode_tables::*;

// Opcode implementations - native only (used by native interpreter)
#[cfg(feature = "native")]
pub mod opcodes_display;
#[cfg(feature = "native")]
pub mod opcodes_io;
#[cfg(feature = "native")]
pub mod opcodes_math;
#[cfg(feature = "native")]
pub mod opcodes_memory;
#[cfg(feature = "native")]
pub mod opcodes_object;
#[cfg(feature = "native")]
pub mod opcodes_stack;

#[cfg(feature = "native")]
pub use self::opcodes_display::*;
#[cfg(feature = "native")]
pub use self::opcodes_io::*;
#[cfg(feature = "native")]
pub use self::opcodes_math::*;
#[cfg(feature = "native")]
pub use self::opcodes_memory::*;
#[cfg(feature = "native")]
pub use self::opcodes_object::*;
#[cfg(feature = "native")]
pub use self::opcodes_stack::*;
