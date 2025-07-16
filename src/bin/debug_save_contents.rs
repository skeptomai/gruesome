use infocom::vm::{Game, VM};
use infocom::interpreter::Interpreter;
use infocom::instruction::Instruction;
use infocom::quetzal::iff::IffFile;
use infocom::quetzal::chunks::StksChunk;
use std::io::Read;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let game_path = "resources/test/zork1/DATA/ZORK1.DAT";
    
    // Load game data
    let mut f = std::fs::File::open(game_path)?;
    let mut memory = Vec::new();
    f.read_to_end(&mut memory)?;
    
    // Create VM and run to first prompt
    let game = Game::from_memory(memory)?;
    let vm = VM::new(game);
    let mut interpreter = Interpreter::new(vm);
    
    // Run until first sread
    loop {
        let pc = interpreter.vm.pc;
        let inst = match Instruction::decode(&interpreter.vm.game.memory, pc as usize, 3) {
            Ok(inst) => inst,
            Err(_) => break,
        };
        
        if inst.opcode == 0x04 && matches!(inst.operand_count, infocom::instruction::OperandCount::VAR) {
            println!("Stopped at sread, PC=0x{:05x}", pc);
            break;
        }
        
        interpreter.vm.pc += inst.size as u32;
        match interpreter.execute_instruction(&inst) {
            Ok(_) => {},
            Err(_) => break,
        }
    }
    
    println!("\n=== VM State Before Save ===");
    println!("Call stack frames: {}", interpreter.vm.call_stack.len());
    for (i, frame) in interpreter.vm.call_stack.iter().enumerate() {
        println!("Frame {}: return_pc=0x{:05x}, locals={}, stack_base={}", 
                 i, frame.return_pc, frame.num_locals, frame.stack_base);
    }
    println!("Stack values: {:?}", interpreter.vm.stack);
    
    // Save the game
    let save = infocom::quetzal::save::SaveGame::from_vm(&interpreter.vm)?;
    save.save_to_file(std::path::Path::new("debug2.sav"))?;
    
    // Load and analyze the save file
    println!("\n=== Save File Contents ===");
    let iff = IffFile::read_from_file(std::path::Path::new("debug2.sav"))?;
    
    if let Some(stks_chunk) = iff.find_chunk(b"Stks") {
        println!("Stks chunk size: {} bytes", stks_chunk.data.len());
        println!("Hex dump of Stks data:");
        for (i, byte) in stks_chunk.data.iter().enumerate() {
            if i % 16 == 0 && i > 0 { println!(); }
            print!("{:02x} ", byte);
        }
        println!();
        
        // Manually parse to understand
        println!("\nManual parse of Stks:");
        let data = &stks_chunk.data;
        let mut offset = 0;
        let mut frame_num = 0;
        
        while offset < data.len() {
            println!("\nFrame {}:", frame_num);
            
            if offset + 4 > data.len() { break; }
            
            let return_pc = ((data[offset] as u32) << 16) 
                          | ((data[offset + 1] as u32) << 8) 
                          | (data[offset + 2] as u32);
            println!("  Return PC: 0x{:05x}", return_pc);
            offset += 3;
            
            let flags = data[offset];
            let local_count = flags & 0x0F;
            let has_result = (flags & 0x10) != 0;
            println!("  Flags: 0x{:02x} (locals={}, has_result={})", flags, local_count, has_result);
            offset += 1;
            
            if has_result {
                if offset < data.len() {
                    println!("  Result var: {}", data[offset]);
                    offset += 1;
                }
            }
            
            if offset + 2 > data.len() { break; }
            let stack_count = u16::from_be_bytes([data[offset], data[offset + 1]]);
            println!("  Stack count: {}", stack_count);
            offset += 2;
            
            // Skip locals and stack values for brevity
            offset += (local_count as usize) * 2;
            offset += (stack_count as usize) * 2;
            
            frame_num += 1;
        }
    }
    
    // Clean up
    std::fs::remove_file("debug2.sav").ok();
    
    Ok(())
}