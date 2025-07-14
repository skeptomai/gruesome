use std::fs::File;
use std::io::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut f = File::open("resources/test/zork1/DATA/ZORK1.DAT")?;
    let mut game_data = Vec::new();
    f.read_to_end(&mut game_data)?;

    println!("Tracing execution through routine 8eaa\n");
    
    println!("From the memory map:");
    println!("- High memory (code) starts at: 0x4e37");
    println!("- Static memory: 0x2e53 - 0x4e36");
    println!("- Dynamic memory: 0x0000 - 0x2e52");
    
    println!("\n0x093bb is at: 0x093bb");
    if 0x093bb < 0x2e53 {
        println!("This is in DYNAMIC memory (0x0000 - 0x2e52)");
    } else if 0x093bb < 0x4e37 {
        println!("This is in STATIC memory (0x2e53 - 0x4e36)");
    } else {
        println!("This is in HIGH memory (code) (0x4e37+)");
    }
    
    println!("\nSo 0x093bb CANNOT be an instruction address!");
    println!("We must be misunderstanding the execution trace.");
    
    println!("\nLooking at routine 8eaa:");
    println!("- It has 10 locals");
    println!("- It's a complex routine with many branches and recursive calls");
    println!("- It calls itself recursively at 0x8f30 and 0x8fc1");
    println!("- It also calls 0x8d92 at 0x8fa7");
    
    println!("\nThe problem must be:");
    println!("1. We're somehow jumping to or executing from non-code memory");
    println!("2. Our PC trace showing 0x093bb must be wrong");
    println!("3. Or we have a corruption in our interpreter state");
    
    // Let's check what's actually at 0x093bb in memory
    println!("\nWhat's at 0x093bb in the game data:");
    if 0x093bb < game_data.len() {
        print!("  Bytes: ");
        for i in 0..8 {
            if 0x093bb + i < game_data.len() {
                print!("{:02x} ", game_data[0x093bb + i]);
            }
        }
        println!();
        
        // If we try to decode it as an instruction (which we shouldn't!)
        if let Ok(inst) = infocom::instruction::Instruction::decode(&game_data, 0x093bb, 3) {
            println!("  If decoded as instruction (WRONG!): {}", inst.format_with_version(3));
        }
    }
    
    println!("\nThe real question is: How did our PC get to 0x093bb?");
    println!("This address is in static memory and should never be executed!");
    
    Ok(())
}