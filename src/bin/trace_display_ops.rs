use gruesome::vm::Game;
use gruesome::vm::VM;
use gruesome::interpreter::Interpreter;
use gruesome::instruction::Instruction;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .init();
    
    let path = PathBuf::from("resources/test/amfv/amfv-r79-s851122.z4");
    
    // Load the game
    let mut f = File::open(path)?;
    let mut all_bytes = Vec::new();
    f.read_to_end(&mut all_bytes)?;
    
    let game = Game::from_memory(all_bytes)?;
    let vm = VM::new(game);
    let mut interpreter = Interpreter::new(vm);
    
    // Set a flag to start logging after we see certain operations
    let mut logging_active = false;
    let mut instruction_count = 0;
    
    println!("=== Tracing Display Operations in AMFV ===\n");
    println!("Running until we see display operations...\n");
    
    // Run the interpreter and log display-related operations
    loop {
        let pc = interpreter.vm.pc;
        
        // Decode instruction
        let inst = match Instruction::decode(&interpreter.vm.game.memory, pc as usize, 4) {
            Ok(inst) => inst,
            Err(e) => {
                eprintln!("Failed to decode at {:04x}: {}", pc, e);
                break;
            }
        };
        
        // Check if this is a display-related opcode
        let is_display_op = matches!(inst.opcode, 
            0xE5 | // print_char
            0xE6 | // print_num
            0x02 | // print
            0x03 | // print_ret
            0x0A | // print_obj
            0xEA | // split_window
            0xF0 | // set_text_style
            0xEB | // set_window
            0xF2 | // set_cursor
            0xED | // erase_window
            0xEE  // erase_line
        );
        
        if is_display_op {
            logging_active = true;
        }
        
        if logging_active && instruction_count < 1000 {
            if is_display_op {
                println!("[{:04}] {:04x}: {}", 
                    instruction_count, pc, inst.format_with_version(4));
            }
        }
        
        // Advance PC
        interpreter.vm.pc += inst.size as u32;
        
        // Execute
        match interpreter.execute_instruction(&inst) {
            Ok(result) => {
                use gruesome::interpreter::ExecutionResult;
                match result {
                    ExecutionResult::Quit => break,
                    ExecutionResult::Continue => {
                        // Check if we just executed a read instruction
                        if inst.opcode == 0x04 { // sread
                            println!("\n[Waiting for input - status line should be visible now]");
                            break;
                        }
                    }
                    _ => {}
                }
            }
            Err(e) => {
                eprintln!("Execution error: {}", e);
                break;
            }
        }
        
        instruction_count += 1;
        if instruction_count > 100000 {
            println!("\n[Stopping after 100k instructions]");
            break;
        }
    }
    
    Ok(())
}