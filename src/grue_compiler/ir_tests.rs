// IR Generation Tests for Grue Language

#[cfg(test)]
mod ir_tests {
    use crate::grue_compiler::ast::*;
    use crate::grue_compiler::error::CompilerError;
    use crate::grue_compiler::ir::*;
    use crate::grue_compiler::lexer::Lexer;
    use crate::grue_compiler::parser::Parser;

    fn generate_ir_from_source(source: &str) -> Result<IrProgram, CompilerError> {
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize()?;
        let mut parser = Parser::new(tokens);
        let ast = parser.parse()?;
        let mut ir_generator = IrGenerator::new();
        ir_generator.generate(ast)
    }

    #[test]
    fn test_simple_function_ir() {
        let source = r#"
            fn greet() {
                print("Hello, World!");
            }
        "#;

        let ir = generate_ir_from_source(source).unwrap();

        assert_eq!(ir.functions.len(), 1);
        let func = &ir.functions[0];
        assert_eq!(func.name, "greet");
        assert_eq!(func.parameters.len(), 0);
        assert!(func.return_type.is_none());

        // Should have at least one instruction (the print call)
        assert!(!func.body.instructions.is_empty());
    }

    #[test]
    fn test_function_with_parameters() {
        let source = r#"
            fn add(a: int, b: int) -> int {
                return a + b;
            }
        "#;

        let ir = generate_ir_from_source(source).unwrap();

        assert_eq!(ir.functions.len(), 1);
        let func = &ir.functions[0];
        assert_eq!(func.name, "add");
        assert_eq!(func.parameters.len(), 2);
        assert_eq!(func.parameters[0].name, "a");
        assert_eq!(func.parameters[0].param_type, Some(Type::Int));
        assert_eq!(func.parameters[1].name, "b");
        assert_eq!(func.parameters[1].param_type, Some(Type::Int));
        assert_eq!(func.return_type, Some(Type::Int));

        // Should have instructions for the binary operation and return
        let instructions = &func.body.instructions;
        assert!(!instructions.is_empty());

        // Last instruction should be Return
        if let Some(IrInstruction::Return { value: Some(_) }) = instructions.last() {
            // Success - return statement found
        } else {
            panic!("Expected Return instruction at end of function");
        }
    }

    #[test]
    fn test_binary_expression_ir() {
        let source = r#"
            fn test_expr() {
                let x = 5 + 3;
            }
        "#;

        let ir = generate_ir_from_source(source).unwrap();

        assert_eq!(ir.functions.len(), 1);
        let func = &ir.functions[0];
        let instructions = &func.body.instructions;

        // Should contain LoadImmediate instructions for 5 and 3
        let load_count = instructions
            .iter()
            .filter(|inst| matches!(inst, IrInstruction::LoadImmediate { .. }))
            .count();
        assert!(load_count >= 2); // At least two loads for the constants

        // Should contain a BinaryOp instruction for addition
        let binary_ops = instructions
            .iter()
            .filter(|inst| {
                matches!(
                    inst,
                    IrInstruction::BinaryOp {
                        op: IrBinaryOp::Add,
                        ..
                    }
                )
            })
            .count();
        assert!(binary_ops >= 1); // At least one add operation
    }

    #[test]
    fn test_world_with_rooms_ir() {
        let source = r#"
            world {
                room west_house "West of House" {
                    desc: "You are standing in an open field."
                    exits: {
                        north: north_house
                    }
                }
                
                room north_house "North of House" {
                    desc: "You are facing the north side of a house."
                    exits: {
                        south: west_house
                    }
                }
            }
        "#;

        let ir = generate_ir_from_source(source).unwrap();

        assert_eq!(ir.rooms.len(), 2);

        let west_house = ir.rooms.iter().find(|r| r.name == "west_house").unwrap();
        assert_eq!(west_house.display_name, "West of House");
        assert_eq!(west_house.description, "You are standing in an open field.");
        assert!(west_house.exits.contains_key("north"));

        let north_house = ir.rooms.iter().find(|r| r.name == "north_house").unwrap();
        assert_eq!(north_house.display_name, "North of House");
        assert_eq!(
            north_house.description,
            "You are facing the north side of a house."
        );
        assert!(north_house.exits.contains_key("south"));
    }

