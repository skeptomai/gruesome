// Grammar pattern matching code generation for Grue→Z-Machine compiler
//
// This module handles the generation of verb pattern matching code, which
// processes user input and dispatches to appropriate handler functions based
// on grammar patterns (literal, noun, default, etc.).

use crate::grue_compiler::codegen::{Operand, ZMachineCodeGen};
use crate::grue_compiler::codegen_memory::{placeholder_word, MemorySpace};
use crate::grue_compiler::codegen_references::{LegacyReferenceType, UnresolvedReference};
use crate::grue_compiler::error::CompilerError;
use crate::grue_compiler::ir::{IrId, IrValue};
use crate::grue_compiler::opcodes::*;
use log::debug;

/// Parse buffer global variable number (Global G6e = Variable(110))
const PARSE_BUFFER_GLOBAL: u8 = 110;

/// Extension trait for ZMachineCodeGen to handle grammar pattern matching
impl ZMachineCodeGen {
    /// Generate verb pattern matching code
    ///
    /// This monster function (1,529 lines!) handles all verb pattern matching logic:
    /// - Literal patterns ("around" in "look around")
    /// - Literal+Noun patterns ("at mailbox" in "look at mailbox")
    /// - Noun patterns (verb + object)
    /// - Default patterns (verb only)
    ///
    /// TODO: Break this up into pattern-specific handler methods
    pub fn generate_verb_matching(
        &mut self,
        verb: &str,
        patterns: &[crate::grue_compiler::ir::IrPattern],
        main_loop_jump_id: u32,
    ) -> Result<(), CompilerError> {
        debug!(
" VERB_MATCH_START: Generating verb matching for '{}' with {} patterns at address 0x{:04x}",
verb,
patterns.len(),
self.code_address
);

        let verb_start_address = self.code_address;

        // CRITICAL FIX: Update ir_id_to_address mapping for this verb's function to point to the new
        // address where literal pattern matching is generated, instead of the old address
        // Extract the function ID from the default pattern
        let default_pattern = patterns.iter().find(|p| {
            p.pattern
                .contains(&crate::grue_compiler::ir::IrPatternElement::Default)
        });
        if let Some(pattern) = default_pattern {
            if let crate::grue_compiler::ir::IrHandler::FunctionCall(func_id, _args) =
                &pattern.handler
            {
                let relative_offset = verb_start_address - self.final_code_base;
                debug!(
                    "🔧 FUNCTION_MAPPING_FIX: Updating function ID {} ('{}') mapping from old address to new address 0x{:04x} (relative offset 0x{:04x}) where literal patterns are generated",
                    func_id, verb, verb_start_address, relative_offset
                );
                // Update the ir_id_to_address mapping to point to this new function location
                // Store as relative offset so it survives convert_offsets_to_addresses()
                self.reference_context
                    .ir_id_to_address
                    .insert(*func_id, relative_offset);
                debug!(
                    "🎯 FUNCTION_MAPPING_COMPLETE: Function ID {} now points to address 0x{:04x} (was 0x1514)",
                    func_id, verb_start_address
                );
            }
        }

        // Create end-of-function label for jump target resolution
        // Generate unique label based on verb name to avoid conflicts
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        verb.hash(&mut hasher);
        let end_function_label = 90000 + (hasher.finish() % 9999) as u32;

        // Phase 3.1: Extract tokens from parse buffer for object resolution
        // Parse buffer layout after SREAD:
        // [0] = max words, [1] = actual word count
        // [2] = word 1 dict addr (low), [3] = word 1 dict addr (high)
        // [4] = word 1 text pos, [5] = word 1 length
        // [6] = word 2 dict addr (low), [7] = word 2 dict addr (high)
        // etc.

        // Constants for buffer globals (matching the ones used in main loop)

        // Step 1: Check if first word matches this verb
        // Parse buffer layout: [0]=max, [1]=count, [2]=word1_dict_low, [3]=word1_dict_high, ...

        // First, check if we have at least 1 word (word count >= 1)
        debug!(
            " CHECK_WORD_COUNT: Check if we have at least 1 word at 0x{:04x}",
            self.code_address
        );

        log::debug!(
            "📝 VAR1_WRITE: '{}' at 0x{:04x} - storing word count to Variable(1)",
            verb,
            self.code_address
        );

        self.emit_instruction_typed(
            Opcode::Op2(Op2::Loadb), // loadb: load byte from array (2OP:16)
            &[
                Operand::Variable(PARSE_BUFFER_GLOBAL), // Parse buffer address
                Operand::SmallConstant(1),              // Byte offset 1 = word count
            ],
            Some(1), // Store word count in local variable 1
            None,
        )?;

        // If word count < 1, skip this verb (no words to match)
        let layout = self.emit_instruction_typed(
            Opcode::Op2(Op2::Jl), // jl: jump if less than
            &[
                Operand::Variable(1),      // word count
                Operand::SmallConstant(1), // compare with 1
            ],
            None,
            Some(0xBFFF_u16 as i16), // Placeholder - branch-on-TRUE (skip when condition is true)
        )?;

        // Register branch to end_function_label (skip this verb if no words)
        if let Some(branch_location) = layout.branch_location {
            log::debug!(
                "🟢 BRANCH_REF_CREATED: location=0x{:04x} → target_ir_id={} (end_function_label)",
                branch_location,
                end_function_label
            );
            self.reference_context
                .unresolved_refs
                .push(UnresolvedReference {
                    reference_type: LegacyReferenceType::Branch,
                    location: branch_location,
                    target_id: end_function_label,
                    is_packed_address: false,
                    offset_size: 2,
                    location_space: MemorySpace::Code,
                });
        }

        // Load word 0 dictionary address from parse buffer (VERB for verb matching)
        debug!(
            " LOAD_VERB: Load word 0 dict address (verb) at 0x{:04x}",
            self.code_address
        );

        self.emit_instruction_typed(
            Opcode::Op2(Op2::Loadw), // loadw: load word from array (2OP:15)
            &[
                Operand::Variable(PARSE_BUFFER_GLOBAL), // Parse buffer address
                Operand::SmallConstant(1), // Offset 1 = word 0 dict addr (verb) - CORRECT for verb matching
            ],
            Some(6), // Store word 0 dict addr in local variable 6 (verb dictionary address)
            None,
        )?;

        // CRITICAL FIX: Load dictionary address into a temporary variable first
        // because je (opcode 0x01) in Long form can't handle LargeConstant > 255.
        // Dictionary addresses are typically > 255, causing je to be encoded as
        // VAR form (0xC1 = call_vs) which is a completely different instruction!
        //
        // Solution: Load the large constant into Global G200 (Variable 216) using store instruction,
        // then use je with two Variable operands (Long form encoding).
        // Note: store opcode (0x0d/2OP:13) operands are: (variable, value)
        //
        // ARCHITECTURAL FIX (Oct 3, 2025): Use VAR:storew instead of 2OP:store
        // Problem: 2OP:0x0D (store) with LargeConstant forces VAR form,
        //          which changes opcode to VAR:output_stream (different instruction!)
        //
        // Solution: Write directly to global variable memory using VAR:storew
        // VAR:storew (0x01) takes: base_address, word_offset, value
        // Global variables start at globals_addr (from header)
        // Variable 216 (Global G200) = globals_addr + (216-16)*2 = globals_addr + 400
        //
        // BUG FIX (Oct 11, 2025): NEVER use G01 (Variable 17) - that's SCORE!
        // G01/Variable 17 = globals_addr + 2 = score display in status line
        // G02/Variable 18 = globals_addr + 4 = moves display in status line
        // Safe to use: G200+ (Variable 216+) for temporary storage
        //
        // This is safe because:
        // 1. storew requires 3 operands, so it's ALWAYS VAR form (no form conflict)
        // 2. Opcode 0x01 in VAR form is storew (correct instruction)
        // 3. We're writing to the exact same memory location the variable system uses
        // 4. G200 is far from score/moves and other game state globals
        //
        // Note: globals_addr is a placeholder that gets resolved during layout
        let storew_layout = self.emit_instruction_typed(
            Opcode::OpVar(OpVar::Storew), // VAR:storew (always VAR with 3 operands - no conflict!)
            &[
                Operand::LargeConstant(placeholder_word()), // base = globals_addr (resolved later)
                Operand::SmallConstant(200), // offset = 200 words (for variable 216 = global G200)
                Operand::LargeConstant(placeholder_word()), // value = dict addr (resolved later)
            ],
            None,
            None,
        )?;

        // Create UnresolvedReference for globals base address (first operand)
        // This will be resolved to the actual globals_addr from the header
        self.reference_context
            .unresolved_refs
            .push(UnresolvedReference {
                reference_type: LegacyReferenceType::GlobalsBase,
                location: storew_layout
                    .operand_location
                    .expect("storew should have operand location"),
                target_id: 0, // Special ID for globals base address
                is_packed_address: false,
                offset_size: 2,
                location_space: MemorySpace::Code,
            });

        // Create UnresolvedReference for dictionary address (third operand)
        // Skip first operand (2 bytes) to get to third operand location
        let dict_addr_location = storew_layout
            .operand_location
            .expect("storew should have operand location")
            + 2
            + 1; // +2 for first operand, +1 for second operand

        let verb_dict_addr = self.lookup_word_in_dictionary_with_fixup(verb, dict_addr_location)?;

        debug!(
            " VERB_DICT_ADDR: Verb '{}' will be resolved to dictionary address (placeholder 0x{:04x} at location 0x{:04x})",
            verb, verb_dict_addr, dict_addr_location
        );

        // Compare word 0 dict addr (verb) with this verb's dict addr (now in Global G200/Variable 216)
        // If they DON'T match, skip this verb handler
        debug!(
            "Emitting je at code_address=0x{:04x}: Variable(6) vs Variable(216)",
            self.code_address
        );
        let layout = self.emit_instruction_typed(
            Opcode::Op2(Op2::Je), // je: jump if equal
            &[
                Operand::Variable(6),   // Word 0 dict addr (verb from parse buffer)
                Operand::Variable(216), // This verb's dict addr (from Global G200)
            ],
            None,
            Some(0xBFFF_u16 as i16), // Placeholder - will branch if EQUAL (branch-on-true, 2-byte format)
        )?;
        debug!(
            "je emitted, now at code_address=0x{:04x}",
            self.code_address
        );

        // Register branch: if equal, continue to handler (skip the next jump)
        let continue_label = self.next_string_id;
        self.next_string_id += 1;
        debug!("je will branch to label {} if equal", continue_label);

        if let Some(branch_location) = layout.branch_location {
            debug!(
                "Registering je branch UnresolvedReference at location=0x{:04x} to target_id={}",
                branch_location, continue_label
            );
            self.reference_context
                .unresolved_refs
                .push(UnresolvedReference {
                    reference_type: LegacyReferenceType::Branch,
                    location: branch_location,
                    target_id: continue_label,
                    is_packed_address: false,
                    offset_size: 2,
                    location_space: MemorySpace::Code,
                });
        }

        // If we get here, verb didn't match - skip to end
        let layout = self.emit_instruction_typed(
            Opcode::Op1(Op1::Jump), // jump (unconditional)
            &[Operand::LargeConstant(placeholder_word())],
            None,
            None,
        )?;

        if let Some(operand_location) = layout.operand_location {
            self.reference_context
                .unresolved_refs
                .push(UnresolvedReference {
                    reference_type: LegacyReferenceType::Jump,
                    location: operand_location,
                    target_id: end_function_label,
                    is_packed_address: false,
                    offset_size: 2,
                    location_space: MemorySpace::Code,
                });
        }

        // Register continue_label here (after the jump)
        self.label_addresses
            .insert(continue_label, self.code_address);
        self.record_final_address(continue_label, self.code_address);

        debug!(
            "🎯 VERB_MATCHED: Continue with verb '{}' handler at 0x{:04x} ({} patterns)",
            verb,
            self.code_address,
            patterns.len()
        );

        // Debug: List all patterns for this verb
        for (i, pattern) in patterns.iter().enumerate() {
            let pattern_desc = pattern
                .pattern
                .iter()
                .map(|elem| match elem {
                    crate::grue_compiler::ir::IrPatternElement::Default => "default".to_string(),
                    crate::grue_compiler::ir::IrPatternElement::Noun => "noun".to_string(),
                    crate::grue_compiler::ir::IrPatternElement::Literal(lit) => {
                        format!("\"{}\"", lit)
                    }
                    crate::grue_compiler::ir::IrPatternElement::Adjective => {
                        "adjective".to_string()
                    }
                    crate::grue_compiler::ir::IrPatternElement::MultiWordNoun => {
                        "multi-word-noun".to_string()
                    }
                    crate::grue_compiler::ir::IrPatternElement::Preposition => {
                        "preposition".to_string()
                    }
                    crate::grue_compiler::ir::IrPatternElement::MultipleObjects => {
                        "multiple-objects".to_string()
                    }
                    crate::grue_compiler::ir::IrPatternElement::DirectObject => {
                        "direct-object".to_string()
                    }
                    _ => format!("unknown-pattern"),
                })
                .collect::<Vec<_>>()
                .join(" + ");
            debug!("🔍 PATTERN[{}]: {} => handler", i, pattern_desc);
        }

        // Step 2: Now check word count for pattern selection (noun vs default)
        for (_i, _pattern) in patterns.iter().enumerate() {}

        // Step 2: Check if we have at least 2 words (verb + noun)
        // If word_count >= 2, extract noun and call handler with object parameter
        // If word_count < 2, call handler with no parameters

        // Phase 3.0: Generate literal pattern matching code
        self.generate_literal_patterns(verb, patterns, main_loop_jump_id)?;

        // Phase 3.1: Distinguish between Default (verb-only) and Noun patterns
        // Find appropriate patterns for verb-only and noun cases
        let default_pattern = patterns.iter().find(|p| {
            p.pattern
                .contains(&crate::grue_compiler::ir::IrPatternElement::Default)
        });
        let noun_pattern = patterns.iter().find(|p| {
            p.pattern
                .contains(&crate::grue_compiler::ir::IrPatternElement::Noun)
        });

        debug!(
            "🎯 PATTERN_ANALYSIS: default_pattern={}, noun_pattern={}",
            default_pattern.is_some(),
            noun_pattern.is_some()
        );

        // We need at least one pattern to proceed
        if default_pattern.is_none() && noun_pattern.is_none() {
            return Err(CompilerError::ParseError(
                format!("Verb '{}' has no valid patterns", verb),
                0,
            ));
        }

        // Check if we have a noun (word count >= 2)
        debug!(
            "🔀 BRANCH_CHECK: Generating jl instruction at 0x{:04x} to check if Variable(1) < 2",
            self.code_address
        );

        // Create label for verb-only case (when word count < 2)
        let verb_only_label = self.next_string_id;
        self.next_string_id += 1;

        debug!(
            "🔀 LABEL_CREATE: Created verb_only_label={} for branch target",
            verb_only_label
        );

        // Emit jl with placeholder branch - will be resolved to verb_only_label
        let layout = self.emit_instruction_typed(
            Opcode::Op2(Op2::Jl), // jl: jump if less than
            &[
                Operand::Variable(1),      // word count
                Operand::SmallConstant(2), // compare with 2
            ],
            None,
            Some(0xBFFF_u16 as i16), // Placeholder - branch-on-TRUE (jump to verb_only when word_count < 2)
        )?;

        // Register branch to verb_only_label using proper branch_location from layout
        if let Some(branch_location) = layout.branch_location {
            self.reference_context
                .unresolved_refs
                .push(UnresolvedReference {
                    reference_type: LegacyReferenceType::Branch,
                    location: branch_location,
                    target_id: verb_only_label,
                    is_packed_address: false,
                    offset_size: 2,
                    location_space: MemorySpace::Code,
                });
            debug!(
                "🔀 BRANCH_REF: Registered UnresolvedReference at location=0x{:04x} to label={}",
                branch_location, verb_only_label
            );
        } else {
            panic!("BUG: emit_instruction didn't return branch_location for jl instruction");
        }


        // Phase 3.2: Generate literal+noun pattern matching code
        self.generate_literal_noun_patterns(patterns, main_loop_jump_id)?;

        // Phase 3.3: Generate verb+noun pattern matching code
        if let Some(pattern) = noun_pattern {
            self.generate_verb_noun_patterns(verb, pattern, main_loop_jump_id)?;
        }

        // VERB-ONLY CASE: We have less than 2 words, process default pattern or noun pattern with object ID 0
        // Register verb_only_label at this location
        self.label_addresses
            .insert(verb_only_label, self.code_address);
        self.record_final_address(verb_only_label, self.code_address);
        debug!(
            "🔀 LABEL_REGISTER: Registered verb_only_label={} at address=0x{:04x}",
            verb_only_label, self.code_address
        );

        // Phase 3.4: Generate default/verb-only pattern matching code
        self.generate_default_pattern(verb, default_pattern, noun_pattern, main_loop_jump_id)?;

        // End of verb matching function - register the label for jump resolution
        self.record_final_address(end_function_label, self.code_address);

        log::debug!(
            "📍 VERB_HANDLER: '{}' code range 0x{:04x}-0x{:04x}",
            verb,
            verb_start_address,
            self.code_address
        );

        Ok(())
    }

