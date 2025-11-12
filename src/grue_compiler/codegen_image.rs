/// codegen_image.rs
/// Image assembly methods for ZMachineCodeGen
///
use crate::grue_compiler::codegen::ZMachineCodeGen;
use crate::grue_compiler::codegen_memory::{MemorySpace, HEADER_SIZE};
use crate::grue_compiler::codegen_utils::CodeGenUtils;
use crate::grue_compiler::error::CompilerError;
use crate::grue_compiler::ir::*;
use crate::grue_compiler::ZMachineVersion;

use log::debug;

impl ZMachineCodeGen {
    /// SEPARATED SPACES GENERATION: New architecture to eliminate memory conflicts
    /// This method uses separate working spaces during compilation and final assembly
    /// to eliminate the memory corruption issues that plagued the unified approach.
    /// Fixed: Header field population and test compatibility issues resolved
    /// PURE SEPARATED SPACES: Complete Z-Machine file generator
    ///
    /// This method creates a complete Z-Machine file using only separated memory spaces.
    /// NO legacy dependencies, NO story_data usage, clean architecture throughout.
    ///
    /// File Layout (documented exactly):
    /// 0x0000-0x003F: Header (64 bytes) - Z-Machine header with all addresses
    /// 0x0040-?????: Code space - Program code (init + main loop + functions)
    /// ?????-?????: String space - All encoded Z-Machine strings
    /// ?????-?????: Object space - Object table + property tables + defaults
    /// ?????-?????: Buffer space - Text input buffer + Parse buffer (for sread)
    ///
    pub fn generate_complete_game_image(
        &mut self,
        ir: IrProgram,
    ) -> Result<Vec<u8>, CompilerError> {
        log::info!("Z-Machine file generation: starting game image generation");

        // Phase 0: IR Input Analysis & Validation (DEBUG)
        CodeGenUtils::log_ir_inventory(&ir);
        CodeGenUtils::validate_ir_input(&ir)?;

        // Phase 0.5: Initialize system messages for builtin functions
        self.initialize_system_messages(&ir);

        // Phase 1: Analyze and prepare all content
        log::info!("Phase 1: Content analysis and preparation");
        self.layout_memory_structures(&ir)?; // CRITICAL: Plan memory layout before generation
        self.setup_comprehensive_id_mappings(&ir);
        // Phase 2 (Oct 12, 2025): Setup IR ID to object number mapping BEFORE code generation
        // This allows InsertObj tracking in init block to resolve object numbers at compile time
        self.setup_ir_id_to_object_mapping(&ir)?;
        self.analyze_properties(&ir)?;
        self.collect_strings(&ir)?;
        let (prompt_id, unknown_command_id) = self.add_main_loop_strings(&ir)?;
        self.main_loop_prompt_id = Some(prompt_id);
        self.main_loop_unknown_command_id = Some(unknown_command_id);
        self.encode_all_strings()?;
        log::info!(" Phase 1 complete: Content analysis and string encoding finished");

        // Phase 2: Generate ALL Z-Machine sections to separated working spaces
        log::info!("Phase 2: Generate ALL Z-Machine sections to separated memory spaces");
        self.generate_all_zmachine_sections(&ir)?;
        log::info!(" Phase 2 complete: All Z-Machine sections generated");

        // DEBUG: Show space population before final assembly
        self.debug_space_population();

        // Phase 3: Calculate precise layout and assemble final image
        log::info!(" Phase 3: Calculate comprehensive layout and assemble complete image");
        let mut final_game_image = self.assemble_complete_zmachine_image(&ir)?;
        log::info!(" Phase 3 complete: Final Z-Machine image assembled");

        // Phase 4: Reinitialize input buffers (after all resizes are complete)
        log::debug!(" Phase 4: Reinitializing input buffers");
        self.reinitialize_input_buffers_in_image(&mut final_game_image);

        // Phase 5: Final validation
        log::debug!(" Phase 5: Validating final Z-Machine image");
        // Final validation disabled - can be enabled for additional checks
        // self.validate_final_assembly()?;

        log::info!(
            "🎉 COMPLETE Z-MACHINE FILE generation complete: {} bytes",
            final_game_image.len()
        );

        Ok(final_game_image)
    }

