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
    
    println!("=== Tracing property 17 check ===\n");
    
    // From debug log:
    // - Call to 0x2bbe (unpacked: 577c)
    // - Immediately: get_property_addr: obj=4, prop=17
    // - Then: Call to 0x0000
    // - Then: get_parent at PC 05805
    
    println!("Looking around PC 05805 where get_parent is called:");
    let disasm = Disassembler::new(&vm.game);
    let output = disasm.disassemble_range(0x57f0, 0x5810)?;
    print!("{}", output);
    
    println!("\n=== Analysis ===");
    println!("The routine at 0x577c must be checking property 17");
    println!("of object 4 and calling it as a function.");
    println!("Since property 17 = 00 00, it calls address 0x0000.");
    
    Ok(())
}