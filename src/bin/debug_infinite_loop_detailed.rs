use infocom::debugger::Debugger;
use infocom::vm::{Game, VM};
use std::fs::File;
use std::io::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut f = File::open("resources/test/zork1/DATA/ZORK1.DAT")?;
    let mut game_data = Vec::new();
    f.read_to_end(&mut game_data)?;

    let game = Game::from_memory(game_data)?;
    let vm = VM::new(game);
    let mut debugger = Debugger::new(vm);

    println!("Debugging infinite loop to understand the variable values...\n");
    
    // Run until we hit the loop
    let mut count = 0;
    let mut in_loop = false;
    
    loop {
        count += 1;
        let pc = debugger.interpreter.vm.pc;
        
        // Check if we've reached the loop
        if pc == 0x05499 {
            if !in_loop {
                println!("*** ENTERED INFINITE LOOP AT 0x05499 ***");
                in_loop = true;
            }
            
            // Show variable values for the comparison
            if let Ok(v04) = debugger.interpreter.vm.read_variable(0x04) {
                if let Ok(v03) = debugger.interpreter.vm.read_variable(0x03) {
                    println!("[{:03}] je V04({}), V03({}) - Equal? {}", count, v04, v03, v04 == v03);
                    
                    if count == 33 || count == 43 { // Show details on specific iterations
                        // Show more context every 10 iterations
                        println!("Loop iteration {}: V04={}, V03={}", count - 23, v04, v03);
                        
                        // Check other variables
                        if let Ok(v92) = debugger.interpreter.vm.read_variable(0x92) {
                            if let Ok(v94) = debugger.interpreter.vm.read_variable(0x94) {
                                println!("  V92={} (0x{:04x}), V94={} (0x{:04x})", v92, v92, v94, v94);
                                
                                // Check if V92 is being treated as signed
                                let v92_signed = v92 as i16;
                                println!("  V92 as signed: {}", v92_signed);
                                
                                // The calculation should be V04 = V94 + V92
                                let expected_v04 = v94.wrapping_add(v92);
                                println!("  V94 + V92 = {} (expected V04, actual V04 is {})", expected_v04, v04);
                            }
                        }
                        
                        // The loop should increment V04 by 6 each time
                        // V03 was set to V94 + 0xb4
                        // So the loop should end when V04 >= V03
                        let iterations_left = if v03 > v04 { (v03 - v04) / 6 } else { 0 };
                        println!("  Estimated iterations left: {}", iterations_left);
                        
                        if iterations_left > 1000 {
                            println!("  *** LOOP WILL RUN TOO LONG - POSSIBLE BUG ***");
                            println!("  V04 starts at V94 + V92, increments by 6 each time");
                            println!("  V03 is V94 + 0xb4");
                            println!("  Loop should end when V04 >= V03");
                            
                            // Check if V92 is negative or V94 is wrong
                            if let Ok(v92) = debugger.interpreter.vm.read_variable(0x92) {
                                if v92 > 32767 { // Negative in 16-bit
                                    println!("  V92 is negative: {} (0x{:04x})", v92 as i16, v92);
                                }
                            }
                            break;
                        }
                    }
                }
            }
            
            if count > 50 {
                println!("Breaking after 50 loop iterations to avoid infinite execution");
                break;
            }
        }
        
        // Step one instruction
        match debugger.step() {
            Ok(_) => {
                // Continue
            }
            Err(e) => {
                println!("Error: {}", e);
                break;
            }
        }
        
        if count > 100 && !in_loop {
            println!("Never reached the loop in 100 instructions");
            break;
        }
    }
    
    Ok(())
}