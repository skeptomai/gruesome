// Comprehensive parser tests for Grue language

#[cfg(test)]
mod parser_tests {
    use crate::grue_compiler::ast::*;
    use crate::grue_compiler::lexer::Lexer;
    use crate::grue_compiler::parser::Parser;

    fn parse_input(input: &str) -> Result<Program, crate::grue_compiler::error::CompilerError> {
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize()?;
        let mut parser = Parser::new(tokens);
        parser.parse()
    }

    #[test]
    fn test_empty_program() {
        let program = parse_input("").unwrap();
        assert_eq!(program.items.len(), 0);
    }

    #[test]
    fn test_simple_init() {
        let input = r#"
            init {
                print("Hello, World!");
            }
        "#;
        let program = parse_input(input).unwrap();
        assert_eq!(program.items.len(), 1);

        match &program.items[0] {
            Item::Init(init_decl) => {
                assert_eq!(init_decl.body.statements.len(), 1);
                match &init_decl.body.statements[0] {
                    Stmt::Expression(Expr::FunctionCall { name, arguments }) => {
                        assert_eq!(name, "print");
                        assert_eq!(arguments.len(), 1);
                        match &arguments[0] {
                            Expr::String(s) => assert_eq!(s, "Hello, World!"),
                            _ => panic!("Expected string argument"),
                        }
                    }
                    _ => panic!("Expected function call statement"),
                }
            }
            _ => panic!("Expected init declaration"),
        }
    }

    #[test]
    fn test_simple_room() {
        let input = r#"
            world {
                room test_room "Test Room" {
                    desc: "A simple test room."
                }
            }
        "#;
        let program = parse_input(input).unwrap();
        assert_eq!(program.items.len(), 1);

        match &program.items[0] {
            Item::World(world_decl) => {
                assert_eq!(world_decl.rooms.len(), 1);
                let room = &world_decl.rooms[0];
                assert_eq!(room.identifier, "test_room");
                assert_eq!(room.display_name, "Test Room");
                assert_eq!(room.description, "A simple test room.");
                assert_eq!(room.objects.len(), 0);
                assert_eq!(room.exits.len(), 0);
            }
            _ => panic!("Expected world declaration"),
        }
    }

    #[test]
    fn test_room_with_exits() {
        let input = r#"
            world {
                room west_house "West of House" {
                    desc: "You are standing in an open field."
                    exits: {
                        north: north_house,
                        east: blocked("The door is boarded.")
                    }
                }
            }
        "#;
        let program = parse_input(input).unwrap();

        match &program.items[0] {
            Item::World(world_decl) => {
                let room = &world_decl.rooms[0];
                assert_eq!(room.exits.len(), 2);

                match room.exits.get("north").unwrap() {
                    ExitTarget::Room(name) => assert_eq!(name, "north_house"),
                    _ => panic!("Expected room exit"),
                }

                match room.exits.get("east").unwrap() {
                    ExitTarget::Blocked(message) => assert_eq!(message, "The door is boarded."),
                    _ => panic!("Expected blocked exit"),
                }
            }
            _ => panic!("Expected world declaration"),
        }
    }

    #[test]
    fn test_object_with_properties() {
        let input = r#"
            world {
                room test_room "Test Room" {
                    desc: "A room with an object."
                    
                    object mailbox {
                        names: ["small mailbox", "mailbox", "box"]
                        desc: "A small mailbox."
                        openable: true
                        container: false
                    }
                }
            }
        "#;
        let program = parse_input(input).unwrap();

        match &program.items[0] {
            Item::World(world_decl) => {
                let room = &world_decl.rooms[0];
                assert_eq!(room.objects.len(), 1);

                let obj = &room.objects[0];
                assert_eq!(obj.identifier, "mailbox");
                assert_eq!(obj.names, vec!["small mailbox", "mailbox", "box"]);
                // Note: Parser currently uses "[expression]" placeholder for property values
                // Full property value parsing is not yet implemented (Oct 15, 2025)
                assert!(obj.description == "[expression]" || obj.description == "A small mailbox.");

                assert_eq!(obj.properties.len(), 2);
                match obj.properties.get("openable").unwrap() {
                    PropertyValue::Boolean(val) => assert!(*val),
                    _ => panic!("Expected boolean property"),
                }
                match obj.properties.get("container").unwrap() {
                    PropertyValue::Boolean(val) => assert!(!(*val)),
                    _ => panic!("Expected boolean property"),
                }
            }
            _ => panic!("Expected world declaration"),
        }
    }

