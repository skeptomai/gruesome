// Intermediate Representation for Grue Language
//
// The IR is designed to be a lower-level representation that's closer to Z-Machine
// instructions while still maintaining some high-level constructs for optimization.

use crate::grue_compiler::ast::{Program, Type};
use crate::grue_compiler::error::CompilerError;
use std::collections::HashMap;

/// Unique identifier for IR instructions, labels, and temporary variables
pub type IrId = u32;

/// IR Program - top-level container for all IR elements
#[derive(Debug, Clone)]
pub struct IrProgram {
    pub functions: Vec<IrFunction>,
    pub globals: Vec<IrGlobal>,
    pub rooms: Vec<IrRoom>,
    pub objects: Vec<IrObject>,
    pub grammar: Vec<IrGrammar>,
    pub init_block: Option<IrBlock>,
    pub string_table: HashMap<String, IrId>, // String literal -> ID mapping
}

/// IR Function representation
#[derive(Debug, Clone)]
pub struct IrFunction {
    pub id: IrId,
    pub name: String,
    pub parameters: Vec<IrParameter>,
    pub return_type: Option<Type>,
    pub body: IrBlock,
    pub local_vars: Vec<IrLocal>,
}

/// Function parameter in IR
#[derive(Debug, Clone)]
pub struct IrParameter {
    pub name: String,
    pub param_type: Option<Type>,
    pub slot: u8, // Local variable slot in Z-Machine
}

/// Local variable in IR
#[derive(Debug, Clone)]
pub struct IrLocal {
    pub name: String,
    pub var_type: Option<Type>,
    pub slot: u8, // Local variable slot
    pub mutable: bool,
}

/// Global variable in IR
#[derive(Debug, Clone)]
pub struct IrGlobal {
    pub id: IrId,
    pub name: String,
    pub var_type: Option<Type>,
    pub initializer: Option<IrValue>,
}

/// IR Room representation
#[derive(Debug, Clone)]
pub struct IrRoom {
    pub id: IrId,
    pub name: String,
    pub display_name: String,
    pub description: String,
    pub exits: HashMap<String, IrExitTarget>,
    pub on_enter: Option<IrBlock>,
    pub on_exit: Option<IrBlock>,
    pub on_look: Option<IrBlock>,
}

/// IR Object representation
#[derive(Debug, Clone)]
pub struct IrObject {
    pub id: IrId,
    pub name: String,
    pub names: Vec<String>, // Vocabulary names
    pub description: String,
    pub properties: HashMap<String, IrValue>,
    pub parent: Option<IrId>, // Parent object or room
    pub children: Vec<IrId>,  // Child objects
}

/// Exit target in IR
#[derive(Debug, Clone)]
pub enum IrExitTarget {
    Room(IrId),
    Blocked(String),
}

/// Grammar rule in IR
#[derive(Debug, Clone)]
pub struct IrGrammar {
    pub verb: String,
    pub patterns: Vec<IrPattern>,
}

/// Grammar pattern in IR
#[derive(Debug, Clone)]
pub struct IrPattern {
    pub pattern: Vec<IrPatternElement>,
    pub handler: IrHandler,
}

/// Pattern elements in IR
#[derive(Debug, Clone)]
pub enum IrPatternElement {
    Literal(String),
    Noun,
    Default,
}

/// Handler for grammar patterns
#[derive(Debug, Clone)]
pub enum IrHandler {
    FunctionCall(IrId, Vec<IrValue>), // Function ID and arguments
    Block(IrBlock),
}

/// IR Basic Block - contains sequential instructions
#[derive(Debug, Clone)]
pub struct IrBlock {
    pub id: IrId,
    pub instructions: Vec<IrInstruction>,
}

/// IR Instructions - the core IR operations
#[derive(Debug, Clone)]
pub enum IrInstruction {
    /// Load immediate value into temporary
    LoadImmediate { target: IrId, value: IrValue },

    /// Load variable value into temporary
    LoadVar { target: IrId, var_id: IrId },

    /// Store temporary value into variable
    StoreVar { var_id: IrId, source: IrId },

    /// Binary operation
    BinaryOp {
        target: IrId,
        op: IrBinaryOp,
        left: IrId,
        right: IrId,
    },

    /// Unary operation
    UnaryOp {
        target: IrId,
        op: IrUnaryOp,
        operand: IrId,
    },

