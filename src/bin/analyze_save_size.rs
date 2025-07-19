use gruesome::instruction::Instruction;
use gruesome::interpreter::{ExecutionResult, Interpreter};
use gruesome::vm::{Game, VM};
use std::io::Read;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let game_path = "resources/test/zork1/DATA/ZORK1.DAT";

    // Load game data
    let mut f = std::fs::File::open(game_path)?;
    let mut memory = Vec::new();
    f.read_to_end(&mut memory)?;

    println!("=== Analyzing Quetzal Save File Sizes ===\n");

    // Save at different points in the game
    println!("1. Initial state (just started):");
    let game = Game::from_memory(memory.clone())?;
    let vm = VM::new(game);
    save_and_analyze(&vm, "initial.sav")?;

    // Run some instructions to change state
    println!("\n2. After running 1000 instructions:");
    let game = Game::from_memory(memory.clone())?;
    let vm = VM::new(game);
    let mut interpreter = Interpreter::new(vm);

    for _ in 0..1000 {
        let pc = interpreter.vm.pc;
        if let Ok(inst) = Instruction::decode(&interpreter.vm.game.memory, pc as usize, 3) {
            interpreter.vm.pc += inst.size as u32;
            match interpreter.execute_instruction(&inst) {
                Ok(ExecutionResult::Quit) => break,
                Err(_) => break,
                _ => {}
            }
        } else {
            break;
        }
    }
    save_and_analyze(&interpreter.vm, "after1000.sav")?;

    // Show what contributes to save file size
    println!("\n3. Memory analysis:");
    let game = Game::from_memory(memory.clone())?;
    let vm = VM::new(game);
    let dynamic_size = vm.game.header.base_static_mem;
    println!("Dynamic memory size: {} bytes", dynamic_size);
    println!("This includes:");
    println!("  - Object table");
    println!("  - Property data");
    println!("  - Global variables");
    println!("  - Arrays and other dynamic data");

    println!("\nThe save file only stores CHANGES to this memory.");
    println!("Early in the game, very little has changed, so the file is tiny!");

    Ok(())
}

fn save_and_analyze(vm: &VM, filename: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Create save game
    let save = gruesome::quetzal::save::SaveGame::from_vm(vm)?;
    let path = std::path::Path::new(filename);
    save.save_to_file(path)?;

    // Analyze the file
    let metadata = std::fs::metadata(filename)?;
    println!("  Save file '{}': {} bytes", filename, metadata.len());

    // Show compression ratio
    let dynamic_size = vm.game.header.base_static_mem;
    let ratio = (metadata.len() as f64 / dynamic_size as f64) * 100.0;
    println!("  Compression ratio: {:.1}% of dynamic memory", ratio);

    // Clean up
    std::fs::remove_file(filename).ok();

    Ok(())
}
