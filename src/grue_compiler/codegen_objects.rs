// Z-Machine Object and Property Code Generator
//
// Handles object table generation, property processing, and core object management
// for the Z-Machine bytecode compiler.

use crate::grue_compiler::error::CompilerError;
use crate::grue_compiler::ir::*;
use crate::grue_compiler::ZMachineVersion;
use indexmap::{IndexMap, IndexSet};
use log::debug;

// Re-export common types for object handling
pub use crate::grue_compiler::codegen::{MemorySpace, ObjectData, Operand, ZMachineCodeGen};

impl ZMachineCodeGen {
    /// Allocate space in object space and return offset
    pub fn allocate_object_space(&mut self, size: usize) -> Result<usize, CompilerError> {
        let offset = self.object_address;

        // Ensure capacity
        if self.object_address + size > self.object_space.len() {
            self.object_space.resize(self.object_address + size, 0);
        }

        self.object_address += size;
        log::debug!(
            "🏗️ OBJECT_ALLOCATED: offset=0x{:04x}, size={}",
            offset,
            size
        );

        Ok(offset)
    }

    /// Write to object space at specific offset
    pub fn write_to_object_space(&mut self, offset: usize, byte: u8) -> Result<(), CompilerError> {
        if offset >= self.object_space.len() {
            self.object_space.resize(offset + 1, 0);
        }

        log::debug!(
            "📝 OBJECT_SPACE: Write 0x{:02x} at offset 0x{:04x} (space size: {})",
            byte,
            offset,
            self.object_space.len()
        );
        self.object_space[offset] = byte;
        Ok(())
    }

    /// Generate objects and properties to object space
    pub fn generate_objects_to_space(&mut self, _ir: &IrProgram) -> Result<(), CompilerError> {
        log::debug!(" Generating minimal object table for Z-Machine compliance");

        // Generate minimal object table required by Z-Machine specification
        // Even simple programs need a basic object table structure

        // Calculate minimum required size
        let default_props_size = match self.version {
            ZMachineVersion::V3 => 62, // 31 properties * 2 bytes
            ZMachineVersion::V4 | ZMachineVersion::V5 => 126, // 63 properties * 2 bytes
        };

        // Minimum 1 object entry (even if program defines no objects)
        let min_objects = 1;
        let obj_entry_size = match self.version {
            ZMachineVersion::V3 => 9,                        // V3: 9 bytes per object
            ZMachineVersion::V4 | ZMachineVersion::V5 => 14, // V4/V5: 14 bytes per object
        };

        // Basic property table for the minimal object (just terminator)
        let min_prop_table_size = 2; // Property table terminator

        let total_size = default_props_size + (min_objects * obj_entry_size) + min_prop_table_size;

        self.allocate_object_space(total_size)?;
        log::debug!(
            " Object space allocated: {} bytes (default_props={}, objects={}, prop_tables={})",
            total_size,
            default_props_size,
            min_objects * obj_entry_size,
            min_prop_table_size
        );

        // Generate the actual object table data
        self.write_minimal_object_table()?;

        Ok(())
    }

    /// Setup object table generation for full object table
    pub fn setup_object_table_generation(&mut self) {
        // For separated spaces, object table starts at the beginning of object space
        self.object_table_addr = 0; // Will be adjusted when assembled into final image
        log::debug!(
            " Object table generation setup: starting address 0x{:04x}",
            self.object_table_addr
        );
    }

