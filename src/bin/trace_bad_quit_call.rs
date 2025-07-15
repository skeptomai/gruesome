use infocom::interpreter::{Interpreter, ExecutionResult};
use infocom::instruction::{Instruction, OperandCount};
use infocom::vm::{Game, VM};
use infocom::text;
use std::fs::File;
use std::io::prelude::*;
use std::collections::VecDeque;

/// Custom interpreter that traces execution to find the bad call to 0x486e
struct TracingInterpreter {
    interpreter: Interpreter,
    inputs: VecDeque<String>,
    tracing_enabled: bool,
    saw_quit_prompt: bool,
    saw_y_input: bool,
    call_depth: usize,
}

impl TracingInterpreter {
    fn new(interpreter: Interpreter, inputs: Vec<String>) -> Self {
        TracingInterpreter {
            interpreter,
            inputs: inputs.into(),
            tracing_enabled: false,
            saw_quit_prompt: false,
            saw_y_input: false,
            call_depth: 0,
        }
    }

    fn execute_with_tracing(&mut self) -> Result<ExecutionResult, String> {
        let pc = self.interpreter.vm.pc;
        
        // Decode current instruction
        let inst = Instruction::decode(
            &self.interpreter.vm.game.memory, 
            pc as usize, 
            self.interpreter.vm.game.header.version
        )?;
        
        // Check for calls from the specific addresses we found
        if pc == 0x06dfc || pc == 0x06e36 {
            println!("\n*** AT SUSPECTED BAD CALL LOCATION! ***");
            println!("PC: {:05x}", pc);
            println!("Instruction: {}", inst.format_with_version(self.interpreter.vm.game.header.version));
            
            // Print call stack
            println!("\nCall stack:");
            for (i, frame) in self.interpreter.vm.call_stack.iter().enumerate() {
                println!("  Level {}: return to 0x{:05x}, {} locals", 
                        i, frame.return_pc, frame.num_locals);
            }
            
            self.tracing_enabled = true;
        }
        
        // Check if this is a call to a suspicious address
        if inst.opcode == 0x01 && inst.operand_count == OperandCount::VAR {
            // call_vs instruction
            if let Some(&packed_addr) = inst.operands.get(0) {
                let unpacked = (packed_addr as u32) * 2;
                
                // Check for the specific bad call
                if packed_addr == 0x486e {
                    println!("\n*** FOUND BAD CALL TO 0x486e! ***");
                    println!("PC: {:05x}", pc);
                    println!("Packed address: 0x{:04x}", packed_addr);
                    println!("Unpacked address: 0x{:05x}", unpacked);
                    println!("Call stack depth: {}", self.interpreter.vm.call_stack.len());
                    
                    // Print call stack
                    println!("\nCall stack:");
                    for (i, frame) in self.interpreter.vm.call_stack.iter().enumerate() {
                        println!("  Level {}: return to 0x{:05x}, {} locals", 
                                i, frame.return_pc, frame.num_locals);
                    }
                    
                    // Print surrounding code
                    println!("\nSurrounding code:");
                    for offset in -20..20 {
                        let addr = (pc as i32 + offset * 5) as u32;
                        if addr >= 0x4e38 && addr < self.interpreter.vm.game.memory.len() as u32 {
                            if let Ok(inst_at) = Instruction::decode(
                                &self.interpreter.vm.game.memory, 
                                addr as usize, 
                                self.interpreter.vm.game.header.version
                            ) {
                                println!("  {:05x}: {}", addr, 
                                    inst_at.format_with_version(self.interpreter.vm.game.header.version));
                            }
                        }
                    }
                    
                    self.tracing_enabled = true;
                }
                
                // Also check if unpacked address is below code start
                if unpacked < 0x4e38 && unpacked != 0 {
                    // Don't print for null calls which are common
                    if self.saw_y_input {
                        println!("\n*** WARNING: Call to address below code start! ***");
                        println!("PC: {:05x}", pc);
                        println!("Packed address: 0x{:04x}", packed_addr);
                        println!("Unpacked address: 0x{:05x} (code starts at 0x4e38)", unpacked);
                    }
                }
            }
        }
        
        // Trace if enabled or after Y input
        if self.tracing_enabled || self.saw_y_input {
            let indent = "  ".repeat(self.call_depth);
            println!("{}{:05x}: {}", indent, pc, inst.format_with_version(self.interpreter.vm.game.header.version));
            
            // Track call depth
            match inst.opcode {
                0x01 | 0x05 | 0x06 | 0x07 | 0x08 => {
                    // call instructions
                    self.call_depth += 1;
                }
                0x0B | 0x09 | 0x0A => {
                    // ret instructions  
                    if self.call_depth > 0 {
                        self.call_depth -= 1;
                    }
                }
                _ => {}
            }
        }
        
        // Handle special opcodes
        match inst.opcode {
            // print (0OP:02)
            0x02 if inst.operand_count == OperandCount::OP0 => {
                // Capture printed text
                let text = self.extract_print_text(&inst, pc)?;
                self.check_text(&text);
            }
            // print_ret (0OP:03)
            0x03 if inst.operand_count == OperandCount::OP0 => {
                // Capture printed text
                let text = self.extract_print_text(&inst, pc)?;
                let text_with_newline = format!("{}\n", text);
                self.check_text(&text_with_newline);
            }
            // sread (VAR in V1-4)
            0x04 if inst.operand_count == OperandCount::VAR => {
                // Intercept sread and provide our input
                let text_buffer = inst.operands[0] as u32;
                let parse_buffer = inst.operands[1] as u32;
                
                // The operands are variable numbers, not addresses!
                let text_buffer_addr = self.interpreter.vm.read_variable(text_buffer as u8)?;
                let parse_buffer_addr = self.interpreter.vm.read_variable(parse_buffer as u8)?;
                
                if let Some(input) = self.inputs.pop_front() {
                    println!("\n[PROVIDING INPUT: '{}']", input);
                    self.provide_input(text_buffer_addr as u32, parse_buffer_addr as u32, &input)?;
                    
                    // Start tracing after "y" input
                    if input == "y" && self.saw_quit_prompt {
                        println!("[Starting trace after 'y' confirmation]");
                        self.saw_y_input = true;
                    }
                } else {
                    return Err("No more predefined inputs available".to_string());
                }
                
                // Skip the actual sread execution
                self.interpreter.vm.pc += inst.size as u32;
                return Ok(ExecutionResult::Continue);
            }
            _ => {}
        }
        
        // Execute the instruction normally
        self.interpreter.vm.pc += inst.size as u32;
        self.interpreter.execute_instruction(&inst)
    }

