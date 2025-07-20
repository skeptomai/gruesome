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

    println!("=== Analyzing QUEUE Implementation ===\n");

    // Based on our findings, let's look at the routine at 0x21fe (packed: 0x10ff)
    // This was the first call in the GO routine

    println!("1. Examining routine at 0x21fe (likely QUEUE):");
    if let Ok(output) = disasm.disassemble_range(0x21fe, 0x2250) {
        println!("{output}");
    }

    // Also look at the timer interrupt routine we found
    println!("\n\n2. Timer interrupt routine that decrements G88:");

    // The routine containing the dec_chk at 0x50dc
    let timer_routine_start = 0x50d0;
    if let Ok(output) = disasm.disassemble_range(timer_routine_start, timer_routine_start + 0x50) {
        println!("{output}");
    }

    // Look for where timers are checked against specific values
    println!("\n\n3. Looking for lantern warning logic:");

    // Search for specific timer values that trigger warnings
    for addr in 0x5000..0x6000 {
        if let Ok((inst, _)) = disasm.disassemble_instruction(addr as u32) {
            // Look for comparisons with specific values
            if matches!(inst.opcode, 0x01 | 0x02 | 0x03 | 0x41 | 0x42 | 0x43) {
                for op in &inst.operands {
                    // Common lantern warning thresholds
                    if *op == 30 || *op == 20 || *op == 10 || *op == 5 {
                        println!("\nFound comparison with {op} at 0x{addr:04x}:");
                        if let Ok(output) =
                            disasm.disassemble_range((addr - 10) as u32, (addr + 20) as u32)
                        {
                            for line in output.lines() {
                                println!("  {line}");
                            }
                        }
                        break;
                    }
                }
            }
        }
    }

    // Summary of timer system
    println!("\n\n=== Timer System Summary ===");
    println!("\n1. QUEUE routine: Likely at 0x21fe");
    println!("   - Called multiple times during GO routine initialization");
    println!("   - Sets up timed events for the game");

    println!("\n2. Timer globals:");
    println!("   - G88 (0x58): Lantern timer");
    println!("   - G89 (0x59): Match timer");
    println!("   - G90 (0x5a): Candle timer");

    println!("\n3. Timer mechanism:");
    println!("   - SREAD can take time and routine parameters");
    println!("   - The routine is called periodically during input");
    println!("   - Timer routines decrement counters and check thresholds");
    println!("   - When a timer reaches certain values, actions occur");

    println!("\n4. Lantern behavior:");
    println!("   - Starts at ~300-400 turns");
    println!("   - Warnings at specific thresholds (30, 20, 10, 5)");
    println!("   - Goes out at 0, plunging player into darkness");

    Ok(())
}
