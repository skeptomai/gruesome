// Grue Compiler Module
// Main compiler infrastructure for the Grue language

pub mod ast;
pub mod codegen;
pub mod codegen_builtins;
pub mod codegen_headers;
pub mod codegen_instructions;
pub mod codegen_objects;
pub mod codegen_strings;
pub mod codegen_utils;
pub mod error;
pub mod ir;
pub mod lexer;
pub mod object_system;
#[cfg(test)]
mod opcode_form_unit_tests;
pub mod opcodes;
#[cfg(test)]
mod opcodes_tests;
pub mod parser;
#[cfg(test)]
mod push_pull_branch_integration_tests;
#[cfg(test)]
mod push_pull_unit_tests;
pub mod semantic;
#[cfg(test)]
mod unresolved_reference_tests;

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

    /// Compile Grue source code to IR only (for debugging)
    pub fn compile_to_ir(&self, source: &str) -> Result<ir::IrProgram, CompilerError> {
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

        Ok(ir_program)
    }

    /// Compile Grue source code to Z-Machine bytecode
    pub fn compile(
        &self,
        source: &str,
        version: ZMachineVersion,
    ) -> Result<(Vec<u8>, codegen::ZMachineCodeGen), CompilerError> {
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
        log::debug!(
            "🔄 Transferring {} builtin functions from IR to codegen",
            ir_generator.get_builtin_functions().len()
        );
        for (function_id, function_name) in ir_generator.get_builtin_functions() {
            log::debug!(
                "🔄 Registering builtin: ID {} -> '{}'",
                function_id,
                function_name
            );
            code_generator.register_builtin_function(*function_id, function_name.clone());
        }

        // Transfer object numbers from IR generator to code generator
        code_generator.set_object_numbers(ir_generator.get_object_numbers().clone());

        let story_data = code_generator.generate_complete_game_image(ir_program)?;

        Ok((story_data, code_generator))
    }
}

/// Print IR program in a human-readable format
pub fn print_ir(ir: &ir::IrProgram) {
    println!("Program Mode: {:?}", ir.program_mode);
    println!();

    // Print globals
    if !ir.globals.is_empty() {
        println!("=== GLOBALS ({}) ===", ir.globals.len());
        for global in &ir.globals {
            print!("  global {} (id={})", global.name, global.id);
            if let Some(init) = &global.initializer {
                print!(": {:?}", init);
            }
            println!();
        }
        println!();
    }

    // Print string table
    if !ir.string_table.is_empty() {
        println!("=== STRING TABLE ({} entries) ===", ir.string_table.len());
        for (string, id) in &ir.string_table {
            let display_str = if string.len() > 60 {
                format!("{}...", &string[..57])
            } else {
                string.clone()
            };
            println!("  [{}] = \"{}\"", id, display_str);
        }
        println!();
    }

    // Print rooms
    if !ir.rooms.is_empty() {
        println!("=== ROOMS ({}) ===", ir.rooms.len());
        for room in &ir.rooms {
            println!(
                "  room {} (id={}, obj#={})",
                room.name,
                room.id,
                ir.object_numbers.get(&room.name).unwrap_or(&0)
            );
            println!("    display_name: \"{}\"", room.display_name);
            println!(
                "    description: \"{}...\"",
                if room.description.len() > 40 {
                    &room.description[..37]
                } else {
                    &room.description
                }
            );
            if !room.exits.is_empty() {
                println!("    exits: {} directions", room.exits.len());
            }
        }
        println!();
    }

    // Print objects
    if !ir.objects.is_empty() {
        println!("=== OBJECTS ({}) ===", ir.objects.len());
        for obj in &ir.objects {
            println!(
                "  object {} (id={}, obj#={})",
                obj.name,
                obj.id,
                ir.object_numbers.get(&obj.name).unwrap_or(&0)
            );
            println!("    (see object details in Debug output)");
        }
        println!();
    }

    // Print functions
    println!("=== FUNCTIONS ({}) ===", ir.functions.len());
    for func in &ir.functions {
        println!("\nfunction {} (id={}):", func.name, func.id);

        if !func.parameters.is_empty() {
            print!("  parameters:");
            for param in &func.parameters {
                print!(" {}(slot={}, id={})", param.name, param.slot, param.ir_id);
            }
            println!();
        }

        if !func.local_vars.is_empty() {
            println!("  locals:");
            for local in &func.local_vars {
                println!(
                    "    {} (slot={}, id={})",
                    local.name, local.slot, local.ir_id
                );
            }
        }

        println!("  body:");
        print_block(&func.body, 2);
    }

    // Print grammar
    if !ir.grammar.is_empty() {
        println!("\n=== GRAMMAR ({} patterns) ===", ir.grammar.len());
        for grammar in &ir.grammar {
            println!("  Grammar: {:?}", grammar);
        }
    }

    // Print init block
    if let Some(init) = &ir.init_block {
        println!("\n=== INIT BLOCK ===");
        if !ir.init_block_locals.is_empty() {
            println!("  locals:");
            for local in &ir.init_block_locals {
                println!(
                    "    {} (slot={}, id={})",
                    local.name, local.slot, local.ir_id
                );
            }
        }
        print_block(init, 1);
    }
}

