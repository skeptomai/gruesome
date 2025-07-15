fn main() {
    println!("=== Clarifying Global Variables ===\n");
    
    println!("Z-Machine variable numbering:");
    println!("- Variables 0x00-0x0F: Local variables");
    println!("- Variables 0x10-0xFF: Global variables");
    println!("- Variable 0x00: Stack");
    println!();
    
    println!("Variable to global mapping:");
    println!("- Variable 0x10 = Global 0 (G00)");
    println!("- Variable 0x7f = Global 0x6f (G6f) = Global 111");
    println!();
    
    println!("From debug log at PC 04f8b:");
    println!("  store #007f, #0004");
    println!("  This stores 4 into GLOBAL 0x7f (127), not variable 0x7f!");
    println!();
    
    println!("But the GET_PROP instruction at 0x57f7 uses:");
    println!("  Variable 0x7f = Global 0x6f (111)");
    println!();
    
    println!("WAIT! The store instruction syntax is:");
    println!("  store <variable>, <value>");
    println!("So 'store #007f, #0004' stores 4 into variable 0x7f");
    println!("Which is global 0x6f (111)!");
    println!();
    
    println!("So everything matches:");
    println!("1. Game stores 4 into variable 0x7f (global 111)");
    println!("2. GET_PROP reads from variable 0x7f (global 111)");
    println!("3. This contains object 4 (cretin with no action)");
}