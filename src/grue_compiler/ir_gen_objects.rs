// IR Generator - Object System Generation
//
// Extracted from ir_generator.rs as part of modularization effort.
// Handles world generation, object creation, placement, and numbering.

use crate::grue_compiler::error::CompilerError;

use super::{
    IrAttributes, IrBlock, IrGenerator, IrId, IrInstruction, IrObject, IrProgram, IrProperties,
    RoomObjectInfo, StandardAttribute, StandardProperty,
};

impl IrGenerator {
    /// Generate world from AST world declaration
    ///
    /// Three-pass approach:
    /// 1. Register all rooms and objects for symbol resolution
    /// 2. Assign object numbers systematically (Player #1 → Rooms #2-N → Objects #N+1-M)
    /// 3. Generate actual IR objects and rooms
    pub(super) fn generate_world(
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
    ///
    /// Ensures systematic ordering: Player(#1) -> Rooms(#2-N) -> Objects(#N+1-M)
    pub(super) fn assign_object_numbers_recursively(
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
    ///
    /// Recursively processes containment hierarchy to ensure all objects get unique numbers.
    pub(super) fn assign_object_number_to_object_and_nested(
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
    ///
    /// Provides sensible defaults for capacity, value, size, etc.
    pub(super) fn setup_property_defaults(&self, ir_program: &mut IrProgram) {
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

    /// Generate IR object from AST object declaration
    ///
    /// Handles:
    /// - Attribute conversion (named attributes and property-based attributes)
    /// - Property conversion (named, numbered, object/room references)
    /// - Containment hierarchy (parent/child/sibling relationships)
    /// - Recursive processing of nested objects
    pub(super) fn generate_object(
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

    /// Generate InsertObj instructions from room_objects mapping for init block
    ///
    /// Converts room object hierarchies to InsertObj instructions to establish object tree.
    pub(super) fn generate_object_placement_instructions(
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
    ///
    /// Recursively handles object containment hierarchy.
    pub(super) fn generate_placement_for_object(
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
    ///
    /// The player is always object #1 and has standard properties.
    /// Creates player with:
    /// - ID 9999 (high to avoid conflicts)
    /// - Object number #1 (assigned during codegen)
    /// - Initial location in first room
    /// - Standard properties (location, description, quit_pending)
    pub(super) fn add_player_object(&mut self, ir_program: &mut IrProgram) -> Result<(), CompilerError> {
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

    /// Generate room object placement instructions (stub)
    ///
    /// Placeholder for future room object placement logic.
    pub(super) fn generate_room_object_placement(
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
}
