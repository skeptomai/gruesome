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

    println!("Examining the add instruction at 0x054bd...\n");
    
    // Disassemble the problematic instruction
    if let Ok(disasm) = debugger.disassemble_at(0x054bd) {
        println!("Instruction: {}", disasm);
    }
    
    // Look at the raw bytes at this address
    let memory = &debugger.interpreter.vm.game.memory;
    println!("\nRaw bytes at 0x054bd:");
    for i in 0..8 {
        let addr = 0x054bd + i;
        if addr < memory.len() {
            print!("{:02x} ", memory[addr]);
        }
    }
    println!();
    
    // Decode the instruction manually
    println!("\nManual instruction decode:");
    if let Ok(inst) = infocom::instruction::Instruction::decode(memory, 0x054bd, 3) {
        println!("Opcode: 0x{:02x}", inst.opcode);
        println!("Form: {:?}", inst.form);
        println!("Operand types: {:?}", inst.operand_types);
        println!("Operands: {:?}", inst.operands);
        println!("Store variable: {:?}", inst.store_var);
        println!("Size: {} bytes", inst.size);
        println!("Formatted: {}", inst.format_with_version(3));
    }
    
    // Check if this is really an add instruction
    let opcode_byte = memory[0x054bd];
    println!("\nOpcode analysis:");
    println!("First byte: 0x{:02x} ({})", opcode_byte, opcode_byte);
    
    // Check the form
    let form = if opcode_byte & 0xC0 == 0xC0 {
        "Variable"
    } else if opcode_byte & 0x80 == 0x80 {
        "Short" 
    } else {
        "Long"
    };
    println!("Form: {}", form);
    
    if form == "Long" {
        let opcode = opcode_byte & 0x1F;
        println!("2OP opcode: 0x{:02x} ({})", opcode, opcode);
        
        // 0x14 = 20 decimal = add instruction
        if opcode == 0x14 {
            println!("This should be ADD instruction (2OP:20)");
        } else {
            println!("This is 2OP:{} instruction", opcode);
        }
        
        // Check operand types
        let op1_type = if opcode_byte & 0x40 != 0 { "Variable" } else { "Small constant" };
        let op2_type = if opcode_byte & 0x20 != 0 { "Variable" } else { "Small constant" };
        println!("Operand 1 type: {}", op1_type);
        println!("Operand 2 type: {}", op2_type);
    }
    
    Ok(())
}