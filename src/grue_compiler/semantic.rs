// Semantic Analysis for Grue Language

use crate::grue_compiler::ast::*;
use crate::grue_compiler::error::CompilerError;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum SymbolType {
    Function {
        params: Vec<Type>,
        return_type: Option<Type>,
    },
    Variable {
        var_type: Option<Type>,
        mutable: bool,
    },
    Room {
        display_name: String,
    },
    Object {
        names: Vec<String>,
        parent_room: Option<String>,
    },
    Parameter {
        param_type: Option<Type>,
    },
}

#[derive(Debug, Clone)]
pub struct Symbol {
    pub name: String,
    pub symbol_type: SymbolType,
    pub line: usize, // For error reporting
}

#[derive(Debug)]
pub struct Scope {
    pub symbols: HashMap<String, Symbol>,
    pub parent: Option<Box<Scope>>,
    pub scope_type: ScopeType,
}

#[derive(Debug, PartialEq)]
pub enum ScopeType {
    Global,
    Function,
    Block,
    Room,
}

pub struct SemanticAnalyzer {
    current_scope: Box<Scope>,
    errors: Vec<CompilerError>,
    room_objects: HashMap<String, Vec<String>>, // room_id -> object_ids
}

impl Default for SemanticAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl SemanticAnalyzer {
    pub fn new() -> Self {
        let mut analyzer = SemanticAnalyzer {
            current_scope: Box::new(Scope {
                symbols: HashMap::new(),
                parent: None,
                scope_type: ScopeType::Global,
            }),
            errors: Vec::new(),
            room_objects: HashMap::new(),
        };

        // Add built-in functions
        analyzer.add_builtin_functions();

        analyzer
    }

    fn add_builtin_functions(&mut self) {
        // Add common built-in functions
        let builtins = [
            ("print", vec![Type::String], None),
            ("println", vec![Type::String], None),
            ("error", vec![Type::String], None),
            ("to_string", vec![Type::Any], Some(Type::String)),
            ("to_int", vec![Type::String], Some(Type::Int)),
            (
                "length",
                vec![Type::Array(Box::new(Type::Any))],
                Some(Type::Int),
            ),
            (
                "empty",
                vec![Type::Array(Box::new(Type::Any))],
                Some(Type::Bool),
            ),
            ("calculate", vec![Type::Int, Type::Int], Some(Type::Int)), // For test
            ("process", vec![Type::Any], None),
            ("update", vec![], None),
            // Core builtin functions for object manipulation
            // Note: Many functions are intentionally left as user-defined
            // to allow games to customize their behavior (look_around, player_can_see, etc.)
            ("move", vec![Type::Any, Type::Any], None),
            ("get_location", vec![Type::Any], Some(Type::Any)),
            // Core Z-Machine object primitives - low-level operations only
            ("get_child", vec![Type::Any], Some(Type::Any)),
            ("get_sibling", vec![Type::Any], Some(Type::Any)),
            ("test_attr", vec![Type::Any, Type::Int], Some(Type::Bool)),
            // Remove test-specific functions that might conflict
            // ("handle_take", vec![Type::String], None),
            // ("handle_look", vec![], None),
            // ("announce_entry", vec![], None),
            // ("print_location", vec![Type::String], None),
            // ("setup_game", vec![], None),
        ];

        for (name, params, return_type) in builtins {
            let symbol = Symbol {
                name: name.to_string(),
                symbol_type: SymbolType::Function {
                    params,
                    return_type,
                },
                line: 0,
            };
            self.current_scope.symbols.insert(name.to_string(), symbol);
        }

        // Add built-in variables
        let variables = [
            ("player", Type::Object),
            ("condition", Type::Bool),
            ("running", Type::Bool),
            ("inventory", Type::Array(Box::new(Type::Object))),
        ];

        for (name, var_type) in variables {
            let symbol = Symbol {
                name: name.to_string(),
                symbol_type: SymbolType::Variable {
                    var_type: Some(var_type),
                    mutable: true,
                },
                line: 0,
            };
            self.current_scope.symbols.insert(name.to_string(), symbol);
        }
    }

