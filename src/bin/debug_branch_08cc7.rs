use std::fs::File;
use std::io::prelude::*;
use infocom::instruction::Instruction;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut f = File::open("resources/test/zork1/DATA/ZORK1.DAT")?;
    let mut game_data = Vec::new();
    f.read_to_end(&mut game_data)?;

    println!("Debugging branch at 0x08cc7 that should go to 0x086d4 but goes to STAND...\n");
    
    // Decode the instruction at 0x08cc7
    let pc = 0x08cc7;
    match Instruction::decode(&game_data, pc, 3) {
        Ok(inst) => {
            println!("Instruction at 0x{:05x}:", pc);
            println!("  {}", inst.format_with_version(3));
            println!("  Size: {} bytes", inst.size);
            
            // Show raw bytes
            print!("  Raw bytes:");
            for i in 0..inst.size.min(10) {
                print!(" {:02x}", game_data[pc + i]);
            }
            println!();
            
            if let Some(branch) = &inst.branch {
                println!("\nBranch info:");
                println!("  On condition: {}", if branch.on_true { "TRUE" } else { "FALSE" });
                println!("  Offset: {} (0x{:04x})", branch.offset, branch.offset as u16);
                
                // Calculate where it should jump
                // Branch offset is relative to address after branch data
                let branch_target = (pc as i32 + inst.size as i32 + branch.offset as i32 - 2) as u32;
                println!("  Calculated target: 0x{:05x}", branch_target);
                println!("  Expected target: 0x086d4");
                println!("  Difference: {} bytes", branch_target as i32 - 0x086d4i32);
                
                // Let's manually parse the branch bytes
                println!("\nManual parsing:");
                
                // Find where branch data starts (after operands and store)
                let mut branch_offset = pc + 1; // Skip opcode
                
                // jl is a 2OP instruction, so we need to skip operands
                // The trace shows "jl #0005, Vd1"
                println!("  Opcode: 0x{:02x}", game_data[pc]);
                
                // For a Long form 2OP, operand types are in the opcode byte
                // Let's check the exact encoding
                let opcode_byte = game_data[pc];
                println!("  Opcode binary: {:08b}", opcode_byte);
                
                // Long form check
                if (opcode_byte >> 6) < 2 {
                    println!("  Form: Long");
                    // Bit 6 = first operand type (0=small, 1=var)
                    // Bit 5 = second operand type
                    let op1_is_var = (opcode_byte & 0x40) != 0;
                    let op2_is_var = (opcode_byte & 0x20) != 0;
                    
                    println!("  Op1 type: {}", if op1_is_var { "variable" } else { "small constant" });
                    println!("  Op2 type: {}", if op2_is_var { "variable" } else { "small constant" });
                    
                    // Skip operands
                    branch_offset += 1; // First operand (small or var)
                    branch_offset += 1; // Second operand
                    
                    // jl doesn't store, so branch data is next
                    println!("\n  Branch data at offset 0x{:05x}:", branch_offset);
                    let branch_byte1 = game_data[branch_offset];
                    println!("  First branch byte: 0x{:02x} (binary: {:08b})", branch_byte1, branch_byte1);
                    
                    let on_true = (branch_byte1 & 0x80) != 0;
                    let is_short = (branch_byte1 & 0x40) != 0;
                    
                    println!("  Branch on: {}", if on_true { "TRUE" } else { "FALSE" });
                    println!("  Branch format: {}", if is_short { "short (6-bit)" } else { "long (14-bit)" });
                    
                    if !is_short {
                        // Long branch - 14 bits across 2 bytes
                        let branch_byte2 = game_data[branch_offset + 1];
                        println!("  Second branch byte: 0x{:02x}", branch_byte2);
                        
                        let raw_offset = (((branch_byte1 & 0x3F) as u16) << 8) | (branch_byte2 as u16);
                        println!("  Raw 14-bit offset: 0x{:04x} ({})", raw_offset, raw_offset);
                        
                        // Sign extend if needed
                        let signed_offset = if raw_offset & 0x2000 != 0 {
                            // Negative - sign extend
                            (raw_offset | 0xC000) as i16
                        } else {
                            raw_offset as i16
                        };
                        
                        println!("  Signed offset: {} (0x{:04x})", signed_offset, signed_offset as u16);
                        
                        // Calculate jump target
                        // PC after instruction = pc + inst.size
                        // Jump target = PC after + offset - 2
                        let pc_after = pc + inst.size;
                        let target = (pc_after as i32 + signed_offset as i32 - 2) as u32;
                        
                        println!("\n  Jump calculation:");
                        println!("    PC of instruction: 0x{:05x}", pc);
                        println!("    Instruction size: {}", inst.size);
                        println!("    PC after instruction: 0x{:05x}", pc_after);
                        println!("    Offset: {} (0x{:04x})", signed_offset, signed_offset as u16);
                        println!("    PC after + offset - 2 = 0x{:05x}", target);
                        
                        if target != 0x086d4 {
                            println!("\n  ERROR: Target 0x{:05x} != expected 0x086d4", target);
                            println!("  Difference: {} bytes", target as i32 - 0x086d4);
                        }
                    }
                }
            }
        }
        Err(e) => println!("Error decoding instruction: {}", e),
    }

    Ok(())
}