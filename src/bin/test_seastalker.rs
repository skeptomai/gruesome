use gruesome::disassembler::Disassembler;
use gruesome::interpreter::Interpreter;
use gruesome::vm::{Game, VM};
use log::info;
use std::env;

fn main() {
    env_logger::init();
    
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <game.z3>", args[0]);
        std::process::exit(1);
    }
    
    // Load game file
    let game_data = match std::fs::read(&args[1]) {
        Ok(data) => data,
        Err(e) => {
            eprintln!("Failed to read game file: {}", e);
            std::process::exit(1);
        }
    };
    
    println!("Loaded {} bytes", game_data.len());
    
    // Create game
    let game = match Game::from_memory(game_data) {
        Ok(g) => g,
        Err(e) => {
            eprintln!("Failed to parse game file: {}", e);
            std::process::exit(1);
        }
    };
    
    println!("Game version: {}", game.header.version);
    println!("Initial PC: 0x{:04x}", game.header.initial_pc);
    println!("High memory: 0x{:04x}", game.header.base_high_mem);
    println!("Static memory: 0x{:04x}", game.header.base_static_mem);
    
    // Check if this looks like Seastalker
    println!("Serial: {}", game.header.serial);
    
    // Create VM
    let vm = VM::new(game);
    let disasm = Disassembler::new(&vm.game);
    
    // Disassemble first few instructions
    println!("\nFirst few instructions:");
    if let Ok(output) = disasm.disassemble_range(vm.game.header.initial_pc as u32, 
                                                 vm.game.header.initial_pc as u32 + 20) {
        println!("{}", output);
    }
    
    println!("\nGame loaded successfully!");
}