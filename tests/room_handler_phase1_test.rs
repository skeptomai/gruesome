/// Phase 1 validation test for room handler implementation
/// Verifies that room handlers are converted to functions correctly
use gruesome::grue_compiler::GrueCompiler;

#[test]
fn test_room_handlers_converted_to_functions() {
    let source = r#"
        world {
            room test_room "Test Room" {
                desc: "A test room."

                on_enter: {
                    print("Entered!");
                }

                on_exit: {
                    print("Exited!");
                }

                on_look: {
                    print("Looking!");
                }
            }
        }

        init {
            player.location = test_room;
            print("Ready");
        }
    "#;

    let compiler = GrueCompiler::new();
    let result = compiler.compile_to_ir(source);

    assert!(result.is_ok(), "Compilation should succeed");

    let ir = result.unwrap();

    // Verify room exists
    assert_eq!(ir.rooms.len(), 1, "Should have exactly 1 room");
    let room = &ir.rooms[0];
    assert_eq!(room.name, "test_room");

    // Verify handlers are function IDs (not blocks)
    assert!(room.on_enter.is_some(), "Room should have on_enter handler");
    assert!(room.on_exit.is_some(), "Room should have on_exit handler");
    assert!(room.on_look.is_some(), "Room should have on_look handler");

    let on_enter_id = room.on_enter.unwrap();
    let on_exit_id = room.on_exit.unwrap();
    let on_look_id = room.on_look.unwrap();

    // Verify these IDs reference actual functions
    let on_enter_func = ir.functions.iter().find(|f| f.id == on_enter_id);
    let on_exit_func = ir.functions.iter().find(|f| f.id == on_exit_id);
    let on_look_func = ir.functions.iter().find(|f| f.id == on_look_id);

    assert!(
        on_enter_func.is_some(),
        "on_enter ID should reference a function"
    );
    assert!(
        on_exit_func.is_some(),
        "on_exit ID should reference a function"
    );
    assert!(
        on_look_func.is_some(),
        "on_look ID should reference a function"
    );

    // Verify function names follow naming convention
    let on_enter_func = on_enter_func.unwrap();
    let on_exit_func = on_exit_func.unwrap();
    let on_look_func = on_look_func.unwrap();

    assert_eq!(
        on_enter_func.name, "test_room__on_enter",
        "on_enter function name should follow convention"
    );
    assert_eq!(
        on_exit_func.name, "test_room__on_exit",
        "on_exit function name should follow convention"
    );
    assert_eq!(
        on_look_func.name, "test_room__on_look",
        "on_look function name should follow convention"
    );

    // Verify functions have no parameters (room handlers don't take arguments)
    assert_eq!(
        on_enter_func.parameters.len(),
        0,
        "on_enter should have no parameters"
    );
    assert_eq!(
        on_exit_func.parameters.len(),
        0,
        "on_exit should have no parameters"
    );
    assert_eq!(
        on_look_func.parameters.len(),
        0,
        "on_look should have no parameters"
    );

    // Verify function bodies are not empty (contain the print statements)
    assert!(
        !on_enter_func.body.instructions.is_empty(),
        "on_enter should have instructions"
    );
    assert!(
        !on_exit_func.body.instructions.is_empty(),
        "on_exit should have instructions"
    );
    assert!(
        !on_look_func.body.instructions.is_empty(),
        "on_look should have instructions"
    );
}

#[test]
fn test_room_without_handlers_has_none() {
    let source = r#"
        world {
            room simple_room "Simple Room" {
                desc: "A simple room with no handlers."
            }
        }

        init {
            player.location = simple_room;
            print("Ready");
        }
    "#;

    let compiler = GrueCompiler::new();
    let result = compiler.compile_to_ir(source);

    assert!(result.is_ok(), "Compilation should succeed");

    let ir = result.unwrap();

    // Verify room exists
    assert_eq!(ir.rooms.len(), 1, "Should have exactly 1 room");
    let room = &ir.rooms[0];

    // Verify all handlers are None
    assert!(
        room.on_enter.is_none(),
        "Room should have no on_enter handler"
    );
    assert!(
        room.on_exit.is_none(),
        "Room should have no on_exit handler"
    );
    assert!(
        room.on_look.is_none(),
        "Room should have no on_look handler"
    );
}

#[test]
fn test_mini_zork_handlers_created() {
    let source =
        std::fs::read_to_string("examples/mini_zork.grue").expect("Failed to read mini_zork.grue");

    let compiler = GrueCompiler::new();
    let result = compiler.compile_to_ir(&source);

    assert!(result.is_ok(), "mini_zork should compile successfully");

    let ir = result.unwrap();

    // Find behind_house room
    let behind_house = ir.rooms.iter().find(|r| r.name == "behind_house");
    assert!(behind_house.is_some(), "Should have behind_house room");
    let behind_house = behind_house.unwrap();

    // Verify on_look handler exists
    assert!(
        behind_house.on_look.is_some(),
        "behind_house should have on_look handler"
    );

    // Find the function
    let on_look_id = behind_house.on_look.unwrap();
    let on_look_func = ir.functions.iter().find(|f| f.id == on_look_id);
    assert!(on_look_func.is_some(), "on_look function should exist");
    assert_eq!(on_look_func.unwrap().name, "behind_house__on_look");
}
