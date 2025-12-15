// IR Generator - converts AST to IR
//
// This module contains the IrGenerator struct and all its implementation methods
// for transforming AST into Intermediate Representation.

use crate::grue_compiler::ast::{ObjectSpecialization, Program, Type};
use crate::grue_compiler::error::CompilerError;
use indexmap::IndexMap;

use super::{
    ExpressionContext, FunctionOverload, IrAttributes, IrBinaryOp, IrBlock, IrExitTarget,
    IrFunction, IrGrammar, IrHandler, IrId, IrIdRegistry, IrInstruction, IrLocal, IrObject,
    IrParameter, IrPattern, IrPatternElement, IrProgram, IrProperties, IrRoom, IrValue,
    PropertyManager, RoomObjectInfo, StandardAttribute, StandardProperty, VariableSource,
};

/// IR Generator - converts AST to IR
pub struct IrGenerator {
    id_counter: IrId,
    pub(super) symbol_ids: IndexMap<String, IrId>, // Symbol name -> IR ID mapping
    pub(super) current_locals: Vec<IrLocal>,       // Track local variables in current function
    pub(super) next_local_slot: u8,                // Next available local variable slot
    builtin_functions: IndexMap<IrId, String>,     // Function ID -> Function name for builtins
    pub(super) object_numbers: IndexMap<String, u16>, // Object name -> Object number mapping
    object_counter: u16, // Next available object number (starts at 2, player is 1)
    property_manager: PropertyManager, // Manages property numbering and inheritance
    id_registry: IrIdRegistry, // NEW: Track all IR IDs for debugging and mapping
    variable_sources: IndexMap<IrId, VariableSource>, // Track variable origins for iteration strategy
    expression_types: IndexMap<IrId, Type>, // NEW: Track expression result types for StringAddress system
    /// Mapping of room names to objects contained within them
    /// Used for automatic object placement during init block generation
    pub(super) room_objects: IndexMap<String, Vec<RoomObjectInfo>>,

    // Polymorphic dispatch support
    pub(super) function_overloads: IndexMap<String, Vec<FunctionOverload>>, // Function name -> list of overloads
    pub(super) dispatch_functions: IndexMap<String, IrId>, // Function name -> dispatch function ID
    pub(super) function_base_names: IndexMap<IrId, String>, // Function ID -> base function name
    pub(super) function_id_map: IndexMap<(String, ObjectSpecialization), u32>, // (name, specialization) -> assigned ID
}

impl Default for IrGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl IrGenerator {
    pub fn new() -> Self {
        let mut object_numbers = IndexMap::new();
        // Player is always object #1
        object_numbers.insert("player".to_string(), 1);

        IrGenerator {
            id_counter: 1, // Start from 1, 0 is reserved
            symbol_ids: IndexMap::new(),
            current_locals: Vec::new(),
            next_local_slot: 1, // Slot 0 reserved for return value
            builtin_functions: IndexMap::new(),
            object_numbers,
            object_counter: 2, // Start at 2, player is object #1
            property_manager: PropertyManager::new(),
            id_registry: IrIdRegistry::new(), // NEW: Initialize ID registry
            variable_sources: IndexMap::new(), // NEW: Initialize variable source tracking
            expression_types: IndexMap::new(), // NEW: Initialize expression type tracking for StringAddress system
            room_objects: IndexMap::new(),     // NEW: Initialize room object mapping

            // Polymorphic dispatch support
            function_overloads: IndexMap::new(),
            dispatch_functions: IndexMap::new(),
            function_base_names: IndexMap::new(),
            function_id_map: IndexMap::new(),
        }
    }

    /// Check if a function name is a known builtin function

    // Function generation methods moved to ir_gen_functions.rs:
    // - mangle_function_name()
    // - detect_specialization()
    // - register_function_overload()
    // - generate_dispatch_functions()
    // - create_dispatch_function()
    // - generate_function()

