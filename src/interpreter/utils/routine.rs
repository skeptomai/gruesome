#![allow(unused_imports)]
use std::fmt::Debug;
use std::fmt::Display;
use std::fmt::Error;
use std::fmt::Formatter;

/// A Routine is an engine-executable section with local variables
pub struct Routine {
    start_position: usize,
    // Header
    num_local_variables: u8,
    local_variables: [u8; 16],
}

impl Routine {}

impl Display for Routine {
    fn fmt(&self, _f: &mut Formatter<'_>) -> Result<(), Error> {
        Ok(())
    }
}
