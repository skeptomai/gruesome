use std::fs::File;
use std::io::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut f = File::open("resources/test/zork1/DATA/ZORK1.DAT")?;
    let mut game_data = Vec::new();
    f.read_to_end(&mut game_data)?;

    println!("Analyzing DESCRIBE-ROOM routine flow at 0x8c9a\n");
    
    println!("From the earlier debugging, we know:");
    println!("1. DESCRIBE-ROOM successfully prints \"West of House\"");
    println!("2. At 0x8d32: get_prop V10, #0011 -> V00");
    println!("3. At 0x8d36: call V00, #0004 -> V00");
    println!("4. V00 contains 0, so the call returns immediately");
    println!("5. Execution continues...\n");
    
    println!("The call chain that leads to the error:");
    println!("├─> 0x8d67: call #4755, V10, V01, #ffff -> V00");
    println!("│   (This calls routine at 0x08eaa)");
    println!("│");
    println!("└─> Eventually reaches 0x093bb: jg #0011, #003b [FALSE -7839]");
    println!("    │");
    println!("    └─> Branches to 0x0751f (in the middle of nowhere!)");
    println!("        │");
    println!("        └─> Executes garbage instructions");
    println!("            │");
    println!("            └─> Error at 0x07554\n");
    
    // Let's check what routine 0x4755 unpacks to
    let packed_4755 = 0x4755u32;
    let unpacked_4755 = packed_4755 * 2;
    println!("Routine 0x4755 unpacks to: 0x{:05x}", unpacked_4755);
    
    // Check what's at the jg instruction location
    println!("\nThe problematic JG instruction at 0x093bb:");
    if let Ok(inst) = infocom::instruction::Instruction::decode(&game_data, 0x093bb, 3) {
        println!("  {}", inst.format_with_version(3));
        println!("  Compares: 0x11 (17) > 0x3b (59)");
        println!("  Result: FALSE (17 is not > 59)");
        println!("  Takes branch to: 0x0751f");
    }
    
    println!("\nThe issue:");
    println!("- The JG instruction branches to 0x0751f");
    println!("- But 0x0751f appears to be in a data section, not valid code");
    println!("- This causes us to execute garbage, leading to the error");
    
    println!("\nPossible root causes:");
    println!("1. The JG instruction has the wrong branch offset");
    println!("2. We're not supposed to reach the JG instruction at all");
    println!("3. The values being compared (17 > 59) are wrong");
    println!("4. There's a bug in our branch offset calculation");
    
    // Let's manually verify the branch offset
    println!("\nManually checking the JG branch offset:");
    let jg_addr = 0x093bb;
    let jg_size = 5; // JG with branch is 5 bytes
    let pc_after = jg_addr + jg_size; // 0x093c0
    
    // The branch offset is -7839 (0xe161 when viewed as u16)
    let offset = -7839i16;
    let target = (pc_after as i32 + offset as i32 - 2) as u32;
    
    println!("  PC after JG: 0x{:05x}", pc_after);
    println!("  Branch offset: {} (0x{:04x})", offset, offset as u16);
    println!("  Calculation: 0x{:05x} + {} - 2 = 0x{:05x}", pc_after, offset, target);
    
    if target == 0x0751f {
        println!("  ✓ Branch calculation is correct");
        println!("  The issue is that 0x0751f is not a valid code location!");
    }
    
    Ok(())
}