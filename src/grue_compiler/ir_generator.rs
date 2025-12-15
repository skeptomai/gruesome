// IR Generator - converts AST to IR
//
// This module contains the IrGenerator struct and all its implementation methods
// for transforming AST into Intermediate Representation.

use crate::grue_compiler::ast::{Expr, ObjectSpecialization, Program, Type};
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
    builtin_functions: IndexMap<IrId, String>, // Function ID -> Function name for builtins
    pub(super) object_numbers: IndexMap<String, u16>, // Object name -> Object number mapping
    object_counter: u16,                // Next available object number (starts at 2, player is 1)
    property_manager: PropertyManager,  // Manages property numbering and inheritance
    id_registry: IrIdRegistry,          // NEW: Track all IR IDs for debugging and mapping
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
    fn is_builtin_function(&self, name: &str) -> bool {
        #[cfg(debug_assertions)]
        {
            matches!(
                name,
                "print"
                    | "print_num"
                    | "println"
                    | "print_ret"
                    | "print_message"
                    | "new_line"
                    | "move"
                    | "add_score"
                    | "subtract_score"
                    | "word_to_number"
                    | "get_location"
                    | "get_child"
                    | "get_sibling"
                    | "get_prop"
                    | "test_attr"
                    | "set_attr"
                    | "clear_attr"
                    | "to_string"
                    | "random"
                    // String utility functions
                    | "indexOf"
                    | "slice"
                    | "substring"
                    | "toLowerCase"
                    | "toUpperCase"
                    | "trim"
                    | "charAt"
                    | "split"
                    | "replace"
                    | "startsWith"
                    | "endsWith"
                    // Math utility functions
                    | "abs"
                    | "min"
                    | "max"
                    | "round"
                    | "floor"
                    | "ceil"
                    // Type checking functions
                    | "is_string"
                    | "is_int"
                    | "is_bool"
                    | "is_array"
                    | "is_object"
                    | "typeof"
                    // Game control
                    | "quit"
                    // Debug breakpoints (debug builds only)
                    | "debug_break"
            )
        }
        #[cfg(not(debug_assertions))]
        {
            matches!(
                name,
                "print"
                    | "print_num"
                    | "println"
                    | "print_ret"
                    | "print_message"
                    | "new_line"
                    | "move"
                    | "add_score"
                    | "subtract_score"
                    | "word_to_number"
                    | "get_location"
                    | "get_child"
                    | "get_sibling"
                    | "get_prop"
                    | "test_attr"
                    | "set_attr"
                    | "clear_attr"
                    | "to_string"
                    | "random"
                    // String utility functions
                    | "indexOf"
                    | "slice"
                    | "substring"
                    | "toLowerCase"
                    | "toUpperCase"
                    | "trim"
                    | "charAt"
                    | "split"
                    | "replace"
                    | "startsWith"
                    | "endsWith"
                    // Math utility functions
                    | "abs"
                    | "min"
                    | "max"
                    | "round"
                    | "floor"
                    | "ceil"
                    // Type checking functions
                    | "is_string"
                    | "is_int"
                    | "is_bool"
                    | "is_array"
                    | "is_object"
                    | "typeof"
                    // Game control
                    | "quit"
            )
        }
    }

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

    /// Record the type of an IR result for StringAddress system
    fn record_expression_type(&mut self, ir_id: IrId, type_info: Type) {
        self.expression_types.insert(ir_id, type_info.clone());
        log::debug!("IR_TYPE: IR ID {} has type {:?}", ir_id, type_info);
    }

    /// Get the type of an IR expression for StringAddress system
    fn get_expression_type(&self, ir_id: IrId) -> Option<&Type> {
        self.expression_types.get(&ir_id)
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

    fn generate_world(
        &mut self,
        world: crate::grue_compiler::ast::WorldDecl,
        ir_program: &mut IrProgram,
    ) -> Result<(), CompilerError> {
        // First pass: register all rooms and objects for symbol resolution
        // IMPORTANT: Defer object numbering to match code generator's systematic ordering
        for room in &world.rooms {
            let room_id = self.next_id();
            self.symbol_ids.insert(room.identifier.clone(), room_id);
            // Register in the centralized registry as a named symbol
            self.id_registry
                .register_id(room_id, "room", "generate_world", false);

            // NOTE: Object number assignment deferred to match code generator ordering

            // Register all objects in the room
            for obj in &room.objects {
                self.register_object_and_nested(obj)?;
            }
        }

        // Second pass: Assign object numbers systematically to match code generator
        // Order: Player(#1) -> All Rooms(#2-N) -> All Objects(#N+1-M)
        let mut object_counter = 2; // Start at 2, player is already #1

        // Assign numbers to all rooms first
        for room in &world.rooms {
            self.object_numbers
                .insert(room.identifier.clone(), object_counter);
            log::debug!(
                "🔢 IR Object Numbering: Room '{}' -> #{}",
                room.identifier,
                object_counter
            );
            object_counter += 1;
        }

        // Then assign numbers to all objects
        self.assign_object_numbers_recursively(&world.rooms, &mut object_counter)?;

        // Update the object_counter field to reflect the final count
        self.object_counter = object_counter;

        // Third pass: generate actual IR objects and rooms
        for room in world.rooms {
            let ir_room = self.generate_room(room.clone())?;
            let room_id = ir_room.id; // Save the room ID before moving ir_room
            ir_program.rooms.push(ir_room);

            // Generate IR objects for this room
            for obj in room.objects {
                let ir_objects = self.generate_object(obj, Some(room_id))?;
                ir_program.objects.extend(ir_objects);
            }
        }

        // Set up property defaults for common properties
        self.setup_property_defaults(ir_program);

        Ok(())
    }

    /// Recursively assign object numbers to all objects in all rooms
    /// This ensures systematic ordering: Player(#1) -> Rooms(#2-N) -> Objects(#N+1-M)
    fn assign_object_numbers_recursively(
        &mut self,
        rooms: &[crate::grue_compiler::ast::RoomDecl],
        object_counter: &mut u16,
    ) -> Result<(), CompilerError> {
        for room in rooms {
            for obj in &room.objects {
                self.assign_object_number_to_object_and_nested(obj, object_counter)?;
            }
        }
        Ok(())
    }

    /// Assign object number to an object and all its nested objects
    fn assign_object_number_to_object_and_nested(
        &mut self,
        obj: &crate::grue_compiler::ast::ObjectDecl,
        object_counter: &mut u16,
    ) -> Result<(), CompilerError> {
        // Assign number to this object
        self.object_numbers
            .insert(obj.identifier.clone(), *object_counter);
        log::debug!(
            "🔢 IR Object Numbering: Object '{}' -> #{}",
            obj.identifier,
            *object_counter
        );
        *object_counter += 1;

        // Recursively assign numbers to nested objects
        for nested_obj in &obj.contains {
            self.assign_object_number_to_object_and_nested(nested_obj, object_counter)?;
        }

        Ok(())
    }

    /// Set up default values for standard Z-Machine properties
    fn setup_property_defaults(&self, ir_program: &mut IrProgram) {
        // Set sensible defaults for common properties
        if let Some(short_name_num) = self
            .property_manager
            .get_standard_property_number(StandardProperty::ShortName)
        {
            ir_program.property_defaults.set_default(short_name_num, 0); // Empty string by default
        }

        if let Some(capacity_num) = self
            .property_manager
            .get_standard_property_number(StandardProperty::Capacity)
        {
            ir_program.property_defaults.set_default(capacity_num, 100); // Default container capacity
        }

        if let Some(value_num) = self
            .property_manager
            .get_standard_property_number(StandardProperty::Value)
        {
            ir_program.property_defaults.set_default(value_num, 0); // Default object value
        }

        if let Some(size_num) = self
            .property_manager
            .get_standard_property_number(StandardProperty::Size)
        {
            ir_program.property_defaults.set_default(size_num, 5); // Default object size
        }
    }

    fn generate_object(
        &mut self,
        obj: crate::grue_compiler::ast::ObjectDecl,
        parent_id: Option<IrId>,
    ) -> Result<Vec<IrObject>, CompilerError> {
        let mut result = Vec::new();

        // Get the object ID that was registered earlier
        let obj_id = *self
            .symbol_ids
            .get(&obj.identifier)
            .ok_or_else(|| CompilerError::UndefinedSymbol(obj.identifier.clone(), 0))?;

        // Convert named attributes to Z-Machine attributes
        // These are attributes declared with syntax: attributes: ["openable", "container"]
        let mut attributes = IrAttributes::new();
        for attr_name in &obj.attributes {
            match attr_name.as_str() {
                "openable" => attributes.set(StandardAttribute::Openable as u8, true),
                "container" => attributes.set(StandardAttribute::Container as u8, true),
                "takeable" => attributes.set(StandardAttribute::Takeable as u8, true),
                "light_source" => attributes.set(StandardAttribute::LightSource as u8, true),
                "treasure" => attributes.set(StandardAttribute::Treasure as u8, true),
                "edible" => attributes.set(StandardAttribute::Edible as u8, true),
                "worn" => attributes.set(StandardAttribute::Worn as u8, true),
                "locked" => attributes.set(StandardAttribute::Locked as u8, true),
                "transparent" => attributes.set(StandardAttribute::Transparent as u8, true),
                _ => {
                    log::warn!(
                        "Unknown attribute '{}' on object '{}'",
                        attr_name,
                        obj.identifier
                    );
                }
            }
        }

        // Set attributes based on properties (for backward compatibility)
        // These are boolean properties declared with syntax: openable: true
        // This allows both attribute and property syntax to set the same Z-Machine attributes
        for (prop_name, prop_value) in &obj.properties {
            match prop_name.as_str() {
                "openable" => {
                    if let crate::grue_compiler::ast::PropertyValue::Boolean(val) = prop_value {
                        attributes.set(StandardAttribute::Openable as u8, *val);
                    }
                }
                "open" => {
                    if let crate::grue_compiler::ast::PropertyValue::Boolean(val) = prop_value {
                        attributes.set(StandardAttribute::Open as u8, *val);
                    }
                }
                "container" => {
                    if let crate::grue_compiler::ast::PropertyValue::Boolean(val) = prop_value {
                        attributes.set(StandardAttribute::Container as u8, *val);
                    }
                }
                "takeable" => {
                    if let crate::grue_compiler::ast::PropertyValue::Boolean(val) = prop_value {
                        attributes.set(StandardAttribute::Takeable as u8, *val);
                    }
                }
                _ => {} // Other properties handled below
            }
        }

        // CRITICAL FIX (Oct 28, 2025): Object name property must use first name from names array
        // Previously used obj.identifier which caused "mailbox" instead of "small mailbox"
        // Bug: obj.name accessed short_name property which was set incorrectly
        // Fix: Use first name from names array, falling back to identifier if names is empty
        let short_name = obj
            .names
            .first()
            .cloned()
            .unwrap_or_else(|| obj.identifier.clone());

        // Convert properties to Z-Machine properties
        let mut properties = IrProperties::new();

        // Set standard properties using computed short_name (not obj.identifier!)
        properties.set_string(StandardProperty::ShortName as u8, short_name.clone());
        properties.set_string(StandardProperty::Description as u8, obj.description.clone());

        // Convert AST properties to Z-Machine properties using property manager
        for (prop_name, prop_value) in &obj.properties {
            let prop_num = self.property_manager.get_property_number(prop_name);
            match prop_value {
                crate::grue_compiler::ast::PropertyValue::Boolean(val) => {
                    properties.set_byte(prop_num, if *val { 1 } else { 0 });
                }
                crate::grue_compiler::ast::PropertyValue::Integer(val) => {
                    if *val >= 0 {
                        properties.set_word(prop_num, *val as u16);
                    }
                }
                crate::grue_compiler::ast::PropertyValue::String(val) => {
                    properties.set_string(prop_num, val.clone());
                }
                crate::grue_compiler::ast::PropertyValue::Byte(val) => {
                    properties.set_byte(prop_num, *val);
                }
                crate::grue_compiler::ast::PropertyValue::Bytes(val) => {
                    properties.set_bytes(prop_num, val.clone());
                }
                crate::grue_compiler::ast::PropertyValue::Object(obj_name) => {
                    // Convert object reference to object number when available
                    if let Some(&obj_num) = self.object_numbers.get(obj_name) {
                        properties.set_word(prop_num, obj_num);
                    }
                }
                crate::grue_compiler::ast::PropertyValue::Room(room_name) => {
                    // Convert room reference to room number when available
                    if let Some(&room_num) = self.object_numbers.get(room_name) {
                        properties.set_word(prop_num, room_num);
                    }
                }
            }
        }

        // Convert numbered properties
        for (prop_num, prop_value) in &obj.numbered_properties {
            match prop_value {
                crate::grue_compiler::ast::PropertyValue::Byte(val) => {
                    properties.set_byte(*prop_num, *val);
                }
                crate::grue_compiler::ast::PropertyValue::Integer(val) => {
                    if *val >= 0 {
                        properties.set_word(*prop_num, *val as u16);
                    }
                }
                crate::grue_compiler::ast::PropertyValue::String(val) => {
                    properties.set_string(*prop_num, val.clone());
                }
                crate::grue_compiler::ast::PropertyValue::Bytes(val) => {
                    properties.set_bytes(*prop_num, val.clone());
                }
                crate::grue_compiler::ast::PropertyValue::Object(obj_name) => {
                    // Convert object reference to object number when available
                    if let Some(&obj_num) = self.object_numbers.get(obj_name) {
                        properties.set_word(*prop_num, obj_num);
                    }
                }
                crate::grue_compiler::ast::PropertyValue::Room(room_name) => {
                    // Convert room reference to room number when available
                    if let Some(&room_num) = self.object_numbers.get(room_name) {
                        properties.set_word(*prop_num, room_num);
                    }
                }
                crate::grue_compiler::ast::PropertyValue::Boolean(_) => {
                    // Already handled above, but included for exhaustiveness
                }
            }
        }

        // Process contains relationship - convert to parent/child relationships
        let mut child_objects = Vec::new();
        for contained_obj in obj.contains {
            let child_ir_objects = self.generate_object(contained_obj, Some(obj_id))?;
            for child in &child_ir_objects {
                child_objects.push(child.id);
            }
            result.extend(child_ir_objects);
        }

        // Build the sibling chain for children
        let first_child = child_objects.first().copied();
        for i in 0..child_objects.len() {
            let next_sibling = if i + 1 < child_objects.len() {
                Some(child_objects[i + 1])
            } else {
                None
            };

            // Find the child in result and update its sibling
            if let Some(child) = result.iter_mut().find(|obj| obj.id == child_objects[i]) {
                child.sibling = next_sibling;
            }
        }

        let ir_object = IrObject {
            id: obj_id,
            name: obj.identifier,
            short_name,
            names: obj.names,
            description: obj.description,
            attributes,
            properties,
            parent: parent_id,
            sibling: None, // Will be set when building sibling chains
            child: first_child,
            comprehensive_object: None, // Will be set when enhanced object system is integrated
        };

        result.insert(0, ir_object);
        Ok(result)
    }

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
    /// Converts room object hierarchies to InsertObj instructions to establish object tree
    fn generate_object_placement_instructions(
        &self,
        block: &mut IrBlock,
    ) -> Result<(), CompilerError> {
        log::debug!(
            "Phase 1c: Generating object placement instructions for {} rooms",
            self.room_objects.len()
        );

        for (room_name, objects) in &self.room_objects {
            // Look up room IR ID from symbol table
            let room_ir_id = *self.symbol_ids.get(room_name).ok_or_else(|| {
                CompilerError::CodeGenError(format!(
                    "Room '{}' not found in symbol table during object placement",
                    room_name
                ))
            })?;

            log::debug!(
                "Phase 1c: Placing {} objects in room '{}' (IR ID {})",
                objects.len(),
                room_name,
                room_ir_id
            );

            // Generate placement instructions for each object in this room
            for object_info in objects {
                self.generate_placement_for_object(object_info, room_ir_id, block)?;
            }
        }

        Ok(())
    }

    /// Generate InsertObj instructions for a single object and its nested objects
    /// Recursively handles object containment hierarchy
    fn generate_placement_for_object(
        &self,
        object_info: &RoomObjectInfo,
        container_ir_id: u32,
        block: &mut IrBlock,
    ) -> Result<(), CompilerError> {
        // Look up object IR ID from symbol table
        let object_ir_id = *self.symbol_ids.get(&object_info.name).ok_or_else(|| {
            CompilerError::CodeGenError(format!(
                "Object '{}' not found in symbol table during placement",
                object_info.name
            ))
        })?;

        // Generate InsertObj instruction to place this object in its container
        block.instructions.push(IrInstruction::InsertObj {
            object: object_ir_id,
            destination: container_ir_id,
        });

        log::debug!(
            "Phase 1c: Generated InsertObj for '{}' (IR {}) into container (IR {})",
            object_info.name,
            object_ir_id,
            container_ir_id
        );

        // Recursively handle nested objects (they go inside this object)
        for nested_object in &object_info.nested_objects {
            self.generate_placement_for_object(nested_object, object_ir_id, block)?;
        }

        Ok(())
    }

    /// Add synthetic player object to IR program
    /// The player is always object #1 and has standard properties
    fn add_player_object(&mut self, ir_program: &mut IrProgram) -> Result<(), CompilerError> {
        // Create player object with ID 9999 (high ID to avoid conflicts)
        let player_id = 9999u32;

        // Register player in symbol table
        self.symbol_ids.insert("player".to_string(), player_id);

        // Player is always object #1 in Z-Machine
        // (Object numbers were incremented for rooms/objects, but player is inserted first during codegen)

        // Create player properties
        let mut player_properties = IrProperties::new();

        // Get property numbers from property manager
        let location_prop = self.property_manager.get_property_number("location");
        let desc_prop = self
            .property_manager
            .get_property_number_by_name("description")
            .or_else(|| self.property_manager.get_property_number_by_name("desc"))
            .unwrap_or(7); // Default to property 7 if not found

        // Set initial player location to first room (will be room object #2 during codegen)
        let initial_location = if !ir_program.rooms.is_empty() { 2 } else { 0 };
        player_properties.set_word(location_prop, initial_location);

        // Set player description
        player_properties.set_string(desc_prop, "yourself".to_string());

        // Add quit_pending property for quit confirmation flow
        let quit_pending_prop = self.property_manager.get_property_number("quit_pending");
        player_properties.set_word(quit_pending_prop, 0); // Initially false

        // Create player object
        // BUG FIX (Oct 11, 2025): Set player's initial parent to match location property
        // Since player.location now reads from object tree (get_parent), not property,
        // we must initialize the tree parent to match the location property value
        let initial_parent = if !ir_program.rooms.is_empty() {
            // Player starts in first room, which will be object #2 (player is #1)
            // Store IR ID of first room as parent
            Some(ir_program.rooms[0].id)
        } else {
            None
        };

        let player_object = IrObject {
            id: player_id,
            name: "player".to_string(),
            short_name: "yourself".to_string(),
            description: String::new(), // Description is in properties
            names: vec!["yourself".to_string()],
            attributes: IrAttributes::new(),
            properties: player_properties,
            parent: initial_parent, // Start as child of first room
            sibling: None,
            child: None, // Player can contain objects (inventory)
            comprehensive_object: None,
        };

        // Add player as first object (it will become object #1 during codegen)
        ir_program.objects.insert(0, player_object);

        log::debug!(
            "Added synthetic player object with ID {} (will be object #1)",
            player_id
        );

        Ok(())
    }

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

    fn generate_object_tree_iteration(
        &mut self,
        for_stmt: Box<crate::grue_compiler::ast::ForStmt>,
        block: &mut IrBlock,
    ) -> Result<(), CompilerError> {
        use crate::grue_compiler::ast::Expr;

        // Extract the object from the contents() method call
        let container_object = if let Expr::MethodCall { object, .. } = for_stmt.iterable {
            self.generate_expression(*object, block)?
        } else {
            return Err(CompilerError::CodeGenError(
                "Expected MethodCall for object tree iteration".to_string(),
            ));
        };

        self.generate_object_tree_iteration_with_container(
            for_stmt.variable,
            *for_stmt.body,
            container_object,
            block,
        )
    }

    fn generate_object_tree_iteration_with_container(
        &mut self,
        variable: String,
        body: crate::grue_compiler::ast::Stmt,
        container_object: IrId,
        block: &mut IrBlock,
    ) -> Result<(), CompilerError> {
        // Create loop variable for current object
        let loop_var_id = self.next_id();
        let local_var = IrLocal {
            ir_id: loop_var_id,
            name: variable.clone(),
            var_type: Some(Type::Any),
            slot: self.next_local_slot,
            mutable: false,
        };
        self.current_locals.push(local_var);
        self.symbol_ids.insert(variable, loop_var_id);
        self.next_local_slot += 1;

        // Create current_object variable to track iteration
        let current_obj_var = self.next_id();
        let current_obj_local = IrLocal {
            ir_id: current_obj_var,
            name: format!("__current_obj_{}", current_obj_var),
            var_type: Some(Type::Int),
            slot: self.next_local_slot,
            mutable: true,
        };
        self.current_locals.push(current_obj_local);
        self.next_local_slot += 1;

        // Create loop counter variable to prevent infinite loops
        let loop_counter_var = self.next_id();
        let loop_counter_local = IrLocal {
            ir_id: loop_counter_var,
            name: format!("__loop_counter_{}", loop_counter_var),
            var_type: Some(Type::Int),
            slot: self.next_local_slot,
            mutable: true,
        };
        self.current_locals.push(loop_counter_local);
        self.next_local_slot += 1;

        // Create labels
        let loop_start = self.next_id();
        let loop_body = self.next_id();
        let loop_end = self.next_id();

        // Get first child: current = get_child(container)
        // IR GetObjectChild branches when there's NO child (parameter semantics)
        let first_child_temp = self.next_id();
        block.add_instruction(IrInstruction::GetObjectChild {
            target: first_child_temp,
            object: container_object,
            branch_if_no_child: loop_end, // Skip loop if container has no children
        });
        block.add_instruction(IrInstruction::StoreVar {
            var_id: current_obj_var,
            source: first_child_temp,
        });

        // Loop start label (we continue here after processing each child)
        block.add_instruction(IrInstruction::Label { id: loop_start });

        // Loop body: set loop variable = current object
        block.add_instruction(IrInstruction::Label { id: loop_body });
        let current_for_body = self.next_id();
        block.add_instruction(IrInstruction::LoadVar {
            target: current_for_body,
            var_id: current_obj_var,
        });
        block.add_instruction(IrInstruction::StoreVar {
            var_id: loop_var_id,
            source: current_for_body,
        });

        // Execute loop body
        self.generate_statement(body, block)?;

        // Get next sibling: current = get_sibling(current)
        let current_for_sibling = self.next_id();
        block.add_instruction(IrInstruction::LoadVar {
            target: current_for_sibling,
            var_id: current_obj_var,
        });
        let next_sibling_temp = self.next_id();

        // Get sibling and branch to loop_end if no more siblings
        // Z-Machine get_sibling returns sibling object and branches when sibling==0
        block.add_instruction(IrInstruction::GetObjectSibling {
            target: next_sibling_temp,
            object: current_for_sibling,
            branch_if_no_sibling: loop_end, // Exit loop when no more siblings
        });

        // Store the sibling as the new current object
        block.add_instruction(IrInstruction::StoreVar {
            var_id: current_obj_var,
            source: next_sibling_temp,
        });

        // Jump back to loop start to process this sibling
        block.add_instruction(IrInstruction::Jump { label: loop_start });

        // Loop end
        block.add_instruction(IrInstruction::Label { id: loop_end });

        Ok(())
    }

    /// Generate InsertObj instructions to place room objects in their containing rooms
    /// Phase 1: Place objects defined inside rooms (e.g., mailbox in west_of_house)
    fn generate_room_object_placement(
        &mut self,
        _block: &mut IrBlock,
    ) -> Result<(), CompilerError> {
        log::debug!("🏠 Generating room object placement instructions");

        // We need access to room data, but it's not stored in self after generation
        // For now, implement a simple approach: track room->objects during generation

        // TODO: Implement room object placement logic
        log::warn!("🚧 generate_room_object_placement: Not yet implemented");

        Ok(())
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
                    ir_id: var_id,
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

                    // Copy variable source from initializer to variable for iteration tracking
                    // This enables for-loops to detect object tree iteration even with variable assignments
                    if let Some(source) = self.variable_sources.get(&init_temp).cloned() {
                        log::debug!(
                            "VarDecl: copying variable source {:?} from init_temp {} to var_id {}",
                            source,
                            init_temp,
                            var_id
                        );
                        self.variable_sources.insert(var_id, source);
                    }
                }
            }
            Stmt::Assignment(assign) => {
                // Generate the value expression with value context
                let value_temp = self.generate_expression_with_context(
                    assign.value.clone(),
                    block,
                    ExpressionContext::Value,
                )?;

                // Handle different types of assignment targets
                match assign.target {
                    crate::grue_compiler::ast::Expr::Identifier(var_name) => {
                        // Simple variable assignment
                        if let Some(&var_id) = self.symbol_ids.get(&var_name) {
                            block.add_instruction(IrInstruction::StoreVar {
                                var_id,
                                source: value_temp,
                            });

                            // CRITICAL FIX (Oct 27, 2025): Copy variable source tracking from value_temp to var_id
                            // This enables for-loop detection to work with variable indirection
                            // e.g., let items = obj.contents(); for item in items
                            // Without this, ObjectTreeRoot source is lost during assignment and for-loop
                            // falls back to array iteration instead of object tree iteration
                            if let Some(source) = self.variable_sources.get(&value_temp).cloned() {
                                log::debug!(
                                    "Assignment: copying variable source {:?} from value_temp {} to var_id {}",
                                    source,
                                    value_temp,
                                    var_id
                                );
                                self.variable_sources.insert(var_id, source);
                            }
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
                        let object_temp = self.generate_expression_with_context(
                            *object,
                            block,
                            ExpressionContext::Value,
                        )?;

                        // Special handling for .location assignment - use insert_obj instead of property
                        // (Oct 12, 2025): Location is object tree containment only, not a property
                        if property == "location" {
                            log::debug!(
                                "🏃 LOCATION_WRITE: Using InsertObj for .location assignment"
                            );
                            block.add_instruction(IrInstruction::InsertObj {
                                object: object_temp,
                                destination: value_temp,
                            });
                        } else if property == "score" {
                            // Special handling for .score assignment - write to Global Variable G17 per Z-Machine standard
                            // G17 is the standard global variable for game score, used by status line
                            log::debug!("📊 SCORE_WRITE: Using Global G17 for .score assignment");
                            block.add_instruction(IrInstruction::StoreVar {
                                var_id: 17, // Global Variable G17 = score
                                source: value_temp,
                            });
                        } else if property == "moves" {
                            // Special handling for .moves assignment - write to Global Variable G18 per Z-Machine standard
                            // G18 is the standard global variable for move counter, used by status line
                            log::debug!("📊 MOVES_WRITE: Using Global G18 for .moves assignment");
                            block.add_instruction(IrInstruction::StoreVar {
                                var_id: 18, // Global Variable G18 = moves
                                source: value_temp,
                            });
                        } else if let Some(standard_attr) = self.get_standard_attribute(&property) {
                            // This is a Z-Machine attribute assignment - use set_attr
                            let attr_num = standard_attr as u8;

                            // CRITICAL FIX (Nov 2, 2025): Extract actual boolean value from assignment expression
                            // Previous bug: All attribute assignments were hardcoded to `value: true`
                            // This caused obj.open = false to have no effect, breaking container state management
                            let boolean_value = match &assign.value {
                                crate::grue_compiler::ast::Expr::Boolean(value) => *value,
                                _ => {
                                    // For non-literal values (e.g., obj.open = some_variable), we would need
                                    // runtime evaluation through Z-Machine instructions. Currently unsupported.
                                    // This affects dynamic assignments but all literal cases (true/false) work.
                                    log::error!(
                                        "SETATTR_UNSUPPORTED: Non-literal boolean assignment not supported: {:?}",
                                        assign.value
                                    );
                                    true
                                }
                            };

                            block.add_instruction(IrInstruction::SetAttribute {
                                object: object_temp,
                                attribute_num: attr_num,
                                value: boolean_value,
                            });
                            log::debug!(
                                "🔧 ATTRIBUTE ASSIGNMENT: {} -> set_attr(object={}, attr={}, value={})",
                                property, object_temp, attr_num, boolean_value
                            );
                        } else if let Some(standard_prop) = self.get_standard_property(&property) {
                            // Check if this is a standard property that should use numbered access
                            if let Some(prop_num) = self
                                .property_manager
                                .get_standard_property_number(standard_prop)
                            {
                                block.add_instruction(IrInstruction::SetPropertyByNumber {
                                    object: object_temp,
                                    property_num: prop_num,
                                    value: value_temp,
                                });
                            } else {
                                // Use dynamic property manager to assign property number even for standard properties without numbers
                                let prop_num = self.property_manager.get_property_number(&property);
                                block.add_instruction(IrInstruction::SetPropertyByNumber {
                                    object: object_temp,
                                    property_num: prop_num,
                                    value: value_temp,
                                });
                            }
                        } else {
                            // Use dynamic property manager to assign property number for non-standard properties
                            let prop_num = self.property_manager.get_property_number(&property);
                            block.add_instruction(IrInstruction::SetPropertyByNumber {
                                object: object_temp,
                                property_num: prop_num,
                                value: value_temp,
                            });
                        }
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

                // PHASE 3: Context-aware IR generation for if statements
                // Check if condition is attribute access for direct TestAttributeBranch optimization
                match &if_stmt.condition {
                    Expr::PropertyAccess { object, property } => {
                        if let Some(standard_attr) = self.get_standard_attribute(property) {
                            let object_temp = self.generate_expression_with_context(
                                (**object).clone(),
                                block,
                                ExpressionContext::Value,
                            )?;
                            let attr_num = standard_attr as u8;

                            log::debug!(
                                "🎯 PHASE 3: Direct TestAttributeBranch optimization for if {}.{} (attr={})",
                                object_temp,
                                property,
                                attr_num
                            );

                            // Generate direct TestAttributeBranch (single Z-Machine instruction)
                            block.add_instruction(IrInstruction::TestAttributeBranch {
                                object: object_temp,
                                attribute_num: attr_num,
                                then_label,
                                else_label,
                            });

                            // CRITICAL FIX: TestAttributeBranch requires special label ordering for Z-Machine semantics
                            //
                            // Z-Machine test_attr instruction behavior:
                            // - When attribute is SET (true): BRANCHES to specified target
                            // - When attribute is CLEAR (false): FALLS THROUGH to next instruction
                            //
                            // This means the code layout must be:
                            // 1. TestAttributeBranch instruction
                            // 2. else_label content (executed on fall-through when attribute is CLEAR)
                            // 3. Jump to end_label
                            // 4. then_label content (executed on branch when attribute is SET)
                            //
                            // Bug was: Generic if-statement processing placed then_label first,
                            // causing "It's already open" message to execute when mailbox was closed

                            // Else branch: Executes when attribute is CLEAR (fall-through path)
                            log::debug!(
                                "IR TestAttributeBranch: Adding else label {} (fall-through)",
                                else_label
                            );
                            block.add_instruction(IrInstruction::Label { id: else_label });
                            if let Some(else_branch) = if_stmt.else_branch {
                                self.generate_statement(*else_branch, block)?;
                            }

                            // Jump to end after else content to skip then_label content
                            log::debug!(
                                "IR TestAttributeBranch: Adding jump to end label {}",
                                end_label
                            );
                            block.add_instruction(IrInstruction::Jump { label: end_label });

                            // Then branch: Executes when attribute is SET (branch target)
                            log::debug!(
                                "IR TestAttributeBranch: Adding then label {} (branch target)",
                                then_label
                            );
                            block.add_instruction(IrInstruction::Label { id: then_label });
                            self.generate_statement(*if_stmt.then_branch, block)?;

                            // End label: Convergence point for both branches
                            log::debug!("IR TestAttributeBranch: Adding end label {}", end_label);
                            block.add_instruction(IrInstruction::Label { id: end_label });

                            // Skip the generic label processing - we handled everything above
                            return Ok(());
                        } else {
                            // Non-attribute property: use generic pattern
                            let condition_temp = self.generate_expression_with_context(
                                if_stmt.condition.clone(),
                                block,
                                ExpressionContext::Conditional,
                            )?;

                            log::debug!(
                                "IF condition temp (non-attribute property): {}",
                                condition_temp
                            );

                            // Branch based on condition
                            block.add_instruction(IrInstruction::Branch {
                                condition: condition_temp,
                                true_label: then_label,
                                false_label: else_label,
                            });
                        }
                    }
                    _ => {
                        // Non-property-access condition: use generic pattern
                        let condition_temp = self.generate_expression_with_context(
                            if_stmt.condition.clone(),
                            block,
                            ExpressionContext::Conditional,
                        )?;

                        log::debug!("IF condition temp (non-property): {}", condition_temp);

                        // Branch based on condition
                        block.add_instruction(IrInstruction::Branch {
                            condition: condition_temp,
                            true_label: then_label,
                            false_label: else_label,
                        });
                    }
                }

                // Then branch
                log::debug!("IR if: Adding then label {}", then_label);
                block.add_instruction(IrInstruction::Label { id: then_label });
                self.generate_statement(*if_stmt.then_branch, block)?;

                // Only emit jump to end_label if there's an else branch
                // Without else branch, fall-through naturally reaches end_label
                if if_stmt.else_branch.is_some() {
                    log::debug!(
                        "IR if: Adding jump to end label {} (else branch exists)",
                        end_label
                    );
                    block.add_instruction(IrInstruction::Jump { label: end_label });
                } else {
                    log::debug!(
                        "IR if: Skipping jump to end label {} (no else branch - fall-through)",
                        end_label
                    );
                }

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
                // Generate the iterable expression first
                let iterable_temp = self.generate_expression(for_stmt.iterable, block)?;

                log::debug!(
                    "🔍 FOR_LOOP_DEBUG: Generated iterable_temp IR ID {} for for-loop",
                    iterable_temp
                );

                // Use variable source tracking to determine iteration strategy
                // This handles variable indirection (e.g., let items = obj.contents(); for item in items)
                let source_info = self.variable_sources.get(&iterable_temp);
                log::debug!(
                    "🔍 FOR_LOOP_DEBUG: Variable source lookup for IR ID {} = {:?}",
                    iterable_temp,
                    source_info
                );

                let container_object =
                    self.variable_sources
                        .get(&iterable_temp)
                        .and_then(|source| {
                            if let VariableSource::ObjectTreeRoot(container_id) = source {
                                log::debug!(
                                "🔍 FOR_LOOP_DEBUG: Found ObjectTreeRoot source! Container ID = {}",
                                container_id
                            );
                                Some(*container_id)
                            } else {
                                log::debug!(
                                    "🔍 FOR_LOOP_DEBUG: Found non-ObjectTreeRoot source: {:?}",
                                    source
                                );
                                None
                            }
                        });

                if let Some(container_id) = container_object {
                    log::debug!(
                        "🔍 FOR_LOOP_DEBUG: TAKING OBJECT TREE ITERATION PATH! Container ID = {}",
                        container_id
                    );
                    // Generate object tree iteration using get_child/get_sibling opcodes
                    return self.generate_object_tree_iteration_with_container(
                        for_stmt.variable,
                        *for_stmt.body,
                        container_id,
                        block,
                    );
                }

                log::debug!(
                    "🔍 FOR_LOOP_DEBUG: TAKING ARRAY ITERATION PATH! ObjectTreeRoot not found"
                );

                // Otherwise, generate array iteration using get_array_element

                // Create a loop variable
                let loop_var_id = self.next_id();
                let local_var = IrLocal {
                    ir_id: loop_var_id,
                    name: for_stmt.variable.clone(),
                    var_type: Some(Type::Any), // Type inferred from array elements
                    slot: self.next_local_slot,
                    mutable: false, // Loop variables are immutable
                };
                self.current_locals.push(local_var);
                self.symbol_ids.insert(for_stmt.variable, loop_var_id);
                self.next_local_slot += 1;

                // Create index variable for array iteration (allocate as local)
                let index_var = self.next_id();
                let index_local = IrLocal {
                    ir_id: index_var,
                    name: format!("__loop_index_{}", index_var),
                    var_type: Some(Type::Int),
                    slot: self.next_local_slot,
                    mutable: true, // Index is incremented
                };
                self.current_locals.push(index_local);
                self.next_local_slot += 1;

                // Initialize index to 0
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

                // CRITICAL FIX: Implement single-iteration loop for placeholder arrays
                // The contents() method returns a placeholder value, not a real array
                // So we should iterate exactly once with our placeholder object (player = 1)
                // Compare index with 1 to terminate after first iteration
                let one_temp = self.next_id();
                block.add_instruction(IrInstruction::LoadImmediate {
                    target: one_temp,
                    value: IrValue::Integer(1), // Array length = 1 (single placeholder object)
                });

                // Compare index < array_length (1)
                let condition_temp = self.next_id();
                block.add_instruction(IrInstruction::BinaryOp {
                    target: condition_temp,
                    op: IrBinaryOp::Less,
                    left: index_temp,
                    right: one_temp,
                });

                // Branch based on condition
                block.add_instruction(IrInstruction::Branch {
                    condition: condition_temp,
                    true_label: loop_body,
                    false_label: loop_end,
                });

                // Loop body: load current element into loop variable
                block.add_instruction(IrInstruction::Label { id: loop_body });
                // CRITICAL: Reload index since index_temp was consumed by Less comparison
                // This prevents SSA violation (reusing consumed stack value)
                let index_for_get = self.next_id();
                block.add_instruction(IrInstruction::LoadVar {
                    target: index_for_get,
                    var_id: index_var,
                });
                let element_temp = self.next_id();
                // ARRAY REMOVAL (Nov 5, 2025): Arrays removed from Z-Machine compiler
                // This was previously GetArrayElement for iterating through array contents
                // Now replaced with placeholder that returns constant value
                // Text adventures typically use object containment rather than arrays
                block.add_instruction(IrInstruction::LoadImmediate {
                    target: element_temp,
                    value: IrValue::Integer(1), // Placeholder object ID - always returns 1
                });
                block.add_instruction(IrInstruction::StoreVar {
                    var_id: loop_var_id,
                    source: element_temp,
                });

                // Execute loop body
                self.generate_statement(*for_stmt.body, block)?;

                // Increment index
                // Reload index_var for increment operation
                // This prevents SSA violation (reusing consumed stack value)
                let index_for_increment = self.next_id();
                block.add_instruction(IrInstruction::LoadVar {
                    target: index_for_increment,
                    var_id: index_var,
                });
                let one_temp = self.next_id();
                block.add_instruction(IrInstruction::LoadImmediate {
                    target: one_temp,
                    value: IrValue::Integer(1),
                });
                let new_index = self.next_id();
                block.add_instruction(IrInstruction::BinaryOp {
                    target: new_index,
                    op: IrBinaryOp::Add,
                    left: index_for_increment,
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
        // Default context for backward compatibility
        self.generate_expression_with_context(expr, block, ExpressionContext::Value)
    }

    /// Generate expression with explicit context for Z-Machine branch instruction handling
    fn generate_expression_with_context(
        &mut self,
        expr: crate::grue_compiler::ast::Expr,
        block: &mut IrBlock,
        context: ExpressionContext,
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
                // CRITICAL ARCHITECTURAL FIX (Sep 13, 2025): Handle player object specially
                //
                // PROBLEM: Player object references were being generated as LoadImmediate(1) → LargeConstant(1)
                // causing stack underflow in get_prop instructions that expected Variable(16).
                //
                // SOLUTION: Player object must be read from Global G00 (Variable 16) per Z-Machine spec.
                // This ensures proper distinction between:
                // - Literal integer 1 → LargeConstant(1)
                // - Player object reference → Variable(16) (reads from Global G00)
                //
                // This fixes the architectural issue where player.location calls generated wrong operand types.
                if name == "player" {
                    log::debug!("🏃 IR_FIX: Generating LoadVar for player object (will read from Global G00)");
                    let temp_id = self.next_id();
                    block.add_instruction(IrInstruction::LoadVar {
                        target: temp_id,
                        var_id: 16, // Global G00 = Variable 16 = player object number
                    });
                    Ok(temp_id)
                } else if let Some(&_object_number) = self.object_numbers.get(&name) {
                    // This is a regular object or room (not player) - store object name for later runtime resolution
                    let temp_id = self.next_id();
                    block.add_instruction(IrInstruction::LoadImmediate {
                        target: temp_id,
                        value: IrValue::Object(name.clone()),
                    });
                    Ok(temp_id)
                } else if let Some(&var_id) = self.symbol_ids.get(&name) {
                    // This is an existing variable - return its original ID directly
                    // No need to create LoadVar instruction since the variable already exists
                    log::debug!(
                        "✅ IR_FIX: Reusing existing variable ID {} for '{}'",
                        var_id,
                        name
                    );
                    Ok(var_id)
                } else {
                    // Identifier not found - this should be caught during semantic analysis
                    Err(CompilerError::SemanticError(
                        format!("Undefined identifier '{}'", name),
                        0,
                    ))
                }
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

                // Check if this is a built-in function that needs special IR handling
                if self.is_builtin_function(&name) {
                    return self.generate_builtin_function_call(&name, &arg_temps, block);
                }

                // POLYMORPHIC DISPATCH FIX: Use dispatch function if available (same logic as grammar patterns)
                // This ensures consistent behavior between direct calls and grammar calls
                let func_id = if let Some(&dispatch_id) = self.dispatch_functions.get(&name) {
                    log::debug!(
                        "🎯 Direct call using dispatch function for '{}': ID {}",
                        name,
                        dispatch_id
                    );
                    dispatch_id
                } else if let Some(&id) = self.symbol_ids.get(&name) {
                    log::debug!(
                        "🎯 Direct call using original function for '{}': ID {}",
                        name,
                        id
                    );
                    id
                } else {
                    return Err(CompilerError::SemanticError(
                        format!(
                            "Function '{}' not found. All functions must be defined before use.",
                            name
                        ),
                        0,
                    ));
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
                // Check if this is an array method call before moving the object
                let _is_array = self.is_array_type(&object);

                // Generate object expression
                let object_temp = self.generate_expression(*object, block)?;

                // Array methods removed - arrays are anti-pattern in Z-Machine

                // Method call: object.method(args)

                // Generate arguments first
                let mut arg_temps = Vec::new();
                for arg in arguments {
                    let arg_temp = self.generate_expression(arg, block)?;
                    arg_temps.push(arg_temp);
                }

                // Check if this is a known built-in pseudo-method that doesn't require property lookup
                let is_builtin_pseudo_method =
                    matches!(method.as_str(), "get_exit" | "empty" | "none" | "contents");

                if is_builtin_pseudo_method {
                    // For built-in pseudo-methods, generate direct call without property check
                    let result_temp = self.next_id();

                    match method.as_str() {
                        "get_exit" => {
                            let builtin_id = self.next_id();
                            self.builtin_functions
                                .insert(builtin_id, "get_exit".to_string());

                            let mut call_args = vec![object_temp];
                            call_args.extend(arg_temps);

                            block.add_instruction(IrInstruction::Call {
                                target: Some(result_temp),
                                function: builtin_id,
                                args: call_args,
                            });
                        }
                        "empty" => {
                            let builtin_id = self.next_id();
                            self.builtin_functions
                                .insert(builtin_id, "object_is_empty".to_string());

                            let mut call_args = vec![object_temp];
                            call_args.extend(arg_temps);

                            block.add_instruction(IrInstruction::Call {
                                target: Some(result_temp),
                                function: builtin_id,
                                args: call_args,
                            });
                        }
                        "none" => {
                            let builtin_id = self.next_id();
                            self.builtin_functions
                                .insert(builtin_id, "value_is_none".to_string());

                            let mut call_args = vec![object_temp];
                            call_args.extend(arg_temps);

                            block.add_instruction(IrInstruction::Call {
                                target: Some(result_temp),
                                function: builtin_id,
                                args: call_args,
                            });
                        }
                        "contents" => {
                            let builtin_id = self.next_id();
                            self.builtin_functions
                                .insert(builtin_id, "get_object_contents".to_string());

                            let call_args = vec![object_temp];

                            block.add_instruction(IrInstruction::Call {
                                target: Some(result_temp),
                                function: builtin_id,
                                args: call_args,
                            });

                            // Track contents() results as object tree roots for iteration
                            // This enables for-loops to detect object tree iteration even with variable indirection
                            self.variable_sources
                                .insert(result_temp, VariableSource::ObjectTreeRoot(object_temp));
                            log::debug!(
                                "Builtin contents(): tracking result_temp {} as ObjectTreeRoot({})",
                                result_temp,
                                object_temp
                            );
                        }
                        _ => unreachable!(),
                    }

                    return Ok(result_temp);
                }

                // For regular property-based methods, generate property lookup and conditional call

                // Check if this is contents() for object tree iteration tracking
                let is_contents_method = method.as_str() == "contents";

                // Generate property access to get the method function
                let property_temp = self.next_id();
                let prop_num = self.property_manager.get_property_number(&method);
                block.add_instruction(IrInstruction::GetPropertyByNumber {
                    target: property_temp,
                    object: object_temp,
                    property_num: prop_num,
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

                // Special handling for property-based methods that have fallback behavior
                match method.as_str() {
                    "size" | "length" => {
                        // size() or length() method: return count of elements/contents
                        let builtin_id = self.next_id();
                        self.builtin_functions
                            .insert(builtin_id, "get_object_size".to_string());

                        let mut call_args = vec![object_temp];
                        call_args.extend(arg_temps);

                        block.add_instruction(IrInstruction::Call {
                            target: Some(result_temp),
                            function: builtin_id,
                            args: call_args,
                        });
                    }
                    "add" => {
                        // Array/collection add method - for arrays like visible_objects.add(obj)
                        // Implement as proper builtin function call instead of LoadImmediate fallback
                        let builtin_id = self.next_id();
                        self.builtin_functions
                            .insert(builtin_id, "array_add_item".to_string());

                        let mut call_args = vec![object_temp];
                        call_args.extend(arg_temps);

                        block.add_instruction(IrInstruction::Call {
                            target: Some(result_temp),
                            function: builtin_id,
                            args: call_args,
                        });
                    }
                    "on_enter" | "on_exit" | "on_look" => {
                        // Object handler methods - these call property-based function handlers
                        // In Grue, these are properties that contain function addresses
                        // The pattern is: if object.property exists, call it as a function

                        // Get property number for this handler - this will register it if not found
                        let property_name = method;
                        let property_number =
                            self.property_manager.get_property_number(&property_name);

                        // Use proper property-based function call
                        block.add_instruction(IrInstruction::GetPropertyByNumber {
                            target: result_temp,
                            object: object_temp,
                            property_num: property_number,
                        });

                        // TODO: In a complete implementation, this would:
                        // 1. Get the property value (function address)
                        // 2. Check if it's non-zero (function exists)
                        // 3. Call the function if it exists
                        // For now, we'll return the property value directly

                        log::debug!(
                            "Object handler '{}' mapped to property #{} for method call",
                            property_name,
                            property_number
                        );
                    }
                    _ => {
                        // For truly unknown methods, return safe non-zero value to prevent object 0 errors
                        // This prevents "Cannot insert object 0" crashes while providing a detectable placeholder
                        log::warn!("Unknown method '{}' called on object - returning safe placeholder value 1", method);
                        block.add_instruction(IrInstruction::LoadImmediate {
                            target: result_temp,
                            value: IrValue::Integer(1), // Safe non-zero value instead of 0
                        });
                    }
                }

                block.add_instruction(IrInstruction::Jump { label: end_label });

                // Else branch: property doesn't exist or isn't callable, return safe non-zero value
                block.add_instruction(IrInstruction::Label { id: else_label });
                block.add_instruction(IrInstruction::LoadImmediate {
                    target: result_temp,
                    value: IrValue::Integer(1), // Use 1 instead of 0 to prevent null operands
                });

                // End label
                block.add_instruction(IrInstruction::Label { id: end_label });

                // Track contents() results as object tree roots for iteration
                // This enables for-loops to detect object tree iteration even with variable indirection
                if is_contents_method {
                    self.variable_sources
                        .insert(result_temp, VariableSource::ObjectTreeRoot(object_temp));
                }

                Ok(result_temp)
            }
            Expr::PropertyAccess { object, property } => {
                // Property access: object.property
                let is_array = self.is_array_type(&object);
                let object_temp = self.generate_expression_with_context(
                    *object,
                    block,
                    ExpressionContext::Value,
                )?;
                let temp_id = self.next_id();

                log::debug!(
                    "🔍 PropertyAccess: property='{}', object_temp={}, context={:?}",
                    property,
                    object_temp,
                    context
                );

                // Check if this is an exit value property access (bit manipulation)
                // Exit values are encoded as: (type << 14) | data
                // where type=0 for normal exits, type=1 for blocked exits
                match property.as_str() {
                    "blocked" => {
                        // Check if bit 14 is set (value >= 0x4000)
                        let builtin_id = self.next_id();
                        self.builtin_functions
                            .insert(builtin_id, "exit_is_blocked".to_string());

                        log::debug!(
                            "🚪 EXIT: Adding Call instruction: target={}, function={}, args=[{}]",
                            temp_id,
                            builtin_id,
                            object_temp
                        );
                        block.add_instruction(IrInstruction::Call {
                            target: Some(temp_id),
                            function: builtin_id,
                            args: vec![object_temp],
                        });
                        log::debug!(
                            "🚪 EXIT: Call instruction added, returning temp_id={}",
                            temp_id
                        );
                        return Ok(temp_id);
                    }
                    "destination" => {
                        // Extract lower 14 bits (value & 0x3FFF) -> room ID
                        log::debug!("🚪 EXIT: Creating exit_get_destination builtin");

                        let builtin_id = self.next_id();
                        self.builtin_functions
                            .insert(builtin_id, "exit_get_destination".to_string());

                        log::debug!(
                            "🚪 EXIT: Adding Call instruction: target={}, function={}, args=[{}]",
                            temp_id,
                            builtin_id,
                            object_temp
                        );
                        block.add_instruction(IrInstruction::Call {
                            target: Some(temp_id),
                            function: builtin_id,
                            args: vec![object_temp],
                        });

                        log::debug!(
                            "🚪 EXIT: Call instruction added, returning temp_id={}",
                            temp_id
                        );
                        return Ok(temp_id);
                    }
                    "message" => {
                        // Extract lower 14 bits (value & 0x3FFF) -> string address
                        log::debug!("🚪 EXIT: Creating exit_get_message builtin");

                        let builtin_id = self.next_id();
                        self.builtin_functions
                            .insert(builtin_id, "exit_get_message".to_string());

                        log::debug!(
                            "🚪 EXIT: Adding Call instruction: target={}, function={}, args=[{}]",
                            temp_id,
                            builtin_id,
                            object_temp
                        );
                        block.add_instruction(IrInstruction::Call {
                            target: Some(temp_id),
                            function: builtin_id,
                            args: vec![object_temp],
                        });

                        // Record that exit_get_message returns StringAddress for type-aware println
                        self.record_expression_type(temp_id, Type::StringAddress);

                        log::debug!(
                            "🚪 EXIT: Call instruction added, returning temp_id={}",
                            temp_id
                        );
                        return Ok(temp_id);
                    }
                    _ => {} // Fall through to normal property access
                }

                // ARRAY REMOVAL (Nov 5, 2025): Array property access removed from Z-Machine compiler
                // This section previously handled array.length and array.size properties
                // Now returns placeholder values since arrays are not supported
                if is_array {
                    match property.as_str() {
                        "length" | "size" => {
                            // Return placeholder length of 0 for removed array functionality
                            block.add_instruction(IrInstruction::LoadImmediate {
                                target: temp_id,
                                value: IrValue::Integer(0), // Placeholder - arrays always have length 0
                            });
                            return Ok(temp_id);
                        }
                        _ => {
                            return Err(CompilerError::CodeGenError(format!(
                                "Unknown array property: {}",
                                property
                            )));
                        }
                    }
                }

                // Special handling for player global variables FIRST, before standard property handling
                // This ensures score/moves access global variables instead of properties
                if property == "score" {
                    // Special handling for .score - read from Global Variable G17 per Z-Machine standard
                    // G17 is the standard global variable for game score, used by status line
                    log::debug!("📊 SCORE_FIX: Using Global G17 for .score property access");
                    block.add_instruction(IrInstruction::LoadVar {
                        target: temp_id,
                        var_id: 17, // Global Variable G17 = score
                    });
                } else if property == "moves" {
                    // Special handling for .moves - read from Global Variable G18 per Z-Machine standard
                    // G18 is the standard global variable for move counter, used by status line
                    log::debug!("📊 MOVES_FIX: Using Global G18 for .moves property access");
                    block.add_instruction(IrInstruction::LoadVar {
                        target: temp_id,
                        var_id: 18, // Global Variable G18 = moves
                    });
                } else if property == "location" {
                    // Special handling for .location - use get_parent instead of property access
                    // BUG FIX (Oct 11, 2025): player.location must read parent from object tree,
                    // not from a property, because move() uses insert_obj which updates the tree
                    log::debug!(
                        "🏃 LOCATION_FIX: Using GetObjectParent for .location property access"
                    );
                    block.add_instruction(IrInstruction::GetObjectParent {
                        target: temp_id,
                        object: object_temp,
                    });
                } else if let Some(standard_attr) = self.get_standard_attribute(&property) {
                    // Phase 2A: Z-Machine attribute access using Option B-2 (reuse existing Branch pattern)
                    let attr_num = standard_attr as u8;

                    match context {
                        ExpressionContext::Conditional => {
                            // OPTION B-2: Reuse existing Branch instruction pattern
                            log::debug!(
                                "🔍 ATTRIBUTE ACCESS (CONDITIONAL): {} -> using existing Branch pattern",
                                property
                            );

                            // Generate TestAttribute for condition (like if statements do)
                            let condition_temp = self.next_id();
                            #[allow(deprecated)]
                            {
                                block.add_instruction(IrInstruction::TestAttribute {
                                    target: condition_temp,
                                    object: object_temp,
                                    attribute_num: attr_num,
                                });
                            }

                            // Return the condition temp - the if statement will handle the Branch
                            return Ok(condition_temp);
                        }

                        ExpressionContext::Value => {
                            // IMPLEMENTED: TestAttributeValue pattern for value contexts
                            log::debug!(
                                "🔍 ATTRIBUTE ACCESS (VALUE): {} -> TestAttributeValue pattern (implemented)",
                                property
                            );

                            // Use proper TestAttributeValue instruction
                            let temp_id = self.next_id();
                            block.add_instruction(IrInstruction::TestAttributeValue {
                                target: temp_id,
                                object: object_temp,
                                attribute_num: attr_num,
                            });
                            return Ok(temp_id);
                        }

                        ExpressionContext::Assignment => {
                            return Err(CompilerError::SemanticError(
                                format!(
                                    "Cannot read attribute '{}' in assignment context",
                                    property
                                ),
                                0,
                            ));
                        }
                    }
                } else if let Some(standard_prop) = self.get_standard_property(&property) {
                    // Check if this is a standard property that should use numbered access
                    if let Some(prop_num) = self
                        .property_manager
                        .get_standard_property_number(standard_prop)
                    {
                        block.add_instruction(IrInstruction::GetPropertyByNumber {
                            target: temp_id,
                            object: object_temp,
                            property_num: prop_num,
                        });
                    } else {
                        // Use dynamic property manager to assign property number even for standard properties without numbers
                        let prop_num = self.property_manager.get_property_number(&property);
                        block.add_instruction(IrInstruction::GetPropertyByNumber {
                            target: temp_id,
                            object: object_temp,
                            property_num: prop_num,
                        });
                    }
                } else {
                    // Use dynamic property manager to assign property number for non-standard properties
                    let prop_num = self.property_manager.get_property_number(&property);
                    block.add_instruction(IrInstruction::GetPropertyByNumber {
                        target: temp_id,
                        object: object_temp,
                        property_num: prop_num,
                    });
                }

                log::debug!(
                    "Property access: object={} property='{}' -> temp={}",
                    object_temp,
                    property,
                    temp_id
                );

                Ok(temp_id)
            }

            Expr::NullSafePropertyAccess { object, property } => {
                // Null-safe property access: object?.property
                let is_array = self.is_array_type(&object);
                let object_temp = self.generate_expression_with_context(
                    *object,
                    block,
                    ExpressionContext::Value,
                )?;
                let temp_id = self.next_id();

                // For null-safe access, we need to check if the object is null/valid first
                let null_check_label = self.next_id();
                let valid_label = self.next_id();
                let end_label = self.next_id();

                // Check if object is null (0)
                let zero_temp = self.next_id();
                block.add_instruction(IrInstruction::LoadImmediate {
                    target: zero_temp,
                    value: IrValue::Integer(0),
                });

                // Compare object with zero
                let condition_temp = self.next_id();
                block.add_instruction(IrInstruction::BinaryOp {
                    target: condition_temp,
                    op: IrBinaryOp::NotEqual,
                    left: object_temp,
                    right: zero_temp,
                });

                // Branch: if object != 0, goto valid_label, else goto null_check_label
                block.add_instruction(IrInstruction::Branch {
                    condition: condition_temp,
                    true_label: valid_label,
                    false_label: null_check_label,
                });

                // Null case: return null/0
                block.add_instruction(IrInstruction::Label {
                    id: null_check_label,
                });
                block.add_instruction(IrInstruction::LoadImmediate {
                    target: temp_id,
                    value: IrValue::Integer(0),
                });
                block.add_instruction(IrInstruction::Jump { label: end_label });

                // Valid case: perform normal property access
                block.add_instruction(IrInstruction::Label { id: valid_label });
                if is_array {
                    match property.as_str() {
                        "length" | "size" => {
                            // Arrays removed - return placeholder length
                            block.add_instruction(IrInstruction::LoadImmediate {
                                target: temp_id,
                                value: IrValue::Integer(0), // Placeholder array length
                            });
                        }
                        _ => {
                            return Err(CompilerError::CodeGenError(format!(
                                "Unknown array property: {}",
                                property
                            )));
                        }
                    }
                } else {
                    // Check if this is a standard property that should use numbered access
                    if let Some(standard_prop) = self.get_standard_property(&property) {
                        if let Some(prop_num) = self
                            .property_manager
                            .get_standard_property_number(standard_prop)
                        {
                            block.add_instruction(IrInstruction::GetPropertyByNumber {
                                target: temp_id,
                                object: object_temp,
                                property_num: prop_num,
                            });
                        } else {
                            // Use dynamic property manager to assign property number even for standard properties without numbers
                            let prop_num = self.property_manager.get_property_number(&property);
                            block.add_instruction(IrInstruction::GetPropertyByNumber {
                                target: temp_id,
                                object: object_temp,
                                property_num: prop_num,
                            });
                        }
                    } else {
                        // Use dynamic property manager to assign property number for non-standard properties
                        let prop_num = self.property_manager.get_property_number(&property);
                        block.add_instruction(IrInstruction::GetPropertyByNumber {
                            target: temp_id,
                            object: object_temp,
                            property_num: prop_num,
                        });
                    }
                }

                // End label
                block.add_instruction(IrInstruction::Label { id: end_label });

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

            // Enhanced parser expressions (for future Phase 1.3 implementation)
            Expr::ParsedObject {
                adjectives: _,
                noun,
                article: _,
            } => {
                // For now, treat as simple string identifier
                let temp_id = self.next_id();
                block.add_instruction(IrInstruction::LoadImmediate {
                    target: temp_id,
                    value: IrValue::String(noun),
                });
                Ok(temp_id)
            }

            Expr::MultipleObjects(objects) => {
                // For now, just use the first object
                if let Some(first_obj) = objects.into_iter().next() {
                    self.generate_expression(first_obj, block)
                } else {
                    let temp_id = self.next_id();
                    block.add_instruction(IrInstruction::LoadImmediate {
                        target: temp_id,
                        value: IrValue::Integer(1), // Use safe non-zero value instead of Null
                    });
                    Ok(temp_id)
                }
            }

            Expr::Array(elements) => {
                // ARRAY RESTORATION (Nov 5, 2025): Implement proper static array support
                // following the NEW_ARRAY_IMPLEMENTATION approach
                log::debug!("Processing array literal with {} elements", elements.len());

                // Convert each element expression to IR value
                let mut ir_elements = Vec::new();
                for element in elements {
                    let element_id = self.generate_expression(element, block)?;
                    // For static arrays, we need to resolve the element to a constant value
                    // Dynamic resolution will be handled during codegen
                    ir_elements.push(IrValue::Integer(element_id as i16)); // Store IR ID as i16
                }

                // Generate array creation instruction
                let array_id = self.next_id();
                log::debug!(
                    "Generated CreateArray instruction: target={}, elements={:?}",
                    array_id,
                    ir_elements
                );

                block.add_instruction(IrInstruction::CreateArray {
                    target: array_id,
                    elements: ir_elements,
                });
                Ok(array_id)
            }
            Expr::DisambiguationContext {
                candidates: _,
                query,
            } => {
                // For now, treat as simple string
                let temp_id = self.next_id();
                block.add_instruction(IrInstruction::LoadImmediate {
                    target: temp_id,
                    value: IrValue::String(query),
                });
                Ok(temp_id)
            }
        }
    }

    pub(super) fn expr_to_ir_value(
        &mut self,
        expr: crate::grue_compiler::ast::Expr,
    ) -> Result<IrValue, CompilerError> {
        use crate::grue_compiler::ast::Expr;

        match expr {
            Expr::Integer(value) => Ok(IrValue::Integer(value)),
            Expr::String(value) => Ok(IrValue::String(value)),
            Expr::Boolean(value) => Ok(IrValue::Boolean(value)),
            Expr::Parameter(param) => {
                // Grammar parameters like $noun should resolve to runtime parsed objects
                // For now, return a special marker that the codegen can handle
                Ok(IrValue::RuntimeParameter(param.clone()))
            }
            Expr::Identifier(name) => {
                // Check if this is an object reference
                if self.object_numbers.contains_key(&name) {
                    Ok(IrValue::Object(name.clone()))
                } else {
                    Err(CompilerError::SemanticError(
                        format!("Cannot resolve identifier '{}' in grammar expression - not a known object or variable", name),
                        0
                    ))
                }
            }
            _ => Err(CompilerError::SemanticError(
                format!(
                    "UNIMPLEMENTED: Complex expression in grammar handler: {:?}",
                    expr
                ),
                0,
            )),
        }
    }

    /// Check if an expression represents an array type
    fn is_array_type(&self, expr: &crate::grue_compiler::ast::Expr) -> bool {
        use crate::grue_compiler::ast::Expr;
        match expr {
            Expr::Array(_) => true,
            Expr::Identifier(name) => {
                // First, check if this variable is tracked in variable_sources
                // This takes precedence over name-based heuristics
                if let Some(&var_id) = self.symbol_ids.get(name) {
                    if let Some(source) = self.variable_sources.get(&var_id) {
                        return match source {
                            // Arrays removed - no variables are arrays anymore
                            VariableSource::ObjectTreeRoot(_) => false, // Contents result - NOT an array
                            VariableSource::Scalar(_) => false, // Scalar value - NOT an array
                        };
                    }
                }

                // Fall back to name-based heuristic for untracked variables
                // Only consider identifiers that are likely to be arrays
                // This is a simplified heuristic - in a full implementation,
                // we'd track variable types through semantic analysis
                name.contains("array")
                    || name.contains("list")
                    || name.contains("items")
                    || name.contains("numbers")
                    || name.contains("strings")
                    || name.contains("elements")
            }
            _ => false,
        }
    }

    fn generate_builtin_function_call(
        &mut self,
        name: &str,
        arg_temps: &[IrId],
        block: &mut IrBlock,
    ) -> Result<IrId, CompilerError> {
        let temp_id = self.next_id();

        match name {
            // String utility functions
            "indexOf" => {
                if arg_temps.len() != 2 {
                    return Err(CompilerError::CodeGenError(
                        "indexOf expects 2 arguments".to_string(),
                    ));
                }
                block.add_instruction(IrInstruction::StringIndexOf {
                    target: temp_id,
                    string: arg_temps[0],
                    substring: arg_temps[1],
                });
            }
            "slice" => {
                if arg_temps.len() != 2 {
                    return Err(CompilerError::CodeGenError(
                        "slice expects 2 arguments".to_string(),
                    ));
                }
                block.add_instruction(IrInstruction::StringSlice {
                    target: temp_id,
                    string: arg_temps[0],
                    start: arg_temps[1],
                });
            }
            "substring" => {
                if arg_temps.len() != 3 {
                    return Err(CompilerError::CodeGenError(
                        "substring expects 3 arguments".to_string(),
                    ));
                }
                block.add_instruction(IrInstruction::StringSubstring {
                    target: temp_id,
                    string: arg_temps[0],
                    start: arg_temps[1],
                    end: arg_temps[2],
                });
            }
            "toLowerCase" => {
                if arg_temps.len() != 1 {
                    return Err(CompilerError::CodeGenError(
                        "toLowerCase expects 1 argument".to_string(),
                    ));
                }
                block.add_instruction(IrInstruction::StringToLowerCase {
                    target: temp_id,
                    string: arg_temps[0],
                });
            }
            "toUpperCase" => {
                if arg_temps.len() != 1 {
                    return Err(CompilerError::CodeGenError(
                        "toUpperCase expects 1 argument".to_string(),
                    ));
                }
                block.add_instruction(IrInstruction::StringToUpperCase {
                    target: temp_id,
                    string: arg_temps[0],
                });
            }
            "trim" => {
                if arg_temps.len() != 1 {
                    return Err(CompilerError::CodeGenError(
                        "trim expects 1 argument".to_string(),
                    ));
                }
                block.add_instruction(IrInstruction::StringTrim {
                    target: temp_id,
                    string: arg_temps[0],
                });
            }
            "charAt" => {
                if arg_temps.len() != 2 {
                    return Err(CompilerError::CodeGenError(
                        "charAt expects 2 arguments".to_string(),
                    ));
                }
                block.add_instruction(IrInstruction::StringCharAt {
                    target: temp_id,
                    string: arg_temps[0],
                    index: arg_temps[1],
                });
            }
            "split" => {
                if arg_temps.len() != 2 {
                    return Err(CompilerError::CodeGenError(
                        "split expects 2 arguments".to_string(),
                    ));
                }
                block.add_instruction(IrInstruction::StringSplit {
                    target: temp_id,
                    string: arg_temps[0],
                    delimiter: arg_temps[1],
                });
            }
            "replace" => {
                if arg_temps.len() != 3 {
                    return Err(CompilerError::CodeGenError(
                        "replace expects 3 arguments".to_string(),
                    ));
                }
                block.add_instruction(IrInstruction::StringReplace {
                    target: temp_id,
                    string: arg_temps[0],
                    search: arg_temps[1],
                    replacement: arg_temps[2],
                });
            }
            "startsWith" => {
                if arg_temps.len() != 2 {
                    return Err(CompilerError::CodeGenError(
                        "startsWith expects 2 arguments".to_string(),
                    ));
                }
                block.add_instruction(IrInstruction::StringStartsWith {
                    target: temp_id,
                    string: arg_temps[0],
                    prefix: arg_temps[1],
                });
            }
            "endsWith" => {
                if arg_temps.len() != 2 {
                    return Err(CompilerError::CodeGenError(
                        "endsWith expects 2 arguments".to_string(),
                    ));
                }
                block.add_instruction(IrInstruction::StringEndsWith {
                    target: temp_id,
                    string: arg_temps[0],
                    suffix: arg_temps[1],
                });
            }
            // Math utility functions
            "abs" => {
                if arg_temps.len() != 1 {
                    return Err(CompilerError::CodeGenError(
                        "abs expects 1 argument".to_string(),
                    ));
                }
                block.add_instruction(IrInstruction::MathAbs {
                    target: temp_id,
                    value: arg_temps[0],
                });
            }
            "min" => {
                if arg_temps.len() != 2 {
                    return Err(CompilerError::CodeGenError(
                        "min expects 2 arguments".to_string(),
                    ));
                }
                block.add_instruction(IrInstruction::MathMin {
                    target: temp_id,
                    a: arg_temps[0],
                    b: arg_temps[1],
                });
            }
            "max" => {
                if arg_temps.len() != 2 {
                    return Err(CompilerError::CodeGenError(
                        "max expects 2 arguments".to_string(),
                    ));
                }
                block.add_instruction(IrInstruction::MathMax {
                    target: temp_id,
                    a: arg_temps[0],
                    b: arg_temps[1],
                });
            }
            "round" => {
                if arg_temps.len() != 1 {
                    return Err(CompilerError::CodeGenError(
                        "round expects 1 argument".to_string(),
                    ));
                }
                block.add_instruction(IrInstruction::MathRound {
                    target: temp_id,
                    value: arg_temps[0],
                });
            }
            "floor" => {
                if arg_temps.len() != 1 {
                    return Err(CompilerError::CodeGenError(
                        "floor expects 1 argument".to_string(),
                    ));
                }
                block.add_instruction(IrInstruction::MathFloor {
                    target: temp_id,
                    value: arg_temps[0],
                });
            }
            "ceil" => {
                if arg_temps.len() != 1 {
                    return Err(CompilerError::CodeGenError(
                        "ceil expects 1 argument".to_string(),
                    ));
                }
                block.add_instruction(IrInstruction::MathCeil {
                    target: temp_id,
                    value: arg_temps[0],
                });
            }
            // Type checking functions
            "is_string" => {
                if arg_temps.len() != 1 {
                    return Err(CompilerError::CodeGenError(
                        "is_string expects 1 argument".to_string(),
                    ));
                }
                block.add_instruction(IrInstruction::TypeCheck {
                    target: temp_id,
                    value: arg_temps[0],
                    type_name: "string".to_string(),
                });
            }
            "is_int" => {
                if arg_temps.len() != 1 {
                    return Err(CompilerError::CodeGenError(
                        "is_int expects 1 argument".to_string(),
                    ));
                }
                block.add_instruction(IrInstruction::TypeCheck {
                    target: temp_id,
                    value: arg_temps[0],
                    type_name: "int".to_string(),
                });
            }
            "is_bool" => {
                if arg_temps.len() != 1 {
                    return Err(CompilerError::CodeGenError(
                        "is_bool expects 1 argument".to_string(),
                    ));
                }
                block.add_instruction(IrInstruction::TypeCheck {
                    target: temp_id,
                    value: arg_temps[0],
                    type_name: "bool".to_string(),
                });
            }
            "is_array" => {
                if arg_temps.len() != 1 {
                    return Err(CompilerError::CodeGenError(
                        "is_array expects 1 argument".to_string(),
                    ));
                }
                block.add_instruction(IrInstruction::TypeCheck {
                    target: temp_id,
                    value: arg_temps[0],
                    type_name: "array".to_string(),
                });
            }
            "is_object" => {
                if arg_temps.len() != 1 {
                    return Err(CompilerError::CodeGenError(
                        "is_object expects 1 argument".to_string(),
                    ));
                }
                block.add_instruction(IrInstruction::TypeCheck {
                    target: temp_id,
                    value: arg_temps[0],
                    type_name: "object".to_string(),
                });
            }
            "typeof" => {
                if arg_temps.len() != 1 {
                    return Err(CompilerError::CodeGenError(
                        "typeof expects 1 argument".to_string(),
                    ));
                }
                block.add_instruction(IrInstruction::TypeOf {
                    target: temp_id,
                    value: arg_temps[0],
                });
            }
            // Debug breakpoint (debug builds only)
            #[cfg(debug_assertions)]
            "debug_break" => {
                if arg_temps.len() != 1 {
                    return Err(CompilerError::CodeGenError(
                        "debug_break expects 1 argument (label string)".to_string(),
                    ));
                }
                // Extract label from the LoadImmediate instruction
                // We need to look back at the IR to find the string value
                // For now, use a placeholder - we'll need to track this properly
                block.add_instruction(IrInstruction::DebugBreak {
                    label: format!("breakpoint_{}", temp_id),
                });
                // Return a dummy value (0) since debug_break doesn't produce a useful result
                block.add_instruction(IrInstruction::LoadImmediate {
                    target: temp_id,
                    value: IrValue::Integer(0),
                });
            }
            // Score management functions - Now implemented as real Z-Machine functions
            // ARCHITECTURE FIX (Nov 4, 2025): Converted from inline generation to real builtin functions
            // per CLAUDE.md directive: "ALL builtin functions MUST be implemented as real Z-Machine functions"
            "add_score" | "subtract_score" | "word_to_number" => {
                // Use standard builtin function call mechanism like get_exit
                // This eliminates opcode 0x15 errors by using proper function calls
                // instead of inline IR generation that violates Z-Machine V3 constraints

                // Look up function ID (or create placeholder)
                let func_id = if let Some(&id) = self.symbol_ids.get(name) {
                    id
                } else {
                    let placeholder_id = self.next_id();
                    self.symbol_ids.insert(name.to_string(), placeholder_id);
                    self.builtin_functions
                        .insert(placeholder_id, name.to_string());
                    placeholder_id
                };

                // Generate function call instruction
                block.add_instruction(IrInstruction::Call {
                    target: Some(temp_id),
                    function: func_id,
                    args: arg_temps.to_vec(),
                });
            }
            // For other builtin functions, use standard call mechanism
            _ => {
                // Look up function ID (or create placeholder)
                let func_id = if let Some(&id) = self.symbol_ids.get(name) {
                    id
                } else {
                    let placeholder_id = self.next_id();
                    self.symbol_ids.insert(name.to_string(), placeholder_id);
                    self.builtin_functions
                        .insert(placeholder_id, name.to_string());
                    placeholder_id
                };

                block.add_instruction(IrInstruction::Call {
                    target: Some(temp_id),
                    function: func_id,
                    args: arg_temps.to_vec(),
                });
            }
        }

        Ok(temp_id)
    }

    /// Testing method to expose room_objects mapping for integration tests
    #[cfg(test)]
    pub fn get_room_objects(&self) -> &IndexMap<String, Vec<RoomObjectInfo>> {
        &self.room_objects
    }
}

// Extracted modules for functional organization
mod ir_gen_grammar;
mod ir_gen_rooms;
mod ir_gen_functions;
