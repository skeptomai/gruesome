fn main() {
    println!("Correcting memory region calculation:\n");
    
    let high_mem_start = 0x4e37;
    let static_mem_start = 0x2e53;
    
    let addr = 0x093bb;
    
    println!("Memory regions:");
    println!("- Dynamic: 0x0000 - 0x{:04x}", static_mem_start - 1);
    println!("- Static:  0x{:04x} - 0x{:04x}", static_mem_start, high_mem_start - 1);
    println!("- High:    0x{:04x} - end", high_mem_start);
    
    println!("\nAddress 0x{:04x} = {} decimal", addr, addr);
    
    if addr < static_mem_start {
        println!("This is in DYNAMIC memory");
    } else if addr < high_mem_start {
        println!("This is in STATIC memory");
    } else {
        println!("This is in HIGH memory (code)");
        println!("0x{:04x} >= 0x{:04x}, so this IS valid code memory!", addr, high_mem_start);
    }
    
    println!("\nActually, 0x093bb IS in the code section!");
    println!("I was wrong - this is a valid code address.");
    
    // But let's check the other addresses
    println!("\nChecking our problematic addresses:");
    
    for &check_addr in &[0x093bb, 0x0751f, 0x07554] {
        print!("0x{:05x}: ", check_addr);
        if check_addr < static_mem_start {
            println!("Dynamic memory");
        } else if check_addr < high_mem_start {
            println!("Static memory");  
        } else {
            println!("High memory (code) ✓");
        }
    }
    
    println!("\nAll three addresses are in code memory!");
    println!("So the issue is not about executing non-code memory.");
    println!("The issue is that we're executing code that branches to");
    println!("what appears to be data within the code section.");
}