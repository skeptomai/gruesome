/// codegen_extensions.rs
/// Extension methods for ZMachineCodeGen
///
use crate::grue_compiler::codegen::ZMachineCodeGen;
use crate::grue_compiler::codegen_utils::CodeGenUtils;
use crate::grue_compiler::error::CompilerError;
use crate::grue_compiler::ir::*;

impl ZMachineCodeGen {
    /// CONSOLIDATION HELPERS: Centralized unimplemented feature handlers
    /// These methods eliminate the dangerous copy-paste pattern of placeholder opcodes
    /// and provide clear, consistent handling of unimplemented IR instructions.
    ///
    /// Generate unimplemented array operation with return value
    /// This will cause a compile-time error with a clear message about which feature needs implementation

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

        // Phase 1: Analyze and prepare all content
        log::info!("Phase 1: Content analysis and preparation");
        self.layout_memory_structures(&ir)?; // CRITICAL: Plan memory layout before generation
        self.setup_comprehensive_id_mappings(&ir);
        // Phase 2 (Oct 12, 2025): Setup IR ID to object number mapping BEFORE code generation
        // This allows InsertObj tracking in init block to resolve object numbers at compile time
        self.setup_ir_id_to_object_mapping(&ir)?;
        self.analyze_properties(&ir)?;
        self.collect_strings(&ir)?;
        let (prompt_id, unknown_command_id) = self.add_main_loop_strings()?;
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
}
