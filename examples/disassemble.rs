use std::env;
use std::fs::File;
use std::io::{self, prelude::*};
use std::path::PathBuf;

use infocom::disassembler;
use infocom::game::GameFile;
use infocom::zrand::ZRand;

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();
    
    // Parse command line arguments
    let mut data_file = "resources/test/zork1/DATA/ZORK1.DAT".to_string();
    let mut start_pc = 0x4f05; // Default initial PC
    let mut end_pc = 0x5920;   // Default end address
    
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--file" => {
                if i + 1 < args.len() {
                    data_file = args[i + 1].clone();
                    i += 1;
                }
            }
            "--start" => {
                if i + 1 < args.len() {
                    if let Ok(pc) = parse_hex_or_dec(&args[i + 1]) {
                        start_pc = pc;
                    }
                    i += 1;
                }
            }
            "--end" => {
                if i + 1 < args.len() {
                    if let Ok(pc) = parse_hex_or_dec(&args[i + 1]) {
                        end_pc = pc;
                    }
                    i += 1;
                }
            }
            "--help" => {
                print_help(&args[0]);
                return Ok(());
            }
            _ => {
                if !args[i].starts_with("--") && i == 1 {
                    // First non-flag argument is the file
                    data_file = args[i].clone();
                }
            }
        }
        i += 1;
    }
    
    println!("Z-Machine Disassembler");
    println!("Loading game: {}", data_file);
    
    let mut path = PathBuf::from(&data_file);
    if !path.exists() {
        // Try relative to project root
        path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push(&data_file);
    }
    
    if !path.exists() {
        eprintln!("Error: Game file not found: {}", data_file);
        return Ok(());
    }
    
    // Load game file
    let mut f = File::open(&path)?;
    let mut all_bytes = Vec::new();
    f.read_to_end(&mut all_bytes)?;
    
    // Create random generator (needed for GameFile)
    let mut zrg = ZRand::new_uniform();
    
    // Create game file
    let game = GameFile::new(&all_bytes, &mut zrg);
    
    println!("\nDisassembling from {:#06x} to {:#06x}", start_pc, end_pc);
    println!("{}", "=".repeat(80));
    
    // Perform disassembly
    match disassembler::disassemble_range(game.bytes(), start_pc, end_pc) {
        Ok(output) => {
            println!("{}", output);
        }
        Err(e) => {
            eprintln!("Disassembly error: {}", e);
        }
    }
    
    Ok(())
}

fn parse_hex_or_dec(s: &str) -> Result<usize, std::num::ParseIntError> {
    if s.starts_with("0x") || s.starts_with("0X") {
        usize::from_str_radix(&s[2..], 16)
    } else {
        s.parse()
    }
}

fn print_help(program_name: &str) {
    println!("Usage: {} [options] [game_file]", program_name);
    println!("\nOptions:");
    println!("  --file <path>     Game file to disassemble (default: resources/test/zork1/DATA/ZORK1.DAT)");
    println!("  --start <addr>    Start address in hex (e.g., 0x4f05) or decimal");
    println!("  --end <addr>      End address in hex (e.g., 0x5920) or decimal");
    println!("  --help            Show this help message");
    println!("\nExample:");
    println!("  {} --start 0x4f05 --end 0x5920", program_name);
    println!("  {} resources/test/zork1/DATA/ZORK1.DAT --start 0x5000 --end 0x5100", program_name);
}