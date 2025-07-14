use std::fs::File;
use std::io::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut f = File::open("resources/test/zork1/DATA/ZORK1.DAT")?;
    let mut game_data = Vec::new();
    f.read_to_end(&mut game_data)?;

    println!("Debugging 14-bit branch offset at 0x8cb2...\n");
    
    // The JZ instruction at 0x8cb2
    let first_byte = game_data[0x8cb4];  // Branch byte
    let second_byte = game_data[0x8cb5]; // Second byte of offset
    
    println!("Raw bytes:");
    println!("  First branch byte:  0x{:02x} = {:08b}", first_byte, first_byte);
    println!("  Second branch byte: 0x{:02x} = {:08b}", second_byte, second_byte);
    
    println!("\nBranch byte analysis (0x{:02x}):", first_byte);
    println!("  Bit 7 (on_true): {} = branch on {}", 
            (first_byte & 0x80) >> 7,
            if first_byte & 0x80 != 0 { "TRUE" } else { "FALSE" });
    println!("  Bit 6 (format):  {} = {} form", 
            (first_byte & 0x40) >> 6,
            if first_byte & 0x40 != 0 { "long (14-bit)" } else { "short (6-bit)" });
    println!("  Bits 5-0:        0x{:02x} = {}", first_byte & 0x3F, first_byte & 0x3F);
    
    // Calculate 14-bit offset
    let high_6 = (first_byte & 0x3F) as u16;
    let low_8 = second_byte as u16;
    let raw_14bit = (high_6 << 8) | low_8;
    
    println!("\n14-bit offset calculation:");
    println!("  High 6 bits: 0x{:02x} = {}", high_6, high_6);
    println!("  Low 8 bits:  0x{:02x} = {}", low_8, low_8);
    println!("  Combined:    0x{:04x} = {} (unsigned)", raw_14bit, raw_14bit);
    
    // Sign extension for 14-bit signed
    let offset = if raw_14bit & 0x2000 != 0 {
        // Negative - sign extend
        (raw_14bit | 0xC000) as i16
    } else {
        raw_14bit as i16
    };
    
    println!("  Bit 13 (sign bit): {}", if raw_14bit & 0x2000 != 0 { "1 (negative)" } else { "0 (positive)" });
    println!("  Signed value: {} (0x{:04x})", offset, offset as u16);
    
    // Calculate target
    let pc_after_inst = 0x8cb6; // PC after the 4-byte instruction
    let target = (pc_after_inst as i32 + offset as i32 - 2) as u32;
    
    println!("\nBranch target calculation:");
    println!("  PC after instruction: 0x{:05x}", pc_after_inst);
    println!("  Offset: {:+} (0x{:04x})", offset, offset as u16);
    println!("  Target: PC + offset - 2 = 0x{:05x}", target);
    
    println!("\nExpected: Should branch to 0x8ce4");
    if target == 0x8ce4 {
        println!("✓ Correct!");
    } else {
        println!("✗ Wrong! Difference: {}", 0x8ce4 as i32 - target as i32);
        
        // What offset would give us 0x8ce4?
        let needed_offset = 0x8ce4 as i32 - pc_after_inst as i32 + 2;
        println!("\nNeeded offset: {} (0x{:04x})", needed_offset, needed_offset as u16);
        
        // Maybe the bytes are swapped?
        let swapped = (low_8 << 8) | high_6;
        println!("\nIf bytes were swapped: 0x{:04x} = {}", swapped, swapped);
    }
    
    Ok(())
}