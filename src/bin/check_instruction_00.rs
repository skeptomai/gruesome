use std::fs::File;
use std::io::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut f = File::open("resources/test/zork1/DATA/ZORK1.DAT")?;
    let mut game_data = Vec::new();
    f.read_to_end(&mut game_data)?;

    println!("Analyzing instruction encoding at 0x086d4...\n");
    
    let addr = 0x086d4;
    let byte = game_data[addr];
    
    println!("Byte at 0x{:05x}: 0x{:02x} (binary: {:08b})", addr, byte, byte);
    
    // Check instruction form
    let form = match byte >> 6 {
        0b11 => "Variable",
        0b10 => "Short",
        _ => "Long"
    };
    
    println!("\nInstruction form based on top 2 bits: {}", form);
    
    if form == "Long" {
        println!("Long form 2OP instruction:");
        println!("  Opcode (bottom 5 bits): 0x{:02x}", byte & 0x1F);
        println!("  First operand type (bit 6): {}", if byte & 0x40 != 0 { "variable" } else { "small constant" });
        println!("  Second operand type (bit 5): {}", if byte & 0x20 != 0 { "variable" } else { "small constant" });
        
        println!("\nBut wait - opcode 0x00 is not defined for 2OP!");
    }
    
    // Let's check if this could be a different interpretation
    println!("\n\nAlternative interpretation - what if this is data?");
    println!("Bytes at 0x086d4: {:02x} {:02x} {:02x} {:02x} {:02x}", 
             game_data[addr], game_data[addr+1], game_data[addr+2], 
             game_data[addr+3], game_data[addr+4]);
             
    // Check the actual STAND routine structure
    println!("\n\nLet's trace the STAND routine from its actual start at 0x086ca:");
    
    let mut offset = 0x086ca;
    println!("\nSTAND routine structure:");
    
    // First instruction at 0x086ca
    println!("0x{:05x}: {:02x} - ", offset, game_data[offset]);
    offset += 1;
    
    // The routine seems to have multiple entry points
    // Let's find where the actual code vs data is
    
    println!("\nLooking for the print_ret at 0x086dd:");
    println!("0x086dd: {:02x} - This should be 0xB3 for print_ret", game_data[0x086dd]);
    
    // Check if there's a jump table or something
    println!("\n\nPossibility: The branch target 0x086d4 might be intentionally in the middle of an instruction!");
    println!("This is a technique sometimes used in assembly code.");
    
    // Let's see what happens if we interpret from different offsets
    for start in [0x086ca, 0x086cb, 0x086cc, 0x086cd, 0x086ce].iter() {
        println!("\nIf we start decoding from 0x{:05x}:", start);
        print!("  Bytes: ");
        for i in 0..10 {
            if start + i < game_data.len() {
                print!("{:02x} ", game_data[start + i]);
            }
        }
        println!();
    }

    Ok(())
}