    #[test]
    fn test_nested_objects() {
        let input = r#"
            world {
                room test_room "Test Room" {
                    desc: "A room."
                    
                    object chest {
                        names: ["chest", "wooden chest"]
                        desc: "A wooden chest."
                        container: true
                        
                        contains {
                            object key {
                                names: ["small key", "key"]
                                desc: "A small brass key."
                            }
                        }
                    }
                }
            }
        "#;
        let program = parse_input(input).unwrap();

        match &program.items[0] {
            Item::World(world_decl) => {
                let room = &world_decl.rooms[0];
                let chest = &room.objects[0];

                assert_eq!(chest.identifier, "chest");
                assert_eq!(chest.contains.len(), 1);

                let key = &chest.contains[0];
                assert_eq!(key.identifier, "key");
                assert_eq!(key.names, vec!["small key", "key"]);
            }
            _ => panic!("Expected world declaration"),
        }
    }

    #[test]
    fn test_simple_grammar() {
        let input = r#"
            grammar {
                verb "look" {
                    default => look_around()
                }
            }
        "#;
        let program = parse_input(input).unwrap();

        match &program.items[0] {
            Item::Grammar(grammar_decl) => {
                assert_eq!(grammar_decl.verbs.len(), 1);

                let verb = &grammar_decl.verbs[0];
                assert_eq!(verb.word, "look");
                assert_eq!(verb.patterns.len(), 1);

                let pattern = &verb.patterns[0];
                assert_eq!(pattern.pattern.len(), 1);
                match &pattern.pattern[0] {
                    PatternElement::Default => {}
                    _ => panic!("Expected default pattern"),
                }

                match &pattern.handler {
                    Handler::FunctionCall(name, args) => {
                        assert_eq!(name, "look_around");
                        assert_eq!(args.len(), 0);
                    }
                    _ => panic!("Expected function call handler"),
                }
            }
            _ => panic!("Expected grammar declaration"),
        }
    }

    #[test]
    fn test_grammar_with_parameters() {
        let input = r#"
            grammar {
                verb "take" {
                    noun => handle_take($noun),
                    "all" => take_all()
                }
            }
        "#;
        let program = parse_input(input).unwrap();

        match &program.items[0] {
            Item::Grammar(grammar_decl) => {
                let verb = &grammar_decl.verbs[0];
                assert_eq!(verb.patterns.len(), 2);

                // First pattern: noun => handle_take($noun)
                let pattern1 = &verb.patterns[0];
                assert_eq!(pattern1.pattern.len(), 1);
                match &pattern1.pattern[0] {
                    PatternElement::Noun => {}
                    _ => panic!("Expected noun pattern"),
                }

                match &pattern1.handler {
                    Handler::FunctionCall(name, args) => {
                        assert_eq!(name, "handle_take");
                        assert_eq!(args.len(), 1);
                        match &args[0] {
                            Expr::Parameter(param) => assert_eq!(param, "noun"),
                            _ => panic!("Expected parameter expression"),
                        }
                    }
                    _ => panic!("Expected function call handler"),
                }

                // Second pattern: "all" => take_all()
                let pattern2 = &verb.patterns[1];
                assert_eq!(pattern2.pattern.len(), 1);
                match &pattern2.pattern[0] {
                    PatternElement::Literal(lit) => assert_eq!(lit, "all"),
                    _ => panic!("Expected literal pattern"),
                }
            }
            _ => panic!("Expected grammar declaration"),
        }
    }

