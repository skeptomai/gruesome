// Direct IR test to isolate jump instruction generation
use gruesome::grue_compiler::codegen::ZMachineCodeGen;
use gruesome::grue_compiler::ir::{IrInstruction, IrProgram, IrFunction, IrBlock};

fn main() {
    let mut codegen = ZMachineCodeGen::new().expect("Failed to create codegen");

    // Create minimal IR with just a jump instruction
    let mut program = IrProgram::new();
    let mut function = IrFunction::new("test".to_string());

    // Add a label and a jump to it
    function.body.add_instruction(IrInstruction::Label { id: 1 });
    function.body.add_instruction(IrInstruction::Jump { label: 1 });

    program.functions.push(function);

    match codegen.compile(&program) {
        Ok(bytecode) => {
            println!("Generated {} bytes", bytecode.len());
            // Print hex dump
            for (i, byte) in bytecode.iter().enumerate() {
                if i % 16 == 0 {
                    print!("\n{:04x}: ", i);
                }
                print!("{:02x} ", byte);
            }
            println!();
        }
        Err(e) => {
            eprintln!("Compilation failed: {}", e);
        }
    }
}