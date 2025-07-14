use std::fs::File;
use std::io::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut f = File::open("resources/test/zork1/DATA/ZORK1.DAT")?;
    let mut game_data = Vec::new();
    f.read_to_end(&mut game_data)?;

    println!("Checking what's at 0x0754c...\n");
    
    // The execution path seems to be:
    // 0x07547 -> 0x0754c -> 0x07550
    
    // Check 0x07547
    println!("At 0x07547:");
    if let Ok(inst) = infocom::instruction::Instruction::decode(&game_data, 0x07547, 3) {
        println!("  {}", inst.format_with_version(3));
        println!("  Size: {} bytes", inst.size);
        println!("  Next PC: 0x{:05x}", 0x07547 + inst.size);
        
        if 0x07547 + inst.size != 0x0754c {
            println!("  ERROR: Expected next PC to be 0x0754c!");
        }
    }
    
    // Check 0x0754c
    println!("\nAt 0x0754c:");
    print!("  Raw bytes: ");
    for i in 0..8 {
        if 0x0754c + i < game_data.len() {
            print!("{:02x} ", game_data[0x0754c + i]);
        }
    }
    println!();
    
    if let Ok(inst) = infocom::instruction::Instruction::decode(&game_data, 0x0754c, 3) {
        println!("  {}", inst.format_with_version(3));
        println!("  Size: {} bytes", inst.size);
        println!("  Next PC: 0x{:05x}", 0x0754c + inst.size);
        
        if 0x0754c + inst.size == 0x07550 {
            println!("  ^^^ This leads directly to 0x07550!");
        }
    }
    
    // Let me trace the exact sequence
    println!("\nTracing from branch target 0x07547:");
    let mut pc = 0x07547;
    
    for step in 0..5 {
        println!("\nStep {}: PC = 0x{:05x}", step, pc);
        
        if pc == 0x07550 {
            println!("  ^^^ Reached 0x07550!");
            
            // What happens here?
            if let Ok(inst) = infocom::instruction::Instruction::decode(&game_data, pc, 3) {
                println!("  Instruction: {}", inst.format_with_version(3));
                println!("  Size: {} bytes", inst.size);
                println!("  Next would be: 0x{:05x}", pc + inst.size);
                
                if pc + inst.size == 0x07554 {
                    println!("  ^^^ This leads to our error address!");
                }
            }
            break;
        }
        
        if let Ok(inst) = infocom::instruction::Instruction::decode(&game_data, pc, 3) {
            println!("  {}", inst.format_with_version(3));
            pc += inst.size;
        } else {
            println!("  DECODE ERROR");
            break;
        }
    }
    
    // The problem summary
    println!("\n\nPROBLEM SUMMARY:");
    println!("1. We branch from 0x0751f to 0x07547");
    println!("2. At 0x07547 we execute some instruction");
    println!("3. At 0x0754c we execute another instruction");  
    println!("4. This leads us to 0x07550 where we execute MOD");
    println!("5. The MOD instruction ends at 0x07554");
    println!("6. BUT 0x07554 is in the middle of another instruction!");
    println!("7. When we try to execute from 0x07554, we get the error");
    
    Ok(())
}