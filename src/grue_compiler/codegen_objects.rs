// Z-Machine Object and Property Code Generator
//
// Handles object table generation, property processing, and core object management
// for the Z-Machine bytecode compiler.

use crate::grue_compiler::error::CompilerError;
use crate::grue_compiler::ir::*;
use crate::grue_compiler::ZMachineVersion;
use log::debug;
use std::collections::{HashMap, HashSet};

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

                // Properties pointer - points to property table right after this object
                let prop_table_offset = default_props_count * 2 + 9; // After default props + this object
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

                // Properties pointer
                let prop_table_offset = default_props_count * 2 + 14; // After default props + this object
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
        let mut all_properties = HashSet::new();

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
        all_properties.insert("location".to_string()); // Player location property

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
        properties: &mut HashSet<String>,
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
        log::info!("🏗️  === OBJECT TABLE GENERATION DEBUG ===");
        log::info!("🏗️  Target version: {:?}", self.version);
        log::info!(
            "🏗️  IR contains: {} rooms, {} objects",
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
        let mut all_objects = Vec::new();

        // CRITICAL FIX: Create player object as object #1
        // This resolves the "get_prop called with object 0" Frotz compatibility issue
        debug!("Creating player object as object #1 for Frotz compatibility");
        let mut player_properties = IrProperties::new();
        // Get property numbers from PropertyManager to ensure consistency
        let location_prop = ir
            .property_manager
            .get_property_number_by_name("location")
            .unwrap_or_else(|| panic!("Property 'location' not found in PropertyManager"));
        let desc_prop = ir
            .property_manager
            .get_property_number_by_name("description")
            .or_else(|| ir.property_manager.get_property_number_by_name("desc"))
            .unwrap_or_else(|| {
                panic!("Property 'description' or 'desc' not found in PropertyManager")
            });
        // Set initial player location to first room (will be room object #2)
        let initial_location = if !ir.rooms.is_empty() { 2 } else { 0 };
        debug!(
            "PROPERTY DEBUG: Setting player location property {} to value {} (0x{:04x})",
            location_prop, initial_location, initial_location
        );
        player_properties.set_word(location_prop, initial_location);
        player_properties.set_string(desc_prop, "yourself".to_string());

        all_objects.push(ObjectData {
            id: 9999u32, // Use high ID to avoid conflicts with actual IR objects
            name: "player".to_string(),
            short_name: "yourself".to_string(),
            attributes: IrAttributes::new(), // Player has default attributes
            properties: player_properties,
            parent: None, // Player parent will be set to location during gameplay
            sibling: None,
            child: None, // Player can contain objects (inventory)
        });
        log::info!(
            "🏗️  Object #1: PLAYER - location property {} = {}, desc property {} = 'yourself'",
            location_prop,
            initial_location,
            desc_prop
        );

        // Add rooms as objects (rooms are just objects with specific properties)
        for room in &ir.rooms {
            let mut room_properties = IrProperties::new();

            // Add essential room properties that games commonly access
            // Get property numbers from the global property registry
            let desc_prop = *self.property_numbers.get("description").unwrap_or(&7);
            let visited_prop = *self.property_numbers.get("visited").unwrap_or(&2);
            let location_prop = *self.property_numbers.get("location").unwrap_or(&8);
            let on_look_prop = *self.property_numbers.get("on_look").unwrap_or(&13);

            // Set default property values for rooms
            room_properties.set_string(desc_prop, room.description.clone());
            room_properties.set_byte(visited_prop, 0); // Initially not visited
            room_properties.set_word(location_prop, 0); // Rooms don't have a location
            room_properties.set_byte(on_look_prop, 0); // No special on_look handler by default

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
                "🏗️  Object #{}: ROOM '{}' (ID: {}, short: '{}')",
                all_objects.len(),
                room.name,
                room.id,
                room.display_name
            );
        }

        // Add regular objects
        for object in &ir.objects {
            let mut object_properties = object.properties.clone();

            // Ensure all objects have essential properties that games commonly access
            let location_prop = *self.property_numbers.get("location").unwrap_or(&8);
            let desc_prop = *self.property_numbers.get("description").unwrap_or(&7);

            // Add location property if missing (default to 0 = no location)
            if !object_properties.properties.contains_key(&location_prop) {
                object_properties.set_word(location_prop, 0);
            }

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
                "🏗️  Object #{}: OBJECT '{}' (ID: {}, short: '{}')",
                all_objects.len(),
                object.name,
                object.id,
                object.short_name
            );
        }

        log::info!("🏗️  === OBJECT ID MAPPING ===",);
        log::info!(
            "🏗️  Total objects to generate: {} ({} rooms + {} objects + 1 player)",
            all_objects.len(),
            ir.rooms.len(),
            ir.objects.len()
        );

        // Step 3: Build object ID mapping table
        let mut object_id_to_number: HashMap<IrId, u8> = HashMap::new();
        for (index, object) in all_objects.iter().enumerate() {
            let obj_num = (index + 1) as u8; // Objects are numbered starting from 1
            object_id_to_number.insert(object.id, obj_num);
            log::info!(
                "🏗️  ID Mapping: IR ID {} → Object #{} ('{}')",
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
        Ok(())
    }
}