    #[test]
    fn test_grammar_ir() {
        let source = r#"
            fn handle_look() {
                print("You look around.");
            }
            
            grammar {
                verb "look" {
                    default => handle_look()
                }
            }
        "#;

        let ir = generate_ir_from_source(source).unwrap();

        assert_eq!(ir.functions.len(), 1);
        assert_eq!(ir.grammar.len(), 1);

        let grammar = &ir.grammar[0];
        assert_eq!(grammar.verb, "look");
        assert_eq!(grammar.patterns.len(), 1);

        let pattern = &grammar.patterns[0];
        assert_eq!(pattern.pattern.len(), 1);
        assert!(matches!(pattern.pattern[0], IrPatternElement::Default));

        // Handler should be a function call (though function ID resolution is placeholder for now)
        assert!(matches!(pattern.handler, IrHandler::FunctionCall(_, _)));
    }

    #[test]
    fn test_init_block_ir() {
        let source = r#"
            init {
                print("Game initialized!");
                let start_time = 0;
            }
        "#;

        let ir = generate_ir_from_source(source).unwrap();

        assert!(ir.init_block.is_some());
        let init_block = ir.init_block.unwrap();
        assert!(!init_block.instructions.is_empty());
    }

    #[test]
    fn test_literals_ir() {
        let source = r#"
            fn test_literals() {
                let num = 42;
                let text = "hello";
                let flag = true;
            }
        "#;

        let ir = generate_ir_from_source(source).unwrap();

        assert_eq!(ir.functions.len(), 1);
        let func = &ir.functions[0];
        let instructions = &func.body.instructions;

        // Check for LoadImmediate instructions with correct values
        let mut found_int = false;
        let mut found_string = false;
        let mut found_bool = false;

        for inst in instructions {
            if let IrInstruction::LoadImmediate { target: _, value } = inst {
                match value {
                    IrValue::Integer(42) => found_int = true,
                    IrValue::String(s) if s == "hello" => found_string = true,
                    IrValue::Boolean(true) => found_bool = true,
                    _ => {}
                }
            }
        }

        assert!(found_int, "Should generate LoadImmediate for integer 42");
        assert!(
            found_string,
            "Should generate LoadImmediate for string 'hello'"
        );
        assert!(found_bool, "Should generate LoadImmediate for boolean true");
    }

    #[test]
    fn test_unary_expression_ir() {
        let source = r#"
            fn test_unary() {
                let neg = -5;
                let not_flag = !true;
            }
        "#;

        let ir = generate_ir_from_source(source).unwrap();

        assert_eq!(ir.functions.len(), 1);
        let func = &ir.functions[0];
        let instructions = &func.body.instructions;

        // Should contain UnaryOp instructions
        let unary_ops: Vec<_> = instructions
            .iter()
            .filter_map(|inst| {
                if let IrInstruction::UnaryOp { op, .. } = inst {
                    Some(op)
                } else {
                    None
                }
            })
            .collect();

        assert!(
            unary_ops.contains(&&IrUnaryOp::Minus),
            "Should have unary minus"
        );
        assert!(
            unary_ops.contains(&&IrUnaryOp::Not),
            "Should have unary not"
        );
    }

    #[test]
    fn test_comparison_operators_ir() {
        let source = r#"
            fn test_comparisons() {
                let a = 5 < 10;
                let b = 3 == 3;
                let c = 7 > 2;
            }
        "#;

        let ir = generate_ir_from_source(source).unwrap();

        assert_eq!(ir.functions.len(), 1);
        let func = &ir.functions[0];
        let instructions = &func.body.instructions;

        // Should contain BinaryOp instructions with comparison operators
        let binary_ops: Vec<_> = instructions
            .iter()
            .filter_map(|inst| {
                if let IrInstruction::BinaryOp { op, .. } = inst {
                    Some(op)
                } else {
                    None
                }
            })
            .collect();

        assert!(
            binary_ops.contains(&&IrBinaryOp::Less),
            "Should have less than"
        );
        assert!(
            binary_ops.contains(&&IrBinaryOp::Equal),
            "Should have equality"
        );
        assert!(
            binary_ops.contains(&&IrBinaryOp::Greater),
            "Should have greater than"
        );
    }

