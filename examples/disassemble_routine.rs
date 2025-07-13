use std::env;
use std::fs::File;
use std::io::{self, prelude::*};
use std::path::PathBuf;

use infocom::disassembler;
use infocom::game::GameFile;
use infocom::zrand::ZRand;

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();
    
    if args.len() < 2 {
        println!("Usage: {} <packed_address>", args[0]);
        println!("Example: {} 13873", args[0]);
        println!("         {} 0x3631", args[0]);
        return Ok(());
    }
    
    // Parse packed address
    let packed_addr = if args[1].starts_with("0x") || args[1].starts_with("0X") {
        u16::from_str_radix(&args[1][2..], 16).unwrap_or(0)
    } else {
        args[1].parse::<u16>().unwrap_or(0)
    };
    
    // Load Zork 1
    let path = PathBuf::from("resources/test/zork1/DATA/ZORK1.DAT");
    let mut f = File::open(&path)?;
    let mut all_bytes = Vec::new();
    f.read_to_end(&mut all_bytes)?;
    
    // Create random generator
    let mut zrg = ZRand::new_uniform();
    
    // Create game file
    let game = GameFile::new(&all_bytes, &mut zrg);
    let version = game.version();
    
    println!("Disassembling routine at packed address {:#06x} (decimal {})", 
             packed_addr, packed_addr);
    println!("Game version: {}", version);
    
    // Disassemble the routine
    match disassembler::disassemble_routine(game.bytes(), packed_addr, version as u8) {
        Ok(output) => println!("{}", output),
        Err(e) => eprintln!("Error: {}", e),
    }
    
    Ok(())
}