use gruesome::vm::Game;
use std::io::Read;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let game_path = "resources/test/zork1/DATA/ZORK1.DAT";

    let mut f = std::fs::File::open(game_path)?;
    let mut memory = Vec::new();
    f.read_to_end(&mut memory)?;

    let game = Game::from_memory(memory)?;

    println!("=== Bytes at 0x5fdf (WORD-PRINT dec_chk) ===");
    println!();

    // Show bytes around 0x5fdf
    for addr in 0x5fda..=0x5ff0 {
        if addr % 8 == 0 {
            print!("\n{addr:04x}: ");
        }
        print!("{:02x} ", game.memory[addr]);
    }
    println!("\n");

    // Decode the dec_chk instruction at 0x5fdf
    println!("Instruction at 0x5fdf:");
    let byte1 = game.memory[0x5fdf];
    let byte2 = game.memory[0x5fe0];
    let byte3 = game.memory[0x5fe1];
    let byte4 = game.memory[0x5fe2];

    println!("Bytes: {byte1:02x} {byte2:02x} {byte3:02x} {byte4:02x}");

    // In Z-Machine, 0x04 is dec_chk
    println!("Opcode: 0x{byte1:02x} (dec_chk)");

    // For Long form 2OP, bits 6-5 of first byte determine form
    let form_bits = (byte1 >> 5) & 0x03;
    println!(
        "Form bits: {:02b} ({})",
        form_bits,
        if form_bits == 0 { "Long" } else { "Variable" }
    );

    // For Long form, bits 6 and 5 indicate operand types
    let op1_type = if (byte1 & 0x40) != 0 {
        "Variable"
    } else {
        "Small constant"
    };
    let op2_type = if (byte1 & 0x20) != 0 {
        "Variable"
    } else {
        "Small constant"
    };

    println!("Op1 type: {op1_type}");
    println!("Op2 type: {op2_type}");
    println!("Op1 value: 0x{byte2:02x}");
    println!("Op2 value: 0x{byte3:02x}");

    // Branch info
    let branch_byte = byte4;
    let branch_on = (branch_byte & 0x80) != 0;
    let branch_offset = if (branch_byte & 0x40) != 0 {
        // Single byte offset
        (branch_byte & 0x3f) as i8 as i32
    } else {
        // Two byte offset
        let byte5 = game.memory[0x5fe3];
        let offset = (((branch_byte & 0x3f) as u16) << 8) | (byte5 as u16);
        if offset & 0x2000 != 0 {
            // Sign extend
            (offset | 0xc000) as i16 as i32
        } else {
            offset as i32
        }
    };

    let branch_target = if branch_offset == 0 {
        "RFALSE".to_string()
    } else if branch_offset == 1 {
        "RTRUE".to_string()
    } else {
        format!("offset {branch_offset}")
    };

    println!("\nBranch: {branch_target} when condition is {branch_on}");

    Ok(())
}