    /// Function call
    Call {
        target: Option<IrId>, // None for void functions
        function: IrId,
        args: Vec<IrId>,
    },

    /// Return from function
    Return { value: Option<IrId> },

    /// Conditional jump
    Branch {
        condition: IrId,
        true_label: IrId,
        false_label: IrId,
    },

    /// Unconditional jump
    Jump { label: IrId },

    /// Label (jump target)
    Label { id: IrId },

    /// Property access
    GetProperty {
        target: IrId,
        object: IrId,
        property: String,
    },

    /// Property assignment
    SetProperty {
        object: IrId,
        property: String,
        value: IrId,
    },

    /// Array access
    GetArrayElement {
        target: IrId,
        array: IrId,
        index: IrId,
    },

    /// Array assignment
    SetArrayElement {
        array: IrId,
        index: IrId,
        value: IrId,
    },

    /// Print string
    Print { value: IrId },

    /// No-operation (used for optimization)
    Nop,
}

/// IR Values - constants and literals
#[derive(Debug, Clone)]
pub enum IrValue {
    Integer(i16),
    Boolean(bool),
    String(String),
    StringRef(IrId), // Reference to string table entry
    Null,
}

/// Binary operations in IR
#[derive(Debug, Clone, PartialEq)]
pub enum IrBinaryOp {
    // Arithmetic
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,

    // Comparison
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,

    // Logical
    And,
    Or,
}

/// Unary operations in IR
#[derive(Debug, Clone, PartialEq)]
pub enum IrUnaryOp {
    Not,
    Minus,
}

impl IrProgram {
    pub fn new() -> Self {
        Self {
            functions: Vec::new(),
            globals: Vec::new(),
            rooms: Vec::new(),
            objects: Vec::new(),
            grammar: Vec::new(),
            init_block: None,
            string_table: HashMap::new(),
        }
    }

    /// Add a string to the string table and return its ID
    pub fn add_string(&mut self, s: String, id_counter: &mut IrId) -> IrId {
        if let Some(&existing_id) = self.string_table.get(&s) {
            existing_id
        } else {
            let new_id = *id_counter;
            *id_counter += 1;
            self.string_table.insert(s, new_id);
            new_id
        }
    }
}

impl IrBlock {
    pub fn new(id: IrId) -> Self {
        Self {
            id,
            instructions: Vec::new(),
        }
    }

    pub fn add_instruction(&mut self, instruction: IrInstruction) {
        self.instructions.push(instruction);
    }
}

impl Default for IrProgram {
    fn default() -> Self {
        Self::new()
    }
}

/// Convert AST binary operators to IR binary operators
impl From<crate::grue_compiler::ast::BinaryOp> for IrBinaryOp {
    fn from(op: crate::grue_compiler::ast::BinaryOp) -> Self {
        use crate::grue_compiler::ast::BinaryOp as AstOp;
        match op {
            AstOp::Add => IrBinaryOp::Add,
            AstOp::Subtract => IrBinaryOp::Subtract,
            AstOp::Multiply => IrBinaryOp::Multiply,
            AstOp::Divide => IrBinaryOp::Divide,
            AstOp::Modulo => IrBinaryOp::Modulo,
            AstOp::Equal => IrBinaryOp::Equal,
            AstOp::NotEqual => IrBinaryOp::NotEqual,
            AstOp::Less => IrBinaryOp::Less,
            AstOp::LessEqual => IrBinaryOp::LessEqual,
            AstOp::Greater => IrBinaryOp::Greater,
            AstOp::GreaterEqual => IrBinaryOp::GreaterEqual,
            AstOp::And => IrBinaryOp::And,
            AstOp::Or => IrBinaryOp::Or,
        }
    }
}

/// Convert AST unary operators to IR unary operators
impl From<crate::grue_compiler::ast::UnaryOp> for IrUnaryOp {
    fn from(op: crate::grue_compiler::ast::UnaryOp) -> Self {
        use crate::grue_compiler::ast::UnaryOp as AstOp;
        match op {
            AstOp::Not => IrUnaryOp::Not,
            AstOp::Minus => IrUnaryOp::Minus,
        }
    }
}

/// IR Generator - converts AST to IR
pub struct IrGenerator {
    id_counter: IrId,
    symbol_ids: HashMap<String, IrId>, // Symbol name -> IR ID mapping
    current_locals: Vec<IrLocal>,      // Track local variables in current function
    next_local_slot: u8,               // Next available local variable slot
    builtin_functions: HashMap<IrId, String>, // Function ID -> Function name for builtins
    object_numbers: HashMap<String, u16>, // Object name -> Object number mapping
}