    /// Generate code for default (verb-only) pattern or noun pattern fallback
    ///
    /// Handles verb-only case when word count < 2.
    /// Calls default pattern handler if available, otherwise calls noun pattern with object ID 0.
    fn generate_default_pattern(
        &mut self,
        verb: &str,
        default_pattern: Option<&crate::grue_compiler::ir::IrPattern>,
        noun_pattern: Option<&crate::grue_compiler::ir::IrPattern>,
        main_loop_jump_id: u32,
    ) -> Result<(), CompilerError> {
        if let Some(pattern) = default_pattern {
            // Handle default pattern (verb-only)
            if let crate::grue_compiler::ir::IrHandler::FunctionCall(func_id, args) =
                &pattern.handler
            {
                debug!(
                    "Generating default pattern call to function ID {} for verb '{}' with {} arguments",
                    func_id, verb, args.len()
                );

                log::debug!(
                    "📍 PATTERN_HANDLER: '{}' default pattern at 0x{:04x} (args={})",
                    verb,
                    self.code_address,
                    args.len()
                );

                // Build operands: function address + arguments
                let mut operands = vec![Operand::LargeConstant(placeholder_word())]; // Function address placeholder

                // Track which arguments need dictionary fixup (position in operands vec -> word string)
                let mut dictionary_fixups: Vec<(usize, String)> = Vec::new();

                // Convert IrValue arguments to operands
                for (i, arg_value) in args.iter().enumerate() {
                    let arg_operand = match arg_value {
                        crate::grue_compiler::ir::IrValue::String(s) => {
                            // CRITICAL FIX: Use placeholder for dictionary addresses, will be patched in Phase 3
                            // Find word position for later resolution
                            let word_lower = s.to_lowercase();
                            let position = self
                                .dictionary_words
                                .iter()
                                .position(|w| w == &word_lower)
                                .ok_or_else(|| {
                                    CompilerError::CodeGenError(format!(
                                        "Word '{}' not found in dictionary",
                                        s
                                    ))
                                })?;

                            debug!("  Argument {}: String '{}' -> position {} (will create DictionaryRef)", i, s, position);

                            // Mark this operand position for dictionary fixup
                            dictionary_fixups.push((operands.len(), s.clone()));

                            // Use placeholder that will be patched in Phase 3
                            Operand::LargeConstant(placeholder_word())
                        }
                        crate::grue_compiler::ir::IrValue::Integer(i) => {
                            if *i >= 0 && *i <= 255 {
                                Operand::SmallConstant(*i as u8)
                            } else {
                                Operand::LargeConstant(*i as u16)
                            }
                        }
                        crate::grue_compiler::ir::IrValue::Boolean(b) => {
                            Operand::SmallConstant(if *b { 1 } else { 0 })
                        }
                        crate::grue_compiler::ir::IrValue::Null => Operand::SmallConstant(0),
                        crate::grue_compiler::ir::IrValue::Object(object_name) => {
                            // Convert object name to runtime object number
                            if let Some(&runtime_number) = self.object_numbers.get(object_name) {
                                Operand::LargeConstant(runtime_number)
                            } else {
                                return Err(CompilerError::CodeGenError(format!(
                                    "Object '{}' not found in runtime mapping for function argument",
                                    object_name
                                )));
                            }
                        }
                        crate::grue_compiler::ir::IrValue::RuntimeParameter(param) => {
                            // Runtime grammar parameter like $noun - needs to be resolved from parse buffer
                            if param == "noun" {
                                // For noun parameter, read word 1 from parse buffer and resolve to object ID
                                // This generates runtime code that:
                                // 1. Loads word 1 dictionary address from parse buffer offset 2
                                // 2. Calls object lookup to convert dict addr to object ID

                                // Load word 1 dictionary address from parse buffer (noun is typically word 1)
                                self.emit_instruction_typed(
                                    Opcode::Op2(Op2::Loadw), // loadw: load word from array
                                    &[
                                        Operand::Variable(PARSE_BUFFER_GLOBAL), // Parse buffer address
                                        Operand::SmallConstant(3), // Offset 3 = word 1 dict addr (noun) - CORRECT: Fixed parse buffer offset
                                    ],
                                    Some(2), // Store word 1 dict addr in local variable 2 (expected by lookup function)
                                    None,
                                )?;

                                // Generate object lookup code to convert dictionary address to object ID
                                // This uses the existing object lookup infrastructure
                                self.generate_object_lookup_from_noun()?;

                                // The object ID is now in variable 3, so return that as the operand
                                Operand::Variable(3)
                            } else {
                                return Err(CompilerError::CodeGenError(format!(
                                    "UNIMPLEMENTED: Runtime parameter '{}' resolution not yet implemented. Currently only 'noun' parameter is supported.",
                                    param
                                )));
                            }
                        }
                        crate::grue_compiler::ir::IrValue::StringRef(_) => {
                            return Err(CompilerError::CodeGenError(format!(
                                "Grammar handler arguments cannot use StringRef - use String instead"
                            )));
                        }
                        crate::grue_compiler::ir::IrValue::StringAddress(addr) => {
                            // StringAddress holds a packed string address (i16)
                            // Treat as integer value for Z-Machine operations
                            if *addr >= 0 && *addr <= 255 {
                                Operand::SmallConstant(*addr as u8)
                            } else {
                                Operand::LargeConstant(*addr as u16)
                            }
                        }
                    };
                    operands.push(arg_operand);
                }

                let layout = self.emit_instruction_typed(
                    Opcode::OpVar(OpVar::CallVs), // call_vs: call routine with arguments, returns value (VAR:0)
                    &operands,
                    Some(0), // Store result on stack
                    None,    // No branch
                )?;

                // FIXED: Use layout.operand_location instead of hardcoded offset calculation
                // This was previously using self.code_address - 2 which caused placeholder resolution failures
                if let Some(mut current_location) = layout.operand_location {
                    // Create UnresolvedReference for function address (first operand)
                    self.reference_context
                        .unresolved_refs
                        .push(UnresolvedReference {
                            reference_type: LegacyReferenceType::FunctionCall,
                            location: current_location, // Correct operand location from emit_instruction
                            target_id: *func_id,
                            is_packed_address: true,
                            offset_size: 2,
                            location_space: MemorySpace::Code,
                        });

                    // Function address is always LargeConstant (2 bytes)
                    current_location += 2;

                    // Create UnresolvedReferences for dictionary fixups
                    for (operand_index, word) in dictionary_fixups {
                        // Calculate location of this operand
                        // Skip operands before this one
                        for i in 1..operand_index {
                            match &operands[i] {
                                Operand::SmallConstant(_) => current_location += 1,
                                Operand::LargeConstant(_) => current_location += 2,
                                Operand::Variable(_) => current_location += 1,
                                Operand::Constant(_) => current_location += 2, // Constants are 2 bytes
                            }
                        }

                        // Find word position in dictionary
                        let word_lower = word.to_lowercase();
                        let position = self
                            .dictionary_words
                            .iter()
                            .position(|w| w == &word_lower)
                            .unwrap() as u32; // Safe because we already validated above

                        debug!(
                            "Creating DictionaryRef for grammar argument '{}' at location 0x{:04x} (position {})",
                            word, current_location, position
                        );

                        // Create UnresolvedReference for this dictionary word
                        self.reference_context
                            .unresolved_refs
                            .push(UnresolvedReference {
                                reference_type: LegacyReferenceType::DictionaryRef {
                                    word: word.clone(),
                                },
                                location: current_location,
                                target_id: position,
                                is_packed_address: false,
                                offset_size: 2,
                                location_space: MemorySpace::Code,
                            });

                        // This operand is LargeConstant (2 bytes)
                        current_location += 2;
                    }
                } else {
                    panic!("BUG: emit_instruction didn't return operand_location for placeholder");
                }

                // Jump back to main loop to read new input - default handler has successfully executed
                debug!(
                    "🔀 JUMP_MAIN_LOOP: Jumping back to main loop start (label {}) after default handler",
                    main_loop_jump_id
                );
                self.emit_jump_to_main_loop(main_loop_jump_id)?;
            }
        } else if let Some(pattern) = noun_pattern {
            // No default pattern, but we have a noun pattern - call it with object ID 0
            if let crate::grue_compiler::ir::IrHandler::FunctionCall(func_id, _args) =
                &pattern.handler
            {
                debug!(
"Generating noun pattern call with object ID 0 for verb '{}' using function ID {}",
verb, func_id
);

                // Call noun handler with object ID 0 (fallback case)
                self.emit_handler_call(*func_id, vec![Operand::SmallConstant(0)], true, 0)?;

                // Jump back to main loop to read new input - noun handler (with ID 0) has successfully executed
                debug!(
                    "🔀 JUMP_MAIN_LOOP: Jumping back to main loop start (label {}) after noun handler (ID 0)",
                    main_loop_jump_id
                );
                self.emit_jump_to_main_loop(main_loop_jump_id)?;
            }
        }

        Ok(())
    }