    /// Write minimal object table structure required by Z-Machine
    ///
    /// DEAD CODE: This function is not called for programs with objects. When IR contains
    /// objects, the full object generation path (create_object_entries_from_ir) is used instead.
    /// This function is only called when no objects exist in IR, which doesn't happen in practice.
    /// Analysis: Neither main nor this branch calls this function for mini_zork.
    #[allow(dead_code)]
    pub fn write_minimal_object_table(&mut self) -> Result<(), CompilerError> {
        log::debug!("📝 Writing minimal object table structure");
        let mut offset = 0;

        // Phase 1: Write default property table (all zeros)
        let default_props_count = match self.version {
            ZMachineVersion::V3 => 31,
            ZMachineVersion::V4 | ZMachineVersion::V5 => 63,
        };

        for _ in 0..default_props_count {
            self.write_to_object_space(offset, 0)?; // Property default value high byte
            offset += 1;
            self.write_to_object_space(offset, 0)?; // Property default value low byte
            offset += 1;
        }
        log::debug!(
            " Default property table written ({} properties)",
            default_props_count
        );

        // Phase 2: Write minimal object entry (object #1)
        match self.version {
            ZMachineVersion::V3 => {
                // V3 object format: 4 bytes attributes + 1 byte parent + 1 byte sibling + 1 byte child + 2 bytes properties
                self.write_to_object_space(offset, 0)?;
                offset += 1; // Attributes byte 0
                self.write_to_object_space(offset, 0)?;
                offset += 1; // Attributes byte 1
                self.write_to_object_space(offset, 0)?;
                offset += 1; // Attributes byte 2
                self.write_to_object_space(offset, 0)?;
                offset += 1; // Attributes byte 3
                self.write_to_object_space(offset, 0)?;
                offset += 1; // Parent object (none)
                self.write_to_object_space(offset, 0)?;
                offset += 1; // Sibling object (none)
                self.write_to_object_space(offset, 0)?;
                offset += 1; // Child object (none)

                // Properties pointer - create actual property table using proper system
                // Create minimal object data for player object (object #1)
                use crate::grue_compiler::codegen_objects::ObjectData;
                use crate::grue_compiler::ir::IrProperties;
                let mut player_properties = IrProperties::new();
                // Player needs basic properties that games might access
                if let Some(&location_prop) = self.property_numbers.get("location") {
                    player_properties.set_byte(location_prop, 0); // No initial location
                }
                if let Some(&quit_pending_prop) = self.property_numbers.get("quit_pending") {
                    player_properties.set_byte(quit_pending_prop, 0); // Not pending quit
                }

                let player_object = ObjectData {
                    id: 9999, // Synthetic player object ID
                    name: "yourself".to_string(),
                    short_name: "yourself".to_string(),
                    attributes: crate::grue_compiler::ir::IrAttributes::new(),
                    properties: player_properties,
                    parent: None,
                    sibling: None,
                    child: None,
                };

                // Use proper property table creation system
                let prop_table_offset = self.create_property_table_from_ir(1, &player_object)?;
                self.write_to_object_space(offset, (prop_table_offset >> 8) as u8)?;
                offset += 1; // High byte
                self.write_to_object_space(offset, (prop_table_offset & 0xFF) as u8)?;
                offset += 1; // Low byte
            }
            ZMachineVersion::V4 | ZMachineVersion::V5 => {
                // V4/V5 object format: 6 bytes attributes + 2 bytes parent + 2 bytes sibling + 2 bytes child + 2 bytes properties
                for _ in 0..6 {
                    self.write_to_object_space(offset, 0)?;
                    offset += 1; // Attributes bytes
                }
                self.write_to_object_space(offset, 0)?;
                offset += 1; // Parent object (none) high
                self.write_to_object_space(offset, 0)?;
                offset += 1; // Parent object (none) low
                self.write_to_object_space(offset, 0)?;
                offset += 1; // Sibling object (none) high
                self.write_to_object_space(offset, 0)?;
                offset += 1; // Sibling object (none) low
                self.write_to_object_space(offset, 0)?;
                offset += 1; // Child object (none) high
                self.write_to_object_space(offset, 0)?;
                offset += 1; // Child object (none) low

                // Properties pointer - create actual property table using proper system
                // Create minimal object data for player object (object #1)
                use crate::grue_compiler::codegen_objects::ObjectData;
                use crate::grue_compiler::ir::IrProperties;
                let mut player_properties = IrProperties::new();
                // Player needs basic properties that games might access
                if let Some(&location_prop) = self.property_numbers.get("location") {
                    player_properties.set_byte(location_prop, 0); // No initial location
                }
                if let Some(&quit_pending_prop) = self.property_numbers.get("quit_pending") {
                    player_properties.set_byte(quit_pending_prop, 0); // Not pending quit
                }

                let player_object = ObjectData {
                    id: 9999, // Synthetic player object ID
                    name: "yourself".to_string(),
                    short_name: "yourself".to_string(),
                    attributes: crate::grue_compiler::ir::IrAttributes::new(),
                    properties: player_properties,
                    parent: None,
                    sibling: None,
                    child: None,
                };

                // Use proper property table creation system
                let prop_table_offset = self.create_property_table_from_ir(1, &player_object)?;
                self.write_to_object_space(offset, (prop_table_offset >> 8) as u8)?;
                offset += 1; // High byte
                self.write_to_object_space(offset, (prop_table_offset & 0xFF) as u8)?;
                offset += 1; // Low byte
            }
        }
        log::debug!(" Minimal object entry written (object #1)");

        // Phase 3: Write minimal property table (just terminator)
        self.write_to_object_space(offset, 0)?;
        offset += 1; // Property table terminator
        self.write_to_object_space(offset, 0)?;
        // offset += 1; // Padding/alignment (unused)
        log::debug!(" Minimal property table written");

        log::debug!(
            "🎯 Minimal object table complete ({} bytes written)",
            self.object_space.len()
        );
        Ok(())
    }

