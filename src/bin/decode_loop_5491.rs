use infocom::instruction::Instruction;
use std::fs::File;
use std::io::Read;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let game_path = "resources/test/zork1/DATA/ZORK1.DAT";
    
    // Load game data
    let mut f = File::open(game_path)?;
    let mut memory = Vec::new();
    f.read_to_end(&mut memory)?;
    
    // Decode instructions around the loop
    println!("Decoding instructions around the loop at 0x5491:");
    println!();
    
    let addresses = vec![0x548f, 0x5491, 0x5495];
    
    for &addr in &addresses {
        match Instruction::decode(&memory, addr, 3) {
            Ok(inst) => {
                println!("{:05x}: {:?}", addr, inst);
                
                // If it's a branch instruction, decode the branch
                if let Some(branch) = &inst.branch {
                    println!("  Branch: on_true={}, offset={}", branch.on_true, branch.offset);
                    
                    // Calculate where it would branch to
                    let branch_from = addr + inst.size;
                    let target = if branch.offset == 0 {
                        0  // Return false
                    } else if branch.offset == 1 {
                        1  // Return true
                    } else {
                        (branch_from as i32 + branch.offset as i32 - 2) as u32
                    };
                    
                    println!("  Branch from {:05x} to {:05x}", branch_from, target);
                    
                    if target == 0x5491 {
                        println!("  >>> This branches back to 0x5491!");
                    }
                }
                
                println!();
            }
            Err(e) => {
                println!("{:05x}: Failed to decode - {}", addr, e);
                println!();
            }
        }
    }
    
    // Now let's trace through the execution
    println!("\nExecution flow:");
    println!("1. At 0x5491: ADD v148 + 180 -> v3");
    println!("2. PC advances to 0x5495");
    println!("3. At 0x5495: Branch instruction");
    println!("   - If branch is taken, where does it go?");
    
    // Let's calculate the branch target for 0x5495
    let branch_inst_addr = 0x5495;
    let branch_byte = memory[branch_inst_addr];
    
    // For a short form branch (0x74), it's je (jump if equal)
    println!("\nBranch at 0x5495:");
    println!("  Opcode byte: {:02x}", branch_byte);
    
    // Decode properly
    if let Ok(branch_inst) = Instruction::decode(&memory, branch_inst_addr, 3) {
        println!("  Instruction: {:?}", branch_inst);
        
        if branch_inst.opcode == 1 { // je
            println!("  This is 'je' (jump if equal)");
            println!("  Comparing: v{} with {}", branch_inst.operands[0], branch_inst.operands[1]);
        }
    }
    
    Ok(())
}