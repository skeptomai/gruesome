use infocom::vm::{Game, VM};
use infocom::interpreter::Interpreter;
use infocom::disassembler::Disassembler;
use std::env;
use std::fs::File;
use std::io::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::init();

    // Get command line arguments
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <game_file.dat>", args[0]);
        eprintln!("Example: {} resources/test/zork1/DATA/ZORK1.DAT", args[0]);
        std::process::exit(1);
    }

    let game_path = &args[1];

    // Load the game file
    println!("Loading Z-Machine game: {}", game_path);
    let mut file = File::open(game_path)?;
    let mut game_data = Vec::new();
    file.read_to_end(&mut game_data)?;

    // Create the game and VM
    let game = Game::from_memory(game_data)?;
    let mut vm = VM::new(game);
    
    // Set up a simple interpreter just to get to the point where lighting is checked
    let mut interpreter = Interpreter::new(vm);
    
    println!("=== Tracing Lighting Check Failure ===\n");
    
    // First, let's look at the code that's called when lighting check fails
    // From the assembly trace, we see that when the lighting check fails,
    // it jumps to address 0x3770
    
    println!("Looking for the routine that handles lighting failure...");
    println!("When je g52, #0001 fails (g52 != 1), it jumps to address 0x3770");
    
    // Let's examine memory at that address
    let disasm = Disassembler::new(&interpreter.vm.game);
    
    // The jump address appears to be 0x3770, but let's verify by looking at the actual
    // branch offset in the instruction at 0x5f70
    println!("\nExamining the je instruction at 0x5f70:");
    match disasm.disassemble_range(0x5f70, 0x5f78) {
        Ok(output) => println!("{}", output),
        Err(e) => println!("Error: {}", e),
    }
    
    // Now let's look at what's at 0x3770
    // But first, let me check if this is actually a routine address or if we need to
    // calculate the actual target
    let pc = 0x5f70;
    let instr_bytes = &interpreter.vm.game.memory[pc..pc+10];
    println!("\nRaw bytes at 0x5f70: {:02x?}", instr_bytes);
    
    // The je instruction is at 0x5f70: 01 52 01 00 06
    // Format: je variable, value [branch]
    // The branch offset appears to be 0x0006 (6 bytes forward if false)
    // So if the test fails, PC would be at 0x5f70 + 5 (instruction size) + 6 = 0x5f7b
    
    let fail_addr = 0x5f7b;
    println!("\nWhen lighting check fails, execution continues at 0x{:04x}", fail_addr);
    println!("\nDisassembling from 0x{:04x}:", fail_addr);
    match disasm.disassemble_range(fail_addr, fail_addr + 100) {
        Ok(output) => println!("{}", output),
        Err(e) => println!("Error: {}", e),
    }
    
    // Let's also check what global 0x52 (82) contains
    println!("\n=== Checking Global Variables ===");
    match interpreter.vm.read_global(0x52) {
        Ok(val) => println!("Global 0x52 (LIT variable) = {}", val),
        Err(e) => println!("Error reading global 0x52: {}", e),
    }
    
    // Let's trace through more of the lighting check to find where it prints the message
    println!("\n=== Looking for print statements ===");
    
    // Search for print opcodes after the lighting check fails
    // Common print opcodes in Z-machine:
    // 0x02 = print (literal string follows)
    // 0x03 = print_ret (print and return)
    // 0xb2 = print_paddr (print from packed address)
    
    for addr in (fail_addr..fail_addr + 200).step_by(1) {
        let opcode = interpreter.vm.game.memory[addr as usize];
        if opcode == 0x02 || opcode == 0x03 || opcode == 0xb2 {
            println!("\nFound print opcode 0x{:02x} at address 0x{:04x}", opcode, addr);
            match disasm.disassemble_range(addr, (addr + 20)) {
                Ok(output) => println!("{}", output),
                Err(e) => println!("Error: {}", e),
            }
        }
    }

    Ok(())
}