    pub fn generate(&mut self, ast: Program) -> Result<IrProgram, CompilerError> {
        log::debug!(
            "IR GENERATOR: Starting IR generation for {} items",
            ast.items.len()
        );
        let mut ir_program = IrProgram::new();

        // Detect program mode from AST
        let program_mode = ast.detect_program_mode();
        ir_program.program_mode = program_mode.clone();
        log::debug!("Detected program mode: {:?}", program_mode);

        // THREE-PASS APPROACH: Fixed function dispatch timing bug
        //
        // PROBLEM: Function dispatch creation happened AFTER function body generation,
        // causing generic functions to infinitely call themselves instead of dispatch functions.
        //
        // SOLUTION: Pre-register all functions and create dispatch functions BEFORE body generation
        //
        // PASS 1: Register all function names and detect which ones have overloads
        // DETERMINISM FIX: Use IndexMap instead of HashMap for consistent function ordering
        // This ensures dispatch functions are generated in the same order across builds
        let mut function_counts: IndexMap<String, Vec<(u32, ObjectSpecialization)>> =
            IndexMap::new();

        // First, count all functions and their specializations
        for item in ast.items.iter() {
            if let crate::grue_compiler::ast::Item::Function(func) = item {
                let func_id = self.next_id();
                let specialization = self.detect_specialization(&func.name, &func.parameters);

                function_counts
                    .entry(func.name.clone())
                    .or_default()
                    .push((func_id, specialization));
            }
        }

        // Track individual function IDs for reuse in Pass 2
        // This prevents ID mismatches between registration and generation phases
        let mut function_id_map: IndexMap<(String, ObjectSpecialization), u32> = IndexMap::new();

        // Now register functions appropriately
        for (name, versions) in function_counts.iter() {
            if versions.len() > 1 {
                // This function has overloads - register each version
                for (func_id, specialization) in versions {
                    self.register_function_overload(name, *func_id, specialization.clone());
                    function_id_map.insert((name.clone(), specialization.clone()), *func_id);
                }

                // Find the generic version for symbol_ids (or use the first if no generic)
                let primary_id = versions
                    .iter()
                    .find(|(_, spec)| matches!(spec, ObjectSpecialization::Generic))
                    .map(|(id, _)| *id)
                    .unwrap_or(versions[0].0);

                self.symbol_ids.insert(name.clone(), primary_id);
                log::debug!(
                    "PASS 1: Registered overloaded function '{}' with {} versions, primary ID {}",
                    name,
                    versions.len(),
                    primary_id
                );
            } else {
                // Single function - just register in symbol_ids
                let (func_id, specialization) = &versions[0];
                self.symbol_ids.insert(name.clone(), *func_id);
                function_id_map.insert((name.clone(), specialization.clone()), *func_id);
                log::debug!(
                    "PASS 1: Registered single function '{}' (specialization: {:?}) with ID {}",
                    name,
                    specialization,
                    func_id
                );
            }
        }

        // Store the function ID map for use in generate_function
        self.function_id_map = function_id_map;

        // PASS 1.5: Pre-allocate dispatch function IDs for overloaded functions
        // This ensures function calls during body generation can resolve to dispatch functions
        // instead of calling generic functions infinitely
        for (base_name, overloads) in &self.function_overloads.clone() {
            if overloads.len() > 1 {
                let dispatch_id = self.next_id();
                self.dispatch_functions
                    .insert(base_name.clone(), dispatch_id);
                log::debug!(
                    "PASS 1.5: Pre-allocated dispatch function ID {} for overloaded '{}'",
                    dispatch_id,
                    base_name
                );
            }
        }

        // PASS 2: Generate IR for all items except grammar (functions will now use registered IDs)
        let mut deferred_grammar = Vec::new();
        for item in ast.items.iter() {
            match item {
                crate::grue_compiler::ast::Item::Grammar(grammar) => {
                    // Defer grammar processing until after dispatch functions are created
                    deferred_grammar.push(grammar.clone());
                }
                _ => {
                    self.generate_item(item.clone(), &mut ir_program)?;
                }
            }
        }

        // Add synthetic player object to IR
        // The player is object #1 and needs to be in the IR like all other objects
        self.add_player_object(&mut ir_program)?;

        // Copy symbol mappings from generator to IR program for use in codegen
        ir_program.symbol_ids = self.symbol_ids.clone();
        ir_program.object_numbers = self.object_numbers.clone();
        ir_program.id_registry = self.id_registry.clone(); // NEW: Transfer ID registry
        ir_program.property_manager = self.property_manager.clone(); // Transfer property manager with consistent mappings
        ir_program.expression_types = self.expression_types.clone(); // NEW: Transfer expression types for StringAddress system

        // PASS 3: Generate dispatch function bodies (IDs already allocated in Pass 1.5)
        self.generate_dispatch_functions(&mut ir_program)?;

        // Now process deferred grammar items with dispatch functions available
        log::debug!(
            "Processing {} deferred grammar items with dispatch functions available",
            deferred_grammar.len()
        );
        for grammar in deferred_grammar {
            let ir_grammar = self.generate_grammar(grammar)?;
            ir_program.grammar.extend(ir_grammar);
        }

        Ok(ir_program)
    }

    /// Get builtin functions discovered during IR generation
    pub fn get_builtin_functions(&self) -> &IndexMap<IrId, String> {
        &self.builtin_functions
    }

