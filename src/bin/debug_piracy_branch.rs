use infocom::debugger::Debugger;
use infocom::vm::{Game, VM};
use std::fs::File;
use std::io::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut f = File::open("resources/test/zork1/DATA/ZORK1.DAT")?;
    let mut game_data = Vec::new();
    f.read_to_end(&mut game_data)?;

    let game = Game::from_memory(game_data)?;
    let vm = VM::new(game);
    let debugger = Debugger::new(vm);

    println!("Examining the piracy instruction at 0x08cad...\n");
    
    // Disassemble the piracy instruction
    if let Ok(disasm) = debugger.disassemble_at(0x08cad) {
        println!("Instruction: {}", disasm);
    }
    
    // Look at the raw bytes at this address
    let memory = &debugger.interpreter.vm.game.memory;
    println!("\nRaw bytes at 0x08cad:");
    for i in 0..6 {
        let addr = 0x08cad + i;
        if addr < memory.len() {
            print!("{:02x} ", memory[addr]);
        }
    }
    println!();
    
    // Decode the instruction manually
    println!("\nManual instruction decode:");
    if let Ok(inst) = infocom::instruction::Instruction::decode(memory, 0x08cad, 3) {
        println!("Opcode: 0x{:02x}", inst.opcode);
        println!("Form: {:?}", inst.form);
        println!("Operand types: {:?}", inst.operand_types);
        println!("Operands: {:?}", inst.operands);
        println!("Store variable: {:?}", inst.store_var);
        println!("Branch: {:?}", inst.branch);
        println!("Size: {} bytes", inst.size);
        println!("Formatted: {}", inst.format_with_version(3));
        
        if let Some(ref branch) = inst.branch {
            println!("\nBranch details:");
            println!("  on_true: {}", branch.on_true);
            println!("  offset: {}", branch.offset);
            println!("  Branch should be taken when condition is: {}", branch.on_true);
        }
    }
    
    // Check the opcode byte analysis
    let opcode_byte = memory[0x08cad];
    println!("\nOpcode analysis:");
    println!("First byte: 0x{:02x} ({})", opcode_byte, opcode_byte);
    
    // Check if this is 0OP:0x0F (piracy)
    if (opcode_byte & 0x80) != 0 && (opcode_byte & 0x40) == 0 {
        let opcode = opcode_byte & 0x0F;
        println!("This is 0OP opcode: 0x{:02x} ({})", opcode, opcode);
        if opcode == 0x0F {
            println!("This is the PIRACY instruction (0OP:15)");
        }
    }
    
    // According to spec, piracy should always branch (be gullible)
    println!("\nZ-Machine spec says:");
    println!("piracy instruction should always branch (be gullible)");
    println!("Current implementation passes condition=true to do_branch");
    
    Ok(())
}