    /// Generate ALL Z-Machine sections to separated memory spaces (COMPLETE Z-MACHINE FORMAT)
    /// This function generates ALL required Z-Machine sections according to specification:
    /// 1. Code space - executable functions and main loop
    /// 2. String space - encoded text literals
    /// 3. Object space - object table, properties, and relationships
    /// 4. Dictionary space - word parsing dictionary
    /// 5. Global variables space - 240 global variable slots
    /// 6. Abbreviations space - string compression abbreviations
    /// Made public for use by codegen_extensions.rs
    pub fn generate_all_zmachine_sections(&mut self, ir: &IrProgram) -> Result<(), CompilerError> {
        log::info!("Phase 2: Generating ALL Z-Machine sections to separated memory spaces");

        // STACK DISCIPLINE FIX (Oct 30, 2025): Analyze comparison usage patterns before code generation

        self.comparison_ids_used_as_values = self.analyze_comparison_usage_patterns(ir);
        log::info!(
            "🔍 USAGE_ANALYSIS: Identified {} comparison operations that need push/pull mechanism",
            self.comparison_ids_used_as_values.len()
        );

        // Phase 2a: Generate strings to string_space
        log::debug!("📝 Step 2a: Generating string space");
        log::debug!(
            " STRING_DEBUG: encoded_strings contains {} entries",
            self.encoded_strings.len()
        );

        // Check if String ID 148 is in encoded_strings
        if self.encoded_strings.contains_key(&148) {
            log::debug!(" STRING_DEBUG: String ID 148 IS in encoded_strings");
        } else {
            log::debug!(" STRING_DEBUG: String ID 148 is NOT in encoded_strings");
            log::debug!(
                " STRING_DEBUG: Available encoded string IDs: {:?}",
                self.encoded_strings.keys().collect::<Vec<_>>()
            );
        }

        // IndexMap preserves insertion order, but need to collect to avoid borrow issues
        let encoded_strings: Vec<_> = self
            .encoded_strings
            .iter()
            .map(|(&id, data)| (id, data.clone()))
            .collect();
        for (string_id, string_data) in encoded_strings {
            self.allocate_string_space(string_id, &string_data)?;
        }
        log::info!(
            " Step 2a complete: String space populated ({} bytes)",
            self.string_space.len()
        );

        // Phase 2b: Generate dictionary to dictionary_space (BEFORE objects for exit system)
        // CRITICAL: Dictionary must be generated before objects because room exit properties
        // need to look up direction words (north, south, etc.) in the dictionary
        log::debug!("📖 Step 2b: Generating dictionary space");
        self.generate_dictionary_space(ir)?;
        log::info!(
            " Step 2b complete: Dictionary space populated ({} bytes)",
            self.dictionary_space.len()
        );

        // Phase 2c: Setup room-to-object ID mapping (needed for exit data generation)
        log::debug!("🗺️  Step 2c-pre: Setting up room-to-object ID mapping");
        self.setup_room_to_object_mapping(ir)?;
        log::info!(
            " Step 2c-pre complete: Mapped {} rooms to object IDs",
            self.room_to_object_id.len()
        );

        // Phase 2c: Generate objects/properties to object_space
        log::debug!("🏠 Step 2c: Generating object space");
        if ir.has_objects() {
            log::debug!("Generating full object table for Interactive program");
            self.setup_object_table_generation();
            // CRITICAL FIX (Oct 28, 2025): Pre-process InsertObj instructions before object generation
            // This populates initial_locations_by_number so objects can be created with correct parent relationships
            self.preprocess_insertobj_instructions(ir)?;
            self.generate_object_tables(ir)?;
        } else {
            log::debug!("Generating minimal object table for Script program");
            self.generate_objects_to_space(ir)?;
        }
        // Check if property table was written correctly
        let obj2_offset = 0x0047;
        let obj2_prop_ptr_offset = obj2_offset + 7;
        if obj2_prop_ptr_offset + 1 < self.object_space.len() {
            let prop_addr = ((self.object_space[obj2_prop_ptr_offset] as u16) << 8)
                | (self.object_space[obj2_prop_ptr_offset + 1] as u16);
            log::warn!(
                "🔍 OBJ_SPACE_CHECK: After object generation, object 2 prop pointer = 0x{:04x}",
                prop_addr
            );
            if (prop_addr as usize) + 20 <= self.object_space.len() {
                log::warn!(
                    "🔍   Property table at offset 0x{:04x}: {:02x?}",
                    prop_addr,
                    &self.object_space[prop_addr as usize..(prop_addr as usize + 20)]
                );
            }
        }

        log::info!(
            " Step 2c complete: Object space populated ({} bytes)",
            self.object_space.len()
        );

        // Phase 2d: Generate global variables to globals_space
        log::debug!("🌐 Step 2d: Generating global variables space");
        self.generate_globals_space(ir)?;
        // CRITICAL FIX: Actually initialize global variable values (especially G00 = player object #1)
        self.generate_global_variables(ir)?;
        log::info!(
            " Step 2d complete: Globals space populated ({} bytes)",
            self.globals_space.len()
        );

        // Phase 2e: Generate abbreviations to abbreviations_space
        log::debug!("📚 Step 2e: Generating abbreviations space");
        self.generate_abbreviations_space(ir)?;
        log::info!(
            " Step 2e complete: Abbreviations space populated ({} bytes)",
            self.abbreviations_space.len()
        );

        // Phase 2f: Generate executable code to code_space
        log::debug!("💻 Step 2f: Generating code space");
        self.generate_code_to_space(ir)?;
        log::info!(
            " Step 2f complete: Code space populated ({} bytes)",
            self.code_space.len()
        );

        // Phase 2g: Collect any new strings created during code generation
        log::debug!(" Step 2g: Checking for new strings created during code generation");
        let initial_string_count = self.string_space.len();

        // Find any new strings that were added during code generation
        // IndexMap preserves insertion order, but need to collect to avoid borrow issues
        let current_encoded_strings: Vec<_> = self
            .encoded_strings
            .iter()
            .map(|(&id, data)| (id, data.clone()))
            .collect();
        for (string_id, string_data) in current_encoded_strings {
            if !self.string_offsets.contains_key(&string_id) {
                log::debug!(
                    " NEW_STRING: Found new string ID {} created during code generation: '{}'",
                    string_id,
                    self.ir_id_to_string
                        .get(&string_id)
                        .unwrap_or(&"[ENCODED_ONLY]".to_string())
                );
                self.allocate_string_space(string_id, &string_data)?;
            }
        }

        let new_string_bytes = self.string_space.len() - initial_string_count;
        if new_string_bytes > 0 {
            log::info!(
                " Step 2g complete: Added {} bytes of new strings created during code generation",
                new_string_bytes
            );
        } else {
            log::debug!(" Step 2g complete: No new strings created during code generation");
        }

        // Phase 2h: Summary of ALL section generation
        log::info!("Z-Machine sections summary:");
        log::info!(
            " ├─ Code space: {} bytes (functions, main loop, initialization)",
            self.code_space.len()
        );
        log::info!(
            " ├─ String space: {} bytes (encoded text literals)",
            self.string_space.len()
        );
        log::info!(
            " ├─ Object space: {} bytes (object table, properties, relationships)",
            self.object_space.len()
        );
        log::info!(
            " ├─ Dictionary space: {} bytes (word parsing dictionary)",
            self.dictionary_space.len()
        );
        log::info!(
            " ├─ Globals space: {} bytes (240 global variable slots)",
            self.globals_space.len()
        );
        log::info!(
            " ├─ Abbreviations space: {} bytes (string compression table)",
            self.abbreviations_space.len()
        );
        log::info!(" └─ Pending address fixups: {}", self.pending_fixups.len());

        Ok(())
    }

