/// codegen_spaces.rs
/// Image assembly methods for ZMachineCodeGen
///
use crate::grue_compiler::codegen::ZMachineCodeGen;
use crate::grue_compiler::codegen_utils::CodeGenUtils;
use crate::grue_compiler::error::CompilerError;
use crate::grue_compiler::ir::*;
use crate::grue_compiler::ZMachineVersion;

impl ZMachineCodeGen {
    /// Generate global variables space (240 variables * 2 bytes = 480 bytes)
    pub fn generate_globals_space(&mut self, _ir: &IrProgram) -> Result<(), CompilerError> {
        log::debug!("🌐 Generating global variables space");

        // Z-Machine specification requires 240 global variables (variables $10-$FF)
        // Each variable is 2 bytes, so total space is 480 bytes
        const NUM_GLOBALS: usize = 240;
        const BYTES_PER_GLOBAL: usize = 2;
        const TOTAL_GLOBALS_SIZE: usize = NUM_GLOBALS * BYTES_PER_GLOBAL;

        self.globals_space.resize(TOTAL_GLOBALS_SIZE, 0);
        log::debug!(
            " Global variables space created: {} bytes ({} variables)",
            self.globals_space.len(),
            NUM_GLOBALS
        );
        Ok(())
    }

    /// Generate abbreviations space for string compression
    pub fn generate_abbreviations_space(&mut self, _ir: &IrProgram) -> Result<(), CompilerError> {
        log::debug!("📚 Generating abbreviations space");

        // Z-Machine abbreviations table has 3 tables of 32 entries each (96 total)
        // Each entry is a word address (2 bytes), so total is 192 bytes
        const NUM_ABBREVIATIONS: usize = 96;
        const BYTES_PER_ABBREVIATION: usize = 2;
        const TOTAL_ABBREVIATIONS_SIZE: usize = NUM_ABBREVIATIONS * BYTES_PER_ABBREVIATION;

        // Analyze strings to identify common patterns for abbreviation
        let abbreviation_candidates = self.analyze_strings_for_abbreviations();

        // Create abbreviation table
        self.abbreviations_space.resize(TOTAL_ABBREVIATIONS_SIZE, 0);

        // Store common abbreviations as strings to be encoded later
        // For now, we'll create placeholders that will be filled during final assembly
        let mut abbreviations_created = 0;
        for (index, candidate) in abbreviation_candidates
            .iter()
            .take(NUM_ABBREVIATIONS)
            .enumerate()
        {
            // Store the abbreviation string for later encoding
            // Each abbreviation gets a unique ID starting from a high number to avoid conflicts
            let abbrev_id = 10000 + index as IrId;
            self.strings.push((abbrev_id, candidate.clone()));
            log::debug!("📚 Created abbreviation {}: '{}'", index, candidate);
            abbreviations_created += 1;
        }

        log::debug!(
            " Abbreviations space created: {} bytes ({}/{} abbreviations populated)",
            self.abbreviations_space.len(),
            abbreviations_created,
            NUM_ABBREVIATIONS
        );
        Ok(())
    }