    #[test]
    fn test_return_statement_ir() {
        let source = r#"
            fn returns_value() -> int {
                return 42;
            }
            
            fn returns_nothing() {
                return;
            }
        "#;

        let ir = generate_ir_from_source(source).unwrap();

        assert_eq!(ir.functions.len(), 2);

        // Check function with return value
        let returns_value = ir
            .functions
            .iter()
            .find(|f| f.name == "returns_value")
            .unwrap();
        let return_with_value = returns_value
            .body
            .instructions
            .iter()
            .find(|inst| matches!(inst, IrInstruction::Return { value: Some(_) }));
        assert!(return_with_value.is_some(), "Should have return with value");

        // Check function with void return
        let returns_nothing = ir
            .functions
            .iter()
            .find(|f| f.name == "returns_nothing")
            .unwrap();
        let return_void = returns_nothing
            .body
            .instructions
            .iter()
            .find(|inst| matches!(inst, IrInstruction::Return { value: None }));
        assert!(return_void.is_some(), "Should have void return");
    }

    #[test]
    fn test_complex_program_ir() {
        let source = r#"
            fn calculate(x: int, y: int) -> int {
                return x * y + 10;
            }

            world {
                room test_room "Test Room" {
                    desc: "A simple test room."
                }
            }

            init {
                let result = calculate(5, 3);
                print("Initialized");
            }
        "#;

        let ir = generate_ir_from_source(source).unwrap();

        // Verify all components are present
        assert_eq!(ir.functions.len(), 1);
        assert_eq!(ir.rooms.len(), 1);
        assert!(ir.init_block.is_some());

        // Verify function
        let func = &ir.functions[0];
        assert_eq!(func.name, "calculate");
        assert_eq!(func.parameters.len(), 2);
        assert_eq!(func.return_type, Some(Type::Int));

        // Verify room
        let room = &ir.rooms[0];
        assert_eq!(room.name, "test_room");
        assert_eq!(room.display_name, "Test Room");

        // Verify init block exists and has instructions
        let init_block = ir.init_block.as_ref().unwrap();
        assert!(!init_block.instructions.is_empty());
    }

    #[test]
    fn test_ir_id_generation() {
        let source = r#"
            fn test() {
                let a = 1;
                let b = 2;
            }
        "#;

        let ir = generate_ir_from_source(source).unwrap();

        assert_eq!(ir.functions.len(), 1);
        let func = &ir.functions[0];

        // Function should have a non-zero ID
        assert!(func.id > 0);

        // Block should have a non-zero ID
        assert!(func.body.id > 0);

        // IDs should be different
        assert_ne!(func.id, func.body.id);
    }

    #[test]
    fn test_if_statement_ir() {
        let source = r#"
            fn test_if() {
                if true {
                    print("true branch");
                } else {
                    print("false branch");
                }
            }
        "#;

        let ir = generate_ir_from_source(source).unwrap();

        assert_eq!(ir.functions.len(), 1);
        let func = &ir.functions[0];
        let instructions = &func.body.instructions;

        // Should contain Branch instruction
        let branches = instructions
            .iter()
            .filter(|inst| matches!(inst, IrInstruction::Branch { .. }))
            .count();
        assert!(branches >= 1, "Should have at least one branch instruction");

        // Should contain Label instructions for control flow
        let labels = instructions
            .iter()
            .filter(|inst| matches!(inst, IrInstruction::Label { .. }))
            .count();
        assert!(labels >= 3, "Should have labels for then, else, and end");
    }

    #[test]
    fn test_while_loop_ir() {
        let source = r#"
            fn test_while() {
                let i = 0;
                while i < 10 {
                    i = i + 1;
                }
            }
        "#;

        let ir = generate_ir_from_source(source).unwrap();

        assert_eq!(ir.functions.len(), 1);
        let func = &ir.functions[0];
        let instructions = &func.body.instructions;

        // Should contain Branch instruction for loop condition
        let branches = instructions
            .iter()
            .filter(|inst| matches!(inst, IrInstruction::Branch { .. }))
            .count();
        assert!(
            branches >= 1,
            "Should have branch instruction for loop condition"
        );

        // Should contain Jump instructions for loop control
        let jumps = instructions
            .iter()
            .filter(|inst| matches!(inst, IrInstruction::Jump { .. }))
            .count();
        assert!(
            jumps >= 1,
            "Should have jump instruction for loop iteration"
        );
    }

