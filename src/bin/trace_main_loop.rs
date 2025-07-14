use infocom::vm::{Game, VM};
use infocom::interpreter::Interpreter;
use infocom::instruction::Instruction;
use std::fs::File;
use std::io::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::builder()
        .filter_level(log::LevelFilter::Debug)
        .init();

    let mut f = File::open("resources/test/zork1/DATA/ZORK1.DAT")?;
    let mut game_data = Vec::new();
    f.read_to_end(&mut game_data)?;

    let game = Game::from_memory(game_data).map_err(|e| format!("Game load error: {}", e))?;
    let vm = VM::new(game);
    let mut interpreter = Interpreter::new(vm);

    let mut count = 0;
    let mut after_serial = false;
    let mut trace_next = 0;
    
    loop {
        count += 1;
        let pc = interpreter.vm.pc;
        
        // Start detailed tracing after serial number
        if pc == 0x06f8c {
            after_serial = true;
            trace_next = 100; // Trace next 100 instructions in detail
            println!("\n=== SERIAL NUMBER PRINTED, NOW TRACING MAIN LOOP ===");
        }
        
        // Decode instruction
        if let Ok(inst) = Instruction::decode(
            &interpreter.vm.game.memory,
            pc as usize,
            interpreter.vm.game.header.version,
        ) {
            // Detailed trace if requested
            if trace_next > 0 {
                trace_next -= 1;
                println!("[{:04}] {:05x}: {}", count, pc, 
                    inst.format_with_version(interpreter.vm.game.header.version));
                
                // Show branches taken
                if let Some(branch) = &inst.branch {
                    println!("      Branch: {} offset {}", 
                        if branch.on_true { "TRUE" } else { "FALSE" },
                        branch.offset);
                }
            }
            
            // Always trace calls to see routine flow
            let name = inst.name(interpreter.vm.game.header.version);
            if name.starts_with("call") && after_serial {
                println!("\n[{:04}] {:05x}: {} to 0x{:04x}", count, pc, name,
                    if inst.operands.is_empty() { 0 } else { inst.operands[0] });
                    
                // Unpack the address to see actual routine
                if !inst.operands.is_empty() {
                    let packed = inst.operands[0];
                    let unpacked = packed * 2; // For V3
                    println!("      Calling routine at 0x{:05x}", unpacked);
                    
                    // Check if this is STAND
                    if unpacked == 0x86ca {
                        println!("      !!! This is the STAND routine !!!");
                        
                        // Dump state
                        println!("\n=== STATE AT STAND CALL ===");
                        println!("Call stack depth: {}", interpreter.vm.call_stack.len());
                        
                        // Check key globals
                        for i in 0..10 {
                            if let Ok(val) = interpreter.vm.read_variable(16 + i) {
                                if val != 0 {
                                    println!("  G{:02x}: {}", i, val);
                                }
                            }
                        }
                        
                        // G6f is checked by STAND
                        if let Ok(val) = interpreter.vm.read_variable(16 + 0x6f) {
                            println!("  G6f: {} (checked by STAND)", val);
                            
                            // Try to identify what object this is
                            if val > 0 && val < 256 {
                                print!("  G6f object name: ");
                                // We need a way to print object name...
                                println!("(object {})", val);
                            }
                        }
                        
                        println!("\n=== RECENT EXECUTION ===");
                        trace_next = 50; // Trace more instructions
                    }
                }
            }
            
            // Watch for STAND routine entry
            if pc == 0x86ca {
                println!("\n!!! ENTERED STAND ROUTINE at instruction {} !!!", count);
                trace_next = 20;
            }
            
            // Execute instruction
            match interpreter.execute_instruction(&inst) {
                Ok(_) => {},
                Err(e) => {
                    eprintln!("Error at PC {:05x}: {}", pc, e);
                    break;
                }
            }
        } else {
            eprintln!("Failed to decode instruction at PC {:05x}", pc);
            break;
        }
        
        if count > 3000 {
            println!("\nStopped after 3000 instructions");
            break;
        }
    }

    Ok(())
}