use gruesome::vm::Game;
use gruesome::disassembler::Disassembler;
use std::io::Read;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let game_path = "resources/test/zork1/DATA/ZORK1.DAT";
    
    let mut f = std::fs::File::open(game_path)?;
    let mut memory = Vec::new();
    f.read_to_end(&mut memory)?;
    
    let game = Game::from_memory(memory)?;
    let disasm = Disassembler::new(&game);
    
    println!("=== Finding QUEUE Calls in GO Routine ===\n");
    
    // The GO routine is called early - let's trace from 0x4f05
    // Look for the 5 QUEUE calls mentioned in the ZIL startup sequence
    
    println!("Disassembling GO routine and initialization:");
    
    // First, find where the 5 QUEUE calls happen
    let mut queue_count = 0;
    let mut call_addresses = Vec::new();
    
    // Search in the early initialization area
    for addr in 0x4f00..0x5200 {
        if let Ok((inst, _)) = disasm.disassemble_instruction(addr as u32) {
            // Look for CALL instructions
            if matches!(inst.opcode, 0x19 | 0x00 | 0x0c | 0xe0) {
                if inst.operands.len() >= 1 {
                    // Check if this might be calling QUEUE
                    // QUEUE routines are typically in the 0x2xxx-0x3xxx range
                    let routine = inst.operands[0];
                    if routine >= 0x1000 && routine <= 0x2000 {
                        call_addresses.push((addr, routine, inst.operands.clone()));
                        queue_count += 1;
                        
                        if queue_count <= 10 {
                            println!("\nCall #{} at 0x{:04x}:", queue_count, addr);
                            println!("  Routine: 0x{:04x} (unpacked: 0x{:04x})", 
                                     routine, routine * 2);
                            
                            if inst.operands.len() > 1 {
                                println!("  Args: {:?}", &inst.operands[1..]);
                            }
                            
                            // Show context
                            if let Ok(output) = disasm.disassemble_range((addr - 5) as u32, (addr + 10) as u32) {
                                println!("  Context:");
                                for line in output.lines() {
                                    println!("    {}", line);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    
    // Now look at what the timer-related globals are set to
    println!("\n\n=== Timer-Related Code Patterns ===");
    
    // Look for DEC_CHK on timer globals in routines
    println!("\nSearching for timer decrement patterns:");
    
    for addr in 0x2e53..0x8000 {
        if let Ok((inst, text)) = disasm.disassemble_instruction(addr as u32) {
            // DEC_CHK on globals 88-90
            if inst.opcode == 0x04 && inst.operands.len() >= 1 {
                let var = inst.operands[0];
                if var >= 0x58 && var <= 0x5a {
                    println!("\nTimer operation at 0x{:04x}: {}", addr, text);
                    
                    // This is likely in a timer interrupt routine
                    // Show the whole routine
                    let routine_start = (addr / 8) * 8;
                    if let Ok(output) = disasm.disassemble_range(routine_start as u32, (addr + 30) as u32) {
                        println!("Routine context:");
                        for line in output.lines().take(15) {
                            println!("  {}", line);
                        }
                    }
                    
                    break; // Just show the first one for now
                }
            }
        }
    }
    
    // Look for the actual lantern burning logic
    println!("\n\n=== Looking for Lantern Logic ===");
    
    // The lantern timer (G88) is decremented and when it reaches certain values:
    // - Warning messages are printed
    // - Eventually the lantern goes out
    
    // Search for comparisons of G88 with specific values
    for addr in 0x2e53..0x8000 {
        if let Ok((inst, text)) = disasm.disassemble_instruction(addr as u32) {
            // Look for JE, JL, JG with G88 (0x58)
            if matches!(inst.opcode, 0x01 | 0x02 | 0x03) && inst.operands.len() >= 2 {
                if inst.operands[0] == 0x58 || 
                   (inst.operands.len() > 1 && inst.operands[1] == 0x58) {
                    println!("\nG88 comparison at 0x{:04x}: {}", addr, text);
                    
                    // Show context
                    if let Ok(output) = disasm.disassemble_range((addr - 10) as u32, (addr + 20) as u32) {
                        for line in output.lines() {
                            if line.contains(&format!("{:04x}:", addr)) {
                                println!(">>> {}", line);
                            } else {
                                println!("    {}", line);
                            }
                        }
                    }
                    break;
                }
            }
        }
    }
    
    Ok(())
}