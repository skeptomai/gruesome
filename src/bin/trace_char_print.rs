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
    
    println!("=== Looking for character printing loops ===\n");
    
    // The game needs to print "leaves" from the text buffer
    // Text buffer at 0x2641 contains "move leaves"
    // Parse buffer has info about where "leaves" starts (position 5) and length (6)
    
    // Look for loops that might be printing characters
    // This would involve:
    // 1. loadb to get a character
    // 2. print_char to print it
    // 3. inc/dec counter
    // 4. jump back
    
    println!("Searching for print_char loops in error handling area...");
    
    for addr in 0x6000..0x6400 {
        if let Ok((inst, text)) = disasm.disassemble_instruction(addr as u32) {
            if inst.opcode == 0x05 { // print_char
                println!("\nFound print_char at 0x{:04x}: {}", addr, text);
                
                // Show surrounding code
                if let Ok(output) = disasm.disassemble_range((addr - 20) as u32, (addr + 20) as u32) {
                    println!("Context:");
                    for line in output.lines() {
                        if line.contains(&format!("{:04x}:", addr)) {
                            println!(">>> {}", line);
                        } else {
                            println!("    {}", line);
                        }
                    }
                }
                
                // Look for loop structure
                println!("\nChecking for loop pattern...");
                
                // Look backwards for loadb
                for back_addr in (addr - 20)..addr {
                    if let Ok((back_inst, _)) = disasm.disassemble_instruction(back_addr as u32) {
                        if back_inst.opcode == 0x10 { // loadb
                            println!("  Found loadb at 0x{:04x}", back_addr);
                        }
                    }
                }
                
                // Look forward for jump
                for fwd_addr in addr..(addr + 20) {
                    if let Ok((fwd_inst, _)) = disasm.disassemble_instruction(fwd_addr as u32) {
                        if fwd_inst.opcode == 0x0C { // jump
                            println!("  Found jump at 0x{:04x}", fwd_addr);
                        }
                    }
                }
            }
        }
    }
    
    // Also check the area around where we know the issue happens
    println!("\n\nDetailed analysis of execution after space (0x630c):");
    if let Ok(output) = disasm.disassemble_range(0x630c, 0x6360) {
        println!("{}", output);
    }
    
    Ok(())
}