    /// Analyze collected strings to identify the best abbreviation candidates
    ///
    /// This function implements intelligent string analysis to find optimal abbreviation
    /// candidates based on frequency analysis and space savings potential. It examines
    /// both individual words and short phrases to maximize compression efficiency.
    ///
    /// The Z-Machine abbreviation system allows up to 32 abbreviations (numbered 0-31)
    /// that can significantly reduce file size by eliminating string duplication.
    fn analyze_strings_for_abbreviations(&self) -> Vec<String> {
        use std::collections::HashMap;

        let mut word_counts = HashMap::new();
        let mut phrase_counts = HashMap::new();

        // Count individual words and short phrases
        for (_, string) in &self.strings {
            // Count words
            for word in string.split_whitespace() {
                if word.len() >= 3 && word.len() <= 8 {
                    *word_counts.entry(word.to_string()).or_insert(0) += 1;
                }
            }

            // Count 2-word phrases
            let words: Vec<&str> = string.split_whitespace().collect();
            for window in words.windows(2) {
                let phrase = format!("{} {}", window[0], window[1]);
                if phrase.len() >= 4 && phrase.len() <= 12 {
                    *phrase_counts.entry(phrase).or_insert(0) += 1;
                }
            }
        }

        // Collect candidates, prioritizing by frequency and savings potential
        let mut candidates = Vec::new();

        // Add high-frequency words first (minimum 3 occurrences, good savings potential)
        let mut words: Vec<(String, usize)> = word_counts
            .into_iter()
            .filter(|(word, count)| *count >= 3 && word.len() >= 3)
            .collect();
        words.sort_by(|(_, a), (_, b)| b.cmp(a)); // Sort by frequency descending

        for (word, count) in words.iter().take(20) {
            let savings = (word.len() - 1) * count; // Rough savings calculation
            log::debug!(
                "📊 Word candidate: '{}' (×{}, ~{} bytes saved)",
                word,
                count,
                savings
            );
            candidates.push(word.clone());
        }

        // Add high-frequency phrases
        let mut phrases: Vec<(String, usize)> = phrase_counts
            .into_iter()
            .filter(|(phrase, count)| *count >= 2 && phrase.len() >= 4)
            .collect();
        phrases.sort_by(|(_, a), (_, b)| b.cmp(a)); // Sort by frequency descending

        for (phrase, count) in phrases.iter().take(10) {
            let savings = (phrase.len() - 1) * count;
            log::debug!(
                "📊 Phrase candidate: '{}' (×{}, ~{} bytes saved)",
                phrase,
                count,
                savings
            );
            candidates.push(phrase.clone());
        }

        // Add some common Z-Machine game patterns manually
        let common_patterns = vec![
            "You can't".to_string(),
            "You are".to_string(),
            "You have".to_string(),
            "There is".to_string(),
            "the ".to_string(),
        ];

        for pattern in common_patterns {
            if !candidates.contains(&pattern) {
                candidates.push(pattern);
            }
        }

        log::debug!("📚 Generated {} abbreviation candidates", candidates.len());
        candidates
    }