    /// Get dispatch functions generated during IR generation
    pub fn get_dispatch_functions(&self) -> &IndexMap<String, IrId> {
        &self.dispatch_functions
    }

    /// Get function base names for dispatch lookup
    pub fn get_function_base_names(&self) -> &IndexMap<IrId, String> {
        &self.function_base_names
    }

    pub fn get_object_numbers(&self) -> &IndexMap<String, u16> {
        &self.object_numbers
    }

    /// Check if a property name corresponds to a standard Z-Machine property
    fn get_standard_property(&self, property_name: &str) -> Option<StandardProperty> {
        match property_name {
            "short_name" | "name" => Some(StandardProperty::ShortName),
            "long_name" => Some(StandardProperty::LongName),
            "desc" | "description" => Some(StandardProperty::Description),
            "initial" => Some(StandardProperty::Initial),
            "before" => Some(StandardProperty::Before),
            "after" => Some(StandardProperty::After),
            "life" => Some(StandardProperty::Life),
            "capacity" => Some(StandardProperty::Capacity),
            "value" => Some(StandardProperty::Value),
            "size" => Some(StandardProperty::Size),
            "article" => Some(StandardProperty::Article),
            "adjective" => Some(StandardProperty::Adjective),
            _ => None,
        }
    }

    /// Map property names to standard Z-Machine attributes
    fn get_standard_attribute(&self, property_name: &str) -> Option<StandardAttribute> {
        match property_name {
            "invisible" => Some(StandardAttribute::Invisible),
            "container" => Some(StandardAttribute::Container),
            "openable" => Some(StandardAttribute::Openable),
            "open" => Some(StandardAttribute::Open),
            "takeable" => Some(StandardAttribute::Takeable),
            "moved" => Some(StandardAttribute::Moved),
            "worn" => Some(StandardAttribute::Worn),
            "light_source" => Some(StandardAttribute::LightSource),
            "visited" => Some(StandardAttribute::Visited),
            "locked" => Some(StandardAttribute::Locked),
            "edible" => Some(StandardAttribute::Edible),
            "treasure" => Some(StandardAttribute::Treasure),
            "special" => Some(StandardAttribute::Special),
            "transparent" => Some(StandardAttribute::Transparent),
            "on" => Some(StandardAttribute::On),
            "workflag" => Some(StandardAttribute::Workflag),
            _ => None,
        }
    }

    pub(super) fn next_id(&mut self) -> IrId {
        let id = self.id_counter;
        self.id_counter += 1;
        id
    }

