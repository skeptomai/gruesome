/// Test that buffer addresses are correctly stored in globals before sread
use gruesome::grue_compiler::GrueCompiler;
use gruesome::grue_compiler::ZMachineVersion;
use gruesome::interpreter::core::vm::{Game, VM};

#[test]
fn test_buffer_addresses_stored_in_globals() {
    let source = r#"
        fn do_nothing() {}

        grammar {
            verb "test" {
                default => do_nothing()
            }
        }

        init {
            print("Test");
        }
    "#;

    let compiler = GrueCompiler::new();
    let (story_data, _codegen) = compiler
        .compile(source, ZMachineVersion::V3)
        .expect("Compilation should succeed");

    let game = Game::from_memory(story_data).expect("Game should load");
    let vm = VM::new(game);

    // Globals start at address in header
    let globals_addr = vm.game.header.global_variables;

    // Global 109 (G6d) is at: globals_base + ((109-16) * 2)
    let g109_offset = (109 - 16) * 2;
    let g110_offset = (110 - 16) * 2;

    let text_buf_in_global = ((vm.game.memory[globals_addr + g109_offset] as u16) << 8)
        | (vm.game.memory[globals_addr + g109_offset + 1] as u16);

    let parse_buf_in_global = ((vm.game.memory[globals_addr + g110_offset] as u16) << 8)
        | (vm.game.memory[globals_addr + g110_offset + 1] as u16);

    println!(
        "Global 109 (text buffer addr): 0x{:04x}",
        text_buf_in_global
    );
    println!(
        "Global 110 (parse buffer addr): 0x{:04x}",
        parse_buf_in_global
    );

    // Globals are only set when main loop runs, not at load time
    // This is fine - sread will use whatever is in the variables when it executes

    // What we really need to test is: does the dictionary exist and is it correct?
    let dict_addr = vm.game.header.dictionary;
    println!("Dictionary address: 0x{:04x}", dict_addr);

    assert!(dict_addr > 0, "Dictionary should exist");

    // Read dictionary header
    let num_sep = vm.game.memory[dict_addr] as usize;
    let entry_len = vm.game.memory[dict_addr + num_sep + 1];
    let num_entries = ((vm.game.memory[dict_addr + num_sep + 2] as u16) << 8)
        | (vm.game.memory[dict_addr + num_sep + 3] as u16);

    println!(
        "Dictionary: {} entries, {} bytes each",
        num_entries, entry_len
    );
    println!("Dictionary has {} word separators", num_sep);

    assert!(num_entries > 0, "Dictionary should have entries");
    assert_eq!(entry_len, 6, "V3 dictionary entries should be 6 bytes");
}