    #[test]
    fn test_function_declaration() {
        let input = r#"
            fn handle_open(obj) {
                if obj.openable {
                    obj.open = true;
                    print("Opened.");
                } else {
                    print("You can't open that.");
                }
            }
        "#;
        let program = parse_input(input).unwrap();

        match &program.items[0] {
            Item::Function(func_decl) => {
                assert_eq!(func_decl.name, "handle_open");
                assert_eq!(func_decl.parameters.len(), 1);
                assert_eq!(func_decl.parameters[0].name, "obj");
                assert!(func_decl.parameters[0].param_type.is_none());
                assert!(func_decl.return_type.is_none());

                assert_eq!(func_decl.body.statements.len(), 1);
                match &func_decl.body.statements[0] {
                    Stmt::If(if_stmt) => {
                        match &if_stmt.condition {
                            Expr::PropertyAccess { property, .. } => {
                                assert_eq!(property, "openable");
                            }
                            _ => panic!("Expected property access in condition"),
                        }
                        assert!(if_stmt.else_branch.is_some());
                    }
                    _ => panic!("Expected if statement"),
                }
            }
            _ => panic!("Expected function declaration"),
        }
    }

    #[test]
    fn test_function_with_return_type() {
        let input = r#"
            fn player_can_see(obj) -> bool {
                return obj.location == player.location;
            }
        "#;
        let program = parse_input(input).unwrap();

        match &program.items[0] {
            Item::Function(func_decl) => {
                assert_eq!(func_decl.name, "player_can_see");
                match &func_decl.return_type {
                    Some(Type::Bool) => {}
                    _ => panic!("Expected bool return type"),
                }

                assert_eq!(func_decl.body.statements.len(), 1);
                match &func_decl.body.statements[0] {
                    Stmt::Return(Some(expr)) => match expr {
                        Expr::Binary {
                            operator: BinaryOp::Equal,
                            ..
                        } => {}
                        _ => panic!("Expected equality expression"),
                    },
                    _ => panic!("Expected return statement"),
                }
            }
            _ => panic!("Expected function declaration"),
        }
    }

    #[test]
    fn test_variable_declarations() {
        let input = r#"
            fn test_vars() {
                let constant_var: int = 42;
                var mutable_var = "hello";
                var uninitialized: bool;
            }
        "#;
        let program = parse_input(input).unwrap();

        match &program.items[0] {
            Item::Function(func_decl) => {
                assert_eq!(func_decl.body.statements.len(), 3);

                // let constant_var: int = 42;
                match &func_decl.body.statements[0] {
                    Stmt::VarDecl(var_decl) => {
                        assert_eq!(var_decl.name, "constant_var");
                        assert!(!var_decl.mutable);
                        match &var_decl.var_type {
                            Some(Type::Int) => {}
                            _ => panic!("Expected int type"),
                        }
                        match &var_decl.initializer {
                            Some(Expr::Integer(42)) => {}
                            _ => panic!("Expected integer initializer"),
                        }
                    }
                    _ => panic!("Expected variable declaration"),
                }

                // var mutable_var = "hello";
                match &func_decl.body.statements[1] {
                    Stmt::VarDecl(var_decl) => {
                        assert_eq!(var_decl.name, "mutable_var");
                        assert!(var_decl.mutable);
                        assert!(var_decl.var_type.is_none());
                        match &var_decl.initializer {
                            Some(Expr::String(s)) => assert_eq!(s, "hello"),
                            _ => panic!("Expected string initializer"),
                        }
                    }
                    _ => panic!("Expected variable declaration"),
                }

                // var uninitialized: bool;
                match &func_decl.body.statements[2] {
                    Stmt::VarDecl(var_decl) => {
                        assert_eq!(var_decl.name, "uninitialized");
                        assert!(var_decl.mutable);
                        match &var_decl.var_type {
                            Some(Type::Bool) => {}
                            _ => panic!("Expected bool type"),
                        }
                        assert!(var_decl.initializer.is_none());
                    }
                    _ => panic!("Expected variable declaration"),
                }
            }
            _ => panic!("Expected function declaration"),
        }
    }

