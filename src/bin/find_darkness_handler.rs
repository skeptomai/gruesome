use infocom::vm::{Game, VM};
use infocom::interpreter::Interpreter;
use infocom::disassembler::Disassembler;
use infocom::instruction::Instruction;
use std::fs::File;
use std::io::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load the game file
    let mut file = File::open("resources/test/zork1/DATA/ZORK1.DAT")?;
    let mut game_data = Vec::new();
    file.read_to_end(&mut game_data)?;

    let game = Game::from_memory(game_data)?;
    let mut interpreter = Interpreter::new(VM::new(game));
    
    println!("=== Finding the Darkness Handler ===\n");
    
    // From our analysis, we see that when in darkness, there's a call_2n V40, Ve0
    // This suggests V40 contains the address of a routine that handles darkness
    
    // Let's trace through the execution to understand what V40 might contain
    // We need to look for where V40 gets set
    
    println!("Looking for instructions that set V40...");
    
    // Search for store instructions to V40 (local variable 40)
    for addr in 0x5000..0x6500 {
        match Instruction::decode(&interpreter.vm.game.memory, addr, 3) {
            Ok(instr) => {
                // Check if this instruction stores to V40
                if instr.store_var == Some(0x40) {
                    println!("\nFound store to V40 at 0x{:04x}:", addr);
                    let disasm = Disassembler::new(&interpreter.vm.game);
                    match disasm.disassemble_instruction(addr as u32) {
                        Ok((_, output)) => println!("{}", output),
                        Err(_) => {}
                    }
                    
                    // Show context
                    println!("Context:");
                    let disasm2 = Disassembler::new(&interpreter.vm.game);
                    match disasm2.disassemble_range((addr.saturating_sub(20)) as u32, (addr + 20) as u32) {
                        Ok(output) => println!("{}", output),
                        Err(_) => {}
                    }
                }
            }
            Err(_) => {}
        }
    }
    
    // Let's also look for the pattern where a routine address is loaded into a variable
    // This might be a loadw instruction or similar
    
    println!("\n\nLooking for potential routine addresses being loaded...");
    
    // In Zork, darkness handling routines are often in the 0x3000-0x4000 range
    // Let's look for constants in that range
    
    for addr in 0x5e00..0x6000 {
        let opcode = interpreter.vm.game.memory[addr];
        
        // Look for instructions that might load a routine address
        if opcode == 0x0F || opcode == 0x2F || opcode == 0x4F || opcode == 0x6F {  // loadw variants
            let disasm = Disassembler::new(&interpreter.vm.game);
            match disasm.disassemble_instruction(addr as u32) {
                Ok((instr, output)) => {
                    // Check if the result is stored to V40
                    if instr.store_var == Some(0x40) {
                        println!("\nFound loadw storing to V40 at 0x{:04x}: {}", addr, output);
                        
                        // Show what's being loaded
                        if instr.operands.len() >= 2 {
                            let base = instr.operands[0];
                            let offset = instr.operands[1];
                            println!("  Loading from base 0x{:04x} + offset {} * 2", base, offset);
                        }
                    }
                }
                Err(_) => {}
            }
        }
    }
    
    // Let's run the interpreter for a bit to see what V40 contains
    println!("\n\nRunning interpreter to check V40 value...");
    
    // Run a limited number of instructions
    match interpreter.run_with_limit(Some(1000)) {
        Ok(_) => {},
        Err(e) => println!("Execution stopped: {}", e),
    }
    
    // Check local variables if we're in a routine
    if interpreter.vm.call_stack.len() > 0 {
        println!("\nChecking local variables in current routine:");
        let frame = &interpreter.vm.call_stack[interpreter.vm.call_stack.len() - 1];
        println!("Current routine at: 0x{:04x}", frame.return_pc);
        println!("Number of locals: {}", frame.locals.len());
        
        // V40 would be local variable index 0x40 - 1 = 63 (if it exists)
        // But V40 might also be a global variable
    }
    
    // Let's also just hardcode check some known darkness handling routines
    println!("\n\n=== Checking known darkness handler addresses ===");
    
    let potential_handlers = [0x3770, 0x3b7c, 0x467a, 0x2fd8];
    
    for &handler_addr in &potential_handlers {
        println!("\nChecking routine at 0x{:04x}:", handler_addr);
        
        // Check if this looks like a routine (starts with local count)
        let locals = interpreter.vm.game.memory[handler_addr];
        println!("  Number of locals: {}", locals);
        
        // Disassemble the start of the routine
        let code_start = handler_addr + 1;
        let disasm = Disassembler::new(&interpreter.vm.game);
        match disasm.disassemble_range(code_start as u32, (code_start + 30) as u32) {
            Ok(output) => println!("{}", output),
            Err(e) => println!("  Error: {}", e),
        }
    }

    Ok(())
}