    #[test]
    fn test_assignment_statement_ir() {
        let source = r#"
            fn test_assignment() {
                let x = 5;
                x = 10;
            }
        "#;

        let ir = generate_ir_from_source(source).unwrap();

        assert_eq!(ir.functions.len(), 1);
        let func = &ir.functions[0];
        let instructions = &func.body.instructions;

        // Should contain StoreVar instructions for both declaration and assignment
        let stores = instructions
            .iter()
            .filter(|inst| matches!(inst, IrInstruction::StoreVar { .. }))
            .count();
        assert!(
            stores >= 2,
            "Should have store instructions for declaration and assignment"
        );
    }

    #[test]
    fn test_ternary_expression_ir() {
        let source = r#"
            fn test_ternary() {
                let result = true ? "yes" : "no";
            }
        "#;

        let ir = generate_ir_from_source(source).unwrap();

        assert_eq!(ir.functions.len(), 1);
        let func = &ir.functions[0];
        let instructions = &func.body.instructions;

        // Should contain Branch instruction for ternary condition
        let branches = instructions
            .iter()
            .filter(|inst| matches!(inst, IrInstruction::Branch { .. }))
            .count();
        assert!(
            branches >= 1,
            "Should have branch instruction for ternary condition"
        );

        // Should contain labels for true and false branches
        let labels = instructions
            .iter()
            .filter(|inst| matches!(inst, IrInstruction::Label { .. }))
            .count();
        assert!(
            labels >= 3,
            "Should have labels for true, false, and end branches"
        );
    }

    #[test]
    fn test_for_loop_ir() {
        let source = r#"
            fn test_for() {
                let items = [1, 2, 3];
                for item in items {
                    print("Item: " + item);
                }
            }
        "#;

        let ir = generate_ir_from_source(source).unwrap();

        assert_eq!(ir.functions.len(), 1);
        let func = &ir.functions[0];
        let instructions = &func.body.instructions;

        // Should contain GetArrayElement instruction for iterating
        let array_gets = instructions
            .iter()
            .filter(|inst| matches!(inst, IrInstruction::GetArrayElement { .. }))
            .count();
        assert!(array_gets >= 1, "Should have GetArrayElement instruction");

        // Should contain loop control instructions
        let branches = instructions
            .iter()
            .filter(|inst| matches!(inst, IrInstruction::Branch { .. }))
            .count();
        assert!(
            branches >= 1,
            "Should have branch instruction for loop condition"
        );
    }

    // Tests for room object containment foundation (Phase 1a)
    #[test]
    fn test_room_object_info_creation() {
        // Test basic RoomObjectInfo creation
        let mailbox = RoomObjectInfo {
            name: "mailbox".to_string(),
            nested_objects: vec![],
        };

        assert_eq!(mailbox.name, "mailbox");
        assert!(mailbox.nested_objects.is_empty());
    }

    #[test]
    fn test_room_object_info_with_nested_objects() {
        // Test RoomObjectInfo with nested objects (like leaflet inside mailbox)
        let leaflet = RoomObjectInfo {
            name: "leaflet".to_string(),
            nested_objects: vec![],
        };

        let mailbox = RoomObjectInfo {
            name: "mailbox".to_string(),
            nested_objects: vec![leaflet],
        };

        assert_eq!(mailbox.name, "mailbox");
        assert_eq!(mailbox.nested_objects.len(), 1);
        assert_eq!(mailbox.nested_objects[0].name, "leaflet");
        assert!(mailbox.nested_objects[0].nested_objects.is_empty());
    }

    #[test]
    fn test_room_object_info_deep_nesting() {
        // Test deep nesting: chest contains box, box contains key
        let key = RoomObjectInfo {
            name: "key".to_string(),
            nested_objects: vec![],
        };

        let box_obj = RoomObjectInfo {
            name: "box".to_string(),
            nested_objects: vec![key],
        };

        let chest = RoomObjectInfo {
            name: "chest".to_string(),
            nested_objects: vec![box_obj],
        };

        assert_eq!(chest.name, "chest");
        assert_eq!(chest.nested_objects.len(), 1);
        assert_eq!(chest.nested_objects[0].name, "box");
        assert_eq!(chest.nested_objects[0].nested_objects.len(), 1);
        assert_eq!(chest.nested_objects[0].nested_objects[0].name, "key");
        assert!(chest.nested_objects[0].nested_objects[0]
            .nested_objects
            .is_empty());
    }

