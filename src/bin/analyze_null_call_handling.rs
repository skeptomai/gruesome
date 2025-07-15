use log::debug;

fn main() {
    println!("=== Analyzing NULL Call Handling ===\n");
    
    println!("From the debug log:");
    println!("1. Call to packed address 0x0000 (unpacked: 0000) with store_var = Some(4)");
    println!("2. Our do_call() correctly returns 0 to variable 4");
    println!("3. Then: Call to packed address 0x4552 (unpacked: 8aa4) with store_var = Some(4)");
    println!();
    
    println!("This suggests the game code structure is:");
    println!("  result = CALL property_17_handler(args)  // Returns 0 due to NULL");
    println!("  if result == 0:");
    println!("    result = CALL fallback_handler(args)  // 0x4552");
    println!();
    
    println!("The fallback handler at 0x8aa4:");
    println!("- Checks property 29 of object 180 (West of House)");
    println!("- Calls print_paddr with 0x4e1c");
    println!();
    
    println!("The issue is that property 29 contains 0x4e,");
    println!("and when the game processes this, it ends up");
    println!("printing from the wrong address.");
    println!();
    
    println!("=== Key Insight ===");
    println!("The bug is NOT in NULL call handling.");
    println!("The bug is in how we handle dictionary type 0x32 entries");
    println!("that leads to this code path being executed.");
    println!();
    println!("Normal 'w' should NOT go through property 17 checks!");
    println!("Type 0x13 directions (like 'n', 'e') work correctly.");
    println!("Type 0x32 directions need different handling.");
}