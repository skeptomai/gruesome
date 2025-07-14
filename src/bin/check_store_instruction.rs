use infocom::instruction::Instruction;
use std::fs::File;
use std::io::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut f = File::open("resources/test/zork1/DATA/ZORK1.DAT")?;
    let mut game_data = Vec::new();
    f.read_to_end(&mut game_data)?;

    println!("Checking specific instruction differences\n");
    
    // Check the STORE instruction at 0x8ed0
    println!("1. STORE instruction at 0x8ed0:");
    println!("   Expected: STORE L06,#00");
    println!("   We show:  STORE #0007, #0000");
    println!("   Note: L06 = Local 7 = variable 0x07");
    println!("   So these are the same! ✓\n");
    
    // Check the JE instruction at 0x8edc
    println!("2. JE instruction at 0x8edc:");
    let addr = 0x8edc;
    let bytes: Vec<u8> = (0..8).map(|i| game_data[addr + i]).collect();
    println!("   Raw bytes: {:02x} {:02x} {:02x} {:02x} {:02x} {:02x}", 
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5]);
    
    if let Ok(inst) = Instruction::decode(&game_data, addr, 3) {
        println!("   Decoded: {}", inst.format_with_version(3));
        println!("   Operands: {:?}", inst.operands);
        
        // The known good shows 3 operands: G6f, L00, (SP)+
        // But we only show 2?
        if inst.operands.len() != 3 {
            println!("   ERROR: Expected 3 operands, got {}", inst.operands.len());
            
            // Check the opcode
            println!("   Opcode: 0x{:02x}", inst.opcode);
            println!("   Form: {:?}", inst.form);
            
            // C1 = 11000001 = Variable form, opcode 0x01 (je)
            // AB = 10101011 = operand types
            //   Op1: 10 = Variable
            //   Op2: 10 = Variable  
            //   Op3: 10 = Variable
            //   Op4: 11 = Omitted
            
            println!("\n   Manual decode of 0xC1 0xAB:");
            println!("   0xC1 = 11000001 binary");
            println!("     Top 2 bits: 11 = Variable form");
            println!("     Bit 5: 0 = 2OP");
            println!("     Bottom 5: 00001 = opcode 0x01 = JE");
            
            println!("   0xAB = 10101011 binary");
            println!("     Op1: 10 = Variable");
            println!("     Op2: 10 = Variable");
            println!("     Op3: 10 = Variable");
            println!("     Op4: 11 = Omitted");
            
            println!("\n   So this is Variable 2OP form of JE with 3 variable operands");
        }
    }
    
    // Check GET_PARENT at 0x8ed9
    println!("\n3. GET_PARENT at 0x8ed9:");
    println!("   Expected: GET_PARENT L00 -> -(SP)");
    println!("   We show:  GET_PARENT L00 -> V00");
    
    if let Ok(inst) = Instruction::decode(&game_data, 0x8ed9, 3) {
        if let Some(store_var) = inst.store_var {
            println!("   Store variable: 0x{:02x}", store_var);
            if store_var == 0 {
                println!("   0x00 = -(SP) in Z-Machine!");
                println!("   So these are the same! ✓");
            }
        }
    }
    
    // The key issue: Variable 2OP with 3 operands
    println!("\n\nKEY FINDING:");
    println!("The JE instruction at 0x8edc is Variable 2OP form but has 3 operands!");
    println!("This might be related to our error with the MOD instruction.");
    println!("\nVariable 2OP instructions should have exactly 2 operands,");
    println!("but the operand types byte specifies 3 operands here.");
    
    Ok(())
}