fn main() {
    println!("=== Verifying packed address calculation ===\n");
    
    let packed = 0x2bbe;
    
    // For Z3, packed addresses are multiplied by 2
    let unpacked_z3 = packed * 2;
    
    println!("Packed address: 0x{:04x}", packed);
    println!("Unpacked (Z3): 0x{:04x}", unpacked_z3);
    println!();
    
    // The debug log said 0x2bbe unpacks to 0x577c
    println!("Debug log claims: 0x2bbe -> 0x577c");
    println!("Our calculation:  0x2bbe -> 0x{:04x}", unpacked_z3);
    
    if unpacked_z3 == 0x577c {
        println!("\n✓ Calculation matches!");
    } else {
        println!("\n✗ Calculation doesn't match!");
        println!("\nDifference: 0x{:04x} - 0x{:04x} = {}", 
                0x577c, unpacked_z3, 0x577c as i32 - unpacked_z3 as i32);
    }
}