impl Default for IrGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl IrGenerator {
    pub fn new() -> Self {
        let mut object_numbers = HashMap::new();
        // Player is always object #1
        object_numbers.insert("player".to_string(), 1);

        IrGenerator {
            id_counter: 1, // Start from 1, 0 is reserved
            symbol_ids: HashMap::new(),
            current_locals: Vec::new(),
            next_local_slot: 1, // Slot 0 reserved for return value
            builtin_functions: HashMap::new(),
            object_numbers,
        }
    }

    /// Check if a function name is a known builtin function
    fn is_builtin_function(&self, name: &str) -> bool {
        matches!(
            name,
            "print"
                | "move"
                | "get_location"
                | "get_child"
                | "get_sibling"
                | "test_attr"
                | "to_string"
        )
    }

    pub fn generate(&mut self, ast: Program) -> Result<IrProgram, CompilerError> {
        log::debug!(
            "IR GENERATOR: Starting IR generation for {} items",
            ast.items.len()
        );
        let mut ir_program = IrProgram::new();

        // Generate IR for each top-level item
        for item in ast.items.iter() {
            self.generate_item(item.clone(), &mut ir_program)?;
        }

        Ok(ir_program)
    }

    /// Get builtin functions discovered during IR generation
    pub fn get_builtin_functions(&self) -> &HashMap<IrId, String> {
        &self.builtin_functions
    }

    pub fn get_object_numbers(&self) -> &HashMap<String, u16> {
        &self.object_numbers
    }

    fn next_id(&mut self) -> IrId {
        let id = self.id_counter;
        self.id_counter += 1;
        id
    }

    fn generate_item(
        &mut self,
        item: crate::grue_compiler::ast::Item,
        ir_program: &mut IrProgram,
    ) -> Result<(), CompilerError> {
        use crate::grue_compiler::ast::Item;

        match item {
            Item::Function(func) => {
                let ir_func = self.generate_function(func)?;
                ir_program.functions.push(ir_func);
            }
            Item::World(world) => {
                self.generate_world(world, ir_program)?;
            }
            Item::Grammar(grammar) => {
                let ir_grammar = self.generate_grammar(grammar)?;
                ir_program.grammar.extend(ir_grammar);
            }
            Item::Init(init) => {
                let ir_block = self.generate_block(init.body)?;
                ir_program.init_block = Some(ir_block);
            }
        }

        Ok(())
    }

    fn generate_function(
        &mut self,
        func: crate::grue_compiler::ast::FunctionDecl,
    ) -> Result<IrFunction, CompilerError> {
        // Check if we already have a placeholder ID for this function
        let func_id = if let Some(&existing_id) = self.symbol_ids.get(&func.name) {
            existing_id
        } else {
            let new_id = self.next_id();
            self.symbol_ids.insert(func.name.clone(), new_id);
            new_id
        };

        // Reset local variable state for this function
        self.current_locals.clear();
        self.next_local_slot = 1; // Slot 0 reserved for return value

        let mut parameters = Vec::new();

        // Add parameters as local variables
        for param in func.parameters {
            let param_id = self.next_id();
            let ir_param = IrParameter {
                name: param.name.clone(),
                param_type: param.param_type.clone(),
                slot: self.next_local_slot,
            };

            // Also track parameters as symbols for identifier resolution
            self.symbol_ids.insert(param.name.clone(), param_id);

            // Add parameter as local variable
            let local_param = IrLocal {
                name: param.name,
                var_type: param.param_type,
                slot: self.next_local_slot,
                mutable: true, // Parameters are typically mutable
            };
            self.current_locals.push(local_param);

            parameters.push(ir_param);
            self.next_local_slot += 1;
        }

        let body = self.generate_block(func.body)?;
        let local_vars = self.current_locals.clone();

        Ok(IrFunction {
            id: func_id,
            name: func.name,
            parameters,
            return_type: func.return_type,
            body,
            local_vars,
        })
    }

    fn generate_world(
        &mut self,
        world: crate::grue_compiler::ast::WorldDecl,
        ir_program: &mut IrProgram,
    ) -> Result<(), CompilerError> {
        for room in world.rooms {
            let ir_room = self.generate_room(room)?;
            ir_program.rooms.push(ir_room);
        }
        Ok(())
    }

