use gruesome::disassembler::Disassembler;
use gruesome::vm::{Game, VM};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let game_path = "resources/test/zork1/DATA/ZORK1.DAT";

    // Load game data
    let mut f = std::fs::File::open(game_path)?;
    let mut memory = Vec::new();
    std::io::Read::read_to_end(&mut f, &mut memory)?;

    let game = Game::from_memory(memory)?;
    let vm = VM::new(game);
    let disasm = Disassembler::new(&vm.game);

    println!("=== Comparing expected vs actual execution ===\n");

    println!("From working interpreter, 'w' should move west to Forest.");
    println!("From our interpreter, 'w' prints garbage text.\n");

    println!("Key execution points we've identified:");
    println!("1. Dictionary lookup finds 'w' with type 0x32");
    println!("2. PERFORM (0x577c) processes the command");
    println!("3. Object 4's property 17 is NULL (correct)");
    println!("4. Continues to check parent (object 180)");
    println!("5. Eventually calls routine 0x8aa4");
    println!("6. That routine does loadw from property 29");
    println!("7. Gets 0x4e1c due to 1-byte property being read as 2 bytes");
    println!("8. Prints garbage\n");

    println!("The question is: What should happen differently?\n");

    // Let's look at the code flow more carefully
    println!("=== Analyzing routine 0x8aa4 ===");
    let mut addr = 0x8aa4;
    println!("This routine seems to be a property handler.");
    println!("When called with V02=2, it loads and prints from an address.\n");

    for _i in 0..15 {
        match disasm.disassemble_instruction(addr) {
            Ok((inst, text)) => {
                println!("{:05x}: {}", addr, text);
                addr += inst.size as u32;
            }
            Err(_) => break,
        }
    }

    println!("\n=== Theory ===");
    println!("Perhaps the issue is earlier in the execution:");
    println!("- Maybe we're not supposed to reach routine 0x8aa4 at all?");
    println!("- Maybe V01 should point to different data?");
    println!("- Maybe the property lookup is wrong?");
    println!("\nThe working interpreter must either:");
    println!("1. Take a different code path that avoids this routine");
    println!("2. Have V01 point to valid data (not property 29)");
    println!("3. Handle the 1-byte property differently");

    Ok(())
}
