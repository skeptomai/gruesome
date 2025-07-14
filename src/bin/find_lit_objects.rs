use infocom::vm::{Game, VM};
use std::env;
use std::fs::File;
use std::io::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Get command line arguments
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <game_file.dat>", args[0]);
        eprintln!("Example: {} resources/test/zork1/DATA/ZORK1.DAT", args[0]);
        std::process::exit(1);
    }

    let game_path = &args[1];

    // Load the game file
    let mut file = File::open(game_path)?;
    let mut game_data = Vec::new();
    file.read_to_end(&mut game_data)?;

    // Create the game and VM
    let game = Game::from_memory(game_data)?;
    let vm = VM::new(game);

    println!("=== Finding Objects with ONBIT (Attribute 3) Set ===");
    
    let mut onbit_objects = Vec::new();
    
    // Check all objects for ONBIT
    for obj_num in 1..=250 {
        match vm.test_attribute(obj_num, 3) {
            Ok(true) => {
                onbit_objects.push(obj_num);
                println!("Object {} has ONBIT (attribute 3) SET", obj_num);
                
                // Get some additional info about this object
                let obj_addr = get_object_addr(&vm, obj_num);
                if let Ok(addr) = obj_addr {
                    let prop_addr = vm.read_word((addr + 7) as u32) as usize;
                    let desc_len = vm.read_byte(prop_addr as u32) as usize;
                    if desc_len > 0 {
                        // Try to get a short description by reading a few bytes
                        print!("  Raw description bytes: ");
                        for i in 0..std::cmp::min(desc_len * 2, 20) {
                            print!("{:02x} ", vm.read_byte((prop_addr + 1 + i) as u32));
                        }
                        println!();
                    }
                }
            },
            Ok(false) => {}, // Not set
            Err(_) => break, // Invalid object number, stop searching
        }
    }
    
    if onbit_objects.is_empty() {
        println!("No objects found with ONBIT (attribute 3) set!");
        println!("This suggests lighting works through a different mechanism.");
    } else {
        println!("\nFound {} objects with ONBIT set: {:?}", onbit_objects.len(), onbit_objects);
    }
    
    println!("\n=== Checking Other Potential Light Attributes ===");
    
    // Check for other potential lighting attributes
    for attr in [2, 8, 12, 15, 21] {
        let mut objects_with_attr = Vec::new();
        
        for obj_num in 1..=100 { // Check first 100 objects
            match vm.test_attribute(obj_num, attr) {
                Ok(true) => objects_with_attr.push(obj_num),
                _ => {},
            }
        }
        
        if !objects_with_attr.is_empty() {
            println!("Attribute {}: {} objects have it set: {:?}", 
                     attr, objects_with_attr.len(), 
                     if objects_with_attr.len() <= 10 { 
                         format!("{:?}", objects_with_attr) 
                     } else { 
                         format!("{:?}...", &objects_with_attr[0..10]) 
                     });
        }
    }

    Ok(())
}

fn get_object_addr(vm: &VM, obj_num: u16) -> Result<usize, String> {
    if obj_num == 0 || obj_num > 255 {
        return Err(format!("Invalid object number: {}", obj_num));
    }

    let obj_table_addr = vm.game.header.object_table_addr as usize;
    let property_defaults = obj_table_addr;
    let obj_tree_base = property_defaults + 31 * 2; // 31 default properties, 2 bytes each

    Ok(obj_tree_base + ((obj_num - 1) as usize * 9))
}