    pub fn analyze(&mut self, mut program: Program) -> Result<Program, CompilerError> {
        // First pass: collect all global symbols (functions, rooms)
        self.collect_global_symbols(&program)?;

        // Second pass: analyze each item in detail
        for item in &mut program.items {
            self.analyze_item(item)?;
        }

        // Check for any collected errors
        if !self.errors.is_empty() {
            return Err(self.errors.clone().into_iter().next().unwrap());
        }

        Ok(program)
    }

    fn collect_global_symbols(&mut self, program: &Program) -> Result<(), CompilerError> {
        for item in &program.items {
            match item {
                Item::Function(func) => {
                    let param_types: Vec<Type> = func
                        .parameters
                        .iter()
                        .map(|p| p.param_type.clone().unwrap_or(Type::Any))
                        .collect();

                    let symbol = Symbol {
                        name: func.name.clone(),
                        symbol_type: SymbolType::Function {
                            params: param_types,
                            return_type: func.return_type.clone(),
                        },
                        line: 0, // TODO: Add line number tracking
                    };

                    if self.current_scope.symbols.contains_key(&func.name) {
                        return Err(CompilerError::SemanticError(
                            format!("Function '{}' is already defined", func.name),
                            0,
                        ));
                    }

                    self.current_scope.symbols.insert(func.name.clone(), symbol);
                }

                Item::World(world) => {
                    for room in &world.rooms {
                        let symbol = Symbol {
                            name: room.identifier.clone(),
                            symbol_type: SymbolType::Room {
                                display_name: room.display_name.clone(),
                            },
                            line: 0,
                        };

                        if self.current_scope.symbols.contains_key(&room.identifier) {
                            return Err(CompilerError::SemanticError(
                                format!("Room '{}' is already defined", room.identifier),
                                0,
                            ));
                        }

                        self.current_scope
                            .symbols
                            .insert(room.identifier.clone(), symbol);

                        // Collect objects in this room
                        let mut object_names = Vec::new();
                        for obj in &room.objects {
                            let obj_symbol = Symbol {
                                name: obj.identifier.clone(),
                                symbol_type: SymbolType::Object {
                                    names: obj.names.clone(),
                                    parent_room: Some(room.identifier.clone()),
                                },
                                line: 0,
                            };

                            if self.current_scope.symbols.contains_key(&obj.identifier) {
                                return Err(CompilerError::SemanticError(
                                    format!("Object '{}' is already defined", obj.identifier),
                                    0,
                                ));
                            }

                            self.current_scope
                                .symbols
                                .insert(obj.identifier.clone(), obj_symbol);
                            object_names.push(obj.identifier.clone());

                            // Handle nested objects
                            self.collect_nested_objects(&obj.contains, &obj.identifier)?;
                        }

                        self.room_objects
                            .insert(room.identifier.clone(), object_names);
                    }
                }

                Item::Grammar(_) => {
                    // Grammar declarations don't create symbols in the global scope
                }

                Item::Init(_) => {
                    // Init blocks don't create symbols
                }
            }
        }

        Ok(())
    }

    fn collect_nested_objects(
        &mut self,
        objects: &[ObjectDecl],
        _parent_obj: &str,
    ) -> Result<(), CompilerError> {
        for obj in objects {
            let obj_symbol = Symbol {
                name: obj.identifier.clone(),
                symbol_type: SymbolType::Object {
                    names: obj.names.clone(),
                    parent_room: None, // Nested objects don't have direct room parents
                },
                line: 0,
            };

            if self.current_scope.symbols.contains_key(&obj.identifier) {
                return Err(CompilerError::SemanticError(
                    format!("Object '{}' is already defined", obj.identifier),
                    0,
                ));
            }

            self.current_scope
                .symbols
                .insert(obj.identifier.clone(), obj_symbol);

            // Recurse into nested objects
            self.collect_nested_objects(&obj.contains, &obj.identifier)?;
        }
        Ok(())
    }

