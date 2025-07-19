use env_logger;
use gruesome::interpreter::Interpreter;
use gruesome::vm::{Game, VM};
use log::{debug, info};
use std::io::Read;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logger to see debug output
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let game_path = "resources/test/zork1/DATA/ZORK1.DAT";

    let mut f = std::fs::File::open(game_path)?;
    let mut memory = Vec::new();
    f.read_to_end(&mut memory)?;

    let game = Game::from_memory(memory)?;
    let mut vm = VM::new(game);
    let mut interpreter = Interpreter::new(vm);

    println!("=== Testing dec_chk instruction ===\n");

    // Test 1: Variable form dec_chk with variable operand
    println!("Test 1: Variable form dec_chk");

    // Set up a local variable
    interpreter.vm.call_stack.push(gruesome::vm::CallFrame {
        return_pc: 0x9999,
        locals: [6, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        return_store: None,
        num_locals: 2,
        stack_base: 0,
    });

    // Create a Variable form dec_chk instruction at PC 0x5fdf
    // This is what we see in WORD-PRINT
    interpreter.vm.pc = 0x5fdf;

    // Decode instruction at 0x5fdf
    if let Ok(inst) = gruesome::instruction::decode_instruction(&interpreter.vm.game.memory, 0x5fdf)
    {
        println!("Instruction at 0x5fdf:");
        println!("  Form: {:?}", inst.form);
        println!("  Opcode: 0x{:02x}", inst.opcode);
        println!("  Operand count: {:?}", inst.operand_count);
        println!("  Operand types: {:?}", inst.operand_types);
        println!("  Operands: {:?}", inst.operands);

        // Show local variables before
        println!(
            "\nLocals before: L01={}, L02={}",
            interpreter.vm.call_stack.last().unwrap().locals[0],
            interpreter.vm.call_stack.last().unwrap().locals[1]
        );

        // Execute the instruction
        match interpreter.execute() {
            Ok(_) => {
                println!(
                    "\nLocals after: L01={}, L02={}",
                    interpreter.vm.call_stack.last().unwrap().locals[0],
                    interpreter.vm.call_stack.last().unwrap().locals[1]
                );
            }
            Err(e) => println!("Error: {}", e),
        }
    }

    Ok(())
}
