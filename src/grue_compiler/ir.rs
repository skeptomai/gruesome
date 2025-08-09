// Intermediate Representation (stub for now)

use crate::grue_compiler::ast::Program;
use crate::grue_compiler::error::CompilerError;

#[derive(Debug, Clone)]
pub struct IRProgram {
    // TODO: Add IR instructions
}

pub struct IRGenerator {
    // TODO: Add state
}

impl Default for IRGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl IRGenerator {
    pub fn new() -> Self {
        IRGenerator {}
    }

    pub fn generate(&mut self, _ast: Program) -> Result<IRProgram, CompilerError> {
        // TODO: Implement IR generation
        Ok(IRProgram {})
    }
}
