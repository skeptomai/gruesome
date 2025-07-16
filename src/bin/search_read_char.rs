use log::info;
use std::fs;
use std::path::Path;

fn main() {
    env_logger::init();
    
    // Load Zork I game file
    let game_path = Path::new("resources/test/zork1/DATA/ZORK1.DAT");
    let data = match fs::read(game_path) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Error loading game file: {:?}", e);
            return;
        }
    };
    
    // Check version
    if data.len() < 2 {
        eprintln!("File too small");
        return;
    }
    let version = data[0];
    
    info!("Loaded Zork I - version {}", version);
    info!("Searching for read_char opcode (VAR:0x16) usage...");
    info!("File size: {} bytes", data.len());
    
    // Brute force scan through the entire game file looking for the opcode pattern
    info!("\nScanning for VAR:0x16 opcode patterns...");
    let mut locations = Vec::new();
    
    // Look for variable form opcode 0x16 (read_char)
    // In variable form, the opcode byte would be:
    // - 0xF6 = 11110110 (Variable form, operand count in next byte, opcode 0x16)
    // - Could also appear as 2OP variable form: 0xD6 = 11010110
    
    for i in 0..data.len() {
        let byte = data[i];
        
        // Check for VAR:0x16 patterns
        if byte == 0xF6 {
            info!("Found potential VAR:0x16 at offset {:04x} (byte: {:02x})", i, byte);
            locations.push((i, "VAR form (0xF6)"));
        }
        // Also check if it might appear as a 2OP variable form
        else if byte == 0xD6 {
            // 2OP variable form with opcode 0x16 (which would be strange for read_char)
            info!("Found potential 2OP:0x16 at offset {:04x} (byte: {:02x})", i, byte);
            locations.push((i, "2OP variable form (0xD6)"));
        }
        // Also check for extended variable forms that might encode 0x16
        else if byte == 0xEC && i + 1 < data.len() && data[i + 1] == 0x16 {
            // Extended instruction with opcode 0x16
            info!("Found potential EXT:0x16 at offset {:04x}", i);
            locations.push((i, "Extended form"));
        }
    }
    
    // Also search for the string "read_char" in case it appears in debug info
    let search_str = b"read_char";
    for i in 0..data.len().saturating_sub(search_str.len()) {
        if &data[i..i + search_str.len()] == search_str {
            info!("Found string 'read_char' at offset {:04x}", i);
            locations.push((i, "String literal"));
        }
    }
    
    // Report results
    info!("\n=== SEARCH RESULTS ===");
    info!("Zork I version: {} (Z-Machine version)", version);
    info!("Expected: Version 3 (should NOT have read_char opcode)");
    info!("read_char is only available in Z-Machine v4 and later");
    
    if locations.is_empty() {
        info!("\nGOOD: No read_char (VAR:0x16) usage found in this v{} game!", version);
        info!("This is expected - read_char was introduced in Z-Machine v4.");
    } else {
        info!("\nFound {} potential read_char references:", locations.len());
        for (loc, desc) in &locations {
            info!("  - At offset {:04x}: {}", loc, desc);
            // Show surrounding bytes for context
            let start = if *loc >= 8 { loc - 8 } else { 0 };
            let end = (loc + 8).min(data.len());
            let context: Vec<String> = data[start..end].iter()
                .enumerate()
                .map(|(i, b)| {
                    if start + i == *loc {
                        format!("[{:02x}]", b)
                    } else {
                        format!("{:02x}", b)
                    }
                })
                .collect();
            info!("    Context: {}", context.join(" "));
        }
        
        if version <= 3 {
            info!("\nWARNING: Found potential read_char usage in v{} game!", version);
            info!("This opcode is only valid in v4+ games.");
        }
    }
    
    // Additional analysis: Check if common v4+ opcodes appear
    info!("\n=== Additional v4+ opcode check ===");
    let v4_opcodes = vec![
        (0xEC, "call_vs2"),
        (0xED, "erase_window"),
        (0xEE, "erase_line"),
        (0xEF, "set_cursor"),
        (0xF0, "get_cursor"),
        (0xF1, "set_text_style"),
        (0xF2, "buffer_mode"),
        (0xF6, "read_char"),
        (0xF7, "scan_table"),
    ];
    
    let mut v4_found = false;
    for (opcode, name) in &v4_opcodes {
        let count = data.iter().filter(|&&b| b == *opcode).count();
        if count > 0 {
            info!("  Found {} occurrences of byte {:02x} (potential {})", count, opcode, name);
            v4_found = true;
        }
    }
    
    if !v4_found {
        info!("  No v4+ specific opcode bytes found (good for v{} game)", version);
    }
}