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
            
            grammar {
                verb "test" {
                    default => print("Testing")
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
        assert_eq!(ir.grammar.len(), 1);
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

        // Verify grammar
        let grammar = &ir.grammar[0];
        assert_eq!(grammar.verb, "test");

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
    fn test_property_access_ir() {
        let source = r#"
            fn test_properties() {
                let obj = player;
                let pos = obj.position;
                obj.visited = true;
            }
        "#;

        let ir = generate_ir_from_source(source).unwrap();

        assert_eq!(ir.functions.len(), 1);
        let func = &ir.functions[0];
        let instructions = &func.body.instructions;

        // Should contain GetPropertyByNumber instruction
        let get_props = instructions
            .iter()
            .filter(|inst| matches!(inst, IrInstruction::GetPropertyByNumber { .. }))
            .count();
        assert!(
            get_props >= 1,
            "Should have GetPropertyByNumber instruction"
        );

        // Should contain SetPropertyByNumber instruction
        let set_props = instructions
            .iter()
            .filter(|inst| matches!(inst, IrInstruction::SetPropertyByNumber { .. }))
            .count();
        assert!(
            set_props >= 1,
            "Should have SetPropertyByNumber instruction"
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
}
