use infocom::vm::{Game, VM};
use std::fs::File;
use std::io::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut f = File::open("resources/test/zork1/DATA/ZORK1.DAT")?;
    let mut game_data = Vec::new();
    f.read_to_end(&mut game_data)?;

    let game = Game::from_memory(game_data)?;
    let vm = VM::new(game);
    
    println!("Z-Machine Memory Layout:");
    println!("Version: {}", vm.game.header.version);
    println!("Initial PC: 0x{:05x}", vm.game.header.initial_pc);
    println!("Static memory base: 0x{:05x}", vm.game.header.base_static_mem);
    println!("High memory base: 0x{:05x}", vm.game.header.base_high_mem);
    println!("Total memory size: 0x{:05x}", vm.game.memory.len());
    
    println!("\nMemory regions:");
    println!("Dynamic memory: 0x00000 - 0x{:05x}", vm.game.header.base_static_mem - 1);
    println!("Static memory:  0x{:05x} - 0x{:05x}", vm.game.header.base_static_mem, vm.game.header.base_high_mem - 1);
    println!("High memory:    0x{:05x} - 0x{:05x}", vm.game.header.base_high_mem, vm.game.memory.len() - 1);
    
    println!("\nAddress 0x4f15 analysis:");
    println!("Is 0x4f15 < base_static_mem (0x{:05x})? {}", 
             vm.game.header.base_static_mem, 
             0x4f15 < vm.game.header.base_static_mem as u32);
    
    if 0x4f15 < vm.game.header.base_static_mem as u32 {
        println!("0x4f15 is in DYNAMIC memory (writable)");
    } else if 0x4f15 < vm.game.header.base_high_mem as u32 {
        println!("0x4f15 is in STATIC memory (read-only)");
    } else {
        println!("0x4f15 is in HIGH memory (read-only)");
    }
    
    Ok(())
}