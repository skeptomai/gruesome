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

    println!("=== TESTING TIMER BEHAVIOR HYPOTHESIS ===\n");

    // First, let's examine the timer interrupt routine more closely
    println!("1. Timer Interrupt Routine at 0x5258:");
    if let Ok(output) = disasm.disassemble_range(0x5258, 0x5280) {
        println!("{output}");
    }

    // Look for what happens when G88 reaches critical values
    println!("\n2. What happens at lantern thresholds:");

    // Search for checks of G88 against specific values (30, 20, 10, 5)
    for threshold in &[30, 20, 10, 5, 0] {
        println!("\nSearching for G88 == {threshold} checks:");

        for addr in 0x5000..0x6000 {
            if let Ok((inst, _)) = disasm.disassemble_instruction(addr as u32) {
                // Look for JE (jump if equal) instructions
                if inst.opcode == 0x01
                    && inst.operands.len() >= 2
                    && ((inst.operands[0] == 0x58 && inst.operands[1] == *threshold)
                        || (inst.operands.len() > 2
                            && inst.operands[0] == 0x58
                            && inst.operands.iter().skip(1).any(|&op| op == *threshold)))
                {
                    println!("  Found at 0x{addr:04x}:");
                    if let Ok(output) = disasm.disassemble_range(addr as u32, (addr + 10) as u32) {
                        for line in output.lines() {
                            println!("    {line}");
                        }
                    }
                    break;
                }
            }
        }
    }

    // Let's also check if there's a difference between timer interrupts and turn-based updates
    println!("\n3. Looking for turn-based updates vs timer interrupts:");

    // The main loop should call something after each command
    // Let's look for calls after SREAD returns
    println!("\nMain loop pattern (looking for post-command updates):");

    // Search for the main game loop pattern
    for addr in 0x5000..0x6000 {
        if let Ok((inst, _text)) = disasm.disassemble_instruction(addr as u32) {
            if inst.opcode == 0x04 {
                // sread
                println!("\nSREAD at 0x{addr:04x}, checking what follows:");
                if let Ok(output) = disasm.disassemble_range(addr as u32, (addr + 20) as u32) {
                    for line in output.lines() {
                        println!("  {line}");
                    }
                }
                // Only show first few examples
                break;
            }
        }
    }

    Ok(())
}