    /// Generate code instructions to code space
    pub fn generate_code_to_space(&mut self, ir: &IrProgram) -> Result<(), CompilerError> {
        log::info!("Phase 2: IR to instruction translation tracking");
        log::info!(
            " INPUT: {} functions, {} IR instructions total",
            ir.functions.len(),
            CodeGenUtils::count_total_ir_instructions(ir)
        );

        // CRITICAL FIX (Oct 28, 2025): Dual Object Numbering System Resolution
        //
        // LEGACY FUNCTION REMOVED: setup_object_mappings() populated ir_id_to_object_number
        // using old ir.object_numbers mapping, creating conflicting numbering systems.
        //
        // TIMELINE OF BUG:
        // 1. setup_object_mappings() called here, populating wrong mapping (mailbox = Object #3)
        // 2. InsertObj instructions processed using this wrong mapping
        // 3. Later setup_ir_id_to_object_mapping() overwrote mapping (mailbox = Object #10)
        // 4. Property tables used correct mapping, but InsertObj already used wrong mapping
        // 5. Runtime: Wrong object properties displayed due to numbering mismatch
        //
        // SOLUTION: Object ID mappings are now set up ONCE in Phase 1 via setup_ir_id_to_object_mapping()
        // ensuring consistent numbering throughout the entire compilation pipeline.
        //
        // debug!("Setting up object mappings for IR → Z-Machine object resolution");
        // self.setup_object_mappings(ir); // REMOVED: Causes dual object numbering conflicts

        // NOTE: Builtin function generation moved to after main code generation
        // This allows builtin functions registered during IR processing to be created
        // See Phase 2.1.5 below

        // CRITICAL ARCHITECTURE FIX: Use code_address to track code_space positions
        // During code generation, we track positions within code_space using code_address
        // This eliminates any ambiguity about which address space we're working in
        // NOTE: main_program_offset is now set AFTER builtin functions are generated (Phase 2A.7)
        self.code_address = self.code_space.len(); // Set to current code_space position
        log::info!(
 "🏁 Code generation phase: Using code_address to track code_space position - starting at offset 0x{:04x}",
 self.code_address
 );

        // CRITICAL: In V1-V5, PC points directly to first instruction, NOT a routine header
        // Only V6 uses routines for main entry point (per Z-Machine spec section 5.5)
        log::info!("🏁 Starting code generation - PC will point directly to first instruction");

        // Phase 2.0: Functions will be registered with REAL addresses during code generation
        // Functions are registered with actual addresses during code generation
        log::info!(
            " FUNCTION_REGISTRATION: Functions will be registered during actual code generation"
        );

        // Phase 2.0.5: Generate init block using proper routine architecture (single path)
        // CRITICAL: This replaces the old inline generation to eliminate competing paths
        // Phase 2.1.5: Init block generation moved to Phase 2A.8 (after builtin functions)
        // This prevents memory space conflict between init block and builtin functions

        let initial_code_size = self.code_space.len();

        // Phase 2.1: Generate ALL function definitions
        // PHASE 2A: Pre-register all function addresses to solve forward reference issues
        log::info!(" PRE-REGISTERING: All function addresses for forward reference resolution");
        let mut simulated_address = self.code_space.len();
        for function in ir.functions.iter() {
            // Simulate alignment padding
            if matches!(self.version, ZMachineVersion::V4 | ZMachineVersion::V5) {
                while !simulated_address.is_multiple_of(4) {
                    simulated_address += 1; // Account for padding bytes
                }
            }

            // Pre-register this function's address in function_addresses only
            // DO NOT insert into ir_id_to_address yet - that will happen during actual code generation
            // Otherwise the corruption prevention in record_code_offset will reject the real address
            self.function_addresses
                .insert(function.id, simulated_address);

            log::debug!(
                " PRE-REGISTERED: Function '{}' (IR ID {}) at projected address 0x{:04x} (estimate only)",
                function.name,
                function.id,
                simulated_address
            );

            // Estimate function size for next function's address calculation
            // Header: 1 byte (local count) + 2*locals (default values) + body instructions
            let estimated_size =
                1 + (function.local_vars.len() * 2) + (function.body.instructions.len() * 4);
            simulated_address += estimated_size;
        }

        // PHASE 2A.5: Generate builtin functions after pre-registration but before main generation
        // This ensures builtin functions are available during main code generation
        log::debug!("Generating builtin functions after pre-registration phase");
        self.generate_builtin_functions()?;

        // PHASE 2A.6: Save builtin space end address (Option C fix for Bug #90)
        // Regular functions will start after this point to prevent overwriting builtin functions
        self.builtin_space_end = self.code_space.len();
        log::debug!(
            "Builtin functions end at code space offset: 0x{:04x}",
            self.builtin_space_end
        );

        // PHASE 2A.7: Set temporary main program offset for builtin alignment
        // This will be updated to actual init start address after init block generation
        let temp_main_program_offset = self.builtin_space_end;
        log::debug!("🎯 TEMP_MAIN_PROGRAM_OFFSET: Init block generation will start at code_space[0x{:04x}] after builtin functions", temp_main_program_offset);
        self.code_address = self.code_space.len(); // Set to current code_space position

        // PHASE 2A.8: Generate init block AFTER builtin functions (CRITICAL FIX)
        // The init block was previously generated before builtins, causing memory conflict
        let _init_locals_count = if let Some(init_block) = &ir.init_block {
            log::info!(
                " GENERATING: Init block as proper Z-Machine routine ({} instructions) - AFTER builtins",
                init_block.instructions.len()
            );
            let (startup_address, init_locals_count) = self.generate_init_block(init_block, ir)?;

            // DISASSEMBLER COMPATIBILITY FIX: Use actual init start for PC calculation
            // startup_address now points to first instruction (after dummy header)
            self.main_program_offset = startup_address;

            self.init_routine_locals_count = init_locals_count;
            log::info!(
                " Init block generated - dummy header at 0x{:04x}, actual PC target at 0x{:04x}",
                temp_main_program_offset,
                startup_address
            );
            init_locals_count
        } else {
            log::debug!("No init block found");
            // No init block - use temp offset as final offset
            self.main_program_offset = temp_main_program_offset;
            0
        };

        // PHASE 2B: Now generate actual function code with all addresses pre-registered
        log::info!(" TRANSLATING: All function definitions");
        for (i, function) in ir.functions.iter().enumerate() {
            let function_start_size = self.code_space.len();
            log::debug!(
                " TRANSLATING: Function #{}: '{}' ({} instructions)",
                i,
                function.name,
                function.body.instructions.len()
            );

            // Align function addresses according to Z-Machine version requirements
            log::debug!(
                " FUNCTION_ALIGN: Function '{}' before alignment at code_address=0x{:04x}",
                function.name,
                self.code_address
            );
            match self.version {
                ZMachineVersion::V3 => {
                    // v3: functions must be at even addresses
                    if !self.code_address.is_multiple_of(2) {
                        log::debug!(" FUNCTION_ALIGN: Adding padding byte for even alignment");
                        self.emit_byte(0xB4)?; // nop instruction (safe padding that won't crash)
                    }
                }
                ZMachineVersion::V4 | ZMachineVersion::V5 => {
                    // v4/v5: functions must be at 4-byte boundaries
                    while !self.code_address.is_multiple_of(4) {
                        self.emit_byte(0xB4)?; // nop instruction (safe padding that won't crash)
                    }
                }
            }
            log::debug!(
                " FUNCTION_ALIGN: Function '{}' after alignment at code_address=0x{:04x}",
                function.name,
                self.code_address
            );

            // CRITICAL: Store relative function address (will be converted to absolute in Phase 3)
            // During Phase 2, final_code_base is still 0x0000, so we store relative addresses
            let relative_func_addr = self.code_space.len();
            log::debug!(" FUNCTION_ADDRESS_FIX: Function '{}' stored at relative address 0x{:04x} (Phase 2)", function.name, relative_func_addr);
            let actual_func_addr = relative_func_addr; // Will be converted to absolute during assembly
            self.function_addresses
                .insert(function.id, actual_func_addr);
            // CRITICAL FIX: Use record_code_space_offset for relative addresses during Phase 2
            // record_final_address is for absolute addresses only
            self.record_code_space_offset(function.id, actual_func_addr);
            log::debug!(
                " FUNCTION_UPDATED: '{}' (ID {}) updated to actual address 0x{:04x}",
                function.name,
                function.id,
                actual_func_addr
            );

            // CRITICAL: Set up all local variable mappings BEFORE translating instructions
            // This includes parameters AND local variables (loop counters, let bindings)
            self.setup_function_local_mappings(function);

            // CRITICAL: Register function IR ID to address mapping for proper resolution BEFORE instruction generation
            self.reference_context
                .ir_id_to_address
                .insert(function.id, actual_func_addr);

            // CRITICAL: Generate Z-Machine routine header (local count + default values)
            log::debug!(
                " GENERATING: Routine header for '{}' with {} locals",
                function.name,
                function.local_vars.len()
            );
            self.generate_function_header(function, ir)?;

            // CRITICAL: Function address must point to header, not first instruction!
            // The Z-Machine interpreter needs to read the header to allocate local variables.
            // The actual_func_addr (code_space.len() before generate_function_header) already
            // points to the header start, which is correct.
            log::debug!(
                " FUNCTION_ADDRESS: '{}' address 0x{:04x} points to header (correct for Z-Machine calls)",
                function.name, actual_func_addr
            );

            // Track each instruction translation
            for (instr_i, instruction) in function.body.instructions.iter().enumerate() {
                let instr_start_size = self.code_space.len();
                log::trace!(" [{:02}] IR: {:?}", instr_i, instruction);

                // Attempt to translate IR instruction
                match self.generate_instruction(instruction) {
                    Ok(()) => {
                        let bytes_generated = self.code_space.len() - instr_start_size;
                        log::trace!(" [{:02}] Generated: {} bytes", instr_i, bytes_generated);

                        if bytes_generated == 0 {
                            // Check if this is expected zero-byte generation
                            match instruction {
                                IrInstruction::LoadImmediate { .. }
                                | IrInstruction::Nop
                                | IrInstruction::Label { .. } => {
                                    // These instructions correctly generate no bytecode
                                }
                                _ => {
                                    log::debug!(
                                        " ZERO BYTES: IR instruction generated no bytecode: {:?}",
                                        instruction
                                    );
                                }
                            }
                        }
                    }
                    Err(e) => {
                        log::debug!(
                            "Translation failed for instruction {:?}: {}",
                            instruction,
                            e
                        );
                        // Continue processing other instructions
                    }
                }
            }

            // Process any pending labels at end of function
            // Labels at the end of a function (like endif labels after a branch)
            // won't have a following instruction to trigger deferred label processing.
            // We must process them here before adding the implicit return.
            // Multiple labels can converge at the same address (e.g., nested if statements).
            if !self.pending_labels.is_empty() {
                let label_address = self.code_address;
                // Collect labels first to avoid borrow issues
                let labels_to_process: Vec<_> = self.pending_labels.drain(..).collect();
                for label_id in labels_to_process {
                    log::debug!(
 "END_OF_FUNCTION_LABEL: Processing pending label {} at end of function at address 0x{:04x}",
 label_id, label_address
 );
                    self.label_addresses.insert(label_id, label_address);
                    self.record_final_address(label_id, label_address);
                }
            }

            // Check if function needs implicit return
            let has_return = self.block_ends_with_return(&function.body);
            log::debug!(
                "Function '{}' ends with return: {}",
                function.name,
                has_return
            );

            if !has_return {
                log::debug!("Adding implicit return to function '{}'", function.name);
                self.emit_return(None)?;
            }

            let function_bytes = self.code_space.len() - function_start_size;
            log::info!(
                " Function '{}' complete: {} bytes generated",
                function.name,
                function_bytes
            );

            if function_bytes == 0 {
                log::debug!(
                    " FUNCTION_ZERO: Function '{}' generated no bytecode from {} instructions",
                    function.name,
                    function.body.instructions.len()
                );
            }

            // CRITICAL: Patch function header with actual local count after instruction generation
            self.finalize_function_header(function.id)?;
        }

        // Phase 2.1.5: Builtin functions already generated at start of code generation
        // (Moved earlier to be available during main code generation)

        // Phase 2.2: Init block now handled as part of main routine (above)

        // Phase 2.3: Add program flow control
        log::debug!(
            " PROGRAM_FLOW: Adding flow control based on mode: {:?}",
            ir.program_mode
        );
        match ir.program_mode {
            crate::grue_compiler::ast::ProgramMode::Script => {
                log::debug!("Script mode: No additional instructions needed (init block already terminates with quit)");
                // No additional instructions needed - init block already has quit instruction
            }
            crate::grue_compiler::ast::ProgramMode::Interactive => {
                log::debug!("Interactive mode: Generating main loop");
                self.generate_program_flow(ir)?;
            }
            crate::grue_compiler::ast::ProgramMode::Custom => {
                log::debug!("Custom mode: Adding main function call placeholder");
                // TODO: Generate call to user main function
                self.emit_byte(0xBA)?; // quit - temporary
            }
        }

        let total_code_generated = self.code_space.len() - initial_code_size;
        let total_ir_instructions = CodeGenUtils::count_total_ir_instructions(ir);
        log::info!(
            " PHASE 2 COMPLETE: Generated {} bytes from {} IR instructions",
            total_code_generated,
            total_ir_instructions
        );

        // Analyze all instructions across functions and init block
        let empty_vec = vec![];
        let all_instructions: Vec<&IrInstruction> = ir
            .functions
            .iter()
            .flat_map(|f| &f.body.instructions)
            .chain(
                ir.init_block
                    .as_ref()
                    .map(|init| &init.instructions)
                    .unwrap_or(&empty_vec)
                    .iter(),
            )
            .collect();

        let cloned_instructions: Vec<IrInstruction> =
            all_instructions.into_iter().cloned().collect();
        let (expected_bytecode_instructions, expected_zero_instructions, actual_instructions) =
            CodeGenUtils::analyze_instruction_expectations(&cloned_instructions);

        if expected_bytecode_instructions > 0 && total_code_generated == 0 {
            log::debug!("Translation failure: {} instructions expected to generate bytecode, but 0 bytes generated", 
 expected_bytecode_instructions);
            log::info!(
                " PHASE2_ANALYSIS: {} bytecode instructions, {} zero-byte instructions, {} total",
                expected_bytecode_instructions,
                expected_zero_instructions,
                actual_instructions
            );
        } else if expected_bytecode_instructions == 0 && expected_zero_instructions > 0 {
            log::info!(" PHASE2_ANALYSIS: All {} instructions correctly generated zero bytes (string literals, labels, etc.)", 
 expected_zero_instructions);
        } else {
            log::info!(" PHASE2_ANALYSIS: {} bytecode instructions, {} zero-byte instructions = {} bytes generated", 
 expected_bytecode_instructions, expected_zero_instructions, total_code_generated);
        }

        Ok(())
    }
}
