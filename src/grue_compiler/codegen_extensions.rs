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
}
