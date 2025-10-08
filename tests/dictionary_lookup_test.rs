/// Test that dictionary lookup returns correct positions
use gruesome::grue_compiler::GrueCompiler;
use gruesome::grue_compiler::ZMachineVersion;

#[test]
fn test_dictionary_word_positions() {
    let source = r#"
        fn go_east() { print("east"); }
        fn go_west() { print("west"); }

        grammar {
            verb "east" { default => go_east() }
            verb "west" { default => go_west() }
        }

        init {
            print("Test");
        }
    "#;

    let compiler = GrueCompiler::new();
    let story_data = compiler
        .compile(source, ZMachineVersion::V3)
        .expect("Compilation should succeed");

    // Read dictionary from compiled game
    let dict_addr = ((story_data[0x08] as usize) << 8) | (story_data[0x09] as usize);
    let num_sep = story_data[dict_addr] as usize;
    let entry_len = story_data[dict_addr + num_sep + 1] as usize;
    let num_entries = ((story_data[dict_addr + num_sep + 2] as usize) << 8)
        | (story_data[dict_addr + num_sep + 3] as usize);
    let first_entry = dict_addr + num_sep + 4;

    println!("Dictionary at 0x{:04x}, {} entries", dict_addr, num_entries);

    // List all words
    for i in 0..num_entries {
        let entry_addr = first_entry + (i * entry_len);
        let bytes = &story_data[entry_addr..entry_addr + 4];
        let decoded = decode_v3(bytes);
        println!("  Position {}: 0x{:04x} = '{}'", i, entry_addr, decoded);
    }
}

fn decode_v3(bytes: &[u8]) -> String {
    let word1 = ((bytes[0] as u16) << 8) | (bytes[1] as u16);
    let word2 = ((bytes[2] as u16) << 8) | (bytes[3] as u16);

    let z0 = ((word1 >> 10) & 0x1F) as u8;
    let z1 = ((word1 >> 5) & 0x1F) as u8;
    let z2 = (word1 & 0x1F) as u8;
    let z3 = ((word2 >> 10) & 0x1F) as u8;
    let z4 = ((word2 >> 5) & 0x1F) as u8;
    let z5 = (word2 & 0x1F) as u8;

    let mut result = String::new();
    for z in [z0, z1, z2, z3, z4, z5] {
        if z == 5 {
            break;
        }
        if z >= 6 && z <= 31 {
            result.push((b'a' + (z - 6)) as char);
        }
    }
    result
}
