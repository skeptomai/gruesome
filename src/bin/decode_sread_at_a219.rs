use infocom::vm::Game;
use log::info;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    
    let game_path = "resources/test/zork1/DATA/ZORK1.DAT";
    
    // Load game data
    let mut f = std::fs::File::open(game_path)?;
    let mut memory = Vec::new();
    std::io::Read::read_to_end(&mut f, &mut memory)?;
    
    let game = Game::from_memory(memory)?;
    
    // Look at SREAD at 0x0a219
    let addr = 0x0a219;
    info!("Examining SREAD at 0x{:05x}", addr);
    
    // Show bytes
    info!("Bytes at 0x{:05x}:", addr);
    for i in 0..8 {
        info!("  [{:05x}] = {:02x}", addr + i, game.memory[addr + i]);
    }
    
    // Decode the instruction
    // 0xea = 11101010 - VAR form with 2 operands (bits 7-6 = 11, bits 5-4 = 10, bits 3-2 = 10, bits 1-0 = 10)
    // bit pattern 10 = variable operand
    let form_byte = game.memory[addr];
    let opcode_byte = game.memory[addr + 1];
    
    info!("\nForm byte: {:02x} = {:08b}", form_byte, form_byte);
    info!("Opcode: {:02x} (SREAD)", opcode_byte);
    
    // Both operands are variables (bit pattern 10)
    let var1 = game.memory[addr + 2];
    let var2 = game.memory[addr + 3];
    
    info!("\nOperands:");
    info!("  Variable 1: {:02x} ({})", var1, decode_variable(var1));
    info!("  Variable 2: {:02x} ({})", var2, decode_variable(var2));
    
    // Show what's in those variables' initial values (globals)
    info!("\nChecking global variables:");
    let globals_start = game.header.global_variables;
    info!("Globals start at: 0x{:04x}", globals_start);
    
    // Variables 0x10-0xff are globals
    if var1 >= 0x10 {
        let global_offset = (var1 - 0x10) as usize * 2;
        let global_addr = globals_start + global_offset;
        let value = ((game.memory[global_addr] as u16) << 8) | (game.memory[global_addr + 1] as u16);
        info!("  Global variable {} (at 0x{:04x}) = 0x{:04x}", var1 - 0x10, global_addr, value);
    }
    
    if var2 >= 0x10 {
        let global_offset = (var2 - 0x10) as usize * 2;
        let global_addr = globals_start + global_offset;
        let value = ((game.memory[global_addr] as u16) << 8) | (game.memory[global_addr + 1] as u16);
        info!("  Global variable {} (at 0x{:04x}) = 0x{:04x}", var2 - 0x10, global_addr, value);
    }
    
    Ok(())
}

fn decode_variable(var: u8) -> String {
    match var {
        0x00 => "(SP) stack".to_string(),
        0x01..=0x0f => format!("local{}", var),
        _ => format!("global{}", var - 0x10),
    }
}