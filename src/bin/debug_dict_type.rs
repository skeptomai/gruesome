// This file contains debug code to add to the interpreter
// Copy these debug statements into the appropriate places in the interpreter

// For dictionary.rs in lookup_dictionary or parse_text:
/*
debug!("Dictionary lookup for '{}'", word);
let addr = // ... existing lookup code
if addr != 0 {
    // Read byte 4 (type byte)
    let type_byte = self.read_byte(addr as u32 + 4);
    let byte5 = self.read_byte(addr as u32 + 5);
    let byte6 = self.read_byte(addr as u32 + 6);
    
    debug!("  Found at {:04x}: type={:02x}, data={:02x} {:02x}", 
           addr, type_byte, byte5, byte6);
    
    if type_byte == 0x32 {
        debug!("  *** Type 0x32 dictionary entry! ***");
    }
}
*/

// For interpreter.rs where dictionary data is used:
/*
// When processing parsed words, check the dictionary entry type
let dict_addr = // ... from parse buffer
if dict_addr != 0 {
    let type_byte = self.vm.read_byte(dict_addr as u32 + 4);
    let byte5 = self.vm.read_byte(dict_addr as u32 + 5);
    let byte6 = self.vm.read_byte(dict_addr as u32 + 6);
    
    debug!("Processing dictionary entry at {:04x}: type={:02x}, data={:02x} {:02x}",
           dict_addr, type_byte, byte5, byte6);
    
    if type_byte == 0x32 {
        debug!("*** WARNING: Type 0x32 entry - special handling needed!");
        // The interpreter might be incorrectly using byte5/byte6 as action number
        let wrong_action = ((byte5 as u16) << 8) | byte6 as u16;
        debug!("*** If treated as action number: {:04x} = {}", wrong_action, wrong_action);
    }
}
*/

// For wherever action numbers are processed:
/*
debug!("Processing action number: {:04x}", action);
if action > 0x100 {
    debug!("*** WARNING: Suspiciously large action number! ***");
}
*/

fn main() {
    println!("Debug statements to add to the interpreter:");
    println!();
    println!("1. In dictionary.rs lookup_dictionary() after finding the word:");
    println!("   - Log the dictionary entry type byte (offset 4)");
    println!("   - Warn if type is 0x32");
    println!();
    println!("2. In the parser/interpreter where dictionary results are used:");
    println!("   - Log when processing dictionary entries");
    println!("   - Check the type byte before using data bytes");
    println!();
    println!("3. Wherever action numbers are used:");
    println!("   - Log the action number");
    println!("   - Warn if it's suspiciously large (> 0x100)");
    println!();
    println!("Key locations to check:");
    println!("- PERFORM routine (0x50a8) - how it interprets dictionary data");
    println!("- V-WALK routine (0x6f76) - how it handles movement");
    println!("- Any code that reads bytes 5-6 from dictionary entries");
}