use infocom::interpreter::{Interpreter, ExecutionResult};
use infocom::instruction::Instruction;
use infocom::vm::{Game, VM};
use std::fs::File;
use std::io::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .init();

    println!("=== Quit Opcode Tracer ===");
    println!("This tool monitors for quit opcode (0x0A) execution\n");

    // Load Zork 1
    let mut f = File::open("resources/test/zork1/DATA/ZORK1.DAT")?;
    let mut game_data = Vec::new();
    f.read_to_end(&mut game_data)?;

    let game = Game::from_memory(game_data)?;
    let vm = VM::new(game);
    let mut interpreter = Interpreter::new(vm);
    
    let mut instruction_count = 0;
    let mut quit_opcode_seen = false;
    
    println!("Running game and monitoring for quit opcode...");
    println!("(This will run for a limited number of instructions)\n");
    
    // Run with instruction limit
    for _ in 0..500000 {
        instruction_count += 1;
        
        let pc = interpreter.vm.pc;
        
        // Decode current instruction
        match Instruction::decode(&interpreter.vm.game.memory, pc as usize, interpreter.vm.game.header.version) {
            Ok(inst) => {
                // Check if this is a quit opcode (0x0A in 0OP format)
                if inst.opcode == 0x0A && inst.operand_count == 0 {
                    quit_opcode_seen = true;
                    println!("\n*** QUIT OPCODE FOUND ***");
                    println!("PC: {:05x}", pc);
                    println!("Instruction: {}", inst.format_with_version(interpreter.vm.game.header.version));
                    println!("Call stack depth: {}", interpreter.vm.call_stack.len());
                    
                    // Show call stack
                    if !interpreter.vm.call_stack.is_empty() {
                        println!("\nCall stack:");
                        for (i, frame) in interpreter.vm.call_stack.iter().enumerate() {
                            println!("  [{}] Return to {:05x}", i, frame.return_pc);
                        }
                    }
                    
                    println!("\nExecuting quit opcode...");
                }
                
                // Execute the instruction
                interpreter.vm.pc += inst.size as u32;
                match interpreter.execute_instruction(&inst) {
                    Ok(ExecutionResult::Quit) => {
                        println!("\nQuit opcode executed successfully!");
                        println!("Total instructions executed: {}", instruction_count);
                        return Ok(());
                    }
                    Ok(_) => {
                        // Continue
                    }
                    Err(e) => {
                        eprintln!("Execution error at PC {:05x}: {}", pc, e);
                        break;
                    }
                }
            }
            Err(e) => {
                eprintln!("Failed to decode instruction at PC {:05x}: {}", pc, e);
                break;
            }
        }
    }
    
    if !quit_opcode_seen {
        println!("\nNo quit opcode was executed in {} instructions.", instruction_count);
        println!("This suggests that the V-QUIT routine returns normally");
        println!("instead of executing the quit opcode.");
    }
    
    Ok(())
}