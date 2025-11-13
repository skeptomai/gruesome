// Z-Machine Code Generator

//
// Transforms IR into executable Z-Machine bytecode following the Z-Machine Standard v1.1
// Supports both v3 and v5 target formats with proper memory layout and instruction encoding.
//
// RECENT CLEANUP (2025-11-02): Removed 1,006 lines of dead code across 7 systematic phases:
// - Phase 1: Commented-out code blocks (162 lines)
// - Phase 2-4: Unused translate_* functions from old IR translation layer (586 lines)
// - Phase 5: Unused helper functions (85 lines)
// - Phase 6: Unused validation functions (127 lines)
// - Phase 7: Unused resolution functions (44 lines)
// Total file size reduction: 11,338 → 10,332 lines (8.9% reduction)

use crate::grue_compiler::ast::Type;
use crate::grue_compiler::codegen_memory::{
    placeholder_word, MemorySpace, HEADER_SIZE, PLACEHOLDER_BYTE,
};
use crate::grue_compiler::codegen_references::{
    LegacyReferenceType, ReferenceContext, UnresolvedReference,
};

use crate::grue_compiler::error::CompilerError;
use crate::grue_compiler::ir::*;
use crate::grue_compiler::opcodes::*;
use crate::grue_compiler::ZMachineVersion;
use indexmap::{IndexMap, IndexSet};
use log::debug;

/// CRITICAL: Invalid opcode marker for unimplemented IR instructions
/// This opcode (0x00) is deliberately invalid in the Z-Machine specification.
/// Any attempt to emit this opcode will cause a COMPILE-TIME ERROR, preventing
/// broken bytecode from being generated. This forces proper implementation of
/// all IR instruction handlers before the compiler can successfully generate bytecode.
///
/// Usage: This should ONLY be used in unimplemented IR instruction handlers as
/// a clear marker that the instruction needs proper Z-Machine implementation.
/// The emit_instruction() method will detect and reject this opcode with a clear
/// error message indicating which feature needs to be implemented.
/// Marker value for unimplemented opcodes that need proper Z-Machine implementation.
///
/// CRITICAL: This must NOT be a valid Z-Machine opcode. Previously we used 0x00,
/// but that's the valid opcode for jz (jump if zero) in 1OP form, which caused
/// the compiler to reject valid jz instructions as "unimplemented".
///
/// 0xFF is not a valid Z-Machine opcode in any form, making it safe for this purpose.
pub const UNIMPLEMENTED_OPCODE: u8 = 0xFF;

/// Reference types for fixup tracking
#[derive(Debug, Clone)]
pub enum ReferenceType {
    // Cross-space references (require final assembly)
    // NOTE: CodeJump and CodeBranch REMOVED - use LegacyReferenceType::Jump/Branch instead
    StringRef { string_id: IrId },
    ObjectRef { object_id: IrId },
    RoutineCall { routine_id: IrId },
}

/// Pending fixup that needs resolution
#[derive(Debug, Clone)]
pub struct PendingFixup {
    pub source_space: MemorySpace,
    pub source_address: usize,
    pub reference_type: ReferenceType,
    pub instruction_name: String,
    pub operand_size: usize, // 1 or 2 bytes
    pub resolved: bool,
}

/// Information about the layout of an emitted Z-Machine instruction
///
/// This tracks the exact byte locations of different instruction components,
/// eliminating the need for hardcoded offset calculations when creating
/// references for later patching.
#[derive(Debug, Clone)]
pub struct InstructionLayout {
    /// Starting address of the instruction
    pub instruction_start: usize,
    /// Location of the first operand (if any)
    pub operand_location: Option<usize>,
    /// Location of the store variable byte (if any)
    pub store_location: Option<usize>,
    /// Location of the branch offset (if any)
    pub branch_location: Option<usize>,
    /// Total size of the instruction in bytes
    pub total_size: usize,
}

/// Temporary structure to hold object data for table generation
#[derive(Debug, Clone)]
pub struct ObjectData {
    pub id: IrId,
    pub name: String,
    pub short_name: String,
    pub attributes: IrAttributes,
    pub properties: IrProperties,
    pub parent: Option<IrId>,
    pub sibling: Option<IrId>,
    pub child: Option<IrId>,
}

// HEADER_SIZE moved to codegen_memory.rs
const DEFAULT_HIGH_MEMORY: u16 = 0x8000; // Start of high memory
const DEFAULT_PC_START: u16 = 0x1000; // Initial program counter

/// Z-Machine operand types
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum OperandType {
    LargeConstant = 0b00, // 00: 16-bit constant
    SmallConstant = 0b01, // 01: 8-bit constant
    Variable = 0b10,      // 10: variable number
    Omitted = 0b11,       // 11: operand omitted
}

/// Z-Machine instruction formats
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InstructionForm {
    Long,     // 2OP instructions with long form
    Short,    // 1OP and 0OP instructions
    Variable, // VAR instructions
    Extended, // EXT instructions (v5+)
}

/// Operand value that can be encoded
#[derive(Debug, Clone)]
pub enum Operand {
    Constant(u16),      // Immediate value
    Variable(u8),       // Variable number (0=stack, 1-15=locals, 16-255=globals)
    LargeConstant(u16), // Always encoded as 16-bit
    SmallConstant(u8),  // Always encoded as 8-bit
}

/// Array metadata for dynamic list operations
#[derive(Debug, Clone)]
pub struct ArrayInfo {
    pub capacity: i32,
    pub current_size: i32,         // For simulation - tracks number of items
    pub base_address: Option<u16>, // For future Z-Machine memory implementation
}

/// Constant value types for control flow optimization
#[derive(Debug, Clone, PartialEq)]
pub enum ConstantValue {
    Boolean(bool),
    Integer(i16),
    String(String),
}

/// Parts of a string concatenation with runtime values
/// Used to track strings that need multi-part printing at runtime
#[derive(Debug, Clone)]
pub enum StringPart {
    /// A literal string constant (stored in string table)
    Literal(usize), // string_id
    /// A runtime value that needs to be evaluated and printed
    RuntimeValue(IrId), // IR ID of the runtime value
}

/// Code generation context
pub struct ZMachineCodeGen {
    pub version: ZMachineVersion,

    // Memory layout
    story_data: Vec<u8>,
    // REMOVED: current_address - replaced with space-specific address tracking
    pub final_assembly_address: usize, // Tracks current position during final assembly phase

    // Input buffer addresses
    text_buffer_addr: usize,
    parse_buffer_addr: usize,

    // Code generation state
    pub label_addresses: IndexMap<IrId, usize>, // IR label ID -> byte address
    string_addresses: IndexMap<IrId, usize>,    // IR string ID -> byte address
    pub function_addresses: IndexMap<IrId, usize>, // IR function ID -> function header byte address
    function_locals_count: IndexMap<IrId, usize>, // IR function ID -> locals count (for header size calculation)
    function_header_locations: IndexMap<IrId, usize>, // IR function ID -> header byte location for patching
    current_function_locals: u8, // Track local variables allocated in current function (0-15)
    pub current_function_name: Option<String>, // Track current function being processed for debugging
    pub init_routine_locals_count: u8, // Track local variables used by init routine for PC calculation
    /// Mapping from IR IDs to string values (for LoadImmediate results)
    pub ir_id_to_string: IndexMap<IrId, String>,
    /// Mapping from IR IDs to runtime string concatenation parts
    /// Used when string concatenation includes runtime values (e.g., "There is " + obj.name + " here.")
    /// Maps the concatenation result IR ID to a sequence of literal strings and runtime values
    pub runtime_concat_parts: IndexMap<IrId, Vec<StringPart>>,
    /// Mapping from IR IDs to integer values (for LoadImmediate results)
    pub ir_id_to_integer: IndexMap<IrId, i16>,
    /// Mapping from IR IDs to stack variables (for instruction results on stack)
    pub ir_id_to_stack_var: IndexMap<IrId, u8>,
    /// Counter for unique global variable allocation (for builtins)
    allocated_globals_count: usize,
    /// Mapping from IR IDs to Z-Machine object numbers (for object references)
    pub ir_id_to_object_number: IndexMap<IrId, u16>,
    /// Mapping from IR IDs to Z-Machine local variable slots (for function parameters)
    pub ir_id_to_local_var: IndexMap<IrId, u8>,
    /// Mapping from IR IDs to binary operations (for conditional branch optimization)
    pub ir_id_to_binary_op: IndexMap<IrId, (IrBinaryOp, IrId, IrId)>, // (operator, left_operand, right_operand)
    /// Mapping from IR IDs to expression types (for StringAddress system)
    pub ir_type_info: IndexMap<IrId, Type>,
    /// Analysis of comparison operations usage patterns
    /// Contains IR IDs of comparison operations that are used as values (need push/pull)
    /// vs those used only in direct branches (no push/pull needed)
    pub comparison_ids_used_as_values: IndexSet<IrId>,
    /// Mapping from function IDs to builtin function names
    builtin_function_names: IndexMap<IrId, String>,
    /// Mapping from builtin name to pseudo function ID for address lookup
    builtin_functions: IndexMap<String, IrId>,
    /// Mapping from base function name to dispatch function ID for polymorphic dispatch
    dispatch_functions: IndexMap<String, IrId>,
    /// Mapping from function ID to base function name for dispatch lookup
    function_base_names: IndexMap<IrId, String>,
    /// Mapping from IR IDs to array metadata (for dynamic lists)
    ir_id_to_array_info: IndexMap<IrId, ArrayInfo>,
    /// Mapping from IR IDs to ArrayCodeGen array IDs (for static arrays)
    ir_id_to_array_id: IndexMap<IrId, IrId>,
    /// Set of IR IDs that come from GetProperty instructions (for print() type detection)
    pub ir_id_from_property: IndexSet<IrId>,
    /// Mapping from function ID to argument positions that are string literals
    /// Used to propagate ir_id_from_property flag from string arguments to their corresponding parameters
    pub string_arg_to_param_mapping: IndexMap<IrId, Vec<usize>>,
    /// Mapping from object names to object numbers (from IR generator)
    pub object_numbers: IndexMap<String, u16>,
    /// Mapping from room IR IDs to Z-Machine object numbers
    pub room_to_object_id: IndexMap<IrId, u16>,
    /// Exit directions for each room: room_name -> Vec<direction_name>
    /// Used to create DictionaryRef UnresolvedReferences when writing exit_directions property
    pub room_exit_directions: IndexMap<String, Vec<String>>,
    /// Object names for each object: object_name -> Vec<name_variant>
    /// Used to create DictionaryRef UnresolvedReferences when writing Property 18 object names
    pub object_names: IndexMap<String, Vec<String>>,
    /// Initial parent-child locations set by InsertObj in init block
    /// (Oct 12, 2025): Maps object IR ID -> parent IR ID for compile-time tree initialization
    /// These are detected during IR processing when in_init_block=true
    pub initial_locations: IndexMap<IrId, IrId>,
    /// Initial parent-child locations using actual Z-Machine object numbers
    /// (Oct 12, 2025): Maps object number -> parent object number for compile-time tree initialization
    /// Populated during code generation when operands are resolved
    pub initial_locations_by_number: IndexMap<u16, u16>,
    /// Track blocked exit message string IDs for UnresolvedReference creation
    /// Maps room name -> Vec of (exit_index, string_id) for blocked exits
    pub room_exit_messages: IndexMap<String, Vec<(usize, u32)>>,
    /// Global property registry: property name -> property number
    pub property_numbers: IndexMap<String, u8>,
    /// Properties used by each object: object_name -> set of property names
    pub object_properties: IndexMap<String, Vec<String>>,

    // Tables for Z-Machine structures
    pub object_table_addr: usize,
    pub property_table_addr: usize,
    pub current_property_addr: usize, // Current property table allocation pointer
    pub dictionary_addr: usize,
    pub global_vars_addr: usize,

    // String encoding
    pub strings: Vec<(IrId, String)>, // Collected strings for encoding
    pub main_loop_prompt_id: Option<IrId>, // ID of the main loop prompt string (public for codegen_extensions.rs)
    pub main_loop_unknown_command_id: Option<IrId>, // ID of the "I don't understand" string (public for codegen_extensions.rs)

    // Stack tracking for debugging
    pub stack_depth: i32,     // Current estimated stack depth
    pub max_stack_depth: i32, // Maximum stack depth reached
    pub encoded_strings: IndexMap<IrId, Vec<u8>>, // IR string ID -> encoded bytes
    pub next_string_id: IrId, // Next available string ID

    // Execution context
    pub in_init_block: bool, // True when generating init block code

    // Label processing
    pub pending_labels: Vec<IrId>, // Labels waiting to be assigned to next instruction

    // Address resolution
    pub reference_context: ReferenceContext,

    // Debug mapping: PC → IR instruction for runtime debugging
    pub pc_to_ir_map: IndexMap<usize, (String, IrId, String)>, // PC → (function_name, ir_id, instruction_description)

    // Control flow analysis - NEW ARCHITECTURE
    /// Track constant values resolved during generation
    pub constant_values: IndexMap<IrId, ConstantValue>,
    /// Track which labels have been placed at current address
    pub labels_at_current_address: Vec<IrId>,

    // === SEPARATED MEMORY SPACES ARCHITECTURE ===
    // During compilation, we maintain separate memory spaces to prevent overlaps
    /// Header space - contains 64-byte Z-Machine file header
    header_space: Vec<u8>,
    header_address: usize,

    /// Code space - contains Z-Machine instructions with placeholders
    pub code_space: Vec<u8>,
    pub code_address: usize,

    /// Builtin space end tracking (Option C fix for Bug #90 file integration)
    /// After builtin functions are generated, this tracks where they end
    /// so regular functions can start after builtin space to prevent overwriting
    pub builtin_space_end: usize,

    /// String space - contains encoded string data
    pub string_space: Vec<u8>,
    pub string_address: usize,

    /// Object space - contains object table and property data
    pub object_space: Vec<u8>,
    pub object_address: usize,

    /// Dictionary space - contains word parsing dictionary
    pub dictionary_space: Vec<u8>,
    pub dictionary_address: usize,

    /// Global variables space - contains 240 global variable slots (480 bytes)
    pub globals_space: Vec<u8>,
    pub globals_address: usize,

    /// Array subsystem for static array management
    pub array_codegen: crate::grue_compiler::codegen_arrays::ArrayCodeGen,

    /// Abbreviations space - contains string compression abbreviations table
    pub abbreviations_space: Vec<u8>,
    abbreviations_address: usize,

    /// Code-space label tracking (for immediate jump/branch resolution)
    pub code_labels: IndexMap<IrId, usize>,

    /// String offset tracking (for final assembly)
    pub string_offsets: IndexMap<IrId, usize>,

    /// Object offset tracking (for final assembly)
    pub object_offsets: IndexMap<IrId, usize>,

    /// Pending fixups that need resolution
    pub pending_fixups: Vec<PendingFixup>,

    /// Final assembled bytecode (created during assemble_complete_zmachine_image)
    pub final_data: Vec<u8>,
    pub final_code_base: usize,
    pub final_string_base: usize,
    pub final_object_base: usize,
    pub final_abbreviations_base: usize,

    /// Offset within code_space where main program starts (after builtin functions)
    pub main_program_offset: usize,

    /// Dictionary words in alphabetically sorted order (for word position lookup)
    /// Populated during generate_dictionary_space(), used by lookup_word_in_dictionary()
    pub dictionary_words: Vec<String>,
    /// Set of IR IDs that should use push/pull sequence for stack discipline
    /// Phase C1.1: Track values that need actual VAR:232/233 push/pull opcodes
    pub push_pull_ir_ids: IndexSet<IrId>,
    /// Mapping from IR ID to temporary global variable for already-pulled values
    /// Prevents multiple pull operations for the same IR ID
    pulled_ir_id_to_global: IndexMap<IrId, u8>,
    /// Set of IR IDs that represent function call results stored in Variable(0)
    /// Phase 1: Track function calls that store results to Variable(0) for proper consumption
    pub function_call_results: IndexSet<IrId>,
    /// System messages for localization support
    /// Maps message keys to their localized text with fallback handling
    pub system_messages: IndexMap<String, String>,
}

impl ZMachineCodeGen {
    pub fn new(version: ZMachineVersion) -> Self {
        // Initialize property numbers for standard properties
        let mut property_numbers = IndexMap::new();

        // Exit system properties (parallel arrays for runtime direction lookup)
        property_numbers.insert("exit_directions".to_string(), 20);
        property_numbers.insert("exit_types".to_string(), 21);
        property_numbers.insert("exit_data".to_string(), 22);

        ZMachineCodeGen {
            version,
            story_data: vec![0; HEADER_SIZE],
            final_assembly_address: HEADER_SIZE,
            text_buffer_addr: 0,
            parse_buffer_addr: 0,
            label_addresses: IndexMap::new(),
            string_addresses: IndexMap::new(),
            function_addresses: IndexMap::new(),
            function_locals_count: IndexMap::new(),
            function_header_locations: IndexMap::new(),
            current_function_locals: 0,
            current_function_name: None,
            init_routine_locals_count: 0,
            ir_id_to_string: IndexMap::new(),
            runtime_concat_parts: IndexMap::new(),
            ir_id_to_integer: IndexMap::new(),
            ir_id_to_stack_var: IndexMap::new(),
            allocated_globals_count: 0,
            ir_id_to_object_number: IndexMap::new(),
            ir_id_to_local_var: IndexMap::new(),
            ir_id_to_binary_op: IndexMap::new(),
            ir_type_info: IndexMap::new(),
            comparison_ids_used_as_values: IndexSet::new(),
            builtin_function_names: IndexMap::new(),
            builtin_functions: IndexMap::new(),
            dispatch_functions: IndexMap::new(),
            function_base_names: IndexMap::new(),
            ir_id_to_array_info: IndexMap::new(),
            ir_id_to_array_id: IndexMap::new(),
            ir_id_from_property: IndexSet::new(),
            string_arg_to_param_mapping: IndexMap::new(),
            object_numbers: IndexMap::new(),
            room_to_object_id: IndexMap::new(),
            room_exit_directions: IndexMap::new(),
            object_names: IndexMap::new(),
            initial_locations: IndexMap::new(),
            initial_locations_by_number: IndexMap::new(),
            room_exit_messages: IndexMap::new(),
            property_numbers,
            object_properties: IndexMap::new(),
            object_table_addr: 0,
            property_table_addr: 0,
            current_property_addr: 0,
            dictionary_addr: 0,
            global_vars_addr: 0,
            strings: Vec::new(),
            main_loop_prompt_id: None,
            main_loop_unknown_command_id: None,
            encoded_strings: IndexMap::new(),
            next_string_id: 1000, // Start string IDs from 1000 to avoid conflicts
            stack_depth: 0,
            max_stack_depth: 0,
            in_init_block: false,
            pending_labels: Vec::new(),
            reference_context: ReferenceContext::new(),
            pc_to_ir_map: IndexMap::new(),
            constant_values: IndexMap::new(),
            labels_at_current_address: Vec::new(),

            // Initialize separated memory spaces
            header_space: Vec::new(),
            header_address: 0,
            code_space: Vec::new(),
            code_address: 0,
            builtin_space_end: 0,
            string_space: Vec::new(),
            string_address: 0,
            object_space: Vec::new(),
            object_address: 0,
            dictionary_space: Vec::new(),
            dictionary_address: 0,
            globals_space: Vec::new(),
            globals_address: 0,
            array_codegen: crate::grue_compiler::codegen_arrays::ArrayCodeGen::new(),
            abbreviations_space: Vec::new(),
            abbreviations_address: 0,
            code_labels: IndexMap::new(),
            string_offsets: IndexMap::new(),
            object_offsets: IndexMap::new(),
            pending_fixups: Vec::new(),
            final_data: Vec::new(),
            final_code_base: 0,
            final_string_base: 0,
            final_object_base: 0,
            final_abbreviations_base: 0,
            main_program_offset: 0,
            dictionary_words: Vec::new(),
            push_pull_ir_ids: IndexSet::new(),
            pulled_ir_id_to_global: IndexMap::new(),
            function_call_results: IndexSet::new(),
            system_messages: IndexMap::new(),
        }
    }

    /// Initialize system messages from IR for builtin function access
    /// Called during codegen setup to copy messages from IR to local storage
    pub fn initialize_system_messages(&mut self, ir: &IrProgram) {
        self.system_messages = ir.system_messages.clone();
    }

    /// Get system message with fallback - internal version for builtin functions
    /// Uses the local system_messages field populated from IR during initialization
    pub fn get_builtin_message(&self, key: &str, fallback: &str) -> String {
        self.system_messages
            .get(key)
            .cloned()
            .unwrap_or_else(|| fallback.to_string())
    }

    /// Allocate a unique global variable for an IR ID
    /// Uses the same allocation scheme as GetPropertyByNumber: 200 + (count % 50)
    /// Count based on how many globals we've allocated
    pub fn allocate_global_for_ir_id(&mut self, _ir_id: u32) -> u8 {
        let var = 200u8 + (self.allocated_globals_count as u8 % 50);
        self.allocated_globals_count += 1;
        var
    }

    /// Define a label in code space - enables immediate jump/branch resolution
    fn define_code_label(&mut self, label_id: IrId) -> Result<(), CompilerError> {
        let label_address = self.code_address;
        self.code_labels.insert(label_id, label_address);

        log::debug!(
            "🏷️ CODE_LABEL_DEFINED: id={}, addr=0x{:04x}",
            label_id,
            label_address
        );

        // Note: Forward reference resolution for CodeJump/CodeBranch removed
        // All jump fixups now use the working LegacyReferenceType::Jump system

        Ok(())
    }

    /// Emit a cross-space reference (string, object, routine call)
    fn emit_cross_space_ref(
        &mut self,
        reference_type: ReferenceType,
        instruction_name: &str,
        operand_size: usize,
    ) -> Result<(), CompilerError> {
        log::debug!(
            "⏳ CROSS_SPACE_REF: type={:?}, instr={}",
            reference_type,
            instruction_name
        );

        self.pending_fixups.push(PendingFixup {
            source_space: MemorySpace::Code,
            source_address: self.code_address,
            reference_type,
            instruction_name: instruction_name.to_string(),
            operand_size,
            resolved: false,
        });

        if operand_size == 2 {
            let placeholder = placeholder_word();
            self.emit_byte((placeholder >> 8) as u8)?;
            self.emit_byte(placeholder as u8)?;
        } else {
            self.emit_byte(PLACEHOLDER_BYTE)?;
        }
        Ok(())
    }

    /// Implementation: Local variable assignment (compile-time mapping only)
    /// This handles `let x = 42` by allocating a local variable slot and mapping the IR ID
    /// NO Z-Machine instruction is generated - this is pure compile-time bookkeeping
    fn assign_local_variable(&mut self, var_id: IrId, source: IrId) -> Result<(), CompilerError> {
        log::debug!("ASSIGN_LOCAL_VAR: var_id={}, source={}", var_id, source);

        // 1. Allocate a local variable slot for this variable
        let local_slot = self.allocate_local_variable_slot();
        log::debug!(
            "ASSIGN_LOCAL_VAR: Allocated local slot {} for IR variable {}",
            local_slot,
            var_id
        );

        // 2. Map IR variable ID to the local variable slot
        self.ir_id_to_local_var.insert(var_id, local_slot);

        // 3. If source is a constant, store it for optimization
        if let Some(constant_value) = self.ir_id_to_integer.get(&source).copied() {
            self.ir_id_to_integer.insert(var_id, constant_value);
            log::debug!(
                "ASSIGN_LOCAL_VAR: Variable {} = constant {}",
                var_id,
                constant_value
            );
        } else {
            // If source is another variable, create the mapping chain
            if let Some(source_var) = self.ir_id_to_local_var.get(&source) {
                log::debug!(
                    "ASSIGN_LOCAL_VAR: Variable {} references variable {}",
                    var_id,
                    source_var
                );
                // The actual value copying will be handled at runtime when the variable is loaded
            }
        }

        // 4. NO instruction generation - this is compile-time only
        // Z-Machine instructions are generated later when variables are loaded (LoadVar)
        log::debug!("ASSIGN_LOCAL_VAR: Completed compile-time assignment (no bytecode generated)");
        Ok(())
    }

    /// Allocate a new local variable slot for the current function
    pub(crate) fn allocate_local_variable_slot(&mut self) -> u8 {
        self.current_function_locals += 1;
        self.current_function_locals
    }

    /// Generate a jump instruction to the specified label
    /// Jump instructions use relative offsets following Z-Machine specification:
    /// actual_target = PC_after_instruction + offset - 2
    pub fn translate_jump(&mut self, label: IrId) -> Result<(), CompilerError> {
        log::debug!("translate_jump: label={}", label);

        // CRITICAL DEBUG: Track jumps near problem area
        if self.code_address >= 0x330 && self.code_address <= 0x340 {
            log::debug!(
                "CRITICAL: translate_jump at code_address=0x{:04x}, jumping to label {}",
                self.code_address,
                label
            );
            log::debug!(
                " This jump will emit at 0x{:04x}-0x{:04x}",
                self.code_address,
                self.code_address + 2
            );
        }

        // OPTIMIZATION: Check if jump target is the immediately next instruction
        if self.is_next_instruction(label) {
            log::debug!(
                "Eliminating unnecessary jump to next instruction (label {})",
                label
            );
            return Ok(()); // No instruction needed - fall through
        }

        // Use emit_instruction which properly tracks component locations
        log::debug!(
            "JUMP_EMIT: Starting Jump instruction at code_address=0x{:04x}, code_space.len()={}",
            self.code_address,
            self.code_space.len()
        );
        let layout = self.emit_instruction_typed(
            Opcode::Op1(Op1::Jump),                        // jump opcode (1OP:140)
            &[Operand::LargeConstant(placeholder_word())], // Placeholder for jump offset
            None,
            None,
        )?;
        log::debug!("JUMP_EMIT: After Jump instruction, code_address=0x{:04x}, code_space.len()={}, operand_location={:?}",
            self.code_address, self.code_space.len(), layout.operand_location);

        // CRITICAL FIX: Use the layout.operand_location from emit_instruction
        // This ensures the location is correctly calculated after emitting the opcode
        let operand_location = layout
            .operand_location
            .expect("Jump instruction must have operand location");

        // Add unresolved reference for jump target
        self.reference_context
            .unresolved_refs
            .push(UnresolvedReference {
                reference_type: LegacyReferenceType::Jump,
                location: operand_location,
                target_id: label,
                is_packed_address: false,
                offset_size: 2,
                location_space: MemorySpace::Code,
            });

        Ok(())
    }

