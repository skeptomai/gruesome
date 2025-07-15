use infocom::vm::Game;
use infocom::instruction::Instruction;
use infocom::disassembler::Disassembler;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let game_path = "resources/test/zork1/DATA/ZORK1.DAT";
    
    // Load game data
    let mut f = std::fs::File::open(game_path)?;
    let mut memory = Vec::new();
    std::io::Read::read_to_end(&mut f, &mut memory)?;
    
    let game = Game::from_memory(memory)?;
    
    println!("=== Analyzing Type 0x32 Dictionary Entry Handling ===\n");
    
    // First, let's look at what the PERFORM and V-WALK routines do
    println!("PERFORM routine at 0x50a8:");
    let mut disasm = Disassembler::new(&game);
    for addr in 0x50a8..0x50c0 {
        if let Ok((inst, text)) = disasm.disassemble_instruction(addr) {
            println!("  {:05x}: {}", addr, text);
        }
    }
    
    println!("\nV-WALK routine at 0x6f76:");
    for addr in 0x6f76..0x6f90 {
        if let Ok((inst, text)) = disasm.disassemble_instruction(addr) {
            println!("  {:05x}: {}", addr, text);
        }
    }
    
    // Now let's trace what happens with byte 4 = 0x32
    println!("\n=== Analyzing dictionary data handling ===");
    
    // The key insight: byte 4 might be checked to determine how to process the entry
    // Let's search for code that loads byte 4 from a dictionary entry
    
    println!("\nSearching for code that might check byte 4 of dictionary entries...");
    
    // Look for patterns like:
    // - loadb <dict_addr>, #04
    // - je <result>, #32
    
    for addr in 0x5000..0x8000 {
        if game.memory[addr] == 0x10 { // loadb
            let inst_result = Instruction::decode(&game.memory, addr, game.header.version);
            if let Ok(inst) = inst_result {
                if inst.operands.len() >= 2 && inst.operands[1] == 4 {
                    // This loads byte 4!
                    println!("\nFound LOADB of byte 4 at {:05x}:", addr);
                    
                    // Show context
                    for i in 0..5 {
                        let context_addr = addr + (i * 3);
                        if context_addr < game.memory.len() {
                            if let Ok((_, text)) = disasm.disassemble_instruction(context_addr as u32) {
                                println!("  {:05x}: {}", context_addr, text);
                            }
                        }
                    }
                    
                    // Check next instruction for comparison with 0x32
                    let next_addr = addr + inst.size as usize;
                    if next_addr < game.memory.len() {
                        if let Ok(next_inst) = Instruction::decode(&game.memory, next_addr, game.header.version) {
                            for op in &next_inst.operands {
                                if *op == 0x32 {
                                    println!("  *** Comparison with 0x32 found! ***");
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    
    // Let's also check what happens at address 0xa11d (the value in bytes 5-6 for 'w')
    println!("\n=== Checking address 0xa11d (from 'w' data bytes) ===");
    
    let addr = 0xa11d;
    println!("Memory at {:04x}:", addr);
    for i in 0..32 {
        if addr + i < game.memory.len() {
            let byte = game.memory[addr + i];
            if byte >= 32 && byte <= 126 {
                print!("{}", byte as char);
            } else {
                print!(".");
            }
        }
    }
    println!();
    
    // Disassemble to see if it's code
    println!("\nDisassembly at {:04x}:", addr);
    for i in 0..5 {
        let da = addr + (i * 3);
        if let Ok((_, text)) = disasm.disassemble_instruction(da as u32) {
            println!("  {:05x}: {}", da, text);
        }
    }
    
    // Try to decode as Z-string
    println!("\nAs Z-string:");
    if let Ok((text, _)) = infocom::text::decode_string(&game.memory[addr..], game.header.abbrev_table, game.header.version as usize) {
        println!("  Decoded: '{}'", text);
    }
    
    // Check if 0xa1 and 0x1d might be separate values
    println!("\n=== Checking if bytes 5-6 are separate values ===");
    println!("Byte 5 (0xa1 = {}): Might be an object number?", 0xa1);
    println!("Byte 6 (0x1d = {}): Might be a property or action number?", 0x1d);
    
    // In many games, directions are implemented as properties on room objects
    // 0x1d = 29 which could be a property number
    
    Ok(())
}