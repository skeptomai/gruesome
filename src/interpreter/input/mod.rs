// Native-only input implementations (use std::io)
#[cfg(feature = "native")]
pub mod input_v3;
#[cfg(feature = "native")]
pub mod input_v4;
#[cfg(feature = "native")]
pub mod timed_input;

#[cfg(feature = "native")]
pub use self::input_v3::*;
#[cfg(feature = "native")]
pub use self::input_v4::*;
#[cfg(feature = "native")]
pub use self::timed_input::*;

// WASM input is handled differently - input comes from JS callbacks
// The WASM module exposes a provide_input() function that JS calls
