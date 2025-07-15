use infocom::vm::Game;
use infocom::disassembler::Disassembler;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let game_path = "resources/test/zork1/DATA/ZORK1.DAT";
    
    // Load game data
    let mut f = std::fs::File::open(game_path)?;
    let mut memory = Vec::new();
    std::io::Read::read_to_end(&mut f, &mut memory)?;
    
    let game = Game::from_memory(memory)?;
    
    println!("Disassembling around 0x5491:");
    
    let mut disasm = Disassembler::new(&game);
    for addr in 0x5480..0x54a0 {
        if let Ok((inst, text)) = disasm.disassemble_instruction(addr) {
            println!("{}", text);
        }
    }
    
    Ok(())
}