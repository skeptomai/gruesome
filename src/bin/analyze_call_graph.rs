use std::fs::File;
use std::io::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Z-Machine Zork I - Main Routine Call Graph\n");
    println!("=========================================\n");
    
    println!("Main routine at 0x4f04:");
    println!("│");
    println!("├─> CALL 0x5472 (#8010, #ffff) - Initialization call 1");
    println!("│");
    println!("├─> CALL 0x5472 (#807c, #ffff) - Initialization call 2");
    println!("│");
    println!("├─> CALL 0x5472 (#80f0, #ffff) - Initialization call 3");
    println!("│");
    println!("├─> CALL 0x5472 (#6f6a, #28)   - Setup call 1");
    println!("│");
    println!("├─> CALL 0x5472 (#6f55, #c8)   - Setup call 2");
    println!("│");
    println!("├─> [Various initialization: PUT_PROP, ADD, STOREW operations]");
    println!("│");
    println!("├─> CALL 0x9530 (#a0)          - Room/object initialization");
    println!("│");
    println!("├─> CALL 0x6ee0                - Conditional call (if object doesn't have attr 3)");
    println!("│");
    println!("└─> Main game loop at 0x4f94:");
    println!("    │");
    println!("    ├─> CALL 0x7e04            - LOOK command (from 0x3f02 packed)");
    println!("    │   │");
    println!("    │   └─> This eventually calls DESCRIBE-ROOM at 0x8c9a");
    println!("    │       │");
    println!("    │       ├─> Prints room name (e.g., \"West of House\")");
    println!("    │       │");
    println!("    │       └─> Should call room description routine");
    println!("    │           (but we hit an error before reaching WEST-HOUSE at 0x953c)");
    println!("    │");
    println!("    ├─> CALL 0x552a            - Main command loop/parser");
    println!("    │");
    println!("    └─> JUMP back to 0x4f05    - Infinite game loop");
    
    println!("\n\nKey observations:");
    println!("1. The main routine sets up the game state");
    println!("2. It calls 0x7e04 (LOOK) to display the initial room");
    println!("3. Then enters the main command loop at 0x552a");
    println!("4. The error occurs during the LOOK command execution");
    
    println!("\nThe problem path:");
    println!("Main → 0x7e04 (LOOK) → ... → 0x8c9a (DESCRIBE-ROOM) → prints \"West of House\"");
    println!("→ tries to get property and call room routine → error at 0x07554");
    
    println!("\nThe error prevents us from:");
    println!("- Reaching WEST-HOUSE routine at 0x953c");
    println!("- Seeing the full room description");
    println!("- Entering the main game loop properly");
    
    Ok(())
}