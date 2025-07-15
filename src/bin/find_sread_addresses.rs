use infocom::vm::{Game, VM};
use infocom::interpreter::{Interpreter, ExecutionResult};
use infocom::instruction::Instruction;
use log::info;

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
    
    // Run game and look for SREAD
    info!("Starting game, looking for SREAD instructions...");
    info!("Initial PC: {:05x}", interpreter.vm.pc);
    info!("Version: {}", interpreter.vm.game.header.version);
    let max_steps = 1000000;
    let mut sread_count = 0;
    
    for step in 0..max_steps {
        let pc = interpreter.vm.pc;
        
        // Progress indicator
        if step % 10000 == 0 && step > 0 {
            info!("Step {}, PC={:05x}", step, pc);
        }
        
        // Decode instruction
        let inst = match Instruction::decode(&interpreter.vm.game.memory, pc as usize, interpreter.vm.game.header.version) {
            Ok(i) => i,
            Err(e) => {
                info!("Failed to decode instruction at PC {:05x}: {}", pc, e);
                break;
            }
        };
        
        // Debug PC 0x5491
        if pc == 0x5491 && step % 10000 == 0 {
            info!("At PC 0x5491, instruction: {:?}", inst);
        }
        
        // Check if this is SREAD (VAR form, opcode 0x04)
        if inst.form == infocom::instruction::InstructionForm::Variable && inst.opcode == 0x04 {
            sread_count += 1;
            info!("=== SREAD #{} at PC {:05x} after {} steps ===", sread_count, pc, step);
            
            // Show operands
            if inst.operands.len() >= 2 {
                let text_addr = inst.operands[0];
                let parse_addr = inst.operands[1];
                info!("Text buffer address: {:04x}", text_addr);
                info!("Parse buffer address: {:04x}", parse_addr);
                
                // Check if these are in dynamic memory (below globals)
                let globals_start = interpreter.vm.game.header.global_variables;
                info!("Globals start at: {:04x}", globals_start);
                
                if text_addr as usize >= globals_start {
                    info!("WARNING: Text buffer {:04x} is in static/high memory!", text_addr);
                }
                if parse_addr as usize >= globals_start {
                    info!("WARNING: Parse buffer {:04x} is in static/high memory!", parse_addr);
                }
                
                // Show some context around the buffers
                info!("Text buffer area (first 16 bytes):");
                for i in 0..16 {
                    let addr = (text_addr as usize) + i;
                    if addr < interpreter.vm.game.memory.len() {
                        let byte = interpreter.vm.game.memory[addr];
                        if i == 0 {
                            info!("  [{:04x}] = {:02x} (max length)", addr, byte);
                        } else {
                            info!("  [{:04x}] = {:02x}", addr, byte);
                        }
                    }
                }
            }
            
            // Skip the SREAD to continue execution
            interpreter.vm.pc += inst.size as u32;
            continue;
        }
        
        // Execute instruction
        match interpreter.execute_instruction(&inst)? {
            ExecutionResult::Continue | ExecutionResult::Branched | ExecutionResult::Called => {},
            ExecutionResult::Quit | ExecutionResult::GameOver => {
                info!("Game quit at PC {:05x} after {} steps", pc, step);
                break;
            }
            ExecutionResult::Returned(_) => {},
            ExecutionResult::Error(e) => {
                info!("Error at PC {:05x}: {}", pc, e);
                break;
            }
        }
        
        // Stop after finding a few SREADs
        if sread_count >= 3 {
            break;
        }
    }
    
    info!("Found {} SREAD instructions", sread_count);
    
    Ok(())
}