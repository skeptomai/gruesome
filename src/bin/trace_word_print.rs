use gruesome::disassembler::Disassembler;
use gruesome::interpreter::Interpreter;
use gruesome::vm::{Game, VM};
use std::io::Read;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logger to see debug output
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug")).init();

    let game_path = "resources/test/zork1/DATA/ZORK1.DAT";

    let mut f = std::fs::File::open(game_path)?;
    let mut memory = Vec::new();
    f.read_to_end(&mut memory)?;

    let game = Game::from_memory(memory)?;
    let vm = VM::new(game);
    let mut interpreter = Interpreter::new(vm);

    // Enable debug mode for specific range around WORD-PRINT
    // interpreter.enable_single_step(0x5fda, 0x5ff0); // This method doesn't exist

    // Set up a call to WORD-PRINT with "leaves" (6 characters)
    // First set up the text buffer with "leaves" at a known location
    let text_addr = 0x1000; // arbitrary address in dynamic memory
    interpreter.vm.write_byte(text_addr, b'l')?;
    interpreter.vm.write_byte(text_addr + 1, b'e')?;
    interpreter.vm.write_byte(text_addr + 2, b'a')?;
    interpreter.vm.write_byte(text_addr + 3, b'v')?;
    interpreter.vm.write_byte(text_addr + 4, b'e')?;
    interpreter.vm.write_byte(text_addr + 5, b's')?;

    // Set global V7d (text buffer address)
    interpreter.vm.write_global(0x7d - 0x10, text_addr as u16)?;

    // Call WORD-PRINT with length=6, position=0
    println!("=== Calling WORD-PRINT with 'leaves' (length=6) ===\n");

    // Set up locals: L01=6 (length), L02=0 (position)
    interpreter.vm.call_stack.push(gruesome::vm::CallFrame {
        return_pc: 0x9999, // dummy return address
        return_store: None,
        num_locals: 2,
        locals: [6, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        stack_base: interpreter.vm.stack.len(),
    });
    interpreter.vm.pc = 0x5fda;

    // Run until routine returns
    let mut step_count = 0;
    loop {
        // Show current instruction
        let disasm = Disassembler::new(&interpreter.vm.game);
        if let Ok(inst_str) = disasm.disassemble_range(interpreter.vm.pc, interpreter.vm.pc + 10) {
            // Extract just the first line
            if let Some(first_line) = inst_str.lines().next() {
                println!("PC {:04x}: {}", interpreter.vm.pc, first_line.trim());
            }
        }

        // Show locals before execution
        if let Some(frame) = interpreter.vm.call_stack.last() {
            print!("  Locals before: ");
            for (i, &local) in frame.locals[0..frame.num_locals as usize]
                .iter()
                .enumerate()
            {
                print!("L{:02}={} ", i + 1, local);
            }
            println!();
        }

        // Execute one instruction
        // Decode and execute one instruction
        let inst = gruesome::instruction::Instruction::decode(
            &interpreter.vm.game.memory,
            interpreter.vm.pc as usize,
            interpreter.vm.game.header.version,
        )?;
        let new_pc = interpreter.vm.pc + inst.size as u32;
        interpreter.vm.pc = new_pc;

        match interpreter.execute_instruction(&inst) {
            Ok(result) => {
                // Show locals after execution
                if let Some(frame) = interpreter.vm.call_stack.last() {
                    print!("  Locals after:  ");
                    for (i, &local) in frame.locals[0..frame.num_locals as usize]
                        .iter()
                        .enumerate()
                    {
                        print!("L{:02}={} ", i + 1, local);
                    }
                    println!();
                }

                match result {
                    gruesome::interpreter::ExecutionResult::Returned(_) => {
                        println!("\nRoutine returned");
                        break;
                    }
                    gruesome::interpreter::ExecutionResult::Quit => {
                        println!("\nQuit");
                        break;
                    }
                    _ => {}
                }
            }
            Err(e) => {
                println!("Error: {e}");
                break;
            }
        }

        step_count += 1;
        if step_count > 50 {
            println!("\nStopping after {step_count} steps");
            break;
        }

        println!();
    }

    Ok(())
}
