// Bytecode Debugging Tool
// Provides hexdump and instruction decoding for Z-Machine files

use std::env;
use std::fs::File;
use std::io::Read;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <story_file.z3>", args[0]);
        std::process::exit(1);
    }

    let filename = &args[1];
    let mut file = File::open(filename)?;
    let mut data = Vec::new();
    file.read_to_end(&mut data)?;

    println!("=== Z-Machine File Analysis: {} ===", filename);
    println!("File size: {} bytes", data.len());

    if data.len() < 64 {
        eprintln!("Error: File too small to be valid Z-Machine file (< 64 bytes)");
        return Ok(());
    }

    // Analyze header
    print_header_info(&data);

    // Print hexdump of critical sections
    println!("\n=== Header (first 64 bytes) ===");
    print_hexdump(&data, 0, 64);

    // Get initial PC from header
    let pc = u16::from_be_bytes([data[6], data[7]]) as usize;
    println!("\n=== Code section starting at PC: 0x{:04x} ===", pc);

    // Print code section with instruction analysis
    if pc < data.len() {
        print_code_analysis(&data, pc, 200); // Analyze first 200 bytes of code
    }

    Ok(())
}

fn print_header_info(data: &[u8]) {
    println!("\n=== Z-Machine Header Analysis ===");

    let version = data[0];
    let high_mem = u16::from_be_bytes([data[4], data[5]]);
    let pc = u16::from_be_bytes([data[6], data[7]]);
    let dict_addr = u16::from_be_bytes([data[8], data[9]]);
    let obj_table = u16::from_be_bytes([data[10], data[11]]);
    let globals = u16::from_be_bytes([data[12], data[13]]);
    let static_mem = u16::from_be_bytes([data[14], data[15]]);

    println!("Version: {}", version);
    println!("High memory start: 0x{:04x}", high_mem);
    println!("Initial PC: 0x{:04x}", pc);
    println!("Dictionary address: 0x{:04x}", dict_addr);
    println!("Object table: 0x{:04x}", obj_table);
    println!("Global variables: 0x{:04x}", globals);
    println!("Static memory: 0x{:04x}", static_mem);
}

fn print_hexdump(data: &[u8], start: usize, length: usize) {
    let end = std::cmp::min(start + length, data.len());

    for i in (start..end).step_by(16) {
        print!("{:04x}: ", i);

        // Hex bytes
        for j in 0..16 {
            if i + j < end {
                print!("{:02x} ", data[i + j]);
            } else {
                print!("   ");
            }
        }

        print!(" | ");

        // ASCII representation
        for j in 0..16 {
            if i + j < end {
                let byte = data[i + j];
                if (32..=126).contains(&byte) {
                    print!("{}", byte as char);
                } else {
                    print!(".");
                }
            }
        }

        println!();
    }
}

fn print_code_analysis(data: &[u8], start: usize, max_bytes: usize) {
    let mut pos = start;
    let end = std::cmp::min(start + max_bytes, data.len());

    println!("Address  | Hex Bytes      | Analysis");
    println!("---------|----------------|---------------------------");

    while pos < end && pos + 1 < data.len() {
        let _byte = data[pos];
        print!("{:04x}    | ", pos);

        // Print the instruction bytes (up to 4 bytes for safety)
        let instr_end = std::cmp::min(pos + 4, data.len());
        for (offset, &byte) in data.iter().enumerate().take(instr_end).skip(pos) {
            if offset < pos + 4 {
                print!("{:02x} ", byte);
            }
        }
        for _ in (instr_end - pos)..4 {
            print!("   ");
        }

        print!("| ");

        // Analyze the instruction
        let analysis = analyze_instruction(data, pos);
        println!("{}", analysis.description);

        pos += analysis.length;

        // Stop if we hit invalid data
        if analysis.length == 0 {
            println!("       | ?? ??          | INVALID INSTRUCTION - stopping analysis");
            break;
        }
    }
}

#[derive(Debug)]
struct InstructionAnalysis {
    length: usize,
    description: String,
}

