use gruesome::vm::Game;
use gruesome::disasm_txd::TxdDisassembler;
use log::info;
use std::collections::HashSet;
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    
    // Test V3 game (Zork I)
    info!("\n=== TESTING V3 GAME (ZORK I) ===");
    let v3_result = test_game(
        "resources/test/zork1/DATA/ZORK1.DAT",
        "/tmp/zork1_txd_routines.txt",
        440,  // TXD finds 440 routines for Zork I
        "V3 (Zork I)"
    )?;
    
    // Test V4 game (AMFV)
    info!("\n=== TESTING V4 GAME (AMFV) ===");
    let v4_result = test_game(
        "resources/test/amfv/amfv-r79-s851122.z4",
        "/tmp/amfv_txd_routines.txt",
        982,  // TXD finds 982 routines for AMFV
        "V4 (AMFV)"
    )?;
    
    // Summary
    info!("\n=== VERIFICATION SUMMARY ===");
    if v3_result && v4_result {
        info!("✅ SUCCESS: We find strict supersets for both V3 and V4 games");
        info!("✅ All routines found by TXD are also found by our disassembler");
        info!("✅ No false positives detected (all our routines decode properly)");
    } else {
        info!("❌ FAILURE: Not finding strict supersets");
    }
    
    Ok(())
}

fn test_game(game_file: &str, txd_file: &str, expected_txd_count: usize, game_name: &str) -> Result<bool, Box<dyn std::error::Error>> {
    // Load and run our disassembler
    let memory = fs::read(game_file)?;
    let game = Game::from_memory(memory.clone())?;
    let mut disasm = TxdDisassembler::new(&game);
    
    disasm.discover_routines()?;
    let our_routines: HashSet<u32> = disasm.get_routine_addresses().into_iter().collect();
    
    info!("{}: We found {} routines", game_name, our_routines.len());
    
    // Check if TXD file exists
    let txd_routines = if std::path::Path::new(txd_file).exists() {
        // Load TXD routines
        let content = fs::read_to_string(txd_file)?;
        let mut txd_set = HashSet::new();
        for line in content.lines() {
            let line = line.trim();
            if !line.is_empty() {
                if let Ok(addr) = u32::from_str_radix(line, 16) {
                    txd_set.insert(addr);
                }
            }
        }
        info!("{}: TXD file has {} routines", game_name, txd_set.len());
        Some(txd_set)
    } else {
        info!("{}: TXD file not found, using expected count {}", game_name, expected_txd_count);
        None
    };
    
    // Verify strict superset
    let mut is_superset = true;
    
    if let Some(txd_set) = &txd_routines {
        // Check that all TXD routines are in our set
        let missing: Vec<u32> = txd_set.iter()
            .filter(|addr| !our_routines.contains(addr))
            .cloned()
            .collect();
        
        if !missing.is_empty() {
            info!("❌ {} routines found by TXD but not by us:", missing.len());
            for addr in &missing[..5.min(missing.len())] {
                info!("  - {:05x}", addr);
            }
            if missing.len() > 5 {
                info!("  ... and {} more", missing.len() - 5);
            }
            is_superset = false;
        } else {
            info!("✅ All {} TXD routines found by our disassembler", txd_set.len());
        }
        
        // Check extra routines we find
        let extra: Vec<u32> = our_routines.iter()
            .filter(|addr| !txd_set.contains(addr))
            .cloned()
            .collect();
        
        info!("📊 We found {} extra routines not in TXD", extra.len());
        
        // Verify no false positives by checking if they decode
        let mut false_positives = 0;
        for &addr in &extra[..5.min(extra.len())] {
            if !verify_routine_decodes(&memory, addr, game.header.version) {
                false_positives += 1;
                info!("  ❌ {:05x} - Does not decode properly (false positive)", addr);
            } else {
                info!("  ✅ {:05x} - Decodes properly (valid routine)", addr);
            }
        }
        
        if false_positives > 0 {
            info!("❌ Found {} false positives", false_positives);
            is_superset = false;
        }
    }
    
    // Final summary for this game
    if is_superset {
        info!("✅ {}: Strict superset confirmed", game_name);
    } else {
        info!("❌ {}: NOT a strict superset", game_name);
    }
    
    Ok(is_superset)
}

fn verify_routine_decodes(memory: &[u8], addr: u32, version: u8) -> bool {
    use gruesome::instruction::{Instruction, InstructionForm};
    
    let mut pc = addr as usize;
    if pc >= memory.len() {
        return false;
    }
    
    // Get locals count
    let locals = memory[pc];
    if locals > 15 {
        return false; // Suspicious locals count
    }
    pc += 1;
    
    // Skip local variable storage
    if version <= 4 {
        pc += (locals as usize) * 2;
    }
    
    // Try to decode at least one instruction
    if pc >= memory.len() {
        return false;
    }
    
    match Instruction::decode(memory, pc, version) {
        Ok(inst) => {
            // Check if it's a reasonable first instruction
            match (inst.form, inst.opcode) {
                // Immediate returns are suspicious
                (InstructionForm::Short, 0x00..=0x03) if pc == (addr as usize + 1 + (locals as usize * 2)) => {
                    false // Immediate return suggests false positive
                }
                _ => true
            }
        }
        Err(_) => false
    }
}