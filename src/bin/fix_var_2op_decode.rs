use std::fs::File;
use std::io::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut f = File::open("resources/test/zork1/DATA/ZORK1.DAT")?;
    let mut game_data = Vec::new();
    f.read_to_end(&mut game_data)?;

    println!("Analyzing the Variable 2OP MOD instruction at 0x07554...\n");
    
    let addr = 0x07554;
    let byte0 = game_data[addr];
    let byte1 = game_data[addr + 1];
    
    println!("Raw bytes: {:02x} {:02x}", byte0, byte1);
    println!("Byte 0: 0xD8 = 11011000 binary");
    println!("  Top 2 bits: 11 = Variable form");
    println!("  Bit 5: 0 = 2OP (not VAR)");
    println!("  Bottom 5 bits: 11000 = 0x18 = MOD");
    
    println!("\nByte 1: 0x{:02x} = operand types", byte1);
    let op1_type = (byte1 >> 6) & 3;
    let op2_type = (byte1 >> 4) & 3;
    println!("  Op1: {} = {}", op1_type, match op1_type {
        0 => "Large", 1 => "Small", 2 => "Variable", 3 => "Omitted", _ => "?"
    });
    println!("  Op2: {} = {}", op2_type, match op2_type {
        0 => "Large", 1 => "Small", 2 => "Variable", 3 => "Omitted", _ => "?"
    });
    
    // The problem: Variable 2OP MUST have exactly 2 operands
    // But we have Op2 = Omitted
    
    println!("\nThis is the problem: Variable 2OP form REQUIRES exactly 2 operands.");
    println!("The operand types byte says Op2 is Omitted, which is invalid.");
    
    println!("\nPossible explanations:");
    println!("1. We're decoding from the wrong address (in the middle of another instruction)");
    println!("2. The game has invalid data (unlikely - Zork 1 is well-tested)");
    println!("3. We have a bug in our instruction decoder");
    
    // Let's check what instruction SHOULD be at 0x07554
    // by looking at what comes before
    println!("\nLooking for the instruction that should contain 0x07554...");
    
    for start_addr in (0x07540..0x07554).rev() {
        if let Ok(inst) = infocom::instruction::Instruction::decode(&game_data, start_addr, 3) {
            let end_addr = start_addr + inst.size;
            if end_addr > 0x07554 {
                println!("\nInstruction at 0x{:05x} extends to 0x{:05x}:", start_addr, end_addr);
                println!("  {}", inst.format_with_version(3));
                println!("  This instruction contains address 0x07554!");
                
                // Show the bytes
                print!("  Bytes: ");
                for i in 0..inst.size {
                    if start_addr + i < game_data.len() {
                        print!("{:02x} ", game_data[start_addr + i]);
                    }
                }
                println!();
                
                break;
            }
        }
    }
    
    // The real issue might be that we're jumping into the middle of an instruction
    println!("\nConclusion: We're likely jumping to 0x07554 which is in the middle");
    println!("of another instruction, not the start of a MOD instruction.");
    
    Ok(())
}