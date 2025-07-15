use infocom::vm::{Game, VM};
use infocom::interpreter::Interpreter;
use infocom::disassembler::Disassembler;
use infocom::instruction::InstructionForm;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let game_path = "resources/test/zork1/DATA/ZORK1.DAT";
    
    // Load game data
    let mut f = std::fs::File::open(game_path)?;
    let mut memory = Vec::new();
    std::io::Read::read_to_end(&mut f, &mut memory)?;
    
    let game = Game::from_memory(memory)?;
    let vm = VM::new(game);
    let mut interpreter = Interpreter::new(vm);
    let disasm = Disassembler::new(&interpreter.vm.game);
    
    println!("=== Analyzing what happens after NULL call in PERFORM ===\n");
    
    // From our trace:
    // [057fb] call V00 -> V04  (V00=0, stores 0 to V04)
    // [057ff] jz V04 [TRUE +5] (V04=0, so branch is taken)
    // Branch offset +5 means PC goes to 0x5801 + 5 = 0x5806
    
    let mut addr = 0x5806;
    println!("After the jz branch, PC would be at 0x{:04x}\n", addr);
    println!("Disassembling from there:");
    
    for _ in 0..20 {
        match disasm.disassemble_instruction(addr) {
            Ok((inst, text)) => {
                println!("  {:05x}: {}", addr, text);
                
                // Check if this is a print instruction
                if inst.opcode == 0x02 && inst.form == InstructionForm::Short {
                    println!("    -> This is a print instruction!");
                }
                
                addr += inst.size as u32;
            }
            Err(e) => {
                println!("  {:05x}: Error: {}", addr, e);
                break;
            }
        }
    }
    
    // Also check if 0x5806 is outside the PERFORM routine
    println!("\n=== Routine boundaries ===");
    println!("PERFORM starts at 0x577c");
    println!("We expected it to end around 0x5800");
    println!("But after the branch, we're at 0x5806");
    println!("This is OUTSIDE the PERFORM routine!");
    
    // Let's see what routine contains 0x5806
    println!("\nChecking what's at 0x5806 and beyond...");
    
    Ok(())
}