    fn check_text(&mut self, text: &str) {
        if text.contains("Do you wish to leave the game?") {
            println!("\n*** QUIT PROMPT DETECTED ***");
            self.saw_quit_prompt = true;
        }
    }

    fn extract_print_text(&self, inst: &Instruction, pc: u32) -> Result<String, String> {
        let abbrev_addr = self.interpreter.vm.game.header.abbrev_table as usize;
        
        match inst.opcode {
            0x02 | 0x03 => {
                // print/print_ret - text follows instruction
                let text_start = pc + inst.size as u32;
                text::decode_string(&self.interpreter.vm.game.memory, text_start as usize, abbrev_addr)
                    .map(|(s, _)| s)
                    .map_err(|e| format!("Failed to decode string: {}", e))
            }
            _ => Ok(String::new())
        }
    }

    fn provide_input(&mut self, text_buffer: u32, parse_buffer: u32, input: &str) -> Result<(), String> {
        // Get max length from text buffer
        let max_len = self.interpreter.vm.read_byte(text_buffer);
        
        // Convert to lowercase as Z-Machine expects
        let input_lower = input.to_lowercase();
        let input_bytes = input_lower.as_bytes();
        let input_len = input_bytes.len().min(max_len as usize - 1);
        
        // Write input to text buffer
        self.interpreter.vm.write_byte(text_buffer + 1, input_len as u8)?;
        for (i, &ch) in input_bytes.iter().take(input_len).enumerate() {
            self.interpreter.vm.write_byte(text_buffer + 2 + i as u32, ch)?;
        }
        
        // Parse the text buffer
        self.interpreter.vm.parse_text(text_buffer, parse_buffer)?;
        
        Ok(())
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Bad Call Tracer for Quit Sequence ===");
    println!("Looking for call to 0x486e during quit confirmation\n");

    // Load Zork 1
    let mut f = File::open("resources/test/zork1/DATA/ZORK1.DAT")?;
    let mut game_data = Vec::new();
    f.read_to_end(&mut game_data)?;

    let game = Game::from_memory(game_data)?;
    
    // Print some debug info
    println!("Game version: {}", game.header.version);
    println!("Code start: 0x{:04x}", game.header.initial_pc);
    println!("Packed address 0x486e unpacks to: 0x{:05x}", 0x486e * 2);
    println!();
    
    let vm = VM::new(game);
    let interpreter = Interpreter::new(vm);
    
    // Create tracing interpreter with predefined inputs
    let mut tracer = TracingInterpreter::new(interpreter, vec![
        "quit".to_string(),
        "y".to_string(),
        "".to_string(),  // Extra empty inputs in case the game continues
        "".to_string(),
        "".to_string(),
    ]);
    
    println!("Running game with inputs: quit, y");
    println!("Waiting for bad call...\n");
    
    let mut instruction_count = 0;
    let max_instructions = 500000;
    
    // Run the game
    loop {
        instruction_count += 1;
        
        if instruction_count % 10000 == 0 {
            println!("[Progress] {} instructions executed...", instruction_count);
        }
        
        if instruction_count > max_instructions {
            println!("\nReached instruction limit ({} instructions)", max_instructions);
            break;
        }
        
        match tracer.execute_with_tracing() {
            Ok(ExecutionResult::Quit) => {
                println!("\n*** GAME QUIT SUCCESSFULLY! ***");
                break;
            }
            Ok(ExecutionResult::Continue) => {
                // Continue execution
            }
            Ok(_) => {
                // Other execution results - continue
            }
            Err(e) => {
                eprintln!("\nExecution error: {}", e);
                eprintln!("PC: {:05x}", tracer.interpreter.vm.pc);
                eprintln!("Instructions executed: {}", instruction_count);
                
                // If we hit an error after seeing the bad call, show more info
                if tracer.tracing_enabled {
                    println!("\nError occurred after bad call was detected!");
                }
                break;
            }
        }
    }
    
    Ok(())
}