    // === IR INPUT ANALYSIS & VALIDATION (DEBUG PHASE 1) ===

    /// Assemble complete Z-Machine image from all separated spaces (COMPLETE Z-MACHINE FORMAT)
    ///
    /// This function takes all the content generated in separated memory spaces and
    /// combines them into a complete, valid Z-Machine file with proper memory layout.
    ///
    /// File Layout (calculated exactly):
    /// 0x0000-0x003F: Header (64 bytes) - Generated fresh with accurate addresses
    /// 0x0040-?????: Code space - All executable code (init, main loop, functions)
    /// ?????-?????: String space - All encoded strings (if any)
    /// ?????-?????: Object space - Object table + properties (if any)
    /// ?????-?????: Buffer space - Input buffers for sread operations (if needed)
    /// Made public for use by codegen_extensions.rs
    ///
    pub fn assemble_complete_zmachine_image(
        &mut self,
        _ir: &IrProgram,
    ) -> Result<Vec<u8>, CompilerError> {
        log::info!(" Phase 3: Assembling complete Z-Machine image from ALL separated spaces");

        // Phase 3a: Calculate precise memory layout for ALL Z-Machine sections
        log::debug!("📏 Step 3a: Calculating comprehensive memory layout");
        let header_size = HEADER_SIZE; // Always 64 bytes
        let globals_size = self.globals_space.len();
        let abbreviations_size = self.abbreviations_space.len();
        let object_size = self.object_space.len();
        let dictionary_size = self.dictionary_space.len();
        log::debug!("📖 Dictionary size: {} bytes", dictionary_size);
        let string_size = self.string_space.len();
        let code_size = self.code_space.len();

        // Calculate base addresses for each section (following Z-Machine memory layout)
        // Dynamic memory layout: Header -> Globals -> Abbreviations -> Objects -> Static boundary
        // Static memory layout: Dictionary -> Strings -> Code (high memory)
        let mut current_address = header_size;

        // Dynamic memory sections
        let globals_base = current_address;
        current_address += globals_size;

        // Arrays section - allocated after globals, before abbreviations
        let arrays_base = current_address;
        let arrays_size = self.array_codegen.total_array_size();
        log::debug!(
            " Arrays allocated at 0x{:04x}, size={} bytes",
            arrays_base,
            arrays_size
        );
        self.array_codegen
            .set_array_base_address(arrays_base as u16);
        current_address += arrays_size;

        let abbreviations_base = current_address;
        current_address += abbreviations_size;

        let object_base = current_address;
        current_address += object_size;

        // Static memory boundary (dynamic memory ends here, static memory begins)
        // Z-Machine specification: dictionary should be at static memory base
        let static_memory_start = current_address;

        // Dictionary is located at static memory base (Z-Machine convention)
        let dictionary_base = current_address;
        log::debug!(
            " Dictionary allocated at 0x{:04x}, size={} bytes",
            dictionary_base,
            dictionary_size
        );
        current_address += dictionary_size;

        // High memory sections - align string base for Z-Machine requirements
        let string_base = match self.version {
            ZMachineVersion::V3 => {
                // V3 requires string addresses to be even
                if current_address % 2 != 0 {
                    current_address += 1; // Add padding byte
                }
                current_address
            }
            ZMachineVersion::V4 | ZMachineVersion::V5 => {
                // V4+ requires string addresses to be divisible by 4
                let remainder = current_address % 4;
                if remainder != 0 {
                    current_address += 4 - remainder; // Add padding bytes
                }
                current_address
            }
        };
        current_address += string_size;

        let code_base = current_address;
        log::debug!(
            " Code allocated at 0x{:04x}, size={} bytes",
            code_base,
            code_size
        );
        current_address += code_size;

        // Total file size calculation
        let total_size = current_address;

        // Store final addresses for header generation
        self.final_code_base = code_base;
        self.final_string_base = string_base;
        self.final_object_base = object_base;
        self.final_abbreviations_base = abbreviations_base;
        self.dictionary_addr = dictionary_base;
        self.global_vars_addr = globals_base;

        // CRITICAL: Convert all code generation offsets to final addresses
        log::debug!(" Step 3-CONVERT: Converting code space offsets to final addresses");
        self.convert_offsets_to_addresses();

        // CRITICAL FIX: Convert relative function addresses to absolute addresses
        // During Phase 2, functions were stored with relative addresses (code_space offset)
        // Now that we know final_code_base, convert them to absolute addresses
        log::debug!(
            " PHASE3_FIX: Converting {} function addresses from relative to absolute",
            self.function_addresses.len()
        );
        let mut updated_mappings = Vec::new();
        for (func_id, relative_addr) in self.function_addresses.iter_mut() {
            let absolute_addr = self.final_code_base + *relative_addr;
            log::debug!(
                " PHASE3_FIX: Function ID {} address 0x{:04x} → 0x{:04x} (relative + 0x{:04x})",
                func_id,
                *relative_addr,
                absolute_addr,
                self.final_code_base
            );
            *relative_addr = absolute_addr;
            updated_mappings.push((*func_id, absolute_addr));
        }
        // Update address mappings after iteration
        for (func_id, absolute_addr) in updated_mappings {
            self.record_final_address(func_id, absolute_addr);
        }

        // CRITICAL FIX: Convert relative label addresses to absolute addresses
        log::debug!(
            " PHASE3_FIX: Converting {} label addresses from relative to absolute",
            self.label_addresses.len()
        );
        for (label_id, relative_addr) in self.label_addresses.iter_mut() {
            // Only convert if it looks like a relative address (small value during generation)
            if *relative_addr < 0x1000 && self.final_code_base != 0 {
                let absolute_addr = self.final_code_base + *relative_addr;
                debug!(
                    "Phase3 fix: Label ID {} address 0x{:04x} → 0x{:04x} (relative + 0x{:04x})",
                    label_id, *relative_addr, absolute_addr, self.final_code_base
                );
                *relative_addr = absolute_addr;
                self.reference_context
                    .ir_id_to_address
                    .insert(*label_id, absolute_addr);
            }
        }

        log::info!("Z-Machine memory layout:");
        log::info!(
            " ├─ Header: 0x{:04x}-0x{:04x} ({} bytes) - Z-Machine header",
            0,
            header_size,
            header_size
        );
        log::info!(
            " ├─ Globals: 0x{:04x}-0x{:04x} ({} bytes) - Global variables",
            globals_base,
            abbreviations_base,
            globals_size
        );
        log::info!(
            " ├─ Abbreviations:0x{:04x}-0x{:04x} ({} bytes) - String compression",
            abbreviations_base,
            object_base,
            abbreviations_size
        );
        log::info!(
            " ├─ Objects: 0x{:04x}-0x{:04x} ({} bytes) - Object table + properties",
            object_base,
            dictionary_base,
            object_size
        );
        log::info!(
            " ├─ Dictionary: 0x{:04x}-0x{:04x} ({} bytes) - Word parsing dictionary",
            dictionary_base,
            string_base,
            dictionary_size
        );
        log::info!(
            " ├─ Strings: 0x{:04x}-0x{:04x} ({} bytes) - Encoded text literals",
            string_base,
            code_base,
            string_size
        );
        log::info!(
            " ├─ Code: 0x{:04x}-0x{:04x} ({} bytes) - Executable functions",
            code_base,
            total_size,
            code_size
        );
        log::info!(" └─ Total: {} bytes (Complete Z-Machine file)", total_size);

        // PC calculation preview (final calculation happens in Step 3e)
        let expected_pc = if self.init_routine_locals_count > 0 {
            let init_header_size = 1 + (self.init_routine_locals_count as usize * 2);
            code_base + init_header_size
        } else {
            code_base // No init block, PC points to first function header
        };
        log::info!(
 "🎯 PC CALCULATION: PC will point to 0x{:04x} (init_locals_count={}, code_base=0x{:04x})",
 expected_pc, self.init_routine_locals_count, code_base
 );

        // Phase 3b: Initialize final game image
        log::debug!(
            "Step 3b: Initializing {} byte complete Z-Machine image",
            total_size
        );
        self.final_data = vec![0; total_size];

        // Phase 3c: Generate static header fields (version, serial, flags)
        // This phase writes only fields that don't change based on memory layout:
        // - Version number, release number, flags
        // - Serial number (compilation date)
        // - Standard revision info
        // Address fields remain as 0x0000 placeholders
        log::debug!("📝 Step 3c: Generating static header fields");
        self.generate_static_header_fields()?;

        // Phase 3d: Copy ALL content spaces to final positions IN MONOTONIC ORDER
        log::debug!("Step 3d: Copying ALL separated spaces to final image (header-first monotonic approach)");

        // Header already written directly to final_data[0..64] by generate_complete_header()
        // No copy needed - this maintains monotonic address allocation

        // Copy global variables space
        if !self.globals_space.is_empty() {
            self.final_data[globals_base..arrays_base].copy_from_slice(&self.globals_space);
            log::debug!(
                " Globals space copied: {} bytes at 0x{:04x}",
                globals_size,
                globals_base
            );
        }

        // Initialize and copy array memory
        if arrays_size > 0 {
            // Create array memory buffer and initialize it
            let mut array_memory = vec![0u8; arrays_size];
            self.array_codegen
                .initialize_array_memory(&mut array_memory)?;

            self.final_data[arrays_base..abbreviations_base].copy_from_slice(&array_memory);
            log::debug!(
                " Arrays space initialized and copied: {} bytes at 0x{:04x}",
                arrays_size,
                arrays_base
            );
        }

        // Copy abbreviations space
        if !self.abbreviations_space.is_empty() {
            self.final_data[abbreviations_base..object_base]
                .copy_from_slice(&self.abbreviations_space);
            log::debug!(
                " Abbreviations space copied: {} bytes at 0x{:04x}",
                abbreviations_size,
                abbreviations_base
            );
        }

        // Copy object space
        if !self.object_space.is_empty() {
            log::warn!("🔧 OBJECT_COPY: About to copy object_space to final_data");
            log::warn!(
                "   object_base=0x{:04x}, dictionary_base=0x{:04x}, object_size={}",
                object_base,
                dictionary_base,
                object_size
            );
            log::warn!(
                "   object_space.len()={}, slice size={}",
                self.object_space.len(),
                dictionary_base - object_base
            );

            // Show West of House property table BEFORE copy (object 2 at offset 0x0047)
            let obj2_offset = 0x0047;
            let obj2_prop_ptr_offset = obj2_offset + 7;
            if obj2_prop_ptr_offset + 1 < self.object_space.len() {
                let prop_addr = ((self.object_space[obj2_prop_ptr_offset] as u16) << 8)
                    | (self.object_space[obj2_prop_ptr_offset + 1] as u16);
                log::warn!(
                    "   BEFORE: Object 2 prop pointer at offset 0x{:04x} = 0x{:04x}",
                    obj2_prop_ptr_offset,
                    prop_addr
                );
                if (prop_addr as usize) < self.object_space.len() {
                    let prop_offset = prop_addr as usize;
                    log::warn!(
                        "   BEFORE: Property table at offset 0x{:04x}: {:02x?}",
                        prop_offset,
                        &self.object_space[prop_offset
                            ..prop_offset
                                .min(self.object_space.len())
                                .min(prop_offset + 20)]
                    );
                }
            }

            self.final_data[object_base..dictionary_base].copy_from_slice(&self.object_space);

            // Show West of House property table AFTER copy
            let final_obj2_offset = object_base + obj2_offset;
            let final_prop_ptr_offset = final_obj2_offset + 7;
            if final_prop_ptr_offset + 1 < self.final_data.len() {
                let prop_addr = ((self.final_data[final_prop_ptr_offset] as u16) << 8)
                    | (self.final_data[final_prop_ptr_offset + 1] as u16);
                log::warn!(
                    "   AFTER: Object 2 prop pointer at 0x{:04x} = 0x{:04x}",
                    final_prop_ptr_offset,
                    prop_addr
                );
                if (prop_addr as usize) < self.final_data.len() {
                    log::warn!(
                        "   AFTER: Property table at 0x{:04x}: {:02x?}",
                        prop_addr,
                        &self.final_data[prop_addr as usize
                            ..(prop_addr as usize + 20).min(self.final_data.len())]
                    );
                }
            }

            log::debug!(
                " Object space copied: {} bytes at 0x{:04x}",
                object_size,
                object_base
            );

            // CRITICAL FIX: Patch property table addresses from object space relative to absolute addresses
            self.patch_property_table_addresses(object_base)?;
        }

        // Copy dictionary space
        if !self.dictionary_space.is_empty() {
            let dictionary_end = dictionary_base + self.dictionary_space.len();
            self.final_data[dictionary_base..dictionary_end]
                .copy_from_slice(&self.dictionary_space);
            log::debug!(
                " Dictionary space copied: {} bytes at 0x{:04x}",
                self.dictionary_space.len(),
                dictionary_base
            );
        }

        // Copy string space
        if !self.string_space.is_empty() {
            let allocated_size = code_base - string_base;
            let actual_size = self.string_space.len();

            if actual_size != allocated_size {
                log::warn!(
                    "⚠️  STRING SPACE SIZE MISMATCH: allocated={} bytes, actual={} bytes",
                    allocated_size,
                    actual_size
                );
            }

            // CRITICAL: Verify slice bounds match string_space size
            if actual_size > allocated_size {
                return Err(CompilerError::CodeGenError(format!(
                    "String space overflow: {} bytes needed but only {} allocated",
                    actual_size, allocated_size
                )));
            }

            // DEBUG: Check what's in string_space before copying
            log::debug!(
                " STRING_SPACE_DEBUG: First 16 bytes of string_space: {:02x?}",
                &self.string_space[0..16.min(actual_size)]
            );

            // Copy only actual_size bytes, not the full allocated slice
            self.final_data[string_base..string_base + actual_size]
                .copy_from_slice(&self.string_space);

            // DEBUG: Verify what got copied
            log::debug!(
                " STRING_SPACE_DEBUG: First 16 bytes at final_data[0x{:04x}]: {:02x?}",
                string_base,
                &self.final_data[string_base..string_base + 16.min(actual_size)]
            );

            log::debug!(
                " String space copied: {} bytes at 0x{:04x} (allocated: {} bytes)",
                actual_size,
                string_base,
                allocated_size
            );
        }

        // Copy code space
        if !self.code_space.is_empty() {
            log::debug!(
                " CODE_COPY_DEBUG: code_base=0x{:04x}, total_size=0x{:04x}, code_space.len()={}",
                code_base,
                total_size,
                self.code_space.len()
            );
            log::debug!(
                " CODE_COPY_DEBUG: Slice bounds [{}..{}] = {} bytes",
                code_base,
                total_size,
                total_size - code_base
            );

            log::debug!(
                " CODE_COPY_DEBUG: Code space first 10 bytes: {:?}",
                &self.code_space[0..std::cmp::min(10, self.code_space.len())]
            );

            // Check what's at the problematic location before copying
            let problem_offset = 0x335; // This becomes 0x127F after adding code_base
            if problem_offset < self.code_space.len() {
                log::debug!("BEFORE COPY: code_space[0x{:04x}..0x{:04x}] = {:02x} {:02x} {:02x} {:02x} {:02x} {:02x}",
 problem_offset,
 problem_offset + 6,
 self.code_space[problem_offset],
 if problem_offset + 1 < self.code_space.len() { self.code_space[problem_offset + 1] } else { 0 },
 if problem_offset + 2 < self.code_space.len() { self.code_space[problem_offset + 2] } else { 0 },
 if problem_offset + 3 < self.code_space.len() { self.code_space[problem_offset + 3] } else { 0 },
 if problem_offset + 4 < self.code_space.len() { self.code_space[problem_offset + 4] } else { 0 },
 if problem_offset + 5 < self.code_space.len() { self.code_space[problem_offset + 5] } else { 0 }
 );
            }

            self.final_data[code_base..total_size].copy_from_slice(&self.code_space);

            log::debug!(
                " Code space copied: {} bytes at 0x{:04x}",
                code_size,
                code_base
            );

            // DEBUG: Check test() function header after copy
            let test_func_offset = 0x0014; // From FINALIZE_DEBUG log
            if test_func_offset < code_size {
                let test_func_addr = code_base + test_func_offset;
                log::debug!(
                    " CODE_COPY_VERIFY: test() function at code_space[0x{:04x}] = 0x{:02x}, final_data[0x{:04x}] = 0x{:02x}",
                    test_func_offset,
                    self.code_space[test_func_offset],
                    test_func_addr,
                    self.final_data[test_func_addr]
                );
            }
            log::debug!(
                " CODE_COPY_VERIFY: Final data at code_base first 10 bytes: {:?}",
                &self.final_data[code_base..code_base + std::cmp::min(10, code_size)]
            );
        }

        // Phase 3e: Update address fields with final calculated addresses
        // This phase updates ONLY the address fields in the header with final memory layout.
        // Critical: Never touches static fields like serial number or version.
        // Updates: PC start, dictionary, objects, globals, static memory, abbreviations, high memory base
        log::debug!(" Step 3e: Updating header address fields with final memory layout");
        // ARCHITECTURAL FIX: PC calculation for main program with proper routine header
        // PC must point to first instruction AFTER the routine header (header size = 1 byte for local count)
        // CRITICAL FIX: Account for builtin functions that precede the main program in code_space
        let calculated_pc = (self.final_code_base + self.main_program_offset + 1) as u16;
        log::debug!(
 " PC_CALCULATION_DEBUG: final_code_base=0x{:04x}, main_program_offset=0x{:04x}, calculated_pc=0x{:04x}",
 self.final_code_base, self.main_program_offset, calculated_pc
 );
        log::debug!(
 " PC_CALCULATION_DEBUG: PC will point to first instruction at 0x{:04x} (after routine header)",
 calculated_pc
 );
        self.fixup_header_addresses(
            calculated_pc,                 // pc_start (points after routine header)
            self.dictionary_addr as u16,   // dictionary_addr
            self.final_object_base as u16, // objects_addr
            self.global_vars_addr as u16,  // globals_addr
            static_memory_start as u16,    // static_memory_base
            abbreviations_base as u16,     // abbreviations_addr
            self.final_code_base as u16,   // high_mem_base
        )?;

        // Phase 3e.5: Map all object IR IDs to addresses (CRITICAL FIX for UnresolvedReference resolution)
        log::debug!(
            " Step 3e.5: Mapping all object IR IDs to addresses for UnresolvedReference resolution"
        );
        self.map_all_object_ir_ids();

        // Phase 3e.6: CENTRALIZED IR MAPPING - Consolidate ALL IR ID types
        log::debug!(" Step 3e.6: Consolidating ALL IR ID mappings (functions, strings, labels)");
        self.consolidate_all_ir_mappings();

        // Phase 3f: Resolve all address references (including string properties)
        log::debug!(" Step 3f: Resolving all address references and fixups");
        self.resolve_all_addresses()?;

        // Phase 3g: Finalize file metadata (length and checksum - must be last)
        // This phase calculates and writes file length and checksum.
        // MUST be called last since it depends on the complete final file.
        // Updates: File length (bytes 26-27), Checksum (bytes 28-29)
        log::debug!(" Step 3g: Finalizing file length and checksum");
        self.finalize_header_metadata()?;

        log::info!(
            "🎉 COMPLETE Z-MACHINE FILE assembled successfully: {} bytes",
            total_size
        );

        Ok(self.final_data.clone())
    }