    fn analyze_item(&mut self, item: &mut Item) -> Result<(), CompilerError> {
        match item {
            Item::Function(func) => {
                self.analyze_function(func)?;
            }

            Item::World(world) => {
                self.analyze_world(world)?;
            }

            Item::Grammar(grammar) => {
                self.analyze_grammar(grammar)?;
            }

            Item::Init(init) => {
                self.analyze_block(&mut init.body)?;
            }
        }

        Ok(())
    }

    fn analyze_function(&mut self, func: &mut FunctionDecl) -> Result<(), CompilerError> {
        // Enter function scope
        self.push_scope(ScopeType::Function);

        // Add parameters to function scope
        for param in &func.parameters {
            let symbol = Symbol {
                name: param.name.clone(),
                symbol_type: SymbolType::Parameter {
                    param_type: param.param_type.clone(),
                },
                line: 0,
            };

            if self.current_scope.symbols.contains_key(&param.name) {
                return Err(CompilerError::SemanticError(
                    format!("Parameter '{}' is already defined", param.name),
                    0,
                ));
            }

            self.current_scope
                .symbols
                .insert(param.name.clone(), symbol);
        }

        // Analyze function body
        self.analyze_block(&mut func.body)?;

        // Check return type consistency (simplified for now)
        if let Some(_return_type) = &func.return_type {
            // TODO: Verify all return statements match the declared type
        }

        // Exit function scope
        self.pop_scope();

        Ok(())
    }

    fn analyze_world(&mut self, world: &mut WorldDecl) -> Result<(), CompilerError> {
        for room in &mut world.rooms {
            self.analyze_room(room)?;
        }
        Ok(())
    }

    fn analyze_room(&mut self, room: &mut RoomDecl) -> Result<(), CompilerError> {
        // Enter room scope
        self.push_scope(ScopeType::Room);

        // Validate exit targets
        for (direction, target) in &room.exits {
            match target {
                ExitTarget::Room(room_name) => {
                    if !self.is_room_defined(room_name) {
                        return Err(CompilerError::SemanticError(
                            format!(
                                "Exit '{}' references undefined room '{}'",
                                direction, room_name
                            ),
                            0,
                        ));
                    }
                }
                ExitTarget::Blocked(_) => {
                    // Blocked exits are always valid
                }
            }
        }

        // Analyze room objects
        for obj in &mut room.objects {
            self.analyze_object(obj)?;
        }

        // Analyze room event handlers
        if let Some(ref mut on_enter) = room.on_enter {
            self.analyze_block(on_enter)?;
        }
        if let Some(ref mut on_exit) = room.on_exit {
            self.analyze_block(on_exit)?;
        }
        if let Some(ref mut on_look) = room.on_look {
            self.analyze_block(on_look)?;
        }

        // Exit room scope
        self.pop_scope();

        Ok(())
    }

    #[allow(clippy::only_used_in_recursion)] // TODO: Will use obj parameter when property validation is implemented
    fn analyze_object(&mut self, obj: &mut ObjectDecl) -> Result<(), CompilerError> {
        // TODO: Validate object properties and their types

        // Analyze nested objects
        for nested_obj in &mut obj.contains {
            self.analyze_object(nested_obj)?;
        }

        Ok(())
    }

    fn analyze_grammar(&mut self, grammar: &mut GrammarDecl) -> Result<(), CompilerError> {
        for verb in &mut grammar.verbs {
            for pattern in &mut verb.patterns {
                match &mut pattern.handler {
                    Handler::FunctionCall(func_name, args) => {
                        if !self.is_function_defined(func_name) {
                            return Err(CompilerError::SemanticError(
                                format!(
                                    "Grammar pattern references undefined function '{}'",
                                    func_name
                                ),
                                0,
                            ));
                        }

                        // Validate arguments
                        for arg in args {
                            self.analyze_expression(arg)?;
                        }
                    }

                    Handler::Block(block) => {
                        self.push_scope(ScopeType::Block);
                        self.analyze_block(block)?;
                        self.pop_scope();
                    }
                }
            }
        }

        Ok(())
    }