    /// Helper: Emit a jump instruction back to the main loop
    ///
    /// This is used after pattern handlers execute to return control to the main input loop.
    /// Emits a jump instruction with placeholder that gets resolved to the main loop label.
    fn emit_jump_to_main_loop(&mut self, main_loop_label: u32) -> Result<(), CompilerError> {
        let layout = self.emit_instruction_typed(
            Opcode::Op1(Op1::Jump),
            &[Operand::LargeConstant(placeholder_word())],
            None,
            None,
        )?;

        if let Some(operand_location) = layout.operand_location {
            self.reference_context
                .unresolved_refs
                .push(UnresolvedReference {
                    reference_type: LegacyReferenceType::Jump,
                    location: operand_location,
                    target_id: main_loop_label,
                    is_packed_address: false,
                    offset_size: 2,
                    location_space: MemorySpace::Code,
                });
        } else {
            panic!("BUG: emit_instruction didn't return operand_location for jump");
        }

        Ok(())
    }

    /// Helper: Emit a call to a pattern handler function
    ///
    /// This emits a call_vs instruction to invoke a grammar pattern handler function.
    /// The function address is a placeholder that gets resolved later.
    /// The return value is stored in the specified variable (typically 0 or 1, both ignored).
    ///
    /// If `use_dispatch` is true, resolves the function ID through the dispatch table
    /// for polymorphic method dispatch.
    fn emit_handler_call(
        &mut self,
        func_id: IrId,
        operands: Vec<Operand>,
        use_dispatch: bool,
        store_var: u8,
    ) -> Result<(), CompilerError> {
        // Build call operands: function address placeholder + arguments
        let mut call_operands = vec![Operand::LargeConstant(placeholder_word())];
        call_operands.extend(operands);

        // Emit function call
        let layout = self.emit_instruction_typed(
            Opcode::OpVar(OpVar::CallVs),
            &call_operands,
            Some(store_var), // Store return value (typically ignored)
            None,
        )?;

        // Resolve function ID (with dispatch if needed)
        let target_func_id = if use_dispatch {
            self.get_function_id_with_dispatch(func_id)
        } else {
            func_id
        };

        // Register function reference for later resolution
        if let Some(operand_location) = layout.operand_location {
            self.reference_context
                .unresolved_refs
                .push(UnresolvedReference {
                    reference_type: LegacyReferenceType::FunctionCall,
                    location: operand_location,
                    target_id: target_func_id,
                    is_packed_address: true,
                    offset_size: 2,
                    location_space: MemorySpace::Code,
                });
        } else {
            panic!("BUG: emit_instruction didn't return operand_location for call_vs");
        }

        Ok(())
    }

