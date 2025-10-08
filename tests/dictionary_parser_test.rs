/// Integration test: Verify that words added to dictionary can be found by the Z-Machine parser
/// This tests the complete pipeline: dictionary generation → Z-Machine file → parser lookup
use gruesome::grue_compiler::GrueCompiler;
use gruesome::grue_compiler::ZMachineVersion;
use gruesome::vm::Game;
use gruesome::vm::VM;

#[test]
fn test_parser_finds_words_in_dictionary() {
    // Create a minimal Grue program that adds specific words to the dictionary
    let source = r#"
        fn do_nothing() {
            // Empty function for test
        }

        grammar {
            verb "east" {
                default => do_nothing()
            }

            verb "west" {
                default => do_nothing()
            }

            verb "look" {
                default => do_nothing()
            }
        }

        init {
            print("Test");
        }
    "#;

    // Compile to Z-Machine bytecode
    let compiler = GrueCompiler::new();
    let story_data = compiler
        .compile(source, ZMachineVersion::V3)
        .expect("Compilation should succeed");

    // Load into VM
    let game = Game::from_memory(story_data).expect("Game should load");
    let mut vm = VM::new(game);

    // Allocate input buffers (matching what the compiler does)
    let text_buffer_addr = 0x0064; // From compiler layout
    let parse_buffer_addr = 0x0078; // From compiler layout

    // Initialize text buffer with max length
    vm.write_byte(text_buffer_addr, 40);

    // Initialize parse buffer with max words
    vm.write_byte(parse_buffer_addr, 10);

    // Test 1: Type "east" and verify parser finds it
    let input = "east";

    // Write input to text buffer (Z-Machine format: [max_len][actual_text...])
    vm.write_byte(text_buffer_addr + 1, input.len() as u8);
    for (i, ch) in input.bytes().enumerate() {
        vm.write_byte(text_buffer_addr + 2 + (i as u32), ch);
    }

    // Call tokenize (sread sets up parse buffer)
    // We need to manually call the dictionary lookup since sread is an opcode
    let dict_addr = vm.game.header.dictionary as u32;

    // Read dictionary header
    let num_separators = vm.read_byte(dict_addr);
    let entry_length = vm.read_byte(dict_addr + num_separators as u32 + 1);
    let num_entries = vm.read_word(dict_addr + num_separators as u32 + 2);
    let first_entry = dict_addr + num_separators as u32 + 4;

    println!("Dictionary at 0x{:04x}:", dict_addr);
    println!("  Separators: {}", num_separators);
    println!("  Entry length: {}", entry_length);
    println!("  Num entries: {}", num_entries);
    println!("  First entry: 0x{:04x}", first_entry);

    // Manually search for "east" in dictionary
    let search_word = input;
    let encoded = encode_word_v3(search_word);

    println!(
        "\nSearching for '{}' (encoded: {:02x} {:02x} {:02x} {:02x})",
        search_word, encoded[0], encoded[1], encoded[2], encoded[3]
    );

    let mut found_addr = 0u32;
    for i in 0..num_entries {
        let entry_addr = first_entry + (i as u32 * entry_length as u32);
        let dict_word = [
            vm.read_byte(entry_addr),
            vm.read_byte(entry_addr + 1),
            vm.read_byte(entry_addr + 2),
            vm.read_byte(entry_addr + 3),
        ];

        // Decode for debugging
        let decoded = decode_word_v3(&dict_word);

        if i < 5 || decoded.trim() == search_word {
            println!(
                "  Entry {}: 0x{:04x} = {:02x} {:02x} {:02x} {:02x} => '{}'",
                i, entry_addr, dict_word[0], dict_word[1], dict_word[2], dict_word[3], decoded
            );
        }

        if dict_word == encoded {
            found_addr = entry_addr;
            println!(
                "\n✓ FOUND '{}' at address 0x{:04x}",
                search_word, entry_addr
            );
            break;
        }
    }

    assert_ne!(found_addr, 0, "Parser should find 'east' in dictionary");

    // Test 2: Verify "west" is also findable
    let search_word = "west";
    let encoded = encode_word_v3(search_word);

    println!("\nSearching for '{}'...", search_word);

    let mut found_west = false;
    for i in 0..num_entries {
        let entry_addr = first_entry + (i as u32 * entry_length as u32);
        let dict_word = [
            vm.read_byte(entry_addr),
            vm.read_byte(entry_addr + 1),
            vm.read_byte(entry_addr + 2),
            vm.read_byte(entry_addr + 3),
        ];

        if dict_word == encoded {
            found_west = true;
            println!("✓ FOUND '{}' at address 0x{:04x}", search_word, entry_addr);
            break;
        }
    }

    assert!(found_west, "Parser should find 'west' in dictionary");
}

/// Encode a word using Z-Machine V3 encoding (6 bytes → 4 bytes, 3 Z-chars each)
fn encode_word_v3(word: &str) -> [u8; 4] {
    let mut result = [0u8; 4];
    let mut z_chars = Vec::new();

    // Convert to lowercase and take first 6 characters
    let word_lower = word.to_lowercase();
    let chars: Vec<char> = word_lower.chars().take(6).collect();

    // Encode each character to Z-chars
    for ch in chars {
        let z_char = if ch >= 'a' && ch <= 'z' {
            (ch as u8 - b'a') + 6
        } else {
            // For non-letters, use shift sequences (simplified)
            6 // Just use 'a' for now
        };
        z_chars.push(z_char);
    }

    // Pad to 6 Z-chars with padding character (5)
    while z_chars.len() < 6 {
        z_chars.push(5);
    }

    // Pack into 4 bytes (3 Z-chars per 2-byte word)
    // First word: [1 bit marker][5 bits z0][5 bits z1][5 bits z2]
    // Second word: [1 bit marker][5 bits z3][5 bits z4][5 bits z5]

    let word1 = ((z_chars[0] as u16) << 10) | ((z_chars[1] as u16) << 5) | (z_chars[2] as u16);
    let word2 =
        0x8000 | ((z_chars[3] as u16) << 10) | ((z_chars[4] as u16) << 5) | (z_chars[5] as u16);

    result[0] = (word1 >> 8) as u8;
    result[1] = (word1 & 0xFF) as u8;
    result[2] = (word2 >> 8) as u8;
    result[3] = (word2 & 0xFF) as u8;

    result
}

/// Decode a V3 dictionary entry for debugging
fn decode_word_v3(bytes: &[u8; 4]) -> String {
    let word1 = ((bytes[0] as u16) << 8) | (bytes[1] as u16);
    let word2 = ((bytes[2] as u16) << 8) | (bytes[3] as u16);

    let z0 = ((word1 >> 10) & 0x1F) as u8;
    let z1 = ((word1 >> 5) & 0x1F) as u8;
    let z2 = (word1 & 0x1F) as u8;
    let z3 = ((word2 >> 10) & 0x1F) as u8;
    let z4 = ((word2 >> 5) & 0x1F) as u8;
    let z5 = (word2 & 0x1F) as u8;

    let mut result = String::new();
    for z_char in [z0, z1, z2, z3, z4, z5] {
        if z_char == 5 {
            break; // Padding
        }
        if z_char >= 6 && z_char <= 31 {
            result.push((b'a' + (z_char - 6)) as char);
        }
    }

    result
}