    fn register_object_and_nested(
        &mut self,
        obj: &crate::grue_compiler::ast::ObjectDecl,
    ) -> Result<(), CompilerError> {
        // Register the object itself
        let obj_id = self.next_id();
        self.symbol_ids.insert(obj.identifier.clone(), obj_id);

        // Assign object number
        let object_number = self.object_numbers.len() as u16 + 1;
        self.object_numbers
            .insert(obj.identifier.clone(), object_number);

        log::debug!(
            "Registered object '{}' with ID {} and object number {}",
            obj.identifier,
            obj_id,
            object_number
        );

        // Process nested objects recursively
        for nested_obj in &obj.contains {
            self.register_object_and_nested(nested_obj)?;
        }

        Ok(())
    }

    fn generate_room(
        &mut self,
        room: crate::grue_compiler::ast::RoomDecl,
    ) -> Result<IrRoom, CompilerError> {
        let room_id = self.next_id();
        self.symbol_ids.insert(room.identifier.clone(), room_id);

        // Assign object number (rooms start from #2, player is #1)
        let object_number = self.object_numbers.len() as u16 + 1;
        self.object_numbers
            .insert(room.identifier.clone(), object_number);

        let mut exits = HashMap::new();
        for (direction, target) in room.exits {
            let ir_target = match target {
                crate::grue_compiler::ast::ExitTarget::Room(_room_name) => {
                    // We'll resolve room IDs in a later pass
                    IrExitTarget::Room(0) // Placeholder
                }
                crate::grue_compiler::ast::ExitTarget::Blocked(message) => {
                    IrExitTarget::Blocked(message)
                }
            };
            exits.insert(direction, ir_target);
        }

        // Process room objects FIRST - add them to symbol_ids for identifier resolution
        // This must happen before processing handlers that might reference these objects
        log::debug!(
            "Processing {} objects for room '{}'",
            room.objects.len(),
            room.identifier
        );
        for obj in &room.objects {
            self.register_object_and_nested(obj)?;
        }

        // Now process handlers - objects are available for reference
        let on_enter = if let Some(block) = room.on_enter {
            Some(self.generate_block(block)?)
        } else {
            None
        };

        let on_exit = if let Some(block) = room.on_exit {
            Some(self.generate_block(block)?)
        } else {
            None
        };

        let on_look = if let Some(block) = room.on_look {
            Some(self.generate_block(block)?)
        } else {
            None
        };

        Ok(IrRoom {
            id: room_id,
            name: room.identifier,
            display_name: room.display_name,
            description: room.description,
            exits,
            on_enter,
            on_exit,
            on_look,
        })
    }

    fn generate_grammar(
        &mut self,
        grammar: crate::grue_compiler::ast::GrammarDecl,
    ) -> Result<Vec<IrGrammar>, CompilerError> {
        let mut ir_grammar = Vec::new();

        for verb in grammar.verbs {
            let mut patterns = Vec::new();

            for pattern in verb.patterns {
                let ir_pattern_elements: Vec<IrPatternElement> = pattern
                    .pattern
                    .into_iter()
                    .map(|elem| match elem {
                        crate::grue_compiler::ast::PatternElement::Literal(s) => {
                            IrPatternElement::Literal(s)
                        }
                        crate::grue_compiler::ast::PatternElement::Noun => IrPatternElement::Noun,
                        crate::grue_compiler::ast::PatternElement::Default => {
                            IrPatternElement::Default
                        }
                    })
                    .collect();

                let ir_handler = match pattern.handler {
                    crate::grue_compiler::ast::Handler::FunctionCall(_name, args) => {
                        // Convert arguments to IR values
                        let mut ir_args = Vec::new();
                        for arg in args {
                            let ir_value = self.expr_to_ir_value(arg)?;
                            ir_args.push(ir_value);
                        }

                        // Function ID will be resolved in a later pass
                        IrHandler::FunctionCall(0, ir_args) // Placeholder
                    }
                    crate::grue_compiler::ast::Handler::Block(block) => {
                        let ir_block = self.generate_block(block)?;
                        IrHandler::Block(ir_block)
                    }
                };

                patterns.push(IrPattern {
                    pattern: ir_pattern_elements,
                    handler: ir_handler,
                });
            }

            ir_grammar.push(IrGrammar {
                verb: verb.word,
                patterns,
            });
        }

        Ok(ir_grammar)
    }

