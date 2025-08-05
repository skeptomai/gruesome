use gruesome::vm::Game;
use gruesome::disasm_txd::TxdDisassembler;
use gruesome::instruction::Instruction;
use log::info;
use std::collections::HashSet;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: {} <game_file> <txd_routines.txt>", args[0]);
        std::process::exit(1);
    }
    
    let game_file = &args[1];
    let txd_file = &args[2];
    
    // Load TXD routines
    let txd_content = std::fs::read_to_string(txd_file)?;
    let txd_routines: HashSet<u32> = txd_content
        .lines()
        .filter_map(|line| u32::from_str_radix(line.trim(), 16).ok())
        .collect();
    
    info!("Loaded {} routines from TXD", txd_routines.len());
    
    // Run our disassembler
    let memory = std::fs::read(game_file)?;
    let game = Game::from_memory(memory)?;
    let mut disasm = TxdDisassembler::new(&game);
    
    disasm.discover_routines()?;
    let our_routines: HashSet<u32> = disasm.get_routine_addresses().into_iter().collect();
    
    info!("We found {} routines", our_routines.len());
    
    // Find extras
    let mut extras: Vec<u32> = our_routines.difference(&txd_routines).cloned().collect();
    extras.sort();
    
    info!("\nExtra routines we found ({}):", extras.len());
    
    // Analyze each extra routine
    for &addr in &extras {
        info!("\nAnalyzing routine at {:04x}:", addr);
        
        // Check locals count
        let locals = game.memory[addr as usize];
        info!("  Locals count: {}", locals);
        
        // Try to decode first few instructions
        let mut pc = addr as usize + 1;
        if game.header.version <= 4 {
            pc += (locals as usize) * 2;
        }
        
        info!("  First instructions:");
        let mut valid_count = 0;
        let mut found_return = false;
        
        for i in 0..10 {
            if pc >= game.memory.len() {
                info!("    [End of memory]");
                break;
            }
            
            match Instruction::decode(&game.memory, pc, game.header.version) {
                Ok(inst) => {
                    info!("    {:04x}: {:?}", pc, inst);
                    valid_count += 1;
                    
                    // Check for return instructions
                    if matches!(inst.opcode, 0x00 | 0x01 | 0x02 | 0x03 | 0x08 | 0x09 | 0x0a | 0x0b) 
                        && matches!(inst.form, gruesome::instruction::InstructionForm::Short) {
                        found_return = true;
                        info!("    ^ RETURN instruction");
                        break;
                    }
                    
                    pc += inst.size;
                }
                Err(e) => {
                    info!("    {:04x}: DECODE ERROR: {}", pc, e);
                    break;
                }
            }
        }
        
        info!("  Decoded {} valid instructions, return found: {}", valid_count, found_return);
        
        // Check if any TXD routine calls this
        let mut called_by_txd = false;
        for &txd_addr in &txd_routines {
            // This is a simplified check - would need full call graph analysis
            // Just checking if the address appears in the routine
            let check_start = txd_addr as usize + 1;
            let check_end = (txd_addr + 1000).min(game.memory.len() as u32) as usize;
            
            for i in check_start..check_end {
                if i + 2 < game.memory.len() {
                    let word = ((game.memory[i] as u16) << 8) | (game.memory[i + 1] as u16);
                    if word == addr as u16 {
                        called_by_txd = true;
                        break;
                    }
                }
            }
            if called_by_txd {
                break;
            }
        }
        
        info!("  Potentially called by TXD routines: {}", called_by_txd);
    }
    
    // Also check what TXD found that we didn't
    let missing: Vec<u32> = txd_routines.difference(&our_routines).cloned().collect();
    if !missing.is_empty() {
        info!("\n\nRoutines TXD found that we missed ({}):", missing.len());
        for addr in missing {
            info!("  {:04x}", addr);
        }
    }
    
    Ok(())
}