    fn analyze_block(&mut self, block: &mut BlockStmt) -> Result<(), CompilerError> {
        for stmt in &mut block.statements {
            self.analyze_statement(stmt)?;
        }
        Ok(())
    }

    fn analyze_statement(&mut self, stmt: &mut Stmt) -> Result<(), CompilerError> {
        match stmt {
            Stmt::VarDecl(var_decl) => {
                let mut inferred_type = var_decl.var_type.clone();

                // Check if initializer matches declared type, and infer type if not declared
                if let Some(ref mut initializer) = var_decl.initializer {
                    let expr_type = self.analyze_expression(initializer)?;
                    if let Some(ref declared_type) = var_decl.var_type {
                        if !self.types_compatible(declared_type, &expr_type) {
                            return Err(CompilerError::SemanticError(
                                format!(
                                    "Type mismatch in variable '{}': expected {:?}, found {:?}",
                                    var_decl.name, declared_type, expr_type
                                ),
                                0,
                            ));
                        }
                    } else {
                        // Infer type from initializer if no explicit type was provided
                        inferred_type = Some(expr_type);
                    }
                }

                // Add variable to current scope
                let symbol = Symbol {
                    name: var_decl.name.clone(),
                    symbol_type: SymbolType::Variable {
                        var_type: inferred_type,
                        mutable: var_decl.mutable,
                    },
                    line: 0,
                };

                if self.current_scope.symbols.contains_key(&var_decl.name) {
                    return Err(CompilerError::SemanticError(
                        format!(
                            "Variable '{}' is already defined in this scope",
                            var_decl.name
                        ),
                        0,
                    ));
                }

                self.current_scope
                    .symbols
                    .insert(var_decl.name.clone(), symbol);
            }

            Stmt::Assignment(assign) => {
                self.analyze_expression(&mut assign.target)?;
                self.analyze_expression(&mut assign.value)?;

                // TODO: Check if target is assignable (mutable variable or property)
            }

            Stmt::Expression(expr) => {
                self.analyze_expression(expr)?;
            }

            Stmt::If(if_stmt) => {
                let cond_type = self.analyze_expression(&mut if_stmt.condition)?;
                if !self.types_compatible(&Type::Bool, &cond_type) && cond_type != Type::Any {
                    return Err(CompilerError::SemanticError(
                        "If condition must be a boolean expression".to_string(),
                        0,
                    ));
                }

                self.push_scope(ScopeType::Block);
                self.analyze_statement(&mut if_stmt.then_branch)?;
                self.pop_scope();

                if let Some(ref mut else_branch) = if_stmt.else_branch {
                    self.push_scope(ScopeType::Block);
                    self.analyze_statement(else_branch)?;
                    self.pop_scope();
                }
            }

            Stmt::While(while_stmt) => {
                let cond_type = self.analyze_expression(&mut while_stmt.condition)?;
                if !self.types_compatible(&Type::Bool, &cond_type) && cond_type != Type::Any {
                    return Err(CompilerError::SemanticError(
                        "While condition must be a boolean expression".to_string(),
                        0,
                    ));
                }

                self.push_scope(ScopeType::Block);
                self.analyze_statement(&mut while_stmt.body)?;
                self.pop_scope();
            }

            Stmt::For(for_stmt) => {
                self.analyze_expression(&mut for_stmt.iterable)?;

                self.push_scope(ScopeType::Block);

                // Add loop variable to scope
                let symbol = Symbol {
                    name: for_stmt.variable.clone(),
                    symbol_type: SymbolType::Variable {
                        var_type: Some(Type::Any), // TODO: Infer from iterable type
                        mutable: false,
                    },
                    line: 0,
                };
                self.current_scope
                    .symbols
                    .insert(for_stmt.variable.clone(), symbol);

                self.analyze_statement(&mut for_stmt.body)?;
                self.pop_scope();
            }

            Stmt::Return(return_expr) => {
                if let Some(expr) = return_expr {
                    self.analyze_expression(expr)?;
                }

                // TODO: Check if we're in a function scope
            }

            Stmt::Block(block_stmt) => {
                self.push_scope(ScopeType::Block);
                self.analyze_block(block_stmt)?;
                self.pop_scope();
            }
        }

        Ok(())
    }