fn print_block(block: &ir::IrBlock, indent: usize) {
    use ir::IrInstruction;

    let prefix = "  ".repeat(indent);
    for inst in &block.instructions {
        print!("{}", prefix);
        match inst {
            IrInstruction::LoadImmediate { target, value } => {
                println!("  t{} = {:?}", target, value);
            }
            IrInstruction::LoadVar { target, var_id } => {
                println!("  t{} = load var t{}", target, var_id);
            }
            IrInstruction::StoreVar { var_id, source } => {
                println!("  store t{} -> var t{}", source, var_id);
            }
            IrInstruction::BinaryOp {
                target,
                op,
                left,
                right,
            } => {
                println!("  t{} = t{} {:?} t{}", target, left, op, right);
            }
            IrInstruction::UnaryOp {
                target,
                op,
                operand,
            } => {
                println!("  t{} = {:?} t{}", target, op, operand);
            }
            IrInstruction::Call {
                target,
                function,
                args,
            } => {
                if let Some(t) = target {
                    print!("  t{} = ", t);
                } else {
                    print!("  ");
                }
                print!("call func#{}", function);
                if !args.is_empty() {
                    print!("(");
                    for (i, arg) in args.iter().enumerate() {
                        if i > 0 {
                            print!(", ");
                        }
                        print!("t{}", arg);
                    }
                    print!(")");
                }
                println!();
            }
            IrInstruction::Return { value } => {
                if let Some(v) = value {
                    println!("  return t{}", v);
                } else {
                    println!("  return");
                }
            }
            IrInstruction::Branch {
                condition,
                true_label,
                false_label,
            } => {
                println!(
                    "  branch t{} ? L{} : L{}",
                    condition, true_label, false_label
                );
            }
            IrInstruction::Jump { label } => {
                println!("  jump L{}", label);
            }
            IrInstruction::Label { id } => {
                println!("L{}:", id);
            }
            IrInstruction::GetProperty {
                target,
                object,
                property,
            } => {
                println!("  t{} = t{}.{}", target, object, property);
            }
            IrInstruction::SetProperty {
                object,
                property,
                value,
            } => {
                println!("  t{}.{} = t{}", object, property, value);
            }
            IrInstruction::GetPropertyByNumber {
                target,
                object,
                property_num,
            } => {
                println!("  t{} = t{}.prop#{}", target, object, property_num);
            }
            IrInstruction::SetPropertyByNumber {
                object,
                property_num,
                value,
            } => {
                println!("  t{}.prop#{} = t{}", object, property_num, value);
            }
            IrInstruction::LogicalComparisonOp { target, op, .. } => {
                println!("  t{} = logical_comparison {:?} (deferred)", target, op);
            }
            _ => {
                println!("  {:?}", inst);
            }
        }
    }
}
