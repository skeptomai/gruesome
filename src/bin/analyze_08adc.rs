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
    
    println!("=== Analyzing the print_paddr at 0x08adc ===\n");
    
    // The garbage text is printed by:
    // print_paddr at 08adc: operand=4e1c
    
    // This is inside routine at 0x8aa4
    println!("The routine at 0x8aa4 contains the print_paddr instruction.");
    println!("Let's disassemble around 0x08adc:\n");
    
    let mut addr = 0x8ac0; // Start a bit before
    for _ in 0..20 {
        match disasm.disassemble_instruction(addr) {
            Ok((inst, text)) => {
                if addr == 0x8adc {
                    println!("{:05x}: {} <-- THIS PRINTS THE GARBAGE!", addr, text);
                } else {
                    println!("{:05x}: {}", addr, text);
                }
                addr += inst.size as u32;
            }
            Err(e) => {
                println!("{:05x}: Error: {}", addr, e);
                break;
            }
        }
    }
    
    println!("\n=== Checking what's at packed address 0x4e1c ===");
    
    // print_paddr uses packed addresses, so unpack it
    let unpacked = (0x4e1c as u32) * 2; // For V3, multiply by 2
    println!("Packed address 0x4e1c unpacks to 0x{:05x}", unpacked);
    
    // This should be 0x9c38, which we found earlier contains the garbage text
    println!("\nThis matches our earlier finding:");
    println!("- Address 0x09c38 contains: \"w can you attack a spirit with material objects?\"");
    
    // Now let's see what calls the routine at 0x8aa4
    println!("\n=== Looking for calls to routine at 0x8aa4 ===");
    
    // The packed address for 0x8aa4 would be 0x4552
    let packed_8aa4 = 0x8aa4 / 2;
    println!("Routine at 0x8aa4 has packed address 0x{:04x}", packed_8aa4);
    
    // From our debug log, we saw:
    // Call to packed address 0x4552 (unpacked: 8aa4)
    
    println!("\nFrom the debug log, this routine is called after:");
    println!("1. PERFORM tries object 4's property 17 (returns NULL)");
    println!("2. NULL call returns 0");
    println!("3. Execution continues and eventually calls 0x4552 (8aa4)");
    println!("4. That routine prints from address 0x4e1c (9c38)");
    println!("5. The garbage text appears!");
    
    Ok(())
}