    /// Generate code for single literal patterns (e.g., "around" in "look around")
    ///
    /// Filters patterns for single-word literals, checks word count == 2,
    /// compares word 1 with literal dictionary address, and calls handler if match.
    fn generate_literal_patterns(
        &mut self,
        verb: &str,
        patterns: &[crate::grue_compiler::ir::IrPattern],
        main_loop_jump_id: u32,
    ) -> Result<(), CompilerError> {
        debug!(
            "🔤 LITERAL_PATTERN_SEARCH: Starting search for literal patterns in verb '{}'",
            verb
        );
        let literal_patterns: Vec<_> = patterns
            .iter()
            .filter(|p| {
                p.pattern.len() == 1
                    && matches!(
                        p.pattern[0],
                        crate::grue_compiler::ir::IrPatternElement::Literal(_)
                    )
            })
            .collect();
        debug!(
            "🔤 LITERAL_PATTERN_FILTER: Found {} literal patterns",
            literal_patterns.len()
        );

        if !literal_patterns.is_empty() {
            debug!(
                "🔤 LITERAL_PATTERNS_FOUND: {} literal patterns in verb '{}'",
                literal_patterns.len(),
                verb
            );

            // For each literal pattern, generate matching code
            for literal_pattern in literal_patterns {
                if let crate::grue_compiler::ir::IrPatternElement::Literal(literal_word) =
                    &literal_pattern.pattern[0]
                {
                    debug!("🔤 Processing literal pattern: '{}'", literal_word);

                    // Create label for skipping this literal pattern if it doesn't match
                    let skip_literal_label = self.next_string_id;
                    self.next_string_id += 1;

                    // Check if word count is exactly 2 (verb + literal word)
                    debug!(
                        "🔍 LITERAL_WORDCOUNT_CHECK: Checking if word count == 2 for literal '{}'",
                        literal_word
                    );
                    let layout = self.emit_instruction_typed(
                        Opcode::Op2(Op2::Je),
                        &[
                            Operand::Variable(1),      // Word count
                            Operand::SmallConstant(2), // Must be exactly 2 words
                        ],
                        None,
                        Some(0xBFFF_u16 as i16), // Branch-on-FALSE: skip if not equal to 2
                    )?;

                    // Register branch to skip this literal pattern if word count != 2
                    if let Some(branch_location) = layout.branch_location {
                        self.reference_context
                            .unresolved_refs
                            .push(UnresolvedReference {
                                reference_type: LegacyReferenceType::Branch,
                                location: branch_location,
                                target_id: skip_literal_label,
                                is_packed_address: false,
                                offset_size: 2,
                                location_space: MemorySpace::Code,
                            });
                    } else {
                        panic!("BUG: emit_instruction didn't return branch_location for je instruction");
                    }

                    // Load word 1 from parse buffer (the literal word to match)
                    debug!(
                        "🔍 LITERAL_LOAD_WORD1: Loading word 1 dictionary address for literal '{}'",
                        literal_word
                    );
                    self.emit_instruction_typed(
                        Opcode::Op2(Op2::Loadw), // loadw: load word from array
                        &[
                            Operand::Variable(PARSE_BUFFER_GLOBAL), // Parse buffer address
                            Operand::SmallConstant(1),              // Offset 1 = word 1 dict addr
                        ],
                        Some(7), // Store in local variable 7 (temporary for literal matching)
                        None,
                    )?;
                    debug!("🔍 LITERAL_LOAD_WORD1_COMPLETE: Stored word 1 in Variable(7) for literal '{}'", literal_word);

                    // Compare word 1 with literal's dictionary address
                    debug!(
                        "🔍 LITERAL_COMPARE: Comparing word 1 with literal '{}'",
                        literal_word
                    );
                    debug!("🔍 LITERAL_COMPARE_SETUP: About to emit JE instruction at code_address=0x{:04x}", self.code_address);
                    debug!("🔍 LITERAL_COMPARE_DETAILS: Will compare Variable(7) [word 1] against literal '{}' dictionary address", literal_word);

                    // Find dictionary position for literal word
                    let word_lower = literal_word.to_lowercase();
                    let position = self
                        .dictionary_words
                        .iter()
                        .position(|w| w == &word_lower)
                        .ok_or_else(|| {
                            CompilerError::CodeGenError(format!(
                                "Literal word '{}' not found in dictionary",
                                literal_word
                            ))
                        })? as u32;

                    let layout = self.emit_instruction_typed(
                        Opcode::Op2(Op2::Je), // je: jump if equal
                        &[
                            Operand::Variable(7),                    // word 1 dict addr from parse buffer
                            Operand::LargeConstant(placeholder_word()), // literal dict addr (placeholder)
                        ],
                        None,
                        Some(0x4000_u16 as i16), // Branch-on-TRUE (match found)
                    )?;
                    debug!("🔍 LITERAL_COMPARE_EMITTED: JE instruction emitted for literal '{}' comparison", literal_word);

                    // Register dictionary reference for the literal word
                    if let Some(mut operand_location) = layout.operand_location {
                        operand_location += 1; // Skip first operand (Variable(7) = 1 byte)
                        debug!(
                            "📝 LITERAL_DICT_REF: Creating DictionaryRef at location 0x{:04x} for word '{}' (position {})",
                            operand_location, literal_word, position
                        );
                        self.reference_context
                            .unresolved_refs
                            .push(UnresolvedReference {
                                reference_type: LegacyReferenceType::DictionaryRef {
                                    word: literal_word.clone(),
                                },
                                location: operand_location,
                                target_id: position,
                                is_packed_address: false,
                                offset_size: 2,
                                location_space: MemorySpace::Code,
                            });
                    }

                    // Register branch to execute handler if match
                    let execute_literal_label = self.next_string_id;
                    self.next_string_id += 1;

                    if let Some(branch_location) = layout.branch_location {
                        self.reference_context
                            .unresolved_refs
                            .push(UnresolvedReference {
                                reference_type: LegacyReferenceType::Branch,
                                location: branch_location,
                                target_id: execute_literal_label,
                                is_packed_address: false,
                                offset_size: 2,
                                location_space: MemorySpace::Code,
                            });
                    } else {
                        panic!("BUG: emit_instruction didn't return branch_location for je instruction");
                    }

                    // Jump to skip this pattern (no match)
                    let layout = self.emit_instruction_typed(
                        Opcode::Op1(Op1::Jump),
                        &[Operand::LargeConstant(placeholder_word())],
                        None,
                        None,
                    )?;

                    if let Some(operand_location) = layout.operand_location {
                        self.reference_context
                            .unresolved_refs
                            .push(UnresolvedReference {
                                reference_type: LegacyReferenceType::Jump,
                                location: operand_location,
                                target_id: skip_literal_label,
                                is_packed_address: false,
                                offset_size: 2,
                                location_space: MemorySpace::Code,
                            });
                    } else {
                        panic!("BUG: emit_instruction didn't return operand_location for jump");
                    }

                    // Label for executing literal pattern handler
                    self.reference_context
                        .ir_id_to_address
                        .insert(execute_literal_label, self.code_address);

                    // Execute handler function
                    if let crate::grue_compiler::ir::IrHandler::FunctionCall(func_id, args) =
                        &literal_pattern.handler
                    {
                        debug!(
                            "🔤 LITERAL_CALL: Calling function {} for literal pattern '{}'",
                            func_id, literal_word
                        );

                        // Generate function call
                        // Convert arguments to operands (literal patterns typically have no args)
                        let mut arg_operands = Vec::new();
                        for arg_value in args.iter() {
                            let arg_operand = match arg_value {
                                IrValue::Integer(val) => Operand::SmallConstant(*val as u8),
                                IrValue::Boolean(true) => Operand::SmallConstant(1),
                                IrValue::Boolean(false) => Operand::SmallConstant(0),
                                IrValue::Null => Operand::SmallConstant(0),
                                IrValue::String(_) => Operand::SmallConstant(0), // String literal not supported as function arg
                                IrValue::StringRef(_) => Operand::SmallConstant(0), // String ref not supported as function arg
                                IrValue::StringAddress(_) => Operand::SmallConstant(0), // String address not supported as function arg
                                IrValue::Object(_) => Operand::SmallConstant(0), // Object ref not supported as function arg
                                IrValue::RuntimeParameter(_) => Operand::SmallConstant(0), // Runtime param not supported as function arg
                            };
                            arg_operands.push(arg_operand);
                        }

                        // Emit call to pattern handler function
                        self.emit_handler_call(*func_id, arg_operands, false, 1)?;

                        // Jump back to main loop after successfully executing literal pattern
                        debug!(
                            "🔀 JUMP_MAIN_LOOP: Jumping back to main loop start (label {}) after literal pattern handler",
                            main_loop_jump_id
                        );
                        self.emit_jump_to_main_loop(main_loop_jump_id)?;
                    }

                    // Define the skip_literal_label here for branches that skip this pattern
                    // This is reached by branches that don't match the word count or literal word
                    debug!("🏷️ LITERAL_LABEL_DEFINE: Defining skip_literal_label={} at address 0x{:04x}", skip_literal_label, self.code_address);

                    // Register label in ir_id_to_address for branch resolution (this is what the resolver looks for)
                    self.reference_context
                        .ir_id_to_address
                        .insert(skip_literal_label, self.code_address);
                }
            }
        }

        Ok(())
    }

