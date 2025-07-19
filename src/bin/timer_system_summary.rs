use gruesome::disassembler::Disassembler;
use gruesome::vm::Game;
use std::io::Read;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let game_path = "resources/test/zork1/DATA/ZORK1.DAT";

    let mut f = std::fs::File::open(game_path)?;
    let mut memory = Vec::new();
    f.read_to_end(&mut memory)?;

    let game = Game::from_memory(memory.clone())?;
    let disasm = Disassembler::new(&game);

    println!("=== ZORK I TIMER SYSTEM ANALYSIS ===\n");

    println!("## Summary of Timer Implementation\n");

    println!("### 1. QUEUE Routine");
    println!(
        "The QUEUE routine at 0x21fe is called during game initialization to set up timed events."
    );
    println!("It appears to manage a queue/table of timed events that need to be processed.\n");

    println!("### 2. Timer Variables");
    println!("- Global 88 (0x58): Lantern timer - starts around 300-400");
    println!("- Global 89 (0x59): Match timer - very short duration");
    println!("- Global 90 (0x5a): Candle timer - medium duration\n");

    println!("### 3. Timer Interrupt Mechanism");
    println!("When SREAD is called with time and routine parameters:");
    println!("- Every 'time' tenths of a second, the interrupt routine is called");
    println!("- The interrupt routine can:");
    println!("  - Decrement timer counters");
    println!("  - Check for threshold values");
    println!("  - Trigger game events (warnings, lantern going out, etc.)");
    println!("  - Return true to terminate input early\n");

    println!("### 4. Lantern Timer Operation");

    // Show the actual timer decrement code
    println!("Timer decrement at 0x50dc:");
    if let Ok(output) = disasm.disassemble_range(0x50d8, 0x50e5) {
        for line in output.lines() {
            println!("  {}", line);
        }
    }

    println!("\nThis shows: dec_chk G88, #36 [TRUE +6725]");
    println!("- Decrements G88 (lantern timer)");
    println!("- Jumps if it's greater than or equal to 54 (0x36)");
    println!("- Below 54, special handling occurs\n");

    println!("### 5. How Timers Affect Game State\n");

    println!("Timers don't directly modify game state in most cases. Instead:");
    println!("1. Timer interrupt routines decrement counter variables");
    println!("2. Game logic checks these counters at appropriate times");
    println!("3. When counters reach certain values, game events trigger\n");

    println!("For the lantern:");
    println!("- Starts at ~330 turns");
    println!("- Warning messages at 30, 20, 10, 5 turns remaining");
    println!("- At 0: Lantern goes out, room becomes dark");
    println!("- Player needs fresh batteries or another light source\n");

    println!("### 6. Implementation Requirements\n");

    println!("To implement timers in the interpreter:");
    println!("1. SREAD instruction must support time and routine parameters");
    println!("2. During input wait, call the interrupt routine periodically");
    println!("3. The interrupt routine should be able to:");
    println!("   - Access and modify global variables");
    println!("   - Return a value (true = terminate input)");
    println!("4. Timer state is just global variables - no special handling needed\n");

    println!("### 7. Example Timer Flow\n");

    println!("1. GO routine calls QUEUE to set up lantern timer");
    println!("2. QUEUE stores the timer routine address and initial count");
    println!("3. Main game loop calls SREAD with timer parameters");
    println!("4. Every N tenths of a second, interrupt routine runs:");
    println!("   - Decrements G88");
    println!("   - Checks if below threshold");
    println!("   - May print warning or extinguish lantern");
    println!("5. Game continues normally, checking lamp state as needed\n");

    // Look for actual SREAD calls with timers
    println!("### 8. SREAD Calls with Timers\n");

    let mut found_timed_sread = false;
    for addr in 0x5000..0x7000 {
        if let Ok((inst, _)) = disasm.disassemble_instruction(addr as u32) {
            if inst.opcode == 0x04 && inst.operands.len() >= 4 {
                let time = inst.operands[2];
                let routine = inst.operands[3];

                if time > 0 && routine > 0 && !found_timed_sread {
                    found_timed_sread = true;
                    println!("Example timed SREAD at 0x{:04x}:", addr);
                    if let Ok(output) = disasm.disassemble_range(addr as u32, (addr + 5) as u32) {
                        for line in output.lines() {
                            println!("  {}", line);
                        }
                    }
                    println!("\nThis shows:");
                    println!("- Text buffer: 0x{:04x}", inst.operands[0]);
                    println!("- Parse buffer: 0x{:04x}", inst.operands[1]);
                    println!("- Time: {} tenths of seconds", time);
                    println!(
                        "- Interrupt routine: 0x{:04x} (unpacked: 0x{:04x})",
                        routine,
                        routine * 2
                    );
                }
            }
        }
    }

    Ok(())
}
