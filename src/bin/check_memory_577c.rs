use infocom::vm::{Game, VM};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let game_path = "resources/test/zork1/DATA/ZORK1.DAT";
    
    // Load game data
    let mut f = std::fs::File::open(game_path)?;
    let mut memory = Vec::new();
    std::io::Read::read_to_end(&mut f, &mut memory)?;
    
    let game = Game::from_memory(memory)?;
    let vm = VM::new(game);
    
    println!("=== Memory around 0x577c ===\n");
    
    // Show memory before and after
    let start = 0x5770;
    let end = 0x5790;
    
    println!("Address  Data");
    for addr in (start..end).step_by(8) {
        print!("{:04x}:", addr);
        for i in 0..8 {
            let byte_addr = addr + i;
            if byte_addr < vm.game.memory.len() {
                print!(" {:02x}", vm.game.memory[byte_addr]);
            }
        }
        println!();
    }
    
    println!("\nObservation: The area around 0x577c is all zeros!");
    println!("This suggests:");
    println!("1. The routine might be elsewhere");
    println!("2. OR this is dynamic memory that gets filled at runtime");
    println!("3. OR there's an issue with how we're loading the game file");
    
    // Let's check what the actual first call after parsing is
    println!("\nFrom debug log, after parsing 'w':");
    println!("- Returns from parse routine");
    println!("- Then calls 0x2bbe (which should unpack to 0x577c)");
    println!("- But 0x577c is all zeros...");
    
    Ok(())
}