fn analyze_instruction(data: &[u8], pos: usize) -> InstructionAnalysis {
    if pos >= data.len() {
        return InstructionAnalysis {
            length: 0,
            description: "END OF DATA".to_string(),
        };
    }

    let byte = data[pos];

    // Check for obvious invalid patterns
    if byte == 0x00 {
        return InstructionAnalysis {
            length: 1,
            description: "INVALID: 0x00 opcode (likely padding or error)".to_string(),
        };
    }

    // Basic opcode analysis
    match byte {
        // 0OP instructions (opcode in bits 4-0)
        0xB0 => InstructionAnalysis {
            length: 1,
            description: "rtrue (return true)".to_string(),
        },
        0xB1 => InstructionAnalysis {
            length: 1,
            description: "rfalse (return false)".to_string(),
        },
        0xBB => InstructionAnalysis {
            length: 1,
            description: "new_line (print newline)".to_string(),
        },

        // 1OP instructions
        0x80..=0x8F => {
            let opcode = byte & 0x0F;
            InstructionAnalysis {
                length: 2, // Assuming 1-byte operand for simplicity
                description: format!("1OP:{} ({})", opcode, get_1op_name(opcode)),
            }
        }

        // 2OP instructions
        0x00..=0x1F => {
            let opcode = byte & 0x1F;
            InstructionAnalysis {
                length: 3, // Assuming 2 operands for simplicity
                description: format!("2OP:{} ({})", opcode, get_2op_name(opcode)),
            }
        }

        // VAR instructions
        0xE0..=0xFF => {
            let opcode = byte & 0x1F;
            InstructionAnalysis {
                length: 2, // Variable length, but start with 2
                description: format!("VAR:{} ({})", opcode, get_var_name(opcode)),
            }
        }

        _ => InstructionAnalysis {
            length: 1,
            description: format!("UNKNOWN: 0x{:02x}", byte),
        },
    }
}

fn get_1op_name(opcode: u8) -> &'static str {
    match opcode {
        0x00 => "jz",           // CORRECTED: jz is 1OP:0, not 1OP:1
        0x01 => "get_sibling",  // CORRECTED: was incorrectly mapped to jz
        0x02 => "get_child",    // CORRECTED: was incorrectly get_sibling
        0x03 => "get_parent",   // CORRECTED: was incorrectly get_child
        0x04 => "get_prop_len", // CORRECTED: was incorrectly get_parent
        0x05 => "inc",          // CORRECTED: was incorrectly get_prop_len
        0x06 => "dec",          // CORRECTED: was incorrectly inc
        0x07 => "print_addr",   // CORRECTED: was incorrectly dec
        0x08 => "call_1s",      // CORRECTED: was incorrectly print_addr
        0x09 => "remove_obj",   // CORRECTED: was incorrectly call_1s
        0x0A => "print_obj",    // CORRECTED: was incorrectly remove_obj
        0x0B => "ret",          // CORRECTED: was incorrectly print_obj
        0x0C => "jump",         // CORRECTED: was incorrectly ret
        0x0D => "print_paddr",  // CORRECTED: was incorrectly jump
        0x0E => "load",         // CORRECTED: was incorrectly print_paddr
        0x0F => "not",          // CORRECTED: was duplicate load
        _ => "unknown_1op",
    }
}

fn get_2op_name(opcode: u8) -> &'static str {
    match opcode {
        0x01 => "je",
        0x02 => "jl",
        0x03 => "jg",
        0x04 => "dec_chk",
        0x05 => "inc_chk",
        0x06 => "jin",
        0x07 => "test",
        0x08 => "or",
        0x09 => "and",
        0x0A => "test_attr",
        0x0B => "set_attr",
        0x0C => "clear_attr",
        0x0D => "store",
        0x0E => "insert_obj",
        0x0F => "loadw",
        0x10 => "loadb",
        0x11 => "get_prop",
        0x12 => "get_prop_addr",
        0x13 => "get_next_prop",
        0x14 => "add",
        0x15 => "sub",
        0x16 => "mul",
        0x17 => "div",
        0x18 => "mod",
        _ => "unknown_2op",
    }
}

fn get_var_name(opcode: u8) -> &'static str {
    match opcode {
        0x00 => "call",
        0x01 => "storew",
        0x02 => "storeb",
        0x03 => "put_prop",
        0x04 => "sread",
        0x05 => "print_char",
        0x06 => "print_num",
        0x07 => "random",
        0x08 => "push",
        0x09 => "pull",
        0x0A => "split_window",
        0x0B => "set_window",
        0x0C => "call_vs2",
        0x0D => "erase_window",
        _ => "unknown_var",
    }
}
