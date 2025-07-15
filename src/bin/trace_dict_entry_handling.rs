use infocom::vm::{Game, VM};
use infocom::interpreter::{Interpreter, ExecutionResult};
use infocom::instruction::Instruction;
use log::{debug, info};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    
    let game_path = "resources/test/zork1/DATA/ZORK1.DAT";
    
    // Load game data
    let mut f = std::fs::File::open(game_path)?;
    let mut memory = Vec::new();
    std::io::Read::read_to_end(&mut f, &mut memory)?;
    
    let game = Game::from_memory(memory)?;
    let vm = VM::new(game);
    let mut interpreter = Interpreter::new(vm);
    
    println!("=== Dictionary Entry Handler Trace ===\n");
    
    // First, let's verify the dictionary entries
    let dict_base = interpreter.vm.game.header.dictionary as u32;
    let sep_count = interpreter.vm.read_byte(dict_base);
    let entry_start = dict_base + 1 + sep_count as u32;
    let entry_length = interpreter.vm.read_byte(entry_start);
    let entries_addr = entry_start + 3;
    
    println!("Dictionary info:");
    println!("  Base: {:04x}", dict_base);
    println!("  Entry length: {} bytes", entry_length);
    println!("  Entries start: {:04x}", entries_addr);
    
    // Calculate addresses for specific entries (0-based)
    let w_entry = 662;  // 663 - 1
    let west_entry = 672;  // 673 - 1
    let go_entry = 258;  // 259 - 1
    let e_entry = 245;  // Approximate, we'll search for it
    
    let w_addr = entries_addr + (w_entry * entry_length as u32);
    let west_addr = entries_addr + (west_entry * entry_length as u32);
    let go_addr = entries_addr + (go_entry * entry_length as u32);
    
    println!("\nDictionary entries:");
    println!("  'w' (entry 663): addr={:04x}", w_addr);
    print!("    Data: ");
    for i in 0..entry_length {
        print!("{:02x} ", interpreter.vm.read_byte(w_addr + i as u32));
    }
    println!();
    
    println!("  'west' (entry 673): addr={:04x}", west_addr);
    print!("    Data: ");
    for i in 0..entry_length {
        print!("{:02x} ", interpreter.vm.read_byte(west_addr + i as u32));
    }
    println!();
    
    println!("  'go' (entry 259): addr={:04x}", go_addr);
    print!("    Data: ");
    for i in 0..entry_length {
        print!("{:02x} ", interpreter.vm.read_byte(go_addr + i as u32));
    }
    println!();
    
    // Find 'e' entry
    let e_dict_addr = interpreter.vm.lookup_dictionary("e");
    println!("  'e': addr={:04x}", e_dict_addr);
    print!("    Data: ");
    for i in 0..entry_length {
        print!("{:02x} ", interpreter.vm.read_byte(e_dict_addr as u32 + i as u32));
    }
    println!();
    
    // Now set up to trace what happens with 'w'
    println!("\n=== Setting up trace for 'w' command ===");
    
    // Run game until we're ready for input
    let mut step_count = 0;
    let max_init_steps = 20000;
    let mut ready_for_input = false;
    
    while step_count < max_init_steps && !ready_for_input {
        let pc = interpreter.vm.pc;
        let inst = Instruction::decode(&interpreter.vm.game.memory, pc as usize, interpreter.vm.game.header.version)?;
        
        // Check if we're at a SREAD instruction
        if inst.opcode == 0xF6 {
            println!("\nFound SREAD at PC {:05x}", pc);
            ready_for_input = true;
            
            // Inject 'w' command
            let text_buffer = 0x01f5;
            let parse_buffer = 0x0225;
            
            interpreter.vm.write_byte(text_buffer + 1, 1)?;  // Length = 1
            interpreter.vm.write_byte(text_buffer + 2, b'w')?;
            
            // Execute the SREAD
            interpreter.execute_instruction(&inst)?;
            
            // Parse should have happened, check parse buffer
            let word_count = interpreter.vm.read_byte(parse_buffer + 1);
            println!("Parse buffer after 'w': {} words found", word_count);
            
            if word_count > 0 {
                let dict_addr = interpreter.vm.read_word(parse_buffer + 2);
                println!("  Dictionary address: {:04x} (should be {:04x})", dict_addr, w_addr);
            }
            
            break;
        }
        
        // Execute instruction
        match interpreter.execute_instruction(&inst)? {
            ExecutionResult::Continue | ExecutionResult::Branched | ExecutionResult::Called => {},
            ExecutionResult::Quit | ExecutionResult::GameOver => {
                println!("Game ended during init");
                return Ok(());
            }
            ExecutionResult::Returned(_) => {},
            ExecutionResult::Error(e) => {
                println!("Error during init: {}", e);
                return Ok(());
            }
        }
        
        step_count += 1;
    }
    
    if !ready_for_input {
        println!("Did not find SREAD in {} steps", step_count);
        return Ok(());
    }
    
    // Now trace through the command processing
    println!("\n=== Tracing command processing ===");
    
    let mut trace_steps = 0;
    let max_trace_steps = 1000;
    let mut found_dict_access = false;
    
    while trace_steps < max_trace_steps {
        let pc = interpreter.vm.pc;
        let inst = Instruction::decode(&interpreter.vm.game.memory, pc as usize, interpreter.vm.game.header.version)?;
        
        // Track when we access dictionary entry addresses
        for (i, operand) in inst.operands.iter().enumerate() {
            // Check if operand matches our dictionary addresses
            if *operand == w_addr as u16 {
                println!("\n>>> Dictionary address 'w' ({:04x}) used at PC {:05x}", operand, pc);
                println!("    Instruction: opcode={:02x}, operands={:?}", inst.opcode, inst.operands);
                found_dict_access = true;
            }
            
            // Check if we're reading from dictionary entry
            if *operand >= w_addr as u16 && *operand < w_addr as u16 + 7 {
                println!("  Reading from 'w' entry offset {}: addr={:04x}", 
                    *operand - w_addr as u16, operand);
            }
        }
        
        // Track routine calls
        match inst.opcode {
            0x0F | 0x1F | 0xE0 => { // call opcodes
                if inst.operands.len() > 0 {
                    let target = inst.operands[0];
                    match target {
                        0x50a8 => println!("\n>>> CALL to PERFORM at PC {:05x}", pc),
                        0x6f76 => println!("\n>>> CALL to V-WALK at PC {:05x}", pc),
                        0x51f0 => println!("\n>>> CALL to GOTO at PC {:05x}", pc),
                        _ => {}
                    }
                }
            }
            _ => {}
        }
        
        // Track loads from dictionary data bytes
        if inst.opcode == 0x10 { // loadb
            let addr = inst.operands[0];
            let offset = inst.operands[1];
            let result_addr = addr as u32 + offset as u32;
            
            // Check if we're loading from a dictionary entry
            if result_addr >= w_addr && result_addr < w_addr + 7 {
                println!("  LOADB from 'w' entry byte {}: addr={:04x}", 
                    result_addr - w_addr, result_addr);
                let value = interpreter.vm.read_byte(result_addr);
                println!("    Value loaded: {:02x}", value);
            }
        }
        
        // Track print instructions that might show garbage
        if inst.opcode == 0x02 || inst.opcode == 0x03 { // print, print_ret
            println!("\n!!! PRINT instruction at PC {:05x}", pc);
            
            // Try to see what text is being printed
            if inst.opcode == 0x02 { // print
                // Text follows immediately after opcode
                let text_addr = pc + inst.size as u32;
                println!("    Text address: {:05x}", text_addr);
            }
        }
        
        // Execute instruction
        match interpreter.execute_instruction(&inst)? {
            ExecutionResult::Continue | ExecutionResult::Branched | ExecutionResult::Called => {},
            ExecutionResult::Quit | ExecutionResult::GameOver => {
                println!("\nGame ended");
                break;
            }
            ExecutionResult::Returned(_) => {},
            ExecutionResult::Error(e) => {
                println!("\nError: {}", e);
                break;
            }
        }
        
        trace_steps += 1;
        
        // Stop if we've processed the command and returned to prompt
        if trace_steps > 100 && inst.opcode == 0xF6 { // Another SREAD
            println!("\nReached next input prompt");
            break;
        }
    }
    
    println!("\n=== Trace complete ===");
    println!("Total trace steps: {}", trace_steps);
    println!("Found dictionary access: {}", found_dict_access);
    
    Ok(())
}