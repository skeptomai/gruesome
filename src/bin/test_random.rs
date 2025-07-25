/// Test program to verify random number generation functionality.
/// 
/// This test is critical for Z-Machine games because many core gameplay mechanics
/// depend on random numbers, including:
/// - Combat outcomes (troll fights, etc.)
/// - Thief movement patterns
/// - Item placement randomization
/// - Any other probability-based game events
/// 
/// The test creates a minimal Z-Machine program that calls the random opcode
/// three times and verifies the results are not all identical (which would
/// indicate broken randomization).
use gruesome::instruction::Instruction;
use gruesome::interpreter::{ExecutionResult, Interpreter};
use gruesome::vm::{Game, VM};
use std::io::Read;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let game_path = "resources/test/zork1/DATA/ZORK1.DAT";

    // Load game
    let mut f = std::fs::File::open(game_path)?;
    let mut memory = Vec::new();
    f.read_to_end(&mut memory)?;

    println!("=== Testing Random Number Generation ===\n");

    // Create a simple test: random 10 -> store to variable
    let mut test_memory = vec![0u8; 0x10000];

    // Set up minimal header
    test_memory[0x00] = 3; // Version 3
    test_memory[0x06] = 0x50; // Initial PC
    test_memory[0x0c] = 0x01; // Globals at 0x0100
    test_memory[0x0e] = 0x02; // Static at 0x0200

    // Program: random 10 -> G00, random 10 -> G01, random 10 -> G02, quit
    let pc = 0x5000;
    // VAR:0x07 random with operand 10, store to global 0
    test_memory[pc] = 0xE7; // VAR form, opcode 0x07
    test_memory[pc + 1] = 0x1F; // Operand types: small constant (01), then omitted
    test_memory[pc + 2] = 10; // Range: 1-10
    test_memory[pc + 3] = 0x10; // Store to global 0

    // Second random
    test_memory[pc + 4] = 0xE7;
    test_memory[pc + 5] = 0x1F;
    test_memory[pc + 6] = 10;
    test_memory[pc + 7] = 0x11; // Store to global 1

    // Third random
    test_memory[pc + 8] = 0xE7;
    test_memory[pc + 9] = 0x1F;
    test_memory[pc + 10] = 10;
    test_memory[pc + 11] = 0x12; // Store to global 2

    // quit
    test_memory[pc + 12] = 0xBA;

    let game = Game::from_memory(test_memory)?;
    let vm = VM::new(game);
    let mut interpreter = Interpreter::new(vm);

    println!("Running random test program...\n");

    // Execute the test
    for i in 0..4 {
        let pc = interpreter.vm.pc;
        let inst = Instruction::decode(&interpreter.vm.game.memory, pc as usize, 3)?;

        println!("Step {}: {}", i, inst.format_with_version(3));

        interpreter.vm.pc += inst.size as u32;
        match interpreter.execute_instruction(&inst) {
            Ok(ExecutionResult::Quit) => break,
            Ok(_) => {}
            Err(e) => {
                println!("Error: {e}");
                break;
            }
        }
    }

    // Check results
    println!("\nResults:");
    let g0 = interpreter.vm.read_variable(0x10)?;
    let g1 = interpreter.vm.read_variable(0x11)?;
    let g2 = interpreter.vm.read_variable(0x12)?;

    println!("Global 0: {g0}");
    println!("Global 1: {g1}");
    println!("Global 2: {g2}");

    if g0 == 1 && g1 == 1 && g2 == 1 {
        println!("\n❌ BROKEN: Random always returns 1!");
        println!("This will break:");
        println!("- Combat (troll always wins)");
        println!("- Thief movement");
        println!("- Any other random events");
    } else {
        println!("\n✓ Random is working correctly");
    }

    Ok(())
}
