use std::fs::File;
use std::io::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut f = File::open("resources/test/zork1/DATA/ZORK1.DAT")?;
    let mut game_data = Vec::new();
    f.read_to_end(&mut game_data)?;

    println!("Examining typical Z-Machine game flow...\n");
    
    println!("Normal Z-Machine game startup should be:");
    println!("1. Print header/copyright");
    println!("2. Initialize game state");
    println!("3. Print initial room description (usually triggered by LOOK)");
    println!("4. Enter main input loop with SREAD");
    
    println!("\nWhat we're seeing:");
    println!("1. ✓ Header/copyright printed");
    println!("2. ✓ Serial number printed");
    println!("3. ✗ Main dispatch calls 0x08c9a with Vd1=0");
    println!("4. ✗ This branches to STAND instead of room description");
    
    println!("\nLet's check if there should be an SREAD loop that we're missing...");
    
    // Look for the pattern: main dispatch -> SREAD loop -> action dispatch
    // The routine at 0x08c9a might be the action dispatch that expects input
    
    println!("\nAnalyzing routine 0x08c9a (action dispatch):");
    let addr = 0x08c9a;
    
    // Find the routine start
    let mut routine_start = addr;
    while routine_start > 0 && routine_start > addr - 50 {
        routine_start -= 1;
        
        if routine_start > 0 && game_data[routine_start - 1] >= 0xB0 && game_data[routine_start - 1] <= 0xBF {
            let locals = game_data[routine_start];
            if locals <= 15 {
                println!("  Routine starts at 0x{:05x} with {} locals", routine_start, locals);
                
                // Show the routine structure
                print!("  Bytes: ");
                for i in 0..30 {
                    if routine_start + i < game_data.len() {
                        print!("{:02x} ", game_data[routine_start + i]);
                    }
                }
                println!();
                
                // Check if this routine should normally be called after input
                let offset_to_comparison = addr - routine_start;
                println!("  Comparison 'jl #0005, Vd1' is at offset {}", offset_to_comparison);
                
                // This suggests the routine expects Vd1 to be set by previous input processing
                break;
            }
        }
    }
    
    println!("\nHypothesis: The main dispatch routine at 0x07e04 should:");
    println!("1. Set up initial LOOK action (store #6 to Vd1), OR");
    println!("2. Call an input routine that would set up the first action");
    println!("3. Then call the action dispatch at 0x08c9a");
    
    println!("\nBut instead it's calling 0x08c9a directly with uninitialized Vd1=0");
    
    // Let's check what the main dispatch routine actually does
    println!("\nDecoding main dispatch at 0x07e04:");
    let start = 0x07e04;
    
    // First instruction: 00 - this is the locals count
    println!("  0x{:05x}: 00 (0 locals)", start);
    
    // Next instruction: e0 1f 46 4d
    let inst_addr = start + 1;
    println!("  0x{:05x}: e0 1f 46 4d", inst_addr);
    
    // e0 is Variable form, check opcode
    let opcode = game_data[inst_addr] & 0x1F;
    println!("    Opcode: 0x{:02x}", opcode);
    
    // 0x1F is the undocumented opcode we've seen before
    if opcode == 0x1F {
        println!("    This is the undocumented opcode 0x1F again!");
        println!("    Operands: 0x{:02x} 0x{:02x}", game_data[inst_addr + 1], game_data[inst_addr + 2]);
    }
    
    // Next instruction should be the call to 0x08c9a
    let next_addr = inst_addr + 4;
    println!("  0x{:05x}: {:02x} {:02x} {:02x}", next_addr, 
             game_data[next_addr], game_data[next_addr + 1], game_data[next_addr + 2]);
    
    // This should be a call instruction
    let call_byte = game_data[next_addr];
    if call_byte == 0xE0 {
        println!("    This is a Variable form call");
        let packed_addr = ((game_data[next_addr + 1] as u16) << 8) | (game_data[next_addr + 2] as u16);
        let unpacked = packed_addr * 2;
        println!("    Calling routine at 0x{:05x}", unpacked);
        
        if unpacked == 0x08c9a {
            println!("    *** This confirms it calls the action dispatch directly! ***");
        }
    }
    
    Ok(())
}