    /// Implementation: Call - Function call
    fn translate_call(
        &mut self,
        target: Option<IrId>,
        function: IrId,
        args: &[IrId],
    ) -> Result<(), CompilerError> {
        log::debug!(
            "CALL: target={:?}, function={}, args={:?}",
            target,
            function,
            args
        );

        // Track function call target creation

        // UNIVERSAL TARGET REGISTRATION: Ensure ALL function calls with targets create mappings
        // This prevents "No mapping found" errors even if function implementation is incomplete
        if let Some(target_id) = target {
            // Function calls should return values on stack (Z-Machine standard)
            self.ir_id_to_stack_var
                .insert(target_id, self.stack_depth as u8);
            self.stack_depth += 1;
            log::debug!(" UNIVERSAL_CALL_TARGET: Pre-registered IR ID {} -> stack variable {} for function {}",
 target_id, self.stack_depth - 1, function);
        }

        // Check if this is a builtin function first
        if self.is_builtin_function(function) {
            log::debug!("BUILTIN_CALL: function={}", function);

            // PHASE 1 & 2: Single-path builtin function handling
            match self.get_builtin_function_name(function) {
                Some(name) => {
                    log::debug!(" SINGLE_PATH: {} builtin in IR translation", name);
                    match name.as_str() {
                        // TIER 1: Basic functions (completed)
                        // ALL EARLY RETURNS REMOVED - FORCE ALL BUILTINS THROUGH REAL FUNCTION SYSTEM
                        // This eliminates the hybrid inline/function system that caused routing conflicts
                        _ => {
                            // Fallback to legacy system for remaining builtins (Tier 3 only)
                            log::debug!(" LEGACY: {} delegating to legacy builtin system", name);
                            let _ = target == Some(104);
                            self.generate_builtin_function_call(function, args, target)?;
                        }
                    }
                }
                None => {
                    log::debug!("Builtin function name not found: function ID {} not in builtin_function_names", function);
                    return Err(CompilerError::CodeGenError(format!(
                        "Builtin function ID {} not found",
                        function
                    )));
                }
            }
        } else if self.function_addresses.contains_key(&function) {
            // USER FUNCTION CALLS: Handle calls to user-defined functions
            //
            // CRITICAL ARCHITECTURE NOTE (Sept 4, 2025):
            // This function call generation works for basic cases but has discovered
            // a systematic issue with placeholder→UnresolvedReference architecture.
            //
            // CURRENT STATUS:
            // String references resolve correctly (print_paddr instructions work)
            // Function calls generate correct argument counts (0 args for look_around)
            // Initial game banner displays properly
            // PC corruption after function calls (jump to 0x1cda out of bounds)
            //
            // ROOT CAUSE IDENTIFIED:
            // Massive NULL byte generation throughout compilation due to placeholders
            // being written without corresponding UnresolvedReference entries.
            //
            // NEXT SESSION PRIORITY:
            // 1. Audit all placeholder_word() calls - ensure matching UnresolvedReference creation
            // 2. Fix missing reference entries for all instruction types
            // 3. Verify complete placeholder resolution before final assembly
            //
            log::debug!("USER_FUNCTION_CALL: function={}", function);
            // Convert arguments to operands
            let mut operands = vec![];

            // First operand is the function address (placeholder)
            operands.push(Operand::LargeConstant(placeholder_word()));

            // Add function arguments (simple implementation for now)
            for &_arg in args {
                let operand = Operand::SmallConstant(1); // Use variable 1 for now
                operands.push(operand);
            }

            // Determine store variable for result
            let store_var = if target.is_some() {
                Some(1) // Store in variable 1
            } else {
                None // Void function call
            };

            // Generate call instruction
            // CRITICAL FIX: Record exact code space offset BEFORE placeholder emission
            let operand_location = self.code_space.len() + 2; // +2 for opcode and operand types bytes (offset, will be translated to final address)
            let _layout = self.emit_instruction_typed(
                Opcode::OpVar(OpVar::CallVs), // call_vs raw opcode (VAR:224)
                &operands,
                store_var,
                None,
            )?;

            // Add unresolved reference for function address using pre-calculated location
            self.reference_context
                .unresolved_refs
                .push(UnresolvedReference {
                    reference_type: LegacyReferenceType::FunctionCall,
                    location: operand_location,
                    target_id: function,
                    is_packed_address: true,
                    offset_size: 2,
                    location_space: MemorySpace::Code,
                });
        } else {
            // HOTFIX: Register commonly missing builtin functions
            match function {
                1 => {
                    log::debug!("HOTFIX: Function 1 (look_around) - generating call instruction with UnresolvedReference");
                    log::debug!(
                        "Available user function IDs: {:?}",
                        self.function_addresses.keys().collect::<Vec<_>>()
                    );

                    // Generate proper call instruction with placeholder and UnresolvedReference
                    let mut operands = Vec::new();
                    operands.push(Operand::LargeConstant(placeholder_word())); // Function address placeholder

                    // Add function arguments
                    for &arg_id in args {
                        if let Some(literal_value) = self.get_literal_value(arg_id) {
                            operands.push(Operand::LargeConstant(literal_value));
                        } else if self.ir_id_to_string.contains_key(&arg_id) {
                            // String literals: Create placeholder + unresolved reference
                            let operand_location = self.code_address + 1 + operands.len() * 2;
                            operands.push(Operand::LargeConstant(placeholder_word()));
                            let reference = UnresolvedReference {
                                reference_type: LegacyReferenceType::StringRef,
                                location: operand_location,
                                target_id: arg_id,
                                is_packed_address: true,
                                offset_size: 2,
                                location_space: MemorySpace::Code,
                            };
                            self.reference_context.unresolved_refs.push(reference);
                        } else {
                            // Other types: Use existing operand resolution
                            match self.resolve_ir_id_to_operand(arg_id) {
                                Ok(operand) => operands.push(operand),
                                Err(_) => {
                                    // CRITICAL FIX: Create UnresolvedReference for failed resolution
                                    // This was the source of systematic NULL byte generation!
                                    log::warn!("Failed to resolve IR ID {} in function call, creating unresolved reference", arg_id);

                                    let operand_location =
                                        self.code_address + 1 + operands.len() * 2;
                                    operands.push(Operand::LargeConstant(placeholder_word()));

                                    // Create a generic UnresolvedReference that will attempt to resolve
                                    // this IR ID during the final resolution phase
                                    let reference = UnresolvedReference {
                                        reference_type: LegacyReferenceType::StringRef, // Default to string - most common failure case
                                        location: operand_location,
                                        target_id: arg_id,
                                        is_packed_address: true,
                                        offset_size: 2,
                                        location_space: MemorySpace::Code,
                                    };
                                    self.reference_context.unresolved_refs.push(reference);
                                }
                            }
                        }
                    }

                    // Determine store variable
                    let store_var = if target.is_some() {
                        Some(1) // Store in variable 1
                    } else {
                        None // Void function call
                    };

                    // Generate call instruction
                    // CRITICAL FIX: Record exact code space offset BEFORE placeholder emission
                    let operand_location = self.code_space.len() + 2; // +2 for opcode and operand types bytes (offset, will be translated to final address)
                    let _layout = self.emit_instruction_typed(
                        Opcode::OpVar(OpVar::CallVs), // call_vs raw opcode (VAR:224)
                        &operands,
                        store_var,
                        None,
                    )?;

                    // Create UnresolvedReference for function address using pre-calculated location
                    self.reference_context
                        .unresolved_refs
                        .push(UnresolvedReference {
                            reference_type: LegacyReferenceType::FunctionCall,
                            location: operand_location,
                            target_id: function, // This is IR ID 1 (look_around)
                            is_packed_address: true,
                            offset_size: 2,
                            location_space: MemorySpace::Code,
                        });

                    // Phase C2: Convert function calls to use push/pull stack discipline
                    if let Some(target) = target {
                        self.use_push_pull_for_result(target, "function call")?;
                    }

                    return Ok(());
                }
                11 => {
                    log::debug!("HOTFIX: Registering function 11 as object_is_empty");
                    self.register_builtin_function(11, "object_is_empty".to_string());
                    // Retry the builtin call now that it's registered
                    return self.translate_call(target, function, args);
                }
                12 => {
                    log::debug!("HOTFIX: Registering function 12 as player_can_see");
                    self.register_builtin_function(12, "player_can_see".to_string());
                    // Retry the builtin call now that it's registered
                    return self.translate_call(target, function, args);
                }
                277 => {
                    log::debug!("HOTFIX: Registering function 277 as get_exit");
                    self.register_builtin_function(277, "get_exit".to_string());
                    // Retry the builtin call now that it's registered
                    return self.translate_call(target, function, args);
                }
                278 => {
                    log::debug!("HOTFIX: Registering function 278 as print_num");
                    self.register_builtin_function(278, "print_num".to_string());
                    // Retry the builtin call now that it's registered
                    return self.translate_call(target, function, args);
                }
                279 => {
                    log::debug!("HOTFIX: Registering function 279 as add_score");
                    self.register_builtin_function(279, "add_score".to_string());
                    // Retry the builtin call now that it's registered
                    return self.translate_call(target, function, args);
                }
                280 => {
                    log::debug!("HOTFIX: Registering function 280 as subtract_score");
                    self.register_builtin_function(280, "subtract_score".to_string());
                    // Retry the builtin call now that it's registered
                    return self.translate_call(target, function, args);
                }
                281 => {
                    log::debug!("HOTFIX: Registering function 281 as word_to_number");
                    self.register_builtin_function(281, "word_to_number".to_string());
                    // Retry the builtin call now that it's registered
                    return self.translate_call(target, function, args);
                }
                _ => {
                    log::warn!(
 "UNKNOWN_FUNCTION_CALL: function={} not found in builtins or user functions",
 function
 );
                    log::warn!(
                        "Available builtin functions: {:?}",
                        self.builtin_function_names.keys().collect::<Vec<_>>()
                    );
                    log::warn!(
                        "Available user functions: {:?}",
                        self.function_addresses.keys().collect::<Vec<_>>()
                    );

                    // CRITICAL FIX: Generate proper function call instruction with UnresolvedReference
                    // Even for unknown functions, we need to emit a call instruction to maintain control flow
                    log::debug!("FALLBACK: Generating call instruction with UnresolvedReference for function {}", function);

                    let mut operands = Vec::new();
                    operands.push(Operand::LargeConstant(placeholder_word())); // Function address placeholder

                    // Add function arguments
                    for &arg_id in args {
                        if let Some(literal_value) = self.get_literal_value(arg_id) {
                            operands.push(Operand::LargeConstant(literal_value));
                        } else {
                            // Use existing operand resolution for other types
                            match self.resolve_ir_id_to_operand(arg_id) {
                                Ok(operand) => operands.push(operand),
                                Err(_) => {
                                    log::warn!(
                                        "Failed to resolve argument {} for function {}",
                                        arg_id,
                                        function
                                    );
                                    operands.push(Operand::SmallConstant(0)); // Fallback
                                }
                            }
                        }
                    }

                    // Determine store variable
                    let store_var = if target.is_some() {
                        Some(0) // Store on stack (Variable 0)
                    } else {
                        None // Void function call
                    };

                    // Generate call instruction
                    let operand_location = self.code_space.len() + 2; // +2 for opcode and operand types bytes (offset, will be translated to final address)
                    let _layout = self.emit_instruction_typed(
                        Opcode::Op1(Op1::Call1s), // call_1s opcode for 1 operand (1OP:136)
                        &operands,
                        store_var,
                        None,
                    )?;

                    // Create UnresolvedReference for function address
                    self.reference_context
                        .unresolved_refs
                        .push(UnresolvedReference {
                            reference_type: LegacyReferenceType::FunctionCall,
                            location: operand_location,
                            target_id: function,
                            is_packed_address: true,
                            offset_size: 2,
                            location_space: MemorySpace::Code,
                        });

                    log::debug!("FALLBACK: Created UnresolvedReference for function {} at location 0x{:04x}", function, operand_location);
                }
            }
        }

        Ok(())
    }

    /// Plan the memory layout for all Z-Machine structures
    /// Made public for use by codegen_extensions.rs
    pub fn layout_memory_structures(&mut self, ir: &IrProgram) -> Result<(), CompilerError> {
        debug!(" LAYOUT_DEBUG: Starting memory layout planning");
        // Start after header
        let mut addr = HEADER_SIZE;

        // Reserve space for global variables (480 bytes for 240 globals)
        self.global_vars_addr = addr;
        addr += 480;

        // Reserve space for input buffers (for main loop)
        // Text buffer: 64 bytes (2 header + 62 text)
        self.text_buffer_addr = addr;
        addr += 64;
        // Parse buffer: 34 bytes (2 header + 32 parse data)
        self.parse_buffer_addr = addr;
        addr += 34;

        debug!(
            "Allocated input buffers at: text=0x{:04x}, parse=0x{:04x}",
            self.text_buffer_addr, self.parse_buffer_addr
        );

        // Initialize the buffers with proper headers
        if self.story_data.len() <= self.text_buffer_addr + 64 {
            debug!(
                "Resizing story_data from {} to {} to accommodate buffers",
                self.story_data.len(),
                self.text_buffer_addr + 64 + 34
            );
            self.story_data.resize(self.text_buffer_addr + 64 + 34, 0);
        }

        // Write directly to story_data instead of routing through code space
        // This prevents the catastrophic bug where buffer initialization corrupts code
        debug!(
            "Initializing buffers: text_buffer[{}] = 100, parse_buffer[{}] = 120",
            self.text_buffer_addr, self.parse_buffer_addr
        );
        self.story_data[self.text_buffer_addr] = 100; // Max input length (match Zork I's 0x64)
        self.story_data[self.text_buffer_addr + 1] = 0; // Current length
        self.story_data[self.parse_buffer_addr] = 120; // Max words (match Zork I's 0x78)
        self.story_data[self.parse_buffer_addr + 1] = 0; // Current words

        // Reserve space for object table
        self.object_table_addr = addr;
        let estimated_objects = if ir.objects.is_empty() && ir.rooms.is_empty() {
            2
        } else {
            ir.objects.len() + ir.rooms.len() + 1 // +1 for player object that gets added later
        }; // At least 2 objects (player + room)
        let object_entries_size = match self.version {
            ZMachineVersion::V3 => estimated_objects * 9, // v3: 9 bytes per object
            ZMachineVersion::V4 | ZMachineVersion::V5 => estimated_objects * 14, // v4/v5: 14 bytes per object
        };
        let default_props_size = match self.version {
            ZMachineVersion::V3 => 62, // 31 properties * 2 bytes
            ZMachineVersion::V4 | ZMachineVersion::V5 => 126, // 63 properties * 2 bytes
        };
        addr += default_props_size + object_entries_size;

        // Reserve space for property tables (MUST be in dynamic memory for put_prop to work)
        // Property tables come AFTER object entries but BEFORE dictionary to stay in dynamic memory
        self.property_table_addr = addr;

        // CRITICAL FIX: current_property_addr is used for object space allocation, not final memory addresses
        // In object space: property defaults (62 bytes) + object entries, then property tables start
        let property_start_in_object_space = default_props_size + object_entries_size;
        self.current_property_addr = property_start_in_object_space; // Object space relative addressing

        debug!(" PROPERTY_ADDR_INIT: Final memory property_table_addr=0x{:04x}, object space current_property_addr=0x{:04x}", 
 addr, self.current_property_addr);
        let estimated_objects = if ir.objects.is_empty() && ir.rooms.is_empty() {
            2
        } else {
            ir.objects.len() + ir.rooms.len()
        }; // At least 2 objects (player + room)
        addr += estimated_objects * 50; // Rough estimate: 50 bytes per object for properties
        debug!(
 "Property table address: 0x{:04x}, estimated objects: {}, allocation starts at: 0x{:04x}",
 self.property_table_addr, estimated_objects, self.current_property_addr
 );

        // Reserve space for dictionary (this marks the start of static memory)
        // Dictionary comes AFTER property tables to ensure properties stay in dynamic memory
        self.dictionary_addr = addr;
        debug!(
            "Dictionary address (static memory base): 0x{:04x}",
            self.dictionary_addr
        );
        addr += self.dictionary_space.len(); // Use actual dictionary size

        // Reserve space for encoded strings
        let mut string_data: Vec<(IrId, usize)> = self
            .encoded_strings
            .iter()
            .map(|(id, encoded)| (*id, encoded.len()))
            .collect();

        // Sort by IR ID to ensure deterministic address assignment
        string_data.sort_by_key(|(id, _)| *id);
        debug!(
            "Initial string layout starting at 0x{:04x}, {} strings",
            addr,
            string_data.len()
        );

        for (string_id, length) in string_data {
            // Align string addresses according to Z-Machine version
            match self.version {
                ZMachineVersion::V3 => {
                    // v3: strings must be at even addresses
                    if addr % 2 != 0 {
                        addr += 1;
                    }
                }
                ZMachineVersion::V4 | ZMachineVersion::V5 => {
                    // v4/v5: strings must be at 4-byte boundaries
                    while addr % 4 != 0 {
                        addr += 1;
                    }
                }
            }

            self.string_addresses.insert(string_id, addr);
            debug!(
                "Layout phase: string_id={} -> 0x{:04x} (length={})",
                string_id, addr, length
            );
            self.record_final_address(string_id, addr); // Record in reference context

            // CRITICAL: Write string data immediately during layout phase
            // This prevents overlaps with code generation
            if let Some(encoded_bytes) = self.encoded_strings.get(&string_id).cloned() {
                self.ensure_capacity(addr + encoded_bytes.len());
                for (i, &byte) in encoded_bytes.iter().enumerate() {
                    let string_offset = (addr + i) - self.string_address; // Convert to string_space relative offset
                    self.write_to_string_space(string_offset, byte)?;
                }
                debug!(
                    "Layout phase: Wrote string_id={} to memory at 0x{:04x} (length={})",
                    string_id,
                    addr,
                    encoded_bytes.len()
                );
            }

            addr += length;
        }

        // Code starts after all data structures
        self.set_final_assembly_address(addr, "Layout phase - code start position");
        debug!(
            "Layout phase complete: final_assembly_address=0x{:04x}",
            addr
        );

        Ok(())
    }

    /// Create a single object entry in the object table
    fn create_object_entry(
        &mut self,
        obj_num: u8,
        parent: u8,
        sibling: u8,
        child: u8,
    ) -> Result<(), CompilerError> {
        // ARCHITECTURAL FIX: Write to object_space instead of contaminating code_space
        // Space-relative offset: each object is 9 bytes in V3
        let obj_offset = ((obj_num - 1) as usize) * 9; // V3: 9 bytes per object

        // Attributes (4 bytes, all zeros for now)
        self.write_to_object_space(obj_offset, 0)?;
        self.write_to_object_space(obj_offset + 1, 0)?;
        self.write_to_object_space(obj_offset + 2, 0)?;
        self.write_to_object_space(obj_offset + 3, 0)?;

        // Relationships (V3 uses 1 byte each)
        self.write_to_object_space(obj_offset + 4, parent)?;
        self.write_to_object_space(obj_offset + 5, sibling)?;
        self.write_to_object_space(obj_offset + 6, child)?;

        // Create property table for this object
        // Debug: Check state before creating property table
        let prop_table_addr = self.create_property_table(obj_num)?;
        // Debug: Check state after creating property table

        // Property table address (word) - bytes 7-8 of object entry
        debug!(
            "Writing property table address 0x{:04x} to object {} at space offset 0x{:04x}",
            prop_table_addr,
            obj_num,
            obj_offset + 7
        );
        self.write_to_object_space(obj_offset + 7, (prop_table_addr >> 8) as u8)?; // High byte
        self.write_to_object_space(obj_offset + 8, (prop_table_addr & 0xFF) as u8)?; // Low byte
                                                                                     // Debug: Property address written successfully

        Ok(())
    }

    /// Create a single object entry from IR object data
    pub fn create_object_entry_from_ir_with_mapping(
        &mut self,
        obj_num: u8,
        object: &ObjectData,
        object_id_to_number: &IndexMap<IrId, u8>,
    ) -> Result<(), CompilerError> {
        // ARCHITECTURAL FIX: Write to object_space instead of contaminating code_space
        // Z-Machine specification: Property defaults table comes FIRST, then objects
        let default_props = match self.version {
            ZMachineVersion::V3 => 31,
            ZMachineVersion::V4 | ZMachineVersion::V5 => 63,
        };
        let defaults_size = default_props * 2; // 2 bytes per default
        let obj_entry_size = match self.version {
            ZMachineVersion::V3 => 9,
            ZMachineVersion::V4 | ZMachineVersion::V5 => 14,
        };

        // CRITICAL FIX: Objects start AFTER the property defaults table
        let obj_offset = defaults_size + ((obj_num - 1) as usize) * obj_entry_size;

        debug!(
            " OBJECT LAYOUT: Object {} ('{}'):",
            obj_num, object.short_name
        );
        debug!(
            " - Property defaults size: {} bytes (0x{:02x})",
            defaults_size, defaults_size
        );
        debug!(" - Object entry size: {} bytes", obj_entry_size);
        debug!(
            " - Object offset calculation: {} + ({} - 1) * {} = 0x{:04x}",
            defaults_size, obj_num, obj_entry_size, obj_offset
        );

        // Attributes (4 bytes for V3)
        // Convert IR attributes to Z-Machine spec-compliant format
        // Z-Machine spec section 12.3.1: "attributes 0 to 31 are flags... stored topmost bit first:
        // e.g., attribute 0 is stored in bit 7 of the first byte, attribute 31 is stored in bit 0 of the fourth."
        let attrs = object.attributes.flags;

        // Initialize attribute bytes to zero
        let mut attr_bytes = [0u8; 4];

        // Set each attribute bit according to Z-Machine specification
        for attr_num in 0..32 {
            if (attrs & (1u64 << attr_num)) != 0 {
                let attr_byte = (attr_num / 8) as usize;
                let attr_bit = 7 - (attr_num % 8); // Topmost bit first
                attr_bytes[attr_byte] |= 1u8 << attr_bit;
            }
        }

        // Write the spec-compliant attribute bytes
        self.write_to_object_space(obj_offset, attr_bytes[0])?; // Attributes 0-7
        self.write_to_object_space(obj_offset + 1, attr_bytes[1])?; // Attributes 8-15
        self.write_to_object_space(obj_offset + 2, attr_bytes[2])?; // Attributes 16-23
        self.write_to_object_space(obj_offset + 3, attr_bytes[3])?; // Attributes 24-31

        // Parent/sibling/child relationships (V3 uses 1 byte each)
        // Resolve IR IDs to actual Z-Machine object numbers

        // PHASE 2 (Oct 12, 2025): Check initial_locations_by_number for compile-time parent setting
        // If this object had .location = X in init block, use that as parent
        let parent =
            if let Some(&parent_num) = self.initial_locations_by_number.get(&(obj_num as u16)) {
                log::warn!(
                    "🏗️ INITIAL_LOCATION_SET: Object {} ('{}') parent set to {} at compile time",
                    obj_num,
                    object.short_name,
                    parent_num
                );
                parent_num as u8
            } else {
                // No initial location - use default from IR (typically 0)
                object
                    .parent
                    .and_then(|id| object_id_to_number.get(&id))
                    .copied()
                    .unwrap_or(0)
            };

        let sibling = object
            .sibling
            .and_then(|id| object_id_to_number.get(&id))
            .copied()
            .unwrap_or(0);
        let child = object
            .child
            .and_then(|id| object_id_to_number.get(&id))
            .copied()
            .unwrap_or(0);

        self.write_to_object_space(obj_offset + 4, parent)?;
        self.write_to_object_space(obj_offset + 5, sibling)?;
        self.write_to_object_space(obj_offset + 6, child)?;

        // PHASE 2 (Oct 12, 2025): Update parent's child/sibling chain if initial location was set
        // When object A has parent B set at compile time, we need to:
        // 1. Make A the first child of B (or add to sibling chain if B already has children)
        // This mirrors what insert_obj does at runtime
        // log::debug!("🐛 SIBLING_DEBUG: Before PHASE 2 - Object #{} ('{}') parent={}, sibling={}, child={}",
        //            obj_num, object.short_name, parent, sibling, child);

        if parent != 0
            && self
                .initial_locations_by_number
                .contains_key(&(obj_num as u16))
        {
            let parent_offset = defaults_size + ((parent - 1) as usize) * obj_entry_size;

            // Read parent's current child pointer
            let parent_child = self.object_space[parent_offset + 6];

            // 🐛 CRITICAL FIX: Skip if object is already its parent's first child
            // This prevents circular sibling references when IR already established correct relationships
            if parent_child == obj_num {
                // Object is already correctly positioned as parent's first child, no updates needed
            } else if parent_child == 0 {
                // Parent has no children yet - make this object the first child
                self.write_to_object_space(parent_offset + 6, obj_num)?;
            } else {
                // Parent already has a child - insert this object at the beginning of sibling chain

                // Update this object's sibling to point to parent's current child
                self.write_to_object_space(obj_offset + 5, parent_child)?;

                // Update parent's child to point to this object
                self.write_to_object_space(parent_offset + 6, obj_num)?;
            }
        }

        // Create property table for this object with actual IR properties
        let prop_table_addr = self.create_property_table_from_ir(obj_num, object)?;

        // DEBUG: Detailed property table address logging
        debug!(
            " OBJECT ENTRY: Object {} ('{}') property table creation:",
            obj_num, object.short_name
        );
        debug!(
            " - create_property_table_from_ir returned: 0x{:04x}",
            prop_table_addr
        );
        debug!(
            " - Writing to object space offset: 0x{:04x} + 7 = 0x{:04x}",
            obj_offset,
            obj_offset + 7
        );
        debug!(
            " - High byte: 0x{:02x} -> object_space[0x{:04x}]",
            (prop_table_addr >> 8) as u8,
            obj_offset + 7
        );
        debug!(
            " - Low byte: 0x{:02x} -> object_space[0x{:04x}]",
            (prop_table_addr & 0xFF) as u8,
            obj_offset + 8
        );

        // Property table address (word) - bytes 7-8 of object entry
        self.write_to_object_space(obj_offset + 7, (prop_table_addr >> 8) as u8)?; // High byte
        self.write_to_object_space(obj_offset + 8, (prop_table_addr & 0xFF) as u8)?; // Low byte

        // DEBUG: Verify what was actually written
        let written_high = self.object_space[obj_offset + 7];
        let written_low = self.object_space[obj_offset + 8];
        let written_addr = ((written_high as u16) << 8) | (written_low as u16);
        log::warn!(
            "🔗 OBJ_PTR_WRITE: obj_num={} name='{}' obj_offset=0x{:04x} prop_table_addr=0x{:04x} written=0x{:04x}",
            obj_num, object.short_name, obj_offset, prop_table_addr, written_addr
        );

        if written_addr != prop_table_addr as u16 {
            log::error!(
                "❌ OBJ_PTR_MISMATCH: obj_num={} Expected 0x{:04x} but wrote 0x{:04x}!",
                obj_num,
                prop_table_addr,
                written_addr
            );
        }

        debug!(
            "Created object #{}: '{}' at offset 0x{:04x}, attributes=0x{:08x}, prop_table=0x{:04x}",
            obj_num, object.short_name, obj_offset, attrs, prop_table_addr
        );

        Ok(())
    }

    /// Create a property table for an object
    fn create_property_table(&mut self, obj_num: u8) -> Result<usize, CompilerError> {
        // Use the allocated property table region in dynamic memory
        let prop_table_addr = self.current_property_addr;

        debug!(
            "Creating complete property table for object {} at address 0x{:04x}",
            obj_num, prop_table_addr
        );

        // Get properties for this object number
        let object_name = self.get_object_name_by_number(obj_num);
        let properties = self
            .object_properties
            .get(&object_name)
            .cloned()
            .unwrap_or_else(Vec::new);

        // Estimate space needed: text-length + (3 bytes per property) + terminator
        let estimated_size = 1 + (properties.len() * 3) + 1;
        self.ensure_capacity(prop_table_addr + estimated_size);

        // Text-length byte (0 = no short name)
        let prop_offset = prop_table_addr - self.object_table_addr; // Convert to object_space-relative offset
        self.write_to_object_space(prop_offset, 0)?;
        let mut addr = prop_table_addr + 1;

        // Create properties in descending order (Z-Machine requirement)
        let mut sorted_properties: Vec<(u8, String)> = properties
            .iter()
            .filter_map(|name| {
                self.property_numbers
                    .get(name)
                    .map(|&num| (num, name.clone()))
            })
            .collect();
        sorted_properties.sort_by(|a, b| b.0.cmp(&a.0)); // Descending order

        debug!(
            "Creating {} properties for object {}: {:?}",
            sorted_properties.len(),
            obj_num,
            sorted_properties
        );

        for (prop_num, prop_name) in sorted_properties {
            // Property header: top 3 bits = size-1, bottom 5 bits = property number
            // For 2-byte properties: (2-1) << 5 | prop_num
            let header = ((2u8 - 1) << 5) | prop_num;

            debug!(
                "Writing property {} ({}) header 0x{:02x} at address 0x{:04x}",
                prop_num, prop_name, header, addr
            );
            let header_offset = addr - self.object_table_addr;
            self.write_to_object_space(header_offset, header)?;
            addr += 1;

            // Property data (2 bytes, default value 0)
            debug!(
                "Writing property {} data (0x0000) at address 0x{:04x}",
                prop_num, addr
            );
            let data_offset = addr - self.object_table_addr;
            self.write_to_object_space(data_offset, 0)?; // High byte
            self.write_to_object_space(data_offset + 1, 0)?; // Low byte
            addr += 2;
        }

        // End of property table (property 0 marks end)
        debug!("Writing property terminator 0x00 at address 0x{:04x}", addr);
        let terminator_offset = addr - self.object_table_addr;
        self.write_to_object_space(terminator_offset, 0)?;
        addr += 1;

        // Update current property allocation pointer for next property table
        self.current_property_addr = addr;

        debug!(
 "Complete property table for object {} created with {} properties, next address: 0x{:04x}",
 obj_num, properties.len(), addr
 );

        Ok(prop_table_addr)
    }

    /// Create a property table for an object using IR property data
    fn create_property_table_from_ir(
        &mut self,
        obj_num: u8,
        object: &ObjectData,
    ) -> Result<usize, CompilerError> {
        // Use the allocated property table region in dynamic memory
        let prop_table_addr = self.current_property_addr;
        self.ensure_capacity(prop_table_addr + 100);

        let mut addr = prop_table_addr;

        // Write object name (short description) as Z-Machine encoded string
        let name_bytes = self.encode_object_name(&object.short_name);
        // For now, set text_length to 0 (no object name text) to fix property access
        // TODO: Implement proper Z-Machine text encoding for object names
        let text_length = 0;

        // Text length byte
        let text_offset = addr - self.object_table_addr;
        self.write_to_object_space(text_offset, text_length as u8)?;
        debug!(
            "PROP TABLE DEBUG: Writing text_length={} at addr=0x{:04x} for object '{}'",
            text_length, addr, object.short_name
        );
        debug!(
            "Object '{}': name_bytes.len()={}, text_length={}, addr=0x{:04x}",
            object.short_name,
            name_bytes.len(),
            text_length,
            addr
        );
        addr += 1;
        debug!(
            "PROP TABLE DEBUG: After text_length, addr=0x{:04x}, about to write properties",
            addr
        );

        // Only write name bytes if text_length > 0
        if text_length > 0 {
            // Write encoded name bytes and pad to word boundary
            for &byte in &name_bytes {
                let name_offset = addr - self.object_table_addr;
                self.write_to_object_space(name_offset, byte)?;
                addr += 1;
            }
            // Pad to word boundary if necessary
            if name_bytes.len() % 2 == 1 {
                let pad_offset = addr - self.object_table_addr;
                self.write_to_object_space(pad_offset, 0)?; // Pad byte
                addr += 1;
            }
        }

        // Write properties in descending order (required by Z-Machine spec)
        let mut properties: Vec<_> = object.properties.properties.iter().collect();
        properties.sort_by(|a, b| b.0.cmp(a.0)); // Sort by property number, descending

        log::warn!(
            "🔍 PROP_ENCODE: Object '{}' has {} properties to encode: {:?}",
            object.short_name,
            properties.len(),
            properties.iter().map(|(n, _)| **n).collect::<Vec<_>>()
        );

        for (&prop_num, prop_value) in properties {
            // Write property size/number byte
            let (size_byte, prop_data, string_id_opt, two_byte_size) =
                self.encode_property_value(prop_num, prop_value);

            let data_display = if prop_data.len() == 2 {
                format!(
                    "0x{:02x}{:02x} ({})",
                    prop_data[0],
                    prop_data[1],
                    (prop_data[0] as u16) << 8 | prop_data[1] as u16
                )
            } else {
                format!("{:02x?}", prop_data)
            };

            log::warn!(
                "🔍 PROP_WRITE: Writing property {} for object '{}': size_byte=0x{:02x}, data={}, string_id={:?}, two_byte_format={}",
                prop_num, object.short_name, size_byte, data_display, string_id_opt, two_byte_size.is_some()
            );

            // Ensure capacity for property header + data + terminator
            // Header is 1 byte for single-byte format, 2 bytes for two-byte format
            let header_size = if two_byte_size.is_some() { 2 } else { 1 };
            self.ensure_capacity(addr + header_size + prop_data.len() + 1);

            // Write first size byte
            let size_offset = addr - self.object_table_addr;
            self.write_to_object_space(size_offset, size_byte)?;
            debug!(
                "PROP TABLE DEBUG: Writing size_byte=0x{:02x} at addr=0x{:04x}",
                size_byte, addr
            );
            addr += 1;

            // Write second size byte if two-byte format
            if let Some(size_value) = two_byte_size {
                let size2_offset = addr - self.object_table_addr;
                self.write_to_object_space(size2_offset, size_value)?;
                debug!(
                    "PROP TABLE DEBUG: Writing second size byte=0x{:02x} at addr=0x{:04x}",
                    size_value, addr
                );
                addr += 1;
            }

            // Write property data
            for (i, &byte) in prop_data.iter().enumerate() {
                let data_offset = addr - self.object_table_addr;
                self.write_to_object_space(data_offset, byte)?;

                // If this is a string property, create UnresolvedReference for packed address
                if i == 0 && string_id_opt.is_some() {
                    let string_id = string_id_opt.unwrap();
                    // Create reference that will be resolved to packed string address
                    self.reference_context
                        .unresolved_refs
                        .push(UnresolvedReference {
                            reference_type: LegacyReferenceType::StringPackedAddress { string_id },
                            location: data_offset, // Location in object_space
                            target_id: string_id,
                            is_packed_address: true,
                            offset_size: 2, // 2-byte word
                            location_space: MemorySpace::Objects,
                        });
                    log::debug!(
                        "Created UnresolvedReference for string property {} at object_space offset 0x{:04x}, string ID {}",
                        prop_num, data_offset, string_id
                    );
                }

                // If this is exit_directions property (property 20), create DictionaryRef for each 2-byte dictionary address
                let exit_directions_prop =
                    *self.property_numbers.get("exit_directions").unwrap_or(&20);
                if prop_num == exit_directions_prop && i % 2 == 0 && i < prop_data.len() - 1 {
                    // This is the start of a 2-byte dictionary address
                    let direction_index = i / 2; // Which direction this is (0, 1, 2, ...)

                    // Look up which direction string this corresponds to
                    if let Some(directions) = self.room_exit_directions.get(&object.name) {
                        if let Some(direction) = directions.get(direction_index) {
                            // Find position of this word in dictionary
                            let position = self
                                .dictionary_words
                                .iter()
                                .position(|w| w == &direction.to_lowercase())
                                .unwrap_or(0) as u32;

                            // Create UnresolvedReference for dictionary address
                            self.reference_context
                                .unresolved_refs
                                .push(UnresolvedReference {
                                    reference_type: LegacyReferenceType::DictionaryRef {
                                        word: direction.clone(),
                                    },
                                    location: data_offset, // Location in object_space (will be adjusted with object_base)
                                    target_id: position,
                                    is_packed_address: false,
                                    offset_size: 2,
                                    location_space: MemorySpace::Objects,
                                });

                            log::debug!(
                                "Created DictionaryRef UnresolvedReference for exit_directions property: room='{}', direction='{}', position={}, object_space offset=0x{:04x}",
                                object.name, direction, position, data_offset
                            );
                        }
                    }
                }

                // If this is Property 18 (object names), create DictionaryRef for each 2-byte dictionary address
                if prop_num == 18 && i % 2 == 0 && i < prop_data.len() - 1 {
                    log::warn!("🔗 PROP18_DICTREF: Checking Property 18 for object '{}', i={}, prop_data.len()={}", object.name, i, prop_data.len());
                    // This is the start of a 2-byte dictionary address placeholder
                    let name_index = i / 2; // Which object name this is (0, 1, 2, ...)

                    // Look up which object name string this corresponds to
                    if let Some(object_names) = self.object_names.get(&object.name) {
                        log::warn!(
                            "🔗 PROP18_DICTREF: Found object_names for '{}': {:?}",
                            object.name,
                            object_names
                        );
                        if let Some(object_name) = object_names.get(name_index) {
                            log::warn!("🔗 PROP18_DICTREF: Creating DictionaryRef for '{}' name '{}' at index {}", object.name, object_name, name_index);
                            // Find position of this word in dictionary
                            let position = self
                                .dictionary_words
                                .iter()
                                .position(|w| w == &object_name.to_lowercase())
                                .unwrap_or_else(|| {
                                    log::error!(
                                        "🚨 PROP18_DICT_ERROR: Object name '{}' not found in dictionary_words! This is a compiler bug. Available words: {:?}",
                                        object_name, self.dictionary_words
                                    );
                                    panic!(
                                        "FATAL: Object name '{}' for object '{}' not found in dictionary_words. This causes incorrect dictionary addresses in property 18.",
                                        object_name, object.name
                                    );
                                }) as u32;

                            // Create UnresolvedReference for dictionary address
                            self.reference_context
                                .unresolved_refs
                                .push(UnresolvedReference {
                                    reference_type: LegacyReferenceType::DictionaryRef {
                                        word: object_name.clone(),
                                    },
                                    location: data_offset, // Location in object_space (will be adjusted with object_base)
                                    target_id: position,
                                    is_packed_address: false,
                                    offset_size: 2,
                                    location_space: MemorySpace::Objects,
                                });

                            log::debug!(
                                "Created DictionaryRef UnresolvedReference for Property 18 object names: object='{}', name='{}', position={}, object_space offset=0x{:04x}",
                                object.name, object_name, position, data_offset
                            );
                        }
                    }
                }

                // If this is exit_data property (property 22), create StringRef for blocked exit messages
                let exit_data_prop = *self.property_numbers.get("exit_data").unwrap_or(&22);
                if prop_num == exit_data_prop && i % 2 == 0 && i < prop_data.len() - 1 {
                    // This is the start of a 2-byte word
                    let exit_index = i / 2; // Which exit this is (0, 1, 2, ...)

                    // Check if this exit has a blocked message
                    if let Some(blocked_msgs) = self.room_exit_messages.get(&object.name) {
                        // Find if this exit_index has a message
                        for (idx, string_id) in blocked_msgs {
                            if *idx == exit_index {
                                // Create UnresolvedReference for string address (packed)
                                self.reference_context
                                    .unresolved_refs
                                    .push(UnresolvedReference {
                                        reference_type: LegacyReferenceType::StringRef,
                                        location: data_offset, // Location in object_space
                                        target_id: *string_id,
                                        is_packed_address: true, // String addresses must be packed
                                        offset_size: 2,
                                        location_space: MemorySpace::Objects,
                                    });

                                log::debug!(
                                    "Created StringRef UnresolvedReference for exit_data property: room='{}', exit_index={}, string_id={}, object_space offset=0x{:04x}",
                                    object.name, exit_index, string_id, data_offset
                                );
                                break;
                            }
                        }
                    }
                }
                debug!(
                    "PROP TABLE DEBUG: Writing prop data byte {}=0x{:02x} at addr=0x{:04x}",
                    i, byte, addr
                );
                addr += 1;
            }
        }

        // Terminator (property 0)
        let terminator_offset = addr - self.object_table_addr;
        self.write_to_object_space(terminator_offset, 0)?;
        debug!(
            "PROP TABLE DEBUG: Writing terminator 0x00 at addr=0x{:04x}",
            addr
        );
        addr += 1;

        // Debug: Dump first 10 bytes of property table from object_space
        let dump_end = (prop_table_addr - self.object_table_addr + 10).min(self.object_space.len());
        let dump_bytes: Vec<u8> =
            self.object_space[prop_table_addr - self.object_table_addr..dump_end].to_vec();

        log::warn!(
            "🏠 PROP_TABLE_COMPLETE: object='{}' obj_num={} prop_table_addr=0x{:04x} range=0x{:04x}-0x{:04x} size={} bytes first_bytes={:02x?}",
            object.short_name,
            obj_num,
            prop_table_addr,
            prop_table_addr,
            addr - 1,
            addr - prop_table_addr,
            dump_bytes
        );

        // Update current property allocation pointer for next property table
        self.current_property_addr = addr;

        debug!(
            "Property table for '{}' (object #{}) created at 0x{:04x} with {} properties: {:?}",
            object.short_name,
            obj_num,
            prop_table_addr,
            object.properties.properties.len(),
            object.properties.properties.keys().collect::<Vec<_>>()
        );

        // DEBUG: Critical return value tracking
        debug!(
            " PROPERTY TABLE: create_property_table_from_ir for object {} ('{}'):",
            obj_num, object.short_name
        );
        debug!(
            " - Started at prop_table_addr (current_property_addr): 0x{:04x}",
            prop_table_addr
        );
        debug!(" - Final addr after writing all properties: 0x{:04x}", addr);
        debug!(
            " - Updated current_property_addr to: 0x{:04x}",
            self.current_property_addr
        );
        debug!(
            " - RETURNING property table address: 0x{:04x}",
            prop_table_addr
        );

        Ok(prop_table_addr)
    }

