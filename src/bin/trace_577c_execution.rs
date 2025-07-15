fn main() {
    println!("=== Tracing execution at routine 0x577c ===\n");
    
    println!("From debug log:");
    println!("1. After parsing 'w' (type 0x32), game calls 0x2bbe (unpacked: 0x577c)");
    println!("2. IMMEDIATELY after entering 0x577c:");
    println!("   - get_property_addr: obj=4, prop=17");
    println!("   - Call to packed address 0x0000");
    println!();
    
    println!("But looking at disassembly of 0x577c:");
    println!("  0577c: test #0000, #0000 [FALSE RFALSE]");
    println!("  05781: unknown instructions...");
    println!();
    
    println!("The disassembly doesn't show a get_property_addr call!");
    println!();
    
    println!("This suggests:");
    println!("1. The routine at 0x577c might not be disassembled correctly");
    println!("2. OR the property check happens in a subroutine");
    println!("3. OR I'm misreading the execution flow");
    println!();
    
    println!("The actual property 17 check we see in disassembly is at:");
    println!("  05805: get_parent V7f -> V00");
    println!("  05808: get_prop V00, #0011 -> V00");  
    println!("  0580c: call V00, #0001 -> V04");
    println!();
    println!("This is checking property 17 of the PARENT of V7f (object 4)");
}