// Grue Compiler Module
// Main compiler infrastructure for the Grue language

pub mod ast;
pub mod codegen;
pub mod codegen_builtins;
pub mod codegen_instructions;
pub mod codegen_utils;
pub mod error;
pub mod ir;
pub mod lexer;
pub mod object_system;
pub mod parser;
pub mod semantic;

use std::fmt;

pub use error::CompilerError;

/// Z-Machine version enum
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ZMachineVersion {
    V3,
    V4,
    V5,
}

impl fmt::Display for ZMachineVersion {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ZMachineVersion::V3 => write!(f, "v3"),
            ZMachineVersion::V4 => write!(f, "v4"),
            ZMachineVersion::V5 => write!(f, "v5"),
        }
    }
}

/// Main compiler structure
pub struct GrueCompiler {
    // Compiler state and configuration
}

impl Default for GrueCompiler {
    fn default() -> Self {
        Self::new()
    }
}

impl GrueCompiler {
    /// Create a new compiler instance
    pub fn new() -> Self {
        GrueCompiler {}
    }

    /// Compile Grue source code to Z-Machine bytecode
    pub fn compile(
        &self,
        source: &str,
        version: ZMachineVersion,
    ) -> Result<Vec<u8>, CompilerError> {
        // Phase 1: Lexical Analysis
        let mut lexer = lexer::Lexer::new(source);
        let tokens = lexer.tokenize()?;

        // Phase 2: Parsing
        let mut parser = parser::Parser::new(tokens);
        let ast = parser.parse()?;

        // Phase 3: Semantic Analysis
        let mut analyzer = semantic::SemanticAnalyzer::new();
        let analyzed_ast = analyzer.analyze(ast)?;

        // Phase 4: IR Generation
        let mut ir_generator = ir::IrGenerator::new();
        let ir_program = ir_generator.generate(analyzed_ast)?;

        // Phase 5: Code Generation
        let mut code_generator = codegen::ZMachineCodeGen::new(version);

        // Transfer builtin function information from IR generator to code generator
        for (function_id, function_name) in ir_generator.get_builtin_functions() {
            code_generator.register_builtin_function(*function_id, function_name.clone());
        }

        // Transfer object numbers from IR generator to code generator
        code_generator.set_object_numbers(ir_generator.get_object_numbers().clone());

        let story_data = code_generator.generate_complete_game_image(ir_program)?;

        Ok(story_data)
    }
}
