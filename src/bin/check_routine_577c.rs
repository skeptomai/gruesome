use infocom::vm::{Game, VM};
use infocom::disassembler::Disassembler;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let game_path = "resources/test/zork1/DATA/ZORK1.DAT";
    
    // Load game data
    let mut f = std::fs::File::open(game_path)?;
    let mut memory = Vec::new();
    std::io::Read::read_to_end(&mut f, &mut memory)?;
    
    let game = Game::from_memory(memory)?;
    let vm = VM::new(game);
    println!("=== Routine at 0x577c (called after parsing 'w') ===\n");
    
    // Disassemble the routine
    println!("Disassembly:");
    let disasm = Disassembler::new(&vm.game);
    let output = disasm.disassemble_range(0x577c, 0x577c + 100)?;
    print!("{}", output);
    
    println!("\n=== What we know ===");
    println!("1. This routine is called after parsing 'w'");
    println!("2. It gets property 17 of object 4");
    println!("3. Object 4's property 17 = 00 00");
    println!("4. This causes a call to address 0x0000");
    
    Ok(())
}