    fn generate_block(
        &mut self,
        block: crate::grue_compiler::ast::BlockStmt,
    ) -> Result<IrBlock, CompilerError> {
        let block_id = self.next_id();
        let mut ir_block = IrBlock::new(block_id);

        log::debug!(
            "IR generate_block: Processing {} statements",
            block.statements.len()
        );
        for (i, stmt) in block.statements.iter().enumerate() {
            log::debug!(
                "IR generate_block: Processing statement {} of type {:?}",
                i,
                stmt
            );
            self.generate_statement(stmt.clone(), &mut ir_block)?;
        }

        Ok(ir_block)
    }

    fn generate_statement(
        &mut self,
        stmt: crate::grue_compiler::ast::Stmt,
        block: &mut IrBlock,
    ) -> Result<(), CompilerError> {
        use crate::grue_compiler::ast::Stmt;

        match stmt {
            Stmt::Expression(expr) => {
                let _temp = self.generate_expression(expr, block)?;
                // Expression result is discarded
            }
            Stmt::VarDecl(var_decl) => {
                // Generate IR for variable declaration
                let var_id = self.next_id();

                // Add to local variables
                let local_var = IrLocal {
                    name: var_decl.name.clone(),
                    var_type: var_decl.var_type,
                    slot: self.next_local_slot,
                    mutable: var_decl.mutable,
                };
                self.current_locals.push(local_var);
                self.symbol_ids.insert(var_decl.name, var_id);
                self.next_local_slot += 1;

                // Generate initializer if present
                if let Some(initializer) = var_decl.initializer {
                    let init_temp = self.generate_expression(initializer, block)?;
                    block.add_instruction(IrInstruction::StoreVar {
                        var_id,
                        source: init_temp,
                    });
                }
            }
            Stmt::Assignment(assign) => {
                // Generate the value expression
                let value_temp = self.generate_expression(assign.value, block)?;

                // Handle different types of assignment targets
                match assign.target {
                    crate::grue_compiler::ast::Expr::Identifier(var_name) => {
                        // Simple variable assignment
                        if let Some(&var_id) = self.symbol_ids.get(&var_name) {
                            block.add_instruction(IrInstruction::StoreVar {
                                var_id,
                                source: value_temp,
                            });
                        } else {
                            // Variable not found - this should be caught in semantic analysis
                            return Err(CompilerError::SemanticError(
                                format!("Undefined variable '{}' in assignment", var_name),
                                0,
                            ));
                        }
                    }
                    crate::grue_compiler::ast::Expr::PropertyAccess { object, property } => {
                        // Property assignment: object.property = value
                        let object_temp = self.generate_expression(*object, block)?;
                        block.add_instruction(IrInstruction::SetProperty {
                            object: object_temp,
                            property,
                            value: value_temp,
                        });
                    }
                    _ => {
                        // Other assignment targets (array elements, etc.)
                        return Err(CompilerError::SemanticError(
                            "Unsupported assignment target".to_string(),
                            0,
                        ));
                    }
                }
            }
            Stmt::If(if_stmt) => {
                // Generate condition expression
                let condition_temp = self.generate_expression(if_stmt.condition, block)?;

                // Create labels for control flow
                let then_label = self.next_id();
                let else_label = self.next_id();
                let end_label = self.next_id();

                log::debug!(
                    "IR if statement: then={}, else={}, end={}",
                    then_label,
                    else_label,
                    end_label
                );

                // Branch based on condition
                block.add_instruction(IrInstruction::Branch {
                    condition: condition_temp,
                    true_label: then_label,
                    false_label: else_label,
                });

                // Then branch
                log::debug!("IR if: Adding then label {}", then_label);
                block.add_instruction(IrInstruction::Label { id: then_label });
                self.generate_statement(*if_stmt.then_branch, block)?;
                log::debug!("IR if: Adding jump to end label {}", end_label);
                block.add_instruction(IrInstruction::Jump { label: end_label });

                // Else branch (if present)
                log::debug!("IR if: Adding else label {}", else_label);
                block.add_instruction(IrInstruction::Label { id: else_label });
                if let Some(else_branch) = if_stmt.else_branch {
                    self.generate_statement(*else_branch, block)?;
                }

                // End label
                log::debug!("IR if: Adding end label {}", end_label);
                block.add_instruction(IrInstruction::Label { id: end_label });
            }
            Stmt::While(while_stmt) => {
                // Create labels for loop control flow
                let loop_start = self.next_id();
                let loop_body = self.next_id();
                let loop_end = self.next_id();

                // Jump to loop start
                block.add_instruction(IrInstruction::Jump { label: loop_start });

                // Loop start: evaluate condition
                block.add_instruction(IrInstruction::Label { id: loop_start });
                let condition_temp = self.generate_expression(while_stmt.condition, block)?;

                // Branch based on condition
                block.add_instruction(IrInstruction::Branch {
                    condition: condition_temp,
                    true_label: loop_body,
                    false_label: loop_end,
                });

                // Loop body
                block.add_instruction(IrInstruction::Label { id: loop_body });
                self.generate_statement(*while_stmt.body, block)?;
                block.add_instruction(IrInstruction::Jump { label: loop_start });

                // Loop end
                block.add_instruction(IrInstruction::Label { id: loop_end });
            }
            Stmt::For(for_stmt) => {
                // For loops in Grue iterate over collections
                // This is a simplified implementation that assumes array iteration

                // Generate the iterable expression
                let iterable_temp = self.generate_expression(for_stmt.iterable, block)?;

                // Create a loop variable
                let loop_var_id = self.next_id();
                let local_var = IrLocal {
                    name: for_stmt.variable.clone(),
                    var_type: Some(Type::Any), // Type inferred from array elements
                    slot: self.next_local_slot,
                    mutable: false, // Loop variables are immutable
                };
                self.current_locals.push(local_var);
                self.symbol_ids.insert(for_stmt.variable, loop_var_id);
                self.next_local_slot += 1;

                // Create index variable for array iteration
                let index_var = self.next_id();
                let zero_temp = self.next_id();
                block.add_instruction(IrInstruction::LoadImmediate {
                    target: zero_temp,
                    value: IrValue::Integer(0),
                });
                block.add_instruction(IrInstruction::StoreVar {
                    var_id: index_var,
                    source: zero_temp,
                });

                // Create labels for loop control flow
                let loop_start = self.next_id();
                let loop_body = self.next_id();
                let loop_end = self.next_id();

                // Loop start: check if index < array length
                block.add_instruction(IrInstruction::Label { id: loop_start });
                let index_temp = self.next_id();
                block.add_instruction(IrInstruction::LoadVar {
                    target: index_temp,
                    var_id: index_var,
                });

                // For simplicity, we'll use a placeholder condition
                // In a full implementation, we'd check array bounds
                let condition_temp = self.next_id();
                block.add_instruction(IrInstruction::LoadImmediate {
                    target: condition_temp,
                    value: IrValue::Boolean(true), // Placeholder
                });

                // Branch based on condition
                block.add_instruction(IrInstruction::Branch {
                    condition: condition_temp,
                    true_label: loop_body,
                    false_label: loop_end,
                });

                // Loop body: load current element into loop variable
                block.add_instruction(IrInstruction::Label { id: loop_body });
                let element_temp = self.next_id();
                block.add_instruction(IrInstruction::GetArrayElement {
                    target: element_temp,
                    array: iterable_temp,
                    index: index_temp,
                });
                block.add_instruction(IrInstruction::StoreVar {
                    var_id: loop_var_id,
                    source: element_temp,
                });

                // Execute loop body
                self.generate_statement(*for_stmt.body, block)?;

                // Increment index
                let one_temp = self.next_id();
                block.add_instruction(IrInstruction::LoadImmediate {
                    target: one_temp,
                    value: IrValue::Integer(1),
                });
                let new_index = self.next_id();
                block.add_instruction(IrInstruction::BinaryOp {
                    target: new_index,
                    op: IrBinaryOp::Add,
                    left: index_temp,
                    right: one_temp,
                });
                block.add_instruction(IrInstruction::StoreVar {
                    var_id: index_var,
                    source: new_index,
                });

                // Jump back to start
                block.add_instruction(IrInstruction::Jump { label: loop_start });

                // Loop end
                block.add_instruction(IrInstruction::Label { id: loop_end });
            }
            Stmt::Return(return_expr) => {
                let value_id = if let Some(expr) = return_expr {
                    Some(self.generate_expression(expr, block)?)
                } else {
                    None
                };

                block.add_instruction(IrInstruction::Return { value: value_id });
            }
            Stmt::Block(inner_block) => {
                let ir_inner_block = self.generate_block(inner_block)?;
                // Inline the inner block's instructions
                block.instructions.extend(ir_inner_block.instructions);
            }
        }

        Ok(())
    }