    #[test]
    fn test_room_object_info_multiple_objects_same_level() {
        // Test multiple objects at same level (mailbox and tree in same room)
        let leaflet = RoomObjectInfo {
            name: "leaflet".to_string(),
            nested_objects: vec![],
        };

        let mailbox = RoomObjectInfo {
            name: "mailbox".to_string(),
            nested_objects: vec![leaflet],
        };

        let tree = RoomObjectInfo {
            name: "tree".to_string(),
            nested_objects: vec![],
        };

        let room_objects = vec![mailbox, tree];

        assert_eq!(room_objects.len(), 2);
        assert_eq!(room_objects[0].name, "mailbox");
        assert_eq!(room_objects[0].nested_objects.len(), 1);
        assert_eq!(room_objects[1].name, "tree");
        assert!(room_objects[1].nested_objects.is_empty());
    }

    #[test]
    fn test_ir_generator_room_objects_field() {
        // Test IrGenerator room_objects field operations
        let mut ir_generator = IrGenerator::new();

        // Verify field is initialized empty
        let room_objects = ir_generator.get_room_objects();
        assert!(room_objects.is_empty());

        // Test that we can access the field (compilation test)
        assert_eq!(room_objects.len(), 0);
    }

    #[test]
    fn test_room_object_info_clone() {
        // Test that RoomObjectInfo implements Clone correctly
        let original = RoomObjectInfo {
            name: "mailbox".to_string(),
            nested_objects: vec![RoomObjectInfo {
                name: "leaflet".to_string(),
                nested_objects: vec![],
            }],
        };

        let cloned = original.clone();

        assert_eq!(original.name, cloned.name);
        assert_eq!(original.nested_objects.len(), cloned.nested_objects.len());
        assert_eq!(
            original.nested_objects[0].name,
            cloned.nested_objects[0].name
        );

        // Verify it's a deep clone - changes to clone don't affect original
        // (Rust's Clone for String and Vec creates deep copies)
    }

    #[test]
    fn test_room_object_info_debug() {
        // Test that RoomObjectInfo implements Debug correctly
        let obj = RoomObjectInfo {
            name: "test".to_string(),
            nested_objects: vec![],
        };

        let debug_str = format!("{:?}", obj);
        assert!(debug_str.contains("test"));
        assert!(debug_str.contains("RoomObjectInfo"));
    }

    #[test]
    fn test_multiple_rooms_with_objects() {
        // Test scenario with multiple rooms each having different object configurations
        let leaflet = RoomObjectInfo {
            name: "leaflet".to_string(),
            nested_objects: vec![],
        };

        let mailbox = RoomObjectInfo {
            name: "mailbox".to_string(),
            nested_objects: vec![leaflet],
        };

        let sword = RoomObjectInfo {
            name: "sword".to_string(),
            nested_objects: vec![],
        };

        let shield = RoomObjectInfo {
            name: "shield".to_string(),
            nested_objects: vec![],
        };

        // west_of_house has mailbox (with leaflet inside)
        let west_house_objects = vec![mailbox];

        // armory has sword and shield
        let armory_objects = vec![sword, shield];

        assert_eq!(west_house_objects.len(), 1);
        assert_eq!(west_house_objects[0].name, "mailbox");
        assert_eq!(west_house_objects[0].nested_objects.len(), 1);
        assert_eq!(west_house_objects[0].nested_objects[0].name, "leaflet");

        assert_eq!(armory_objects.len(), 2);
        assert_eq!(armory_objects[0].name, "sword");
        assert_eq!(armory_objects[1].name, "shield");
        assert!(armory_objects[0].nested_objects.is_empty());
        assert!(armory_objects[1].nested_objects.is_empty());
    }

    #[test]
    fn test_room_object_info_empty_vs_populated() {
        // Test distinction between empty and populated nested_objects
        let empty_container = RoomObjectInfo {
            name: "empty_chest".to_string(),
            nested_objects: vec![],
        };

        let full_container = RoomObjectInfo {
            name: "full_chest".to_string(),
            nested_objects: vec![RoomObjectInfo {
                name: "gold".to_string(),
                nested_objects: vec![],
            }],
        };

        assert!(empty_container.nested_objects.is_empty());
        assert!(!full_container.nested_objects.is_empty());
        assert_eq!(full_container.nested_objects.len(), 1);
        assert_eq!(full_container.nested_objects[0].name, "gold");
    }