    /// Translate space-relative address to final assembly layout address (DETERMINISTIC)
    pub fn translate_space_address_to_final(
        &self,
        space: MemorySpace,
        space_offset: usize,
    ) -> Result<usize, CompilerError> {
        let final_address = match space {
            MemorySpace::Header => space_offset,
            MemorySpace::Globals => self.global_vars_addr + space_offset,
            MemorySpace::Abbreviations => self.final_abbreviations_base + space_offset,
            MemorySpace::Objects => self.final_object_base + space_offset,
            MemorySpace::Dictionary => self.dictionary_addr + space_offset,
            MemorySpace::Strings => self.final_string_base + space_offset,
            MemorySpace::Code => {
                // CRITICAL FIX: Use final_code_base directly instead of hardcoded calculation
                // Previous calculation used hardcoded section sizes that didn't match actual layout,
                // causing UnresolvedReference locations to point to operand type bytes instead of operand data
                self.final_code_base + space_offset
            }
            MemorySpace::CodeSpace => {
                // Same as Code
                self.final_code_base + space_offset
            }
        };

        if final_address >= self.final_data.len() {
            return Err(CompilerError::CodeGenError(format!(
                "Address translation {:?}[0x{:04x}] -> 0x{:04x} exceeds final_data size {}",
                space,
                space_offset,
                final_address,
                self.final_data.len()
            )));
        }

        log::debug!(
            "📍 ADDRESS_TRANSLATE: {:?}[0x{:04x}] -> final=0x{:04x}",
            space,
            space_offset,
            final_address
        );
        Ok(final_address)
    }

