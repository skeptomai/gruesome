pub mod game;
pub mod instruction;
pub mod vm;

// Native interpreter - uses std::io for input/output
#[cfg(feature = "native")]
pub mod interpreter;

pub use self::game::*;
pub use self::instruction::*;
pub use self::vm::*;

#[cfg(feature = "native")]
pub use self::interpreter::*;