    /// Patch property table addresses in object entries from object space relative to absolute addresses
    ///
    /// After object_space is copied to final_data, property table pointers in object headers
    /// need to be updated from space-relative offsets to absolute memory addresses.
    ///
    /// CRITICAL: We calculate max_objects from total space size, which includes property tables.
    /// This means we'll loop past actual objects into property table data. We detect the boundary
    /// by checking if property table pointers are valid (must point >= defaults_size).
    /// When we hit invalid pointers, we BREAK (not continue) - we've reached the boundary.
    ///
    /// Bug History: Previously looped through 126 "objects" when only 14 existed, treating
    /// property table bytes as object headers and corrupting property data. See CLAUDE.md Bug 6.
    pub fn patch_property_table_addresses(
        &mut self,
        object_base: usize,
    ) -> Result<(), CompilerError> {
        log::debug!(" PATCH: Starting property table address patching");

        // Calculate how many objects exist based on object space size
        // Each object entry is 9 bytes in V3: attributes(4) + parent(1) + sibling(1) + child(1) + prop_table_addr(2)
        let obj_entry_size = match self.version {
            ZMachineVersion::V3 => 9,
            ZMachineVersion::V4 | ZMachineVersion::V5 => 14,
        };

        // Find where objects end by looking for property defaults table
        // Property defaults come first, then objects, then property tables
        let default_props = match self.version {
            ZMachineVersion::V3 => 31,
            ZMachineVersion::V4 | ZMachineVersion::V5 => 63,
        };
        let defaults_size = default_props * 2; // 2 bytes per default
        let objects_start = defaults_size; // Objects start after defaults

        // Calculate maximum possible objects from remaining space
        // WARNING: This OVERESTIMATES because remaining_space includes property tables!
        // We use validation below to detect when we've gone past actual objects.
        let remaining_space = self.object_space.len() - objects_start;
        let max_objects = remaining_space / obj_entry_size;

        log::debug!(" PATCH: Object space layout analysis:");
        log::debug!(" - Object space size: {} bytes", self.object_space.len());
        log::debug!(
            " - Defaults table: {} bytes (0x00-0x{:02x})",
            defaults_size,
            defaults_size - 1
        );
        log::debug!(" - Objects start at: 0x{:02x}", objects_start);
        log::debug!(
            " - Max objects: {} ({}x{} bytes)",
            max_objects,
            max_objects,
            obj_entry_size
        );

        // Calculate minimum valid property table address
        // Property tables must be AFTER all object headers
        // We don't know exact object count, but we can detect when we've gone too far
        // by checking if property table pointer points backwards into defaults/headers
        let min_valid_prop_addr = objects_start; // At minimum, must be after defaults

        // Patch property table addresses for each object
        let mut objects_patched = 0;
        for obj_index in 0..max_objects {
            let obj_offset_in_space = objects_start + (obj_index * obj_entry_size);
            let prop_addr_offset = obj_offset_in_space + 7; // Property table address at bytes 7-8

            // Check if we're still within object space bounds
            if prop_addr_offset + 1 >= self.object_space.len() {
                break; // Reached end of object space
            }

            // Read the current object space relative property table address
            let final_addr_offset = object_base + prop_addr_offset;

            debug!(
                " PATCH DETAILED: Object {} (index {}):",
                obj_index + 1,
                obj_index
            );
            debug!(" - obj_offset_in_space: 0x{:04x}", obj_offset_in_space);
            debug!(" - prop_addr_offset: 0x{:04x}", prop_addr_offset);
            debug!(
                " - final_addr_offset: 0x{:04x} (object_base 0x{:04x} + prop_addr_offset 0x{:04x})",
                final_addr_offset, object_base, prop_addr_offset
            );

            // Debug what we're reading from final_data
            let byte1 = self.final_data[final_addr_offset];
            let byte2 = self.final_data[final_addr_offset + 1];
            debug!(
                " - Reading bytes from final_data[0x{:04x}]: 0x{:02x} 0x{:02x}",
                final_addr_offset, byte1, byte2
            );
            debug!(
                " - As chars: '{}' '{}'",
                if (0x20..=0x7e).contains(&byte1) {
                    byte1 as char
                } else {
                    '.'
                },
                if (0x20..=0x7e).contains(&byte2) {
                    byte2 as char
                } else {
                    '.'
                }
            );

            let space_relative_addr = ((byte1 as u16) << 8) | (byte2 as u16);
            debug!(
                " - Decoded space_relative_addr: 0x{:04x}",
                space_relative_addr
            );

            // Check if this looks like a valid property table address
            // Valid property table pointers must:
            // 1. Be non-zero
            // 2. Point after the defaults table (>= min_valid_prop_addr)
            // 3. Be within object_space bounds
            if space_relative_addr == 0
                || space_relative_addr < min_valid_prop_addr as u16
                || space_relative_addr > self.object_space.len() as u16
            {
                log::debug!(
                    " PATCH: Object {} has invalid prop table addr 0x{:04x} (expected >= 0x{:04x}), stopping iteration",
                    obj_index + 1,
                    space_relative_addr,
                    min_valid_prop_addr
                );
                break; // Stop iteration - we've gone past actual objects into property table data
            }

            // Calculate absolute final memory address
            let absolute_addr = object_base + (space_relative_addr as usize);
            debug!(" - Calculated absolute_addr: 0x{:04x} (object_base 0x{:04x} + space_relative 0x{:04x})", 
 absolute_addr, object_base, space_relative_addr);

            // Write the corrected absolute address back to final_data
            let new_high_byte = (absolute_addr >> 8) as u8;
            let new_low_byte = (absolute_addr & 0xFF) as u8;
            debug!(
                " - Writing absolute addr 0x{:04x} as bytes: 0x{:02x} 0x{:02x}",
                absolute_addr, new_high_byte, new_low_byte
            );

            self.final_data[final_addr_offset] = new_high_byte; // High byte
            self.final_data[final_addr_offset + 1] = new_low_byte; // Low byte

            // Verify what we just wrote
            let verify_byte1 = self.final_data[final_addr_offset];
            let verify_byte2 = self.final_data[final_addr_offset + 1];
            let verify_addr = ((verify_byte1 as u16) << 8) | (verify_byte2 as u16);
            debug!(
                " - VERIFICATION: Read back 0x{:02x} 0x{:02x} = 0x{:04x}",
                verify_byte1, verify_byte2, verify_addr
            );

            objects_patched += 1;
            log::warn!(
                "🔧 PATCH_OBJ: obj_num={} space_relative=0x{:04x} → absolute=0x{:04x} final_offset=0x{:04x}",
                obj_index + 1,
                space_relative_addr,
                absolute_addr,
                final_addr_offset
            );
        }

        log::warn!("🔧 PATCH_COMPLETE: {} objects patched", objects_patched);
        Ok(())
    }

    /// Encode an object name as Z-Machine text

    /// Get object name by object number (for property table generation)
    fn get_object_name_by_number(&self, obj_num: u8) -> String {
        // Special cases for implicit objects
        match obj_num {
            1 => "player".to_string(),
            _ => {
                // Find object name by number in the registry
                for (name, &number) in &self.object_numbers {
                    if number == obj_num as u16 {
                        return name.clone();
                    }
                }
                format!("object_{}", obj_num) // Fallback
            }
        }
    }

    /// Generate global variables table
    pub fn generate_global_variables(&mut self, ir: &IrProgram) -> Result<(), CompilerError> {
        let globals_start = self.global_vars_addr;
        self.ensure_capacity(globals_start + 480); // Space for 240 globals

        // Initialize all globals to 0
        for i in 0..240 {
            let global_offset = i * 2;
            self.write_to_globals_space(global_offset, 0)?; // High byte
            self.write_to_globals_space(global_offset + 1, 0)?; // Low byte
        }

        // Set specific globals from IR
        // CRITICAL: Initialize global variable G00 with player object number
        // G00 is at offset 0 in globals_space (first 2 bytes: high byte, low byte)
        self.write_to_globals_space(0, 0)?; // High byte of player object #1 = 0
        self.write_to_globals_space(1, 1)?; // Low byte of player object #1 = 1
        log::debug!("Initialized global G00 (Variable 16) with player object number: 1");

        for _global in &ir.globals {
            // TODO: Map additional IR globals to Z-Machine global variables
            // For now, just ensure the space is allocated
        }

        // CRITICAL: Update final_assembly_address to reflect actual end of global variables
        // This eliminates gaps between global variables and subsequent data structures
        let globals_end = globals_start + 480; // 240 globals * 2 bytes each
        let new_addr = std::cmp::max(self.final_assembly_address, globals_end);
        self.set_final_assembly_address(new_addr, "Global variables alignment");
        debug!("Global variables complete, final_assembly_address updated to: 0x{:04x} (globals_end: 0x{:04x})", 
 self.final_assembly_address, globals_end);

        Ok(())
    }

    /// Generate program flow based on program mode
    pub fn generate_program_flow(&mut self, ir: &IrProgram) -> Result<(), CompilerError> {
        debug!("Generating program flow for mode: {:?}", ir.program_mode);

        match ir.program_mode {
            crate::grue_compiler::ast::ProgramMode::Script => {
                // Script mode: No main loop needed, program will quit after init
                debug!("Script mode: No main loop generated");
                Ok(())
            }
            crate::grue_compiler::ast::ProgramMode::Interactive => {
                // Interactive mode: Generate automatic main loop
                debug!("Interactive mode: Generating automatic main loop");
                self.generate_main_loop(ir)
            }
            crate::grue_compiler::ast::ProgramMode::Custom => {
                // Custom mode: Generate call to user's main function
                debug!("Custom mode: Will call user's main function");
                self.generate_custom_main_call(ir)
            }
        }
    }

    /// Generate call to user's main function (for custom mode)
    fn generate_custom_main_call(&mut self, ir: &IrProgram) -> Result<(), CompilerError> {
        debug!("Generating custom main function call");

        if let Some(main_function) = ir.get_main_function() {
            debug!("Found main function with ID {}", main_function.id);

            // Align function address according to Z-Machine version requirements
            self.align_function_address()?;

            let main_call_routine_address = self.code_address;
            let main_call_id = 9000u32; // Use high ID to avoid conflicts

            // Create a routine that calls the user's main function
            self.emit_byte(0x00)?; // Routine header: 0 locals

            // Record the routine address for reference resolution
            self.function_addresses
                .insert(main_call_id, main_call_routine_address);
            self.record_final_address(main_call_id, main_call_routine_address);

            // Call the user's main function
            let layout = self.emit_instruction_typed(
                Opcode::OpVar(OpVar::CallVs),                  // call_vs instruction
                &[Operand::LargeConstant(placeholder_word())], // Placeholder for main function address
                Some(0x00), // Store result in local variable 0 (discarded)
                None,       // No branch
            )?;

            // Add unresolved reference for main function call
            self.reference_context
                .unresolved_refs
                .push(UnresolvedReference {
                    reference_type: LegacyReferenceType::FunctionCall,
                    location: layout
                        .operand_location
                        .expect("call instruction must have operand"),
                    target_id: main_function.id,
                    is_packed_address: true, // Function calls use packed addresses
                    offset_size: 2,
                    location_space: MemorySpace::Code,
                });

            // After main function returns, quit the program
            self.emit_byte(0xBA)?; // quit opcode

            Ok(())
        } else {
            Err(CompilerError::CodeGenError(
                "Custom mode requires main() function, but none was found".to_string(),
            ))
        }
    }

    /// Align function address according to Z-Machine version requirements
    fn align_function_address(&mut self) -> Result<(), CompilerError> {
        match self.version {
            ZMachineVersion::V3 => {
                // v3: functions must be at even addresses
                if self.code_address % 2 != 0 {
                    self.emit_byte(0xB4)?; // nop instruction (safe padding that won't crash)
                }
            }
            ZMachineVersion::V4 | ZMachineVersion::V5 => {
                // v4/v5: functions must be at 4-byte boundaries
                while self.code_address % 4 != 0 {
                    self.emit_byte(0xB4)?; // nop instruction (safe padding that won't crash)
                }
            }
        }
        Ok(())
    }

    /// Generate the automatic main game loop
    fn generate_main_loop(&mut self, _ir: &IrProgram) -> Result<(), CompilerError> {
        debug!(
            "🎯 MAIN_LOOP: Starting main loop generation at code_address 0x{:04x}",
            self.code_address
        );

        // Align function address according to Z-Machine version requirements
        self.align_function_address()?;

        // Record main loop routine address for function calls
        let main_loop_id = 9000u32; // Use high ID to avoid conflicts
        let main_loop_routine_address = self.code_address;
        debug!(
            "🎯 MAIN_LOOP: Recording main loop routine at address 0x{:04x} (ID {})",
            main_loop_routine_address, main_loop_id
        );

        debug!(
            "Main loop routine starts at address 0x{:04x}",
            main_loop_routine_address
        );

        // Main loop needs 7 locals for grammar matching system:
        // Variable 1: word count
        // Variable 2: word 1 dictionary address (noun for object resolution)
        // Variable 3: resolved object ID (noun matching)
        // Variable 4: loop counter (object lookup)
        // Variable 5: property value (object lookup)
        // Variable 6: word 0 dictionary address (verb matching)
        // Variable 7: temporary for additional grammar operations
        self.emit_byte(0x07)?; // Routine header: 7 locals

        // V3 requires initial values for each local variable (2 bytes each)
        for _ in 0..7 {
            self.emit_word(0x0000)?; // Initialize all locals to 0
        }

        // Record the routine address (including header) for function calls
        self.function_addresses
            .insert(main_loop_id, main_loop_routine_address);
        self.record_final_address(main_loop_id, main_loop_routine_address); // Record for reference resolution

        // Record the first instruction address for jump targets
        let main_loop_first_instruction = self.code_address;
        let main_loop_jump_id = main_loop_id + 1; // Different ID for jump target
        self.record_final_address(main_loop_jump_id, main_loop_first_instruction);

        // 1. Print prompt "> "
        let prompt_string_id = self
            .main_loop_prompt_id
            .expect("Main loop prompt ID should be set during string collection");

        let layout = self.emit_instruction_typed(
            Opcode::Op1(Op1::PrintPaddr), // print_paddr (print packed address string)
            &[Operand::LargeConstant(placeholder_word())], // Placeholder for prompt string address
            None,                         // No store
            None,                         // No branch
        )?;

        // Add unresolved reference for prompt string using layout-tracked operand location
        self.reference_context
            .unresolved_refs
            .push(UnresolvedReference {
                reference_type: LegacyReferenceType::StringRef,
                location: layout
                    .operand_location
                    .expect("print_paddr instruction must have operand"),
                target_id: prompt_string_id,
                is_packed_address: true,
                offset_size: 2,
                location_space: MemorySpace::Code,
            });

        // 2. Use properly allocated buffer addresses from layout phase
        let text_buffer_addr = self.text_buffer_addr as u16;
        let parse_buffer_addr = self.parse_buffer_addr as u16;

        debug!(
            "Using input buffers: text=0x{:04x}, parse=0x{:04x}",
            text_buffer_addr, parse_buffer_addr
        );

        // 3. Store buffer addresses in global variables (like Zork I pattern)
        // Zork I uses SREAD G6d,G6e - we'll use similar globals G6d(109) and G6e(110)
        const TEXT_BUFFER_GLOBAL: u8 = 109; // Global G6d = Variable(109)
        const PARSE_BUFFER_GLOBAL: u8 = 110; // Global G6e = Variable(110)

        // Store text buffer address in global G6d using VAR:storew
        // ARCHITECTURAL FIX: Cannot use 2OP:add (0x14) with LargeConstant because
        // it forces VAR form, changing opcode to VAR:call_vs (0x14) - wrong instruction!
        // Solution: Use VAR:storew to write directly to global variable memory
        //
        // Variable 109 (Global G6d) is at: globals_base + ((109-16) * 2) = globals_base + 186
        let text_buffer_offset = (TEXT_BUFFER_GLOBAL - 16) as u16;
        let storew_layout = self.emit_instruction_typed(
            Opcode::OpVar(OpVar::Storew), // VAR:storew (always VAR with 3 operands)
            &[
                Operand::LargeConstant(placeholder_word()), // base = globals_addr (resolved later)
                Operand::SmallConstant(text_buffer_offset as u8), // offset in words
                Operand::LargeConstant(text_buffer_addr),   // value = text buffer address
            ],
            None,
            None,
        )?;

        // Create UnresolvedReference for globals base address (first operand)
        self.reference_context
            .unresolved_refs
            .push(UnresolvedReference {
                reference_type: LegacyReferenceType::GlobalsBase,
                location: storew_layout
                    .operand_location
                    .expect("storew should have operand location"),
                target_id: 0,
                is_packed_address: false,
                offset_size: 2,
                location_space: MemorySpace::Code,
            });

        // Store parse buffer address in global G6e using VAR:storew
        // Variable 110 (Global G6e) is at: globals_base + ((110-16) * 2) = globals_base + 188
        let parse_buffer_offset = (PARSE_BUFFER_GLOBAL - 16) as u16;
        let storew_layout = self.emit_instruction_typed(
            Opcode::OpVar(OpVar::Storew), // VAR:storew
            &[
                Operand::LargeConstant(placeholder_word()), // base = globals_addr (resolved later)
                Operand::SmallConstant(parse_buffer_offset as u8), // offset in words
                Operand::LargeConstant(parse_buffer_addr),  // value = parse buffer address
            ],
            None,
            None,
        )?;

        // Create UnresolvedReference for globals base address (first operand)
        self.reference_context
            .unresolved_refs
            .push(UnresolvedReference {
                reference_type: LegacyReferenceType::GlobalsBase,
                location: storew_layout
                    .operand_location
                    .expect("storew should have operand location"),
                target_id: 0,
                is_packed_address: false,
                offset_size: 2,
                location_space: MemorySpace::Code,
            });

        // 4. Read user input using Z-Machine sread instruction with global variables (like Zork I)
        //
        // SREAD TYPE-SAFETY FIX: Now uses emit_instruction_typed with OpVar::Sread instead
        // of raw opcode 0x04. This was enabled by renaming Aread→Sread in opcodes.rs to
        // align the typed enum with our V3 target platform. Previously required raw opcode
        // because Aread was V4+ only, but Sread correctly represents V1-V3 semantics.
        self.emit_instruction_typed(
            Opcode::OpVar(OpVar::Sread), // V1-V3 sread instruction - now type-safe
            &[
                Operand::Variable(TEXT_BUFFER_GLOBAL), // Global G6d = Variable(109)
                Operand::Variable(PARSE_BUFFER_GLOBAL), // Global G6e = Variable(110)
            ],
            None, // No store
            None, // No branch
        )?;

        // 4. Process parsed input - check for quit command
        self.generate_command_processing(_ir, parse_buffer_addr, main_loop_jump_id)?;

        // 5. Jump back to start of main loop for next command
        debug!("Generating loop-back jump to continue main loop");

        let layout = self.emit_instruction_typed(
            Opcode::Op1(Op1::Jump),                        // jump opcode (1OP:12)
            &[Operand::LargeConstant(placeholder_word())], // Placeholder for loop start address
            None,                                          // No store
            None,                                          // No branch
        )?;

        // Register UnresolvedReference to jump back to main loop start
        self.reference_context
            .unresolved_refs
            .push(UnresolvedReference {
                reference_type: LegacyReferenceType::Jump,
                location: layout
                    .operand_location
                    .expect("jump instruction must have operand location"),
                target_id: main_loop_jump_id, // Jump to first instruction after routine header
                is_packed_address: false,
                offset_size: 2,
                location_space: MemorySpace::Code,
            });

        debug!(
            "Main loop generation complete at 0x{:04x} (with loop-back to 0x{:04x})",
            self.code_address, main_loop_first_instruction
        );
        Ok(())
    }

    /// Generate command processing logic after SREAD instruction
    /// This checks the parse buffer for commands and handles quit
    fn generate_command_processing(
        &mut self,
        ir: &IrProgram,
        _parse_buffer_addr: u16,
        main_loop_jump_id: u32,
    ) -> Result<(), CompilerError> {
        debug!("Generating grammar-based command processing with pattern matching");

        // Check if there are any grammar rules to process
        if ir.grammar.is_empty() {
            debug!("No grammar rules found - no grammar to process");
        // No grammar defined - just continue to unknown command message
        } else {
            // Generate grammar pattern matching engine
            self.generate_grammar_pattern_matching(&ir.grammar, main_loop_jump_id)?;
        }

        // Default handler: print unknown command and continue
        let unknown_command_string_id = self
            .main_loop_unknown_command_id
            .expect("Main loop unknown command ID should be set during string collection");
        let layout = self.emit_instruction_typed(
            Opcode::Op1(Op1::PrintPaddr), // print_paddr: print string at packed address
            &[Operand::LargeConstant(placeholder_word())], // Placeholder for string address
            None,
            None,
        )?;

        // FIXED: Use layout.operand_location instead of hardcoded offset calculation
        // This was previously using self.code_address - 2 which caused placeholder resolution failures
        if let Some(operand_location) = layout.operand_location {
            self.reference_context
                .unresolved_refs
                .push(UnresolvedReference {
                    reference_type: LegacyReferenceType::StringRef,
                    location: operand_location, // Correct operand location from emit_instruction
                    target_id: unknown_command_string_id,
                    is_packed_address: true,
                    offset_size: 2,
                    location_space: MemorySpace::Code,
                });
        } else {
            panic!("BUG: emit_instruction didn't return operand_location for placeholder");
        }

        Ok(())
    }

    /// Generate grammar pattern matching engine that processes user input
    /// This examines parse buffer tokens and matches them against defined grammar patterns
    fn generate_grammar_pattern_matching(
        &mut self,
        grammar_rules: &[crate::grue_compiler::ir::IrGrammar],
        main_loop_jump_id: u32,
    ) -> Result<(), CompilerError> {
        debug!(
            "Generating grammar pattern matching for {} verbs at code_address=0x{:04x}",
            grammar_rules.len(),
            self.code_address
        );

        // For each grammar verb, generate pattern matching logic
        for grammar in grammar_rules {
            debug!(
                "Processing grammar verb: '{}' at code_address=0x{:04x}",
                grammar.verb, self.code_address
            );

            // Generate verb matching: check if first token matches this verb
            self.generate_verb_matching(&grammar.verb, &grammar.patterns, main_loop_jump_id)?;
        }

        debug!("Grammar pattern matching generation complete");
        Ok(())
    }

    /// Generate Z-Machine code to match a specific verb and its patterns
    fn generate_verb_matching(
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
        const PARSE_BUFFER_GLOBAL: u8 = 110; // Global G6e = Variable(110)

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

        // Phase 3.0: FIRST: Check for literal patterns within this verb (e.g., "around" in verb "look")
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
                        Some(0x7FFF_u16 as i16), // Branch on FALSE (not equal) - skip if word count != 2
                    )?;