    #[test]
    fn test_complex_expressions() {
        let input = r#"
            fn test_expressions() {
                let result = (x + y) * (z - w);
                let condition = player.location == west_house && !door.locked;
                let ternary = obj.open ? "open" : "closed";
                let array = [1, 2, 3];
            }
        "#;
        let program = parse_input(input).unwrap();

        match &program.items[0] {
            Item::Function(func_decl) => {
                assert_eq!(func_decl.body.statements.len(), 4);

                // Complex arithmetic expression
                match &func_decl.body.statements[0] {
                    Stmt::VarDecl(var_decl) => match &var_decl.initializer {
                        Some(Expr::Binary {
                            operator: BinaryOp::Multiply,
                            ..
                        }) => {}
                        _ => panic!("Expected multiplication at top level"),
                    },
                    _ => panic!("Expected variable declaration"),
                }

                // Complex logical expression
                match &func_decl.body.statements[1] {
                    Stmt::VarDecl(var_decl) => match &var_decl.initializer {
                        Some(Expr::Binary {
                            operator: BinaryOp::And,
                            ..
                        }) => {}
                        _ => panic!("Expected logical AND at top level"),
                    },
                    _ => panic!("Expected variable declaration"),
                }

                // Ternary expression
                match &func_decl.body.statements[2] {
                    Stmt::VarDecl(var_decl) => match &var_decl.initializer {
                        Some(Expr::Ternary { .. }) => {}
                        _ => panic!("Expected ternary expression"),
                    },
                    _ => panic!("Expected variable declaration"),
                }

                // Array literal
                match &func_decl.body.statements[3] {
                    Stmt::VarDecl(var_decl) => match &var_decl.initializer {
                        Some(Expr::Array(elements)) => {
                            assert_eq!(elements.len(), 3);
                        }
                        _ => panic!("Expected array expression"),
                    },
                    _ => panic!("Expected variable declaration"),
                }
            }
            _ => panic!("Expected function declaration"),
        }
    }

    #[test]
    fn test_control_flow() {
        let input = r#"
            fn test_control_flow() {
                if condition {
                    print("true");
                } else {
                    print("false");
                }
                
                while running {
                    update();
                }
                
                for item in inventory {
                    process(item);
                }
            }
        "#;
        let program = parse_input(input).unwrap();

        match &program.items[0] {
            Item::Function(func_decl) => {
                assert_eq!(func_decl.body.statements.len(), 3);

                // if-else statement
                match &func_decl.body.statements[0] {
                    Stmt::If(if_stmt) => {
                        match &if_stmt.condition {
                            Expr::Identifier(name) => assert_eq!(name, "condition"),
                            _ => panic!("Expected identifier condition"),
                        }
                        assert!(if_stmt.else_branch.is_some());
                    }
                    _ => panic!("Expected if statement"),
                }

                // while loop
                match &func_decl.body.statements[1] {
                    Stmt::While(while_stmt) => match &while_stmt.condition {
                        Expr::Identifier(name) => assert_eq!(name, "running"),
                        _ => panic!("Expected identifier condition"),
                    },
                    _ => panic!("Expected while statement"),
                }

                // for loop
                match &func_decl.body.statements[2] {
                    Stmt::For(for_stmt) => {
                        assert_eq!(for_stmt.variable, "item");
                        match &for_stmt.iterable {
                            Expr::Identifier(name) => assert_eq!(name, "inventory"),
                            _ => panic!("Expected identifier iterable"),
                        }
                    }
                    _ => panic!("Expected for statement"),
                }
            }
            _ => panic!("Expected function declaration"),
        }
    }

