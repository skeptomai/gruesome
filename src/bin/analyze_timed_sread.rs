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

    println!("=== Analyzing Timed SREAD in Zork ===\n");

    // Look at a different timed sread at 0x6065
    println!("Disassembling around 0x6065:");
    let output = disasm.disassemble_range(0x6060, 0x6070)?;
    println!("{output}");

    // Decode the instruction specifically
    if let Ok((inst, _text)) = disasm.disassemble_instruction(0x6065) {
        println!("\nInstruction details:");
        println!("  Opcode: 0x{:02x} (sread)", inst.opcode);
        println!("  Operands: {} total", inst.operands.len());

        if inst.operands.len() >= 4 {
            println!("  Text buffer: 0x{:04x}", inst.operands[0]);
            println!("  Parse buffer: 0x{:04x}", inst.operands[1]);
            println!("  Time: {} (tenths of seconds)", inst.operands[2]);
            println!("  Routine: 0x{:04x}", inst.operands[3]);

            // Try to find what the interrupt routine does
            if inst.operands[3] != 0 {
                let routine_addr = inst.operands[3] as u32 * 2; // Packed address
                println!("\nInterrupt routine at 0x{routine_addr:04x}:");
                let routine_output = disasm.disassemble_range(routine_addr, routine_addr + 20)?;
                println!("{routine_output}");
            }
        }
    }

    // Check global variables that might be timers
    println!("\nChecking potential timer globals:");
    println!("Global 88 (0x58) - often used for lamp/lantern");
    println!("Global 89 (0x59) - often used for match");
    println!("Global 90 (0x5a) - often used for candles");

    Ok(())
}
