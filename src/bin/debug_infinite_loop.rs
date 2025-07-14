use infocom::vm::{Game, VM};
use infocom::instruction::Instruction;
use std::fs::File;
use std::io::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut f = File::open("resources/test/zork1/DATA/ZORK1.DAT")?;
    let mut game_data = Vec::new();
    f.read_to_end(&mut game_data)?;

    let game = Game::from_memory(game_data)?;
    let vm = VM::new(game);
    
    println!("Investigating infinite loop at PC 0x054b7...\n");
    
    let pc: u32 = 0x054b7;
    
    // Decode the instruction at this location
    if let Ok(inst) = Instruction::decode(&vm.game.memory, pc as usize, vm.game.header.version) {
        println!("Instruction at 0x{:05x}:", pc);
        println!("  {}", inst.format_with_version(vm.game.header.version));
        println!("  Opcode: 0x{:02x}", inst.opcode);
        println!("  Form: {:?}", inst.form);
        println!("  Size: {} bytes", inst.size);
        
        // Show raw bytes
        print!("  Raw bytes: ");
        for i in 0..inst.size as usize {
            print!("{:02x} ", vm.game.memory[pc as usize + i]);
        }
        println!();
        
        // Check if this is a branch instruction
        if let Some(ref branch) = inst.branch {
            println!("  Branch: {:?}", branch);
            println!("  This is a branch instruction - could be causing the loop");
        }
        
        // Check if this is a jump instruction
        let name = inst.name(vm.game.header.version);
        if name.contains("jump") || name.contains("jz") || name.contains("jl") || name.contains("je") {
            println!("  This is a conditional jump that might be looping");
        }
    } else {
        println!("Failed to decode instruction at 0x{:05x}", pc);
    }
    
    // Show surrounding context
    println!("\nSurrounding instructions:");
    let mut addr = pc.saturating_sub(10);
    while addr < pc + 20 {
        if let Ok(inst) = Instruction::decode(&vm.game.memory, addr as usize, vm.game.header.version) {
            let marker = if addr == pc { " --> " } else { "     " };
            println!("{}{:05x}: {}", marker, addr, inst.format_with_version(vm.game.header.version));
            addr += inst.size as u32;
        } else {
            addr += 1;
        }
    }
    
    println!("\nPossible causes of infinite loop:");
    println!("1. Branch instruction always jumping back to itself");
    println!("2. Conditional instruction with condition that never changes");
    println!("3. Bug in argument passing affecting loop condition");
    
    Ok(())
}