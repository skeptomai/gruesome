use infocom::instruction::Instruction;
use std::fs::File;
use std::io::Read;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let game_path = "resources/test/zork1/DATA/ZORK1.DAT";
    
    // Load game data
    let mut f = File::open(game_path)?;
    let mut memory = Vec::new();
    f.read_to_end(&mut memory)?;
    
    // Check what's at PC 0x5491
    let pc: usize = 0x5491;
    println!("Checking instruction at PC {:05x}", pc);
    
    // Show context
    print!("Context: ");
    for i in pc.saturating_sub(4)..=std::cmp::min(pc + 10, memory.len() - 1) {
        if i == pc {
            print!("[{:02x}] ", memory[i]);
        } else {
            print!("{:02x} ", memory[i]);
        }
    }
    println!();
    
    // Try to decode instruction
    match Instruction::decode(&memory, pc, 3) {
        Ok(inst) => {
            println!("Decoded instruction: {:?}", inst);
        }
        Err(e) => {
            println!("Failed to decode: {}", e);
        }
    }
    
    Ok(())
}