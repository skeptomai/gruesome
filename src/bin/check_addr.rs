use gruesome::vm::Game;
use std::env;
use std::fs::File;
use std::io::Read;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: {} <z-code-file> <hex-address>", args[0]);
        std::process::exit(1);
    }

    let filename = &args[1];
    let addr_str = &args[2];
    
    // Parse hex address
    let addr = u32::from_str_radix(addr_str.trim_start_matches("0x"), 16)?;
    
    // Load game file
    let mut file = File::open(filename)?;
    let mut memory = Vec::new();
    file.read_to_end(&mut memory)?;
    
    let game = Game::from_memory(memory)?;
    
    // Check what's at this address
    if (addr as usize) < game.memory.len() {
        let vars = game.memory[addr as usize];
        println!("Address 0x{:04x}:", addr);
        println!("  Locals count byte: {} (0x{:02x})", vars, vars);
        
        if vars <= 15 {
            println!("  Valid locals count");
            
            // For V1-4, show local variable initial values
            if game.header.version <= 4 {
                let mut pc = addr + 1;
                for i in 0..vars {
                    if (pc as usize + 1) < game.memory.len() {
                        let local_val = ((game.memory[pc as usize] as u16) << 8) | 
                                      (game.memory[(pc + 1) as usize] as u16);
                        println!("    Local {}: 0x{:04x}", i, local_val);
                        pc += 2;
                    }
                }
            }
        } else {
            println!("  NOT a valid locals count (> 15)");
        }
        
        // Show next few bytes
        println!("  Next 16 bytes:");
        print!("    ");
        for i in 0..16 {
            if (addr as usize + i) < game.memory.len() {
                print!("{:02x} ", game.memory[addr as usize + i]);
            }
        }
        println!();
    } else {
        println!("Address 0x{:04x} is beyond file size", addr);
    }
    
    Ok(())
}