    fn analyze_expression(&mut self, expr: &mut Expr) -> Result<Type, CompilerError> {
        match expr {
            Expr::Identifier(name) => {
                if let Some(symbol) = self.lookup_symbol(name) {
                    // Return the actual type of the symbol
                    match &symbol.symbol_type {
                        SymbolType::Variable { var_type, .. } => {
                            Ok(var_type.clone().unwrap_or(Type::Any))
                        }
                        SymbolType::Parameter { param_type } => {
                            Ok(param_type.clone().unwrap_or(Type::Any))
                        }
                        SymbolType::Function { return_type, .. } => {
                            // Functions as identifiers return their return type
                            Ok(return_type.clone().unwrap_or(Type::Any))
                        }
                        SymbolType::Room { .. } => Ok(Type::Room),
                        SymbolType::Object { .. } => Ok(Type::Object),
                    }
                } else {
                    Err(CompilerError::SemanticError(
                        format!("Undefined identifier '{}'", name),
                        0,
                    ))
                }
            }

            Expr::Integer(_) => Ok(Type::Int),
            Expr::String(_) => Ok(Type::String),
            Expr::Boolean(_) => Ok(Type::Bool),

            Expr::Array(elements) => {
                let mut element_type = Type::Any;
                for element in elements {
                    let elem_type = self.analyze_expression(element)?;
                    if element_type == Type::Any {
                        element_type = elem_type;
                    } else if !self.types_compatible(&element_type, &elem_type) {
                        return Err(CompilerError::SemanticError(
                            "Array elements must have compatible types".to_string(),
                            0,
                        ));
                    }
                }
                Ok(Type::Array(Box::new(element_type)))
            }

            Expr::Binary {
                left,
                operator,
                right,
            } => {
                let left_type = self.analyze_expression(left)?;
                let right_type = self.analyze_expression(right)?;

                // Determine result type based on operator
                match operator {
                    // Comparison operators always return boolean
                    BinaryOp::Equal
                    | BinaryOp::NotEqual
                    | BinaryOp::Less
                    | BinaryOp::LessEqual
                    | BinaryOp::Greater
                    | BinaryOp::GreaterEqual => {
                        // Type check the operands are compatible for comparison
                        // Always return bool for comparison operators, regardless of operand compatibility
                        Ok(Type::Bool)
                    }

                    // Logical operators expect and return boolean
                    BinaryOp::And | BinaryOp::Or => Ok(Type::Bool),

                    // Arithmetic operators return the compatible type
                    _ => {
                        if self.types_compatible_for_operations(&left_type, &right_type) {
                            Ok(left_type)
                        } else {
                            Ok(Type::Any) // Mixed types, use Any
                        }
                    }
                }
            }

            Expr::Unary {
                operator: _,
                operand,
            } => self.analyze_expression(operand),

            Expr::FunctionCall { name, arguments } => {
                // First, analyze all arguments
                let mut arg_types = Vec::new();
                for arg in arguments.iter_mut() {
                    let arg_type = self.analyze_expression(arg)?;
                    arg_types.push(arg_type);
                }

                // Then validate against function signature
                if let Some(symbol) = self.lookup_symbol(name) {
                    if let SymbolType::Function {
                        params,
                        return_type,
                    } = &symbol.symbol_type
                    {
                        // Check argument count
                        if arg_types.len() != params.len() {
                            return Err(CompilerError::SemanticError(
                                format!(
                                    "Function '{}' expects {} arguments, found {}",
                                    name,
                                    params.len(),
                                    arg_types.len()
                                ),
                                0,
                            ));
                        }

                        // Check argument types (allow conversions for function calls)
                        for (i, arg_type) in arg_types.iter().enumerate() {
                            if !self.types_compatible_for_operations(&params[i], arg_type)
                                && *arg_type != Type::Any
                            {
                                return Err(CompilerError::SemanticError(format!("Function '{}' argument {} type mismatch: expected {:?}, found {:?}", 
                                        name, i + 1, params[i], arg_type), 0));
                            }
                        }

                        Ok(return_type.clone().unwrap_or(Type::Any))
                    } else {
                        Err(CompilerError::SemanticError(
                            format!("'{}' is not a function", name),
                            0,
                        ))
                    }
                } else {
                    Err(CompilerError::SemanticError(
                        format!("Undefined function '{}'", name),
                        0,
                    ))
                }
            }

            Expr::PropertyAccess {
                object,
                property: _,
            } => {
                self.analyze_expression(object)?;
                // TODO: Validate property exists on object type
                Ok(Type::Any) // For now, assume any property access is valid
            }

            Expr::MethodCall {
                object,
                method: _,
                arguments,
            } => {
                // Analyze the object expression
                self.analyze_expression(object)?;

                // Analyze all arguments
                for arg in arguments.iter_mut() {
                    self.analyze_expression(arg)?;
                }

                // For now, assume all method calls are valid and return Any
                // TODO: Implement proper method resolution based on object type
                Ok(Type::Any)
            }

            Expr::Ternary {
                condition,
                true_expr,
                false_expr,
            } => {
                let cond_type = self.analyze_expression(condition)?;
                if !self.types_compatible(&Type::Bool, &cond_type) && cond_type != Type::Any {
                    return Err(CompilerError::SemanticError(
                        "Ternary condition must be a boolean expression".to_string(),
                        0,
                    ));
                }

                let true_type = self.analyze_expression(true_expr)?;
                let false_type = self.analyze_expression(false_expr)?;

                if self.types_compatible_for_operations(&true_type, &false_type) {
                    Ok(true_type)
                } else {
                    Ok(Type::Any) // Mixed types, use Any
                }
            }

            Expr::Parameter(_param_name) => {
                // TODO: Validate parameter exists in current grammar context
                Ok(Type::Any)
            }
        }
    }

