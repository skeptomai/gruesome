// Comprehensive semantic analysis tests for Grue language

#[cfg(test)]
mod semantic_tests {
    use crate::grue_compiler::ast::*;
    use crate::grue_compiler::error::CompilerError;
    use crate::grue_compiler::lexer::Lexer;
    use crate::grue_compiler::parser::Parser;
    use crate::grue_compiler::semantic::SemanticAnalyzer;

    fn analyze_input(input: &str) -> Result<Program, CompilerError> {
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize()?;
        let mut parser = Parser::new(tokens);
        let ast = parser.parse()?;
        let mut analyzer = SemanticAnalyzer::new();
        analyzer.analyze(ast)
    }

    #[test]
    fn test_simple_function_definition() {
        let input = r#"
            fn greet(name) {
                print("Hello, " + name + "!");
            }
        "#;
        let result = analyze_input(input);
        if result.is_err() {
            println!("Error: {:?}", result);
        }
        assert!(result.is_ok());
    }

    #[test]
    fn test_function_with_typed_parameters() {
        let input = r#"
            fn calculate_damage(base: int, multiplier: int) -> int {
                return base * multiplier;
            }
        "#;
        let result = analyze_input(input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_duplicate_function_definition() {
        let input = r#"
            fn test_func() {
                print("first");
            }
            
            fn test_func() {
                print("second");
            }
        "#;
        let result = analyze_input(input);
        assert!(result.is_err());
        if let Err(CompilerError::SemanticError(msg, _)) = result {
            assert!(msg.contains("already defined"));
        } else {
            panic!("Expected semantic error about duplicate function");
        }
    }

    #[test]
    fn test_simple_world_and_rooms() {
        let input = r#"
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
        let result = analyze_input(input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_room_with_undefined_exit() {
        let input = r#"
            world {
                room west_house "West of House" {
                    desc: "You are standing in an open field."
                    exits: {
                        north: undefined_room
                    }
                }
            }
        "#;
        let result = analyze_input(input);
        assert!(result.is_err());
        if let Err(CompilerError::SemanticError(msg, _)) = result {
            assert!(msg.contains("undefined room"));
        } else {
            panic!("Expected semantic error about undefined room");
        }
    }

    #[test]
    fn test_duplicate_room_definition() {
        let input = r#"
            world {
                room test_room "Test Room 1" {
                    desc: "First room."
                }
                
                room test_room "Test Room 2" {
                    desc: "Second room."
                }
            }
        "#;
        let result = analyze_input(input);
        assert!(result.is_err());
        if let Err(CompilerError::SemanticError(msg, _)) = result {
            assert!(msg.contains("already defined"));
        } else {
            panic!("Expected semantic error about duplicate room");
        }
    }

    #[test]
    fn test_objects_in_rooms() {
        let input = r#"
            world {
                room west_house "West of House" {
                    desc: "You are standing in an open field."
                    
                    object mailbox {
                        names: ["small mailbox", "mailbox", "box"]
                        desc: "A small mailbox."
                        openable: true
                        container: true
                        
                        contains {
                            object leaflet {
                                names: ["leaflet", "mail"]
                                desc: "A leaflet."
                                readable: true
                            }
                        }
                    }
                }
            }
        "#;
        let result = analyze_input(input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_duplicate_object_definition() {
        let input = r#"
            world {
                room test_room "Test Room" {
                    desc: "A test room."
                    
                    object test_obj {
                        names: ["test object"]
                        desc: "First object."
                    }
                    
                    object test_obj {
                        names: ["test object"]
                        desc: "Second object."
                    }
                }
            }
        "#;
        let result = analyze_input(input);
        assert!(result.is_err());
        if let Err(CompilerError::SemanticError(msg, _)) = result {
            assert!(msg.contains("already defined"));
        } else {
            panic!("Expected semantic error about duplicate object");
        }
    }

    #[test]
    fn test_variable_declarations() {
        let input = r#"
            fn test_variables() {
                let constant_value: int = 42;
                var mutable_counter = 0;
                var uninitialized_flag: bool;
                
                mutable_counter = constant_value + 1;
                uninitialized_flag = true;
            }
        "#;
        let result = analyze_input(input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_type_mismatch_in_variable_declaration() {
        let input = r#"
            fn test_type_error() {
                let number: int = "this is a string";
            }
        "#;
        let result = analyze_input(input);
        if result.is_ok() {
            println!("Type mismatch test unexpectedly passed - should have failed");
        }
        assert!(result.is_err());
        if let Err(CompilerError::SemanticError(msg, _)) = result {
            assert!(msg.contains("type mismatch") || msg.contains("Type mismatch"));
        } else {
            panic!("Expected semantic error about type mismatch");
        }
    }

    #[test]
    fn test_duplicate_variable_in_same_scope() {
        let input = r#"
            fn test_duplicate() {
                let x = 1;
                let x = 2;
            }
        "#;
        let result = analyze_input(input);
        assert!(result.is_err());
        if let Err(CompilerError::SemanticError(msg, _)) = result {
            assert!(msg.contains("already defined"));
        } else {
            panic!("Expected semantic error about duplicate variable");
        }
    }

    #[test]
    fn test_function_call_validation() {
        let input = r#"
            fn helper(value: int) -> string {
                return "result: " + value;
            }
            
            fn main() {
                let result = helper(42);
                print(result);
            }
        "#;
        let result = analyze_input(input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_undefined_function_call() {
        let input = r#"
            fn test() {
                let result = undefined_function(123);
            }
        "#;
        let result = analyze_input(input);
        assert!(result.is_err());
        if let Err(CompilerError::SemanticError(msg, _)) = result {
            assert!(msg.contains("Undefined function"));
        } else {
            panic!("Expected semantic error about undefined function");
        }
    }

    #[test]
    fn test_function_call_argument_count_mismatch() {
        let input = r#"
            fn add_numbers(a: int, b: int) -> int {
                return a + b;
            }
            
            fn test() {
                let result = add_numbers(1, 2, 3); // Too many arguments
            }
        "#;
        let result = analyze_input(input);
        assert!(result.is_err());
        if let Err(CompilerError::SemanticError(msg, _)) = result {
            assert!(msg.contains("expects") && msg.contains("arguments"));
        } else {
            panic!("Expected semantic error about argument count");
        }
    }

    #[test]
    fn test_grammar_with_function_references() {
        let input = r#"
            fn handle_look() {
                print("You look around.");
            }
            
            fn handle_take(obj) {
                print("You take the " + obj + ".");
            }
            
            grammar {
                verb "look" {
                    default => handle_look()
                }
                
                verb "take" {
                    noun => handle_take($noun)
                }
            }
        "#;
        let result = analyze_input(input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_grammar_with_undefined_function() {
        let input = r#"
            grammar {
                verb "look" {
                    default => undefined_handler()
                }
            }
        "#;
        let result = analyze_input(input);
        assert!(result.is_err());
        if let Err(CompilerError::SemanticError(msg, _)) = result {
            assert!(msg.contains("undefined function"));
        } else {
            panic!("Expected semantic error about undefined function in grammar");
        }
    }

    #[test]
    fn test_control_flow_statements() {
        let input = r#"
            fn test_control_flow() {
                let condition = true;
                let counter = 0;
                let items = [1, 2, 3];
                
                if condition {
                    print("condition is true");
                } else {
                    print("condition is false");
                }
                
                while counter < 10 {
                    counter = counter + 1;
                }
                
                for item in items {
                    print("item: " + item);
                }
            }
        "#;
        let result = analyze_input(input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_invalid_condition_types() {
        let input = r#"
            fn test_bad_condition() {
                let number = 42;
                
                if number {  // Should be boolean, not int
                    print("this shouldn't work");
                }
            }
        "#;
        let result = analyze_input(input);
        assert!(result.is_err());
        if let Err(CompilerError::SemanticError(msg, _)) = result {
            assert!(msg.contains("boolean"));
        } else {
            panic!("Expected semantic error about non-boolean condition");
        }
    }

    #[test]
    fn test_nested_scopes() {
        let input = r#"
            fn test_scopes() {
                let outer_var = "outer";
                
                if true {
                    let inner_var = "inner";
                    print(outer_var); // Should be accessible
                    print(inner_var); // Should be accessible
                }
                
                // inner_var should not be accessible here
                print(outer_var); // Should still be accessible
            }
        "#;
        let result = analyze_input(input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_function_parameter_scoping() {
        let input = r#"
            fn test_func(param1: string, param2: int) {
                let local_var = param1 + " - " + param2;
                
                if param2 > 0 {
                    let nested_var = param1; // Parameter should be accessible
                    print(nested_var);
                }
                
                print(local_var);
            }
        "#;
        let result = analyze_input(input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_undefined_identifier() {
        let input = r#"
            fn test() {
                print(undefined_variable);
            }
        "#;
        let result = analyze_input(input);
        assert!(result.is_err());
        if let Err(CompilerError::SemanticError(msg, _)) = result {
            assert!(msg.contains("Undefined identifier"));
        } else {
            panic!("Expected semantic error about undefined identifier");
        }
    }

    #[test]
    fn test_array_type_consistency() {
        let input = r#"
            fn test_arrays() {
                let numbers = [1, 2, 3];           // Should be ok
                let strings = ["a", "b", "c"];     // Should be ok
                let mixed = [1, "two", true];      // Should fail
            }
        "#;
        let result = analyze_input(input);
        assert!(result.is_err());
        if let Err(CompilerError::SemanticError(msg, _)) = result {
            assert!(msg.contains("compatible types"));
        } else {
            panic!("Expected semantic error about array type compatibility");
        }
    }

    #[test]
    fn test_property_access() {
        let input = r#"
            fn test_properties() {
                let obj = player; // Assume player is defined somewhere
                let position = obj.current_room;
                let is_open = obj.open;
                
                obj.visited = true;
            }
        "#;
        let result = analyze_input(input);
        if result.is_err() {
            println!("Property access error: {:?}", result);
        }
        // This should pass for now since we allow any property access
        // In a more complete implementation, we'd validate object properties
        assert!(result.is_ok());
    }

    #[test]
    fn test_ternary_expression() {
        let input = r#"
            fn test_ternary() {
                let condition = true;
                let result = condition ? "yes" : "no";
                let number_result = condition ? 1 : 2;
            }
        "#;
        let result = analyze_input(input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_ternary_with_non_boolean_condition() {
        let input = r#"
            fn test_bad_ternary() {
                let number = 42;
                let result = number ? "yes" : "no"; // Should require boolean condition
            }
        "#;
        let result = analyze_input(input);
        assert!(result.is_err());
        if let Err(CompilerError::SemanticError(msg, _)) = result {
            assert!(msg.contains("boolean"));
        } else {
            panic!("Expected semantic error about non-boolean ternary condition");
        }
    }

    #[test]
    fn test_complex_program() {
        let input = r#"
            fn print_location(loc: string) {
                print("You are at: " + loc);
            }
            
            world {
                room starting_room "Starting Room" {
                    desc: "A simple starting room."
                    
                    object key {
                        names: ["small key", "key"]
                        desc: "A small brass key."
                    }
                    
                    on_enter: {
                        print_location("starting room");
                    }
                    
                    exits: {
                        north: second_room
                    }
                }
                
                room second_room "Second Room" {
                    desc: "Another room."
                    
                    exits: {
                        south: starting_room
                    }
                }
            }
            
            grammar {
                verb "look" {
                    default => print_location("here")
                }
            }
            
            init {
                print("Game initialized!");
            }
        "#;
        let result = analyze_input(input);
        if result.is_err() {
            println!("Complex program error: {:?}", result);
        }
        assert!(result.is_ok());
    }

    #[test]
    fn test_init_block_semantic_analysis() {
        let input = r#"
            fn setup_game() {
                print("Setting up the game...");
            }
            
            init {
                setup_game();
                let start_time = 0;
                print("Game started at time: " + start_time);
            }
        "#;
        let result = analyze_input(input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_room_event_handlers() {
        let input = r#"
            fn announce_entry() {
                print("You have entered!");
            }
            
            world {
                room test_room "Test Room" {
                    desc: "A room for testing."
                    
                    on_enter: {
                        announce_entry();
                        let visited = true;
                    }
                    
                    on_exit: {
                        print("Leaving the room...");
                    }
                    
                    on_look: {
                        print("You examine the room carefully.");
                    }
                }
            }
        "#;
        let result = analyze_input(input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_for_loop_variable_scoping() {
        let input = r#"
            fn test_for_loop() {
                let items = ["apple", "banana", "cherry"];
                
                for item in items {
                    print("Current item: " + item);
                    let temp = item + " processed";
                    print(temp);
                }
                
                // item and temp should not be accessible here
                // This test just verifies the loop compiles correctly
            }
        "#;
        let result = analyze_input(input);
        assert!(result.is_ok());
    }
}
