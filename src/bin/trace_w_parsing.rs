use infocom::vm::{Game, VM};
use infocom::interpreter::{Interpreter, ExecutionResult};
use env_logger;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    
    let game_path = "resources/test/zork1/DATA/ZORK1.DAT";
    
    // Load game data
    let mut f = std::fs::File::open(game_path)?;
    let mut memory = Vec::new();
    std::io::Read::read_to_end(&mut f, &mut memory)?;
    
    let game = Game::from_memory(memory)?;
    let vm = VM::new(game);
    let mut interpreter = Interpreter::new(vm);
    
    // Enable debug mode
    interpreter.set_debug(true);
    
    println!("=== Tracing 'w' parsing through PARSER and PERFORM ===\n");
    
    // We know from previous debugging:
    // - Dictionary entry for 'w' is at 0x4d42 with type 0x32, data 0xa1 0x1d
    // - PARSER routine is at 0x5880
    // - PERFORM routine is at 0x577c
    
    // Set up to trace specific PC ranges
    let trace_ranges = vec![
        (0x5880, 0x5900, "PARSER"),
        (0x552a, 0x5580, "MAIN-LOOP"), 
        (0x577c, 0x5800, "PERFORM"),
    ];
    
    let mut instruction_count = 0;
    let max_instructions = 50000;
    
    // Run until we hit interesting code
    loop {
        let pc = interpreter.vm.pc;
        
        // Check if we're in a range we want to trace
        let mut in_trace_range = false;
        for (start, end, name) in &trace_ranges {
            if pc >= *start && pc < *end {
                in_trace_range = true;
                
                // Print current state
                println!("\n[{:05}] PC={:05x} (in {}+{:02x})", 
                    instruction_count, pc, name, pc - start);
                
                // Print global variables that might be relevant
                println!("  Globals:");
                println!("    G0 (HERE) = {:?}", interpreter.vm.read_global(0x10));
                println!("    G56 (PRSO) = {:?}", interpreter.vm.read_global(0x66));
                println!("    G57 (PRSI) = {:?}", interpreter.vm.read_global(0x67));
                println!("    G72 (ACT) = {:?}", interpreter.vm.read_global(0x82));
                println!("    G76 (P-WALK-DIR) = {:?}", interpreter.vm.read_global(0x86));
                println!("    G78 = {:?}", interpreter.vm.read_global(0x88));
                println!("    G6f (Actor) = {:?}", interpreter.vm.read_global(0x7f));
                
                // Print parse buffer contents
                let parse_buffer = 0x2551; // From sread debug output
                println!("  Parse buffer at {:04x}:", parse_buffer);
                let word_count = interpreter.vm.read_byte(parse_buffer);
                println!("    Word count: {}", word_count);
                
                if word_count > 0 {
                    let dict_addr = interpreter.vm.read_word(parse_buffer + 1);
                    let text_len = interpreter.vm.read_byte(parse_buffer + 3);
                    let text_pos = interpreter.vm.read_byte(parse_buffer + 4);
                    println!("    Word 1: dict_addr={:04x}, len={}, pos={}", 
                        dict_addr, text_len, text_pos);
                    
                    // Read dictionary entry details
                    if dict_addr != 0 {
                        let type_byte = interpreter.vm.read_byte((dict_addr + 4) as u32);
                        let data1 = interpreter.vm.read_byte((dict_addr + 5) as u32);
                        let data2 = interpreter.vm.read_byte((dict_addr + 6) as u32);
                        println!("    Dict entry: type={:02x}, data={:02x} {:02x}", 
                            type_byte, data1, data2);
                    }
                }
                break;
            }
        }
        
        if !in_trace_range && pc == 0x590c {
            // INPUT-LOOP - about to read input
            println!("\n[At INPUT-LOOP] About to read user input...");
        }
        
        // Execute one instruction
        match interpreter.execute_instruction() {
            Ok(ExecutionResult::Continue) | Ok(ExecutionResult::Branched) => {},
            Ok(ExecutionResult::Called) => {},
            Ok(ExecutionResult::Returned(_)) => {},
            Ok(ExecutionResult::Quit) | Ok(ExecutionResult::GameOver) => {
                println!("Game ended");
                break;
            },
            Err(e) => {
                eprintln!("Error: {}", e);
                break;
            }
        }
        
        instruction_count += 1;
        if instruction_count > max_instructions {
            println!("\nReached instruction limit");
            break;
        }
    }
    
    Ok(())
}