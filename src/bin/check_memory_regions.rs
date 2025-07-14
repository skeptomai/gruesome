use std::fs::File;
use std::io::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut f = File::open("resources/test/zork1/DATA/ZORK1.DAT")?;
    let mut game_data = Vec::new();
    f.read_to_end(&mut game_data)?;

    println!("Checking Z-Machine memory regions...\n");
    
    // Read header values
    let high_mem_start = ((game_data[0x04] as u16) << 8) | (game_data[0x05] as u16);
    let globals_addr = ((game_data[0x0C] as u16) << 8) | (game_data[0x0D] as u16);
    let static_mem_start = ((game_data[0x0E] as u16) << 8) | (game_data[0x0F] as u16);
    let abbrev_table = ((game_data[0x18] as u16) << 8) | (game_data[0x19] as u16);
    let file_length = ((game_data[0x1A] as u32) << 8) | (game_data[0x1B] as u32);
    
    println!("Header information:");
    println!("  High memory start: 0x{:04x}", high_mem_start);
    println!("  Globals address: 0x{:04x}", globals_addr);
    println!("  Static memory start: 0x{:04x}", static_mem_start);
    println!("  Abbreviations table: 0x{:04x}", abbrev_table);
    println!("  File length: 0x{:04x} ({} bytes)", file_length * 2, file_length * 2);
    
    println!("\nMemory regions:");
    println!("  Dynamic: 0x0000 - 0x{:04x}", static_mem_start - 1);
    println!("  Static:  0x{:04x} - 0x{:04x}", static_mem_start, high_mem_start - 1);
    println!("  High:    0x{:04x} - end", high_mem_start);
    
    // Check where our problematic addresses fall
    println!("\nChecking our addresses:");
    
    let check_addrs = vec![0x07500, 0x07554, 0x08d70, 0x953c];
    
    for addr in check_addrs {
        print!("  0x{:05x}: ", addr);
        
        if addr < static_mem_start {
            println!("Dynamic memory");
        } else if addr < high_mem_start {
            println!("Static memory");
        } else {
            println!("High memory (code)");
        }
        
        // For high memory addresses, check if they look like routine starts
        if addr >= high_mem_start && (addr as usize) < game_data.len() {
            let num_locals = game_data[addr as usize];
            if num_locals <= 15 {
                println!("    Could be routine with {} locals", num_locals);
            } else {
                println!("    First byte 0x{:02x} = {} (too many for locals)", num_locals, num_locals);
            }
        }
    }
    
    // Also check what the actual code start is
    println!("\nCode analysis:");
    println!("  First routine should be at 0x{:04x}", high_mem_start);
    
    // The main routine is usually the first one
    if high_mem_start < game_data.len() as u16 {
        let main_locals = game_data[high_mem_start as usize];
        println!("  Main routine has {} locals", main_locals);
    }
    
    Ok(())
}