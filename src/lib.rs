#![crate_name = "gruesome"]
#![allow(dead_code)]

#[macro_use]
extern crate lazy_static;

pub mod debug_symbols;
pub mod debugger;
pub mod dictionary;
pub mod disassembler;
pub mod display_headless;
pub mod display_logging;
pub mod display_manager;
pub mod display_ratatui;
pub mod display_trait;
pub mod display_v3;
pub mod game;
pub mod gamememorymap;
pub mod header;
pub mod input_v3;
pub mod input_v4;
pub mod instruction;
pub mod interpreter;
pub mod opcode_tables;
pub mod property_defaults;
pub mod quetzal;
pub mod routine;
pub mod text;
pub mod timed_input;
pub mod util;
pub mod vm;
pub mod zobject;
pub mod zobject_v3;
pub mod zobject_v4;
pub mod zobject_interface;
pub mod zrand;

#[cfg(test)]
mod test_execution;

#[cfg(test)]
mod tests {
    use crate::disassembler::Disassembler;
    use crate::game::GameFile;
    use crate::instruction::Instruction;
    use crate::interpreter::Interpreter;
    use crate::vm::{Game, VM};
    use crate::zrand::ZRand;
    use std::env;
    use std::fs::File;
    use std::io;
    use std::io::prelude::*;
    use std::path::PathBuf;

    const DATAFILEPATH: &str = "resources/test/zork1/DATA/ZORK1.DAT";

    use log::{debug, info};
    use test_log::test;

    #[test]
    fn read_zork1() -> io::Result<()> {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push(DATAFILEPATH);

        // open file and read all bytes into vector
        let mut f = File::open(path)?;
        let mut all_bytes = Vec::new();
        f.read_to_end(&mut all_bytes).unwrap();

        // create random generator
        let mut zrg = ZRand::new_uniform();

        // Instantiate gamefile structure
        let g = GameFile::new(&all_bytes, &mut zrg);

        // dump the game structure
        info!("{}", g);
        Ok(())
    }

    #[test]
    fn test_object_debug_dump() -> io::Result<()> {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push(DATAFILEPATH);

        // Load the game file
        let mut f = File::open(path)?;
        let mut all_bytes = Vec::new();
        f.read_to_end(&mut all_bytes)?;

        let mut zrg = ZRand::new_uniform();
        let g = GameFile::new(&all_bytes, &mut zrg);

        info!("Testing object debug dump functionality");

        // Get the object table if it exists
        if let Some(obj_table) = g.get_object_table() {
            // Dump a few specific objects
            info!("Dumping object #1 (West of House):");
            obj_table.debug_dump_object(1);

            info!("\nDumping object #2 (Stone Barrow):");
            obj_table.debug_dump_object(2);

            // Try an invalid object
            debug!("Testing invalid object number:");
            obj_table.debug_dump_object(999);
        } else {
            info!("No object table found in game file");
        }

        Ok(())
    }

    #[test]
    fn test_vm_with_zork() -> io::Result<()> {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push(DATAFILEPATH);

        // Load the game file
        let mut f = File::open(path)?;
        let mut all_bytes = Vec::new();
        f.read_to_end(&mut all_bytes)?;

        // Create VM
        let game = Game::from_memory(all_bytes).unwrap();
        let vm = VM::new(game);

        // Display VM info
        println!("VM created for Zork I");
        println!("Version: {}", vm.game.header.version);
        println!("Initial PC: {:04x}", vm.game.header.initial_pc);
        println!("Global variables: {:04x}", vm.game.header.global_variables);
        println!("Object table: {:04x}", vm.game.header.object_table_addr);
        println!("Static memory: {:04x}", vm.game.header.base_static_mem);

        Ok(())
    }

