use infocom::vm::Game;
use infocom::disassembler::Disassembler;
use std::env;
use std::fs::File;
use std::io::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <game_file> [--addr <addr>] [--len <len>]", args[0]);
        return Ok(());
    }

    let filename = &args[1];
    let mut addr = 0x3770u32;  // Default to the routine we want
    let mut len = 100u32;      // Default length

    // Parse command line arguments
    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--addr" => {
                if i + 1 < args.len() {
                    addr = u32::from_str_radix(&args[i + 1].trim_start_matches("0x"), 16)?;
                    i += 2;
                } else {
                    eprintln!("--addr requires an argument");
                    return Ok(());
                }
            }
            "--len" => {
                if i + 1 < args.len() {
                    len = args[i + 1].parse()?;
                    i += 2;
                } else {
                    eprintln!("--len requires an argument");
                    return Ok(());
                }
            }
            _ => {
                eprintln!("Unknown argument: {}", args[i]);
                i += 1;
            }
        }
    }

    // Load the game file
    let mut file = File::open(filename)?;
    let mut game_data = Vec::new();
    file.read_to_end(&mut game_data)?;
    
    let game = Game::from_memory(game_data)?;
    let disasm = Disassembler::new(&game);

    println!("Disassembling from 0x{:05x} for {} bytes:", addr, len);
    println!("----------------------------------------");
    
    match disasm.disassemble_range(addr, addr + len) {
        Ok(output) => println!("{}", output),
        Err(e) => eprintln!("Error disassembling: {}", e),
    }

    Ok(())
}