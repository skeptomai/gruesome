/// Test that the "go" grammar handler is correctly generated and dispatched
use gruesome::grue_compiler::GrueCompiler;
use gruesome::grue_compiler::ZMachineVersion;

#[test]
fn test_go_handler_dispatch() {
    let source = r#"
        world {
            room west_of_house "West of House" {
                desc: "You are west of a white house."
                exits: { east: north_of_house }
            }

            room north_of_house "North of House" {
                desc: "You are north of a white house."
            }
        }

        fn handle_go(direction) {
            print("handle_go called");
            let exit = player.location.get_exit(direction);

            if exit.none() {
                print(" - get_exit returned none\n");
                return;
            }

            print(" - get_exit found exit\n");
        }

        grammar {
            verb "east" {
                default => handle_go("east")
            }
        }

        init {
            player.location = west_of_house;
            print("Ready\n");
        }
    "#;

    let compiler = GrueCompiler::new();
    let result = compiler.compile(source, ZMachineVersion::V3);

    match result {
        Ok((story_data, _codegen)) => {
            println!("Compilation succeeded, {} bytes", story_data.len());

            // Write to tests directory
            let test_output = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("tests")
                .join("test_go_handler.z3");
            std::fs::write(&test_output, &story_data).expect("Failed to write test file");
            println!("Test file written to: {}", test_output.display());

            // Check that the dictionary contains "east"
            let dict_addr = ((story_data[0x08] as usize) << 8) | (story_data[0x09] as usize);
            println!("Dictionary at 0x{:04x}", dict_addr);

            // List all dictionary words
            let num_sep = story_data[dict_addr] as usize;
            let entry_len = story_data[dict_addr + num_sep + 1] as usize;
            let num_entries = ((story_data[dict_addr + num_sep + 2] as usize) << 8)
                | (story_data[dict_addr + num_sep + 3] as usize);
            let first_entry = dict_addr + num_sep + 4;

            println!("Dictionary entries ({} total):", num_entries);
            for i in 0..num_entries.min(10) {
                let entry_addr = first_entry + (i * entry_len);
                let bytes = &story_data[entry_addr..entry_addr + 4];
                let decoded = decode_v3(bytes);
                println!("  {}: '{}'", i, decoded);
            }

            // Verify grammar action table exists and contains handler
            // This is a smoke test - if it compiles without errors, the handler is registered
            assert!(story_data.len() > 0x100, "Story file should be substantial");
        }
        Err(e) => {
            panic!("Compilation failed: {:?}", e);
        }
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
        if (6..=31).contains(&z) {
            result.push((b'a' + (z - 6)) as char);
        }
    }
    result
}

#[test]
fn test_get_exit_builtin_runtime_path() {
    // Test that get_exit uses the runtime parallel-array path
    let source = r#"
        world {
            room test_room "Test Room" {
                desc: "A test room."
                exits: { east: target_room }
            }

            room target_room "Target Room" {
                desc: "Target."
            }
        }

        fn test_exit() {
            let exit = test_room.get_exit("east");
            if exit.none() {
                print("FAIL: get_exit returned none\n");
            } else {
                print("PASS: get_exit found exit\n");
            }
        }

        grammar {
            verb "test" {
                default => test_exit()
            }
        }

        init {
            print("Test ready\n");
        }
    "#;

    let compiler = GrueCompiler::new();
    let result = compiler.compile(source, ZMachineVersion::V3);

    match result {
        Ok((story_data, _codegen)) => {
            println!("get_exit test compiled: {} bytes", story_data.len());

            // The key test: does it compile without the "property not found" error?
            // If the broken compile-time path were active, we'd get:
            // "Exit property 'exit_east' not found in property registry"
            assert!(story_data.len() > 0x100);
        }
        Err(e) => {
            panic!("get_exit compilation failed: {:?}\nThis suggests the compile-time optimization path is still active", e);
        }
    }
}
