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
    let disasm = Disassembler::new(&vm.game);
    
    println!("=== Why is routine 0x8aa4 printing from 0x9c38? ===\n");
    
    // We know:
    // - Routine at 0x8aa4 does: print_paddr V00 at 0x8adc
    // - V00 contains 0x4e1c (packed address)
    // - 0x4e1c unpacks to 0x9c38
    // - 0x9c38 is in the MIDDLE of a print instruction in the spirit routine
    
    println!("The spirit interaction routine at 0x9be8:");
    let mut addr = 0x9c30;
    for _ in 0..10 {
        match disasm.disassemble_instruction(addr) {
            Ok((inst, text)) => {
                if addr == 0x9c35 {
                    println!("{:05x}: {} <-- PRINT_RET starts here", addr, text);
                } else if addr == 0x9c38 {
                    println!("{:05x}: <-- Address 0x9c38 is HERE (middle of string data!)", addr);
                } else {
                    println!("{:05x}: {}", addr, text);
                }
                addr += inst.size as u32;
            }
            Err(_) => break,
        }
    }
    
    println!("\n=== Understanding the issue ===");
    println!("1. The PRINT_RET instruction at 0x9c35 includes embedded string data");
    println!("2. The string starts at 0x9c36 or 0x9c37");
    println!("3. Address 0x9c38 is INSIDE the encoded string data");
    println!("4. When print_paddr tries to 'print' from 0x9c38, it's starting");
    println!("   in the middle of the encoded text, producing garbage");
    
    println!("\n=== The real question ===");
    println!("Why is V00 set to 0x4e1c at the time print_paddr is called?");
    println!("This suggests that the code is using the wrong data as an address.");
    
    // Let's check what the bytes are around the call to 0x8aa4
    println!("\n=== Checking memory around where 0x8aa4 is called ===");
    
    // From debug logs, 0x8aa4 is called from somewhere after PERFORM
    // Let's see if we can find where V01 gets set to point to bad data
    
    Ok(())
}