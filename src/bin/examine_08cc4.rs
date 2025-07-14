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

    println!("Examining the problematic call_2s at 0x08cc4...\n");
    
    // Disassemble around the call
    println!("Instructions around 0x08cc4:");
    let instructions = debugger.disassemble_range(0x08ca0, 40);
    for (i, inst) in instructions.iter().enumerate() {
        let marker = if inst.contains("08cc4") { " --> " } else { "     " };
        println!("{}{}", marker, inst);
    }
    
    // Look at specific instruction
    println!("\nThe problematic instruction:");
    if let Ok(inst) = debugger.disassemble_at(0x08cc4) {
        println!("{}", inst);
    }
    
    // Decode it manually
    let memory = &debugger.interpreter.vm.game.memory;
    println!("\nRaw bytes at 0x08cc4:");
    for i in 0..5 {
        let addr = 0x08cc4 + i;
        if addr < memory.len() {
            print!("{:02x} ", memory[addr]);
        }
    }
    println!();
    
    if let Ok(inst) = infocom::instruction::Instruction::decode(memory, 0x08cc4, 3) {
        println!("\nDecoded instruction:");
        println!("Opcode: 0x{:02x}", inst.opcode);
        println!("Operands: {:?}", inst.operands);
        println!("This is call_2s Vb2, #0000 -> V22");
        println!("\nThe issue: Vb2 (V178) contains a value that's too small to be a valid routine address");
        println!("When called, it jumps to the header area instead of actual code.");
    }
    
    // Show what the main routine at 0x04f05 does before getting here
    println!("\n\nTo understand why Vb2 has a bad value, we need to trace back through:");
    println!("1. Main routine at 0x04f05");
    println!("2. Call to 0x3f02 at PC 0x04f94"); 
    println!("3. Call to 0x464d at PC 0x07e05");
    println!("4. Finally reaching this routine at 0x08c98");
    
    Ok(())
}