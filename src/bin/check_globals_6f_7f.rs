fn main() {
    println!("=== Global Variable Confusion ===\n");
    
    println!("From debug log:");
    println!("- At PC 04f8b: store #007f, #0004");
    println!("  This stores 4 into global 0x7f (127)");
    println!();
    
    println!("From routine 0x577c disassembly:");
    println!("- At 0x57f7: GET_PROP G6f,#11");
    println!("  This reads from global 0x6f (111)");
    println!();
    
    println!("WAIT! Let me check the disassembly notation...");
    println!("In some disassemblers:");
    println!("- G6f might mean 'Global 6f' (hex) = Global 111");
    println!("- Or it might mean something else");
    println!();
    
    println!("Actually, looking at 0x5805:");
    println!("- GET_PARENT G6f -> -(SP)");
    println!();
    println!("And from debug log we see:");
    println!("- get_parent: obj_num=4 at PC 05805");
    println!();
    println!("So G6f must contain object 4!");
    println!();
    println!("The question is: which global variable is G6f?");
    println!("It could be global 0x6f (111) or 0x7f (127)...");
}