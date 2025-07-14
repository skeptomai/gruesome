use infocom::vm::{Game, VM};
use infocom::interpreter::Interpreter;
use std::fs::File;
use std::io::prelude::*;

fn decode_zstring(memory: &[u8], addr: usize) -> String {
    let mut result = String::new();
    let mut addr = addr;
    
    // Z-string decoding for V3
    let alphabet_table = [
        " ******abcdefghijklmnopqrstuvwxyz",  // A0
        " ******ABCDEFGHIJKLMNOPQRSTUVWXYZ",  // A1
        " ******\n0123456789.,!?_#'\"/\\-:()", // A2
    ];
    
    let mut current_alphabet = 0;
    let mut abbrev_mode = 0;
    
    loop {
        if addr + 1 >= memory.len() {
            break;
        }
        
        let word = ((memory[addr] as u16) << 8) | (memory[addr + 1] as u16);
        addr += 2;
        
        // Extract three 5-bit characters
        let chars = [
            ((word >> 10) & 0x1F) as u8,
            ((word >> 5) & 0x1F) as u8,
            (word & 0x1F) as u8,
        ];
        
        for &ch in &chars {
            if abbrev_mode > 0 {
                // Handle abbreviation (simplified - just skip it)
                abbrev_mode = 0;
                continue;
            }
            
            match ch {
                0 => result.push(' '),
                1..=3 => abbrev_mode = ch,
                4 => current_alphabet = 1,
                5 => current_alphabet = 2,
                6..=31 => {
                    let alphabet = &alphabet_table[current_alphabet];
                    if ch as usize <= alphabet.len() {
                        result.push(alphabet.chars().nth(ch as usize).unwrap_or('?'));
                    }
                    if current_alphabet != 0 {
                        current_alphabet = 0; // Reset to A0
                    }
                }
                _ => {}
            }
        }
        
        // Check if this is the last word (bit 15 set)
        if word & 0x8000 != 0 {
            break;
        }
    }
    
    result
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load the game file
    let mut file = File::open("resources/test/zork1/DATA/ZORK1.DAT")?;
    let mut game_data = Vec::new();
    file.read_to_end(&mut game_data)?;

    let game = Game::from_memory(game_data)?;
    let interpreter = Interpreter::new(VM::new(game));
    
    println!("=== Decoding Strings from Print Instructions ===\n");
    
    // From our trace, we found several print_paddr instructions
    // Let's decode the strings at these addresses
    
    let print_addresses = [
        (0x5ffb, "after lighting check fails"),
        (0x600a, "in darkness handling"),
        (0x6032, "further in darkness handling"),
        (0x604b, "another print"),
        (0x605a, "another print"),
        (0x6082, "another print"),
        (0x60be, "another print"),
        (0x614a, "another print"),
    ];
    
    for (addr, context) in &print_addresses {
        println!("Print at 0x{:04x} ({}):", addr, context);
        
        // The print instruction is followed by the encoded string
        let string_addr = addr + 1;
        
        // Show raw bytes
        let raw_bytes = &interpreter.vm.game.memory[string_addr..string_addr + 20];
        println!("  Raw bytes: {:02x?}", raw_bytes);
        
        // Try to decode
        let decoded = decode_zstring(&interpreter.vm.game.memory, string_addr);
        println!("  Decoded: \"{}\"", decoded);
        println!();
    }
    
    // Let's also check if any of these strings contain "bats" or "dark"
    println!("\n=== Looking for strings containing 'bats' or 'dark' ===\n");
    
    // Search more broadly for print instructions
    for addr in 0x5f00..0x6200 {
        let opcode = interpreter.vm.game.memory[addr];
        
        if opcode == 0xB2 {  // print (0OP)
            let string_addr = addr + 1;
            if string_addr + 10 < interpreter.vm.game.memory.len() {
                let decoded = decode_zstring(&interpreter.vm.game.memory, string_addr);
                if decoded.to_lowercase().contains("bat") || decoded.to_lowercase().contains("dark") {
                    println!("Found at 0x{:04x}: \"{}\"", addr, decoded);
                    
                    // Show context
                    println!("  Raw bytes: {:02x?}", &interpreter.vm.game.memory[string_addr..string_addr + 20]);
                }
            }
        }
    }
    
    // Let's also check what happens when we follow the jump at 0x5f98
    println!("\n=== Following the jump at 0x5f98 ===");
    
    // The jump at 0x5f98 has offset 0xff94
    let jump_offset = 0xff94u16 as i16;  // Convert to signed
    let jump_target = (0x5f98 + 3 + jump_offset as i32 - 2) as u32;
    println!("Jump target: 0x{:04x}", jump_target);
    
    // Actually, let's calculate it properly
    // Jump offset ff94 is -108 in signed 16-bit
    let proper_offset = -108i16;
    let proper_target = (0x5f98 as i32 + 3 + proper_offset as i32 - 2) as u32;
    println!("Proper jump target: 0x{:04x}", proper_target);

    Ok(())
}