    #[test]
    fn test_disassembler_with_zork() -> io::Result<()> {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push(DATAFILEPATH);

        // Load the game file
        let mut f = File::open(path)?;
        let mut all_bytes = Vec::new();
        f.read_to_end(&mut all_bytes)?;

        // Create VM and disassembler
        let game = Game::from_memory(all_bytes).unwrap();
        let disasm = Disassembler::new(&game);

        // Disassemble the main routine
        println!("\n=== Disassembling Zork I Main Routine ===");
        match disasm.disassemble_main() {
            Ok(output) => println!("{output}"),
            Err(e) => println!("Error disassembling: {e}"),
        }

        // Also try to disassemble a specific range
        let start_pc = game.header.initial_pc as u32;
        println!("\n=== First 20 instructions ===");
        match disasm.disassemble_range(start_pc, start_pc + 50) {
            Ok(output) => println!("{output}"),
            Err(e) => println!("Error: {e}"),
        }

        Ok(())
    }

    #[test]
    fn test_zork_opening() -> io::Result<()> {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push(DATAFILEPATH);

        // Load the game file
        let mut f = File::open(path)?;
        let mut all_bytes = Vec::new();
        f.read_to_end(&mut all_bytes)?;

        // Create VM and interpreter
        let game = Game::from_memory(all_bytes).unwrap();
        let vm = VM::new(game);
        let mut interpreter = Interpreter::new(vm);
        interpreter.set_debug(true);

        info!("\n=== Running Zork I Opening ===");

        // Execute a limited number of instructions to see what happens
        for i in 0..500 {
            let pc = interpreter.vm.pc;
            let inst = match Instruction::decode(&interpreter.vm.game.memory, pc as usize, 3) {
                Ok(inst) => inst,
                Err(e) => {
                    info!("Failed to decode at {:04x}: {}", pc, e);
                    break;
                }
            };

            info!(
                "Step {}: {:04x}: {} (stack len: {})",
                i,
                pc,
                inst.format_with_version(3),
                interpreter.vm.stack.len()
            );

            // Advance PC
            interpreter.vm.pc += inst.size as u32;

            // Execute
            match interpreter.execute_instruction(&inst) {
                Ok(result) => {
                    debug!("Result: {:?}", result);
                    match result {
                        crate::interpreter::ExecutionResult::Quit => {
                            info!("Game quit");
                            break;
                        }
                        crate::interpreter::ExecutionResult::Called => {
                            info!("Called routine, PC now at {:04x}", interpreter.vm.pc);
                        }
                        _ => {}
                    }
                }
                Err(e) => {
                    info!("Execution error: {}", e);
                    break;
                }
            }
        }

        Ok(())
    }

    #[test]
    fn test_interpreter_simple() {
        // Create a simple test program
        let mut memory = vec![0u8; 0x10000];

        // Set up header
        memory[0x00] = 3; // Version 3
        memory[0x04] = 0x10; // High memory at 0x1000
        memory[0x05] = 0x00;
        memory[0x06] = 0x50; // Initial PC at 0x5000
        memory[0x07] = 0x00;
        memory[0x0c] = 0x01; // Global table at 0x0100
        memory[0x0d] = 0x00;
        memory[0x0e] = 0x02; // Static memory at 0x0200
        memory[0x0f] = 0x00;

        // Simple program: print_num 42, new_line, quit
        let pc = 0x5000;
        memory[pc] = 0xE6; // VAR:2OP print_num
        memory[pc + 1] = 0x7F; // Operand types: small constant (01), then omitted (11, 11, 11)
        memory[pc + 2] = 42; // Value: 42
        memory[pc + 3] = 0xBB; // new_line
        memory[pc + 4] = 0xBA; // quit

        let game = Game::from_memory(memory).unwrap();
        let vm = VM::new(game);
        let mut interpreter = Interpreter::new(vm);

        println!("\n=== Running simple test program ===");
        match interpreter.run() {
            Ok(()) => println!("Program completed successfully"),
            Err(e) => println!("Error: {e}"),
        }
    }
}

/*
An example memory map of a small game
Dynamic	00000	header
        00040	abbreviation strings
        00042	abbreviation table
        00102	property defaults
        00140	objects
        002f0	object descriptions and properties
        006e3	global variables
        008c3	arrays
Static	00b48	grammar table
        010a7	actions table
        01153	preactions table
        01201	adjectives table
        0124d	dictionary
High	01a0a	Z-code
        05d56	static strings
        06ae6	end of file
*/
