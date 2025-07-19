use gruesome::disassembler::Disassembler;
use gruesome::vm::Game;
use std::io::Read;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let game_path = "resources/test/zork1/DATA/ZORK1.DAT";

    let mut f = std::fs::File::open(game_path)?;
    let mut memory = Vec::new();
    f.read_to_end(&mut memory)?;

    let game = Game::from_memory(memory)?;
    let disasm = Disassembler::new(&game);

    println!("=== Disassembling around 0x630c (space print) ===\n");

    // Show the area around where the space is printed
    if let Ok(output) = disasm.disassemble_range(0x6300, 0x6320) {
        println!("{}", output);
    }

    println!("\n=== Checking what instruction is at 0x630c ===");
    if let Ok((inst, text)) = disasm.disassemble_instruction(0x630c) {
        println!("Instruction: {}", text);
        println!("Opcode: 0x{:02x}", inst.opcode);
        println!("Operands: {:?}", inst.operands);
    }

    // Also check the area that should print the object name
    println!("\n=== Looking for where 'leaves' should be printed ===");

    // Search for print_obj instructions in the vicinity
    for addr in 0x6300..0x6400 {
        if let Ok((inst, text)) = disasm.disassemble_instruction(addr as u32) {
            if inst.opcode == 0x0A {
                // print_obj
                println!("\nFound print_obj at 0x{:04x}: {}", addr, text);
                // Show context
                if let Ok(output) = disasm.disassemble_range((addr - 5) as u32, (addr + 10) as u32)
                {
                    println!("Context:");
                    for line in output.lines() {
                        println!("  {}", line);
                    }
                }
            }
        }
    }

    Ok(())
}