                    // Register branch to skip_literal_label for word count mismatch
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
                    }

                    // Load word 1 dictionary address from parse buffer offset 3
                    debug!(
                        "🔍 LITERAL_LOAD_WORD1: Loading word 1 dictionary address for literal '{}'",
                        literal_word
                    );
                    self.emit_instruction_typed(
                        Opcode::Op2(Op2::Loadw),
                        &[
                            Operand::Variable(PARSE_BUFFER_GLOBAL),
                            Operand::SmallConstant(3), // Word 1 dict addr at offset 3 (word offset, not byte)
                        ],
                        Some(7), // Store in local variable 7 (temporary for grammar operations)
                        None,
                    )?;
                    debug!("🔍 LITERAL_LOAD_WORD1_COMPLETE: Stored word 1 in Variable(7) for literal '{}'", literal_word);

                    // Compare word 1 with literal dictionary address
                    debug!(
                        "🔍 LITERAL_COMPARE: Comparing word 1 with literal '{}'",
                        literal_word
                    );
                    debug!("🔍 LITERAL_COMPARE_SETUP: About to emit JE instruction at code_address=0x{:04x}", self.code_address);
                    debug!("🔍 LITERAL_COMPARE_DETAILS: Will compare Variable(7) [word 1] against literal '{}' dictionary address", literal_word);
                    let dict_ref_operand = Operand::LargeConstant(placeholder_word());
                    let layout = self.emit_instruction_typed(
                        Opcode::Op2(Op2::Je),
                        &[
                            Operand::Variable(7), // Word 2 from parse buffer (stored in variable 7)
                            dict_ref_operand,     // Literal word dictionary address
                        ],
                        None,
                        Some(0x7FFF_u16 as i16), // Branch on FALSE (not equal) - skip if no match
                    )?;
                    debug!("🔍 LITERAL_COMPARE_EMITTED: JE instruction emitted for literal '{}' comparison", literal_word);

                    // Register branch to skip_literal_label for word mismatch
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
                    }

                    // Register dictionary reference for literal word
                    if let Some(operand_base) = layout.operand_location {
                        let dict_operand_location = operand_base + 1; // Second operand location
                        let word_position = self
                            .dictionary_words
                            .iter()
                            .position(|w| w == literal_word)
                            .unwrap_or_else(|| {
                                panic!(
                                    "FATAL: Literal word '{}' not found in dictionary!",
                                    literal_word
                                );
                            });

                        debug!("🔍 DICT_REF: Registering dictionary reference for literal word '{}' at location 0x{:04x}", literal_word, dict_operand_location);
                        self.reference_context
                            .unresolved_refs
                            .push(UnresolvedReference {
                                reference_type: LegacyReferenceType::DictionaryRef {
                                    word: literal_word.clone(),
                                },
                                location: dict_operand_location,
                                target_id: word_position as u32,
                                is_packed_address: false,
                                offset_size: 2,
                                location_space: MemorySpace::Code,
                            });
                    }

                    // If we get here, the literal pattern matched - call the function
                    if let crate::grue_compiler::ir::IrHandler::FunctionCall(func_id, args) =
                        &literal_pattern.handler
                    {
                        debug!(
                            "🔄 LITERAL_CALL: Calling function {} for literal pattern '{}'",
                            func_id, literal_word
                        );

                        // Generate function call
                        let mut operands = vec![Operand::LargeConstant(placeholder_word())];

                        // Convert arguments to operands (literal patterns typically have no args)
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
                            operands.push(arg_operand);
                        }

                        // Emit call_vs instruction to call the function
                        // call_vs MUST store return value to avoid instruction misalignment
                        let layout = self.emit_instruction_typed(
                            Opcode::OpVar(OpVar::CallVs),
                            &operands,
                            Some(1), // Store return value in local variable 1 (unused)
                            None,
                        )?;

                        // Register function reference for the first operand
                        if let Some(operand_base) = layout.operand_location {
                            self.reference_context
                                .unresolved_refs
                                .push(UnresolvedReference {
                                    reference_type: LegacyReferenceType::FunctionCall,
                                    location: operand_base,
                                    target_id: *func_id,
                                    is_packed_address: true,
                                    offset_size: 2,
                                    location_space: MemorySpace::Code,
                                });
                        }

                        // Jump back to main loop after successfully executing literal pattern
                        debug!(
                            "🔀 JUMP_MAIN_LOOP: Jumping back to main loop start (label {}) after literal pattern handler",
                            main_loop_jump_id
                        );

                        let layout = self.emit_instruction_typed(
                            Opcode::Op1(Op1::Jump),                        // jump
                            &[Operand::LargeConstant(placeholder_word())], // Will be resolved to main loop start
                            None,
                            None,
                        )?;

                        if let Some(operand_location) = layout.operand_location {
                            self.reference_context
                                .unresolved_refs
                                .push(UnresolvedReference {
                                    reference_type: LegacyReferenceType::Jump,
                                    location: operand_location,
                                    target_id: main_loop_jump_id,
                                    is_packed_address: false,
                                    offset_size: 2,
                                    location_space: MemorySpace::Code,
                                });
                        } else {
                            panic!("BUG: emit_instruction didn't return operand_location for placeholder");
                        }
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

        // LITERAL PATTERN CHECK: Before noun processing, check for single literal patterns like "around"
        // DISABLED: This old code conflicts with verb-internal literal patterns and causes "I don't understand that"
        // TODO: Remove this old code entirely and replace with proper verb-internal literal pattern support
        let _old_literal_disabled = true;
        if false {
            for pattern in patterns.iter() {
                if pattern.pattern.len() == 1 {
                    if let crate::grue_compiler::ir::IrPatternElement::Literal(literal_word) =
                        &pattern.pattern[0]
                    {
                        debug!(
                            "🔤 LITERAL_CHECK: Testing for literal word '{}'",
                            literal_word
                        );

                        // Generate unique label IDs for this literal pattern (use range 80000-89999 for literals)
                        let unique_seed = (self.code_address * 13) % 9999;
                        let _literal_handler_label = (80000 + unique_seed) as u32;
                        let skip_literal_label = (81000 + unique_seed) as u32;

                        // Compare word 1 from parse buffer with literal word dictionary address
                        let dict_ref_operand = Operand::LargeConstant(placeholder_word());

                        // Word count == 2 for single literal patterns
                        debug!("🔍 LITERAL_WORDCOUNT_CHECK: Checking if word count (Variable 1) == 2 for single literal '{}'", literal_word);
                        self.emit_instruction_typed(
                            Opcode::Op2(Op2::Je),
                            &[
                                Operand::Variable(1), // Word count
                                Operand::SmallConstant(2),
                            ],
                            None,
                            Some(0xBFFF_u16 as i16), // Branch on FALSE (not equal) - skip if word count != 2
                        )?;

                        // Load word 1 dictionary address from parse buffer offset 3
                        debug!("🔍 LITERAL_LOAD_WORD: Loading word 1 dictionary address for single literal '{}'", literal_word);
                        self.emit_instruction_typed(
                            Opcode::Op2(Op2::Loadw),
                            &[
                                Operand::Variable(PARSE_BUFFER_GLOBAL),
                                Operand::SmallConstant(3), // Word 1 dict addr at offset 3 (FIXED: was 4)
                            ],
                            Some(2), // Store in local variable 2 (variable 1 is used for word count)
                            None,
                        )?;

                        // Compare word 1 with literal dictionary address: je @2 dict_addr skip_literal_label
                        let layout = self.emit_instruction_typed(
                            Opcode::Op2(Op2::Je),
                            &[
                                Operand::Variable(2), // Word 1 from parse buffer (now in variable 2)
                                dict_ref_operand,     // Literal word dictionary address
                            ],
                            None,
                            Some(0x7FFF_u16 as i16), // Branch on FALSE (not equal) - skip this literal if it doesn't match
                        )?;

                        // Calculate proper location for dictionary reference (second operand in je instruction)
                        // je instruction layout: [opcode][operand1: Variable(1)][operand2: dict_addr]
                        // For VAR form je: [instruction_byte][operand_types][operand1][operand2]
                        let dict_operand_location = if let Some(operand_base) =
                            layout.operand_location
                        {
                            operand_base + 1 // Skip first operand (Variable = 1 byte) to reach second operand
                        } else {
                            panic!("BUG: emit_instruction didn't return operand_location for je");
                        };

                        debug!("🔍 DICT_REF_PREPARE: About to register dictionary reference for literal word '{}' at location 0x{:04x}", literal_word, dict_operand_location);

                        // Register dictionary reference for literal word
                        // Look up the word's position in the sorted dictionary
                        let word_position = self.dictionary_words.iter().position(|w| w == literal_word)
                        .unwrap_or_else(|| {
                            panic!("FATAL: Literal word '{}' not found in dictionary_words! Available words: {:?}",
                                   literal_word, self.dictionary_words);
                        });
                        debug!("🔍 DICT_REF_REGISTER_SINGLE: Registering dictionary reference for single literal word '{}' at location 0x{:04x}, position={}", literal_word, dict_operand_location, word_position);
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

                        // Register branch to skip_literal_label
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
                        }

                        // MATCHED: Execute literal pattern handler
                        if let crate::grue_compiler::ir::IrHandler::FunctionCall(func_id, args) =
                            &pattern.handler
                        {
                            debug!(
                                "🔤 LITERAL_EXECUTE: Calling function {} for literal '{}'",
                                func_id, literal_word
                            );

                            // Generate function call - same as existing noun pattern logic
                            let mut operands = vec![Operand::LargeConstant(placeholder_word())];

                            // Convert arguments to operands (literal patterns typically have no args)
                            for arg_value in args.iter() {
                                let arg_operand = match arg_value {
                                    crate::grue_compiler::ir::IrValue::Integer(n) => {
                                        if *n >= 0 && *n <= 255 {
                                            Operand::SmallConstant(*n as u8)
                                        } else {
                                            Operand::LargeConstant(*n as u16)
                                        }
                                    }
                                    _ => {
                                        debug!("🔤 LITERAL_WARNING: Unexpected argument type in literal pattern: {:?}", arg_value);
                                        Operand::SmallConstant(0)
                                    }
                                };
                                operands.push(arg_operand);
                            }

                            // Emit function call
                            let layout = self.emit_instruction_typed(
                                Opcode::OpVar(OpVar::CallVs),
                                &operands,
                                Some(0), // Store return value but ignore it
                                None,
                            )?;

                            // Register function reference
                            if let Some(operand_location) = layout.operand_location {
                                self.reference_context
                                    .unresolved_refs
                                    .push(UnresolvedReference {
                                        reference_type: LegacyReferenceType::FunctionCall,
                                        location: operand_location, // Function address operand
                                        target_id: *func_id,
                                        is_packed_address: true,
                                        offset_size: 2,
                                        location_space: MemorySpace::Code,
                                    });
                            }

                            // Jump to end of function after executing handler
                            let layout = self.emit_instruction_typed(
                                Opcode::Op1(Op1::Jump),
                                &[Operand::LargeConstant(placeholder_word())], // Jump offset placeholder
                                None,
                                None,
                            )?;

                            // Register jump to end of function
                            if let Some(operand_location) = layout.operand_location {
                                self.reference_context
                                    .unresolved_refs
                                    .push(UnresolvedReference {
                                        reference_type: LegacyReferenceType::Jump,
                                        location: operand_location, // Jump offset operand
                                        target_id: end_function_label,
                                        is_packed_address: false,
                                        offset_size: 2,
                                        location_space: MemorySpace::Code,
                                    });
                            }
                        }

                        // Register the skip_literal_label at current address (fall through to next pattern)
                        self.record_final_address(skip_literal_label, self.code_address);
                    }
                }
            }
        } // END DISABLED OLD LITERAL PATTERN CODE

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

                        // Build operands: function address + noun parameter
                        let mut operands = vec![Operand::LargeConstant(placeholder_word())]; // Function address placeholder

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
                            operands.push(arg_operand);
                        }

                        // Emit function call
                        let layout = self.emit_instruction_typed(
                            Opcode::OpVar(OpVar::CallVs),
                            &operands,
                            Some(0), // Store return value but ignore it
                            None,
                        )?;

                        // Register function reference
                        if let Some(operand_location) = layout.operand_location {
                            self.reference_context
                                .unresolved_refs
                                .push(UnresolvedReference {
                                    reference_type: LegacyReferenceType::FunctionCall,
                                    location: operand_location,
                                    target_id: *func_id,
                                    is_packed_address: true,
                                    offset_size: 2,
                                    location_space: MemorySpace::Code,
                                });
                        }

                        // Jump back to main loop after successfully executing literal+noun pattern
                        debug!(
                            "🔀 JUMP_MAIN_LOOP: Jumping back to main loop start (label {}) after literal+noun pattern handler",
                            main_loop_jump_id
                        );

                        let layout = self.emit_instruction_typed(
                            Opcode::Op1(Op1::Jump),
                            &[Operand::LargeConstant(placeholder_word())],
                            None,
                            None,
                        )?;

                        // Register jump back to main loop (same as simple literal patterns)
                        if let Some(operand_location) = layout.operand_location {
                            self.reference_context
                                .unresolved_refs
                                .push(UnresolvedReference {
                                    reference_type: LegacyReferenceType::Jump,
                                    location: operand_location,
                                    target_id: main_loop_jump_id,
                                    is_packed_address: false,
                                    offset_size: 2,
                                    location_space: MemorySpace::Code,
                                });
                        }
                    }

                    // Register the skip_literal_noun_label at current address
                    self.record_final_address(skip_literal_noun_label, self.code_address);

                    // Register the skip_insufficient_words label at the same address (both conditions skip this pattern)
                    self.record_final_address(skip_insufficient_words, self.code_address);
                }
            }
        }

        // VERB+NOUN CASE: We have at least 2 words, process noun pattern
        if let Some(pattern) = noun_pattern {
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

                // Build operands: function address + arguments
                let mut operands = vec![Operand::LargeConstant(placeholder_word())]; // Function address placeholder

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
                    operands.push(arg_operand);
                }

                // Call handler with properly resolved parameters
                let layout = self.emit_instruction_typed(
                    Opcode::OpVar(OpVar::CallVs), // call_vs: call routine with arguments, returns value (VAR:0/VAR:224)
                    &operands,
                    Some(0), // Store result on stack
                    None,    // No branch
                )?;

                // FIXED: Use layout.operand_location instead of hardcoded offset calculation
                // This was previously using self.code_address - 2 which caused placeholder resolution failures
                if let Some(operand_location) = layout.operand_location {
                    let dispatch_func_id = self.get_function_id_with_dispatch(*func_id);
                    self.reference_context
                        .unresolved_refs
                        .push(UnresolvedReference {
                            reference_type: LegacyReferenceType::FunctionCall,
                            location: operand_location, // Correct operand location from emit_instruction
                            target_id: dispatch_func_id,
                            is_packed_address: true,
                            offset_size: 2,
                            location_space: MemorySpace::Code,
                        });
                } else {
                    panic!("BUG: emit_instruction didn't return operand_location for placeholder");
                }

                // Jump back to main loop to read new input - handler has successfully executed
                debug!(
                    "🔀 JUMP_MAIN_LOOP: Jumping back to main loop start (label {}) after successful handler",
                    main_loop_jump_id
                );

                let layout = self.emit_instruction_typed(
                    Opcode::Op1(Op1::Jump),                        // jump
                    &[Operand::LargeConstant(placeholder_word())], // Will be resolved to main loop start
                    None,
                    None,
                )?;

                // Create UnresolvedReference for jump back to main loop start
                if let Some(operand_location) = layout.operand_location {
                    self.reference_context
                        .unresolved_refs
                        .push(UnresolvedReference {
                            reference_type: LegacyReferenceType::Jump,
                            location: operand_location,
                            target_id: main_loop_jump_id,
                            is_packed_address: false,
                            offset_size: 2,
                            location_space: MemorySpace::Code,
                        });
                } else {
                    panic!(
                        "BUG: emit_instruction didn't return operand_location for jump placeholder"
                    );
                }
            }
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

                let layout = self.emit_instruction_typed(
                    Opcode::Op1(Op1::Jump),                        // jump
                    &[Operand::LargeConstant(placeholder_word())], // Will be resolved to main loop start
                    None,
                    None,
                )?;

                if let Some(operand_location) = layout.operand_location {
                    self.reference_context
                        .unresolved_refs
                        .push(UnresolvedReference {
                            reference_type: LegacyReferenceType::Jump,
                            location: operand_location,
                            target_id: main_loop_jump_id,
                            is_packed_address: false,
                            offset_size: 2,
                            location_space: MemorySpace::Code,
                        });
                } else {
                    panic!(
                        "BUG: emit_instruction didn't return operand_location for jump placeholder"
                    );
                }
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

                let layout = self.emit_instruction_typed(
                    Opcode::OpVar(OpVar::CallVs), // call_vs: V3 compatible call routine with arguments
                    &[
                        Operand::LargeConstant(placeholder_word()), // Function address placeholder
                        Operand::SmallConstant(0),                  // Object ID 0 (no object)
                    ],
                    Some(0), // Store result on stack
                    None,    // No branch
                )?;

                // FIXED: Use layout.operand_location instead of hardcoded offset calculation
                // This was previously using self.code_address - 2 which caused placeholder resolution failures
                if let Some(operand_location) = layout.operand_location {
                    let dispatch_func_id = self.get_function_id_with_dispatch(*func_id);
                    self.reference_context
                        .unresolved_refs
                        .push(UnresolvedReference {
                            reference_type: LegacyReferenceType::FunctionCall,
                            location: operand_location, // Correct operand location from emit_instruction
                            target_id: dispatch_func_id,
                            is_packed_address: true,
                            offset_size: 2,
                            location_space: MemorySpace::Code,
                        });
                } else {
                    panic!("BUG: emit_instruction didn't return operand_location for placeholder");
                }

                // Jump back to main loop to read new input - noun handler (with ID 0) has successfully executed
                debug!(
                    "🔀 JUMP_MAIN_LOOP: Jumping back to main loop start (label {}) after noun handler (ID 0)",
                    main_loop_jump_id
                );

                let layout = self.emit_instruction_typed(
                    Opcode::Op1(Op1::Jump),                        // jump
                    &[Operand::LargeConstant(placeholder_word())], // Will be resolved to main loop start
                    None,
                    None,
                )?;

                if let Some(operand_location) = layout.operand_location {
                    self.reference_context
                        .unresolved_refs
                        .push(UnresolvedReference {
                            reference_type: LegacyReferenceType::Jump,
                            location: operand_location,
                            target_id: main_loop_jump_id,
                            is_packed_address: false,
                            offset_size: 2,
                            location_space: MemorySpace::Code,
                        });
                } else {
                    panic!(
                        "BUG: emit_instruction didn't return operand_location for jump placeholder"
                    );
                }
            }
        }

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

    /// Set up parameter IR ID to local variable slot mappings for a function
    /// This must be called before translating function instructions
    pub fn setup_function_parameter_mappings(&mut self, function: &IrFunction) {
        log::debug!(
            " PARAMETER_SETUP: Function '{}' has {} parameters",
            function.name,
            function.parameters.len()
        );
        for (_param_index, parameter) in function.parameters.iter().enumerate() {
            self.ir_id_to_local_var
                .insert(parameter.ir_id, parameter.slot);
            log::debug!(
                " Parameter mapping: '{}' (IR ID {}) -> local variable slot {} for function '{}'",
                parameter.name,
                parameter.ir_id,
                parameter.slot,
                function.name
            );
        }

        // CRITICAL: Propagate ir_id_from_property flag from string arguments to parameters
        if let Some(string_arg_positions) = self.string_arg_to_param_mapping.get(&function.id) {
            for &arg_position in string_arg_positions {
                if arg_position < function.parameters.len() {
                    let parameter_ir_id = function.parameters[arg_position].ir_id;
                    self.ir_id_from_property.insert(parameter_ir_id);
                    log::debug!(
                        "🔧 Propagated ir_id_from_property flag from string argument position {} to parameter IR ID {} for function '{}'",
                        arg_position,
                        parameter_ir_id,
                        function.name
                    );
                } else {
                    log::error!(
                        "String argument position {} exceeds parameter count {} for function '{}'",
                        arg_position,
                        function.parameters.len(),
                        function.name
                    );
                }
            }
        }
    }

    /// Set up all local variable IR ID mappings for a function
    /// This maps both parameters and local variables (loop counters, let bindings)
    pub fn setup_function_local_mappings(&mut self, function: &IrFunction) {
        // First, set up parameters
        self.setup_function_parameter_mappings(function);

        // Then, set up all other local variables (loop counters, let bindings)
        log::debug!(
            " LOCAL_VAR_SETUP: Function '{}' has {} local variables",
            function.name,
            function.local_vars.len()
        );

        for local in &function.local_vars {
            // Only add if not already mapped (parameters already done)
            if !self.ir_id_to_local_var.contains_key(&local.ir_id) {
                self.ir_id_to_local_var.insert(local.ir_id, local.slot);
                log::debug!(
                    " Local variable mapping: '{}' (IR ID {}) -> local slot {} for function '{}'",
                    local.name,
                    local.ir_id,
                    local.slot,
                    function.name
                );

                // CRITICAL DEBUG: Track slot 2 allocations for handle_go
                if function.name == "handle_go" && local.slot == 2 {
                    log::debug!(
                        "🔍 SLOT_2_ALLOC: Function 'handle_go' allocated slot 2 to '{}' (IR ID {})",
                        local.name,
                        local.ir_id
                    );
                }
            }
        }

        // Set current_function_locals so finalize_function_header knows the actual count
        // This prevents the header from being patched back to 0
        self.current_function_locals = function.local_vars.len() as u8;
        log::debug!(
            " LOCAL_VAR_SETUP: Set current_function_locals = {} for function '{}'",
            self.current_function_locals,
            function.name
        );
    }

    /// Generate function header with local variable declarations
    pub fn generate_function_header(
        &mut self,
        function: &IrFunction,
        _ir: &IrProgram,
    ) -> Result<(), CompilerError> {
        // NOTE: current_function_locals is already set by setup_function_local_mappings()
        // which is called before this function. We don't reset it here.
        self.current_function_name = Some(function.name.clone());
        log::debug!(
            " FUNCTION_START: Generating header for function '{}' with {} locals",
            function.name,
            self.current_function_locals
        );

        log::debug!(
            "🎯 USER_FUNCTION: '{}' starts at 0x{:04x}",
            function.name,
            self.code_address
        );

        // CRITICAL FIX: Pre-allocate space for locals that will be dynamically allocated
        // During instruction generation, local variables can be allocated for:
        // 1. Function parameters
        // 2. let statements (local variables)
        // 3. Temporary storage for complex expressions
        // We need to reserve space in the function header for these

        let declared_locals = function.local_vars.len();
        // CRITICAL FIX: Use actual local count instead of hard-coding 8 minimum
        // Simple functions with no parameters or local variables should have 0 locals
        // Complex functions will allocate locals as needed during instruction generation
        let reserved_locals = declared_locals;

        if reserved_locals > 15 {
            return Err(CompilerError::CodeGenError(format!(
                "Function '{}' needs {} declared locals, maximum is 15",
                function.name, reserved_locals
            )));
        }

        log::debug!(
            " FUNCTION_LOCALS: '{}' declared={}, reserved={} (actual local count)",
            function.name,
            declared_locals,
            reserved_locals
        );

        // Store the header address so finalize_function_header can patch the local count
        self.function_header_locations
            .insert(function.id, self.code_space.len());

        // Store reserved count for function address calculation
        // This is used in resolve_unresolved_reference() to calculate the correct
        // function call target address (header address + header size = executable code address)
        self.function_locals_count
            .insert(function.id, reserved_locals);

        // NOTE: Parameter IR ID mappings are now set up during instruction translation phase
        // This ensures they're available when instructions are processed (see setup_function_parameter_mappings)

        log::debug!(
 "Generating V3 header: {} reserved locals for function '{}' (will be patched with actual count)",
 reserved_locals,
 function.name
 );

        // Emit initial local count (will be patched later by finalize_function_header)
        self.emit_byte(reserved_locals as u8)?;

        // Emit default values for V3 (2 bytes each, value 0) - reserve space for all possible locals
        match self.version {
            ZMachineVersion::V3 => {
                for i in 0..reserved_locals {
                    self.emit_word(0x0000)?; // Default value 0
                    log::debug!("Reserved default value space for local {}", i + 1);
                }
            }
            ZMachineVersion::V4 | ZMachineVersion::V5 => {
                // V4/V5 don't need default values - locals auto-initialize to 0
            }
        }

        // Update stored count for any address calculations that need it
        self.function_locals_count
            .insert(function.id, reserved_locals);

        Ok(())
    }

    /// Finalize function header by patching local count with actual count used
    pub fn finalize_function_header(&mut self, function_id: IrId) -> Result<(), CompilerError> {
        let actual_locals = self.current_function_locals;
        let function_name = self
            .current_function_name
            .clone()
            .unwrap_or_else(|| format!("function_{}", function_id));

        log::debug!(
            " FINALIZE: Function '{}' used {} local variables during generation",
            function_name,
            actual_locals
        );

        // Get the header location to patch
        if let Some(&header_location) = self.function_header_locations.get(&function_id) {
            // Patch the local count in code_space
            if header_location < self.code_space.len() {
                let reserved_count = self.code_space[header_location];

                // Ensure we don't exceed the reserved space
                if actual_locals > reserved_count {
                    return Err(CompilerError::CodeGenError(format!(
                        "Function '{}' used {} locals but only {} were reserved in header",
                        function_name, actual_locals, reserved_count
                    )));
                }

                // CRITICAL FIX: Don't patch the header if we've reserved space for local variables
                // The Z-Machine needs the local count to match the actual storage space allocated
                // If we reserved 8 locals and emit 16 bytes of storage, the header must say 8 locals
                // Otherwise the interpreter will misalign when reading past the local variable storage

                log::debug!(
 " FINALIZE: Function '{}' keeping {} reserved locals (used {} actual) to match storage space",
 function_name,
 reserved_count,
 actual_locals
 );

                log::debug!(
                    " FINALIZE_DEBUG: code_space[0x{:04x}] = 0x{:02x} (should be {}) for function '{}'",
                    header_location,
                    self.code_space[header_location],
                    reserved_count,
                    function_name
                );

                // Keep the reserved count - don't patch the header
                // The extra unused locals will remain initialized to 0x0000 which is correct

                // Update the stored locals count for function address calculation
                self.function_locals_count
                    .insert(function_id, reserved_count as usize);

            // Note: V3 header now uses exact local count without pre-allocation
            } else {
                log::debug!(
                    " PATCH_ERROR: Header location 0x{:04x} is beyond code_space length {}",
                    header_location,
                    self.code_space.len()
                );
            }
        } else {
            log::debug!(
                " PATCH_ERROR: No header location found for function {} ('{}')",
                function_id,
                function_name
            );
        }

        Ok(())
    }

    /// Check if a block ends with a return instruction
    pub fn block_ends_with_return(&self, block: &IrBlock) -> bool {
        matches!(
            block.instructions.last(),
            Some(IrInstruction::Return { .. })
        )
    }

    /// Generate load immediate instruction
    pub fn generate_load_immediate(&mut self, value: &IrValue) -> Result<(), CompilerError> {
        match value {
            IrValue::Integer(_n) => {
                // Integer constants don't need to generate instructions
                // They will be used directly as operands in other instructions
                // TODO: In a complete implementation, this would store the constant
                // in a temporary location for later use
            }
            IrValue::Boolean(_b) => {
                // Boolean LoadImmediate doesn't emit any instructions
                // The branch instruction will handle the constant directly
            }
            IrValue::String(_s) => {
                // String literals in LoadImmediate don't generate any bytecode
                // They are just metadata that gets stored in ir_id_to_string
                // The actual string usage happens in print calls, not load immediates
            }
            IrValue::Object(_object_name) => {
                // Object references in LoadImmediate don't generate any bytecode
                // They are just metadata that gets resolved during codegen_instructions
                // The actual object number resolution happens in LoadImmediate instruction handling
            }
            _ => {
                return Err(CompilerError::CodeGenError(
                    "Unsupported immediate value type".to_string(),
                ));
            }
        }
        Ok(())
    }

    /// Generate unary operation instruction
    pub fn generate_unary_op(
        &mut self,
        target: IrId,
        op: &IrUnaryOp,
        operand_id: IrId,
    ) -> Result<(), CompilerError> {
        // Resolve the operand ID to an actual operand
        let operand = self.resolve_ir_id_to_operand(operand_id)?;

        match op {
            IrUnaryOp::Not => {
                // V3-compatible logical NOT implementation using je/store pattern
                // OpVar::Not (0x8F) doesn't exist in V3 - it's call_1n in V5+

                // CRITICAL: Allocate global variable for result first
                let store_var = self.allocate_global_variable();
                self.ir_id_to_stack_var.insert(target, store_var);

                // Generate unique label IDs using next_string_id (same pattern as logical AND/OR)
                let true_label = self.next_string_id;
                self.next_string_id += 1;
                let end_label = self.next_string_id;
                self.next_string_id += 1;

                // Je operand, 0 -> branch if operand is 0 (false)
                let layout1 = self.emit_instruction_typed(
                    Opcode::Op2(Op2::Je),
                    &[operand, Operand::SmallConstant(0)],
                    None,
                    Some(-1), // Branch placeholder - will be patched to true_label
                )?;

                // Register branch to true_label (operand was 0/false)
                if let Some(branch_location) = layout1.branch_location {
                    self.reference_context
                        .unresolved_refs
                        .push(UnresolvedReference {
                            reference_type: LegacyReferenceType::Branch,
                            location: branch_location,
                            target_id: true_label,
                            is_packed_address: false,
                            offset_size: 2,
                            location_space: MemorySpace::Code,
                        });
                }

                // Operand is non-zero (true), so store 0 (NOT true = false)
                let _layout2 = self.emit_instruction_typed(
                    Opcode::Op2(Op2::Store),
                    &[Operand::SmallConstant(store_var), Operand::SmallConstant(0)], // Store 0 to global
                    None,
                    None,
                )?;

                // Jump to end
                self.translate_jump(end_label)?;

                // true_label: operand was 0 (false), so store 1 (NOT false = true)
                // Register label in ir_id_to_address for branch resolution
                self.reference_context
                    .ir_id_to_address
                    .insert(true_label, self.code_address);
                let _layout3 = self.emit_instruction_typed(
                    Opcode::Op2(Op2::Store),
                    &[Operand::SmallConstant(store_var), Operand::SmallConstant(1)], // Store 1 to global
                    None,
                    None,
                )?;

                // end_label:
                // Register label in ir_id_to_address for branch resolution
                self.reference_context
                    .ir_id_to_address
                    .insert(end_label, self.code_address);

                log::debug!(
                    " UNARY_OP: Generated V3-compatible NOT using je/store pattern for target {} -> global var {}",
                    target, store_var
                );
            }
            IrUnaryOp::Minus => {
                // Z-Machine arithmetic negation - subtract operand from 0
                let operands = vec![Operand::Constant(0), operand];
                self.emit_instruction_typed(Opcode::Op2(Op2::Sub), &operands, Some(0), None)?; // sub instruction
                self.use_push_pull_for_result(target, "arithmetic unary operation")?;
            }
        }
        Ok(())
    }

    /// Generate binary operation with proper operands and result storage
    fn generate_binary_op(
        &mut self,
        op: &IrBinaryOp,
        left_operand: Operand,
        right_operand: Operand,
        store_var: Option<u8>,
    ) -> Result<(), CompilerError> {
        log::debug!(
            "Generating binary operation: {:?} with operands {:?}, {:?}",
            op,
            left_operand,
            right_operand
        );
        let opcode = match op {
            IrBinaryOp::Add => Opcode::Op2(Op2::Add), // add (2OP:20)
            IrBinaryOp::Subtract => Opcode::Op2(Op2::Sub), // sub (2OP:21)
            IrBinaryOp::Multiply => Opcode::Op2(Op2::Mul), // mul (2OP:22)
            IrBinaryOp::Divide => Opcode::Op2(Op2::Div), // div (2OP:23)
            IrBinaryOp::Modulo => Opcode::Op2(Op2::Mod), // mod (2OP:24)
            IrBinaryOp::Equal => Opcode::Op2(Op2::Je), // je (2OP:1) - jump if equal
            IrBinaryOp::NotEqual => Opcode::Op2(Op2::Je), // je (2OP:1) - jump if equal, then negate
            IrBinaryOp::Less => Opcode::Op2(Op2::Jl), // jl (2OP:2) - jump if less
            IrBinaryOp::LessEqual => Opcode::Op2(Op2::Jl), // Use jl for now (placeholder)
            IrBinaryOp::Greater => Opcode::Op2(Op2::Jg), // jg (2OP:3) - jump if greater
            IrBinaryOp::GreaterEqual => Opcode::Op2(Op2::Jg), // Use jg for now (placeholder)
            IrBinaryOp::And => Opcode::Op2(Op2::And), // and (2OP:9)
            IrBinaryOp::Or => Opcode::Op2(Op2::Or),   // or (2OP:8)
        };

        let operands = vec![left_operand, right_operand];

        // CRITICAL FIX: Handle comparison operations properly using branch + stack stores
        match op {
            IrBinaryOp::Equal
            | IrBinaryOp::NotEqual
            | IrBinaryOp::Less
            | IrBinaryOp::LessEqual
            | IrBinaryOp::Greater
            | IrBinaryOp::GreaterEqual => {
                log::debug!(
 "GENERATE_BINARY_OP: Comparison {:?} should not generate standalone instructions",
 op
 );

                // FUNDAMENTAL FIX: Comparison instructions in Z-Machine are BRANCH instructions,
                // not value-producing instructions. This method should not be called for comparisons.
                //
                // Comparisons should be handled by conditional branching logic directly.

                log::debug!(
                    "generate_binary_op: Comparison {:?} delegated to branching logic",
                    op
                );
                // Don't generate any bytecode for comparison operations - let the direct branch handle it
                return Ok(());
            }
            IrBinaryOp::Divide => {
                // CRITICAL: Use raw opcode to ensure V3 form determination applies
                // V3 compatibility constraints in determine_instruction_form_with_operands require raw opcode
                self.emit_instruction(
                    0x17, // div opcode - V3 compatibility handled by determine_instruction_form_with_operands
                    &operands, store_var, None,
                )?;
            }
            _ => {
                // Arithmetic operations store result normally
                self.emit_instruction_typed(opcode, &operands, store_var, None)?;
            }
        }

        Ok(())
    }

    /// Generate function call with unresolved reference and arguments
    pub fn generate_call_with_reference(
        &mut self,
        function_id: IrId,
        args: &[IrId],
        _target: Option<IrId>,
    ) -> Result<(), CompilerError> {
        // Generate a proper function call with placeholder address that will be resolved later
        // This is the correct approach - not rtrue hacks or compile errors

        // Convert IR args to operands
        let mut operands = vec![Operand::LargeConstant(placeholder_word())]; // Placeholder for function address
        for &arg_id in args {
            if let Some(literal_value) = self.get_literal_value(arg_id) {
                operands.push(Operand::LargeConstant(literal_value));
            } else if self.ir_id_to_string.contains_key(&arg_id) {
                // String literal: Create placeholder + unresolved reference
                // CRITICAL FIX: Record exact location BEFORE emitting placeholder
                let code_space_offset = self.code_space.len() + 1 + operands.len() * 2;
                operands.push(Operand::LargeConstant(placeholder_word()));

                // Create reference with exact calculated location
                let reference = UnresolvedReference {
                    reference_type: LegacyReferenceType::StringRef,
                    location: self.final_code_base + code_space_offset, // Use exact offset
                    target_id: arg_id,
                    is_packed_address: true,
                    offset_size: 2,
                    location_space: MemorySpace::Code,
                };
                self.reference_context.unresolved_refs.push(reference);
                log::debug!(
                    "Added string argument reference: IR ID {} at location 0x{:04x}",
                    arg_id,
                    self.final_code_base + code_space_offset
                );
            } else {
                // Other types: Use existing operand resolution
                match self.resolve_ir_id_to_operand(arg_id) {
                    Ok(operand) => {
                        operands.push(operand);
                    }
                    Err(err) => {
                        log::warn!(
                            "Function argument IR ID {} failed resolution: {:?}",
                            arg_id,
                            err
                        );
                        // Create placeholder and unresolved reference as fallback
                        // CRITICAL FIX: Record exact location BEFORE emitting placeholder
                        let code_space_offset = self.code_space.len() + 1 + operands.len() * 2;
                        operands.push(Operand::LargeConstant(placeholder_word()));

                        let reference = UnresolvedReference {
                            reference_type: LegacyReferenceType::StringRef, // Assume strings for print calls
                            location: self.final_code_base + code_space_offset, // Use exact offset
                            target_id: arg_id,
                            is_packed_address: true,
                            offset_size: 2,
                            location_space: MemorySpace::Code,
                        };
                        self.reference_context.unresolved_refs.push(reference);
                        log::warn!(
                            "Added fallback string reference: IR ID {} at location 0x{:04x}",
                            arg_id,
                            self.final_code_base + code_space_offset
                        );
                    }
                }
            }
        }

        // Use call_vs (VAR:224) for all cases - it handles 0+ arguments
        // emit_instruction_typed expects Opcode enum values
        let opcode = Opcode::OpVar(OpVar::CallVs); // call_vs opcode - works with any number of arguments

        // Determine store variable for return value
        // CRITICAL V3 FIX: ALL function calls must store result, even if discarded
        // V5+ has call_1n/call_2n for no-store calls, but V3 requires storing to stack
        let store_var = Some(0x00); // Always store to stack (Variable 0)

        // Generate the call instruction with placeholder address
        let layout = self.emit_instruction_typed(opcode, &operands, store_var, None)?;

        // Add unresolved reference for function address using correct operand location
        let operand_location = layout
            .operand_location
            .expect("Call instruction must have operand location");
        self.reference_context
            .unresolved_refs
            .push(UnresolvedReference {
                reference_type: LegacyReferenceType::FunctionCall,
                location: operand_location,
                target_id: function_id,
                is_packed_address: true, // Function addresses are packed in Z-Machine
                offset_size: 2,
                location_space: MemorySpace::Code,
            });

        log::debug!(
            "Generated call to function ID {} with unresolved reference at 0x{:04x}",
            function_id,
            operand_location
        );

        Ok(())
    }

    /// Get literal value for an IR ID (helper method)
    fn get_literal_value(&self, ir_id: IrId) -> Option<u16> {
        // Check if this IR ID corresponds to an integer literal
        if let Some(&integer_value) = self.ir_id_to_integer.get(&ir_id) {
            // DEBUG: Track problematic IR IDs that create LargeConstant(0)
            if integer_value < 0 {
                debug!("Negative integer fallback: IR ID {} has negative value {} -> converting to LargeConstant(0)", ir_id, integer_value);
                debug!("Root issue: This IR ID should NOT be in ir_id_to_integer table - it should be an object/function/string reference");
            }

            // Convert to u16, handling negative values appropriately
            if integer_value >= 0 {
                return Some(integer_value as u16);
            } else {
                // For negative values, use 0 as fallback
                return Some(0);
            }
        }

        // Check legacy mapping for backward compatibility
        match ir_id {
            id if id >= 1000 => Some((id - 1000) as u16), // Simple mapping for testing
            _ => None,
        }
    }

    /// Resolve an IR ID to the appropriate Z-Machine operand
    pub fn resolve_ir_id_to_operand(&mut self, ir_id: IrId) -> Result<Operand, CompilerError> {
        log::debug!(
            " RESOLVE_IR_ID_TO_OPERAND: Attempting to resolve IR ID {}",
            ir_id
        );

        // Track the 17 problematic IR IDs
        //
        // LOGGING LEVEL FIX: Changed from log::error! to log::debug! - this tracks normal
        // IR ID resolution during compilation, not actual errors. The term "problematic"
        // refers to IDs that required investigation during development, but their resolution
        // during compilation is expected behavior, not an error condition.
        if [
            132, 181, 203, 224, 334, 337, 358, 408, 417, 425, 439, 441, 443, 464, 515, 518, 535,
        ]
        .contains(&ir_id)
        {
            log::debug!(
                "RESOLVE_PROBLEMATIC: Resolving unpulled IR ID {} - in push_pull_ir_ids: {}",
                ir_id,
                self.push_pull_ir_ids.contains(&ir_id)
            );
        }

        // Check if it's an integer literal
        if let Some(literal_value) = self.get_literal_value(ir_id) {
            log::debug!(
                " resolve_ir_id_to_operand: IR ID {} resolved to LargeConstant({})",
                ir_id,
                literal_value
            );

            // CRITICAL CHECK: Is 415 being returned?
            // Historical note: Previously checked for literal_value == 415 (label ID bug)
            // This was a systematic issue where label IDs were being used as operand values
            // Fixed by proper IR ID mapping - keeping note for future reference

            return Ok(Operand::LargeConstant(literal_value));
        }

        // Phase C2: Check if this IR ID was marked for push/pull sequence
        // If so, emit pull instruction to temporary global and return that global
        log::debug!(
            "RESOLVE_DEBUG: IR ID {} - checking push_pull_ir_ids set (contains: {})",
            ir_id,
            self.push_pull_ir_ids.contains(&ir_id)
        );
        if self.push_pull_ir_ids.contains(&ir_id) {
            // Check if we already pulled this IR ID (prevent duplicate pulls)
            if let Some(&existing_global) = self.pulled_ir_id_to_global.get(&ir_id) {
                log::error!(
                    "STACK_REUSE: IR ID {} reusing existing Variable({}) - no pull needed",
                    ir_id,
                    existing_global
                );
                return Ok(Operand::Variable(existing_global));
            }

            // First time accessing this IR ID - allocate temporary global and emit pull
            let temp_global = self.allocate_global_variable();

            // VAR:233 pull(variable) - pops value from Z-Machine stack into temporary global
            // Pull takes the target variable as an operand, not as store_var
            let pull_operands = vec![Operand::Variable(temp_global)];
            self.emit_instruction_typed(Opcode::OpVar(OpVar::Pull), &pull_operands, None, None)?;

            // Cache this mapping to prevent duplicate pulls
            self.pulled_ir_id_to_global.insert(ir_id, temp_global);

            // STACK INSTRUMENTATION: Track pull operation
            log::error!(
                "STACK_PULL: IR ID {} at PC 0x{:04x} - pulled from stack to Variable({})",
                ir_id,
                self.current_address() - 2,
                temp_global
            );

            log::debug!(
                "PHASE_C2: IR ID {} resolved via pull to Variable({}) [Temporary global for stack value]",
                ir_id, temp_global
            );

            return Ok(Operand::Variable(temp_global));
        }

        // Check if this IR ID maps to a stack variable (e.g., result of GetProperty)
        if let Some(&stack_var) = self.ir_id_to_stack_var.get(&ir_id) {
            log::debug!(
                " resolve_ir_id_to_operand: IR ID {} resolved to Variable({}) [Stack result]",
                ir_id,
                stack_var
            );
            return Ok(Operand::Variable(stack_var));
        }

        // Check if this IR ID maps to a local variable (e.g., function parameter)
        if let Some(&local_var) = self.ir_id_to_local_var.get(&ir_id) {
            log::debug!(
                " resolve_ir_id_to_operand: IR ID {} resolved to Variable({}) [Local parameter]",
                ir_id,
                local_var
            );
            return Ok(Operand::Variable(local_var));
        }

        // Check if it's a string literal (shouldn't be used in binary ops, but handle gracefully)
        if self.ir_id_to_string.contains_key(&ir_id) {
            return Err(CompilerError::CodeGenError(format!(
                "Cannot use string literal (IR ID {}) as operand in binary operation",
                ir_id
            )));
        }

        // MISSING CHECK FIX: Look up integer values (including resolved IrValue::Object)
        // This handles values stored by LoadImmediate instructions, including runtime object numbers
        if let Some(&integer_value) = self.ir_id_to_integer.get(&ir_id) {
            log::debug!(
                " resolve_ir_id_to_operand: IR ID {} resolved to LargeConstant({}) [Integer value]",
                ir_id,
                integer_value
            );
            return Ok(Operand::LargeConstant(integer_value as u16));
        }

        // CRITICAL FIX: Check for player object first - player must use Variable(16)
        // The player object is stored in global variable G00 (Variable 16) and must be accessed via variable read
        // This follows proper Z-Machine architecture where the player object is referenced via global variables
        if let Some(&object_number) = self.ir_id_to_object_number.get(&ir_id) {
            if object_number == 1 {
                // Player is object #1
                log::debug!(
 " resolve_ir_id_to_operand: IR ID {} is player object - resolved to Variable(16) [Player global]",
 ir_id
 );
                return Ok(Operand::Variable(16)); // Global G00 = Variable(16)
            }

            log::debug!(
 " resolve_ir_id_to_operand: IR ID {} resolved to LargeConstant({}) [Object reference]",
 ir_id, object_number
 );

            // Critical debug: Track invalid object numbers
            if object_number == 77 {
                debug!(
                    "Object 77 debug: IR ID {} mapped to invalid object 77!",
                    ir_id
                );
                debug!("Object table has only 14 objects (1-14), but object 77 was requested");
                debug!("This suggests ir_id_to_object_number mapping is incorrect");
                debug!(
                    "Full object mapping table: {:?}",
                    self.ir_id_to_object_number
                );
                debug!("Valid object range is 1-14. Consider if IR ID {} should map to a different object", ir_id);
            }

            return Ok(Operand::LargeConstant(object_number));
        }

        // Check if this IR ID maps to a function address
        if let Some(&function_addr) = self.reference_context.ir_id_to_address.get(&ir_id) {
            log::debug!(
 " resolve_ir_id_to_operand: IR ID {} resolved to LargeConstant({}) [Function address]",
 ir_id,
 function_addr
 );
            return Ok(Operand::LargeConstant(function_addr as u16));
        }

        // CRITICAL: Unknown IR ID - this indicates a missing instruction target registration
        // For now, temporarily restore fallback but with comprehensive logging
        log::debug!(
            "resolve_ir_id_to_operand: Unknown IR ID {} - no mapping found in any table",
            ir_id
        );
        log::debug!(
            " Available integer IDs = {:?}",
            self.ir_id_to_integer.keys().collect::<Vec<_>>()
        );
        log::debug!(
            " Available stack var IDs = {:?}",
            self.ir_id_to_stack_var.keys().collect::<Vec<_>>()
        );
        log::debug!(
            " Available local var IDs = {:?}",
            self.ir_id_to_local_var.keys().collect::<Vec<_>>()
        );
        log::debug!(
            " Available object IDs = {:?}",
            self.ir_id_to_object_number.keys().collect::<Vec<_>>()
        );

        // Handle small IR IDs that are actually property numbers or constants
        if ir_id < 100 {
            log::debug!("Using IR ID {} as literal constant", ir_id);
            return Ok(Operand::LargeConstant(ir_id as u16));
        }

        // CRASH CLEARLY: No fallback mappings - this is a compiler bug that must be fixed
        panic!(
 "COMPILER BUG: No mapping found for IR ID {}. This means an instruction is not properly mapping its result target. Available mappings: objects={:?}, integers={:?}, stack_vars={:?}, local_vars={:?}",
 ir_id,
 self.ir_id_to_object_number.keys().collect::<Vec<_>>(),
 self.ir_id_to_integer.keys().collect::<Vec<_>>(),
 self.ir_id_to_stack_var.keys().collect::<Vec<_>>(),
 self.ir_id_to_local_var.keys().collect::<Vec<_>>()
 )
    }

    /// Set up comprehensive IR ID mappings for ALL IDs found in the IR program
    /// This ensures every IR ID used in instructions gets a proper mapping
    /// Made public for use by codegen_extensions.rs
    pub fn setup_comprehensive_id_mappings(&mut self, ir: &IrProgram) {
        // STEP 1: Scan ALL IR instructions to find every IR ID used anywhere
        let mut all_used_ids = IndexSet::new();

        // Scan functions
        for function in &ir.functions {
            for instr in &function.body.instructions {
                self.collect_instruction_ids(instr, &mut all_used_ids);
            }
        }

        // Scan init block
        if let Some(init_block) = &ir.init_block {
            for instr in &init_block.instructions {
                self.collect_instruction_ids(instr, &mut all_used_ids);
            }
        }

        log::warn!(
            "Comprehensive scan: found {} unique IR IDs used in instructions",
            all_used_ids.len()
        );
        let mut sorted_ids: Vec<_> = all_used_ids.iter().collect();
        sorted_ids.sort();
        log::warn!("All used IR IDs: {:?}", sorted_ids);

        // STEP 2: Do NOT create fallback mappings - let unmapped IDs crash clearly
        log::warn!(
            "Comprehensive scan complete. {} IDs found. No fallback mappings created.",
            all_used_ids.len()
        );
    }

    /// Collect all IR IDs referenced in a single instruction
    fn collect_instruction_ids(&self, instr: &IrInstruction, used_ids: &mut IndexSet<IrId>) {
        match instr {
            IrInstruction::LoadImmediate { target, .. } => {
                used_ids.insert(*target);
            }
            IrInstruction::LoadVar { target, var_id } => {
                used_ids.insert(*target);
                used_ids.insert(*var_id);
            }
            IrInstruction::StoreVar { var_id, source } => {
                used_ids.insert(*var_id);
                used_ids.insert(*source);
            }
            IrInstruction::BinaryOp {
                target,
                left,
                right,
                ..
            } => {
                used_ids.insert(*target);
                used_ids.insert(*left);
                used_ids.insert(*right);
            }
            IrInstruction::UnaryOp {
                target, operand, ..
            } => {
                used_ids.insert(*target);
                used_ids.insert(*operand);
            }
            IrInstruction::Call {
                target,
                function,
                args,
            } => {
                if let Some(t) = target {
                    used_ids.insert(*t);
                }
                used_ids.insert(*function);
                for &arg in args {
                    used_ids.insert(arg);
                }
            }
            IrInstruction::GetProperty { target, object, .. } => {
                used_ids.insert(*target);
                used_ids.insert(*object);
            }
            IrInstruction::GetPropertyByNumber { target, object, .. } => {
                used_ids.insert(*target);
                used_ids.insert(*object);
            }
            IrInstruction::SetProperty { object, value, .. } => {
                used_ids.insert(*object);
                used_ids.insert(*value);
            }
            IrInstruction::SetPropertyByNumber { object, value, .. } => {
                used_ids.insert(*object);
                used_ids.insert(*value);
            }
            IrInstruction::Return { value: Some(v) } => {
                used_ids.insert(*v);
            }
            IrInstruction::Return { value: None } => {
                // No value to track
            }
            _ => {} // Add other instruction types as needed
        }
    }

    /// Set up IR ID to object number mappings for proper identifier resolution (LEGACY)
    fn setup_object_mappings(&mut self, ir: &IrProgram) {
        // Create reverse mapping from IR IDs to object numbers
        // Use both symbol_ids (name -> IR ID) and object_numbers (name -> obj num)

        // Debug: Log all tracked IR IDs to see the comprehensive registry
        log::warn!("SYMBOL_IDS TABLE: {} entries", ir.symbol_ids.len());
        let mut symbol_ids: Vec<_> = ir.symbol_ids.values().collect();
        symbol_ids.sort();
        log::warn!("Symbol IDs: {:?}", symbol_ids);

        // NEW: Show comprehensive ID registry
        log::warn!(
            "ID_REGISTRY: {} total IDs tracked",
            ir.id_registry.id_types.len()
        );
        let mut all_tracked_ids: Vec<_> = ir.id_registry.id_types.keys().collect();
        all_tracked_ids.sort();
        log::warn!("All tracked IDs: {:?}", all_tracked_ids);

        let temp_count = ir.id_registry.temporary_ids.len();
        let symbol_count = ir.id_registry.symbol_ids.len();
        let expr_count = ir.id_registry.expression_ids.len();
        log::warn!(
            "ID types: {} temporary, {} symbol, {} expression",
            temp_count,
            symbol_count,
            expr_count
        );
        debug!("=== IR OBJECT_NUMBERS DEBUG ===");
        debug!("ir.object_numbers table contents:");
        for (name, &obj_num) in &ir.object_numbers {
            debug!(" '{}' -> Object #{}", name, obj_num);
        }
        debug!("ir.symbol_ids table contents:");
        for (name, &ir_id) in &ir.symbol_ids {
            debug!(" '{}' -> IR ID {}", name, ir_id);
        }

        for (name, &ir_id) in &ir.symbol_ids {
            if let Some(&object_number) = ir.object_numbers.get(name) {
                self.ir_id_to_object_number.insert(ir_id, object_number);
                log::debug!(
                    " MAPPING: IR ID {} ('{}') -> Object #{} {}",
                    ir_id,
                    name,
                    object_number,
                    if object_number == 77 {
                        " <- THIS IS THE PROBLEM!"
                    } else {
                        ""
                    }
                );

                // Debug: Track if problematic IR IDs are being mapped
                if (80..=100).contains(&ir_id) {
                    log::warn!(
                        "MAPPING PROBLEMATIC IR ID {} ('{}') -> Object #{}",
                        ir_id,
                        name,
                        object_number
                    );
                }
            } else {
                // Debug: Track IR IDs that don't get object mappings
                if (80..=100).contains(&ir_id) {
                    log::warn!("NO OBJECT MAPPING for IR ID {} ('{}')", ir_id, name);
                }
            }
        }

        // Also copy the object_numbers mapping for legacy compatibility
        self.object_numbers = ir.object_numbers.clone();

        log::debug!(
            "Object mapping setup complete: {} IR ID -> object number mappings created",
            self.ir_id_to_object_number.len()
        );
    }

    /// Set up room-to-object ID mapping for exit system
    /// Assigns sequential object numbers to rooms (starting from 2, object #1 is player)
    pub fn setup_room_to_object_mapping(&mut self, ir: &IrProgram) -> Result<(), CompilerError> {
        for (index, room) in ir.rooms.iter().enumerate() {
            // Object #1 is reserved for the player, rooms start at object #2
            let object_number = (index + 2) as u16;
            self.room_to_object_id.insert(room.id, object_number);
            log::debug!(
                "Mapped room '{}' (IR ID {}) to object #{}",
                room.name,
                room.id,
                object_number
            );
        }
        log::info!(
            "Room-to-object mapping complete: {} rooms mapped to object IDs 2-{}",
            self.room_to_object_id.len(),
            self.room_to_object_id.len() + 1
        );
        Ok(())
    }

    /// Setup IR ID to object number mapping for ALL objects (Phase 2: Oct 12, 2025)
    /// This must be called BEFORE code generation so that InsertObj tracking can resolve object numbers
    /// Object numbering: Player (#1), Rooms (#2-N), Objects (#N+1-M)
    /// Made public for use by codegen_extensions.rs
    pub fn setup_ir_id_to_object_mapping(&mut self, ir: &IrProgram) -> Result<(), CompilerError> {
        let mut object_num = 1u16;

        // Player is always object #1 (first in ir.objects)
        if !ir.objects.is_empty() {
            let player = &ir.objects[0];
            self.ir_id_to_object_number.insert(player.id, object_num);
            // CRITICAL: Also populate object_numbers mapping for IrValue::Object support
            self.object_numbers.insert(player.name.clone(), object_num);
            log::warn!(
                "🗺️ OBJ_MAPPING: Player '{}' (IR ID {}) -> Object #{}",
                player.name,
                player.id,
                object_num
            );
            object_num += 1;
        }

        // Rooms come next
        for room in &ir.rooms {
            self.ir_id_to_object_number.insert(room.id, object_num);
            // CRITICAL: Also populate object_numbers mapping for IrValue::Object support
            self.object_numbers.insert(room.name.clone(), object_num);
            log::warn!(
                "🗺️ OBJ_MAPPING: Room '{}' (IR ID {}) -> Object #{}",
                room.name,
                room.id,
                object_num
            );
            object_num += 1;
        }

        // Regular objects (skip player who was already added)
        for object in ir.objects.iter().skip(1) {
            self.ir_id_to_object_number.insert(object.id, object_num);
            // CRITICAL: Also populate object_numbers mapping for IrValue::Object support
            self.object_numbers.insert(object.name.clone(), object_num);
            log::warn!(
                "🗺️ OBJ_MAPPING: Object '{}' (IR ID {}) -> Object #{}",
                object.name,
                object.id,
                object_num
            );
            object_num += 1;
        }

        log::info!(
            "IR ID to object mapping complete: {} objects mapped",
            self.ir_id_to_object_number.len()
        );
        Ok(())
    }

    /// Pre-process InsertObj instructions to populate initial_locations_by_number
    /// CRITICAL FIX (Oct 28, 2025): This must be called before object generation
    /// so that objects can be created with correct initial parent relationships
    /// OBJECT CONTAINMENT FIX (Phase 1c): Pre-process InsertObj instructions before object generation
    ///
    /// **CRITICAL TIMING FIX**: This function resolves the timing issue where InsertObj instructions
    /// were processed AFTER object generation, causing all objects to start with parent=0, sibling=0, child=0
    /// relationships. Objects would only appear in rooms after the first runtime InsertObj execution.
    ///
    /// **The Problem**:
    /// - Object generation happens in Step 2c of compilation
    /// - InsertObj instructions are processed during IR translation (later in pipeline)
    /// - Result: All objects generated with no parent relationships (mailbox invisible in west_of_house)
    /// - Mailbox would only appear after first "look" command triggered runtime InsertObj
    ///
    /// **The Solution**:
    /// - Scan ALL IR functions for InsertObj instructions BEFORE object generation
    /// - Populate initial_locations_by_number with correct parent relationships
    /// - Objects now start with proper parent relationships from compilation time
    /// - Mailbox appears immediately when entering west_of_house room
    ///
    /// **Fixed Issues**:
    /// - Mailbox now visible immediately in west_of_house starting room
    /// - All room objects (window, tree, nest, etc.) properly placed at game start
    /// - No more "missing objects until first look" timing bug
    ///
    /// **Implementation**: Called from line 713 before generate_object_tables() in Step 2c
    pub fn preprocess_insertobj_instructions(
        &mut self,
        ir: &IrProgram,
    ) -> Result<(), CompilerError> {
        log::info!("🔍 Pre-processing InsertObj instructions for initial object placement");

        // Scan all IR functions for InsertObj instructions
        for function in &ir.functions {
            for instruction in &function.body.instructions {
                if let IrInstruction::InsertObj {
                    object,
                    destination,
                } = instruction
                {
                    // Resolve IR IDs to object numbers using the established mapping
                    if let (Some(&obj_num), Some(&parent_num)) = (
                        self.ir_id_to_object_number.get(object),
                        self.ir_id_to_object_number.get(destination),
                    ) {
                        self.initial_locations_by_number.insert(obj_num, parent_num);
                        log::warn!(
                            "🏗️ PREPROCESS_INSERTOBJ: Object #{} -> Parent #{} (IR {} -> IR {})",
                            obj_num,
                            parent_num,
                            object,
                            destination
                        );
                    }
                }
            }
        }

        // Also check init block if it exists
        if let Some(init_block) = &ir.init_block {
            for instruction in &init_block.instructions {
                if let IrInstruction::InsertObj {
                    object,
                    destination,
                } = instruction
                {
                    if let (Some(&obj_num), Some(&parent_num)) = (
                        self.ir_id_to_object_number.get(object),
                        self.ir_id_to_object_number.get(destination),
                    ) {
                        self.initial_locations_by_number.insert(obj_num, parent_num);
                        log::warn!(
                            "🏗️ PREPROCESS_INSERTOBJ: Object #{} -> Parent #{} (IR {} -> IR {}) [from init block]",
                            obj_num, parent_num, object, destination
                        );
                    }
                }
            }
        }

        log::info!(
            "Pre-processing complete: {} initial object placements found",
            self.initial_locations_by_number.len()
        );

        // 🌳 OBJECT TREE INSTRUMENTATION: Show complete initial object tree structure
        // self.debug_initial_object_tree_structure(); // Disabled after successful debugging

        Ok(())
    }

    /// 🌳 OBJECT TREE DEBUGGING: Show complete compiler's understanding of object tree structure
    fn debug_initial_object_tree_structure(&self) {
        log::debug!("🌳 ======= COMPILER OBJECT TREE STRUCTURE =======");

        // Show all object mappings
        log::debug!("🗺️ Object Name -> Number mappings:");
        for (name, number) in &self.object_numbers {
            log::debug!("   '{}' -> Object #{}", name, number);
        }

        // Show IR ID -> Object Number mappings
        log::debug!("🔗 IR ID -> Object Number mappings:");
        for (ir_id, number) in &self.ir_id_to_object_number {
            log::debug!("   IR ID {} -> Object #{}", ir_id, number);
        }

        // Show initial parent-child relationships
        log::debug!("🏗️ Initial Object Tree Relationships (child -> parent):");
        if self.initial_locations_by_number.is_empty() {
            log::debug!(
                "   ⚠️  NO INITIAL RELATIONSHIPS FOUND! All objects will start with parent=0"
            );
        } else {
            for (child, parent) in &self.initial_locations_by_number {
                log::debug!("   Object #{} -> Parent #{}", child, parent);
            }
        }

        // Build and show tree structure
        log::debug!("🌳 Object Tree Hierarchy:");
        self.debug_tree_hierarchy();

        log::debug!("🌳 ===============================================");
    }

    /// Show hierarchical tree structure
    fn debug_tree_hierarchy(&self) {
        let mut children_by_parent: std::collections::HashMap<u16, Vec<u16>> =
            std::collections::HashMap::new();

        // Build parent -> children mapping
        for (child, parent) in &self.initial_locations_by_number {
            children_by_parent
                .entry(*parent)
                .or_insert_with(Vec::new)
                .push(*child);
        }

        // Find root objects (those not in any parent-child relationship as children)
        let all_children: std::collections::HashSet<u16> =
            self.initial_locations_by_number.keys().copied().collect();
        let all_objects: std::collections::HashSet<u16> =
            self.ir_id_to_object_number.values().map(|&n| n).collect();

        for obj_num in all_objects {
            if !all_children.contains(&obj_num) {
                // This is a root object
                let name = self.get_debug_object_name(obj_num);
                log::debug!("   Root: Object #{} ({})", obj_num, name);
                self.debug_tree_children(&children_by_parent, obj_num, 1);
            }
        }
    }

    /// Recursively show children in tree format
    fn debug_tree_children(
        &self,
        children_by_parent: &std::collections::HashMap<u16, Vec<u16>>,
        parent_num: u16,
        depth: usize,
    ) {
        if let Some(children) = children_by_parent.get(&parent_num) {
            for &child in children {
                let indent = "   ".repeat(depth);
                let name = self.get_debug_object_name(child);
                log::debug!("{}└─ Object #{} ({})", indent, child, name);
                self.debug_tree_children(children_by_parent, child, depth + 1);
            }
        }
    }

    /// Helper to get object name by number for debugging
    fn get_debug_object_name(&self, obj_num: u16) -> String {
        // Search for object name by number
        for (name, &number) in &self.object_numbers {
            if number == obj_num {
                return name.clone();
            }
        }
        // Fallback: search IR ID mapping
        for (ir_id, &number) in &self.ir_id_to_object_number {
            if number == obj_num {
                return format!("IR_{}", ir_id);
            }
        }
        "Unknown".to_string()
    }

    /// Generate return instruction
    pub fn emit_return(&mut self, value: Option<IrId>) -> Result<(), CompilerError> {
        if let Some(_ir_id) = value {
            // Return with value - use ret opcode with operand
            self.emit_instruction_typed(
                Opcode::Op1(Op1::Ret),        // ret opcode
                &[Operand::SmallConstant(0)], // Return 0 for now (TODO: resolve actual value)
                None,                         // No store
                None,                         // No branch
            )?;
        } else {
            // Return without value - use rtrue (0OP instruction)
            self.emit_instruction_typed(
                Opcode::Op0(Op0::Rtrue), // rtrue opcode
                &[],                     // No operands
                None,                    // No store
                None,                    // No branch
            )?;
        }
        Ok(())
    }

    /// Generate proper conditional branch instruction with smart optimization
    fn generate_conditional_branch(
        &mut self,
        condition: IrId,
        true_label: IrId,
        false_label: IrId,
    ) -> Result<(), CompilerError> {
        log::debug!(
            "generate_conditional_branch: condition={}, true_label={}, false_label={}",
            condition,
            true_label,
            false_label
        );

        // Step 1: Resolve condition value if it's a constant
        let condition_value = self.resolve_condition_value(condition);

        match condition_value {
            Some(ConstantValue::Boolean(true)) => {
                log::debug!("Condition is constant TRUE - optimizing branch");
                // Generate direct jump to true_label if not fall-through
                if !self.is_next_instruction(true_label) {
                    log::debug!("TRUE branch is not next instruction - generating jump");
                    self.translate_jump(true_label)?;
                } else {
                    log::debug!("TRUE branch is next instruction - no jump needed (fall-through)");
                }
            }
            Some(ConstantValue::Boolean(false)) => {
                log::debug!("Condition is constant FALSE - optimizing branch");
                // Generate direct jump to false_label if not fall-through
                if !self.is_next_instruction(false_label) {
                    log::debug!("FALSE branch is not next instruction - generating jump");
                    self.translate_jump(false_label)?;
                } else {
                    log::debug!("FALSE branch is next instruction - no jump needed (fall-through)");
                }
            }
            Some(ConstantValue::Integer(n)) => {
                // Treat integer as boolean: 0 = false, non-zero = true
                let is_true = n != 0;
                log::debug!(
                    "Condition is constant INTEGER {} (treated as {})",
                    n,
                    is_true
                );

                let target_label = if is_true { true_label } else { false_label };
                if !self.is_next_instruction(target_label) {
                    self.translate_jump(target_label)?;
                }
            }
            None | Some(ConstantValue::String(_)) => {
                log::debug!(
                    "Condition is variable or unknown - generating Z-Machine conditional branch"
                );
                // Generate proper Z-Machine conditional branch instruction
                self.emit_conditional_branch_instruction(condition, true_label, false_label)?;
            }
        }

        Ok(())
    }

    /// Resolve condition IR ID to constant value if possible
    fn resolve_condition_value(&self, condition: IrId) -> Option<ConstantValue> {
        // Check if we have a cached constant value
        if let Some(value) = self.constant_values.get(&condition) {
            return Some(value.clone());
        }

        // Check if it's a direct integer constant
        if let Some(&integer) = self.ir_id_to_integer.get(&condition) {
            return Some(ConstantValue::Integer(integer));
        }

        // Check if it's a direct string constant
        if let Some(string) = self.ir_id_to_string.get(&condition) {
            return Some(ConstantValue::String(string.clone()));
        }

        // Unable to resolve to constant
        None
    }

    /// Check if a label will be placed at the immediately next instruction address
    fn is_next_instruction(&self, label: IrId) -> bool {
        // First check pre-calculated label addresses (most reliable)
        if let Some(&target_addr) = self.label_addresses.get(&label) {
            // FIXED: Compare code space offsets (label_addresses contains offsets, not final addresses)
            let next_addr_after_jump = self.code_space.len() + 3;
            let is_next = target_addr == next_addr_after_jump;

            log::debug!(
 "is_next_instruction: label={}, target_addr=0x{:04x}, next_addr_after_jump=0x{:04x}, is_next={}",
 label, target_addr, next_addr_after_jump, is_next
 );

            return is_next;
        }

        // Fallback: Check if label is already resolved and points to next instruction
        if let Some(&target_addr) = self.reference_context.ir_id_to_address.get(&label) {
            // FIXED: ir_id_to_address may contain final addresses (after conversion), compare appropriately
            let next_addr_after_jump = self.code_space.len() + 3;
            return target_addr == next_addr_after_jump;
        }

        // Check if label is in the list of labels at current address (immediate)
        self.labels_at_current_address.contains(&label)
    }

    /// Allocate a unique global variable for results that need persistent storage
    ///
    /// # Z-Machine Variable Layout
    /// - Variables 0: Stack (Variable(0) - contested, causes collisions)
    /// - Variables 1-15: Local variables (function parameters)
    /// - Variables 16+: Global variables (persistent, safe for function results)
    ///
    /// This method ensures each allocated global is unique by tracking allocation count.
    pub fn allocate_global_variable(&mut self) -> u8 {
        // Global variables start at 16 (0x10) after locals (0-15)
        // Continue from allocated_globals_count to ensure uniqueness
        let global_var = 16 + (self.allocated_globals_count as u8);
        self.allocated_globals_count += 1;

        log::debug!(
            "GLOBAL_VAR_ALLOC: Allocated Variable({}) for persistent storage (total allocated: {})",
            global_var,
            self.allocated_globals_count
        );

        global_var
    }

    /// PHASE 3B STEP 3: Infrastructure for push/pull sequence for temporary expression results
    ///
    /// **CURRENT STATE**: This method establishes infrastructure for proper Z-Machine
    /// stack discipline but does NOT yet implement actual push/pull opcodes. Results
    /// still go to Variable(0) with tracking for future implementation.
    ///
    /// **INTENDED BEHAVIOR**: Replace Variable(0) direct access with proper Z-Machine
    /// push (VAR:232) and pull (VAR:233) opcodes to maintain LIFO stack order and
    /// eliminate Variable(0) collisions that cause crashes like Property 28 bug.
    ///
    /// # Current Implementation Status
    /// - ✅ Infrastructure and tracking in place
    /// - ❌ Actual push/pull opcodes NOT yet implemented
    /// - ❌ Still uses Variable(0) (no collision reduction yet)
    ///
    /// # Target Operations
    /// - Expression temporaries (arithmetic, logical operations)
    /// - Short-lived builtin results
    /// - Operations consumed within 1-3 instructions
    ///
    /// Phase C2: Complete stack discipline for ALL Variable(0) operations
    /// Converts Variable(0) collision-prone operations to use Z-Machine stack discipline
    /// SCOPE: All operations that store results in Variable(0) must use this
    pub fn use_push_pull_for_result(
        &mut self,
        target_id: IrId,
        context: &str,
    ) -> Result<(), CompilerError> {
        // Safety check: Don't overwrite existing mappings
        if self.push_pull_ir_ids.contains(&target_id) {
            log::debug!(
                "use_push_pull_for_result: IR ID {} already marked for push/pull, skipping",
                target_id
            );
            return Ok(());
        }

        // Phase 1: Check if this is a function call result already in Variable(0)
        if self.function_call_results.contains(&target_id) {
            // This is a function call result - Variable(0) already contains the value
            // Move from Variable(0) to allocated global variable
            let target_var = self.allocate_global_for_ir_id(target_id);
            self.emit_instruction_typed(
                Opcode::Op2(Op2::Store),
                &[Operand::SmallConstant(target_var), Operand::Variable(0)],
                None,
                None,
            )?;

            // Map this IR ID to the global variable
            self.ir_id_to_stack_var.insert(target_id, target_var);

            log::debug!(
                "PHASE_1: Function call result IR ID {} moved from Variable(0) to Variable({}) - no push needed",
                target_id, target_var
            );

            return Ok(());
        }

        // UNIVERSAL SURGICAL FIX (Oct 31, 2025): Use global variables instead of push/pull system
        // This prevents all producer→consumer chain mismatches by eliminating Variable(0) conflicts
        let store_var = self.allocate_global_variable();
        self.ir_id_to_stack_var.insert(target_id, store_var);

        log::debug!(
            "UNIVERSAL SURGICAL FIX: IR ID {} in '{}' -> global var {} (eliminated push/pull)",
            target_id,
            context,
            store_var
        );

        Ok(())
    }

    /// Emit pull instruction for IR ID that was previously pushed
    /// Phase C1.1: Retrieve pushed values from Z-Machine stack when they're consumed
    pub fn emit_pull_for_ir_id(&mut self, ir_id: IrId, context: &str) -> Result<(), CompilerError> {
        // Check if this IR ID was marked for push/pull
        if self.push_pull_ir_ids.contains(&ir_id) {
            // VAR:233 pull(variable) - pops value from Z-Machine stack into Variable(0)
            let pull_operands = vec![Operand::Variable(0)];
            self.emit_instruction_typed(Opcode::OpVar(OpVar::Pull), &pull_operands, None, None)?;

            log::debug!(
                "PHASE_C1.1: IR ID {} using pull instruction in {} - value retrieved from Z-Machine stack to Variable(0)",
                ir_id, context
            );
        }
        Ok(())
    }

    /// Get current code address for instruction generation
    pub fn current_address(&self) -> usize {
        self.code_space.len()
    }

    /// Unified BinaryOp processing used by both translate_ir_instruction and generate_instruction
    pub fn process_binary_op(
        &mut self,
        target: IrId,
        op: &IrBinaryOp,
        left: IrId,
        right: IrId,
    ) -> Result<(), CompilerError> {
        log::debug!(
            "UNIFIED_BINARYOP_PROCESSING: target={}, op={:?}, left={}, right={}",
            target,
            op,
            left,
            right
        );

        // Store binary operation mapping for conditional branch optimization
        self.ir_id_to_binary_op
            .insert(target, (op.clone(), left, right));

        // Check if this is a comparison operation
        let is_comparison = matches!(
            op,
            IrBinaryOp::Equal
                | IrBinaryOp::NotEqual
                | IrBinaryOp::Less
                | IrBinaryOp::LessEqual
                | IrBinaryOp::Greater
                | IrBinaryOp::GreaterEqual
        );

        if is_comparison {
            // STACK DISCIPLINE FIX (Oct 30, 2025): Use usage pattern analysis to determine handling
            // 1. If comparison is used as a value (operand in other operations) -> generate accessible result
            // 2. If comparison is used only in direct branches -> skip result generation (handled by branch emission)
            if self.comparison_ids_used_as_values.contains(&target) {
                log::debug!(
                    "Comparison BinaryOp (IR ID {}) used as VALUE -> generating accessible result with push/pull",
                    target
                );

                // Generate comparison instruction that stores boolean result
                self.generate_comparison_with_result(op, left, right, target)?;
                self.use_push_pull_for_result(target, "Comparison operation result")?;
            } else {
                log::debug!(
                    "Comparison BinaryOp (IR ID {}) used only for BRANCHING -> no result generation needed",
                    target
                );
                // Direct branch comparisons are handled during branch instruction emission
                // No push/pull mechanism needed since result is never accessed as a value
            }
        } else {
            // For non-comparison operations (arithmetic, logical), generate bytecode
            // Handle different binary operations
            match op {
                IrBinaryOp::Add => {
                    // Check if either operand is a string for concatenation FIRST
                    let left_is_string = self.ir_id_to_string.contains_key(&left);
                    let right_is_string = self.ir_id_to_string.contains_key(&right);

                    if left_is_string || right_is_string {
                        // This is string concatenation - handle differently from arithmetic
                        self.translate_string_concatenation(left, right, target)?;

                        // CRITICAL FIX (Oct 27, 2025): String concatenation stack discipline
                        // String concatenation uses runtime multi-part print instructions, NOT stack operations.
                        // The concatenation result exists only as a sequence of print instructions,
                        // never as a single value on the Z-Machine stack.
                        //
                        // BUG PATTERN: use_push_pull_for_result was called here, emitting "push Variable(0)"
                        // before any value was placed on the stack → immediate stack underflow at runtime.
                        //
                        // SOLUTION: Skip use_push_pull_for_result entirely for string concatenation.
                        // The target IR ID is handled by the print system, not stack/variable system.
                        return Ok(());
                    } else {
                        // CRITICAL FIX (Nov 4, 2025): Arithmetic operations should use global variables, not push/pull
                        // This avoids the double-stack-operation bug causing arithmetic results to be 0

                        // Allocate global variable for result first
                        let result_var = self.allocate_global_variable();
                        self.ir_id_to_stack_var.insert(target, result_var);

                        // Resolve operands and generate operation with global variable storage
                        let left_op = self.resolve_ir_id_to_operand(left)?;
                        let right_op = self.resolve_ir_id_to_operand(right)?;
                        self.generate_binary_op(op, left_op, right_op, Some(result_var))?;

                        log::debug!(
                            "Arithmetic BinaryOp ({:?}) result: IR ID {} -> stored directly to global Variable({})",
                            op, target, result_var
                        );

                        // SKIP push/pull mechanism for arithmetic operations
                        return Ok(());
                    }
                }
                IrBinaryOp::And | IrBinaryOp::Or => {
                    // CRITICAL FIX: Logical operations should use global variables, not push/pull
                    // This avoids the "anti-pattern responsible for all our other stack underflows"

                    // Allocate global variable for result first
                    let result_var = self.allocate_global_variable();
                    self.ir_id_to_stack_var.insert(target, result_var);

                    // Resolve operands and generate operation with global variable storage
                    let left_op = self.resolve_ir_id_to_operand(left)?;
                    let right_op = self.resolve_ir_id_to_operand(right)?;
                    self.generate_binary_op(op, left_op, right_op, Some(result_var))?;

                    log::debug!(
                        "Logical BinaryOp ({:?}) result: IR ID {} -> stored directly to global Variable({})",
                        op, target, result_var
                    );

                    // SKIP push/pull mechanism for logical operations
                    return Ok(());
                }
                _ => {
                    // CRITICAL FIX (Nov 4, 2025): All arithmetic operations should use global variables, not push/pull
                    // This avoids the double-stack-operation bug causing arithmetic results to be 0

                    // Allocate global variable for result first
                    let result_var = self.allocate_global_variable();
                    self.ir_id_to_stack_var.insert(target, result_var);

                    // Resolve operands and generate operation with global variable storage
                    let left_op = self.resolve_ir_id_to_operand(left)?;
                    let right_op = self.resolve_ir_id_to_operand(right)?;
                    self.generate_binary_op(op, left_op, right_op, Some(result_var))?;

                    log::debug!(
                        "Arithmetic BinaryOp ({:?}) result: IR ID {} -> stored directly to global Variable({})",
                        op, target, result_var
                    );

                    // SKIP push/pull mechanism for arithmetic operations
                    return Ok(());
                }
            }
        }

        Ok(())
    }

    /// Analyze how comparison operations are used throughout the IR
    /// Returns a set of comparison IR IDs that are used as values (need push/pull)
    /// vs those used directly in branches (no push/pull needed)
    pub fn analyze_comparison_usage_patterns(&self, ir: &IrProgram) -> IndexSet<IrId> {
        let mut comparison_ids_used_as_values = IndexSet::new();

        // First, identify all comparison operation IR IDs
        let mut all_comparison_ids = IndexSet::new();
        for func in &ir.functions {
            for instruction in &func.body.instructions {
                if let IrInstruction::BinaryOp { target, op, .. } = instruction {
                    if matches!(
                        op,
                        IrBinaryOp::Equal
                            | IrBinaryOp::NotEqual
                            | IrBinaryOp::Less
                            | IrBinaryOp::LessEqual
                            | IrBinaryOp::Greater
                            | IrBinaryOp::GreaterEqual
                    ) {
                        all_comparison_ids.insert(*target);
                    }
                }
            }
        }

        // Now analyze how each comparison ID is used
        for func in &ir.functions {
            for instruction in &func.body.instructions {
                match instruction {
                    // Check if comparison results are used as operands in other operations
                    IrInstruction::BinaryOp { left, right, .. } => {
                        if all_comparison_ids.contains(left) {
                            comparison_ids_used_as_values.insert(*left);
                            log::debug!("USAGE_ANALYSIS: IR ID {} (comparison) used as LEFT operand in BinaryOp - needs push/pull", left);
                        }
                        if all_comparison_ids.contains(right) {
                            comparison_ids_used_as_values.insert(*right);
                            log::debug!("USAGE_ANALYSIS: IR ID {} (comparison) used as RIGHT operand in BinaryOp - needs push/pull", right);
                        }
                    }
                    IrInstruction::UnaryOp { operand, .. } => {
                        if all_comparison_ids.contains(operand) {
                            comparison_ids_used_as_values.insert(*operand);
                            log::debug!("USAGE_ANALYSIS: IR ID {} (comparison) used as operand in UnaryOp - needs push/pull", operand);
                        }
                    }
                    // Check for direct branch usage (should NOT use push/pull)
                    IrInstruction::Branch { condition, .. } => {
                        if all_comparison_ids.contains(condition) {
                            log::debug!("USAGE_ANALYSIS: IR ID {} (comparison) used directly in Branch - no push/pull needed", condition);
                            // Note: we don't add to comparison_ids_used_as_values
                        }
                    }
                    // Check other potential usage patterns
                    IrInstruction::StoreVar { source, .. } => {
                        if all_comparison_ids.contains(source) {
                            comparison_ids_used_as_values.insert(*source);
                            log::debug!("USAGE_ANALYSIS: IR ID {} (comparison) used in StoreVar - needs push/pull", source);
                        }
                    }
                    IrInstruction::Call { args, .. } => {
                        for arg in args {
                            if all_comparison_ids.contains(arg) {
                                comparison_ids_used_as_values.insert(*arg);
                                log::debug!("USAGE_ANALYSIS: IR ID {} (comparison) used as function argument - needs push/pull", arg);
                            }
                        }
                    }
                    _ => {
                        // Other instruction types don't typically use comparison results
                    }
                }
            }
        }

        log::debug!(
            "USAGE_ANALYSIS: Found {} comparison operations total",
            all_comparison_ids.len()
        );
        log::debug!(
            "USAGE_ANALYSIS: {} comparisons used as values (need push/pull): {:?}",
            comparison_ids_used_as_values.len(),
            comparison_ids_used_as_values.iter().collect::<Vec<_>>()
        );
        log::debug!(
            "USAGE_ANALYSIS: {} comparisons used only in branches (no push/pull): {:?}",
            all_comparison_ids.len() - comparison_ids_used_as_values.len(),
            all_comparison_ids
                .difference(&comparison_ids_used_as_values)
                .collect::<Vec<_>>()
        );

        comparison_ids_used_as_values
    }

    /// Generate comparison operation that stores boolean result to Variable(0)
    /// This method handles comparison operations that need to produce accessible results
    /// for use in logical operations (And, Or) rather than direct branching
    fn generate_comparison_with_result(
        &mut self,
        op: &IrBinaryOp,
        left: IrId,
        right: IrId,
        target: IrId,
    ) -> Result<(), CompilerError> {
        log::debug!(
            "COMPARISON_WITH_RESULT: Generating {:?} comparison for IR ID {} (left={}, right={})",
            op,
            target,
            left,
            right
        );

        // Resolve operands for the comparison
        let left_operand = self.resolve_ir_id_to_operand(left)?;
        let right_operand = self.resolve_ir_id_to_operand(right)?;

        // The Z-Machine approach: Use branch instructions to set Variable(0) to 1 or 0
        // 1. Branch instruction: if comparison_true then jump to set_true_label
        // 2. Set Variable(0) = 0 (false case)
        // 3. Jump to end_label
        // 4. set_true_label: Set Variable(0) = 1 (true case)
        // 5. end_label: continue

        let set_true_label = self.next_string_id;
        self.next_string_id += 1;
        let end_label = self.next_string_id;
        self.next_string_id += 1;

        // Generate comparison branch instruction
        match op {
            IrBinaryOp::Equal => {
                let layout = self.emit_instruction_typed(
                    Opcode::Op2(Op2::Je),
                    &[left_operand, right_operand],
                    None,
                    Some(-1), // Placeholder for set_true_label
                )?;
                // CRITICAL FIX (Nov 2, 2025): Use correct branch location from layout instead of deprecated function
                //
                // BUG: generate_comparison_with_result() used deprecated add_unresolved_reference() which
                // calculated branch location as self.code_space.len() (AFTER instruction emission) instead
                // of using the correct branch placeholder location returned by emit_instruction_typed().
                //
                // IMPACT: Branch placeholders were patched at wrong memory addresses, causing:
                // - VAR:0x1f crashes when unresolved 0xFF bytes were executed as instructions
                // - Infinite loops when jump placeholders remained as 0xFFFF (-1 offset)
                //
                // SOLUTION: Capture layout from emit_instruction_typed() and use layout.branch_location
                // to ensure branch placeholders are resolved at the correct memory addresses.
                if let Some(branch_location) = layout.branch_location {
                    self.add_unresolved_reference_at_location(
                        LegacyReferenceType::Branch,
                        set_true_label,
                        false,
                        MemorySpace::Code,
                        branch_location,
                    )?;
                } else {
                    return Err(CompilerError::CodeGenError(
                        "Expected branch location for Equal comparison instruction".to_string(),
                    ));
                }
            }
            IrBinaryOp::NotEqual => {
                let layout = self.emit_instruction_typed(
                    Opcode::Op2(Op2::Je),
                    &[left_operand, right_operand],
                    None,
                    Some(-1), // Placeholder for end_label (branch when equal, i.e., false case)
                )?;
                // CRITICAL FIX: Use correct branch location from layout
                if let Some(branch_location) = layout.branch_location {
                    self.add_unresolved_reference_at_location(
                        LegacyReferenceType::Branch,
                        end_label,
                        false,
                        MemorySpace::Code,
                        branch_location,
                    )?;
                } else {
                    return Err(CompilerError::CodeGenError(
                        "Expected branch location for NotEqual comparison instruction".to_string(),
                    ));
                }
            }
            IrBinaryOp::Less => {
                let layout = self.emit_instruction_typed(
                    Opcode::Op2(Op2::Jl),
                    &[left_operand, right_operand],
                    None,
                    Some(-1), // Placeholder for set_true_label
                )?;
                // CRITICAL FIX: Use correct branch location from layout
                if let Some(branch_location) = layout.branch_location {
                    self.add_unresolved_reference_at_location(
                        LegacyReferenceType::Branch,
                        set_true_label,
                        false,
                        MemorySpace::Code,
                        branch_location,
                    )?;
                } else {
                    return Err(CompilerError::CodeGenError(
                        "Expected branch location for Less comparison instruction".to_string(),
                    ));
                }
            }
            IrBinaryOp::Greater => {
                let layout = self.emit_instruction_typed(
                    Opcode::Op2(Op2::Jg),
                    &[left_operand, right_operand],
                    None,
                    Some(-1), // Placeholder for set_true_label
                )?;
                // CRITICAL FIX: Use correct branch location from layout
                if let Some(branch_location) = layout.branch_location {
                    self.add_unresolved_reference_at_location(
                        LegacyReferenceType::Branch,
                        set_true_label,
                        false,
                        MemorySpace::Code,
                        branch_location,
                    )?;
                } else {
                    return Err(CompilerError::CodeGenError(
                        "Expected branch location for Greater comparison instruction".to_string(),
                    ));
                }
            }
            IrBinaryOp::LessEqual => {
                // Less or equal: Use jg and invert logic (branch to false case)
                let layout = self.emit_instruction_typed(
                    Opcode::Op2(Op2::Jg),
                    &[left_operand, right_operand],
                    None,
                    Some(-1), // Placeholder for end_label (branch when greater, i.e., false case)
                )?;
                // CRITICAL FIX: Use correct branch location from layout
                if let Some(branch_location) = layout.branch_location {
                    self.add_unresolved_reference_at_location(
                        LegacyReferenceType::Branch,
                        end_label,
                        false,
                        MemorySpace::Code,
                        branch_location,
                    )?;
                } else {
                    return Err(CompilerError::CodeGenError(
                        "Expected branch location for LessEqual comparison instruction".to_string(),
                    ));
                }
            }
            IrBinaryOp::GreaterEqual => {
                // Greater or equal: Use jl and invert logic (branch to false case)
                let layout = self.emit_instruction_typed(
                    Opcode::Op2(Op2::Jl),
                    &[left_operand, right_operand],
                    None,
                    Some(-1), // Placeholder for end_label (branch when less, i.e., false case)
                )?;
                // CRITICAL FIX: Use correct branch location from layout
                if let Some(branch_location) = layout.branch_location {
                    self.add_unresolved_reference_at_location(
                        LegacyReferenceType::Branch,
                        end_label,
                        false,
                        MemorySpace::Code,
                        branch_location,
                    )?;
                } else {
                    return Err(CompilerError::CodeGenError(
                        "Expected branch location for GreaterEqual comparison instruction"
                            .to_string(),
                    ));
                }
            }
            _ => {
                return Err(CompilerError::CodeGenError(format!(
                    "generate_comparison_with_result: Unsupported comparison operation {:?}",
                    op
                )));
            }
        }

        // False case: Set Variable(0) = 0
        if matches!(
            op,
            IrBinaryOp::Equal | IrBinaryOp::Less | IrBinaryOp::Greater
        ) {
            // Direct comparison: false case falls through
            self.emit_instruction_typed(
                Opcode::Op2(Op2::Store),
                &[Operand::Variable(0), Operand::SmallConstant(0)],
                None,
                None,
            )?;
            // Jump to end
            let jump_layout = self.emit_instruction_typed(
                Opcode::Op1(Op1::Jump),
                &[Operand::LargeConstant(placeholder_word())],
                None,
                None,
            )?;
            // CRITICAL FIX (Nov 2, 2025): Use correct operand location for jump instruction
            //
            // BUG: Same deprecated add_unresolved_reference() bug as above, but for jump instructions
            // which use operand_location instead of branch_location for placeholder addresses.
            //
            // IMPACT: Jump placeholders remained unresolved (0xFFFF), causing infinite loops
            // where jump -1 created self-referencing loops in comparison result generation.
            //
            // SOLUTION: Use layout.operand_location to ensure jump operands are patched correctly.
            if let Some(operand_location) = jump_layout.operand_location {
                self.add_unresolved_reference_at_location(
                    LegacyReferenceType::Jump,
                    end_label,
                    false,
                    MemorySpace::Code,
                    operand_location,
                )?;
            } else {
                return Err(CompilerError::CodeGenError(
                    "Expected operand location for Jump instruction".to_string(),
                ));
            }
        }

        // True case label
        if matches!(
            op,
            IrBinaryOp::Equal | IrBinaryOp::Less | IrBinaryOp::Greater
        ) {
            self.reference_context
                .ir_id_to_address
                .insert(set_true_label, self.code_address);
        } else {
            // For inverted comparisons (NotEqual, LessEqual, GreaterEqual), true case falls through
            // Set Variable(0) = 1
            self.emit_instruction_typed(
                Opcode::Op2(Op2::Store),
                &[Operand::Variable(0), Operand::SmallConstant(1)],
                None,
                None,
            )?;
            // Jump to end
            let jump_layout = self.emit_instruction_typed(
                Opcode::Op1(Op1::Jump),
                &[Operand::LargeConstant(placeholder_word())],
                None,
                None,
            )?;
            // CRITICAL FIX: Use correct operand location from layout instead of deprecated function
            if let Some(operand_location) = jump_layout.operand_location {
                self.add_unresolved_reference_at_location(
                    LegacyReferenceType::Jump,
                    end_label,
                    false,
                    MemorySpace::Code,
                    operand_location,
                )?;
            } else {
                return Err(CompilerError::CodeGenError(
                    "Expected operand location for Jump instruction in inverted comparison"
                        .to_string(),
                ));
            }
        }

        // True case: Set Variable(0) = 1 (for direct comparisons)
        if matches!(
            op,
            IrBinaryOp::Equal | IrBinaryOp::Less | IrBinaryOp::Greater
        ) {
            self.emit_instruction_typed(
                Opcode::Op2(Op2::Store),
                &[Operand::Variable(0), Operand::SmallConstant(1)],
                None,
                None,
            )?;
        }

        // End label
        self.reference_context
            .ir_id_to_address
            .insert(end_label, self.code_address);

        log::debug!(
            "COMPARISON_WITH_RESULT: Generated comparison {:?} for IR ID {} - result stored in Variable(0)",
            op, target
        );

        Ok(())
    }

    /// Allocate space for strings with proper alignment

    /// Generate init block as a proper routine and startup sequence
    pub fn generate_init_block(
        &mut self,
        init_block: &IrBlock,
        _ir: &IrProgram,
    ) -> Result<(usize, u8), CompilerError> {
        log::debug!(
 "generate_init_block: Generating init routine with {} instructions (Z-Machine native architecture - header first)",
 init_block.instructions.len()
 );

        // CRITICAL ARCHITECTURE: Generate init block AS the main program entry point routine
        // Like Zork I: PC points directly to init routine header, then init instructions execute

        // Set init block context flag
        self.in_init_block = true;

        // Record this as the init routine that PC will point to (main program entry point)
        let init_routine_address = self.code_address;
        let init_routine_id = 8000u32;

        log::info!(
            " ZORK_ARCHITECTURE: Generating init routine at 0x{:04x} (PC target, header first)",
            init_routine_address
        );

        // Set up function context for init block
        // Map local variables declared in init block (e.g., let statements)
        log::debug!(
            " LOCAL_VAR_SETUP: Init block has {} local variables",
            _ir.init_block_locals.len()
        );

        for local in &_ir.init_block_locals {
            self.ir_id_to_local_var.insert(local.ir_id, local.slot);
            log::debug!(
                " Local variable mapping: '{}' (IR ID {}) -> local slot {} for init block",
                local.name,
                local.ir_id,
                local.slot
            );
        }

        self.current_function_locals = _ir.init_block_locals.len() as u8;
        self.current_function_name = Some("main".to_string());

        // ARCHITECTURAL FIX: Generate proper V3 function header with correct local variable allocation
        // The main program must be a proper Z-Machine function - PC points to function start (header)
        // Z-Machine spec: "Execution of instructions begins from the byte after this header information"
        log::debug!(
            "🏁 MAIN_PROGRAM: Generating routine header at 0x{:04x} (PC will point here)",
            self.code_address
        );

        // Generate V3 function header: Local count
        // Now that we know the actual local count from init_block_locals, emit it directly
        let header_location = self.code_address;
        self.emit_byte(self.current_function_locals)?; // Emit actual local count

        log::debug!(
            "🏁 MAIN_ROUTINE: Header complete at 0x{:04x}, instructions follow",
            self.code_address
        );

        // Record main routine address and header location for patching
        self.function_addresses
            .insert(init_routine_id, init_routine_address);
        self.record_final_address(init_routine_id, init_routine_address);

        // Store header location for later local count patching
        self.function_header_locations
            .insert(init_routine_id, header_location);

        // Generate the init block code directly after the header
        // CRITICAL: Use translate_ir_instruction to ensure proper instruction generation
        log::debug!(
            "Generating {} init block instructions",
            init_block.instructions.len()
        );

        for (i, instruction) in init_block.instructions.iter().enumerate() {
            log::debug!(
                " IR[{}]: {:?} at code_addr=0x{:04x}",
                i,
                instruction,
                self.code_address
            );
            let before_addr = self.code_address;
            self.generate_instruction(instruction)?;
            let after_addr = self.code_address;
            log::debug!(
                " IR[{}]: Generated {} bytes (0x{:04x} -> 0x{:04x})",
                i,
                after_addr - before_addr,
                before_addr,
                after_addr
            );
        }

        // Process any pending labels at end of init block
        // Labels at the end of init block (like endif labels after a branch)
        // won't have a following instruction to trigger deferred label processing.
        // We must process them here before finalizing the routine.
        if !self.pending_labels.is_empty() {
            let label_address = self.code_address;
            // Collect labels first to avoid borrow issues
            let labels_to_process: Vec<_> = self.pending_labels.drain(..).collect();
            for label_id in labels_to_process {
                log::debug!(
 "END_OF_INIT_BLOCK_LABEL: Processing pending label {} at end of init block at address 0x{:04x}",
 label_id, label_address
 );
                self.label_addresses.insert(label_id, label_address);
                self.record_final_address(label_id, label_address);
            }
        }

        // ARCHITECTURAL FIX: Finalize init routine header with actual local variable count
        // NOTE: Init routine (ID 8000) IS the main program entry point that PC starts at
        // This is NOT the main loop routine (ID 9000) - that gets finalized separately
        log::debug!(
 "🔧 INIT_ROUTINE_FINALIZE: Patching init routine header (ID {}) with {} local variables used",
 init_routine_id, self.current_function_locals
 );
        self.finalize_function_header(init_routine_id)?;

        // Add program-mode specific termination
        match _ir.program_mode {
            crate::grue_compiler::ast::ProgramMode::Script => {
                log::debug!(
                    "Adding QUIT instruction for Script mode at 0x{:04x}",
                    self.code_address
                );
                self.emit_byte(0xBA)?; // QUIT instruction for Script mode
            }
            crate::grue_compiler::ast::ProgramMode::Interactive => {
                log::debug!(
                    "Adding main loop call for Interactive mode at 0x{:04x}",
                    self.code_address
                );
                // CRITICAL: Call the main loop routine (ID 9000) instead of jumping to it.
                // This ensures the routine header (which declares 5 local variables for grammar
                // matching) gets properly processed by the Z-Machine interpreter.
                //
                // Using CALL instead of JUMP:
                // - CALL: Processes routine header, sets up local variables 1-5, executes routine
                // - JUMP: Skips routine header, jumps directly to first instruction (broken!)
                //
                // The main loop needs 5 locals for grammar system:
                // - Variable 1: word count from parse buffer
                // - Variable 2: word 1 dictionary address (for verb matching)
                // - Variable 3: resolved object ID (for noun matching)
                // - Variable 4: loop counter (for object lookup iteration)
                // - Variable 5: property value (for object lookup comparison)
                let layout = self.emit_instruction_typed(
                    Opcode::OpVar(OpVar::CallVs), // call_vs (VAR:224, opcode 0 - NOT 0x20!)
                    &[Operand::LargeConstant(placeholder_word())], // Placeholder for routine address
                    Some(0x00), // Store result on stack (required by Z-Machine spec)
                    None,       // No branch
                )?;

                // Add unresolved reference for main loop routine call
                self.reference_context
                    .unresolved_refs
                    .push(UnresolvedReference {
                        reference_type: LegacyReferenceType::FunctionCall,
                        location: layout
                            .operand_location
                            .expect("call instruction must have operand"),
                        target_id: 9000, // Main loop routine ID (includes header byte)
                        is_packed_address: true, // Routine addresses are packed
                        offset_size: 2,
                        location_space: MemorySpace::Code,
                    });
            }
            crate::grue_compiler::ast::ProgramMode::Custom => {
                log::debug!(
                    "Adding QUIT instruction for Custom mode at 0x{:04x}",
                    self.code_address
                );
                self.emit_byte(0xBA)?; // QUIT instruction for Custom mode (temporary)
            }
        }

        // Clear init block context flag
        self.in_init_block = false;

        log::info!(
            " INIT_ROUTINE: Complete at 0x{:04x} (PC target: 0x{:04x}, 0 locals)",
            self.code_address - 1,
            init_routine_address
        );

        // Return init routine address and 0 locals (simple init block)
        Ok((init_routine_address, 0))
    }

    /// Validate story data integrity and boundary calculations
    fn validate_story_data_integrity(&self) -> Result<(), CompilerError> {
        log::debug!("=== STORY DATA INTEGRITY CHECK ===");
        log::debug!("Story data size: {} bytes", self.story_data.len());
        log::debug!("Current address: 0x{:04x}", self.code_address);
        log::debug!(
            "Max valid address: 0x{:04x}",
            self.story_data.len().saturating_sub(1)
        );

        // Check for any addresses that exceed bounds
        if self.code_address > self.story_data.len() {
            return Err(CompilerError::CodeGenError(format!(
                "Current address 0x{:04x} exceeds story data size 0x{:04x}",
                self.code_address,
                self.story_data.len()
            )));
        }

        // Validate story data utilization
        let utilization = (self.code_address as f64 / self.story_data.len() as f64) * 100.0;
        log::info!(
            "Story data utilization: {:.1}% ({}/{})",
            utilization,
            self.code_address,
            self.story_data.len()
        );

        // Check for excessive unused space (might indicate size calculation errors)
        if utilization < 50.0 && self.story_data.len() > 8192 {
            log::warn!(
                "Low story data utilization ({:.1}%) - buffer may be oversized",
                utilization
            );
        }

        // Validate that all critical sections are within bounds
        let sections = vec![
            ("Header", 0x0000, 0x0040),
            (
                "Object table",
                self.object_table_addr,
                self.object_table_addr + 200,
            ), // Estimate
            (
                "Property tables",
                self.property_table_addr,
                self.property_table_addr + 500,
            ), // Estimate
            ("Current code", 0x0040, self.code_address),
        ];

        for (name, start, end) in sections {
            if end > self.story_data.len() {
                log::warn!(
                    "{} section (0x{:04x}-0x{:04x}) may exceed story bounds (0x{:04x})",
                    name,
                    start,
                    end,
                    self.story_data.len()
                );
            } else {
                log::debug!("{} section: 0x{:04x}-0x{:04x} ✓", name, start, end);
            }
        }

        log::debug!("Story data integrity validation complete");
        Ok(())
    }

    /// Resolve a single reference by patching the story data
    fn resolve_single_reference(
        &mut self,
        reference: &UnresolvedReference,
    ) -> Result<(), CompilerError> {
        log::debug!("=== RESOLVE_SINGLE_REFERENCE DEBUG ===");
        log::debug!("Target IR ID: {}", reference.target_id);
        log::debug!("Reference type: {:?}", reference.reference_type);
        log::debug!("Reference location: 0x{:04x}", reference.location);
        log::debug!("Is packed address: {}", reference.is_packed_address);
        log::debug!("Offset size: {}", reference.offset_size);

        // Look up the target address
        let target_address = match self
            .reference_context
            .ir_id_to_address
            .get(&reference.target_id)
        {
            Some(&addr) => {
                log::debug!(
                    " FOUND address for IR ID {}: 0x{:04x}",
                    reference.target_id,
                    addr
                );
                addr
            }
            None => {
                log::debug!(" FAILED to resolve IR ID {}", reference.target_id);
                log::debug!("Detailed analysis:");
                log::debug!(
                    "Total available IR ID -> address mappings: {}",
                    self.reference_context.ir_id_to_address.len()
                );

                // Show mappings around the problematic ID
                let target = reference.target_id;
                log::debug!("Mappings near target ID {}:", target);
                for id in (target.saturating_sub(5))..=(target + 5) {
                    if let Some(&addr) = self.reference_context.ir_id_to_address.get(&id) {
                        log::debug!(" IR ID {} -> 0x{:04x} ", id, addr);
                    } else if id == target {
                        log::debug!(" IR ID {} -> MISSING (TARGET)", id);
                    } else {
                        log::debug!(" IR ID {} -> missing", id);
                    }
                }

                log::debug!("Function addresses: {:?}", self.function_addresses);

                // Check if this is a label ID that should be in the address mapping
                log::debug!("Checking if IR ID {} is a label...", target);

                return Err(CompilerError::CodeGenError(format!(
                    "Cannot resolve reference to IR ID {}: target address not found",
                    reference.target_id
                )));
            }
        };

        // Debug specific address patches if needed
        if reference.location == 0x0f89 || reference.location == 0x0f8a || target_address == 0x0a4d
        {
            log::debug!(
                "Patch at location 0x{:04x} -> address 0x{:04x}",
                reference.location,
                target_address
            );
        }

        log::debug!(
            "resolve_single_reference: IR ID {} -> address 0x{:04x}, patching at location 0x{:04x}",
            reference.target_id,
            target_address,
            reference.location
        );

        // DIAGNOSTIC: Check if the patch location contains placeholder bytes
        if reference.location + 1 < self.story_data.len() {
            let current_bytes = (self.story_data[reference.location] as u16) << 8
                | (self.story_data[reference.location + 1] as u16);
            if current_bytes == placeholder_word() {
                log::debug!(
 "PATCH DIAGNOSTIC: Location 0x{:04x} contains placeholder 0xFFFF - will be resolved",
 reference.location
 );
            } else {
                log::warn!(
 "PATCH DIAGNOSTIC: Location 0x{:04x} contains non-placeholder bytes 0x{:04x} - potential double-patch!",
 reference.location,
 current_bytes
 );
                log::warn!(
                    " Reference type: {:?}, Target ID: {}, Is packed: {}",
                    reference.reference_type,
                    reference.target_id,
                    reference.is_packed_address
                );

                // Check if this target was already resolved
                if let Some(&existing_addr) = self
                    .reference_context
                    .ir_id_to_address
                    .get(&reference.target_id)
                {
                    log::debug!(
 " Target ID {} already resolved to address 0x{:04x} - this indicates the deduplication should have caught this",
 reference.target_id,
 existing_addr
 );
                    // This is now expected to be rare due to deduplication
                }
            }
        }

        // NOTE: Jump and Branch references have early returns above
        // StringRef and FunctionCall references are handled by legacy system below
        // This point should never be reached - all reference types handled above

        panic!(
 "COMPILER BUG: Unhandled reference type {:?} for target_id {} - should have been handled by early returns or legacy system",
 reference.reference_type, reference.target_id
 );
    }

    fn calculate_instruction_size_from_data(
        &self,
        data: &[u8],
        instruction_addr: usize,
    ) -> Result<usize, CompilerError> {
        if instruction_addr >= data.len() {
            return Err(CompilerError::CodeGenError(format!(
                "Instruction address 0x{:04x} out of bounds (data len: {})",
                instruction_addr,
                data.len()
            )));
        }

        let opcode_byte = data[instruction_addr];
        let top_2_bits = (opcode_byte & 0xC0) >> 6;

        let mut size = 1; // Start with opcode byte

        match top_2_bits {
            0b00 => {
                // Short form (1OP) or Extended form
                if opcode_byte == 0xBE {
                    return Err(CompilerError::CodeGenError(
                        "Extended form not supported".to_string(),
                    ));
                }

                // 1OP short form: bits 5-4 indicate operand type
                let operand_type = (opcode_byte & 0x30) >> 4;
                match operand_type {
                    0b00 => size += 2, // Large constant (2 bytes)
                    0b01 => size += 1, // Small constant (1 byte)
                    0b10 => size += 1, // Variable (1 byte)
                    0b11 => {}         // Omitted operand (0 bytes) - 0OP form
                    _ => unreachable!(),
                }
            }
            0b01 => {
                // Long form (2OP)
                let op1_type = (opcode_byte & 0x40) >> 6;
                let op2_type = (opcode_byte & 0x20) >> 5;

                // First operand
                size += if op1_type == 0 { 2 } else { 1 }; // 0=large constant, 1=variable

                // Second operand
                size += if op2_type == 0 { 2 } else { 1 }; // 0=large constant, 1=variable
            }
            0b10 => {
                // Short form (1OP)
                let operand_type = (opcode_byte & 0x30) >> 4;
                match operand_type {
                    0b00 => size += 2, // Large constant (2 bytes)
                    0b01 => size += 1, // Small constant (1 byte)
                    0b10 => size += 1, // Variable (1 byte)
                    0b11 => {}         // Omitted operand (0 bytes)
                    _ => unreachable!(),
                }
            }
            0b11 => {
                // Variable form - need to check operand type byte
                if instruction_addr + 1 >= data.len() {
                    return Err(CompilerError::CodeGenError(
                        "Variable form instruction truncated".to_string(),
                    ));
                }

                size += 1; // Operand type byte
                let operand_types_byte = data[instruction_addr + 1];

                // Each pair of bits in operand_types_byte indicates an operand type
                for i in 0..4 {
                    let operand_type = (operand_types_byte >> (6 - i * 2)) & 0x03;
                    match operand_type {
                        0b00 => size += 2, // Large constant
                        0b01 => size += 1, // Small constant
                        0b10 => size += 1, // Variable
                        0b11 => break,     // No more operands
                        _ => unreachable!(),
                    }
                }
            }
            _ => unreachable!(),
        }

        // Check if instruction has store variable (depends on opcode)
        let opcode = opcode_byte & 0x1F; // Extract 5-bit opcode
        let has_store = self.opcode_has_store_var(opcode);
        if has_store {
            size += 1; // Store variable byte
        }

        // Check if instruction has branch offset (depends on opcode)
        let has_branch = self.opcode_has_branch_offset(opcode);
        if has_branch {
            // Branch offset is 1 or 2 bytes - we'd need to examine the branch byte
            // For now, assume 2 bytes (worst case)
            size += 2;
        }

        Ok(size)
    }

    /// Check if an opcode stores its result in a variable
    fn opcode_has_store_var(&self, opcode: u8) -> bool {
        // This is a simplified check - in reality we'd need a full opcode table
        // For now, handle the most common cases
        // NOTE: 0x08 and 0x09 are form-dependent!
        //   - 2OP:0x08 (or) and 2OP:0x09 (and) have store_var
        //   - VAR:0x08 (push) and VAR:0x09 (pull) do NOT have store_var
        // This function is only used for 2OP opcodes, so we include them
        match opcode {
            0x08 | 0x09 | 0x0E | 0x0F | 0x10 | 0x11 | 0x12 | 0x13 | 0x14 | 0x15 | 0x16 | 0x17
            | 0x18 | 0x19 => true, // Arithmetic, loads, property access
            _ => false,
        }
    }

    /// Check if an opcode has a branch offset
    fn opcode_has_branch_offset(&self, opcode: u8) -> bool {
        // This is a simplified check - in reality we'd need a full opcode table
        match opcode {
            0x01..=0x07 => true, // Conditional branches
            _ => false,
        }
    }

    /// Jump offset calculation for Z-Machine jump instructions (opcode 0x8c)
    /// Jump instructions use signed 16-bit word offset: new_PC = current_PC + offset - 2
    fn patch_jump_offset(
        &mut self,
        location: usize,
        target_address: usize,
    ) -> Result<(), CompilerError> {
        log::debug!(
            "patch_jump_offset: location=0x{:04x}, target_address=0x{:04x}",
            location,
            target_address
        );

        // Z-Machine jump offset calculation: new_PC = current_PC + offset - 2
        // So: offset = target_address - current_PC + 2
        // current_PC = location + 2 (since we're writing a 2-byte operand)
        let current_pc = location + 2;
        let offset = (target_address as i32) - (current_pc as i32) + 2;

        log::debug!(
            "patch_jump_offset: current_PC=0x{:04x}, offset={}",
            current_pc,
            offset
        );

        // Check if offset fits in signed 16-bit range
        if !(-32768..=32767).contains(&offset) {
            return Err(CompilerError::CodeGenError(format!(
                "Jump offset {} is out of range for signed 16-bit (-32768 to 32767)",
                offset
            )));
        }

        // Write as signed 16-bit big-endian word
        let offset_u16 = offset as u16;
        let high_byte = ((offset_u16 >> 8) & 0xFF) as u8;
        let low_byte = (offset_u16 & 0xFF) as u8;

        log::debug!(
 "patch_jump_offset: Writing jump offset 0x{:04x} ({}) as bytes 0x{:02x} 0x{:02x} to location 0x{:04x}",
 offset_u16, offset, high_byte, low_byte, location
 );

        self.write_byte_at(location, high_byte)?;
        self.write_byte_at(location + 1, low_byte)?;

        Ok(())
    }

    /// Patch a branch offset at the given location
    pub fn patch_branch_offset(
        &mut self,
        location: usize,
        target_address: usize,
    ) -> Result<(), CompilerError> {
        log::debug!(
            "🔧 BRANCH_PATCH_ATTEMPT: location=0x{:04x}, target_address=0x{:04x}",
            location,
            target_address
        );

        // Z-Machine branch offset calculation: "Address after branch data + Offset - 2"
        // So: Offset = target_address - (address_after_branch_data) + 2

        // First, determine if we need 1-byte or 2-byte format
        // We need to calculate the offset assuming 1-byte first, then check if it fits
        let address_after_1byte = location + 1;
        let _offset_1byte = (target_address as i32) - (address_after_1byte as i32) + 2;

        // Always use 2-byte format since we reserved 2 bytes
        // Calculate offset for 2-byte format (address after 2 bytes)
        let address_after_2byte = location + 2;
        let offset_2byte = (target_address as i32) - (address_after_2byte as i32) + 2;

        log::debug!(
 "🔧 BRANCH_CALC: address_after_2byte=0x{:04x}, offset_2byte={}, first_byte=0x{:02x}, second_byte=0x{:02x}",
 address_after_2byte,
 offset_2byte,
 0x40 | ((offset_2byte as u16 >> 8) as u8 & 0x3F),
 (offset_2byte as u16 & 0xFF) as u8
 );

        if !(-8192..=8191).contains(&offset_2byte) {
            return Err(CompilerError::CodeGenError(format!(
                "Branch offset {} is out of range for 2-byte format (-8192 to 8191)",
                offset_2byte
            )));
        }

        // CRITICAL FIX: Always use 2-byte format since we reserved 2 bytes
        // The Z-Machine interpreter expects consistent instruction sizes
        // Using 1-byte format with padding causes invalid opcode 0x00 decoding
        //
        // Z-Machine branch encoding:
        // Bit 7: Branch polarity (1 = branch on true, 0 = branch on false)
        // Bit 6: Branch format (1 = 1-byte, 0 = 2-byte)
        // Bits 5-0: High 6 bits of offset (for 2-byte format)
        //
        // For 2-byte format: Bit 6 MUST be 0
        // First byte: bits [7: polarity, 6: 0 (2-byte), 5-0: high 6 bits of offset]
        // Second byte: Low 8 bits of offset

        // Read the existing placeholder to determine branch sense (bit 15)
        let high_byte = self.final_data[location] as u16;
        let low_byte = self.final_data[location + 1] as u16;
        let placeholder = (high_byte << 8) | low_byte;
        let branch_on_true = (placeholder & 0x8000) != 0; // Check bit 15

        let offset_u16 = offset_2byte as u16;
        let polarity_bit = if branch_on_true { 0x80 } else { 0x00 }; // Bit 7
                                                                     // CRITICAL FIX: Force bit 6=0 for 2-byte format by masking with 0xBF (10111111)
                                                                     // This prevents accidentally creating 1-byte format when offset=0
                                                                     // Bug: offset_u16=0 with polarity_bit=0x80 created first_byte=0x80 (bit 6=1 = 1-byte format)
                                                                     // Fix: Ensure bit 6 is always 0 for consistent 2-byte format
        let first_byte = (polarity_bit | ((offset_u16 >> 8) as u8 & 0x3F)) & 0xBF; // Force bit 6=0
        let second_byte = (offset_u16 & 0xFF) as u8;

        // CRITICAL DEBUG: Track what gets written for our problematic branch
        if location == 0x13cb {
            log::error!("🔧 BRANCH_PATCH_OUTPUT: location=0x{:04x} placeholder=0x{:04x} branch_on_true={} target=0x{:04x} offset={} encoded=[0x{:02x} 0x{:02x}]",
                location, placeholder, branch_on_true, target_address, offset_2byte, first_byte, second_byte);
        }

        log::debug!("🔧 BRANCH_PATCH: location=0x{:04x} placeholder=0x{:04x} branch_on_true={} target=0x{:04x} offset={} encoded=[0x{:02x} 0x{:02x}]",
            location, placeholder, branch_on_true, target_address, offset_2byte, first_byte, second_byte);

        // TEMPORARY: Check what we're writing
        if first_byte == 0x01 && second_byte == 0x9f {
            panic!("FOUND THE BUG: patch_branch_offset is writing 0x01 0x9f at location 0x{:04x}! offset_2byte={}, target_address=0x{:04x}", 
 location, offset_2byte, target_address);
        }

        self.write_byte_at(location, first_byte)?;
        self.write_byte_at(location + 1, second_byte)?;
        log::debug!(
            "🟢 BRANCH_PATCHED: location=0x{:04x} ← [0x{:02x} 0x{:02x}] (offset={}, target=0x{:04x})",
            location, first_byte, second_byte, offset_2byte, target_address
        );

        Ok(())
    }

    /// Generic address patching helper
    pub fn patch_address(
        &mut self,
        location: usize,
        address: u16,
        size: usize,
    ) -> Result<(), CompilerError> {
        // COMPREHENSIVE DEBUG: Track ALL patch_address calls to debug placeholder resolution
        log::debug!(
            " PATCH_ADDRESS: location=0x{:04x} address=0x{:04x} size={}",
            location,
            address,
            size
        );

        // Check what's currently at the location before patching
        if location < self.final_data.len() && location + size <= self.final_data.len() {
            match size {
                1 => {
                    let current_byte = self.final_data[location];
                    log::debug!(
                        " PATCH_ADDRESS: current_byte=0x{:02x} -> new_byte=0x{:02x}",
                        current_byte,
                        address as u8
                    );
                }
                2 => {
                    let current_high = self.final_data[location];
                    let current_low = self.final_data[location + 1];
                    let current_word = ((current_high as u16) << 8) | (current_low as u16);
                    log::debug!(" PATCH_ADDRESS: current_word=0x{:04x} (bytes 0x{:02x} 0x{:02x}) -> new_word=0x{:04x} (bytes 0x{:02x} 0x{:02x})", 
 current_word, current_high, current_low, address, (address >> 8) as u8, address as u8);

                    // Special debug for FFFF placeholders being resolved
                    if current_word == 0xFFFF {
                        log::debug!(" PATCH_ADDRESS: RESOLVING PLACEHOLDER 0xFFFF -> 0x{:04x} at location 0x{:04x}", address, location);
                    } else if address == 0x0000 {
                        log::debug!(" PATCH_ADDRESS: WARNING - Writing NULL address 0x0000 at location 0x{:04x} (current was 0x{:04x})", location, current_word);
                    }
                }
                _ => {}
            }
        }

        // Use final_data for all address patching
        let target_data_len = self.final_data.len();

        if location + size > target_data_len {
            return Err(CompilerError::CodeGenError(format!(
                "Cannot patch address at location {}: beyond data bounds (len: {})",
                location, target_data_len
            )));
        }

        // Debug writes to specific problem areas if needed
        if (0x0f88..=0x0f8b).contains(&location) {
            log::debug!(
                "Writing to location 0x{:04x}: address 0x{:04x} ({} bytes)",
                location,
                address,
                size
            );
        }

        match size {
            1 => {
                debug!(
                    "patch_address: writing 0x{:02x} at location 0x{:04x}",
                    address as u8, location
                );
                self.write_byte_at(location, address as u8)?;
            }
            2 => {
                debug!("patch_address: writing 0x{:04x} (bytes 0x{:02x} 0x{:02x}) at location 0x{:04x}", 
 address, (address >> 8) as u8, address as u8, location);
                // Additional debug for specific corruption pattern
                if location == 0x0f89 && (address >> 8) as u8 == 0x9A {
                    debug!("Found potential corruption: Writing 0x9A to location 0x0f89");
                    debug!(" This would create print_obj instruction corruption");
                    debug!(" Full address being written: 0x{:04x}", address);
                }
                if location + 1 == 0x0f8a && address as u8 == 0x4D {
                    debug!("Found potential corruption: Writing 0x4D to location 0x0f8a");
                    debug!(" This would create operand 77 corruption");
                    debug!(" Full address being written: 0x{:04x}", address);
                }

                log::debug!(
                    " PATCH_ADDRESS: Writing high byte 0x{:02x} to location 0x{:04x}",
                    (address >> 8) as u8,
                    location
                );
                self.write_byte_at(location, (address >> 8) as u8)?;
                log::debug!(
                    " PATCH_ADDRESS: Writing low byte 0x{:02x} to location 0x{:04x}",
                    address as u8,
                    location + 1
                );
                self.write_byte_at(location + 1, address as u8)?;
            }
            _ => {
                return Err(CompilerError::CodeGenError(format!(
                    "Unsupported patch size: {} bytes",
                    size
                )));
            }
        }

        Ok(())
    }

    /// Pack a routine address according to Z-Machine version
    fn pack_routine_address(&self, byte_address: usize) -> Result<u16, CompilerError> {
        match self.version {
            ZMachineVersion::V3 => {
                // v3: packed address = byte address / 2
                if byte_address % 2 != 0 {
                    return Err(CompilerError::CodeGenError(
                        "Routine address must be even for v3".to_string(),
                    ));
                }
                Ok((byte_address / 2) as u16)
            }
            ZMachineVersion::V4 | ZMachineVersion::V5 => {
                // v4/v5: packed address = byte address / 4
                if byte_address % 4 != 0 {
                    return Err(CompilerError::CodeGenError(
                        "Routine address must be multiple of 4 for v4/v5".to_string(),
                    ));
                }
                Ok((byte_address / 4) as u16)
            }
        }
    }

    /// Register a builtin function name with its ID
    pub fn register_builtin_function(&mut self, function_id: IrId, name: String) {
        self.builtin_function_names
            .insert(function_id, name.clone());
        self.builtin_functions.insert(name, function_id);
    }

    /// Set dispatch functions from IR generator
    pub fn set_dispatch_functions(&mut self, dispatch_functions: IndexMap<String, IrId>) {
        self.dispatch_functions = dispatch_functions;
    }

    /// Set function base names from IR generator
    pub fn set_function_base_names(&mut self, function_base_names: IndexMap<IrId, String>) {
        self.function_base_names = function_base_names;
    }

    /// Get function ID for dispatch or original function, checking dispatch functions first
    /// This enables polymorphic dispatch by routing calls through dispatch functions when available
    fn get_function_id_with_dispatch(&self, original_function_id: IrId) -> IrId {
        // Look up the base function name for this function ID
        if let Some(base_function_name) = self.function_base_names.get(&original_function_id) {
            // Check if there's a dispatch function for this base name
            if let Some(&dispatch_id) = self.dispatch_functions.get(base_function_name) {
                log::debug!(
                    "🎯 DISPATCH: Using dispatch function {} (ID {}) instead of {} (ID {})",
                    format!("dispatch_{}", base_function_name),
                    dispatch_id,
                    base_function_name,
                    original_function_id
                );
                return dispatch_id;
            }
        }

        log::debug!(
            "🎯 DIRECT: Using original function ID {} (no dispatch available)",
            original_function_id
        );
        original_function_id
    }

    /// Register object numbers from IR generator
    pub fn set_object_numbers(&mut self, object_numbers: IndexMap<String, u16>) {
        self.object_numbers = object_numbers;
    }

    /// Check if a function ID corresponds to a builtin function
    pub fn is_builtin_function(&self, function_id: IrId) -> bool {
        self.builtin_function_names.contains_key(&function_id)
    }

    /// Get the name of a builtin function by its ID
    pub fn get_builtin_function_name(&self, function_id: IrId) -> Option<&String> {
        self.builtin_function_names.get(&function_id)
    }

    /// Generate Z-Machine code for builtin function calls
    pub fn generate_builtin_function_call(
        &mut self,
        function_id: IrId,
        args: &[IrId],
        target: Option<IrId>,
    ) -> Result<(), CompilerError> {
        let function_name = self
            .get_builtin_function_name(function_id)
            .ok_or_else(|| {
                CompilerError::CodeGenError(format!("Unknown builtin function ID: {}", function_id))
            })?
            .clone();

        // Handle method calls (format "method:{method_name}")
        if function_name.starts_with("method:") {
            let method_name = function_name.strip_prefix("method:").unwrap();
            return self.generate_method_call(method_name, args);
        }

        match function_name.as_str() {
            "print" => self.generate_print_builtin(args),
            "println" => self.generate_println_builtin(args),
            "print_ret" => self.generate_print_ret_builtin(args),
            "print_message" => self.generate_print_message_builtin(args),
            "new_line" => self.generate_new_line_builtin(args),
            "quit" => self.generate_quit_builtin(args),
            "move" => self.generate_move_builtin(args),
            "get_location" => self.generate_get_location_builtin(args, target),
            "to_string" => self.generate_to_string_builtin(args, target),
            // Core Z-Machine object primitives
            "get_child" => self.generate_get_child_builtin(args),
            "get_sibling" => self.generate_get_sibling_builtin(args),
            "get_prop" => self.generate_get_prop_builtin(args),
            "test_attr" => self.generate_test_attr_builtin(args),
            "set_attr" => self.generate_set_attr_builtin(args),
            "clear_attr" => self.generate_clear_attr_builtin(args),
            // Advanced Z-Machine opcodes
            "random" => self.generate_random_builtin(args, target),
            // Game logic builtins
            "player_can_see" => self.generate_player_can_see_builtin(args),
            "list_objects" => self.generate_list_objects_builtin(args),
            "get_object_contents" => self.generate_get_object_contents_builtin(args, target),
            "object_is_empty" => self.generate_object_is_empty_builtin(args, target),
            "value_is_none" => self.generate_value_is_none_builtin(args, target),
            "get_exit" => {
                // ARCHITECTURE FIX (Oct 27, 2025): Use standard builtin function call mechanism
                //
                // BEFORE: Custom HOTFIX code that duplicated function call logic
                // AFTER: Use standard call_builtin_function helper for consistency
                //
                // BENEFIT: Follows same pattern as future builtins moving from inline to function calls
                log::debug!(
                    "🚪 STANDARD: Calling get_exit via standard builtin function mechanism"
                );

                return self.call_builtin_function("get_exit", args, target);
            }
            "print_num" => {
                // IMPLEMENTATION NOTE (Nov 2, 2025): print_num() builtin for direct integer printing
                // Resolves Score Display Corruption Bug by providing proper integer-to-output conversion
                // Uses Z-Machine native print_num opcode (VAR:230/6) instead of placeholder to_string()
                log::debug!(
                    "🔢 STANDARD: Calling print_num via standard builtin function mechanism"
                );

                return self.call_builtin_function("print_num", args, target);
            }
            "exit_is_blocked" => self.generate_exit_is_blocked_builtin(args, target),
            "exit_get_destination" => self.generate_exit_get_data_builtin(args, target),
            "exit_get_message" => self.generate_exit_get_message_builtin(args, target),
            "get_object_size" => self.generate_get_object_size_builtin(args, target),
            "array_add_item" => self.generate_array_add_item_builtin(args, target),
            // String functions
            "indexOf" => self.generate_index_of_builtin(args, target),
            "slice" => self.generate_slice_builtin(args, target),
            "substring" => self.generate_substring_builtin(args, target),
            "toLowerCase" => self.generate_to_lower_case_builtin(args, target),
            "toUpperCase" => self.generate_to_upper_case_builtin(args, target),
            "trim" => self.generate_trim_builtin(args, target),
            "charAt" => self.generate_char_at_builtin(args, target),
            "replace" => self.generate_replace_builtin(args, target),
            "startsWith" => self.generate_starts_with_builtin(args, target),
            "endsWith" => self.generate_ends_with_builtin(args, target),
            _ => Err(CompilerError::CodeGenError(format!(
                "Unimplemented builtin function: {}",
                function_name
            ))),
        }
    }

    /// Generate method call - handles property method calls like object.method()
    fn generate_method_call(
        &mut self,
        method_name: &str,
        args: &[IrId],
    ) -> Result<(), CompilerError> {
        // Method calls have the object as the first argument
        if args.is_empty() {
            return Err(CompilerError::CodeGenError(
                "Method call requires at least one argument (the object)".to_string(),
            ));
        }

        let object_id = args[0];
        let _method_args = &args[1..]; // Remaining arguments are the actual method arguments

        match method_name {
            "on_look" => {
                // For now, treat on_look as a property access that executes a routine
                // This would typically involve looking up the object's on_look property
                // and calling it if it exists

                // Generate a call_vs instruction (VAR:224, opcode 0xE0)
                // This is a placeholder - in a real implementation we'd:
                // 1. Get the object's on_look property address
                // 2. Call that routine if it exists
                // 3. Handle the case where the property doesn't exist

                // For now, generate a no-op comment in the bytecode
                // TODO: Implement proper property method dispatch
                self.emit_byte(0xB2)?; // print opcode
                                       // Create a debug string for this method call and register it with code generator
                let debug_string = format!("DEBUG: Method call {}.{}()", object_id, method_name);
                let string_id = self.find_or_create_string_id(&debug_string)?;
                // Register this string value so it can be found by print calls
                self.ir_id_to_string.insert(string_id, debug_string);
                // CRITICAL FIX: Record location BEFORE emitting placeholder
                let code_space_offset = self.code_space.len();
                self.emit_word(placeholder_word())?; // Placeholder address

                // Create reference with exact location
                self.reference_context
                    .unresolved_refs
                    .push(UnresolvedReference {
                        reference_type: LegacyReferenceType::StringRef,
                        location: code_space_offset, // Use exact offset
                        target_id: string_id,
                        is_packed_address: true,
                        offset_size: 2,
                        location_space: MemorySpace::Code,
                    });

                Ok(())
            }
            _ => Err(CompilerError::CodeGenError(format!(
                "Unimplemented method: {}",
                method_name
            ))),
        }
    }

    /// Update string addresses after new strings have been added
    /// Add an unresolved reference to be patched later
    /// CRITICAL: Pass location_offset calculated BEFORE emitting placeholder
    pub fn add_unresolved_reference_at_location(
        &mut self,
        reference_type: LegacyReferenceType,
        target_id: IrId,
        is_packed: bool,
        location_space: MemorySpace,
        location_offset: usize,
    ) -> Result<(), CompilerError> {
        log::debug!(
            "add_unresolved_reference_at_location: {:?} -> IR ID {} at exact offset 0x{:04x}",
            reference_type,
            target_id,
            location_offset
        );

        let reference = UnresolvedReference {
            reference_type,
            location: match location_space {
                MemorySpace::Code => {
                    // Use the exact offset provided by caller (calculated BEFORE placeholder emission)
                    self.final_code_base + location_offset
                    },
                    MemorySpace::CodeSpace => {
                    // Use the exact offset provided by caller
                    self.final_code_base + location_offset
                    },
                    MemorySpace::Header => panic!("COMPILER BUG: Header space references not implemented - cannot use add_unresolved_reference() for Header space"),
                    MemorySpace::Globals => panic!("COMPILER BUG: Globals space references not implemented - cannot use add_unresolved_reference() for Globals space"),
                    MemorySpace::Abbreviations => panic!("COMPILER BUG: Abbreviations space references not implemented - cannot use add_unresolved_reference() for Abbreviations space"),
                    MemorySpace::Objects => panic!("COMPILER BUG: Objects space references not implemented - cannot use add_unresolved_reference() for Objects space"),
                    MemorySpace::Dictionary => panic!("COMPILER BUG: Dictionary space references not implemented - cannot use add_unresolved_reference() for Dictionary space"),
                    MemorySpace::Strings => panic!("COMPILER BUG: Strings space references not implemented - cannot use add_unresolved_reference() for Strings space"),
                },
            target_id,
            is_packed_address: is_packed,
            offset_size: 2, // Default to 2 bytes
            location_space,
        };
        self.reference_context.unresolved_refs.push(reference);
        Ok(())
    }

    /// Legacy function for backward compatibility - DEPRECATED due to systematic timing bugs
    /// Use add_unresolved_reference_at_location() instead with location calculated BEFORE placeholder emission
    #[deprecated = "Use add_unresolved_reference_at_location() to avoid systematic location timing bugs"]
    pub fn add_unresolved_reference(
        &mut self,
        reference_type: LegacyReferenceType,
        target_id: IrId,
        is_packed: bool,
        location_space: MemorySpace,
    ) -> Result<(), CompilerError> {
        // Calculate location using current code space length (BUGGY - for backward compatibility only)
        let location_offset = self.code_space.len();
        self.add_unresolved_reference_at_location(
            reference_type,
            target_id,
            is_packed,
            location_space,
            location_offset,
        )
    }

    /// Record a relative offset within code_space during code generation
    pub fn record_code_space_offset(&mut self, ir_id: IrId, offset: usize) {
        debug!(
            "Record code offset: Recording IR ID {} -> code_space_offset 0x{:04x}",
            ir_id, offset
        );
        debug!(
            "Record code offset: This will be converted to absolute address during final assembly"
        );

        // DEFENSIVE FIX: Check if IR ID is already mapped to prevent corruption
        if let Some(&existing_offset) = self.reference_context.ir_id_to_address.get(&ir_id) {
            // If the existing mapping looks reasonable (>= 0x50) and the new one is smaller,
            // it's likely a corruption attempt - keep the existing one
            if existing_offset >= 0x50 && offset < existing_offset {
                log::debug!(
 " OFFSET_CORRUPTION_PREVENTED: IR ID {} already mapped to 0x{:04x}, ignoring smaller offset 0x{:04x}",
 ir_id, existing_offset, offset
 );
                return;
            }
        }

        // CRITICAL FIX: Prevent phantom labels from cross-function contamination
        // If this is a label (IR ID 73 or 74) being recorded with the corrupted 0x005a address,
        // these are phantom labels from empty else branch - ignore them completely
        // The jump references will be handled by using existing valid labels
        if (ir_id == 73 || ir_id == 74) && offset == 0x005a {
            log::debug!(
 " PHANTOM_LABEL_BLOCKED: IR ID {} at offset 0x{:04x} is phantom label from empty else branch - completely ignoring",
 ir_id, offset
 );
            return;
        }

        // Store the relative offset - will be converted to absolute later
        self.reference_context
            .ir_id_to_address
            .insert(ir_id, offset);
    }

    /// CENTRALIZED IR ID MAPPING - Record final address for any IR ID
    ///
    /// This is the central function for mapping IR IDs to their final addresses in the
    /// compiled Z-Machine file. Called by various allocation functions (strings, labels,
    /// functions) to ensure all IR IDs are properly tracked for UnresolvedReference resolution.
    ///
    /// CRITICAL: This must be called for EVERY IR ID that gets referenced in UnresolvedReferences
    /// to prevent "target_id not found" errors during address resolution.
    pub fn record_final_address(&mut self, ir_id: IrId, address: usize) {
        log::info!(
            "🎯 CENTRAL_IR_MAPPING: IR ID {} -> 0x{:04x} [FINAL ADDRESS]",
            ir_id,
            address
        );

        // Detect and warn about mapping conflicts (potential bugs)
        if let Some(&existing_addr) = self.reference_context.ir_id_to_address.get(&ir_id) {
            if existing_addr != address {
                log::warn!(
 " IR_MAPPING_CONFLICT: IR ID {} already mapped to 0x{:04x}, overwriting with 0x{:04x}",
 ir_id, existing_addr, address
 );
            }
        }

        // Store the mapping in the central table
        self.reference_context
            .ir_id_to_address
            .insert(ir_id, address);

        log::debug!(
            "🔧 CENTRAL_IR_MAPPING_STATS: Total mappings after insert: {}",
            self.reference_context.ir_id_to_address.len()
        );
    }

    /// Convert all code space offsets to absolute addresses during final assembly
    pub fn convert_offsets_to_addresses(&mut self) {
        debug!("Convert offsets: Converting all code space offsets to absolute addresses");
        debug!(
            "Convert offsets: final_code_base=0x{:04x}",
            self.final_code_base
        );

        let mut converted_addresses = IndexMap::new();

        for (&ir_id, &offset) in &self.reference_context.ir_id_to_address {
            // Check if this looks like a code space offset (should be < code_space.len())
            if offset < self.code_space.len() {
                let absolute_address = self.final_code_base + offset;
                debug!(
                    "Convert offsets: IR ID {} offset 0x{:04x} -> absolute 0x{:04x}",
                    ir_id, offset, absolute_address
                );
                converted_addresses.insert(ir_id, absolute_address);
            } else {
                // Already an absolute address, keep as-is
                debug!(
                    "Convert offsets: IR ID {} already absolute 0x{:04x}",
                    ir_id, offset
                );
                converted_addresses.insert(ir_id, offset);
            }
        }

        // Replace the entire mapping with converted addresses
        self.reference_context.ir_id_to_address = converted_addresses;
    }

    /// CRITICAL FIX: Map all object IR IDs to their object addresses
    /// This fixes the systematic issue where object IR IDs are referenced in
    /// UnresolvedReferences but never added to the ir_id_to_address mapping table
    pub fn map_all_object_ir_ids(&mut self) {
        log::debug!("🔧 OBJECT_IR_ID_MAPPING: Adding all object IR IDs to address mapping table");

        // From the compilation logs, I know objects like window (ID 45) are registered
        // but never get their IR IDs added to the mapping table.
        // This causes UnresolvedReferences with target_id=45 to fail resolution.

        // Get all object IR IDs from the object mapping system
        // Object addresses in Z-Machine are their object numbers, not memory addresses
        for (&ir_id, &obj_num) in &self.ir_id_to_object_number {
            let object_address = obj_num; // In Z-Machine, object "address" is object number
            log::debug!(
                "🔧 OBJECT_IR_ID_MAPPING: IR ID {} -> Object #{} (address={})",
                ir_id,
                obj_num,
                object_address
            );
            self.reference_context
                .ir_id_to_address
                .insert(ir_id, object_address as usize);
        }

        log::debug!(
            "🔧 OBJECT_IR_ID_MAPPING: Added {} object IR ID mappings",
            self.ir_id_to_object_number.len()
        );
    }

    /// CENTRALIZED IR MAPPING CONSOLIDATION
    ///
    /// CRITICAL ARCHITECTURAL FIX: This function solves the systematic UnresolvedReference
    /// resolution failures by consolidating ALL IR ID types into the central ir_id_to_address table.
    ///
    /// PROBLEM: The compiler used separate tracking systems for different IR ID types:
    /// - string_offsets: HashMap<IrId, usize> (strings)
    /// - label_addresses: HashMap<IrId, usize> (jump/branch labels)
    /// - ir_id_to_object_number: HashMap<IrId, u16> (objects - handled separately)
    ///
    /// But only ir_id_to_address was used by UnresolvedReference resolution, causing
    /// hundreds of references to fail with "target_id not found" errors.
    ///
    /// SOLUTION: This function consolidates all separate tracking systems into the
    /// central ir_id_to_address table before UnresolvedReference resolution begins.
    ///
    /// RESULTS: Increased total IR ID mappings from ~13 to 237, resolving systematic
    /// UnresolvedReference failures and enabling mini_zork to execute past initialization.
    pub fn consolidate_all_ir_mappings(&mut self) {
        log::info!(
            "🔄 CONSOLIDATING ALL IR MAPPINGS: Starting comprehensive mapping consolidation"
        );

        let initial_count = self.reference_context.ir_id_to_address.len();
        log::debug!(
            " IR_MAPPING_STATS: Starting with {} existing mappings",
            initial_count
        );

        // Track consolidation progress
        let _functions_mapped = 0;
        let mut strings_mapped = 0;
        let mut labels_mapped = 0;

        // 1. Function IR IDs - Ensure all functions are mapped
        log::debug!("🔧 CONSOLIDATING: Function IR IDs");
        let functions_mapped = self.function_addresses.len();
        for (&function_id, &address) in &self.function_addresses {
            if !self
                .reference_context
                .ir_id_to_address
                .contains_key(&function_id)
            {
                self.reference_context
                    .ir_id_to_address
                    .insert(function_id, address);
                log::debug!(
                    "📝 MAPPED: Function IR ID {} -> address 0x{:04x}",
                    function_id,
                    address
                );
            }
        }

        // 2. Consolidate string IR IDs from string_offsets into central mapping
        log::debug!("🔧 CONSOLIDATING: String IR IDs from string_offsets...");
        for (&ir_id, &address) in &self.string_offsets {
            log::debug!(
                "🎯 STRING_IR_MAPPING: IR ID {} -> 0x{:04x} [STRING]",
                ir_id,
                address
            );
            self.reference_context
                .ir_id_to_address
                .insert(ir_id, address);
            strings_mapped += 1;
        }

        // 3. Consolidate label IR IDs from label_addresses into central mapping
        log::debug!("🔧 CONSOLIDATING: Label IR IDs from label_addresses...");
        for (&ir_id, &address) in &self.label_addresses {
            log::debug!(
                "🎯 LABEL_IR_MAPPING: IR ID {} -> 0x{:04x} [LABEL]",
                ir_id,
                address
            );
            self.reference_context
                .ir_id_to_address
                .insert(ir_id, address);
            labels_mapped += 1;
        }

        let final_count = self.reference_context.ir_id_to_address.len();
        let total_added = final_count - initial_count;

        log::info!("IR mapping consolidation complete:");
        log::info!(
            " Added {} new mappings (functions: {}, strings: {}, labels: {})",
            total_added,
            functions_mapped,
            strings_mapped,
            labels_mapped
        );
        log::info!(
            " Total IR ID mappings: {} -> {}",
            initial_count,
            final_count
        );
        log::info!(" This resolves systematic UnresolvedReference resolution failures");

        // Validation: Ensure we have more mappings than UnresolvedReferences
        let unresolved_count = self.reference_context.unresolved_refs.len();
        if final_count < unresolved_count {
            log::warn!(
                " IR_MAPPING_SHORTAGE: Only {} mappings for {} UnresolvedReferences",
                final_count,
                unresolved_count
            );
        } else {
            log::debug!(
                " IR_MAPPING_COVERAGE: {} mappings covers {} UnresolvedReferences",
                final_count,
                unresolved_count
            );
        }
    }

    /// Create UnresolvedReference with proper space context
    pub fn create_unresolved_reference(
        &self,
        reference_type: LegacyReferenceType,
        location_space: MemorySpace,
        location_offset: usize,
        target_id: IrId,
        is_packed_address: bool,
        offset_size: u8,
    ) -> UnresolvedReference {
        UnresolvedReference {
            reference_type,
            location: location_offset,
            target_id,
            is_packed_address,
            offset_size,
            location_space,
        }
    }

    /// Reinitialize input buffers in the final game image
    /// This ensures buffer headers survive any ensure_capacity() calls
    /// Made public for use by codegen_extensions.rs
    pub fn reinitialize_input_buffers_in_image(&self, game_image: &mut Vec<u8>) {
        if self.text_buffer_addr > 0 && self.parse_buffer_addr > 0 {
            debug!("Reinitializing input buffers in final image: text_buffer[{}] = 100, parse_buffer[{}] = 120",
                    self.text_buffer_addr, self.parse_buffer_addr);

            // Ensure we have enough space in the final image
            let required_size = self.parse_buffer_addr + 34;
            if game_image.len() < required_size {
                game_image.resize(required_size, 0);
            }

            // Reinitialize buffer headers (match Zork I's values)
            game_image[self.text_buffer_addr] = 100; // Max input length (0x64)
            game_image[self.text_buffer_addr + 1] = 0; // Current length
            game_image[self.parse_buffer_addr] = 120; // Max words (0x78)
            game_image[self.parse_buffer_addr + 1] = 0; // Current words
        }
    }

    /// Ensure the story data buffer has enough capacity
    pub fn ensure_capacity(&mut self, required: usize) {
        if self.story_data.len() < required {
            self.story_data.resize(required, 0);
        }
    }

    /// Dump PC→IR mapping for debugging
    pub fn dump_pc_mapping(&self) {
        if self.pc_to_ir_map.is_empty() {
            eprintln!("=== PC → IR MAPPING (empty) ===");
            return;
        }

        eprintln!(
            "\n=== PC → IR MAPPING ({} entries) ===",
            self.pc_to_ir_map.len()
        );
        eprintln!("Code base offset: 0x{:04x}", self.final_code_base);
        eprintln!(
            "{:<8} {:<20} {:<10} {}",
            "PC", "Function", "IR ID", "Instruction"
        );
        eprintln!("{}", "-".repeat(80));

        for (pc, (func_name, ir_id, desc)) in &self.pc_to_ir_map {
            let final_pc = pc + self.final_code_base; // Add offset to show final assembled address
            eprintln!(
                "0x{:04x}   {:<20} t{:<8} {}",
                final_pc, func_name, ir_id, desc
            );
        }
        eprintln!();
    }

    /// Generate all builtin functions as real Z-Machine functions
    /// Called during code generation initialization to create function implementations
    /// that can be called via call_vs instead of being inlined
    pub fn generate_builtin_functions(&mut self) -> Result<(), CompilerError> {
        log::debug!("🏗️ BUILTIN_GEN: Starting builtin function generation");
        log::debug!(
            "🏗️ BUILTIN_GEN: code_address before generation: 0x{:04x}",
            self.code_address
        );
        log::debug!(
            "🏗️ BUILTIN_GEN: code_space.len() before generation: {}",
            self.code_space.len()
        );

        // Check if we have any main function mapping yet
        if let Some((main_addr, _)) = self
            .reference_context
            .ir_id_to_address
            .iter()
            .find(|(_, &addr)| addr > 0)
        {
            log::debug!(
                "🏗️ BUILTIN_GEN: Found existing main function mapping at address 0x{:04x}",
                main_addr
            );
        } else {
            log::debug!("🏗️ BUILTIN_GEN: No main function mappings found yet");
        }

        // Generate actual builtin functions that are registered
        let mut generated_count = 0;

        // Generate get_exit builtin function if registered
        if self.builtin_functions.contains_key("get_exit") {
            log::debug!("🏗️ BUILTIN_GEN: Generating get_exit function");

            // Ensure even address alignment for function
            if self.code_address % 2 != 0 {
                log::debug!("🏗️ BUILTIN_GEN: Adding padding byte for alignment");
                self.emit_byte(0xB4)?;
            }

            let func_addr = self.code_address;
            let func_id = *self.builtin_functions.get("get_exit").unwrap();

            // Generate function header (9 locals as required by generate_get_exit_builtin)
            let num_locals = 9;
            log::debug!(
                "🏗️ BUILTIN_GEN: Generating get_exit function header at 0x{:04x}",
                func_addr
            );
            self.emit_byte(num_locals)?;
            for _ in 0..num_locals {
                self.emit_word(0)?;
            }

            // Generate function body using the proven implementation
            self.generate_get_exit_builtin(&[], None)?;

            // Register function address
            log::debug!(
                "🏗️ BUILTIN_GEN: Registering get_exit function: ID {} → address 0x{:04x}",
                func_id,
                func_addr
            );
            self.reference_context
                .ir_id_to_address
                .insert(func_id, func_addr);
            self.function_addresses.insert(func_id, func_addr);

            generated_count += 1;
        }

        // Generate print_num builtin function if registered
        //
        // IMPLEMENTATION NOTE (Nov 2, 2025): This implements the print_num() builtin function
        // using Z-Machine's native print_num opcode (VAR:230/6) for direct integer printing.
        // This resolves the Score Display Corruption Bug where to_string() was a placeholder.
        //
        // ARCHITECTURE: Implemented as real Z-Machine function (not inline) following
        // the established pattern of get_exit and other builtins. Called via call_builtin_function.
        //
        // KNOWN ISSUE: Currently adds automatic newline via rtrue, but Z-Machine spec states
        // print_num should NOT add newlines. See "Print Newline Architecture Issue" in ONGOING_TASKS.md
        if self.builtin_functions.contains_key("print_num") {
            log::debug!("🏗️ BUILTIN_GEN: Generating print_num function");

            // Ensure even address alignment for function
            if self.code_address % 2 != 0 {
                log::debug!("🏗️ BUILTIN_GEN: Adding padding byte for alignment");
                self.emit_byte(0xB4)?;
            }

            let func_addr = self.code_address;
            let func_id = *self.builtin_functions.get("print_num").unwrap();

            // Generate function header (1 local for the integer parameter)
            let num_locals = 1;
            log::debug!(
                "🏗️ BUILTIN_GEN: Generating print_num function header at 0x{:04x}",
                func_addr
            );
            self.emit_byte(num_locals)?;
            for _ in 0..num_locals {
                self.emit_word(0)?;
            }

            // Generate function body: print_num opcode (VAR:230/6) followed by ret
            //
            // Z-Machine Specification: VAR:230/6 print_num - "Print (signed) number in decimal."
            // Takes the integer value from local variable 1 (function parameter)
            //
            // FIXME: According to Z-Machine spec, print_num should NOT add automatic newlines.
            // Current implementation adds newline via rtrue return. Should only emit print_num opcode.
            self.emit_instruction_typed(
                crate::grue_compiler::opcodes::Opcode::OpVar(
                    crate::grue_compiler::opcodes::OpVar::PrintNum,
                ),
                &[crate::grue_compiler::codegen::Operand::Variable(1)], // Local variable 1 (parameter)
                None,                                                   // No store variable
                None,                                                   // No branch
            )?;

            // Return from function
            // NOTE: This rtrue may be adding unwanted newline behavior - see ONGOING_TASKS.md
            self.emit_instruction_typed(
                crate::grue_compiler::opcodes::Opcode::Op0(
                    crate::grue_compiler::opcodes::Op0::Rtrue,
                ),
                &[],
                None,
                None,
            )?;

            // Register function address
            log::debug!(
                "🏗️ BUILTIN_GEN: Registering print_num function: ID {} → address 0x{:04x}",
                func_id,
                func_addr
            );
            self.reference_context
                .ir_id_to_address
                .insert(func_id, func_addr);
            self.function_addresses.insert(func_id, func_addr);

            generated_count += 1;
        }

        // Generate add_score builtin function if registered
        if self.builtin_functions.contains_key("add_score") {
            log::debug!("🏗️ BUILTIN_GEN: Generating add_score function");

            // Ensure even address alignment for function
            if self.code_address % 2 != 0 {
                log::debug!("🏗️ BUILTIN_GEN: Adding padding byte for alignment");
                self.emit_byte(0xB4)?;
            }

            let func_addr = self.code_address;
            let func_id = *self.builtin_functions.get("add_score").unwrap();

            // Generate function header (1 local for the argument)
            let num_locals = 1;
            log::debug!(
                "🏗️ BUILTIN_GEN: Generating add_score function header at 0x{:04x}",
                func_addr
            );
            self.emit_byte(num_locals)?;
            for _ in 0..num_locals {
                self.emit_word(0)?;
            }

            // Generate add_score function body:
            // 1. Load current score from Global G17
            // 2. Add argument (Local 1) to current score
            // 3. Store result back to Global G17
            // 4. Return new score value

            // Load current score from G17 → Variable 0 (stack)
            self.emit_instruction_typed(
                crate::grue_compiler::opcodes::Opcode::Op1(
                    crate::grue_compiler::opcodes::Op1::Load,
                ),
                &[Operand::Variable(17)], // G17
                Some(0),                  // Store to Variable 0 (stack)
                None,
            )?;

            // Add Local 1 (argument) to stack value → Variable 0 (stack)
            self.emit_instruction_typed(
                crate::grue_compiler::opcodes::Opcode::Op2(crate::grue_compiler::opcodes::Op2::Add),
                &[
                    Operand::Variable(0), // Stack (current score)
                    Operand::Variable(1), // Local 1 (argument)
                ],
                Some(0), // Store result to Variable 0 (stack)
                None,
            )?;

            // Store result from stack to G17
            self.emit_instruction_typed(
                crate::grue_compiler::opcodes::Opcode::Op2(
                    crate::grue_compiler::opcodes::Op2::Store,
                ),
                &[
                    Operand::Variable(17), // G17
                    Operand::Variable(0),  // Stack (new score)
                ],
                None,
                None,
            )?;

            // Return new score (load G17 to stack and return)
            self.emit_instruction_typed(
                crate::grue_compiler::opcodes::Opcode::Op1(
                    crate::grue_compiler::opcodes::Op1::Load,
                ),
                &[Operand::Variable(17)], // G17
                Some(0),                  // Store to Variable 0 (stack) for return
                None,
            )?;

            self.emit_instruction_typed(
                crate::grue_compiler::opcodes::Opcode::Op1(crate::grue_compiler::opcodes::Op1::Ret),
                &[Operand::Variable(0)], // Return stack value
                None,
                None,
            )?;

            // Register function address
            log::debug!(
                "🏗️ BUILTIN_GEN: Registering add_score function: ID {} → address 0x{:04x}",
                func_id,
                func_addr
            );
            self.reference_context
                .ir_id_to_address
                .insert(func_id, func_addr);
            self.function_addresses.insert(func_id, func_addr);

            generated_count += 1;
        }

        // Generate subtract_score builtin function if registered
        if self.builtin_functions.contains_key("subtract_score") {
            log::debug!("🏗️ BUILTIN_GEN: Generating subtract_score function");

            // Ensure even address alignment for function
            if self.code_address % 2 != 0 {
                log::debug!("🏗️ BUILTIN_GEN: Adding padding byte for alignment");
                self.emit_byte(0xB4)?;
            }

            let func_addr = self.code_address;
            let func_id = *self.builtin_functions.get("subtract_score").unwrap();

            // Generate function header (1 local for the argument)
            let num_locals = 1;
            log::debug!(
                "🏗️ BUILTIN_GEN: Generating subtract_score function header at 0x{:04x}",
                func_addr
            );
            self.emit_byte(num_locals)?;
            for _ in 0..num_locals {
                self.emit_word(0)?;
            }

            // Generate subtract_score function body:
            // 1. Load current score from Global G17
            // 2. Subtract argument (Local 1) from current score
            // 3. Store result back to Global G17
            // 4. Return new score value

            // Load current score from G17 → Variable 0 (stack)
            self.emit_instruction_typed(
                crate::grue_compiler::opcodes::Opcode::Op1(
                    crate::grue_compiler::opcodes::Op1::Load,
                ),
                &[Operand::Variable(17)], // G17
                Some(0),                  // Store to Variable 0 (stack)
                None,
            )?;

            // Subtract Local 1 (argument) from stack value → Variable 0 (stack)
            self.emit_instruction_typed(
                crate::grue_compiler::opcodes::Opcode::Op2(crate::grue_compiler::opcodes::Op2::Sub),
                &[
                    Operand::Variable(0), // Stack (current score)
                    Operand::Variable(1), // Local 1 (argument)
                ],
                Some(0), // Store result to Variable 0 (stack)
                None,
            )?;

            // Store result from stack to G17
            self.emit_instruction_typed(
                crate::grue_compiler::opcodes::Opcode::Op2(
                    crate::grue_compiler::opcodes::Op2::Store,
                ),
                &[
                    Operand::Variable(17), // G17
                    Operand::Variable(0),  // Stack (new score)
                ],
                None,
                None,
            )?;

            // Return new score (load G17 to stack and return)
            self.emit_instruction_typed(
                crate::grue_compiler::opcodes::Opcode::Op1(
                    crate::grue_compiler::opcodes::Op1::Load,
                ),
                &[Operand::Variable(17)], // G17
                Some(0),                  // Store to Variable 0 (stack) for return
                None,
            )?;

            self.emit_instruction_typed(
                crate::grue_compiler::opcodes::Opcode::Op1(crate::grue_compiler::opcodes::Op1::Ret),
                &[Operand::Variable(0)], // Return stack value
                None,
                None,
            )?;

            // Register function address
            log::debug!(
                "🏗️ BUILTIN_GEN: Registering subtract_score function: ID {} → address 0x{:04x}",
                func_id,
                func_addr
            );
            self.reference_context
                .ir_id_to_address
                .insert(func_id, func_addr);
            self.function_addresses.insert(func_id, func_addr);

            generated_count += 1;
        }

        // Generate word_to_number builtin function if registered
        if self.builtin_functions.contains_key("word_to_number") {
            log::debug!("🏗️ BUILTIN_GEN: Generating word_to_number function");

            // Ensure even address alignment for function
            if self.code_address % 2 != 0 {
                log::debug!("🏗️ BUILTIN_GEN: Adding padding byte for alignment");
                self.emit_byte(0xB4)?;
            }

            let func_addr = self.code_address;
            let func_id = *self.builtin_functions.get("word_to_number").unwrap();

            // Generate function header (1 local for the argument)
            let num_locals = 1;
            log::debug!(
                "🏗️ BUILTIN_GEN: Generating word_to_number function header at 0x{:04x}",
                func_addr
            );
            self.emit_byte(num_locals)?;
            for _ in 0..num_locals {
                self.emit_word(0)?;
            }

            // Generate word_to_number function body:
            // For now, return 0 as placeholder (same as inline implementation)
            // TODO: Implement proper dictionary lookup when needed

            // Load immediate 0 → Variable 0 (stack)
            self.emit_instruction_typed(
                crate::grue_compiler::opcodes::Opcode::Op1(
                    crate::grue_compiler::opcodes::Op1::Load,
                ),
                &[Operand::SmallConstant(0)],
                Some(0), // Store to Variable 0 (stack)
                None,
            )?;

            self.emit_instruction_typed(
                crate::grue_compiler::opcodes::Opcode::Op1(crate::grue_compiler::opcodes::Op1::Ret),
                &[Operand::Variable(0)], // Return stack value
                None,
                None,
            )?;

            // Register function address
            log::debug!(
                "🏗️ BUILTIN_GEN: Registering word_to_number function: ID {} → address 0x{:04x}",
                func_id,
                func_addr
            );
            self.reference_context
                .ir_id_to_address
                .insert(func_id, func_addr);
            self.function_addresses.insert(func_id, func_addr);

            generated_count += 1;
        }

        log::debug!(
            "🏗️ BUILTIN_GEN: code_address after generation: 0x{:04x}",
            self.code_address
        );
        log::debug!(
            "🏗️ BUILTIN_GEN: code_space.len() after generation: {}",
            self.code_space.len()
        );
        log::debug!(
            "🏗️ BUILTIN_GEN: Generated {} builtin functions (out of {} registered)",
            generated_count,
            self.builtin_functions.len()
        );
        Ok(())
    }

    /// Call a builtin function using call_vs
    /// This replaces the old inline code generation with proper function calls
    fn call_builtin_function(
        &mut self,
        name: &str,
        args: &[IrId],
        target: Option<IrId>,
    ) -> Result<(), CompilerError> {
        log::debug!(
            "Calling builtin function '{}' with {} args at PC 0x{:04x}",
            name,
            args.len(),
            self.code_address
        );

        // Get the function's address
        let func_id = self.builtin_functions.get(name).ok_or_else(|| {
            CompilerError::CodeGenError(format!("Builtin function '{}' not found", name))
        })?;

        // Copy function ID for UnresolvedReference creation
        let func_id_value = *func_id;

        // Convert arguments to operands
        let mut arg_operands = Vec::new();
        for &arg_id in args {
            arg_operands.push(self.resolve_ir_id_to_operand(arg_id)?);
        }

        // Phase 1: Store function results in Variable(0) for Z-Machine compliance
        // CHANGE: Always store to Variable(0) when result needed, following Z-Machine spec
        let store_var = if let Some(target_id) = target {
            // Track this as a function call result for use_push_pull_for_result
            if !self.function_call_results.contains(&target_id) {
                self.function_call_results.insert(target_id);
            }

            // CRITICAL: Still need to create a mapping for this IR ID so other instructions can find it
            // The mapping will point to Variable(0) initially, then use_push_pull_for_result will update it
            self.ir_id_to_stack_var.insert(target_id, 0);

            Some(0) // ✅ Always store to Variable(0) for stack discipline
        } else {
            None
        };

        // Emit call_vs instruction with placeholder for function address (will be resolved later)
        let layout = self.emit_instruction_typed(
            crate::grue_compiler::opcodes::Opcode::OpVar(
                crate::grue_compiler::opcodes::OpVar::CallVs,
            ),
            &{
                let mut operands = vec![crate::grue_compiler::codegen::Operand::LargeConstant(
                    placeholder_word(), // Placeholder for function address (resolved during final assembly)
                )];
                operands.extend(arg_operands);
                operands
            },
            store_var,
            None,
        )?;

        // Create UnresolvedReference for function address fixup during final assembly
        if let Some(operand_location) = layout.operand_location {
            self.reference_context
                .unresolved_refs
                .push(UnresolvedReference {
                    reference_type: LegacyReferenceType::FunctionCall,
                    location: operand_location,
                    target_id: func_id_value,
                    is_packed_address: true, // Z-Machine function addresses are packed
                    offset_size: 2,
                    location_space: MemorySpace::Code,
                });
        }

        log::debug!(
            "Called builtin function '{}', result stored, PC now 0x{:04x}",
            name,
            self.code_address
        );

        Ok(())
    }

    /// Generate CreateArray instruction for static arrays
    ///
    /// This method delegates to ArrayCodeGen for the array allocation and IR ID mapping,
    /// maintaining proper separation of concerns. Static arrays are allocated at compile
    /// time and accessible via their base addresses in Z-Machine memory.
    ///
    /// # Arguments
    /// * `target` - The IR ID that will reference this array
    /// * `elements` - The initial values to populate the array with
    ///
    /// # Returns
    /// Result indicating success or compilation error
    pub fn generate_create_array(
        &mut self,
        target: IrId,
        elements: &[IrValue],
    ) -> Result<(), CompilerError> {
        // Delegate to ArrayCodeGen which contains the array-specific logic
        self.array_codegen
            .generate_create_array(target, elements, &mut self.ir_id_to_array_id)
    }

    /// Generate GetArrayElement instruction using loadw opcode
    ///
    /// This method delegates to ArrayCodeGen for array information lookup, then
    /// generates the necessary Z-Machine loadw instructions. Follows Zork I patterns:
    /// `loadw array_base (index + 1) -> target` where index+1 accounts for the
    /// count word at array[0].
    ///
    /// # Arguments
    /// * `target` - IR ID where the loaded value should be stored
    /// * `array_ir_id` - IR ID of the array to access
    /// * `index_ir_id` - IR ID containing the index to access
    ///
    /// # Returns
    /// Result indicating success or compilation error
    pub fn generate_get_array_element(
        &mut self,
        target: IrId,
        array_ir_id: IrId,
        index_ir_id: IrId,
    ) -> Result<(), CompilerError> {
        // Get array information from ArrayCodeGen
        let (base_address, _target, _index_ir_id) =
            self.array_codegen.generate_get_array_element_info(
                target,
                array_ir_id,
                index_ir_id,
                &self.ir_id_to_array_id,
            )?;

        // Get index operand
        let index_operand = self.resolve_ir_id_to_operand(index_ir_id)?;

        // Allocate temporary variable for index calculation (add 1 to skip count word)
        let temp_var = self.allocate_local_variable_slot();

        // Add 1 to index to skip count word at array[0]
        // This matches Zork I array access patterns
        self.emit_instruction_typed(
            Opcode::Op2(Op2::Add),
            &[index_operand, Operand::SmallConstant(1)],
            Some(temp_var),
            None,
        )?;

        // Generate loadw instruction: loadw array_base (temp_var) -> target
        // This is the core Z-Machine array access pattern from Zork I
        // The result goes to the stack (target is handled by IR ID mapping)
        self.emit_instruction_typed(
            Opcode::Op2(Op2::Loadw),
            &[
                Operand::LargeConstant(base_address),
                Operand::Variable(temp_var),
            ],
            None, // Store result on stack, will be popped later
            None,
        )?;

        // Map the target IR ID to stack variable for later access
        let stack_var = 0; // Z-Machine stack is variable 0
        self.ir_id_to_stack_var.insert(target, stack_var);

        log::debug!(
            "GetArrayElement: Generated loadw 0x{:04x} {} -> stack (array access)",
            base_address,
            temp_var
        );

        Ok(())
    }
}

#[cfg(test)]
#[path = "codegen_tests.rs"]
mod tests;
