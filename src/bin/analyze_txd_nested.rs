use std::env;
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <txd_routines.txt>", args[0]);
        std::process::exit(1);
    }
    
    let txd_file = &args[1];
    let content = fs::read_to_string(txd_file)?;
    
    // Parse TXD routines
    let mut txd_routines: Vec<u32> = Vec::new();
    for line in content.lines() {
        let line = line.trim();
        if !line.is_empty() {
            if let Ok(addr) = u32::from_str_radix(line, 16) {
                txd_routines.push(addr);
            }
        }
    }
    
    println!("TXD found {} routines", txd_routines.len());
    
    // Check our specific nested routines
    let nested_cases = vec![
        (0x0d198, 0x0d184, "d198 in d184"),
        (0x0d6f4, 0x0d6e8, "d6f4 in d6e8"), 
        (0x0e6f8, 0x0e6e8, "e6f8 in e6e8"),
        (0x0e96c, 0x0e960, "e96c in e960"),
        (0x25564, 0x25550, "25564 in 25550"),
        (0x2b3b4, 0x2b384, "2b3b4 in 2b384"),
    ];
    
    println!("\n=== NESTED ROUTINES IN TXD ===");
    for (nested, parent, desc) in nested_cases {
        let nested_in_txd = txd_routines.contains(&nested);
        let parent_in_txd = txd_routines.contains(&parent);
        
        println!("{:20} nested={:5} parent={:5}", 
                 desc, 
                 if nested_in_txd { "YES" } else { "NO" },
                 if parent_in_txd { "YES" } else { "NO" });
    }
    
    // Also check 0cafc (our alternate entry point)
    println!("\n=== ALTERNATE ENTRY POINT ===");
    let cafc_in_txd = txd_routines.contains(&0x0cafc);
    let caf4_in_txd = txd_routines.contains(&0x0caf4);
    println!("cafc in caf4         nested={:5} parent={:5}", 
             if cafc_in_txd { "YES" } else { "NO" },
             if caf4_in_txd { "YES" } else { "NO" });
    
    Ok(())
}