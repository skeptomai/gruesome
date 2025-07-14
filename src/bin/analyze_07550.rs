use std::fs::File;
use std::io::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut f = File::open("resources/test/zork1/DATA/ZORK1.DAT")?;
    let mut game_data = Vec::new();
    f.read_to_end(&mut game_data)?;

    println!("Analyzing what's really at 0x07550...\n");
    
    // Show bytes around 0x07550
    println!("Bytes around 0x07550:");
    for addr in 0x07548..=0x07558 {
        if addr < game_data.len() {
            println!("{:05x}: {:02x}", addr, game_data[addr]);
        }
    }
    
    println!("\nTrying to decode as different instruction forms:");
    
    // What our trace says
    println!("\n1. Our trace says it's: mod V64, #00c7 -> V45");
    println!("   This would be bytes: 58 64 c7 45");
    
    // Check actual bytes at 0x07550
    print!("\n2. Actual bytes at 0x07550: ");
    for i in 0..4 {
        if 0x07550 + i < game_data.len() {
            print!("{:02x} ", game_data[0x07550 + i]);
        }
    }
    println!();
    
    // Try to decode
    if let Ok(inst) = infocom::instruction::Instruction::decode(&game_data, 0x07550, 3) {
        println!("   Decodes as: {}", inst.format_with_version(3));
        println!("   Size: {} bytes", inst.size);
    }
    
    // But wait - according to our earlier analysis, 0x07550 should be
    // in the middle of a print_ret that starts at 0x0753a
    println!("\n3. According to earlier analysis:");
    println!("   0x0753a-0x07561 should be a print_ret instruction");
    println!("   0x07550 would be in the middle of that");
    
    // Let's verify the print_ret at 0x0753a
    println!("\n4. Checking the print_ret at 0x0753a:");
    if let Ok(inst) = infocom::instruction::Instruction::decode(&game_data, 0x0753a, 3) {
        println!("   {}", inst.format_with_version(3));
        println!("   Size: {} bytes (0x{:05x} to 0x{:05x})", 
                inst.size, 0x0753a, 0x0753a + inst.size);
        
        if inst.text.is_some() {
            println!("   Text: \"{}\"", inst.text.as_ref().unwrap());
        }
    }
    
    // The issue might be that the earlier decode was wrong
    // Let's trace instruction by instruction from 0x0751f
    println!("\n5. Step-by-step decode from 0x0751f:");
    let mut pc = 0x0751f;
    let mut step = 0;
    
    while pc < 0x07560 && step < 20 {
        if let Ok(inst) = infocom::instruction::Instruction::decode(&game_data, pc, 3) {
            println!("   {:05x}: {} (size {})", pc, inst.format_with_version(3), inst.size);
            pc += inst.size;
        } else {
            println!("   {:05x}: DECODE ERROR", pc);
            break;
        }
        step += 1;
    }
    
    Ok(())
}