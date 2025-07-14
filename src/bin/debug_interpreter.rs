use infocom::debugger::Debugger;
use infocom::vm::{Game, VM};
use std::env;
use std::fs::File;
use std::io::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <game_file.dat>", args[0]);
        eprintln!("Example: {} resources/test/zork1/DATA/ZORK1.DAT", args[0]);
        std::process::exit(1);
    }

    let filename = &args[1];
    println!("Loading Z-Machine game: {}", filename);

    // Read the game file
    let mut f = File::open(filename)?;
    let mut game_data = Vec::new();
    f.read_to_end(&mut game_data)?;

    // Create VM and debugger
    let game = Game::from_memory(game_data)?;
    let vm = VM::new(game);
    let mut debugger = Debugger::new(vm);

    println!("Z-Machine Debug Interpreter v0.1.0");
    println!("Game version: {}", debugger.interpreter.vm.game.header.version);
    println!("Initial PC: {:05x}", debugger.interpreter.vm.game.header.initial_pc);
    println!();
    println!("Commands: n(ext), c(ontinue), s(tate), h(istory), d(isasm), q(uit)");
    println!("         b <addr> (breakpoint), rb <addr> (remove), bl (list)");
    println!("Type 'n' to start single-stepping or 'c' to run normally");
    println!();

    // Start in single-step mode
    debugger.set_single_step(true);
    
    // Run the debugger
    match debugger.run() {
        Ok(()) => println!("Debugging session ended."),
        Err(e) => {
            eprintln!("Error during execution: {}", e);
            std::process::exit(1);
        }
    }

    Ok(())
}