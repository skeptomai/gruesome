use std::fs;

fn main() {
    let data = fs::read("mini_zork.z3").unwrap();
    
    // Focus on the problematic instruction at 0x141b
    let start = 0x141b;
    println!("Bytes at 0x{:04x}: {:02x} {:02x} {:02x} {:02x} {:02x} {:02x} {:02x} {:02x}", 
             start,
             data[start], data[start+1], data[start+2], data[start+3],
             data[start+4], data[start+5], data[start+6], data[start+7]);
    
    // Manually decode the Variable instruction
    let opcode_byte = data[start];
    println!("Opcode byte: 0x{:02x} (binary: {:08b})", opcode_byte, opcode_byte);
    println!("Form: {:02b} = Variable", opcode_byte >> 6);
    println!("Opcode: 0x{:02x}", opcode_byte & 0x3f);
    
    let operand_types_byte = data[start + 1];
    println!("Operand types byte: 0x{:02x} (binary: {:08b})", operand_types_byte, operand_types_byte);
    
    for i in 0..4 {
        let bits = (operand_types_byte >> (6 - i * 2)) & 0x03;
        let op_type = match bits {
            0b00 => "LargeConstant",
            0b01 => "SmallConstant", 
            0b10 => "Variable",
            0b11 => "Omitted",
            _ => "Unknown"
        };
        println!("Operand {}: {:02b} = {}", i, bits, op_type);
        if bits == 0b11 { break; }
    }
}
