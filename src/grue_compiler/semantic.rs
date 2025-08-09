// Semantic Analysis (stub for now)

use crate::grue_compiler::ast::Program;
use crate::grue_compiler::error::CompilerError;

pub struct SemanticAnalyzer {
    // TODO: Add symbol tables, etc.
}

impl Default for SemanticAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl SemanticAnalyzer {
    pub fn new() -> Self {
        SemanticAnalyzer {}
    }

    pub fn analyze(&mut self, ast: Program) -> Result<Program, CompilerError> {
        // TODO: Implement semantic analysis
        Ok(ast)
    }
}
