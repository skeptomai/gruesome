fn main() {
    println!("=== Final Analysis: 'w' Command Bug ===\n");
    
    println!("FACTS:");
    println!("1. 'w' has dictionary type 0x32 (special handling)");
    println!("2. Other directions like 'n' have type 0x13 (simple)");
    println!("3. After parsing 'w', game calls routine at 0x2bbe (0x577c)");
    println!("4. That routine immediately checks property 17 of object 4");
    println!("5. Object 4's property 17 = 00 00 (no function)");
    println!("6. This causes a NULL call that returns 0");
    println!("7. Then execution continues, eventually printing garbage");
    println!();
    
    println!("THE CORE ISSUE:");
    println!("The game expects objects with action handlers to have");
    println!("valid function addresses in property 17. Object 4 doesn't.");
    println!();
    
    println!("WHY IT HAPPENS:");
    println!("Dictionary type 0x32 triggers a different code path");
    println!("that checks the actor's (player's) action handler.");
    println!("Type 0x13 entries use a simpler movement mechanism.");
    println!();
    
    println!("THE GARBAGE TEXT:");
    println!("After the failed property 17 check, the game");
    println!("continues with fallback logic that ends up");
    println!("printing from the wrong text address.");
}