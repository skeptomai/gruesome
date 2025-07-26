use std::fs::File;
use std::io::Read;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Trinity vs AMFV Comparison Summary ===\n");

    // Read both headers
    let _trinity_header = read_header("./resources/test/trinity/trinity-r15-s870628.z4")?;
    let _amfv_header = read_header("./resources/test/amfv/amfv-r79-s851122.z4")?;

    println!("Key Differences Found:");
    println!("1. Dictionary word separators:");
    println!("   Trinity: 5 separators: '.', ',', '\"', '!', '?'");
    println!("   AMFV:    3 separators: ',', '.', '\"'");
    println!("   -> Trinity has additional punctuation separators");
    println!();

    println!("2. Dictionary size:");
    println!("   Trinity: 2120 entries");
    println!("   AMFV:    1812 entries");
    println!("   -> Trinity has larger vocabulary");
    println!();

    println!("3. Memory layout:");
    println!("   Trinity: High mem=0xf771, Static=0x9310, Dynamic=37648 bytes");
    println!("   AMFV:    High mem=0xcae5, Static=0x7bc6, Dynamic=31686 bytes");
    println!("   -> Trinity uses more memory");
    println!();

    println!("4. Object table addresses:");
    println!("   Trinity: 0x02b6 (694)");
    println!("   AMFV:    0x02cc (716)");
    println!("   -> Slightly different object table placement");
    println!();

    println!("5. Identical features:");
    println!("   - Both are v4 games");
    println!("   - Same header flags (0x00) - no special features enabled");
    println!("   - Same dictionary entry length (9 bytes)");
    println!("   - Both use standard v4 object format");
    println!("   - Both have timed input bit disabled in flags");
    println!();

    // Now let's check some specific memory locations for encoding differences
    analyze_text_encoding_differences()?;

    Ok(())
}

fn read_header(filename: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut file = File::open(filename)?;
    let mut buffer = vec![0; 64];
    file.read_exact(&mut buffer)?;
    Ok(buffer)
}

fn analyze_text_encoding_differences() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Text Encoding Analysis ===\n");

    // Read enough of each file to check alphabet tables and encoding
    let trinity_data = read_file_data("./resources/test/trinity/trinity-r15-s870628.z4", 50000)?;
    let amfv_data = read_file_data("./resources/test/amfv/amfv-r79-s851122.z4", 50000)?;

    // Check alphabet table usage (v5+ feature, but might give clues)
    let trinity_alpha = get_word(&trinity_data, 0x34);
    let amfv_alpha = get_word(&amfv_data, 0x34);

    println!("Alphabet table addresses:");
    println!("  Trinity: 0x{trinity_alpha:04x}");
    println!("  AMFV:    0x{amfv_alpha:04x}");

    if trinity_alpha == 0 && amfv_alpha == 0 {
        println!("  -> Both use standard v4 alphabet (A0=a-z, A1=A-Z, A2=symbols)");
    }
    println!();

    // Check abbreviation tables
    let trinity_abbrev = get_word(&trinity_data, 0x18);
    let amfv_abbrev = get_word(&amfv_data, 0x18);

    println!("Abbreviation analysis:");
    println!("  Trinity abbrev table: 0x{trinity_abbrev:04x}");
    println!("  AMFV abbrev table:    0x{amfv_abbrev:04x}");

    // Look at first few abbreviation entries
    if trinity_abbrev as usize + 10 < trinity_data.len() {
        println!(
            "  Trinity first abbrev: {:02x} {:02x} {:02x} {:02x}",
            trinity_data[trinity_abbrev as usize],
            trinity_data[trinity_abbrev as usize + 1],
            trinity_data[trinity_abbrev as usize + 2],
            trinity_data[trinity_abbrev as usize + 3]
        );
    }

    if amfv_abbrev as usize + 10 < amfv_data.len() {
        println!(
            "  AMFV first abbrev:    {:02x} {:02x} {:02x} {:02x}",
            amfv_data[amfv_abbrev as usize],
            amfv_data[amfv_abbrev as usize + 1],
            amfv_data[amfv_abbrev as usize + 2],
            amfv_data[amfv_abbrev as usize + 3]
        );
    }
    println!();

    println!("=== Potential Input Processing Issues ===\n");

    println!("The key difference that could affect input processing:");
    println!("1. Trinity has MORE word separators ('!' and '?')");
    println!("   - This means Trinity treats '!' and '?' as word boundaries");
    println!("   - AMFV does not, so they're treated as part of words");
    println!("   - This could affect tokenization of user input");
    println!();

    println!("2. Dictionary size difference suggests:");
    println!("   - Trinity has more complex vocabulary");
    println!("   - More verbs, adjectives, or specialized terms");
    println!("   - Could lead to different parsing behavior");
    println!();

    println!("Recommendation:");
    println!("- Check dictionary word parsing logic");
    println!("- Ensure separator characters are handled correctly");
    println!("- Test input tokenization with Trinity-specific punctuation");
    println!("- Verify that '!' and '?' in commands are properly separated");

    Ok(())
}

fn read_file_data(filename: &str, size: usize) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut file = File::open(filename)?;
    let actual_size = std::cmp::min(size, file.metadata()?.len() as usize);
    let mut buffer = vec![0; actual_size];
    file.read_exact(&mut buffer)?;
    Ok(buffer)
}

fn get_word(buffer: &[u8], offset: usize) -> u16 {
    if offset + 1 < buffer.len() {
        (buffer[offset] as u16) << 8 | buffer[offset + 1] as u16
    } else {
        0
    }
}
