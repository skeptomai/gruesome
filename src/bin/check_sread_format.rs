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
    
    println!("=== Zork I SREAD Format Check ===");
    println!("Version: {}", game.header.version);
    println!();
    
    // In V3, SREAD is opcode 0x04 and can be:
    // - 2OP format (most common)
    // - VAR format with extended operands
    
    // Let's check the actual format at a known SREAD location
    println!("Checking SREAD at 0x1f58:");
    if let Ok(output) = disasm.disassemble_range(0x1f58, 0x1f60) {
        println!("{}", output);
    }
    
    println!("\nChecking SREAD at 0x5015:");
    if let Ok(output) = disasm.disassemble_range(0x5015, 0x501d) {
        println!("{}", output);
    }
    
    // Let's look for ALL sread instructions
    println!("\n=== All SREAD Instructions in Game ===");
    let mut sread_count = 0;
    let mut timer_sread_count = 0;
    
    for addr in 0x0..game.memory.len() {
        if let Ok((inst, _text)) = disasm.disassemble_instruction(addr as u32) {
            if inst.opcode == 0x04 { // sread
                sread_count += 1;
                print!("0x{:04x}: sread ", addr);
                
                // Print operands
                for (i, op) in inst.operands.iter().enumerate() {
                    if i > 0 { print!(", "); }
                    print!("0x{:04x}", op);
                }
                
                if inst.operands.len() >= 4 && inst.operands[2] > 0 && inst.operands[3] > 0 {
                    timer_sread_count += 1;
                    println!(" <-- HAS TIMER!");
                } else {
                    println!();
                }
                
                // Show first 10 to avoid too much output
                if sread_count >= 10 {
                    println!("... (showing first 10)");
                    break;
                }
            }
        }
    }
    
    println!("\nTotal SREAD instructions found: {}", sread_count);
    println!("SREAD with timers: {}", timer_sread_count);
    
    Ok(())
}