    #[test]
    fn test_phase_1b_integration_with_mini_zork() {
        // Integration test: Verify Phase 1b works with actual mini_zork.grue parsing
        let source = r#"
            world {
                room west_of_house "West of House" {
                    desc: "You are standing in an open field west of a white house."

                    object mailbox {
                        names: ["small mailbox", "mailbox", "box"]
                        desc: "The small mailbox is closed."
                        openable: true
                        container: true

                        contains {
                            object leaflet {
                                names: ["leaflet", "paper"]
                                desc: "Welcome to Zork!"
                            }
                        }
                    }
                }

                room empty_room "Empty Room" {
                    desc: "This room has no objects."
                }
            }
        "#;

        let ir = generate_ir_from_source(source).unwrap();

        // Verify rooms were created
        assert_eq!(ir.rooms.len(), 2);

        // Create IrGenerator to test Phase 1b functionality
        let mut ir_generator = IrGenerator::new();
        let mut lexer = crate::grue_compiler::lexer::Lexer::new(source);
        let tokens = lexer.tokenize().unwrap();
        let mut parser = crate::grue_compiler::parser::Parser::new(tokens);
        let ast = parser.parse().unwrap();
        let _ = ir_generator.generate(ast).unwrap();

        // Verify Phase 1b: Check room_objects mapping
        let room_objects = ir_generator.get_room_objects();

        // Should have recorded objects for west_of_house but not empty_room
        assert!(room_objects.contains_key("west_of_house"));
        assert!(!room_objects.contains_key("empty_room"));

        // Verify west_of_house has mailbox with leaflet nested inside
        let west_house_objects = &room_objects["west_of_house"];
        assert_eq!(west_house_objects.len(), 1);
        assert_eq!(west_house_objects[0].name, "mailbox");
        assert_eq!(west_house_objects[0].nested_objects.len(), 1);
        assert_eq!(west_house_objects[0].nested_objects[0].name, "leaflet");
        assert!(west_house_objects[0].nested_objects[0]
            .nested_objects
            .is_empty());
    }

    #[test]
    fn test_score_property_access_maps_to_global_g17() {
        let source = r#"
            fn test_score() {
                let current = player.score;
                print_num(current);
            }
        "#;

        let ir = generate_ir_from_source(source).unwrap();

        // Find the test_score function
        let test_function = ir
            .functions
            .iter()
            .find(|f| f.name == "test_score")
            .expect("test_score function should exist");

        // Look for LoadVar instruction that reads from Global G17 (score)
        let mut found_score_read = false;
        for instruction in &test_function.body.instructions {
            if let IrInstruction::LoadVar { var_id, .. } = instruction {
                if *var_id == 17 {
                    // Global G17 = score
                    found_score_read = true;
                }
            }
        }

        assert!(
            found_score_read,
            "player.score should generate LoadVar with var_id=17 (Global G17)"
        );
    }

    #[test]
    fn test_score_property_assignment_maps_to_global_g17() {
        let source = r#"
            fn set_score() {
                player.score = 100;
            }
        "#;

        let ir = generate_ir_from_source(source).unwrap();

        // Find the set_score function
        let set_function = ir
            .functions
            .iter()
            .find(|f| f.name == "set_score")
            .expect("set_score function should exist");

        // Look for StoreVar instruction that writes to Global G17 (score)
        let mut found_score_write = false;
        for instruction in &set_function.body.instructions {
            if let IrInstruction::StoreVar { var_id, .. } = instruction {
                if *var_id == 17 {
                    // Global G17 = score
                    found_score_write = true;
                }
            }
        }

        assert!(
            found_score_write,
            "player.score assignment should generate StoreVar with var_id=17 (Global G17)"
        );
    }

