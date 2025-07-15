use infocom::vm::{Game, VM};
use infocom::interpreter::{Interpreter, ExecutionResult};
use infocom::instruction::Instruction;
use log::{debug, info};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug")).init();
    
    let game_path = "resources/test/zork1/DATA/ZORK1.DAT";
    
    // Load game data
    let mut f = std::fs::File::open(game_path)?;
    let mut memory = Vec::new();
    std::io::Read::read_to_end(&mut f, &mut memory)?;
    
    let game = Game::from_memory(memory)?;
    let vm = VM::new(game);
    let mut interpreter = Interpreter::new(vm);
    
    // Run until we get to first SREAD
    info!("Starting game until first input...");
    let max_steps = 100000;
    
    for step in 0..max_steps {
        let pc = interpreter.vm.pc;
        
        // Progress indicator
        if step % 1000 == 0 && step > 0 {
            info!("Step {}, PC={:05x}", step, pc);
        }
        
        // Debug if stuck at 0x5491
        if pc == 0x5491 && step > 0 && step < 10 {
            info!("At PC 0x5491, decoding instruction...");
            let inst = Instruction::decode(&interpreter.vm.game.memory, pc as usize, interpreter.vm.game.header.version)?;
            info!("Instruction at 0x5491: {:?}", inst);
        }
        
        // Decode instruction to check if it's SREAD
        let inst_byte = interpreter.vm.game.memory[pc as usize];
        
        // Check if this is a VAR form instruction (bits 7-5 = 111)
        if inst_byte >= 0xE0 && pc < interpreter.vm.game.memory.len() as u32 - 1 {
            let opcode_byte = interpreter.vm.game.memory[(pc + 1) as usize];
            
            // SREAD is VAR opcode 246, which is 0x16 in the opcode byte
            if opcode_byte == 0x16 {
                info!("Found SREAD at PC {:05x} after {} steps (form={:02x})", pc, step, inst_byte);
                
                // Inject 'n' into text buffer
                let text_buffer = 0x5635;
                interpreter.vm.game.memory[text_buffer] = 50; // Max length
                interpreter.vm.game.memory[text_buffer + 1] = 1; // Length = 1
                interpreter.vm.game.memory[text_buffer + 2] = b'n';
                
                info!("Injected 'n' into text buffer at {:04x}", text_buffer);
                
                // Let SREAD complete
                let inst = Instruction::decode(&interpreter.vm.game.memory, pc as usize, interpreter.vm.game.header.version)?;
                interpreter.vm.pc += inst.size as u32;  // Advance PC before execution
                let _result = interpreter.execute_instruction(&inst)?;
                
                // Now trace what happens next
                info!("Continuing after SREAD, PC={:05x}", interpreter.vm.pc);
                
                for trace_step in 0..200 {
                    let trace_pc = interpreter.vm.pc;
                    
                    // Log key routines
                    match trace_pc {
                        0x5c40 => info!(">>> Entering PARSER routine at {:05x}", trace_pc),
                        0x5d78 => info!(">>> Entering READ routine at {:05x}", trace_pc), 
                        0x50a8 => info!(">>> Entering PERFORM routine at {:05x}", trace_pc),
                        0x6f76 => info!(">>> Entering V-WALK routine at {:05x}", trace_pc),
                        0x51f0 => info!(">>> Entering GOTO routine at {:05x}", trace_pc),
                        _ => {}
                    }
                    
                    // Decode and execute next instruction
                    let trace_inst = Instruction::decode(&interpreter.vm.game.memory, trace_pc as usize, interpreter.vm.game.header.version)?;
                    
                    // Log scan_table instructions (dictionary lookups)
                    if trace_inst.opcode == 0x13 {
                        let form = if trace_inst.operands.len() > 3 { trace_inst.operands[3] } else { 0x82 };
                        debug!("scan_table at PC {:05x}: word={:04x}, table={:04x}, len={}, form={}",
                            trace_pc, trace_inst.operands[0], trace_inst.operands[1], 
                            trace_inst.operands[2], form);
                    }
                    
                    // Log print instructions
                    if trace_inst.opcode == 0x02 || trace_inst.opcode == 0x03 {
                        debug!("Print instruction at PC {:05x}, opcode={:02x}", trace_pc, trace_inst.opcode);
                    }
                    
                    // Advance PC before execution
                    interpreter.vm.pc += trace_inst.size as u32;
                    
                    match interpreter.execute_instruction(&trace_inst)? {
                        ExecutionResult::Continue | ExecutionResult::Branched | ExecutionResult::Called => {},
                        ExecutionResult::Quit | ExecutionResult::GameOver => {
                            info!("Game quit");
                            break;
                        }
                        ExecutionResult::Returned(_) => {},
                        ExecutionResult::Error(e) => {
                            info!("Error: {}", e);
                            break;
                        }
                    }
                }
                
                break;
            }
        }
        
        // Execute normal instruction
        let inst = Instruction::decode(&interpreter.vm.game.memory, pc as usize, interpreter.vm.game.header.version)?;
        
        // Advance PC before execution (like the run() method does)
        interpreter.vm.pc += inst.size as u32;
        
        match interpreter.execute_instruction(&inst)? {
            ExecutionResult::Continue | ExecutionResult::Branched | ExecutionResult::Called => {},
            ExecutionResult::Quit | ExecutionResult::GameOver => {
                info!("Game quit before SREAD");
                break;
            }
            ExecutionResult::Returned(_) => {},
            ExecutionResult::Error(e) => {
                info!("Error before SREAD: {}", e);
                break;
            }
        }
    }
    
    Ok(())
}