    /// Generate code for literal+noun patterns (e.g., "at" + noun in "look at mailbox")
    ///
    /// Filters patterns for 2-element [Literal, Noun] patterns, checks word count >= 3,
    /// compares word 1 with literal dictionary address, extracts noun from word 2,
    /// converts noun dictionary address to object ID, and calls handler with object parameter.
    fn generate_literal_noun_patterns(
        &mut self,
        patterns: &[crate::grue_compiler::ir::IrPattern],
        main_loop_jump_id: u32,
    ) -> Result<(), CompilerError> {
        // LITERAL+NOUN CASE: Check for 2-element patterns like [Literal("at"), Noun]
        for pattern in patterns.iter() {
            if pattern.pattern.len() == 2 {
                if let (
                    crate::grue_compiler::ir::IrPatternElement::Literal(literal_word),
                    crate::grue_compiler::ir::IrPatternElement::Noun,
                ) = (&pattern.pattern[0], &pattern.pattern[1])
                {
                    debug!(
                        "🔤 LITERAL+NOUN_CHECK: Testing for literal '{}' + noun pattern",
                        literal_word
                    );

                    // Check if we have at least 3 words (verb + literal + noun)
                    let skip_insufficient_words = (83000 + (self.code_address * 19) % 9999) as u32;
                    let layout = self.emit_instruction_typed(
                        Opcode::Op2(Op2::Jl), // jl: jump if less than
                        &[
                            Operand::Variable(1),      // word count
                            Operand::SmallConstant(3), // compare with 3
                        ],
                        None,
                        Some(0xBFFF_u16 as i16), // Branch-on-TRUE (skip if word_count < 3)
                    )?;

                    // Register branch to skip this pattern if insufficient words
                    if let Some(branch_location) = layout.branch_location {
                        self.reference_context
                            .unresolved_refs
                            .push(UnresolvedReference {
                                reference_type: LegacyReferenceType::Branch,
                                location: branch_location,
                                target_id: skip_insufficient_words,
                                is_packed_address: false,
                                offset_size: 2,
                                location_space: MemorySpace::Code,
                            });
                    }

                    // Generate unique label IDs for this literal+noun pattern
                    let unique_seed = (self.code_address * 17) % 9999;
                    let skip_literal_noun_label = (82000 + unique_seed) as u32;

                    // Load word 1 (offset 3) from parse buffer - this should be the literal "at"
                    self.emit_instruction_typed(
                        Opcode::Op2(Op2::Loadw),
                        &[
                            Operand::Variable(PARSE_BUFFER_GLOBAL),
                            Operand::SmallConstant(3), // Word 1 dict addr at offset 3 (FIXED: was 4)
                        ],
                        Some(2), // Store in local variable 2 (variable 1 is used for word count)
                        None,
                    )?;

                    // Compare word 1 with literal dictionary address: je @2 dict_addr skip_label
                    let dict_ref_operand = Operand::LargeConstant(placeholder_word());
                    let layout = self.emit_instruction_typed(
                        Opcode::Op2(Op2::Je),
                        &[
                            Operand::Variable(2), // Word 1 from parse buffer (now in variable 2)
                            dict_ref_operand,     // Literal word dictionary address
                        ],
                        None,
                        Some(0x7FFF_u16 as i16), // Branch on FALSE (not equal) - skip this pattern if literal doesn't match
                    )?;

                    // Calculate dictionary reference location (second operand)
                    let dict_operand_location = if let Some(operand_base) = layout.operand_location
                    {
                        operand_base + 1 // Skip first operand to reach second operand
                    } else {
                        panic!("BUG: emit_instruction didn't return operand_location for je");
                    };

                    // Register dictionary reference for literal word
                    // Look up the word's position in the sorted dictionary
                    let word_position = self.dictionary_words.iter().position(|w| w == literal_word)
                        .unwrap_or_else(|| {
                            panic!("FATAL: Literal word '{}' not found in dictionary_words! Available words: {:?}",
                                   literal_word, self.dictionary_words);
                        });
                    debug!("🔍 DICT_REF_REGISTER: Registering dictionary reference for literal word '{}' at location 0x{:04x}, position={}", literal_word, dict_operand_location, word_position);
                    self.reference_context
                        .unresolved_refs
                        .push(UnresolvedReference {
                            reference_type: LegacyReferenceType::DictionaryRef {
                                word: literal_word.clone(),
                            },
                            location: dict_operand_location,
                            target_id: word_position as u32, // Use actual dictionary position
                            is_packed_address: false,
                            offset_size: 2,
                            location_space: MemorySpace::Code,
                        });

                    // Register branch to skip this pattern if literal doesn't match
                    if let Some(branch_location) = layout.branch_location {
                        self.reference_context
                            .unresolved_refs
                            .push(UnresolvedReference {
                                reference_type: LegacyReferenceType::Branch,
                                location: branch_location,
                                target_id: skip_literal_noun_label,
                                is_packed_address: false,
                                offset_size: 2,
                                location_space: MemorySpace::Code,
                            });
                    }

                    // MATCHED: Load word 2 (offset 3) as noun parameter and execute handler
                    if let crate::grue_compiler::ir::IrHandler::FunctionCall(func_id, args) =
                        &pattern.handler
                    {
                        debug!(
                            "🔤 LITERAL+NOUN_EXECUTE: Calling function {} for '{}' + noun",
                            func_id, literal_word
                        );

                        // Load word 2 (offset 3) from parse buffer - this is the noun
                        self.emit_instruction_typed(
                            Opcode::Op2(Op2::Loadw),
                            &[
                                Operand::Variable(PARSE_BUFFER_GLOBAL),
                                Operand::SmallConstant(5), // Word 2 dict addr at offset 5
                            ],
                            Some(7), // Store in local variable 7 (temporary for grammar operations)
                            None,
                        )?;

                        // Build argument operands for function call
                        let mut arg_operands = Vec::new();

                        // LITERAL+NOUN PARAMETER RESOLUTION: Process RuntimeParameter arguments for patterns like "at" + noun
                        // This fixes the "look at mailbox" crash by properly converting dictionary addresses to object IDs
                        for arg_value in args.iter() {
                            let arg_operand = match arg_value {
                                crate::grue_compiler::ir::IrValue::RuntimeParameter(param)
                                    if param == "2" =>
                                {
                                    // FIXED: Proper $2 parameter resolution for literal+noun patterns
                                    // 1. Copy noun dictionary address from variable 7 to variable 2 (lookup function expects input in var 2)
                                    // 2. Convert dictionary address to object ID using standard lookup mechanism
                                    // 3. Return object ID from variable 3 (lookup function outputs object ID to var 3)

                                    self.emit_instruction_typed(
                                        Opcode::Op2(Op2::Store),
                                        &[
                                            Operand::SmallConstant(2), // Store to variable 2 (lookup input)
                                            Operand::Variable(7), // From variable 7 (noun dict addr loaded from parse buffer)
                                        ],
                                        None,
                                        None,
                                    )?;

                                    // Generate object lookup code to convert dictionary address to object ID
                                    // This ensures functions receive object IDs (e.g., 10 for mailbox) instead of dict addresses (e.g., 2678)
                                    self.generate_object_lookup_from_noun()?;

                                    // The object ID is now in variable 3, so return that as the operand for function call
                                    Operand::Variable(3)
                                }
                                crate::grue_compiler::ir::IrValue::Integer(n) => {
                                    if *n >= 0 && *n <= 255 {
                                        Operand::SmallConstant(*n as u8)
                                    } else {
                                        Operand::LargeConstant(*n as u16)
                                    }
                                }
                                _ => {
                                    debug!(
                                        "🔤 LITERAL+NOUN_WARNING: Unexpected argument type: {:?}",
                                        arg_value
                                    );
                                    Operand::SmallConstant(0)
                                }
                            };
                            arg_operands.push(arg_operand);
                        }

                        // Emit call to pattern handler function
                        self.emit_handler_call(*func_id, arg_operands, false, 0)?;

                        // Jump back to main loop after successfully executing literal+noun pattern
                        debug!(
                            "🔀 JUMP_MAIN_LOOP: Jumping back to main loop start (label {}) after literal+noun pattern handler",
                            main_loop_jump_id
                        );
                        self.emit_jump_to_main_loop(main_loop_jump_id)?;
                    }

                    // Register the skip_literal_noun_label at current address
                    self.record_final_address(skip_literal_noun_label, self.code_address);

                    // Register the skip_insufficient_words label at the same address (both conditions skip this pattern)
                    self.record_final_address(skip_insufficient_words, self.code_address);
                }
            }
        }

        Ok(())
    }