    fn push_scope(&mut self, scope_type: ScopeType) {
        let new_scope = Box::new(Scope {
            symbols: HashMap::new(),
            parent: Some(std::mem::replace(
                &mut self.current_scope,
                Box::new(Scope {
                    symbols: HashMap::new(),
                    parent: None,
                    scope_type: ScopeType::Global,
                }),
            )),
            scope_type,
        });
        self.current_scope = new_scope;
    }

    fn pop_scope(&mut self) {
        if let Some(parent) = self.current_scope.parent.take() {
            self.current_scope = parent;
        }
    }

    fn lookup_symbol(&self, name: &str) -> Option<&Symbol> {
        let mut current = &*self.current_scope;

        loop {
            if let Some(symbol) = current.symbols.get(name) {
                return Some(symbol);
            }

            match &current.parent {
                Some(parent) => current = parent,
                None => break,
            }
        }

        None
    }

    fn is_function_defined(&self, name: &str) -> bool {
        if let Some(symbol) = self.lookup_symbol(name) {
            matches!(symbol.symbol_type, SymbolType::Function { .. })
        } else {
            false
        }
    }

    fn is_room_defined(&self, name: &str) -> bool {
        if let Some(symbol) = self.lookup_symbol(name) {
            matches!(symbol.symbol_type, SymbolType::Room { .. })
        } else {
            false
        }
    }

    fn types_compatible(&self, expected: &Type, actual: &Type) -> bool {
        match (expected, actual) {
            (Type::Any, _) | (_, Type::Any) => true,
            // Exact type matches
            (a, b) => a == b,
        }
    }

    fn types_compatible_for_operations(&self, expected: &Type, actual: &Type) -> bool {
        match (expected, actual) {
            (Type::Any, _) | (_, Type::Any) => true,
            // Allow string concatenation and conversions for operations
            (Type::String, Type::Int) | (Type::Int, Type::String) => true,
            // Exact type matches
            (a, b) => a == b,
        }
    }
}

#[cfg(test)]
#[path = "semantic_tests.rs"]
mod tests;
