use std::fs::File;
use std::io::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut f = File::open("resources/test/zork1/DATA/ZORK1.DAT")?;
    let mut game_data = Vec::new();
    f.read_to_end(&mut game_data)?;

    println!("Hex dump of 0x08ca0 to 0x08ce0:");
    println!("Address  00 01 02 03 04 05 06 07 08 09 0a 0b 0c 0d 0e 0f");
    println!("-------  -----------------------------------------------");
    
    for addr in (0x08ca0..=0x08ce0).step_by(16) {
        print!("{:05x}:  ", addr);
        for i in 0..16 {
            let byte_addr = addr + i;
            if byte_addr <= 0x08ce0 && byte_addr < game_data.len() {
                print!("{:02x} ", game_data[byte_addr]);
            } else {
                print!("   ");
            }
        }
        println!();
    }
    
    println!("\nSpecific bytes of interest:");
    println!("0x8cac: {:02x} {:02x} {:02x} (PUSH G47)", 
            game_data[0x8cac], game_data[0x8cad], game_data[0x8cae]);
    println!("0x8caf: {:02x} {:02x} {:02x} (PULL L01)",
            game_data[0x8caf], game_data[0x8cb0], game_data[0x8cb1]);
    
    println!("\nIf we decode from 0x8cad (middle of PUSH):");
    println!("0x8cad: {:02x} {:02x} = ", game_data[0x8cad], game_data[0x8cae]);
    let byte1 = game_data[0x8cad];
    let byte2 = game_data[0x8cae];
    
    // Check if byte1 could be interpreted as piracy (0OP:0x0F)
    if byte1 == 0xBF {
        println!("  0xBF = 1011 1111 binary");
        println!("  Top 2 bits: 10 = Short form");
        println!("  Bit 5: 1 = 0OP");
        println!("  Bottom 4 bits: 1111 = 0x0F = piracy!");
    }
    
    Ok(())
}