    /// Centralized IR instruction emission with automatic ID tracking
    /// This ensures all IR IDs are properly registered for codegen mapping
    fn emit_ir_instruction(&mut self, block: &mut IrBlock, instruction: IrInstruction) -> IrId {
        let target_id = match &instruction {
            // Extract target ID from instructions that create new values
            IrInstruction::LoadImmediate { target, .. } => Some(*target),
            IrInstruction::LoadVar { target, .. } => Some(*target),
            IrInstruction::StoreVar { var_id, .. } => Some(*var_id),
            IrInstruction::BinaryOp { target, .. } => Some(*target),
            IrInstruction::UnaryOp { target, .. } => Some(*target),
            IrInstruction::Call { target, .. } => *target,
            IrInstruction::GetProperty { target, .. } => Some(*target),
            IrInstruction::GetPropertyByNumber { target, .. } => Some(*target),
            #[allow(deprecated)]
            IrInstruction::TestAttribute { target, .. } => Some(*target),
            IrInstruction::TestAttributeBranch { .. } => None, // Branch instructions don't produce values
            IrInstruction::TestAttributeValue { target, .. } => Some(*target),
            IrInstruction::SetProperty { .. } => None,
            IrInstruction::SetPropertyByNumber { .. } => None,
            IrInstruction::SetAttribute { .. } => None,
            IrInstruction::Jump { .. } => None,
            IrInstruction::Label { .. } => None,
            IrInstruction::Return { .. } => None,
            IrInstruction::CreateArray { target, .. } => Some(*target),
            IrInstruction::GetArrayElement { target, .. } => Some(*target),
            _ => None,
        };

        // Track this IR ID and its type for debugging and mapping
        if let Some(tid) = target_id {
            let instruction_type = match &instruction {
                IrInstruction::LoadImmediate { .. } => "LoadImmediate",
                IrInstruction::LoadVar { .. } => "LoadVar",
                IrInstruction::StoreVar { .. } => "StoreVar",
                IrInstruction::BinaryOp { .. } => "BinaryOp",
                IrInstruction::UnaryOp { .. } => "UnaryOp",
                IrInstruction::Call { .. } => "Call",
                IrInstruction::GetProperty { .. } => "GetProperty",
                IrInstruction::GetPropertyByNumber { .. } => "GetPropertyByNumber",
                #[allow(deprecated)]
                IrInstruction::TestAttribute { .. } => "TestAttribute",
                IrInstruction::TestAttributeBranch { .. } => "TestAttributeBranch",
                IrInstruction::TestAttributeValue { .. } => "TestAttributeValue",
                IrInstruction::CreateArray { .. } => "CreateArray",
                IrInstruction::GetArrayElement { .. } => "GetArrayElement",
                _ => "Other",
            };

            // Register this IR ID in the centralized registry
            self.id_registry
                .register_expression_id(tid, instruction_type);

            log::debug!(
                "IR EMISSION: ID {} <- {} instruction",
                tid,
                instruction_type
            );

            // Debug: Track problematic ID range
            if (80..=100).contains(&tid) {
                log::warn!(
                    "TRACKING PROBLEMATIC ID {}: {} instruction",
                    tid,
                    instruction_type
                );
            }
        }

        block.add_instruction(instruction);
        target_id.unwrap_or(0) // Return the target ID or 0 if no target
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
                let mut ir_block = self.generate_block(init.body)?;

                // Phase 1c: Inject object placement instructions after user's init code
                self.generate_object_placement_instructions(&mut ir_block)?;

                ir_program.init_block = Some(ir_block);

                // Save local variables declared in init block (e.g., let statements)
                // These need to be tracked separately since init blocks are IrBlock not IrFunction
                ir_program.init_block_locals = self.current_locals.clone();
                self.current_locals.clear(); // Clear for next function/block
                self.next_local_slot = 1; // Reset slot counter
            }
            Item::Mode(_mode) => {
                // Mode declarations are handled during program mode detection in generate()
                // No IR generation needed for the mode declaration itself
            }
            Item::Messages(messages) => {
                // Process system messages catalog for localization support
                for (key, value) in &messages.messages {
                    ir_program
                        .system_messages
                        .insert(key.clone(), value.clone());
                }
            }
        }

        Ok(())
    }

    // generate_function() moved to ir_gen_functions.rs

    /// Recursively assign object numbers to all objects in all rooms

    pub(super) fn register_object_and_nested(
        &mut self,
        obj: &crate::grue_compiler::ast::ObjectDecl,
    ) -> Result<(), CompilerError> {
        // Register the object itself
        let obj_id = self.next_id();
        self.symbol_ids.insert(obj.identifier.clone(), obj_id);

        // NOTE: Object number assignment deferred to assign_object_numbers_recursively()
        // This ensures systematic ordering: Player(#1) -> All Rooms(#2-N) -> All Objects(#N+1-M)

        log::debug!(
            "Registered object '{}' with ID {} (object number will be assigned systematically later)",
            obj.identifier,
            obj_id
        );

        // Process nested objects recursively
        for nested_obj in &obj.contains {
            self.register_object_and_nested(nested_obj)?;
        }

        Ok(())
    }

    /// Extract object hierarchy from AST ObjectDecl for room object mapping
    /// Converts ObjectDecl and its nested objects into RoomObjectInfo structure
    pub(super) fn extract_object_hierarchy(
        &self,
        obj: &crate::grue_compiler::ast::ObjectDecl,
    ) -> RoomObjectInfo {
        // Extract nested objects recursively
        let nested_objects: Vec<RoomObjectInfo> = obj
            .contains
            .iter()
            .map(|nested_obj| self.extract_object_hierarchy(nested_obj))
            .collect();

        RoomObjectInfo {
            name: obj.identifier.clone(),
            nested_objects,
        }
    }

    /// Generate InsertObj instructions from room_objects mapping for init block

    /// Generate InsertObj instructions for a single object and its nested objects

    /// Add synthetic player object to IR program

    // generate_room() moved to ir_gen_rooms.rs

    // generate_grammar() moved to ir_gen_grammar.rs

    pub(super) fn generate_block(
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

    /// Testing method to expose room_objects mapping for integration tests
    #[cfg(test)]
    pub fn get_room_objects(&self) -> &IndexMap<String, Vec<RoomObjectInfo>> {
        &self.room_objects
    }
}

// Extracted modules for functional organization
mod ir_gen_builtins;
mod ir_gen_expressions;
mod ir_gen_functions;
mod ir_gen_grammar;
mod ir_gen_objects;
mod ir_gen_rooms;
mod ir_gen_statements;