    fn generate_expression(
        &mut self,
        expr: crate::grue_compiler::ast::Expr,
        block: &mut IrBlock,
    ) -> Result<IrId, CompilerError> {
        use crate::grue_compiler::ast::Expr;

        match expr {
            Expr::Integer(value) => {
                let temp_id = self.next_id();
                block.add_instruction(IrInstruction::LoadImmediate {
                    target: temp_id,
                    value: IrValue::Integer(value),
                });
                Ok(temp_id)
            }
            Expr::String(value) => {
                let temp_id = self.next_id();
                block.add_instruction(IrInstruction::LoadImmediate {
                    target: temp_id,
                    value: IrValue::String(value),
                });
                Ok(temp_id)
            }
            Expr::Boolean(value) => {
                let temp_id = self.next_id();
                block.add_instruction(IrInstruction::LoadImmediate {
                    target: temp_id,
                    value: IrValue::Boolean(value),
                });
                Ok(temp_id)
            }
            Expr::Identifier(name) => {
                let temp_id = self.next_id();

                // Check if this is an object identifier first
                if let Some(&object_number) = self.object_numbers.get(&name) {
                    // This is an object - load its number as a constant
                    block.add_instruction(IrInstruction::LoadImmediate {
                        target: temp_id,
                        value: IrValue::Integer(object_number as i16),
                    });
                } else if let Some(&var_id) = self.symbol_ids.get(&name) {
                    // This is a variable - load its value
                    block.add_instruction(IrInstruction::LoadVar {
                        target: temp_id,
                        var_id,
                    });
                } else {
                    // Identifier not found - this should be caught during semantic analysis
                    return Err(CompilerError::SemanticError(
                        format!("Undefined identifier '{}'", name),
                        0,
                    ));
                }

                Ok(temp_id)
            }
            Expr::Binary {
                left,
                operator,
                right,
            } => {
                let left_id = self.generate_expression(*left, block)?;
                let right_id = self.generate_expression(*right, block)?;
                let temp_id = self.next_id();

                block.add_instruction(IrInstruction::BinaryOp {
                    target: temp_id,
                    op: operator.into(),
                    left: left_id,
                    right: right_id,
                });

                Ok(temp_id)
            }
            Expr::Unary { operator, operand } => {
                let operand_id = self.generate_expression(*operand, block)?;
                let temp_id = self.next_id();

                block.add_instruction(IrInstruction::UnaryOp {
                    target: temp_id,
                    op: operator.into(),
                    operand: operand_id,
                });

                Ok(temp_id)
            }
            Expr::FunctionCall { name, arguments } => {
                // Generate arguments first
                let mut arg_temps = Vec::new();
                for arg in arguments {
                    let arg_temp = self.generate_expression(arg, block)?;
                    arg_temps.push(arg_temp);
                }

                // Look up function ID (or create placeholder)
                let func_id = if let Some(&id) = self.symbol_ids.get(&name) {
                    id
                } else {
                    // Only register as builtin if it's actually a builtin function
                    let placeholder_id = self.next_id();
                    self.symbol_ids.insert(name.clone(), placeholder_id);
                    if self.is_builtin_function(&name) {
                        self.builtin_functions.insert(placeholder_id, name.clone());
                    }
                    placeholder_id
                };

                let temp_id = self.next_id();
                block.add_instruction(IrInstruction::Call {
                    target: Some(temp_id), // Assume all function calls return something
                    function: func_id,
                    args: arg_temps,
                });

                Ok(temp_id)
            }
            Expr::MethodCall {
                object,
                method,
                arguments,
            } => {
                // Method call: object.method(args)
                // This should be handled as: get property from object, if callable then call it

                // Generate arguments first
                let mut arg_temps = Vec::new();
                for arg in arguments {
                    let arg_temp = self.generate_expression(arg, block)?;
                    arg_temps.push(arg_temp);
                }

                // Generate object expression
                let object_temp = self.generate_expression(*object, block)?;

                // Generate property access to get the method function
                let property_temp = self.next_id();
                block.add_instruction(IrInstruction::GetProperty {
                    target: property_temp,
                    object: object_temp,
                    property: method.clone(),
                });

                // Generate conditional call - only call if property is non-zero (valid function address)
                let result_temp = self.next_id();
                let then_label = self.next_id();
                let else_label = self.next_id();
                let end_label = self.next_id();

                // Branch: if property_temp != 0, goto then_label, else goto else_label
                block.add_instruction(IrInstruction::Branch {
                    condition: property_temp,
                    true_label: then_label,
                    false_label: else_label,
                });

                // Then branch: call the function stored in the property
                block.add_instruction(IrInstruction::Label { id: then_label });
                // TODO: Implement indirect function call via property value
                // For now, set result to 0
                block.add_instruction(IrInstruction::LoadImmediate {
                    target: result_temp,
                    value: IrValue::Integer(0),
                });
                block.add_instruction(IrInstruction::Jump { label: end_label });

                // Else branch: property doesn't exist or isn't callable, return 0
                block.add_instruction(IrInstruction::Label { id: else_label });
                block.add_instruction(IrInstruction::LoadImmediate {
                    target: result_temp,
                    value: IrValue::Integer(0),
                });

                // End label
                block.add_instruction(IrInstruction::Label { id: end_label });

                Ok(result_temp)
            }
            Expr::PropertyAccess { object, property } => {
                // Property access: object.property
                let object_temp = self.generate_expression(*object, block)?;
                let temp_id = self.next_id();

                block.add_instruction(IrInstruction::GetProperty {
                    target: temp_id,
                    object: object_temp,
                    property,
                });

                Ok(temp_id)
            }
            Expr::Array(elements) => {
                // Array literal - for now, we'll create a series of load instructions
                // In a full implementation, this would create an array object
                let mut _temp_ids = Vec::new();
                for element in elements {
                    let element_temp = self.generate_expression(element, block)?;
                    _temp_ids.push(element_temp);
                    // TODO: Store in array structure
                }

                // Return placeholder array ID
                let temp_id = self.next_id();
                Ok(temp_id)
            }
            Expr::Ternary {
                condition,
                true_expr,
                false_expr,
            } => {
                // Ternary conditional: condition ? true_expr : false_expr
                let condition_temp = self.generate_expression(*condition, block)?;

                // Create labels for control flow
                let true_label = self.next_id();
                let false_label = self.next_id();
                let end_label = self.next_id();
                let result_temp = self.next_id();

                // Branch based on condition
                block.add_instruction(IrInstruction::Branch {
                    condition: condition_temp,
                    true_label,
                    false_label,
                });

                // True branch
                block.add_instruction(IrInstruction::Label { id: true_label });
                let true_temp = self.generate_expression(*true_expr, block)?;
                block.add_instruction(IrInstruction::StoreVar {
                    var_id: result_temp,
                    source: true_temp,
                });
                block.add_instruction(IrInstruction::Jump { label: end_label });

                // False branch
                block.add_instruction(IrInstruction::Label { id: false_label });
                let false_temp = self.generate_expression(*false_expr, block)?;
                block.add_instruction(IrInstruction::StoreVar {
                    var_id: result_temp,
                    source: false_temp,
                });

                // End label
                block.add_instruction(IrInstruction::Label { id: end_label });

                // Load result
                let final_temp = self.next_id();
                block.add_instruction(IrInstruction::LoadVar {
                    target: final_temp,
                    var_id: result_temp,
                });

                Ok(final_temp)
            }
            Expr::Parameter(param_name) => {
                // Grammar parameter reference (e.g., $noun)
                let temp_id = self.next_id();
                // For now, just create a placeholder
                // In a full implementation, this would reference the parsed parameter
                block.add_instruction(IrInstruction::LoadImmediate {
                    target: temp_id,
                    value: IrValue::String(param_name),
                });
                Ok(temp_id)
            }
        }
    }

    fn expr_to_ir_value(
        &mut self,
        expr: crate::grue_compiler::ast::Expr,
    ) -> Result<IrValue, CompilerError> {
        use crate::grue_compiler::ast::Expr;

        match expr {
            Expr::Integer(value) => Ok(IrValue::Integer(value)),
            Expr::String(value) => Ok(IrValue::String(value)),
            Expr::Boolean(value) => Ok(IrValue::Boolean(value)),
            _ => {
                // For complex expressions, we'd need to generate temporary instructions
                // For now, return a placeholder
                Ok(IrValue::Null)
            }
        }
    }
}

#[cfg(test)]
#[path = "ir_tests.rs"]
mod tests;
