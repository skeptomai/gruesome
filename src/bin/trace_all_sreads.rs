use gruesome::interpreter::Interpreter;
use gruesome::vm::{Game, VM};
use log::info;
use std::io::Read;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp(None)
        .init();
    
    let game_path = "resources/test/zork1/DATA/ZORK1.DAT";
    
    let mut f = std::fs::File::open(game_path)?;
    let mut memory = Vec::new();
    f.read_to_end(&mut memory)?;
    
    let game = Game::from_memory(memory)?;
    let vm = VM::new(game);
    let mut interpreter = Interpreter::new(vm);
    
    // Patch the interpreter to log ALL sread locations
    let original_pc = interpreter.vm.pc;
    info!("Scanning for SREAD instructions...");
    
    // Scan memory for sread instructions
    for addr in 0..game.memory.len() {
        if let Ok(inst) = gruesome::instruction::Instruction::decode(
            &game.memory, 
            addr,
            game.header.version
        ) {
            if inst.opcode == 0x04 && inst.operands.len() >= 2 {
                info!("SREAD at 0x{:04x}:", addr);
                info!("  Operands: {:?}", inst.operands);
                if inst.operands.len() >= 4 && inst.operands[2] > 0 && inst.operands[3] > 0 {
                    info!("  >>> HAS TIMER: time={} ({}s), routine=0x{:04x}",
                          inst.operands[2], inst.operands[2] as f32 / 10.0, inst.operands[3]);
                }
            }
        }
    }
    
    Ok(())
}