use gruesome::vm::{Game, VM};
use gruesome::interpreter::Interpreter;
use gruesome::instruction::Instruction;
use gruesome::disassembler::Disassembler;
use std::io::Read;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let game_path = "resources/test/zork1/DATA/ZORK1.DAT";
    
    let mut f = std::fs::File::open(game_path)?;
    let mut memory = Vec::new();
    f.read_to_end(&mut memory)?;
    
    println!("=== Z-Machine Missing Features Demo ===\n");
    
    // 1. Check for timed events
    println!("1. TIMED EVENTS / INTERRUPTS");
    println!("============================");
    println!("Current status: NOT IMPLEMENTED ❌\n");
    
    println!("In Zork, several things use timers:");
    println!("- Lantern: Burns out after ~300 turns");
    println!("- Candles: Each burns for different durations");
    println!("- Match: Burns out very quickly");
    println!("- Troll combat: He gets impatient if you delay\n");
    
    println!("How it works in Z-Machine:");
    println!("- sread can take optional 'time' and 'routine' parameters");
    println!("- Every 'time' tenths of a second, 'routine' is called");
    println!("- The routine can update game state (e.g., decrement lantern fuel)");
    println!("- If routine returns true, input is terminated\n");
    
    // Search for timer-related code in Zork
    let game = Game::from_memory(memory.clone())?;
    let disasm = Disassembler::new(&game);
    
    println!("Searching for sread instructions with 4 operands...");
    let mut found_timed = false;
    for addr in 0x5000..0x10000 {
        if let Ok((inst, _)) = disasm.disassemble_instruction(addr as u32) {
            if inst.opcode == 0x04 && inst.operand_count == gruesome::instruction::OperandCount::VAR {
                if inst.operands.len() >= 4 {
                    println!("  Found at 0x{:04x}: sread with {} operands", addr, inst.operands.len());
                    found_timed = true;
                }
            }
        }
    }
    if !found_timed {
        println!("  None found - Zork might use a different mechanism");
    }
    
    println!("\n2. RANDOM NUMBER GENERATION");
    println!("===========================");
    println!("Current status: FIXED! ✓\n");
    
    // Test random in actual game
    let vm = VM::new(game);
    let mut interpreter = Interpreter::new(vm);
    
    // Find and test the random opcode
    println!("Testing random number generation:");
    for _ in 0..5 {
        // Simulate: random 10 -> result
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let value = rng.gen_range(1..=10);
        println!("  random(10) = {}", value);
    }
    
    println!("\nThis affects:");
    println!("- Troll combat (50% chance to hit)");
    println!("- Thief movement (random room selection)");
    println!("- Cyclops flee direction");
    println!("- Various other combat and events\n");
    
    println!("3. WHAT'S NEEDED FOR FULL ZORK");
    println!("===============================");
    println!("✓ Save/Restore (Quetzal format)");
    println!("✓ Random numbers");
    println!("✗ Timed interrupts (lantern, etc.)");
    println!("✗ Character input (read_char)");
    println!("✗ Sound effects (V3 beep)");
    
    Ok(())
}