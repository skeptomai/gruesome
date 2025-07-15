use infocom::vm::{Game, VM};
use infocom::interpreter::Interpreter;
use infocom::disassembler::Disassembler;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let game_path = "resources/test/zork1/DATA/ZORK1.DAT";
    
    // Load game data
    let mut f = std::fs::File::open(game_path)?;
    let mut memory = Vec::new();
    std::io::Read::read_to_end(&mut f, &mut memory)?;
    
    let game = Game::from_memory(memory)?;
    let vm = VM::new(game);
    let mut interpreter = Interpreter::new(vm);
    let disasm = Disassembler::new(&interpreter.vm.game);
    
    println!("=== Tracing execution flow after PERFORM's NULL call ===\n");
    
    // From our debugging, we know:
    // 1. PERFORM at 0x577c gets property 17 from object 4, which is 0
    // 2. It calls address 0, which returns 0 
    // 3. At 0x57ff: jz V04 [TRUE +5] branches to 0x5806
    
    println!("After NULL call and branch, execution continues at 0x5806...\n");
    
    // Disassemble a larger chunk to see the flow
    let mut addr = 0x5806;
    let mut found_print = false;
    
    for i in 0..50 {
        match disasm.disassemble_instruction(addr) {
            Ok((inst, text)) => {
                println!("{:05x}: {}", addr, text);
                
                // Check for print-related instructions
                if text.contains("print") {
                    println!("       ^^^ PRINT INSTRUCTION FOUND!");
                    found_print = true;
                    
                    // If it's a print_addr, show what address it's printing from
                    if inst.opcode == 0x07 && inst.operands.len() > 0 {
                        let print_addr = inst.operands[0];
                        println!("       Printing from address: 0x{:04x}", print_addr);
                        
                        // Check if this matches our garbage text location
                        if print_addr == 0x9c38 || print_addr == 0x4e1c {
                            println!("       *** THIS IS THE GARBAGE TEXT ADDRESS! ***");
                        }
                    }
                }
                
                // Check for branches/jumps that might skip over code
                if text.contains("jump") || text.contains("ret") {
                    println!("       ^^^ FLOW CONTROL: {}", 
                        if text.contains("ret") { "RETURN" } else { "JUMP" });
                }
                
                addr += inst.size as u32;
            }
            Err(e) => {
                println!("{:05x}: Error: {}", addr, e);
                break;
            }
        }
        
        // Stop if we've gone too far
        if addr > 0x5900 {
            break;
        }
    }
    
    if !found_print {
        println!("\nNo print instructions found in immediate vicinity.");
        println!("The garbage text might be printed from a subroutine call.");
    }
    
    // Let's also check what MAIN-LOOP does after PERFORM returns
    println!("\n=== Checking MAIN-LOOP after PERFORM ===");
    println!("MAIN-LOOP calls PERFORM at some point, then continues...");
    
    // We know MAIN-LOOP is at 0x552a
    // Let's see what it does after calling PERFORM
    addr = 0x552a;
    println!("\nDisassembling MAIN-LOOP to find where it calls PERFORM:");
    
    let mut found_perform_call = false;
    let mut after_call_count = 0;
    for _ in 0..100 {
        match disasm.disassemble_instruction(addr) {
            Ok((inst, text)) => {
                if text.contains("call") && text.contains("577c") {
                    found_perform_call = true;
                    println!("\n{:05x}: {} <-- CALLS PERFORM", addr, text);
                    println!("\nContinuing after this call:");
                } else if found_perform_call && after_call_count < 10 {
                    println!("{:05x}: {}", addr, text);
                    after_call_count += 1;
                }
                
                addr += inst.size as u32;
                
                if addr > 0x5600 {
                    break;
                }
            }
            Err(_) => break,
        }
    }
    
    Ok(())
}