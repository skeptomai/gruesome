use infocom::vm::Game;
use infocom::instruction::Instruction;
use infocom::disassembler::Disassembler;
use std::fs::File;
use std::io::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Analyzing the Quit Bug ===\n");

    // Load Zork 1
    let mut f = File::open("resources/test/zork1/DATA/ZORK1.DAT")?;
    let mut game_data = Vec::new();
    f.read_to_end(&mut game_data)?;

    let game = Game::from_memory(game_data)?;
    
    println!("1. The bad call occurs at PC 0x06dfc:");
    println!("   call #486e -> V00");
    println!();
    
    println!("2. Packed address 0x486e unpacks to 0x090dc");
    println!();
    
    println!("3. Let's see what's at the calling location (0x06dfc):");
    // Disassemble around the bad call
    for addr in 0x06df0..=0x06e10 {
        if let Ok(inst) = Instruction::decode(&game.memory, addr, game.header.version) {
            println!("   {:05x}: {}", addr, inst.format_with_version(game.header.version));
        }
    }
    println!();
    
    println!("4. Now let's see what's at 0x090dc (where the bad call goes):");
    // Show raw memory
    print!("   Raw bytes: ");
    for i in 0..32 {
        print!("{:02x} ", game.memory[0x090dc + i]);
    }
    println!("\n");
    
    // Try to decode as instructions
    println!("   Decoded as instructions:");
    let mut offset = 0;
    while offset < 32 {
        if let Ok(inst) = Instruction::decode(&game.memory, 0x090dc + offset, game.header.version) {
            println!("   {:05x}: {}", 0x090dc + offset, inst.format_with_version(game.header.version));
            offset += inst.size;
        } else {
            break;
        }
    }
    println!();
    
    println!("5. Let's trace back to find the routine that contains the bad call:");
    // Find the routine that contains 0x06dfc
    let mut routine_start = None;
    for addr in (0x06d00..=0x06dfc).rev() {
        let byte = game.memory[addr];
        // Check if this could be a routine header (0-15 locals)
        if byte <= 15 {
            // Verify it's actually a routine by checking if the code after locals makes sense
            let code_start = addr + 1 + (byte as usize * 2);
            if code_start < game.memory.len() {
                if let Ok(_) = Instruction::decode(&game.memory, code_start, game.header.version) {
                    routine_start = Some(addr);
                    break;
                }
            }
        }
    }
    
    if let Some(start) = routine_start {
        println!("   Routine starts at: 0x{:05x}", start);
        println!("   Number of locals: {}", game.memory[start]);
        
        // Disassemble the routine
        println!("\n   Full routine disassembly:");
        let disasm = Disassembler::new(&game);
        match disasm.disassemble_range(start as u32, 0x06e10) {
            Ok(output) => println!("{}", output),
            Err(e) => println!("   Error disassembling: {}", e),
        }
    }
    
    println!("\n6. Analysis:");
    println!("   The code at 0x06dfc is trying to call 0x486e as if it were a routine.");
    println!("   However, 0x090dc (unpacked) contains print/sread instructions, not a routine header.");
    println!("   This suggests either:");
    println!("   a) The packed address calculation is wrong");
    println!("   b) The code is corrupted");
    println!("   c) There's a bug in how we're decoding the call instruction");
    
    // Let's check the exact bytes at the call instruction
    println!("\n7. Raw bytes at the call instruction (0x06dfc):");
    for i in 0..8 {
        println!("   {:05x}: {:02x}", 0x06dfc + i, game.memory[0x06dfc + i]);
    }
    
    Ok(())
}