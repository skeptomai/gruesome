use infocom::vm::{Game, VM};
use infocom::instruction::Instruction;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let game_path = "resources/test/zork1/DATA/ZORK1.DAT";
    
    // Load game data
    let mut f = std::fs::File::open(game_path)?;
    let mut memory = Vec::new();
    std::io::Read::read_to_end(&mut f, &mut memory)?;
    
    let game = Game::from_memory(memory)?;
    let vm = VM::new(game);
    
    println!("=== Properly decoding instruction at 0x57f7 ===\n");
    
    let addr = 0x57f7;
    
    match Instruction::decode(&vm.game.memory, addr, vm.game.header.version) {
        Ok(inst) => {
            println!("Decoded instruction: {:?}", inst);
            println!();
            println!("Opcode: 0x{:02x} ({})", inst.opcode,
                     match inst.opcode {
                         0x11 => "get_prop",
                         _ => "?"
                     });
            println!("Operands: {:?}", inst.operands);
            
            if inst.operands.len() >= 2 {
                println!("\nFirst operand value: 0x{:02x}", inst.operands[0]);
                println!("Second operand value: 0x{:02x}", inst.operands[1]);
                
                // Check operand types
                if inst.operand_types.len() >= 2 {
                    println!("\nFirst operand type: {:?}", inst.operand_types[0]);
                    println!("Second operand type: {:?}", inst.operand_types[1]);
                    
                    if let infocom::instruction::OperandType::Variable = inst.operand_types[0] {
                        let var_num = inst.operands[0];
                        println!("\nFirst operand is variable 0x{:02x}", var_num);
                        if var_num >= 0x10 {
                            println!("This is global 0x{:02x} = global {}", var_num - 0x10, var_num - 0x10);
                        }
                    }
                }
            }
        }
        Err(e) => println!("Error decoding: {}", e),
    }
    
    println!("\nFrom debug log: store #007f, #0004 at PC 04f8b");
    println!("This stored 4 into global 0x6f (111)");
    
    Ok(())
}