    /// Generate code for verb+noun patterns (e.g., "take <object>", "drop <object>")
    ///
    /// Handles simple verb+noun patterns where word count >= 2.
    /// Loads noun from parse buffer, converts to object ID, calls handler with object parameter.
    fn generate_verb_noun_patterns(
        &mut self,
        verb: &str,
        pattern: &crate::grue_compiler::ir::IrPattern,
        main_loop_jump_id: u32,
    ) -> Result<(), CompilerError> {
        if let crate::grue_compiler::ir::IrHandler::FunctionCall(func_id, args) =
            &pattern.handler
        {
            debug!(
" NOUN_CASE_EXECUTING: Generating noun pattern call to function ID {} for verb '{}' at 0x{:04x}",
func_id, verb, self.code_address
);

            log::debug!(
                "📍 PATTERN_HANDLER: '{}' noun pattern at 0x{:04x} with {} arguments",
                verb,
                self.code_address,
                args.len()
            );

            // POLYMORPHIC DISPATCH FIX: Process RuntimeParameter arguments properly
            // Instead of hardcoding object lookup, use the existing RuntimeParameter resolution system

            // Build argument operands for function call
            let mut arg_operands = Vec::new();

            // Convert IrValue arguments to operands
            for arg_value in args.iter() {
                let arg_operand = match arg_value {
                    crate::grue_compiler::ir::IrValue::RuntimeParameter(param) => {
                        // Runtime grammar parameter - needs to be resolved from parse buffer
                        if param == "noun" {
                            // Semantic "noun" parameter - load word 1 from parse buffer
                            self.emit_instruction_typed(
                                Opcode::Op2(Op2::Loadw), // loadw: load word from array
                                &[
                                    Operand::Variable(PARSE_BUFFER_GLOBAL), // Parse buffer address
                                    Operand::SmallConstant(3), // Offset 3 = word 1 dict addr (noun) - CORRECT: Fixed parse buffer offset
                                ],
                                Some(2), // Store word 1 dict addr in local variable 2 (expected by lookup function)
                                None,
                            )?;

                            // Generate object lookup code to convert dictionary address to object ID
                            self.generate_object_lookup_from_noun()?;

                            // The object ID is now in variable 3, so return that as the operand
                            Operand::Variable(3)
                        } else if let Ok(word_position) = param.parse::<u8>() {
                            // Positional parameter like "2", "3", etc. - load word at specified position
                            if word_position >= 1 && word_position <= 15 {
                                // Load word N dictionary address from parse buffer offset N
                                self.emit_instruction_typed(
                                    Opcode::Op2(Op2::Loadw), // loadw: load word from array
                                    &[
                                        Operand::Variable(PARSE_BUFFER_GLOBAL), // Parse buffer address
                                        Operand::SmallConstant(word_position), // Offset N = word N dict addr
                                    ],
                                    Some(2), // Store word N dict addr in local variable 2 (expected by lookup function)
                                    None,
                                )?;

                                // Generate object lookup code to convert dictionary address to object ID
                                self.generate_object_lookup_from_noun()?;

                                // The object ID is now in variable 3, so return that as the operand
                                Operand::Variable(3)
                            } else {
                                return Err(CompilerError::CodeGenError(format!(
                                    "Invalid word position '{}' in RuntimeParameter. Must be between 1 and 15.",
                                    word_position
                                )));
                            }
                        } else {
                            return Err(CompilerError::CodeGenError(format!(
                                "UNIMPLEMENTED: Runtime parameter '{}' resolution not yet implemented. Supported parameters: 'noun' or numeric positions (1-15).",
                                param
                            )));
                        }
                    }
                    crate::grue_compiler::ir::IrValue::Object(object_name) => {
                        // Compile-time object reference - resolve to object number
                        if let Some(&obj_number) = self.object_numbers.get(object_name) {
                            Operand::SmallConstant(obj_number as u8)
                        } else {
                            return Err(CompilerError::CodeGenError(format!(
                                "Object '{}' not found in object_numbers mapping for function argument",
                                object_name
                            )));
                        }
                    }
                    _ => {
                        return Err(CompilerError::CodeGenError(format!(
                            "UNIMPLEMENTED: Unsupported IrValue type in grammar handler arguments: {:?}",
                            arg_value
                        )));
                    }
                };
                arg_operands.push(arg_operand);
            }

            // Call handler with polymorphic dispatch
            self.emit_handler_call(*func_id, arg_operands, true, 0)?;

            // Jump back to main loop to read new input - handler has successfully executed
            debug!(
                "🔀 JUMP_MAIN_LOOP: Jumping back to main loop start (label {}) after successful handler",
                main_loop_jump_id
            );
            self.emit_jump_to_main_loop(main_loop_jump_id)?;
        }

        Ok(())
    }
}
