use infocom::vm::{Game, VM};
use infocom::interpreter::Interpreter;
use std::env;
use std::fs::File;
use std::io::prelude::*;
use log::debug;

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
    let vm = VM::new(game);
    let mut interpreter = Interpreter::new(vm);

    println!("=== Checking Lighting Attributes Before and After Initialization ===");
    
    // Check lighting attributes BEFORE running any code
    println!("\n=== BEFORE initialization ===");
    check_location_lighting(&interpreter, 180, "West of House");
    check_location_lighting(&interpreter, 78, "Forest");
    check_location_lighting(&interpreter, 79, "Behind House");
    check_location_lighting(&interpreter, 80, "South of House");
    check_location_lighting(&interpreter, 81, "North of House");
    
    // Run a limited number of instructions to allow initialization
    println!("\n=== Running initialization (first 1000 instructions) ===");
    match interpreter.run_with_limit(Some(1000)) {
        Ok(()) => println!("Initialization completed successfully"),
        Err(e) => println!("Initialization stopped: {}", e),
    }
    
    // Check lighting attributes AFTER initialization
    println!("\n=== AFTER initialization ===");
    check_location_lighting(&interpreter, 180, "West of House");
    check_location_lighting(&interpreter, 78, "Forest");
    check_location_lighting(&interpreter, 79, "Behind House");
    check_location_lighting(&interpreter, 80, "South of House");
    check_location_lighting(&interpreter, 81, "North of House");
    
    // Also check for any changes in the current location
    println!("\n=== Current Location Check ===");
    match interpreter.vm.read_global(0x10) {
        Ok(location) => {
            println!("Current location is object {}", location);
            if location > 0 && location <= 255 {
                check_location_lighting(&interpreter, location, "Current Location");
            }
        },
        Err(e) => println!("Could not read current location: {}", e),
    }

    Ok(())
}

fn check_location_lighting(interpreter: &Interpreter, obj_num: u16, name: &str) {
    println!("  {} (obj {}):", name, obj_num);
    
    // Check attribute 3 (ONBIT)
    match interpreter.vm.test_attribute(obj_num, 3) {
        Ok(has_onbit) => println!("    Attribute 3 (ONBIT): {}", has_onbit),
        Err(e) => println!("    Error checking ONBIT: {}", e),
    }
    
    // Check other potential lighting attributes
    for attr in [2, 8, 12, 15, 21] {
        match interpreter.vm.test_attribute(obj_num, attr) {
            Ok(true) => println!("    Attribute {} is SET", attr),
            Ok(false) => {}, // Don't spam with unset attributes
            Err(_) => {},
        }
    }
}