    // Property analysis functions

    /// Analyze all property accesses across the IR program and build global property registry
    pub fn analyze_properties(&mut self, ir: &IrProgram) -> Result<(), CompilerError> {
        debug!("Starting property analysis...");

        // Step 1: Collect all property names from all instructions
        let mut all_properties = IndexSet::new();

        // Analyze functions
        for function in &ir.functions {
            self.collect_properties_from_block(&function.body, &mut all_properties);
        }

        // Analyze init block
        if let Some(init_block) = &ir.init_block {
            self.collect_properties_from_block(init_block, &mut all_properties);
        }

        // Step 2: Add essential properties that player object always needs
        all_properties.insert("description".to_string()); // Player description property
                                                          // location removed - uses object tree parent only (Oct 12, 2025)

        // Use property numbers from IR's PropertyManager to ensure consistency
        // This ensures object table generation uses the same property numbers as IR code generation
        for (property_name, property_number) in ir.property_manager.get_property_numbers() {
            self.property_numbers
                .insert(property_name.clone(), *property_number);
            debug!(
                "Using IR property '{}' -> number {} (from PropertyManager)",
                property_name, property_number
            );
        }

        // Step 3: Analyze which properties each object uses
        self.analyze_object_property_usage(ir);

        debug!(
            "Property analysis complete. {} properties registered.",
            self.property_numbers.len()
        );
        Ok(())
    }

    /// Collect all property names from instructions in a block
    pub fn collect_properties_from_block(
        &mut self,
        block: &IrBlock,
        properties: &mut IndexSet<String>,
    ) {
        for instruction in &block.instructions {
            match instruction {
                IrInstruction::GetProperty { property, .. } => {
                    properties.insert(property.clone());
                }
                IrInstruction::SetProperty { property, .. } => {
                    properties.insert(property.clone());
                }
                _ => {} // Other instructions don't access properties
            }
        }
    }

    /// Analyze which properties each object uses (for complete property table generation)
    pub fn analyze_object_property_usage(&mut self, ir: &IrProgram) {
        // For now, assume all objects use all properties (conservative approach)
        // This ensures every object has complete property tables
        let all_property_names: Vec<String> = self.property_numbers.keys().cloned().collect();

        // Add the implicit "player" object
        self.object_properties
            .insert("player".to_string(), all_property_names.clone());

        // Add all room names (rooms are objects in Z-Machine)
        for room in &ir.rooms {
            self.object_properties
                .insert(room.name.clone(), all_property_names.clone());
        }

        // Add all explicit objects
        for object in &ir.objects {
            self.object_properties
                .insert(object.name.clone(), all_property_names.clone());
        }

        debug!(
            "Object property usage analysis complete. {} objects analyzed.",
            self.object_properties.len()
        );
    }