    #[test]
    fn test_add_score_builtin_function() {
        let source = r#"
            fn gain_points() {
                add_score(10);
            }
        "#;

        let ir = generate_ir_from_source(source).unwrap();

        // Find the gain_points function
        let function = ir
            .functions
            .iter()
            .find(|f| f.name == "gain_points")
            .expect("gain_points function should exist");

        // Check that add_score generates the correct sequence:
        // 1. LoadVar from G17 (current score)
        // 2. BinaryOp Add
        // 3. StoreVar to G17 (new score)
        // 4. LoadVar from G17 (return value)
        let mut load_vars = 0;
        let mut binary_ops = 0;
        let mut store_vars = 0;

        for instruction in &function.body.instructions {
            match instruction {
                IrInstruction::LoadVar { var_id, .. } if *var_id == 17 => load_vars += 1,
                IrInstruction::BinaryOp {
                    op: IrBinaryOp::Add,
                    ..
                } => binary_ops += 1,
                IrInstruction::StoreVar { var_id, .. } if *var_id == 17 => store_vars += 1,
                _ => {}
            }
        }

        assert_eq!(
            load_vars, 2,
            "add_score should load from G17 twice (current + return)"
        );
        assert_eq!(binary_ops, 1, "add_score should have one Add operation");
        assert_eq!(store_vars, 1, "add_score should store to G17 once");
    }

    #[test]
    fn test_subtract_score_builtin_function() {
        let source = r#"
            fn lose_points() {
                subtract_score(5);
            }
        "#;

        let ir = generate_ir_from_source(source).unwrap();

        // Find the lose_points function
        let function = ir
            .functions
            .iter()
            .find(|f| f.name == "lose_points")
            .expect("lose_points function should exist");

        // Check that subtract_score generates the correct sequence:
        // 1. LoadVar from G17 (current score)
        // 2. BinaryOp Subtract
        // 3. StoreVar to G17 (new score)
        // 4. LoadVar from G17 (return value)
        let mut load_vars = 0;
        let mut binary_ops = 0;
        let mut store_vars = 0;

        for instruction in &function.body.instructions {
            match instruction {
                IrInstruction::LoadVar { var_id, .. } if *var_id == 17 => load_vars += 1,
                IrInstruction::BinaryOp {
                    op: IrBinaryOp::Subtract,
                    ..
                } => binary_ops += 1,
                IrInstruction::StoreVar { var_id, .. } if *var_id == 17 => store_vars += 1,
                _ => {}
            }
        }

        assert_eq!(
            load_vars, 2,
            "subtract_score should load from G17 twice (current + return)"
        );
        assert_eq!(
            binary_ops, 1,
            "subtract_score should have one Subtract operation"
        );
        assert_eq!(store_vars, 1, "subtract_score should store to G17 once");
    }

    #[test]
    fn test_score_system_comprehensive() {
        let source = r#"
            world {
                room start "Test Room" {
                    desc: "Test room"
                }
            }

            fn test_all_score_operations() {
                let current = player.score;      // Read from G17
                player.score = 50;               // Write to G17
                add_score(10);                   // Increment via builtin
                subtract_score(5);               // Decrement via builtin
            }

            init {
                player.location = start;
            }
        "#;

        let ir = generate_ir_from_source(source).unwrap();

        // Verify we have the expected functions
        assert!(ir
            .functions
            .iter()
            .any(|f| f.name == "test_all_score_operations"));

        // Find the test function
        let test_function = ir
            .functions
            .iter()
            .find(|f| f.name == "test_all_score_operations")
            .expect("test_all_score_operations function should exist");

        // Count Global G17 operations
        let mut g17_loads = 0;
        let mut g17_stores = 0;
        let mut adds = 0;
        let mut subtracts = 0;

        for instruction in &test_function.body.instructions {
            match instruction {
                IrInstruction::LoadVar { var_id, .. } if *var_id == 17 => g17_loads += 1,
                IrInstruction::StoreVar { var_id, .. } if *var_id == 17 => g17_stores += 1,
                IrInstruction::BinaryOp {
                    op: IrBinaryOp::Add,
                    ..
                } => adds += 1,
                IrInstruction::BinaryOp {
                    op: IrBinaryOp::Subtract,
                    ..
                } => subtracts += 1,
                _ => {}
            }
        }

        // Should have:
        // - 1 load for reading current score
        // - 1 store for direct assignment
        // - 2 loads + 1 store from add_score()
        // - 2 loads + 1 store from subtract_score()
        // Total: 5 loads, 3 stores, 1 add, 1 subtract
        assert!(
            g17_loads >= 5,
            "Should have at least 5 loads from G17, got {}",
            g17_loads
        );
        assert!(
            g17_stores >= 3,
            "Should have at least 3 stores to G17, got {}",
            g17_stores
        );
        assert_eq!(adds, 1, "Should have exactly 1 Add operation");
        assert_eq!(subtracts, 1, "Should have exactly 1 Subtract operation");
    }
}
