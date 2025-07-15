use infocom::game::GameFile;
use infocom::zrand::{ZRand, RandMode};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let game_path = "resources/test/zork1/DATA/ZORK1.DAT";
    
    // Load game data
    let mut f = std::fs::File::open(game_path)?;
    let mut memory = Vec::new();
    std::io::Read::read_to_end(&mut f, &mut memory)?;
    
    // Create GameFile to access object table
    let mut rng = ZRand::new(RandMode::RandomUniform);
    let game_file = GameFile::new(&memory, &mut rng);
    
    println!("=== Checking property 29 of object 180 (West of House) ===\n");
    
    // Get object table
    if let Some(obj_table) = game_file.get_object_table() {
        // Debug dump object 180
        obj_table.debug_dump_object(180);
    }
    
    // Also check the raw memory at the property address
    println!("\n=== Raw memory check ===");
    
    // From the debug log, property 29 is at address 0x1c2f
    let prop_addr = 0x1c2f;
    println!("Property 29 data at address 0x{:04x}:", prop_addr);
    
    // In Z-Machine v3, properties have a size byte followed by data
    // The size byte encodes both the property number and size
    let size_byte = memory[prop_addr - 1];
    let prop_num = size_byte & 0x1f;
    let prop_size = ((size_byte >> 5) & 0x07) + 1;
    
    println!("Size byte: 0x{:02x}", size_byte);
    println!("Property number: {}", prop_num);
    println!("Property size: {} bytes", prop_size);
    
    print!("Property data: ");
    for i in 0..prop_size as usize {
        if prop_addr + i < memory.len() {
            print!("{:02x} ", memory[prop_addr + i]);
        }
    }
    println!();
    
    // Check if the first two bytes are 0x4e 0x1c
    if prop_size >= 2 {
        let word = ((memory[prop_addr] as u16) << 8) | memory[prop_addr + 1] as u16;
        println!("\nFirst word of property data: 0x{:04x}", word);
        if word == 0x4e1c {
            println!("*** This is our problematic address 0x4e1c! ***");
            let unpacked = word * 2;
            println!("Unpacked address: 0x{:05x}", unpacked);
            println!("This points into the middle of the spirit routine!");
        }
    }
    
    Ok(())
}