    /// Generate object and property tables
    pub fn generate_object_tables(&mut self, ir: &IrProgram) -> Result<(), CompilerError> {
        log::info!("=== OBJECT TABLE GENERATION DEBUG ===");
        log::info!("Target version: {:?}", self.version);
        log::info!(
            "IR contains: {} rooms, {} objects",
            ir.rooms.len(),
            ir.objects.len()
        );

        // Step 1: Generate property defaults table
        let default_props = match self.version {
            ZMachineVersion::V3 => 31,
            ZMachineVersion::V4 | ZMachineVersion::V5 => 63,
        };

        debug!(
            "Generating property defaults table ({} entries)",
            default_props
        );

        // Calculate required space for object table
        let default_props_size = default_props * 2;
        let num_objects = ir.rooms.len() + ir.objects.len() + 1; // +1 for player
        let obj_entry_size = match self.version {
            ZMachineVersion::V3 => 9,
            ZMachineVersion::V4 | ZMachineVersion::V5 => 14,
        };
        let estimated_size = default_props_size + num_objects * obj_entry_size + 1000; // Extra for property tables

        self.allocate_object_space(estimated_size)?;
        log::debug!(" Object space allocated: {} bytes", estimated_size);

        let mut offset = 0;

        // Write property defaults table to object space
        for i in 0..default_props {
            // Use IR property defaults if available, otherwise 0
            let prop_num = (i + 1) as u8;
            let default_value = ir.property_defaults.get_default(prop_num);

            self.write_to_object_space(offset, (default_value >> 8) as u8)?; // High byte
            offset += 1;
            self.write_to_object_space(offset, (default_value & 0xFF) as u8)?; // Low byte
            offset += 1;
        }

        // Step 2: Create object entries for all IR objects (rooms + objects)
        let objects_start = offset; // Continue from where defaults ended
        debug!("Object entries start at offset {}", objects_start);

        // Collect all objects (rooms and objects) from IR
        // Player object is now created in IR (see ir.rs add_player_object())
        // IMPORTANT: Player MUST be object #1 per Z-Machine convention
        let mut all_objects = Vec::new();

        // Add player FIRST (must be object #1)
        // Player is always first in ir.objects (inserted at index 0 by add_player_object)
        if !ir.objects.is_empty() {
            let player = &ir.objects[0];
            all_objects.push(ObjectData {
                id: player.id,
                name: player.name.clone(),
                short_name: player.short_name.clone(),
                attributes: player.attributes.clone(),
                properties: player.properties.clone(),
                parent: player.parent,
                sibling: player.sibling,
                child: player.child,
            });
            log::info!(
                "Object #1: PLAYER '{}' (ID: {}, short: '{}')",
                player.name,
                player.id,
                player.short_name
            );
        }

        // Add rooms as objects (rooms are just objects with specific properties)
        for room in &ir.rooms {
            let mut room_properties = IrProperties::new();

            // Add essential room properties that games commonly access
            // Get property numbers from the global property registry
            let desc_prop = *self.property_numbers.get("description").unwrap_or(&7);
            let visited_prop = *self.property_numbers.get("visited").unwrap_or(&2);
            // location_prop removed - uses object tree parent only (Oct 12, 2025)
            let on_look_prop = *self.property_numbers.get("on_look").unwrap_or(&13);

            // Set default property values for rooms
            room_properties.set_string(desc_prop, room.description.clone());
            room_properties.set_byte(visited_prop, 0); // Initially not visited
                                                       // location property removed - rooms use object tree containment (Oct 12, 2025)
            room_properties.set_byte(on_look_prop, 0); // No special on_look handler by default

            // Generate exit properties for room navigation using parallel arrays
            // Architecture: Three parallel-array properties enable runtime direction lookup
            // - exit_directions: Packed array of dictionary word addresses (2 bytes each)
            // - exit_types: Array of type codes (1 byte each): 0=room, 1=blocked
            // - exit_data: Packed array of data values (2 bytes each): room_id or message_addr
            // Return encoding: (type << 14) | data
            // See docs/ARCHITECTURE.md "Exit System Architecture" for full specification
            if !room.exits.is_empty() {
                let exit_directions_prop =
                    *self.property_numbers.get("exit_directions").unwrap_or(&20);
                let exit_types_prop = *self.property_numbers.get("exit_types").unwrap_or(&21);
                let exit_data_prop = *self.property_numbers.get("exit_data").unwrap_or(&22);

                let mut direction_addrs: Vec<u8> = Vec::new();
                let mut exit_types: Vec<u8> = Vec::new();
                let mut exit_data: Vec<u8> = Vec::new();
                let mut direction_names: Vec<String> = Vec::new();
                // Track blocked exit messages for UnresolvedReference creation during property serialization
                let mut blocked_messages: Vec<(usize, u32)> = Vec::new();

                for (exit_index, (direction, exit_target)) in room.exits.iter().enumerate() {
                    // Store direction name for later DictionaryRef UnresolvedReference creation
                    direction_names.push(direction.clone());

                    // Write placeholder for dictionary address (will be resolved during property serialization)
                    // This matches Bug 9 fix pattern for exit_data string addresses
                    direction_addrs.push(0xFF);
                    direction_addrs.push(0xFF);

                    // Add to exit_types and exit_data based on target type
                    match exit_target {
                        crate::grue_compiler::ir::IrExitTarget::Room(room_ir_id) => {
                            // Type 0 = normal room exit
                            exit_types.push(0);

                            // BUG FIX (Oct 11, 2025): Translate IR ID to Z-Machine object number
                            // The room_ir_id is from semantic analysis (e.g., 20-30)
                            // We need the actual Z-Machine object number (e.g., 1-14)
                            // Use room_to_object_id map which was set up in setup_room_to_object_mapping()
                            let room_obj_num = self
                                .room_to_object_id
                                .get(room_ir_id)
                                .copied()
                                .unwrap_or_else(|| {
                                    log::error!(
                                        "Exit system: Room '{}' exit direction '{}' references IR ID {} which has no object number mapping, using 0",
                                        room.name,
                                        direction,
                                        room_ir_id
                                    );
                                    0
                                });

                            // Data = room object ID (2 bytes)
                            exit_data.push((room_obj_num >> 8) as u8);
                            exit_data.push((room_obj_num & 0xFF) as u8);

                            log::debug!(
                                "Exit system: Room '{}' exit {} direction '{}' -> room IR ID {} = object {}",
                                room.name,
                                exit_index,
                                direction,
                                room_ir_id,
                                room_obj_num
                            );
                        }
                        crate::grue_compiler::ir::IrExitTarget::Blocked(message) => {
                            // Type 1 = blocked exit with message
                            exit_types.push(1);

                            // Get string ID for the blocked message
                            // The message was already collected during string collection phase
                            let string_id = match self.find_or_create_string_id(message) {
                                Ok(id) => id,
                                Err(e) => {
                                    log::error!(
                                        "Failed to get string ID for blocked exit message in room '{}': {:?}",
                                        room.name,
                                        e
                                    );
                                    continue;
                                }
                            };

                            // Write placeholder - will be patched with packed string address during property serialization
                            // This matches Bug 9 fix pattern for exit_data string addresses
                            exit_data.push(0xFF);
                            exit_data.push(0xFF);

                            // Track this blocked exit for UnresolvedReference creation
                            blocked_messages.push((exit_index, string_id));

                            log::debug!(
                                "Exit system: Room '{}' exit {} direction '{}' blocked with message '{}' (string_id={})",
                                room.name,
                                exit_index,
                                direction,
                                message,
                                string_id
                            );
                        }
                    }
                }

                // Write parallel array properties to room object
                if !direction_addrs.is_empty() {
                    log::debug!(
                        "🔍 EXIT_PROPS: Room '{}' BEFORE set_bytes - exit_data length={}, contents={:02x?}",
                        room.name,
                        exit_data.len(),
                        exit_data
                    );
                    log::debug!(
                        "🔍 EXIT_PROPS: Room '{}' - exit_types length={}, contents={:02x?}",
                        room.name,
                        exit_types.len(),
                        exit_types
                    );

                    room_properties.set_bytes(exit_directions_prop, direction_addrs);
                    room_properties.set_bytes(exit_types_prop, exit_types.clone());
                    room_properties.set_bytes(exit_data_prop, exit_data.clone());

                    // Verify properties were stored
                    if let Some(stored_data) = room_properties.properties.get(&exit_data_prop) {
                        log::debug!(
                            "🔍 EXIT_PROPS: Room '{}' AFTER set_bytes - Property {} stored successfully: {:?}",
                            room.name,
                            exit_data_prop,
                            stored_data
                        );
                    } else {
                        log::error!(
                            "❌ EXIT_PROPS: Room '{}' - Property {} NOT FOUND after set_bytes!",
                            room.name,
                            exit_data_prop
                        );
                    }

                    // Store direction names for DictionaryRef UnresolvedReference creation during serialization
                    self.room_exit_directions
                        .insert(room.name.clone(), direction_names);

                    // Store blocked exit messages for StringRef UnresolvedReference creation during serialization
                    if !blocked_messages.is_empty() {
                        self.room_exit_messages
                            .insert(room.name.clone(), blocked_messages);
                    }

                    log::debug!(
                        "Exit system: Generated parallel arrays for room '{}' with {} exits",
                        room.name,
                        room.exits.len()
                    );
                }
            }

            // Log all properties for this room
            log::debug!(
                "🔍 ROOM_PROPS: Room '{}' has {} properties:",
                room.name,
                room_properties.properties.len()
            );
            for (prop_num, prop_value) in &room_properties.properties {
                log::debug!("🔍   Property {}: {:?}", prop_num, prop_value);
            }

            all_objects.push(ObjectData {
                id: room.id,
                name: room.name.clone(),
                short_name: room.display_name.clone(),
                attributes: IrAttributes::new(), // Rooms have default attributes
                properties: room_properties,
                parent: None,
                sibling: None,
                child: None,
            });

            log::info!(
                "Object #{}: ROOM '{}' (ID: {}, short: '{}')",
                all_objects.len(),
                room.name,
                room.id,
                room.display_name
            );
        }

        // Add regular objects (skip player - already added as object #1)
        for object in ir.objects.iter().skip(1) {
            let mut object_properties = object.properties.clone();

            // Ensure all objects have essential properties that games commonly access
            // location_prop removed - uses object tree parent only (Oct 12, 2025)
            let desc_prop = *self.property_numbers.get("description").unwrap_or(&7);

            // location property removed - objects use object tree containment (Oct 12, 2025)

            // Add desc property if missing (use short_name as fallback)
            if !object_properties.properties.contains_key(&desc_prop) {
                object_properties.set_string(desc_prop, object.short_name.clone());
            }

            all_objects.push(ObjectData {
                id: object.id,
                name: object.name.clone(),
                short_name: object.short_name.clone(),
                attributes: object.attributes.clone(),
                properties: object_properties,
                parent: object.parent,
                sibling: object.sibling,
                child: object.child,
            });

            log::info!(
                "Object #{}: OBJECT '{}' (ID: {}, short: '{}')",
                all_objects.len(),
                object.name,
                object.id,
                object.short_name
            );
        }

        log::info!("=== OBJECT ID MAPPING ===",);
        log::info!(
            "Total objects to generate: {} ({} rooms + {} objects + 1 player)",
            all_objects.len(),
            ir.rooms.len(),
            ir.objects.len()
        );

        // Step 3: Build object ID mapping table
        let mut object_id_to_number: IndexMap<IrId, u8> = IndexMap::new();
        for (index, object) in all_objects.iter().enumerate() {
            let obj_num = (index + 1) as u8; // Objects are numbered starting from 1
            object_id_to_number.insert(object.id, obj_num);
            log::info!(
                "ID Mapping: IR ID {} → Object #{} ('{}')",
                object.id,
                obj_num,
                object.short_name
            );
        }

        // Step 4: Create object table entries
        for (index, object) in all_objects.iter().enumerate() {
            let obj_num = (index + 1) as u8; // Objects are numbered starting from 1
            self.create_object_entry_from_ir_with_mapping(obj_num, object, &object_id_to_number)?;
        }

        // Update final_assembly_address to reflect the end of object and property tables
        // current_property_addr points to where the next property table would go
        self.set_final_assembly_address(
            self.current_property_addr,
            "Object table generation complete",
        );

        debug!(
            "Object table generation complete, object_address updated to: 0x{:04x}",
            self.object_address
        );

        // COMPREHENSIVE OBJECT TABLE DUMP FOR DEBUGGING
        log::warn!("=== COMPLETE OBJECT TABLE GENERATION DUMP ===");
        for (index, object) in all_objects.iter().enumerate() {
            let obj_num = (index + 1) as u8;
            log::warn!(
                "🏠 OBJECT #{}: '{}' (ID: {}, short: '{}')",
                obj_num,
                object.name,
                object.id,
                object.short_name
            );
            log::warn!(
                "   Properties ({} total):",
                object.properties.properties.len()
            );

            for (&prop_num, prop_value) in &object.properties.properties {
                match prop_value {
                    crate::grue_compiler::ir::IrPropertyValue::String(s) => {
                        log::warn!("     Property {}: String = \"{}\"", prop_num, s);
                    }
                    crate::grue_compiler::ir::IrPropertyValue::Word(w) => {
                        log::warn!("     Property {}: Word = {}", prop_num, w);
                    }
                    crate::grue_compiler::ir::IrPropertyValue::Byte(b) => {
                        log::warn!("     Property {}: Byte = {}", prop_num, b);
                    }
                    crate::grue_compiler::ir::IrPropertyValue::Bytes(bytes) => {
                        log::warn!("     Property {}: Bytes = {:?}", prop_num, bytes);
                    }
                }
            }
            log::warn!("");
        }
        log::warn!("=== END OBJECT TABLE GENERATION DUMP ===");

        Ok(())
    }
}