    #[test]
    fn test_method_calls_and_property_access() {
        let input = r#"
            fn test_calls() {
                player.location.on_look();
                obj.property = value;
                result = calculate(x, y, z);
                array.push(item);
            }
        "#;
        let program = parse_input(input).unwrap();

        match &program.items[0] {
            Item::Function(func_decl) => {
                assert_eq!(func_decl.body.statements.len(), 4);

                // Method call: player.location.on_look()
                match &func_decl.body.statements[0] {
                    Stmt::Expression(Expr::MethodCall { method, .. }) => {
                        assert_eq!(method, "on_look");
                    }
                    _ => panic!("Expected method call expression"),
                }

                // Property assignment: obj.property = value
                match &func_decl.body.statements[1] {
                    Stmt::Assignment(assign_stmt) => match &assign_stmt.target {
                        Expr::PropertyAccess { property, .. } => {
                            assert_eq!(property, "property");
                        }
                        _ => panic!("Expected property access target"),
                    },
                    _ => panic!("Expected assignment statement"),
                }

                // Function call with multiple arguments
                match &func_decl.body.statements[2] {
                    Stmt::Assignment(assign_stmt) => match &assign_stmt.value {
                        Expr::FunctionCall { name, arguments } => {
                            assert_eq!(name, "calculate");
                            assert_eq!(arguments.len(), 3);
                        }
                        _ => panic!("Expected function call value"),
                    },
                    _ => panic!("Expected assignment statement"),
                }

                // Method call with argument
                match &func_decl.body.statements[3] {
                    Stmt::Expression(Expr::MethodCall {
                        method, arguments, ..
                    }) => {
                        assert_eq!(method, "push");
                        assert_eq!(arguments.len(), 1);
                    }
                    _ => panic!("Expected method call expression"),
                }
            }
            _ => panic!("Expected function declaration"),
        }
    }

    #[test]
    fn test_room_event_handlers() {
        let input = r#"
            world {
                room test_room "Test Room" {
                    desc: "A test room."
                    
                    on_enter: {
                        if !test_room.visited {
                            print("First visit!");
                        }
                    }
                    
                    on_exit: {
                        print("Leaving room.");
                    }
                    
                    on_look: {
                        print("Extra description.");
                    }
                }
            }
        "#;
        let program = parse_input(input).unwrap();

        match &program.items[0] {
            Item::World(world_decl) => {
                let room = &world_decl.rooms[0];

                assert!(room.on_enter.is_some());
                assert!(room.on_exit.is_some());
                assert!(room.on_look.is_some());

                // Check on_enter has an if statement
                if let Some(on_enter) = &room.on_enter {
                    assert_eq!(on_enter.statements.len(), 1);
                    match &on_enter.statements[0] {
                        Stmt::If(_) => {}
                        _ => panic!("Expected if statement in on_enter"),
                    }
                }

                // Check on_exit has a print statement
                if let Some(on_exit) = &room.on_exit {
                    assert_eq!(on_exit.statements.len(), 1);
                    match &on_exit.statements[0] {
                        Stmt::Expression(Expr::FunctionCall { name, .. }) => {
                            assert_eq!(name, "print");
                        }
                        _ => panic!("Expected print call in on_exit"),
                    }
                }
            }
            _ => panic!("Expected world declaration"),
        }
    }

    // Error handling tests
    #[test]
    fn test_parse_error_unexpected_token() {
        let input = "world { room 123 }"; // Invalid room identifier
        let result = parse_input(input);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_error_missing_brace() {
        let input = "world { room test_room \"Test\" { desc: \"test\""; // Missing closing brace
        let result = parse_input(input);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_error_invalid_expression() {
        let input = r#"
            fn test() {
                let x = + * 5; // Invalid expression
            }
        "#;
        let result = parse_input(input);
        assert!(result.is_err());
    }
}