    /// Debug function: Show comprehensive space population analysis
    pub fn debug_space_population(&self) {
        log::info!(" SPACE POPULATION ANALYSIS:");

        // Code space analysis
        log::info!(" CODE_SPACE: {} bytes", self.code_space.len());
        if !self.code_space.is_empty() {
            let first_10: Vec<String> = self
                .code_space
                .iter()
                .take(10)
                .map(|b| format!("0x{:02x}", b))
                .collect();
            let last_10: Vec<String> = self
                .code_space
                .iter()
                .rev()
                .take(10)
                .rev()
                .map(|b| format!("0x{:02x}", b))
                .collect();
            log::info!(" First 10 bytes: [{}]", first_10.join(", "));
            log::info!(" Last 10 bytes: [{}]", last_10.join(", "));
        }

        // Object space analysis
        log::info!(" 📦 OBJECT_SPACE: {} bytes", self.object_space.len());
        if !self.object_space.is_empty() {
            let first_10: Vec<String> = self
                .object_space
                .iter()
                .take(10)
                .map(|b| format!("0x{:02x}", b))
                .collect();
            log::info!(" First 10 bytes: [{}]", first_10.join(", "));
            let non_zero_count = self.object_space.iter().filter(|&&b| b != 0).count();
            log::info!(
                " Non-zero bytes: {}/{} ({:.1}%)",
                non_zero_count,
                self.object_space.len(),
                (non_zero_count as f32 / self.object_space.len() as f32) * 100.0
            );
        }

        // String space analysis
        log::info!(" 📝 STRING_SPACE: {} bytes", self.string_space.len());
        if !self.string_space.is_empty() {
            let first_10: Vec<String> = self
                .string_space
                .iter()
                .take(10)
                .map(|b| format!("0x{:02x}", b))
                .collect();
            log::info!(" First 10 bytes: [{}]", first_10.join(", "));
            let non_zero_count = self.string_space.iter().filter(|&&b| b != 0).count();
            log::info!(
                " Non-zero bytes: {}/{} ({:.1}%)",
                non_zero_count,
                self.string_space.len(),
                (non_zero_count as f32 / self.string_space.len() as f32) * 100.0
            );
        }

        // Basic logging for globals and dictionary space
        log::debug!("Globals space: {} bytes", self.globals_space.len());
        log::debug!("Dictionary space: {} bytes", self.dictionary_space.len());
    }
}
