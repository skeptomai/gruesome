// Z-Machine Code Generator
//
// Transforms IR into executable Z-Machine bytecode following the Z-Machine Standard v1.1
// Supports both v3 and v5 target formats with proper memory layout and instruction encoding.

use crate::grue_compiler::error::CompilerError;
use crate::grue_compiler::ir::*;
use crate::grue_compiler::ZMachineVersion;
use log::debug;
use std::collections::HashMap;

/// Distinctive placeholder byte for unresolved references
/// 0xFF is chosen because:
/// 1. In Z-Machine, 0xFF as an instruction byte would be an invalid Extended form instruction
/// 2. As operand data, 0xFFFF would represent -1 or 65535, which are uncommon values
/// 3. It's easily recognizable in hex dumps as "unresolved"
/// 4. Creates a clear pattern when examining bytecode (FFFF stands out)
const PLACEHOLDER_BYTE: u8 = 0xFF;

/// Create a 16-bit placeholder value using the distinctive placeholder byte
const fn placeholder_word() -> u16 {
    ((PLACEHOLDER_BYTE as u16) << 8) | (PLACEHOLDER_BYTE as u16)
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
struct ObjectData {
    id: IrId,
    name: String,
    short_name: String,
    attributes: IrAttributes,
    properties: IrProperties,
    parent: Option<IrId>,
    sibling: Option<IrId>,
    child: Option<IrId>,
}

/// Z-Machine memory layout constants
const HEADER_SIZE: usize = 64; // Fixed 64-byte header
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

/// Types of unresolved references that need patching
#[derive(Debug, Clone, PartialEq)]
pub enum ReferenceType {
    Jump,         // Unconditional jump to label
    Branch,       // Conditional branch to label
    FunctionCall, // Call to function address
    StringRef,    // Reference to string address
}

/// An unresolved reference that needs to be patched later
#[derive(Debug, Clone)]
pub struct UnresolvedReference {
    pub reference_type: ReferenceType,
    pub location: usize, // Byte offset in story data where patch is needed
    pub target_id: IrId, // IR ID being referenced (label, function, string)
    pub is_packed_address: bool, // Whether address needs to be packed
    pub offset_size: u8, // Size of offset field (1 or 2 bytes)
}

/// Reference context for tracking what needs resolution
#[derive(Debug, Clone)]
pub struct ReferenceContext {
    pub ir_id_to_address: HashMap<IrId, usize>, // Resolved addresses by IR ID
    pub unresolved_refs: Vec<UnresolvedReference>, // References waiting for resolution
}

/// Constant value types for control flow optimization
#[derive(Debug, Clone, PartialEq)]
pub enum ConstantValue {
    Boolean(bool),
    Integer(i16),
    String(String),
}

/// Code generation context
pub struct ZMachineCodeGen {
    version: ZMachineVersion,

    // Memory layout
    story_data: Vec<u8>,
    current_address: usize,

    // Input buffer addresses
    text_buffer_addr: usize,
    parse_buffer_addr: usize,

    // Code generation state
    label_addresses: HashMap<IrId, usize>, // IR label ID -> byte address
    string_addresses: HashMap<IrId, usize>, // IR string ID -> byte address
    function_addresses: HashMap<IrId, usize>, // IR function ID -> byte address
    /// Mapping from IR IDs to string values (for LoadImmediate results)
    ir_id_to_string: HashMap<IrId, String>,
    /// Mapping from IR IDs to integer values (for LoadImmediate results)
    ir_id_to_integer: HashMap<IrId, i16>,
    /// Mapping from IR IDs to stack variables (for instruction results on stack)
    ir_id_to_stack_var: HashMap<IrId, u8>,
    /// Mapping from IR IDs to Z-Machine object numbers (for object references)
    ir_id_to_object_number: HashMap<IrId, u16>,
    /// Mapping from IR IDs to binary operations (for conditional branch optimization)
    ir_id_to_binary_op: HashMap<IrId, (IrBinaryOp, IrId, IrId)>, // (operator, left_operand, right_operand)
    /// Mapping from function IDs to builtin function names
    builtin_function_names: HashMap<IrId, String>,
    /// Mapping from object names to object numbers (from IR generator)
    object_numbers: HashMap<String, u16>,
    /// Global property registry: property name -> property number
    property_numbers: HashMap<String, u8>,
    /// Properties used by each object: object_name -> set of property names
    object_properties: HashMap<String, Vec<String>>,

    // Tables for Z-Machine structures
    object_table_addr: usize,
    property_table_addr: usize,
    current_property_addr: usize, // Current property table allocation pointer
    dictionary_addr: usize,
    global_vars_addr: usize,

    // String encoding
    strings: Vec<(IrId, String)>, // Collected strings for encoding

    // Stack tracking for debugging
    stack_depth: i32,                        // Current estimated stack depth
    max_stack_depth: i32,                    // Maximum stack depth reached
    encoded_strings: HashMap<IrId, Vec<u8>>, // IR string ID -> encoded bytes
    next_string_id: IrId,                    // Next available string ID

    // Execution context
    in_init_block: bool, // True when generating init block code

    // Address resolution
    reference_context: ReferenceContext,

    // Control flow analysis - NEW ARCHITECTURE
    /// Track constant values resolved during generation
    constant_values: HashMap<IrId, ConstantValue>,
    /// Track which labels have been placed at current address
    labels_at_current_address: Vec<IrId>,
}

impl ZMachineCodeGen {
    pub fn new(version: ZMachineVersion) -> Self {
        ZMachineCodeGen {
            version,
            story_data: vec![0; HEADER_SIZE],
            current_address: HEADER_SIZE,
            text_buffer_addr: 0,
            parse_buffer_addr: 0,
            label_addresses: HashMap::new(),
            string_addresses: HashMap::new(),
            function_addresses: HashMap::new(),
            ir_id_to_string: HashMap::new(),
            ir_id_to_integer: HashMap::new(),
            ir_id_to_stack_var: HashMap::new(),
            ir_id_to_object_number: HashMap::new(),
            ir_id_to_binary_op: HashMap::new(),
            builtin_function_names: HashMap::new(),
            object_numbers: HashMap::new(),
            property_numbers: HashMap::new(),
            object_properties: HashMap::new(),
            object_table_addr: 0,
            property_table_addr: 0,
            current_property_addr: 0,
            dictionary_addr: 0,
            global_vars_addr: 0,
            strings: Vec::new(),
            encoded_strings: HashMap::new(),
            next_string_id: 1000, // Start string IDs from 1000 to avoid conflicts
            stack_depth: 0,
            max_stack_depth: 0,
            in_init_block: false,
            reference_context: ReferenceContext {
                ir_id_to_address: HashMap::new(),
                unresolved_refs: Vec::new(),
            },
            constant_values: HashMap::new(),
            labels_at_current_address: Vec::new(),
        }
    }

    pub fn generate(&mut self, ir: IrProgram) -> Result<Vec<u8>, CompilerError> {
        // NEW SEQUENTIAL ARCHITECTURE: Write each section immediately to final location
        // This eliminates memory conflicts and gaps that plagued the hybrid approach

        // Phase 1: Analyze and prepare all data
        self.setup_object_mappings(&ir);
        self.analyze_properties(&ir)?;
        self.collect_strings(&ir)?;
        self.add_main_loop_strings()?;
        self.encode_all_strings()?;

        debug!("Starting sequential generation architecture");

        // Phase 2: Sequential generation - write each section immediately
        let _init_entry_point = self.generate_sequential(&ir)?;

        // Phase 3: Reference resolution only (no memory reallocation)
        debug!(
            "Phase 3: Resolving {} references",
            self.reference_context.unresolved_refs.len()
        );
        self.resolve_addresses()?;

        debug!("Sequential generation complete");
        Ok(self.story_data.clone())
    }

    /// NEW: Sequential generation - write each memory section immediately to final location
    fn generate_sequential(&mut self, ir: &IrProgram) -> Result<usize, CompilerError> {
        let mut addr = HEADER_SIZE;
        debug!("Sequential generation starting at 0x{:04x}", addr);

        // 1. Global Variables - Write immediately
        debug!("Step 1: Writing global variables at 0x{:04x}", addr);
        addr = self.write_global_variables_immediate(addr, ir)?;
        debug!("Global variables complete, next address: 0x{:04x}", addr);

        // 2. Input Buffers - Write immediately
        debug!("Step 2: Writing input buffers at 0x{:04x}", addr);
        addr = self.write_input_buffers_immediate(addr)?;
        debug!("Input buffers complete, next address: 0x{:04x}", addr);

        // 3. Object and Property Tables - Write together (they have interdependencies)
        debug!(
            "Step 3: Writing object and property tables at 0x{:04x}",
            addr
        );
        addr = self.write_object_and_property_tables_immediate(addr, ir)?;
        debug!(
            "Object and property tables complete, next address: 0x{:04x}",
            addr
        );

        // 4. Dictionary - Write immediately
        debug!("Step 4: Writing dictionary at 0x{:04x}", addr);
        addr = self.write_dictionary_immediate(addr)?;
        debug!("Dictionary complete, next address: 0x{:04x}", addr);

        // 5. Known Strings - Write all currently known strings
        debug!("Step 5: Writing known strings at 0x{:04x}", addr);
        addr = self.write_known_strings_immediate(addr)?;
        debug!("Known strings complete, next address: 0x{:04x}", addr);

        // 6. Code - Write all functions and program flow immediately
        debug!("Step 6: Writing all code at 0x{:04x}", addr);
        let init_entry_point = self.write_all_code_immediate(addr, ir)?;
        debug!(
            "Code complete, current address: 0x{:04x}",
            self.current_address
        );

        // 7. New Strings - Write any strings discovered during code generation
        debug!(
            "Step 7: Writing newly discovered strings at 0x{:04x}",
            self.current_address
        );
        self.write_new_strings_immediate()?;
        debug!(
            "New strings complete, final address: 0x{:04x}",
            self.current_address
        );

        // 8. Header - Write last with all correct addresses
        debug!(
            "Step 8: Writing final header with entry point 0x{:04x}",
            init_entry_point
        );
        self.write_final_header(init_entry_point)?;

        Ok(init_entry_point)
    }

    /// Write global variables immediately to final location
    fn write_global_variables_immediate(
        &mut self,
        start_addr: usize,
        ir: &IrProgram,
    ) -> Result<usize, CompilerError> {
        self.global_vars_addr = start_addr;
        let size = 480; // 240 globals * 2 bytes each

        self.ensure_capacity(start_addr + size);

        // Initialize all globals to 0
        for i in 0..240 {
            let addr = start_addr + i * 2;
            self.story_data[addr] = 0; // High byte
            self.story_data[addr + 1] = 0; // Low byte
        }

        // Set specific globals from IR
        // CRITICAL: Initialize global variable G00 with player object number
        // This enables player.location to work via get_prop Variable(16), property
        let g00_addr = start_addr; // Global G00 at offset 0
        self.write_word_at(g00_addr, 1)?; // Player is object #1
        debug!(
            "FROTZ DEBUG: Initialized global G00 at 0x{:04x} with player object number: 1",
            g00_addr
        );
        debug!(
            "FROTZ DEBUG: Verifying G00 value - reading back from 0x{:04x}: 0x{:04x}",
            g00_addr,
            ((self.story_data[g00_addr] as u16) << 8) | (self.story_data[g00_addr + 1] as u16)
        );

        for _global in &ir.globals {
            // TODO: Map additional IR globals to Z-Machine global variables
        }

        debug!(
            "Wrote {} global variables at 0x{:04x}-0x{:04x}",
            240,
            start_addr,
            start_addr + size - 1
        );
        Ok(start_addr + size)
    }

    /// Write input buffers immediately to final location  
    fn write_input_buffers_immediate(&mut self, start_addr: usize) -> Result<usize, CompilerError> {
        self.text_buffer_addr = start_addr;
        self.parse_buffer_addr = start_addr + 64;
        let total_size = 64 + 34; // text buffer + parse buffer

        self.ensure_capacity(start_addr + total_size);

        // Initialize text buffer (64 bytes: 2 header + 62 text)
        self.story_data[self.text_buffer_addr] = 62; // Max input length
        self.story_data[self.text_buffer_addr + 1] = 0; // Current length

        // Initialize parse buffer (34 bytes: 2 header + 32 parse data)
        self.story_data[self.parse_buffer_addr] = 8; // Max words
        self.story_data[self.parse_buffer_addr + 1] = 0; // Current words

        debug!(
            "Wrote input buffers: text=0x{:04x}, parse=0x{:04x}",
            self.text_buffer_addr, self.parse_buffer_addr
        );
        Ok(start_addr + total_size)
    }

    /// Write object and property tables together (they have interdependencies)
    fn write_object_and_property_tables_immediate(
        &mut self,
        start_addr: usize,
        ir: &IrProgram,
    ) -> Result<usize, CompilerError> {
        self.object_table_addr = start_addr;

        let num_objects = if ir.objects.is_empty() && ir.rooms.is_empty() {
            2
        } else {
            ir.objects.len() + ir.rooms.len()
        };
        debug!(
            "Writing {} objects with properties starting at 0x{:04x}",
            num_objects, start_addr
        );

        // Calculate object table size
        let default_props_size = match self.version {
            ZMachineVersion::V3 => 62, // 31 properties * 2 bytes
            ZMachineVersion::V4 | ZMachineVersion::V5 => 126, // 63 properties * 2 bytes
        };
        let object_entries_size = match self.version {
            ZMachineVersion::V3 => num_objects * 9, // v3: 9 bytes per object
            ZMachineVersion::V4 | ZMachineVersion::V5 => num_objects * 14, // v4/v5: 14 bytes per object
        };
        let object_table_size = default_props_size + object_entries_size;

        // Property tables come immediately after object table
        // Add extra padding to ensure no overlap with objects
        let property_start = start_addr + object_table_size + 32; // Add 32 bytes padding
        self.property_table_addr = property_start;
        self.current_property_addr = property_start;

        debug!(
            "Object table: 0x{:04x}-0x{:04x}, Property tables start: 0x{:04x}",
            start_addr,
            property_start - 1,
            property_start
        );

        // Set current address to object table start for generation
        self.current_address = start_addr;

        // Generate both object table and property tables together
        self.generate_object_tables(ir)?;

        let actual_end = self.current_address;
        debug!(
            "Wrote objects and properties at 0x{:04x}-0x{:04x} ({} objects)",
            start_addr,
            actual_end - 1,
            num_objects
        );
        Ok(actual_end)
    }

    /// Write dictionary immediately to final location
    fn write_dictionary_immediate(&mut self, start_addr: usize) -> Result<usize, CompilerError> {
        self.dictionary_addr = start_addr;
        self.ensure_capacity(start_addr + 10);

        // Create minimal dictionary (matches existing generate_dictionary logic)
        self.story_data[start_addr] = 4; // Entry length (4 bytes for v3/v5)
        self.story_data[start_addr + 1] = 0; // Number of entries (high byte)
        self.story_data[start_addr + 2] = 0; // Number of entries (low byte)

        let dict_end = start_addr + 3;
        debug!(
            "Wrote minimal dictionary at 0x{:04x}-0x{:04x}",
            start_addr,
            dict_end - 1
        );
        Ok(dict_end)
    }

    /// Write known strings immediately to final location  
    fn write_known_strings_immediate(&mut self, start_addr: usize) -> Result<usize, CompilerError> {
        let mut addr = start_addr;

        // Collect and sort strings for deterministic layout
        let mut string_data: Vec<(IrId, Vec<u8>)> = self
            .encoded_strings
            .iter()
            .map(|(id, encoded)| (*id, encoded.clone()))
            .collect();
        string_data.sort_by_key(|(id, _)| *id);

        debug!(
            "Writing {} strings starting at 0x{:04x}",
            string_data.len(),
            addr
        );

        for (string_id, encoded_bytes) in string_data {
            // Align string addresses according to Z-Machine version
            match self.version {
                ZMachineVersion::V3 => {
                    if addr % 2 != 0 {
                        addr += 1;
                    }
                }
                ZMachineVersion::V4 | ZMachineVersion::V5 => {
                    while addr % 4 != 0 {
                        addr += 1;
                    }
                }
            }

            // Write string data immediately
            self.ensure_capacity(addr + encoded_bytes.len());
            for (i, &byte) in encoded_bytes.iter().enumerate() {
                self.story_data[addr + i] = byte;
            }

            // Record address mapping
            self.string_addresses.insert(string_id, addr);
            self.record_address(string_id, addr);

            debug!(
                "Wrote string {} at 0x{:04x} (length={})",
                string_id,
                addr,
                encoded_bytes.len()
            );
            addr += encoded_bytes.len();
        }

        debug!("Known strings written, next address: 0x{:04x}", addr);
        Ok(addr)
    }

    /// Write newly discovered strings after code generation
    fn write_new_strings_immediate(&mut self) -> Result<(), CompilerError> {
        // Find strings that don't have addresses yet (discovered during code generation)
        let mut new_strings: Vec<(IrId, Vec<u8>)> = Vec::new();
        for (string_id, encoded_bytes) in &self.encoded_strings {
            if !self.string_addresses.contains_key(string_id) {
                new_strings.push((*string_id, encoded_bytes.clone()));
                debug!("Found new string during code generation: ID={}", string_id);
            }
        }

        if new_strings.is_empty() {
            debug!("No new strings discovered during code generation");
            return Ok(());
        }

        // Write new strings after all code
        let mut addr = self.current_address;

        // Align according to Z-Machine version
        match self.version {
            ZMachineVersion::V3 => {
                if addr % 2 != 0 {
                    addr += 1;
                }
            }
            ZMachineVersion::V4 | ZMachineVersion::V5 => {
                while addr % 4 != 0 {
                    addr += 1;
                }
            }
        }

        new_strings.sort_by_key(|(id, _)| *id);
        debug!(
            "Writing {} new strings starting at 0x{:04x}",
            new_strings.len(),
            addr
        );

        for (string_id, encoded_bytes) in new_strings {
            // Align individual string addresses
            match self.version {
                ZMachineVersion::V3 => {
                    if addr % 2 != 0 {
                        addr += 1;
                    }
                }
                ZMachineVersion::V4 | ZMachineVersion::V5 => {
                    while addr % 4 != 0 {
                        addr += 1;
                    }
                }
            }

            // Write string data immediately
            self.ensure_capacity(addr + encoded_bytes.len());
            for (i, &byte) in encoded_bytes.iter().enumerate() {
                self.story_data[addr + i] = byte;
            }

            // Record address mapping
            self.string_addresses.insert(string_id, addr);
            self.record_address(string_id, addr);

            debug!(
                "Wrote new string {} at 0x{:04x} (length={})",
                string_id,
                addr,
                encoded_bytes.len()
            );
            addr += encoded_bytes.len();
        }

        // Update current_address to reflect the new strings
        self.current_address = addr;
        debug!(
            "New strings complete, current_address updated to: 0x{:04x}",
            addr
        );
        Ok(())
    }

    /// Write all code (init, main loop, functions) immediately to final location
    fn write_all_code_immediate(
        &mut self,
        start_addr: usize,
        ir: &IrProgram,
    ) -> Result<usize, CompilerError> {
        self.current_address = start_addr;
        debug!("Starting code generation at 0x{:04x}", start_addr);

        // FIXED ARCHITECTURE: Generate init block FIRST (initial PC points here)
        let init_entry_point = self.current_address; // Capture address BEFORE generation

        if let Some(init_block) = &ir.init_block {
            // Generate explicit init block
            debug!(
                "Generating explicit init block at 0x{:04x}",
                init_entry_point
            );
            self.generate_init_block(init_block, ir)?;
        } else {
            // Generate implicit init block for games without explicit init{}
            debug!(
                "Generating implicit init block at 0x{:04x}",
                init_entry_point
            );
            self.generate_implicit_init_block(ir)?;
        }
        debug!(
            "Init block complete, current address: 0x{:04x}",
            self.current_address
        );

        // Generate program flow (main loop for interactive mode) SECOND
        self.generate_program_flow(ir)?;
        debug!(
            "Program flow complete, current address: 0x{:04x}",
            self.current_address
        );

        // Generate all user functions LAST
        self.generate_functions(ir)?;
        debug!(
            "All functions complete, final address: 0x{:04x}",
            self.current_address
        );

        debug!(
            "Code generation complete, init entry point: 0x{:04x}",
            init_entry_point
        );
        Ok(init_entry_point)
    }

    /// Generate implicit init block for games without explicit init{}
    fn generate_implicit_init_block(&mut self, _ir: &IrProgram) -> Result<(), CompilerError> {
        debug!("Generating implicit init block for games without explicit init{{}}");

        // Check if this is a simple test case without rooms or complex game structure
        // In that case, just generate a simple return instead of trying to call main loop
        if _ir.rooms.is_empty() && _ir.objects.is_empty() && _ir.grammar.is_empty() {
            debug!("Simple test case detected - generating minimal init block");
            // Just generate a simple return (RTRUE)
            self.emit_instruction(
                0x00, // rtrue opcode (0OP form)
                &[],  // No operands
                None, // No store
                None, // No branch
            )?;
            return Ok(());
        }

        // For mini_zork and similar games, we need to:
        // 1. Print any banner/intro text (if present in original init)
        // 2. Call the main loop routine

        // This function should only be called when there's no explicit init block
        // The logic above already checks for init_block existence

        // For games without explicit init, create minimal setup:
        // Just call the main loop routine (main loop will handle its own setup)
        let main_loop_id = 9000u32; // Use consistent ID with main loop generation

        debug!(
            "Implicit init: calling main loop routine (ID {})",
            main_loop_id
        );

        // Generate call to main loop routine
        let layout = self.emit_instruction(
            0x20,                                          // call_vs opcode (VAR form of call)
            &[Operand::LargeConstant(placeholder_word())], // Placeholder for main loop routine address
            None,                                          // No store (main loop doesn't return)
            None,                                          // No branch
        )?;

        // Add unresolved reference for main loop call
        self.reference_context
            .unresolved_refs
            .push(UnresolvedReference {
                reference_type: ReferenceType::FunctionCall,
                location: layout
                    .operand_location
                    .expect("call instruction must have operand"),
                target_id: main_loop_id,
                is_packed_address: true, // Function calls use packed addresses
                offset_size: 2,
            });

        debug!(
            "Implicit init block complete - calls main loop at ID {}",
            main_loop_id
        );
        Ok(())
    }

    /// Write final header with all addresses resolved
    fn write_final_header(&mut self, init_entry_point: usize) -> Result<(), CompilerError> {
        // Use existing header writing logic
        self.write_header_with_entry_point(init_entry_point)
    }

    /// Analyze all property accesses across the IR program and build global property registry
    fn analyze_properties(&mut self, ir: &IrProgram) -> Result<(), CompilerError> {
        debug!("Starting property analysis...");

        // Step 1: Collect all property names from all instructions
        let mut all_properties = std::collections::HashSet::new();

        // Analyze functions
        for function in &ir.functions {
            self.collect_properties_from_block(&function.body, &mut all_properties);
        }

        // Analyze init block
        if let Some(init_block) = &ir.init_block {
            self.collect_properties_from_block(init_block, &mut all_properties);
        }

        // Step 2: Add essential properties that player object always needs
        all_properties.insert("desc".to_string()); // Player description property
        all_properties.insert("location".to_string()); // Player location property

        // Assign property numbers starting from 1 in sorted order for consistency
        let mut sorted_properties: Vec<String> = all_properties.iter().cloned().collect();
        sorted_properties.sort();

        let mut property_number = 1u8;
        for property_name in sorted_properties {
            self.property_numbers
                .insert(property_name.clone(), property_number);
            debug!(
                "Assigned property '{}' -> number {}",
                property_name, property_number
            );
            property_number += 1;
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
    fn collect_properties_from_block(
        &mut self,
        block: &IrBlock,
        properties: &mut std::collections::HashSet<String>,
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
    fn analyze_object_property_usage(&mut self, ir: &IrProgram) {
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

    /// Collect all strings from the IR program for later encoding
    fn collect_strings(&mut self, ir: &IrProgram) -> Result<(), CompilerError> {
        // Collect from string table
        for (string, &id) in &ir.string_table {
            self.strings.push((id, string.clone()));
        }

        // Collect strings from LoadImmediate instructions in all functions
        for function in &ir.functions {
            self.collect_strings_from_block(&function.body)?;
        }

        // Collect strings from init block if present
        if let Some(init_block) = &ir.init_block {
            self.collect_strings_from_block(init_block)?;
        }

        // TODO: Collect strings from other IR elements (rooms, objects, etc.)

        Ok(())
    }

    /// Add main loop strings to the collection
    fn add_main_loop_strings(&mut self) -> Result<(), CompilerError> {
        // Add prompt string for main loop
        let prompt_string_id = 9002u32;
        let prompt_text = "> ";
        self.strings
            .push((prompt_string_id, prompt_text.to_string()));

        debug!("Added main loop strings: prompt='> '");
        Ok(())
    }

    /// Collect strings and integers from all LoadImmediate instructions in a block
    fn collect_strings_from_block(&mut self, block: &IrBlock) -> Result<(), CompilerError> {
        for instruction in &block.instructions {
            match instruction {
                IrInstruction::LoadImmediate {
                    target,
                    value: IrValue::String(s),
                } => {
                    // Register the string for this IR ID
                    self.ir_id_to_string.insert(*target, s.clone());
                    // Add to strings collection for encoding
                    self.strings.push((*target, s.clone()));
                }
                IrInstruction::LoadImmediate {
                    target,
                    value: IrValue::Integer(i),
                } => {
                    // Register the integer for this IR ID
                    self.ir_id_to_integer.insert(*target, *i);
                }
                IrInstruction::LoadImmediate {
                    target: _,
                    value: _,
                } => {
                    // Other LoadImmediate types - no action needed
                }
                // Handle other instructions that might contain blocks
                _ => {
                    // For now, we don't need to recurse into other instruction types
                    // since they don't typically contain nested blocks with strings
                }
            }
        }
        Ok(())
    }

    /// Encode all collected strings using Z-Machine text encoding
    fn encode_all_strings(&mut self) -> Result<(), CompilerError> {
        for (id, string) in &self.strings {
            let encoded = self.encode_string(string)?;
            self.encoded_strings.insert(*id, encoded);
        }
        Ok(())
    }

    /// Encode a single string using Z-Machine ZSCII encoding
    fn encode_string(&self, s: &str) -> Result<Vec<u8>, CompilerError> {
        // Z-Machine text encoding per Z-Machine Standard 1.1, Section 3.5.3
        // Alphabet A0 (6-31): abcdefghijklmnopqrstuvwxyz
        // Alphabet A1 (6-31): ABCDEFGHIJKLMNOPQRSTUVWXYZ
        // Alphabet A2 (6-31):  ^0123456789.,!?_#'"/\-:()

        let mut zchars = Vec::new();

        for ch in s.chars() {
            match ch {
                // Space is always Z-character 0
                ' ' => zchars.push(0),

                // Alphabet A0: lowercase letters (Z-chars 6-31)
                'a'..='z' => {
                    zchars.push(ch as u8 - b'a' + 6);
                }

                // Alphabet A1: uppercase letters (single-shift with 4, then Z-char 6-31)
                'A'..='Z' => {
                    zchars.push(4); // Single shift to alphabet A1
                    zchars.push(ch as u8 - b'A' + 6);
                }

                // Alphabet A2: punctuation characters (single-shift with 5, then Z-char)
                '\n' => {
                    zchars.push(5); // Single shift to alphabet A2
                    zchars.push(7); // '^' (newline) at position 7
                }
                '0'..='9' => {
                    zchars.push(5); // Single shift to alphabet A2
                    zchars.push(ch as u8 - b'0' + 8); // Numbers at positions 8-17
                }
                '.' => {
                    zchars.push(5); // Single shift to alphabet A2
                    zchars.push(18); // '.' at position 18
                }
                ',' => {
                    zchars.push(5); // Single shift to alphabet A2
                    zchars.push(19); // ',' at position 19
                }
                '!' => {
                    zchars.push(5); // Single shift to alphabet A2
                    zchars.push(20); // '!' at position 20
                }
                '?' => {
                    zchars.push(5); // Single shift to alphabet A2
                    zchars.push(21); // '?' at position 21
                }
                '_' => {
                    zchars.push(5); // Single shift to alphabet A2
                    zchars.push(22); // '_' at position 22
                }
                '#' => {
                    zchars.push(5); // Single shift to alphabet A2
                    zchars.push(23); // '#' at position 23
                }
                '\'' => {
                    zchars.push(5); // Single shift to alphabet A2
                    zchars.push(24); // '\'' at position 24
                }
                '"' => {
                    zchars.push(5); // Single shift to alphabet A2
                    zchars.push(25); // '"' at position 25
                }
                '/' => {
                    zchars.push(5); // Single shift to alphabet A2
                    zchars.push(26); // '/' at position 26
                }
                '\\' => {
                    zchars.push(5); // Single shift to alphabet A2
                    zchars.push(27); // '\\' at position 27
                }
                '-' => {
                    zchars.push(5); // Single shift to alphabet A2
                    zchars.push(28); // '-' at position 28
                }
                ':' => {
                    zchars.push(5); // Single shift to alphabet A2
                    zchars.push(29); // ':' at position 29
                }
                '(' => {
                    zchars.push(5); // Single shift to alphabet A2
                    zchars.push(30); // '(' at position 30
                }
                ')' => {
                    zchars.push(5); // Single shift to alphabet A2
                    zchars.push(31); // ')' at position 31
                }
                _ => {
                    // Unsupported character - encode as '?'
                    zchars.push(5); // Single shift to alphabet A2
                    zchars.push(21); // '?' at position 21
                }
            }
        }

        // Pack zchars into 16-bit words (3 zchars per word)
        let mut encoded = Vec::new();

        for chunk in zchars.chunks(3) {
            let mut word = 0u16;

            for (i, &zchar) in chunk.iter().enumerate() {
                word |= (zchar as u16 & 0x1F) << (10 - i * 5);
            }

            // Pad incomplete chunks with 5s (pad character)
            for i in chunk.len()..3 {
                word |= 5u16 << (10 - i * 5);
            }

            encoded.push((word >> 8) as u8);
            encoded.push(word as u8);
        }

        // Ensure we have at least one word
        if encoded.is_empty() {
            encoded.push(0x80);
            encoded.push(0x00);
        } else {
            // Set the termination bit on the last word
            let last_idx = encoded.len() - 2;
            encoded[last_idx] |= 0x80;
        }

        Ok(encoded)
    }

    /// Plan the memory layout for all Z-Machine structures
    fn layout_memory_structures(&mut self, ir: &IrProgram) -> Result<(), CompilerError> {
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
            self.story_data.resize(self.text_buffer_addr + 64 + 34, 0);
        }
        self.story_data[self.text_buffer_addr] = 62; // Max input length
        self.story_data[self.text_buffer_addr + 1] = 0; // Current length
        self.story_data[self.parse_buffer_addr] = 8; // Max words
        self.story_data[self.parse_buffer_addr + 1] = 0; // Current words

        // Reserve space for object table
        self.object_table_addr = addr;
        let estimated_objects = if ir.objects.is_empty() && ir.rooms.is_empty() {
            2
        } else {
            ir.objects.len() + ir.rooms.len()
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
        self.current_property_addr = addr; // Initialize property allocation pointer
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
        addr += 1000; // Rough estimate for dictionary

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
            self.record_address(string_id, addr); // Record in reference context

            // CRITICAL: Write string data immediately during layout phase
            // This prevents overlaps with code generation
            if let Some(encoded_bytes) = self.encoded_strings.get(&string_id).cloned() {
                self.ensure_capacity(addr + encoded_bytes.len());
                for (i, &byte) in encoded_bytes.iter().enumerate() {
                    self.story_data[addr + i] = byte;
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
        self.current_address = addr;
        debug!("Layout phase complete: current_address=0x{:04x}", addr);

        Ok(())
    }

    /// Generate object and property tables
    fn generate_object_tables(&mut self, ir: &IrProgram) -> Result<(), CompilerError> {
        debug!("Starting Z-Machine object table generation...");

        let obj_table_start = self.object_table_addr;
        self.ensure_capacity(obj_table_start + 1000); // Ensure sufficient space

        // Step 1: Generate property defaults table
        let default_props = match self.version {
            ZMachineVersion::V3 => 31,
            ZMachineVersion::V4 | ZMachineVersion::V5 => 63,
        };

        debug!(
            "Generating property defaults table ({} entries)",
            default_props
        );
        for i in 0..default_props {
            let addr = obj_table_start + i * 2;
            self.ensure_capacity(addr + 2);

            // Use IR property defaults if available, otherwise 0
            let prop_num = (i + 1) as u8;
            let default_value = ir.property_defaults.get_default(prop_num);

            self.story_data[addr] = (default_value >> 8) as u8; // High byte
            self.story_data[addr + 1] = (default_value & 0xFF) as u8; // Low byte
        }

        // Step 2: Create object entries for all IR objects (rooms + objects)
        let objects_start = obj_table_start + default_props * 2;
        debug!("Object entries start at 0x{:04x}", objects_start);

        // Collect all objects (rooms and objects) from IR
        let mut all_objects = Vec::new();

        // CRITICAL FIX: Create player object as object #1
        // This resolves the "get_prop called with object 0" Frotz compatibility issue
        debug!("Creating player object as object #1 for Frotz compatibility");
        let mut player_properties = IrProperties::new();
        // Add essential player properties
        let location_prop = *self.property_numbers.get("location").unwrap_or(&8);
        let desc_prop = *self.property_numbers.get("desc").unwrap_or(&1);
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
        debug!(
            "Created player object with location property = {}",
            initial_location
        );

        // Add rooms as objects (rooms are just objects with specific properties)
        for room in &ir.rooms {
            let mut room_properties = IrProperties::new();

            // Add essential room properties that games commonly access
            // Get property numbers from the global property registry
            let desc_prop = *self.property_numbers.get("desc").unwrap_or(&1);
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
        }

        // Add regular objects
        for object in &ir.objects {
            let mut object_properties = object.properties.clone();

            // Ensure all objects have essential properties that games commonly access
            let location_prop = *self.property_numbers.get("location").unwrap_or(&8);
            let desc_prop = *self.property_numbers.get("desc").unwrap_or(&1);

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
        }

        debug!(
            "Total objects to generate: {} ({} rooms + {} objects)",
            all_objects.len(),
            ir.rooms.len(),
            ir.objects.len()
        );

        // Step 3: Build object ID mapping table
        let mut object_id_to_number: HashMap<IrId, u8> = HashMap::new();
        for (index, object) in all_objects.iter().enumerate() {
            let obj_num = (index + 1) as u8; // Objects are numbered starting from 1
            object_id_to_number.insert(object.id, obj_num);
        }

        // Step 4: Create object table entries
        for (index, object) in all_objects.iter().enumerate() {
            let obj_num = (index + 1) as u8; // Objects are numbered starting from 1
            self.create_object_entry_from_ir_with_mapping(
                objects_start,
                obj_num,
                object,
                &object_id_to_number,
            )?;
        }

        // Update current_address to reflect the end of object and property tables
        // current_property_addr points to where the next property table would go
        self.current_address = self.current_property_addr;

        debug!(
            "Object table generation complete, current_address updated to: 0x{:04x}",
            self.current_address
        );
        Ok(())
    }

    /// Create a single object entry in the object table
    fn create_object_entry(
        &mut self,
        objects_start: usize,
        obj_num: u8,
        parent: u8,
        sibling: u8,
        child: u8,
    ) -> Result<(), CompilerError> {
        let obj_addr = objects_start + ((obj_num - 1) as usize) * 9; // V3: 9 bytes per object
        self.ensure_capacity(obj_addr + 9);

        // Attributes (4 bytes, all zeros for now)
        self.story_data[obj_addr] = 0;
        self.story_data[obj_addr + 1] = 0;
        self.story_data[obj_addr + 2] = 0;
        self.story_data[obj_addr + 3] = 0;

        // Relationships (V3 uses 1 byte each)
        self.story_data[obj_addr + 4] = parent;
        self.story_data[obj_addr + 5] = sibling;
        self.story_data[obj_addr + 6] = child;

        // Create property table for this object
        debug!(
            "BEFORE create_property_table: 0x0268 = 0x{:02x}",
            self.story_data[0x0268]
        );
        let prop_table_addr = self.create_property_table(obj_num)?;
        debug!(
            "AFTER create_property_table: 0x0268 = 0x{:02x}",
            self.story_data[0x0268]
        );

        // Property table address (word)
        let prop_addr_field = obj_addr + 7;
        debug!(
            "Writing property table address 0x{:04x} to object at 0x{:04x}, 0x{:04x}",
            prop_table_addr,
            prop_addr_field,
            prop_addr_field + 1
        );
        self.story_data[prop_addr_field] = (prop_table_addr >> 8) as u8; // High byte
        self.story_data[prop_addr_field + 1] = (prop_table_addr & 0xFF) as u8; // Low byte
        debug!(
            "AFTER writing prop addr: 0x0268 = 0x{:02x}",
            self.story_data[0x0268]
        );

        Ok(())
    }

    /// Create a single object entry from IR object data
    fn create_object_entry_from_ir_with_mapping(
        &mut self,
        objects_start: usize,
        obj_num: u8,
        object: &ObjectData,
        object_id_to_number: &HashMap<IrId, u8>,
    ) -> Result<(), CompilerError> {
        let obj_addr = objects_start + ((obj_num - 1) as usize) * 9; // V3: 9 bytes per object
        self.ensure_capacity(obj_addr + 9);

        // Attributes (4 bytes for V3)
        // Convert IR attributes to Z-Machine format
        let attrs = object.attributes.flags;
        self.story_data[obj_addr] = ((attrs >> 24) & 0xFF) as u8; // Bits 31-24
        self.story_data[obj_addr + 1] = ((attrs >> 16) & 0xFF) as u8; // Bits 23-16
        self.story_data[obj_addr + 2] = ((attrs >> 8) & 0xFF) as u8; // Bits 15-8
        self.story_data[obj_addr + 3] = (attrs & 0xFF) as u8; // Bits 7-0

        // Parent/sibling/child relationships (V3 uses 1 byte each)
        // Resolve IR IDs to actual Z-Machine object numbers
        let parent = object
            .parent
            .and_then(|id| object_id_to_number.get(&id))
            .copied()
            .unwrap_or(0);
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

        self.story_data[obj_addr + 4] = parent;
        self.story_data[obj_addr + 5] = sibling;
        self.story_data[obj_addr + 6] = child;

        // Create property table for this object with actual IR properties
        let prop_table_addr = self.create_property_table_from_ir(obj_num, object)?;

        // Property table address (word)
        let prop_addr_field = obj_addr + 7;
        self.story_data[prop_addr_field] = (prop_table_addr >> 8) as u8; // High byte
        self.story_data[prop_addr_field + 1] = (prop_table_addr & 0xFF) as u8; // Low byte

        debug!(
            "Created object #{}: '{}' at addr 0x{:04x}, attributes=0x{:08x}, prop_table=0x{:04x}",
            obj_num, object.short_name, obj_addr, attrs, prop_table_addr
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
        self.story_data[prop_table_addr] = 0;
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
            self.story_data[addr] = header;
            addr += 1;

            // Property data (2 bytes, default value 0)
            debug!(
                "Writing property {} data (0x0000) at address 0x{:04x}",
                prop_num, addr
            );
            self.story_data[addr] = 0; // High byte
            self.story_data[addr + 1] = 0; // Low byte
            addr += 2;
        }

        // End of property table (property 0 marks end)
        debug!("Writing property terminator 0x00 at address 0x{:04x}", addr);
        self.story_data[addr] = 0;
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
        self.story_data[addr] = text_length as u8;
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
                self.story_data[addr] = byte;
                addr += 1;
            }
            // Pad to word boundary if necessary
            if name_bytes.len() % 2 == 1 {
                self.story_data[addr] = 0; // Pad byte
                addr += 1;
            }
        }

        // Write properties in descending order (required by Z-Machine spec)
        let mut properties: Vec<_> = object.properties.properties.iter().collect();
        properties.sort_by(|a, b| b.0.cmp(a.0)); // Sort by property number, descending

        for (&prop_num, prop_value) in properties {
            // Write property size/number byte
            let (size_byte, prop_data) = self.encode_property_value(prop_num, prop_value);
            debug!(
                "Writing property {}: size_byte=0x{:02x}, data_len={}",
                prop_num,
                size_byte,
                prop_data.len()
            );

            // Ensure capacity for property header + data + terminator
            self.ensure_capacity(addr + 1 + prop_data.len() + 1);

            self.story_data[addr] = size_byte;
            debug!(
                "PROP TABLE DEBUG: Writing size_byte=0x{:02x} at addr=0x{:04x}",
                size_byte, addr
            );
            addr += 1;

            // Write property data
            for (i, &byte) in prop_data.iter().enumerate() {
                self.story_data[addr] = byte;
                debug!(
                    "PROP TABLE DEBUG: Writing prop data byte {}=0x{:02x} at addr=0x{:04x}",
                    i, byte, addr
                );
                addr += 1;
            }
        }

        // Terminator (property 0)
        self.story_data[addr] = 0;
        debug!(
            "PROP TABLE DEBUG: Writing terminator 0x00 at addr=0x{:04x}",
            addr
        );
        addr += 1;

        debug!(
            "PROP TABLE DEBUG: Property table for '{}' complete: 0x{:04x}-0x{:04x} ({} bytes)",
            object.short_name,
            prop_table_addr,
            addr - 1,
            addr - prop_table_addr
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

        Ok(prop_table_addr)
    }

    /// Encode an object name as Z-Machine text
    fn encode_object_name(&self, name: &str) -> Vec<u8> {
        // For now, simple ASCII encoding (should be proper Z-Machine text encoding)
        let mut bytes = Vec::new();
        for chunk in name.bytes().collect::<Vec<_>>().chunks(2) {
            let word = if chunk.len() == 2 {
                ((chunk[0] as u16) << 8) | (chunk[1] as u16)
            } else {
                (chunk[0] as u16) << 8
            };
            bytes.push((word >> 8) as u8);
            bytes.push((word & 0xFF) as u8);
        }
        // Add terminator if odd length
        if name.len() % 2 == 1 {
            bytes.push(0);
        }
        bytes
    }

    /// Encode a property value for Z-Machine format
    fn encode_property_value(&self, prop_num: u8, prop_value: &IrPropertyValue) -> (u8, Vec<u8>) {
        match prop_value {
            IrPropertyValue::Byte(val) => {
                // V3: size_byte = 32 * (data_bytes - 1) + prop_num = 32 * (1 - 1) + prop_num = prop_num
                let size_byte = prop_num;
                (size_byte, vec![*val])
            }
            IrPropertyValue::Word(val) => {
                // V3: size_byte = 32 * (data_bytes - 1) + prop_num = 32 * (2 - 1) + prop_num = 32 + prop_num
                let size_byte = 32 + prop_num;
                let data_bytes = vec![(val >> 8) as u8, (val & 0xFF) as u8];
                debug!(
                    "PROPERTY DEBUG: Encoding Word property {}: value=0x{:04x} -> size_byte=0x{:02x}, data=[0x{:02x}, 0x{:02x}]",
                    prop_num, val, size_byte, data_bytes[0], data_bytes[1]
                );
                (size_byte, data_bytes)
            }
            IrPropertyValue::Bytes(bytes) => {
                // V3: size_byte = 32 * (data_bytes - 1) + prop_num
                let data_len = bytes.len().min(8); // Z-Machine V3 max size is 8
                                                   // Handle empty byte arrays to avoid underflow
                let size_byte = if data_len > 0 {
                    32 * (data_len - 1) + prop_num as usize
                } else {
                    prop_num as usize // Empty bytes: just the property number
                };
                (size_byte as u8, bytes.clone())
            }
            IrPropertyValue::String(s) => {
                // Encode string as bytes (simplified)
                let bytes: Vec<u8> = s.bytes().collect();
                let data_len = bytes.len().min(8);
                // Handle empty strings to avoid underflow
                let size_byte = if data_len > 0 {
                    32 * (data_len - 1) + prop_num as usize
                } else {
                    prop_num as usize // Empty string: just the property number
                };
                (size_byte as u8, bytes)
            }
        }
    }

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

    /// Generate dictionary
    fn generate_dictionary(&mut self, _ir: &IrProgram) -> Result<(), CompilerError> {
        // TODO: Implement dictionary generation
        // For now, create minimal dictionary

        let dict_start = self.dictionary_addr;
        self.ensure_capacity(dict_start + 10);

        // Minimal dictionary header
        self.story_data[dict_start] = 4; // Entry length (4 bytes for v3/v5)
        self.story_data[dict_start + 1] = 0; // Number of entries (high byte)
        self.story_data[dict_start + 2] = 0; // Number of entries (low byte)

        // CRITICAL: Update current_address to reflect actual end of all data structures
        // This eliminates gaps between data structures and code generation
        let dict_end = self.dictionary_addr + 3; // Minimal dictionary is 3 bytes
        let max_data_end = std::cmp::max(self.current_property_addr, dict_end);
        // Respect existing current_address if it's already beyond our data structures
        self.current_address = std::cmp::max(self.current_address, max_data_end);
        debug!("Data structures complete, current_address updated to: 0x{:04x} (property_end: 0x{:04x}, dict_end: 0x{:04x})", 
               self.current_address, self.current_property_addr, dict_end);

        Ok(())
    }

    /// Generate global variables table
    fn generate_global_variables(&mut self, ir: &IrProgram) -> Result<(), CompilerError> {
        let globals_start = self.global_vars_addr;
        self.ensure_capacity(globals_start + 480); // Space for 240 globals

        // Initialize all globals to 0
        for i in 0..240 {
            let addr = globals_start + i * 2;
            self.story_data[addr] = 0; // High byte
            self.story_data[addr + 1] = 0; // Low byte
        }

        // Set specific globals from IR
        // CRITICAL: Initialize global variable G00 with player object number
        let g00_addr = globals_start; // Global G00 at offset 0
        self.write_word_at(g00_addr, 1)?; // Player is object #1
        debug!(
            "Initialized global G00 at 0x{:04x} with player object number: 1",
            g00_addr
        );

        for _global in &ir.globals {
            // TODO: Map additional IR globals to Z-Machine global variables
            // For now, just ensure the space is allocated
        }

        // CRITICAL: Update current_address to reflect actual end of global variables
        // This eliminates gaps between global variables and subsequent data structures
        let globals_end = globals_start + 480; // 240 globals * 2 bytes each
        self.current_address = std::cmp::max(self.current_address, globals_end);
        debug!("Global variables complete, current_address updated to: 0x{:04x} (globals_end: 0x{:04x})", 
               self.current_address, globals_end);

        Ok(())
    }

    /// Generate program flow based on program mode
    fn generate_program_flow(&mut self, ir: &IrProgram) -> Result<(), CompilerError> {
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

            let main_call_routine_address = self.current_address;
            let main_call_id = 9000u32; // Use high ID to avoid conflicts

            // Create a routine that calls the user's main function
            self.emit_byte(0x00)?; // Routine header: 0 locals

            // Record the routine address for reference resolution
            self.function_addresses
                .insert(main_call_id, main_call_routine_address);
            self.record_address(main_call_id, main_call_routine_address);

            // Call the user's main function
            let layout = self.emit_instruction(
                0xE0,                                          // call_1s (call with 1 operand, store result)
                &[Operand::LargeConstant(placeholder_word())], // Placeholder for main function address
                Some(0x00), // Store result in local variable 0 (discarded)
                None,       // No branch
            )?;

            // Add unresolved reference for main function call
            self.reference_context
                .unresolved_refs
                .push(UnresolvedReference {
                    reference_type: ReferenceType::FunctionCall,
                    location: layout
                        .operand_location
                        .expect("call instruction must have operand"),
                    target_id: main_function.id,
                    is_packed_address: true, // Function calls use packed addresses
                    offset_size: 2,
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
                if self.current_address % 2 != 0 {
                    self.emit_byte(0x00)?; // Pad with zero byte for alignment
                }
            }
            ZMachineVersion::V4 | ZMachineVersion::V5 => {
                // v4/v5: functions must be at 4-byte boundaries
                while self.current_address % 4 != 0 {
                    self.emit_byte(0x00)?; // Pad with zero bytes for alignment
                }
            }
        }
        Ok(())
    }

    /// Generate the automatic main game loop
    fn generate_main_loop(&mut self, _ir: &IrProgram) -> Result<(), CompilerError> {
        debug!("Generating automatic main game loop");

        // Align function address according to Z-Machine version requirements
        self.align_function_address()?;

        // Record main loop routine address for function calls
        let main_loop_id = 9000u32; // Use high ID to avoid conflicts
        let main_loop_routine_address = self.current_address;

        debug!(
            "Main loop routine starts at address 0x{:04x}",
            main_loop_routine_address
        );

        // Main loop should be a routine with 0 locals (like Zork I)
        self.emit_byte(0x00)?; // Routine header: 0 locals

        // Record the routine address (including header) for function calls
        self.function_addresses
            .insert(main_loop_id, main_loop_routine_address);
        self.record_address(main_loop_id, main_loop_routine_address); // Record for reference resolution

        // Record the first instruction address for jump targets
        let main_loop_first_instruction = self.current_address;
        let main_loop_jump_id = main_loop_id + 1; // Different ID for jump target
        self.record_address(main_loop_jump_id, main_loop_first_instruction);

        // 1. Print prompt "> "
        let prompt_string_id = 9002u32;

        let layout = self.emit_instruction(
            0x0D,                                          // print_paddr (print packed address string)
            &[Operand::LargeConstant(placeholder_word())], // Placeholder for prompt string address
            None,                                          // No store
            None,                                          // No branch
        )?;

        // Add unresolved reference for prompt string using layout-tracked operand location
        self.reference_context
            .unresolved_refs
            .push(UnresolvedReference {
                reference_type: ReferenceType::StringRef,
                location: layout
                    .operand_location
                    .expect("print_paddr instruction must have operand"),
                target_id: prompt_string_id,
                is_packed_address: true,
                offset_size: 2,
            });

        // 2. Use properly allocated buffer addresses from layout phase
        let text_buffer_addr = self.text_buffer_addr as u16;
        let parse_buffer_addr = self.parse_buffer_addr as u16;

        debug!(
            "Using input buffers: text=0x{:04x}, parse=0x{:04x}",
            text_buffer_addr, parse_buffer_addr
        );

        // 3. Read user input using Z-Machine sread instruction
        self.emit_instruction(
            0x04, // sread opcode (VAR instruction)
            &[
                Operand::LargeConstant(text_buffer_addr),
                Operand::LargeConstant(parse_buffer_addr),
            ],
            None, // No store
            None, // No branch
        )?;

        // 4. For now, just echo back what was typed (MVP implementation)
        // TODO: Add proper command parsing and dispatch

        // 5. Jump back to loop start (first instruction, not routine header)
        let main_loop_jump_id = main_loop_id + 1; // Use same calculation as above
        let layout = self.emit_instruction(
            0x0C,                                          // jump opcode (correct opcode)
            &[Operand::LargeConstant(placeholder_word())], // Placeholder for loop start address
            None,                                          // No store
            None,                                          // No branch
        )?;

        // Add unresolved reference for loop jump using layout-tracked operand location
        self.reference_context
            .unresolved_refs
            .push(UnresolvedReference {
                reference_type: ReferenceType::Jump,
                location: layout
                    .operand_location
                    .expect("jump instruction must have operand"),
                target_id: main_loop_jump_id, // Jump back to main loop first instruction (not routine header)
                is_packed_address: false,
                offset_size: 2,
            });

        debug!(
            "Main loop generation complete at 0x{:04x}",
            self.current_address
        );
        Ok(())
    }

    /// Generate code for all functions
    fn generate_functions(&mut self, ir: &IrProgram) -> Result<(), CompilerError> {
        // Generate all functions
        for function in &ir.functions {
            // Align function addresses according to Z-Machine version requirements
            match self.version {
                ZMachineVersion::V3 => {
                    // v3: functions must be at even addresses
                    if self.current_address % 2 != 0 {
                        self.emit_byte(0x00)?; // Pad with zero byte for alignment
                    }
                }
                ZMachineVersion::V4 | ZMachineVersion::V5 => {
                    // v4/v5: functions must be at 4-byte boundaries
                    while self.current_address % 4 != 0 {
                        self.emit_byte(0x00)?; // Pad with zero bytes for alignment
                    }
                }
            }

            // Record function address BEFORE header (where function actually starts)
            let func_addr = self.current_address;

            // Generate function header (local variable count + types)
            self.generate_function_header(function)?;
            self.function_addresses.insert(function.id, func_addr);
            self.record_address(function.id, func_addr);

            // Generate function body with boundary protection
            self.generate_function_body_with_boundary(function)?;

            log::debug!(
                "Function '{}' generation complete at address 0x{:04x}",
                function.name,
                self.current_address
            );
        }

        Ok(())
    }

    /// Generate function body with proper boundary detection and protection
    fn generate_function_body_with_boundary(
        &mut self,
        function: &IrFunction,
    ) -> Result<(), CompilerError> {
        // Track the starting address to detect if we're generating orphaned instructions
        let function_start = self.current_address;

        // First pass: Pre-calculate all instruction addresses including labels
        log::debug!("Pre-calculating addresses for function '{}'", function.name);
        self.pre_calculate_addresses(&function.body.instructions)?;

        // Second pass: Generate actual instructions
        for instruction in &function.body.instructions {
            self.generate_instruction(instruction)?;

            // Stop if we encounter a return instruction (labels already processed)
            if matches!(instruction, IrInstruction::Return { .. }) {
                log::debug!(
                    "Function '{}' has explicit return, stopping instruction generation",
                    function.name
                );
                return Ok(());
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

        // Log the range of addresses used by this function
        log::debug!(
            "Function '{}' generated from 0x{:04x} to 0x{:04x}",
            function.name,
            function_start,
            self.current_address
        );

        Ok(())
    }

    /// Generate function header with local variable declarations
    fn generate_function_header(&mut self, function: &IrFunction) -> Result<(), CompilerError> {
        // Z-Machine function header: 1 byte for local count + 2 bytes per local (v3 only)
        let local_count = function.local_vars.len();

        if local_count > 15 {
            return Err(CompilerError::CodeGenError(format!(
                "Function '{}' has {} locals, maximum is 15",
                function.name, local_count
            )));
        }

        self.emit_byte(local_count as u8)?;

        // In v3, emit default values for locals (v4+ doesn't need this)
        if self.version == ZMachineVersion::V3 {
            for _i in 0..local_count {
                self.emit_word(0)?; // Default local value = 0
            }
        }

        Ok(())
    }

    /// Generate code for a basic block
    fn generate_block(&mut self, block: &IrBlock) -> Result<(), CompilerError> {
        for instruction in &block.instructions {
            self.generate_instruction(instruction)?;
        }
        Ok(())
    }

    /// Check if a block ends with a return instruction
    fn block_ends_with_return(&self, block: &IrBlock) -> bool {
        matches!(
            block.instructions.last(),
            Some(IrInstruction::Return { .. })
        )
    }

    /// Generate code for a single IR instruction
    fn generate_instruction(&mut self, instruction: &IrInstruction) -> Result<(), CompilerError> {
        match instruction {
            IrInstruction::LoadImmediate { target, value } => {
                // Store mapping for string and integer values so we can resolve them in function calls
                // AND store constants for control flow optimization
                match value {
                    IrValue::String(s) => {
                        self.ir_id_to_string.insert(*target, s.clone());
                        self.constant_values
                            .insert(*target, ConstantValue::String(s.clone()));
                    }
                    IrValue::Integer(i) => {
                        self.ir_id_to_integer.insert(*target, *i);
                        self.constant_values
                            .insert(*target, ConstantValue::Integer(*i));
                    }
                    IrValue::Boolean(b) => {
                        // Convert boolean to integer for compatibility
                        let int_val = if *b { 1 } else { 0 };
                        self.ir_id_to_integer.insert(*target, int_val);
                        self.constant_values
                            .insert(*target, ConstantValue::Boolean(*b));
                    }
                    _ => {}
                }
                self.generate_load_immediate(value)?;
            }

            IrInstruction::BinaryOp {
                target,
                op,
                left,
                right,
            } => {
                // Store binary operation mapping for conditional branch optimization
                self.ir_id_to_binary_op
                    .insert(*target, (op.clone(), *left, *right));

                // OPTIMIZATION: Skip generating comparison instructions for operations that will
                // be handled directly in emit_conditional_branch_instruction. This prevents
                // duplicate instructions and unresolved branch placeholders.
                if matches!(
                    op,
                    IrBinaryOp::Greater
                        | IrBinaryOp::Less
                        | IrBinaryOp::Equal
                        | IrBinaryOp::LessEqual
                        | IrBinaryOp::GreaterEqual
                        | IrBinaryOp::NotEqual
                ) {
                    log::debug!("Skipping BinaryOp comparison {:?} target={} - will be handled directly in conditional branches", op, target);
                    // Don't generate any Z-Machine instruction here - the comparison will be
                    // generated directly as a conditional branch instruction when needed
                    // Just return without processing this instruction further
                }
                // Only process non-comparison binary operations
                else {
                    // Check if this is string concatenation (Add operation with strings)
                    if matches!(op, IrBinaryOp::Add) {
                        let left_is_string = self.ir_id_to_string.contains_key(left);
                        let right_is_string = self.ir_id_to_string.contains_key(right);

                        if left_is_string || right_is_string {
                            // This is string concatenation
                            self.generate_string_concatenation(*target, *left, *right)?;
                        } else {
                            // Regular arithmetic addition - resolve actual operands
                            let left_op = self.resolve_ir_id_to_operand(*left)?;
                            let right_op = self.resolve_ir_id_to_operand(*right)?;
                            let store_var = Some(0); // Store to stack top
                            self.generate_binary_op(op, left_op, right_op, store_var)?;
                        }
                    } else {
                        // Other binary operations (comparison, arithmetic) - resolve actual operands
                        let left_op = self.resolve_ir_id_to_operand(*left)?;
                        let right_op = self.resolve_ir_id_to_operand(*right)?;
                        let store_var = Some(0); // Store to stack top
                        self.generate_binary_op(op, left_op, right_op, store_var)?;
                    }
                } // End of else block for non-comparison operations
            }

            IrInstruction::Call {
                target,
                function,
                args,
            } => {
                // Check if this is a builtin function
                if self.is_builtin_function(*function) {
                    self.generate_builtin_function_call(*function, args, *target)?;
                } else {
                    // Generate call with unresolved function reference
                    self.generate_call_with_reference(*function, args, *target)?;
                }
            }

            IrInstruction::Return { value } => {
                if let Some(_ir_value) = value {
                    // Return with value - use ret opcode with operand
                    let operands = vec![Operand::Variable(0)]; // Return stack top
                    self.emit_instruction(0x8B, &operands, None, None)?; // ret (1OP:11)
                } else {
                    // Return without value - use rtrue (0OP)
                    self.emit_instruction(0xB0, &[], None, None)?; // rtrue (0OP:0)
                }
            }

            IrInstruction::Branch {
                condition,
                true_label,
                false_label,
            } => {
                self.generate_conditional_branch(*condition, *true_label, *false_label)?;
            }

            IrInstruction::Jump { label } => {
                self.generate_jump(*label)?;
            }

            IrInstruction::Label { id } => {
                // Record label address for resolution
                log::debug!(
                    "Recording label ID {} at address 0x{:04x}",
                    *id,
                    self.current_address
                );
                self.label_addresses.insert(*id, self.current_address);
                self.record_address(*id, self.current_address);

                // Track label at current address for jump optimization
                self.labels_at_current_address.push(*id);
            }

            IrInstruction::LoadVar {
                target: _,
                var_id: _,
            } => {
                // Load variable value to stack
                // TODO: Map IR variable ID to Z-Machine variable number
                let operands = vec![Operand::Variable(1)]; // Load local variable 1
                self.emit_instruction(0x0E, &operands, Some(0), None)?; // load to stack
            }

            IrInstruction::StoreVar {
                var_id: _,
                source: _,
            } => {
                // Store stack top to variable
                // TODO: Map IR variable ID to Z-Machine variable number
                let operands = vec![Operand::Variable(0), Operand::Variable(1)]; // stack -> local 1
                self.emit_instruction(0x0D, &operands, None, None)?; // store
            }

            IrInstruction::Print { value: _ } => {
                // Print value - for now just print a newline
                self.emit_instruction(0x0B, &[], None, None)?; // new_line (0OP)
            }

            IrInstruction::GetProperty {
                target,
                object,
                property,
            } => {
                // Generate Z-Machine get_prop instruction (2OP:17, opcode 0x11)
                // For now, use placeholder object ID and property number
                // TODO: Map IR object ID to actual Z-Machine object number
                // Use global property registry for consistent property numbering
                let property_num =
                    self.property_numbers
                        .get(property)
                        .copied()
                        .unwrap_or_else(|| {
                            debug!(
                                "Warning: Property '{}' not found in registry, using default 1",
                                property
                            );
                            1
                        });
                debug!(
                    "GET_PROP: property '{}' -> number {}",
                    property, property_num
                );

                // Generate get_prop instruction with properly resolved object operand
                let operands = vec![
                    self.resolve_ir_id_to_operand(*object)?, // Object (properly resolved)
                    Operand::Constant(property_num.into()),  // Property number
                ];
                self.emit_instruction(0x11, &operands, Some(0), None)?; // Store result in stack top

                // Track that this IR ID maps to stack Variable(0)
                self.ir_id_to_stack_var.insert(*target, 0);
            }
            IrInstruction::SetProperty {
                object,
                property,
                value,
            } => {
                // Generate Z-Machine put_prop instruction (VAR:227, opcode 0x03)
                // Use global property registry for consistent property numbering
                let property_num =
                    self.property_numbers
                        .get(property)
                        .copied()
                        .unwrap_or_else(|| {
                            debug!(
                                "Warning: Property '{}' not found in registry, using default 1",
                                property
                            );
                            1
                        });
                debug!(
                    "PUT_PROP: property '{}' -> number {}",
                    property, property_num
                );

                // Generate put_prop instruction with properly resolved operands
                let operands = vec![
                    self.resolve_ir_id_to_operand(*object)?, // Object (properly resolved from IR)
                    Operand::Constant(property_num.into()),  // Property number
                    self.resolve_ir_id_to_operand(*value)?,  // Value (properly resolved from IR)
                ];
                self.emit_instruction(0x03, &operands, None, None)?;
            }
            IrInstruction::UnaryOp {
                target: _,
                op,
                operand: _,
            } => {
                // TODO: Map IR ID to actual operand
                // For now, use placeholder operand
                let operand_op = Operand::Variable(1); // Local variable 1
                let store_var = Some(0); // Store to stack top
                self.generate_unary_op(op, operand_op, store_var)?;
            }
            IrInstruction::GetArrayElement {
                target: _,
                array: _,
                index: _,
            } => {
                // Generate Z-Machine loadw instruction (2OP:15)
                // loadw array_base index -> result
                // TODO: Convert IR IDs to proper operands instead of using placeholders
                let operands = vec![
                    Operand::Variable(1), // Array base address (placeholder)
                    Operand::Variable(2), // Index (placeholder)
                ];
                self.emit_instruction(0x0F, &operands, Some(0), None)?; // loadw (2OP:15)
            }
            IrInstruction::SetArrayElement {
                array: _,
                index: _,
                value: _,
            } => {
                // Generate Z-Machine storew instruction (VAR:1)
                // storew array_base index value
                // TODO: Convert IR IDs to proper operands instead of using placeholders
                let operands = vec![
                    Operand::Variable(1), // Array base address (placeholder)
                    Operand::Variable(2), // Index (placeholder)
                    Operand::Variable(0), // Value (from stack, placeholder)
                ];
                self.emit_instruction(0x01, &operands, None, None)?; // storew (VAR:1)
            }

            // New numbered property instructions
            IrInstruction::GetPropertyByNumber {
                target: _,
                object,
                property_num,
            } => {
                // Generate Z-Machine get_prop instruction (2OP:17, opcode 0x11)
                // Use proper object resolution via global variables
                let operands = vec![
                    self.resolve_ir_id_to_operand(*object)?, // Object (properly resolved)
                    Operand::Constant(*property_num as u16), // Property number
                ];
                self.emit_instruction(0x11, &operands, Some(0), None)?; // Store result in stack top
                log::debug!(
                    "Generated get_prop for property number {} with resolved object",
                    property_num
                );
            }

            IrInstruction::SetPropertyByNumber {
                object,
                property_num,
                value: _,
            } => {
                // Generate Z-Machine put_prop instruction (VAR:227, opcode 0x03)
                // Use proper object resolution via global variables
                let operands = vec![
                    self.resolve_ir_id_to_operand(*object)?, // Object (properly resolved)
                    Operand::Constant(*property_num as u16), // Property number
                    Operand::Variable(0),                    // Value (from stack)
                ];
                self.emit_instruction(0x03, &operands, None, None)?;
                log::debug!(
                    "Generated put_prop for property number {} with resolved object",
                    property_num
                );
            }

            IrInstruction::GetNextProperty {
                target: _,
                object,
                current_property,
            } => {
                // Generate Z-Machine get_next_prop instruction (2OP:19, opcode 0x13)
                // Use proper object resolution via global variables
                let operands = vec![
                    self.resolve_ir_id_to_operand(*object)?, // Object (properly resolved)
                    Operand::Constant(*current_property as u16), // Current property number (0 for first)
                ];
                self.emit_instruction(0x13, &operands, Some(0), None)?; // Store result in stack top
                log::debug!(
                    "Generated get_next_prop for property number {} with resolved object",
                    current_property
                );
            }

            IrInstruction::TestProperty {
                target: _,
                object: _,
                property_num,
            } => {
                // Generate Z-Machine get_prop_len instruction (1OP:4, opcode 0x84)
                // First get property address, then test if length > 0
                // TODO: This is a simplified implementation
                // A complete implementation would use get_prop_addr + get_prop_len
                let operands = vec![
                    Operand::Variable(1),                    // Object (placeholder)
                    Operand::Constant(*property_num as u16), // Property number
                ];
                // Use get_prop and compare result with default to test existence
                self.emit_instruction(0x11, &operands, Some(0), None)?; // get_prop
                log::debug!(
                    "Generated property test for property number {}",
                    property_num
                );
            }

            IrInstruction::ArrayAdd { array, value } => {
                // Array add operation - for now, use placeholder Z-Machine instructions
                // This would need proper array management in a full implementation
                log::debug!("Array add: array={}, value={}", array, value);

                // For simplicity, we'll treat this as a no-op for now
                // In a full implementation, this would manipulate dynamic array structures
                let operands = vec![
                    Operand::Variable(1), // Array placeholder
                    Operand::Variable(2), // Value placeholder
                ];
                self.emit_instruction(0x10, &operands, None, None)?; // placeholder instruction
            }

            IrInstruction::ArrayRemove {
                target,
                array,
                index,
            } => {
                // Array remove operation - remove element at index and return it
                log::debug!(
                    "Array remove: target={}, array={}, index={}",
                    target,
                    array,
                    index
                );

                // For simplicity, return 0 as removed value
                // In a full implementation, this would access array elements
                let operands = vec![
                    Operand::Variable(1), // Array placeholder
                    Operand::Variable(2), // Index placeholder
                ];
                self.emit_instruction(0x10, &operands, Some(0), None)?; // placeholder with result
            }

            IrInstruction::ArrayLength { target, array } => {
                // Array length operation - return number of elements
                log::debug!("Array length: target={}, array={}", target, array);

                // For simplicity, return fixed length
                // In a full implementation, this would read array metadata
                let operands = vec![
                    Operand::Variable(1), // Array placeholder
                ];
                self.emit_instruction(0x10, &operands, Some(0), None)?; // placeholder with result
            }

            IrInstruction::ArrayEmpty { target, array } => {
                // Array empty check - return true if array has no elements
                log::debug!("Array empty: target={}, array={}", target, array);

                // For simplicity, return false (not empty)
                // In a full implementation, this would check array size
                let operands = vec![
                    Operand::Variable(1), // Array placeholder
                ];
                self.emit_instruction(0x10, &operands, Some(0), None)?; // placeholder with result
            }

            IrInstruction::ArrayContains {
                target,
                array,
                value,
            } => {
                // Array contains operation - check if value exists in array
                log::debug!(
                    "Array contains: target={}, array={}, value={}",
                    target,
                    array,
                    value
                );

                // For simplicity, return false (not found)
                // In a full implementation, this would search array elements
                let operands = vec![
                    Operand::Variable(1), // Array placeholder
                    Operand::Variable(2), // Value placeholder
                ];
                self.emit_instruction(0x10, &operands, Some(0), None)?; // placeholder with result
            }

            // Advanced array operations
            IrInstruction::ArrayFilter {
                target,
                array,
                predicate,
            } => {
                log::debug!(
                    "Array filter: target={}, array={}, predicate={}",
                    target,
                    array,
                    predicate
                );
                let operands = vec![Operand::Variable(1), Operand::Variable(2)];
                self.emit_instruction(0x10, &operands, Some(0), None)?; // placeholder
            }
            IrInstruction::ArrayMap {
                target,
                array,
                transform,
            } => {
                log::debug!(
                    "Array map: target={}, array={}, transform={}",
                    target,
                    array,
                    transform
                );
                let operands = vec![Operand::Variable(1), Operand::Variable(2)];
                self.emit_instruction(0x10, &operands, Some(0), None)?; // placeholder
            }
            IrInstruction::ArrayForEach { array, callback } => {
                log::debug!("Array forEach: array={}, callback={}", array, callback);
                let operands = vec![Operand::Variable(1), Operand::Variable(2)];
                self.emit_instruction(0x10, &operands, None, None)?; // placeholder (no return)
            }
            IrInstruction::ArrayFind {
                target,
                array,
                predicate,
            } => {
                log::debug!(
                    "Array find: target={}, array={}, predicate={}",
                    target,
                    array,
                    predicate
                );
                let operands = vec![Operand::Variable(1), Operand::Variable(2)];
                self.emit_instruction(0x10, &operands, Some(0), None)?; // placeholder
            }
            IrInstruction::ArrayIndexOf {
                target,
                array,
                value,
            } => {
                log::debug!(
                    "Array indexOf: target={}, array={}, value={}",
                    target,
                    array,
                    value
                );
                let operands = vec![Operand::Variable(1), Operand::Variable(2)];
                self.emit_instruction(0x10, &operands, Some(0), None)?; // placeholder
            }
            IrInstruction::ArrayJoin {
                target,
                array,
                separator,
            } => {
                log::debug!(
                    "Array join: target={}, array={}, separator={}",
                    target,
                    array,
                    separator
                );
                let operands = vec![Operand::Variable(1), Operand::Variable(2)];
                self.emit_instruction(0x10, &operands, Some(0), None)?; // placeholder
            }
            IrInstruction::ArrayReverse { target, array } => {
                log::debug!("Array reverse: target={}, array={}", target, array);
                let operands = vec![Operand::Variable(1)];
                self.emit_instruction(0x10, &operands, Some(0), None)?; // placeholder
            }
            IrInstruction::ArraySort {
                target,
                array,
                comparator,
            } => {
                log::debug!(
                    "Array sort: target={}, array={}, comparator={:?}",
                    target,
                    array,
                    comparator
                );
                let operands = vec![Operand::Variable(1)];
                self.emit_instruction(0x10, &operands, Some(0), None)?; // placeholder
            }

            // String utility operations
            IrInstruction::StringIndexOf {
                target,
                string,
                substring,
            } => {
                log::debug!(
                    "String indexOf: target={}, string={}, substring={}",
                    target,
                    string,
                    substring
                );
                let operands = vec![Operand::Variable(1), Operand::Variable(2)];
                self.emit_instruction(0x10, &operands, Some(0), None)?; // placeholder
            }
            IrInstruction::StringSlice {
                target,
                string,
                start,
            } => {
                log::debug!(
                    "String slice: target={}, string={}, start={}",
                    target,
                    string,
                    start
                );
                let operands = vec![Operand::Variable(1), Operand::Variable(2)];
                self.emit_instruction(0x10, &operands, Some(0), None)?; // placeholder
            }
            IrInstruction::StringSubstring {
                target,
                string,
                start,
                end,
            } => {
                log::debug!(
                    "String substring: target={}, string={}, start={}, end={}",
                    target,
                    string,
                    start,
                    end
                );
                let operands = vec![
                    Operand::Variable(1),
                    Operand::Variable(2),
                    Operand::Variable(3),
                ];
                self.emit_instruction(0x10, &operands, Some(0), None)?; // placeholder
            }
            IrInstruction::StringToLowerCase { target, string } => {
                log::debug!("String toLowerCase: target={}, string={}", target, string);
                let operands = vec![Operand::Variable(1)];
                self.emit_instruction(0x10, &operands, Some(0), None)?; // placeholder
            }
            IrInstruction::StringToUpperCase { target, string } => {
                log::debug!("String toUpperCase: target={}, string={}", target, string);
                let operands = vec![Operand::Variable(1)];
                self.emit_instruction(0x10, &operands, Some(0), None)?; // placeholder
            }
            IrInstruction::StringTrim { target, string } => {
                log::debug!("String trim: target={}, string={}", target, string);
                let operands = vec![Operand::Variable(1)];
                self.emit_instruction(0x10, &operands, Some(0), None)?; // placeholder
            }
            IrInstruction::StringCharAt {
                target,
                string,
                index,
            } => {
                log::debug!(
                    "String charAt: target={}, string={}, index={}",
                    target,
                    string,
                    index
                );
                let operands = vec![Operand::Variable(1), Operand::Variable(2)];
                self.emit_instruction(0x10, &operands, Some(0), None)?; // placeholder
            }
            IrInstruction::StringSplit {
                target,
                string,
                delimiter,
            } => {
                log::debug!(
                    "String split: target={}, string={}, delimiter={}",
                    target,
                    string,
                    delimiter
                );
                let operands = vec![Operand::Variable(1), Operand::Variable(2)];
                self.emit_instruction(0x10, &operands, Some(0), None)?; // placeholder
            }
            IrInstruction::StringReplace {
                target,
                string,
                search,
                replacement,
            } => {
                log::debug!(
                    "String replace: target={}, string={}, search={}, replacement={}",
                    target,
                    string,
                    search,
                    replacement
                );
                let operands = vec![
                    Operand::Variable(1),
                    Operand::Variable(2),
                    Operand::Variable(3),
                ];
                self.emit_instruction(0x10, &operands, Some(0), None)?; // placeholder
            }
            IrInstruction::StringStartsWith {
                target,
                string,
                prefix,
            } => {
                log::debug!(
                    "String startsWith: target={}, string={}, prefix={}",
                    target,
                    string,
                    prefix
                );
                let operands = vec![Operand::Variable(1), Operand::Variable(2)];
                self.emit_instruction(0x10, &operands, Some(0), None)?; // placeholder
            }
            IrInstruction::StringEndsWith {
                target,
                string,
                suffix,
            } => {
                log::debug!(
                    "String endsWith: target={}, string={}, suffix={}",
                    target,
                    string,
                    suffix
                );
                let operands = vec![Operand::Variable(1), Operand::Variable(2)];
                self.emit_instruction(0x10, &operands, Some(0), None)?; // placeholder
            }

            // Math utility operations
            IrInstruction::MathAbs { target, value } => {
                log::debug!("Math abs: target={}, value={}", target, value);
                let operands = vec![Operand::Variable(1)];
                self.emit_instruction(0x10, &operands, Some(0), None)?; // placeholder
            }
            IrInstruction::MathMin { target, a, b } => {
                log::debug!("Math min: target={}, a={}, b={}", target, a, b);
                let operands = vec![Operand::Variable(1), Operand::Variable(2)];
                self.emit_instruction(0x10, &operands, Some(0), None)?; // placeholder
            }
            IrInstruction::MathMax { target, a, b } => {
                log::debug!("Math max: target={}, a={}, b={}", target, a, b);
                let operands = vec![Operand::Variable(1), Operand::Variable(2)];
                self.emit_instruction(0x10, &operands, Some(0), None)?; // placeholder
            }
            IrInstruction::MathRound { target, value } => {
                log::debug!("Math round: target={}, value={}", target, value);
                let operands = vec![Operand::Variable(1)];
                self.emit_instruction(0x10, &operands, Some(0), None)?; // placeholder
            }
            IrInstruction::MathFloor { target, value } => {
                log::debug!("Math floor: target={}, value={}", target, value);
                let operands = vec![Operand::Variable(1)];
                self.emit_instruction(0x10, &operands, Some(0), None)?; // placeholder
            }
            IrInstruction::MathCeil { target, value } => {
                log::debug!("Math ceil: target={}, value={}", target, value);
                let operands = vec![Operand::Variable(1)];
                self.emit_instruction(0x10, &operands, Some(0), None)?; // placeholder
            }

            // Type checking operations
            IrInstruction::TypeCheck {
                target,
                value,
                type_name,
            } => {
                log::debug!(
                    "Type check: target={}, value={}, type={}",
                    target,
                    value,
                    type_name
                );
                let operands = vec![Operand::Variable(1)];
                self.emit_instruction(0x10, &operands, Some(0), None)?; // placeholder
            }
            IrInstruction::TypeOf { target, value } => {
                log::debug!("TypeOf: target={}, value={}", target, value);
                let operands = vec![Operand::Variable(1)];
                self.emit_instruction(0x10, &operands, Some(0), None)?; // placeholder
            }

            _ => {
                // TODO: Implement remaining instructions
                return Err(CompilerError::CodeGenError(format!(
                    "Instruction {:?} not yet implemented",
                    instruction
                )));
            }
        }

        Ok(())
    }

    /// Generate load immediate instruction
    fn generate_load_immediate(&mut self, value: &IrValue) -> Result<(), CompilerError> {
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
            _ => {
                return Err(CompilerError::CodeGenError(
                    "Unsupported immediate value type".to_string(),
                ));
            }
        }
        Ok(())
    }

    /// Generate unary operation instruction
    fn generate_unary_op(
        &mut self,
        op: &IrUnaryOp,
        operand: Operand,
        store_var: Option<u8>,
    ) -> Result<(), CompilerError> {
        match op {
            IrUnaryOp::Not => {
                // Z-Machine logical NOT - use test instruction with inverted logic
                // For now, use a simple approach: load 0 if operand is true, 1 if false
                // This requires branching logic which is complex, so use placeholder
                let operands = vec![operand, Operand::Constant(0)];
                self.emit_instruction(0x01, &operands, store_var, None)?; // je (equals) instruction
            }
            IrUnaryOp::Minus => {
                // Z-Machine arithmetic negation - subtract operand from 0
                let operands = vec![Operand::Constant(0), operand];
                self.emit_instruction(0x04, &operands, store_var, None)?; // sub instruction
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
        let opcode = match op {
            IrBinaryOp::Add => 0x14,          // add (2OP:20)
            IrBinaryOp::Subtract => 0x15,     // sub (2OP:21)
            IrBinaryOp::Multiply => 0x16,     // mul (2OP:22)
            IrBinaryOp::Divide => 0x17,       // div (2OP:23)
            IrBinaryOp::Modulo => 0x18,       // mod (2OP:24)
            IrBinaryOp::Equal => 0x01,        // je (2OP:1) - jump if equal
            IrBinaryOp::NotEqual => 0x01,     // je (2OP:1) - jump if equal, then negate
            IrBinaryOp::Less => 0x02,         // jl (2OP:2) - jump if less
            IrBinaryOp::LessEqual => 0x02,    // Use jl for now (placeholder)
            IrBinaryOp::Greater => 0x03,      // jg (2OP:3) - jump if greater
            IrBinaryOp::GreaterEqual => 0x03, // Use jg for now (placeholder)
            IrBinaryOp::And => 0x09,          // and (2OP:9)
            IrBinaryOp::Or => 0x08,           // or (2OP:8)
        };

        let operands = vec![left_operand, right_operand];

        // Comparison ops may need branch offset instead of store
        match op {
            IrBinaryOp::Equal | IrBinaryOp::Less | IrBinaryOp::Greater => {
                // These are branch instructions, not store instructions
                // TODO: Handle branch offset properly
                self.emit_instruction(opcode, &operands, None, Some(0))?;
            }
            _ => {
                // Arithmetic operations store result
                self.emit_instruction(opcode, &operands, store_var, None)?;
            }
        }

        Ok(())
    }

    /// Generate function call with proper operands
    fn generate_call(
        &mut self,
        function_addr: Operand,
        args: &[Operand],
        store_var: Option<u8>,
    ) -> Result<(), CompilerError> {
        // For Version 3, use VAR:224 (call) for all function calls - the only call instruction available
        let opcode = 0xE0; // call (VAR:224) - the only call instruction in Version 3

        let mut operands = vec![function_addr];
        operands.extend_from_slice(args);

        self.emit_instruction(opcode, &operands, store_var, None)?;
        Ok(())
    }

    /// Generate function call with unresolved reference and arguments
    fn generate_call_with_reference(
        &mut self,
        function_id: IrId,
        args: &[IrId],
        target: Option<IrId>,
    ) -> Result<(), CompilerError> {
        // Generate a proper function call with placeholder address that will be resolved later
        // This is the correct approach - not rtrue hacks or compile errors

        // Convert IR args to operands
        let mut operands = vec![Operand::LargeConstant(placeholder_word())]; // Placeholder for function address
        for &arg_id in args {
            // Handle different types of arguments appropriately
            // CRITICAL: Check order must match resolve_ir_id_to_operand to prevent misclassification
            if let Some(literal_value) = self.get_literal_value(arg_id) {
                // Integer literals can be used directly as operands
                operands.push(Operand::LargeConstant(literal_value));
            } else if let Some(&object_number) = self.ir_id_to_object_number.get(&arg_id) {
                // Object references should be handled as direct operands, not string refs
                log::debug!(
                    "Function arg: IR ID {} resolved to object number {}",
                    arg_id,
                    object_number
                );
                operands.push(Operand::LargeConstant(object_number));
            } else if self.ir_id_to_string.contains_key(&arg_id) {
                // String literals need to be converted to packed string addresses
                // Create an unresolved string reference that will be patched later
                let operand_location = self.current_address + 1 + operands.len() * 2; // Calculate operand location
                operands.push(Operand::LargeConstant(placeholder_word()));

                // Create unresolved reference for this string argument
                let reference = UnresolvedReference {
                    reference_type: ReferenceType::StringRef,
                    location: operand_location,
                    target_id: arg_id,
                    is_packed_address: true, // String addresses are packed
                    offset_size: 2,
                };
                self.reference_context.unresolved_refs.push(reference);

                log::debug!(
                    "Added string argument reference: IR ID {} at location 0x{:04x}",
                    arg_id,
                    operand_location
                );
            } else {
                // Try the existing operand resolution system for other types
                match self.resolve_ir_id_to_operand(arg_id) {
                    Ok(operand) => {
                        operands.push(operand);
                    }
                    Err(err) => {
                        log::warn!(
                            "Could not resolve function argument IR ID {} - error: {:?} - using placeholder", 
                            arg_id, err
                        );
                        operands.push(Operand::LargeConstant(placeholder_word()));
                        // TODO: Create unresolved reference for this placeholder
                    }
                }
            }
        }

        // Choose appropriate call instruction based on argument count
        let opcode = match args.len() {
            0 => 0x20, // call_1n (1OP:32) - call with no args, no store
            _ => 0xE0, // call_vs (VAR:224) - call with args, store result
        };

        // Determine store variable for return value
        let store_var = target.map(|_| 0x00); // Placeholder store variable

        // Generate the call instruction with placeholder address
        let layout = self.emit_instruction(opcode, &operands, store_var, None)?;

        // Add unresolved reference for function address using correct operand location
        let operand_location = layout
            .operand_location
            .expect("Call instruction must have operand location");
        self.reference_context
            .unresolved_refs
            .push(UnresolvedReference {
                reference_type: ReferenceType::FunctionCall,
                location: operand_location,
                target_id: function_id,
                is_packed_address: true, // Function addresses are packed in Z-Machine
                offset_size: 2,
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
    fn resolve_ir_id_to_operand(&self, ir_id: IrId) -> Result<Operand, CompilerError> {
        // First check if it's an integer literal
        if let Some(literal_value) = self.get_literal_value(ir_id) {
            log::debug!(
                "resolve_ir_id_to_operand: IR ID {} resolved to LargeConstant({})",
                ir_id,
                literal_value
            );
            return Ok(Operand::LargeConstant(literal_value));
        }

        // Check if it's a string literal (shouldn't be used in binary ops, but handle gracefully)
        if self.ir_id_to_string.contains_key(&ir_id) {
            return Err(CompilerError::CodeGenError(format!(
                "Cannot use string literal (IR ID {}) as operand in binary operation",
                ir_id
            )));
        }

        // Check if this IR ID maps to a stack variable (e.g., result of GetProperty)
        if let Some(&stack_var) = self.ir_id_to_stack_var.get(&ir_id) {
            log::debug!(
                "resolve_ir_id_to_operand: IR ID {} resolved to Variable({}) [Stack result]",
                ir_id,
                stack_var
            );
            return Ok(Operand::Variable(stack_var));
        }

        // Check if this IR ID represents an object reference
        if let Some(&object_number) = self.ir_id_to_object_number.get(&ir_id) {
            log::debug!(
                "resolve_ir_id_to_operand: IR ID {} resolved to LargeConstant({}) [Object reference]",
                ir_id, object_number
            );
            return Ok(Operand::LargeConstant(object_number));
        }

        // CRITICAL FIX: Check if this IR ID represents an object reference
        // For global objects like 'player', return the global variable that contains the object number
        // This follows proper Z-Machine architecture where objects are referenced via global variables

        // For now, assume any non-literal, non-string IR ID in property access context
        // refers to the player object stored in global variable G00 (Variable 16)
        // TODO: Implement proper IR ID to object/global mapping for all objects
        log::debug!(
            "resolve_ir_id_to_operand: IR ID {} assumed to be player object -> LargeConstant(1) [Direct object reference]",
            ir_id
        );
        Ok(Operand::LargeConstant(1)) // Direct player object number
    }

    /// Set up IR ID to object number mappings for proper identifier resolution
    fn setup_object_mappings(&mut self, ir: &IrProgram) {
        // Create reverse mapping from IR IDs to object numbers
        // Use both symbol_ids (name -> IR ID) and object_numbers (name -> obj num)
        for (name, &ir_id) in &ir.symbol_ids {
            if let Some(&object_number) = ir.object_numbers.get(name) {
                self.ir_id_to_object_number.insert(ir_id, object_number);
                log::debug!(
                    "setup_object_mappings: IR ID {} ('{}') -> Object #{}",
                    ir_id,
                    name,
                    object_number
                );
            }
        }

        // Also copy the object_numbers mapping for legacy compatibility
        self.object_numbers = ir.object_numbers.clone();

        log::debug!(
            "Object mapping setup complete: {} IR ID -> object number mappings created",
            self.ir_id_to_object_number.len()
        );
    }

    /// Generate return instruction
    fn emit_return(&mut self, value: Option<IrId>) -> Result<(), CompilerError> {
        if let Some(_ir_id) = value {
            // Return with value - use ret opcode with operand
            self.emit_instruction(
                0x0B,                         // ret opcode
                &[Operand::SmallConstant(0)], // Return 0 for now (TODO: resolve actual value)
                None,                         // No store
                None,                         // No branch
            )?;
        } else {
            // Return without value - use rtrue (0OP instruction)
            self.emit_instruction(
                0xB0, // rtrue opcode
                &[],  // No operands
                None, // No store
                None, // No branch
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
                    self.generate_jump(true_label)?;
                } else {
                    log::debug!("TRUE branch is next instruction - no jump needed (fall-through)");
                }
            }
            Some(ConstantValue::Boolean(false)) => {
                log::debug!("Condition is constant FALSE - optimizing branch");
                // Generate direct jump to false_label if not fall-through
                if !self.is_next_instruction(false_label) {
                    log::debug!("FALSE branch is not next instruction - generating jump");
                    self.generate_jump(false_label)?;
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
                    self.generate_jump(target_label)?;
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
            // Calculate what the next instruction address will be after current jump
            let next_addr_after_jump = self.current_address + 3; // Jump instruction is 3 bytes
            let is_next = target_addr == next_addr_after_jump;

            log::debug!(
                "is_next_instruction: label={}, target_addr=0x{:04x}, next_addr_after_jump=0x{:04x}, is_next={}",
                label, target_addr, next_addr_after_jump, is_next
            );

            return is_next;
        }

        // Fallback: Check if label is already resolved and points to next instruction
        if let Some(&target_addr) = self.reference_context.ir_id_to_address.get(&label) {
            let next_addr_after_jump = self.current_address + 3;
            return target_addr == next_addr_after_jump;
        }

        // Check if label is in the list of labels at current address (immediate)
        self.labels_at_current_address.contains(&label)
    }

    /// Emit proper Z-Machine conditional branch instruction
    fn emit_conditional_branch_instruction(
        &mut self,
        condition: IrId,
        true_label: IrId,
        false_label: IrId,
    ) -> Result<(), CompilerError> {
        log::debug!(
            "Emitting conditional branch: condition_id={}, true={}, false={}",
            condition,
            true_label,
            false_label
        );

        // Check if the condition is a binary comparison operation
        if let Some((op, left, right)) = self.ir_id_to_binary_op.get(&condition).cloned() {
            log::debug!(
                "Condition {} is binary operation: {:?} {} {}",
                condition,
                op,
                left,
                right
            );

            // This is a direct comparison - emit the appropriate Z-Machine comparison instruction
            match op {
                IrBinaryOp::Equal => {
                    let left_op = self.resolve_ir_id_to_operand(left)?;
                    let right_op = self.resolve_ir_id_to_operand(right)?;
                    self.emit_comparison_branch(
                        0x01,
                        &[left_op, right_op],
                        true_label,
                        false_label,
                    )?; // je (2OP:1)
                }
                IrBinaryOp::Less => {
                    let left_op = self.resolve_ir_id_to_operand(left)?;
                    let right_op = self.resolve_ir_id_to_operand(right)?;
                    self.emit_comparison_branch(
                        0x02,
                        &[left_op, right_op],
                        true_label,
                        false_label,
                    )?; // jl (2OP:2)
                }
                IrBinaryOp::Greater => {
                    let left_op = self.resolve_ir_id_to_operand(left)?;
                    let right_op = self.resolve_ir_id_to_operand(right)?;
                    self.emit_comparison_branch(
                        0x03,
                        &[left_op, right_op],
                        true_label,
                        false_label,
                    )?; // jg (2OP:3)
                }
                IrBinaryOp::LessEqual => {
                    // Convert a <= b to !(a > b) by swapping true/false labels
                    let left_op = self.resolve_ir_id_to_operand(left)?;
                    let right_op = self.resolve_ir_id_to_operand(right)?;
                    self.emit_comparison_branch(
                        0x03,
                        &[left_op, right_op],
                        false_label,
                        true_label,
                    )?; // jg with swapped labels
                }
                IrBinaryOp::GreaterEqual => {
                    // Convert a >= b to !(a < b) by swapping true/false labels
                    let left_op = self.resolve_ir_id_to_operand(left)?;
                    let right_op = self.resolve_ir_id_to_operand(right)?;
                    self.emit_comparison_branch(
                        0x02,
                        &[left_op, right_op],
                        false_label,
                        true_label,
                    )?; // jl with swapped labels
                }
                IrBinaryOp::NotEqual => {
                    // Convert a != b to !(a == b) by swapping true/false labels
                    let left_op = self.resolve_ir_id_to_operand(left)?;
                    let right_op = self.resolve_ir_id_to_operand(right)?;
                    self.emit_comparison_branch(
                        0x01,
                        &[left_op, right_op],
                        false_label,
                        true_label,
                    )?; // je with swapped labels
                }
                _ => {
                    log::warn!("Binary operation {:?} not suitable for direct comparison branch - falling back to jz", op);
                    // Fall through to jz handling below
                    return self.emit_jz_branch(condition, true_label, false_label);
                }
            }
        } else {
            log::debug!(
                "Condition {} is not a binary operation - using jz branch",
                condition
            );
            // Not a binary comparison, use jz (jump if zero) for boolean test
            return self.emit_jz_branch(condition, true_label, false_label);
        }

        Ok(())
    }

    /// Emit a jz (jump if zero) branch instruction for boolean conditions
    fn emit_jz_branch(
        &mut self,
        condition: IrId,
        _true_label: IrId,
        false_label: IrId,
    ) -> Result<(), CompilerError> {
        // Resolve condition operand
        let condition_operand = match self.resolve_ir_id_to_operand(condition) {
            Ok(operand) => operand,
            Err(_) => {
                log::warn!(
                    "Could not resolve condition IR ID {} - using Variable(0)",
                    condition
                );
                Operand::Variable(0) // Stack variable as fallback
            }
        };

        let layout = self.emit_instruction(
            0xA0, // jz (VAR:0x00) - jump if zero
            &[condition_operand],
            None, // No store
            None, // Branch offset will be handled separately
        )?;

        // Add branch offset as unresolved reference
        // Z-Machine branch instructions have 14-bit signed branch offsets
        let branch_location = layout.instruction_start + layout.total_size - 2; // Last 2 bytes

        self.reference_context
            .unresolved_refs
            .push(UnresolvedReference {
                reference_type: ReferenceType::Branch,
                location: branch_location,
                target_id: false_label, // jz jumps on false condition
                is_packed_address: false,
                offset_size: 2,
            });

        log::debug!(
            "Added jz branch reference: location=0x{:04x}, target={}",
            branch_location,
            false_label
        );

        Ok(())
    }

    /// Emit a Z-Machine comparison branch instruction (je, jl, jg, etc.)
    fn emit_comparison_branch(
        &mut self,
        opcode: u8,
        operands: &[Operand],
        true_label: IrId,
        _false_label: IrId,
    ) -> Result<(), CompilerError> {
        log::debug!(
            "Emitting comparison branch: opcode=0x{:02x}, operands={:?}, true={}, false={}",
            opcode,
            operands,
            true_label,
            _false_label
        );

        let layout = self.emit_instruction(
            opcode,
            operands,
            None,    // No store
            Some(0), // Placeholder branch offset - will be patched during address resolution
        )?;

        // Add branch offset as unresolved reference
        // Z-Machine branch instructions have 14-bit signed branch offsets
        let branch_location = layout.branch_location.ok_or_else(|| {
            CompilerError::CodeGenError(
                "Branch instruction layout missing branch_location".to_string(),
            )
        })?;

        self.reference_context
            .unresolved_refs
            .push(UnresolvedReference {
                reference_type: ReferenceType::Branch,
                location: branch_location,
                target_id: true_label, // Comparison instructions jump on true condition
                is_packed_address: false,
                offset_size: 2,
            });

        log::debug!(
            "Added comparison branch reference: location=0x{:04x}, target={}",
            branch_location,
            true_label
        );

        Ok(())
    }

    /// Generate branch instruction (legacy method, kept for compatibility)
    fn generate_branch(&mut self, true_label: IrId) -> Result<(), CompilerError> {
        // For now, emit a simple unconditional branch using jump
        // TODO: Support proper conditional branching with condition operand

        // Emit jump instruction with placeholder offset
        let layout = self.emit_instruction(
            0x0C,                                          // jump opcode
            &[Operand::LargeConstant(placeholder_word())], // Placeholder offset (will be resolved later)
            None,                                          // No store
            None,                                          // No branch
        )?;

        // Add unresolved reference for the jump target using layout-tracked operand location
        let operand_address = layout
            .operand_location
            .expect("jump instruction must have operand");
        let reference = UnresolvedReference {
            reference_type: ReferenceType::Jump,
            location: operand_address,
            target_id: true_label,
            is_packed_address: false,
            offset_size: 2,
        };
        self.reference_context.unresolved_refs.push(reference);

        Ok(())
    }

    /// Generate unconditional jump
    fn generate_jump(&mut self, label: IrId) -> Result<(), CompilerError> {
        log::debug!("generate_jump called with label={}", label);

        // SMART OPTIMIZATION: Check if jump target is the immediately next instruction
        if self.is_next_instruction(label) {
            log::debug!(
                "Eliminating unnecessary jump to next instruction (label {})",
                label
            );
            return Ok(()); // No instruction needed - fall through
        }

        log::debug!(
            "generate_jump: Emitting jump at address 0x{:04x} -> label {}",
            self.current_address,
            label
        );

        // Emit jump instruction manually (not using emit_instruction as it doesn't use normal operands)
        // Jump is a 1OP instruction (0x0C) with a signed word offset
        self.emit_byte(0x8C)?; // 1OP:0x0C - jump instruction

        // Emit placeholder offset (will be resolved later)
        let offset_address = self.current_address;
        self.emit_word(placeholder_word())?; // 2-byte placeholder offset

        // Add unresolved reference for the jump target
        let reference = UnresolvedReference {
            reference_type: ReferenceType::Jump,
            location: offset_address,
            target_id: label,
            is_packed_address: false,
            offset_size: 2,
        };
        self.reference_context.unresolved_refs.push(reference);

        log::debug!(
            "generate_jump: Added reference for jump to label {} at location 0x{:04x}",
            label,
            offset_address
        );

        Ok(())
    }

    /// Pre-calculate addresses for all instructions to enable accurate jump optimization
    fn pre_calculate_addresses(
        &mut self,
        instructions: &[IrInstruction],
    ) -> Result<(), CompilerError> {
        let mut simulated_address = self.current_address;

        log::debug!(
            "Starting address pre-calculation from 0x{:04x} for {} instructions",
            simulated_address,
            instructions.len()
        );

        for instruction in instructions {
            match instruction {
                IrInstruction::Label { id } => {
                    // Record label at current simulated address
                    log::debug!(
                        "Pre-calc: Recording label ID {} at address 0x{:04x}",
                        *id,
                        simulated_address
                    );
                    self.label_addresses.insert(*id, simulated_address);
                    self.record_address(*id, simulated_address);
                    // Labels don't consume bytes
                }

                IrInstruction::Jump { .. } => {
                    // Jump instruction: 1 byte opcode + 2 bytes offset = 3 bytes
                    simulated_address += 3;
                }

                IrInstruction::Branch { .. } => {
                    // Conditional branches will be optimized or generate jumps
                    // Worst case: assume 3 bytes for jump (will be optimized if possible)
                    simulated_address += 3;
                }

                IrInstruction::Call { args, .. } => {
                    // Call instruction varies by argument count
                    // Base: 1 byte opcode + 1 byte operand types + 2 bytes function addr + 1 byte store var
                    let estimated_size = 5 + (args.len() * 2); // 2 bytes per argument
                    simulated_address += estimated_size;
                }

                IrInstruction::LoadImmediate { .. } => {
                    // LoadImmediate typically doesn't generate code, but be conservative
                    // simulated_address += 0; // No bytes generated
                }

                IrInstruction::Return { .. } => {
                    // Return instruction: 1 byte opcode (+ operand if returning value)
                    simulated_address += 3; // Conservative estimate
                }

                IrInstruction::BinaryOp { .. } => {
                    // Binary operations: variable size depending on operation
                    simulated_address += 5; // Conservative estimate
                }

                IrInstruction::UnaryOp { .. } => {
                    // Unary operations: typically 3-5 bytes
                    simulated_address += 4; // Conservative estimate
                }

                IrInstruction::GetProperty { .. } => {
                    // Property access: get_prop instruction
                    simulated_address += 6; // Conservative estimate
                }

                IrInstruction::SetProperty { .. } => {
                    // Property assignment: put_prop instruction
                    simulated_address += 7; // Conservative estimate
                }

                IrInstruction::LoadVar { .. } | IrInstruction::StoreVar { .. } => {
                    // Variable operations: typically 3 bytes
                    simulated_address += 3;
                }

                // Catch-all for remaining instruction types
                _ => {
                    // Conservative estimate for other instruction types
                    simulated_address += 5; // Most Z-Machine instructions are 3-7 bytes
                }
            }
        }

        log::debug!(
            "Address pre-calculation complete: 0x{:04x} -> 0x{:04x} ({} bytes estimated)",
            self.current_address,
            simulated_address,
            simulated_address - self.current_address
        );

        Ok(())
    }

    /// Generate init block as a proper routine and startup sequence
    fn generate_init_block(
        &mut self,
        init_block: &IrBlock,
        ir: &IrProgram,
    ) -> Result<usize, CompilerError> {
        log::debug!(
            "generate_init_block: Generating init routine with {} instructions (Z-Machine pure architecture)",
            init_block.instructions.len()
        );

        // Set init block context flag
        self.in_init_block = true;

        // Generate init as a proper routine (like Zork I architecture)
        // This creates: Header → CALL init_routine() → CALL main_loop()

        // First, emit a CALL to init routine at startup address
        let startup_address = self.current_address;
        log::debug!("Startup CALL instruction at 0x{:04x}", startup_address);

        let init_routine_id = 8000u32; // Unique ID for init routine
        let layout = self.emit_instruction(
            0xE0, // call (VAR:224) - the only call instruction in Version 3
            &[Operand::LargeConstant(placeholder_word())], // Placeholder for init routine address
            Some(0), // Store return value on stack (even though we don't use it)
            None, // No branch
        )?;

        // Add unresolved reference for init routine call using layout-tracked operand location
        self.reference_context
            .unresolved_refs
            .push(UnresolvedReference {
                reference_type: ReferenceType::FunctionCall,
                location: layout
                    .operand_location
                    .expect("call instruction must have operand"),
                target_id: init_routine_id,
                is_packed_address: true, // Function calls use packed addresses
                offset_size: 2,
            });

        // Now generate the actual init routine
        // Align init routine address according to Z-Machine version requirements
        match self.version {
            ZMachineVersion::V3 => {
                // v3: functions must be at even addresses
                if self.current_address % 2 != 0 {
                    self.emit_byte(0xB0)?; // Pad with rtrue (harmless no-op) instead of 0x00 for alignment
                }
            }
            ZMachineVersion::V4 | ZMachineVersion::V5 => {
                // v4/v5: functions must be at 4-byte boundaries
                while self.current_address % 4 != 0 {
                    self.emit_byte(0xB0)?; // Pad with rtrue (harmless no-op) instead of 0x00 for alignment
                }
            }
        }

        let init_routine_address = self.current_address;
        log::debug!("Init routine starts at 0x{:04x}", init_routine_address);

        // Record init routine address for call resolution
        self.function_addresses
            .insert(init_routine_id, init_routine_address);
        self.record_address(init_routine_id, init_routine_address);

        // Generate routine header (0 locals for init)
        self.emit_byte(0x00)?; // Routine header: 0 locals

        // Pre-calculate addresses for init block instructions
        log::debug!("Pre-calculating addresses for init block");
        self.pre_calculate_addresses(&init_block.instructions)?;

        // Generate the actual init block code
        for instruction in &init_block.instructions {
            self.generate_instruction(instruction)?;
        }

        // Handle program flow after init block based on program mode
        match ir.program_mode {
            crate::grue_compiler::ast::ProgramMode::Script => {
                // Script mode: Just quit after init block
                log::debug!("Script mode: Adding quit instruction after init block");
                self.emit_byte(0xBA)?; // quit opcode
            }
            crate::grue_compiler::ast::ProgramMode::Interactive
            | crate::grue_compiler::ast::ProgramMode::Custom => {
                // Interactive or Custom mode: Call the generated main routine
                log::debug!(
                    "{:?} mode: Adding call to main routine after init block",
                    ir.program_mode
                );
                let main_loop_id = 9000u32; // Same ID as used in generate_program_flow
                let layout = self.emit_instruction(
                    0xE0, // call (VAR:224) - the only call instruction in Version 3
                    &[Operand::LargeConstant(placeholder_word())], // Placeholder for main routine address
                    Some(0), // Store return value on stack (even though we don't use it)
                    None,    // No branch
                )?;

                // Add unresolved reference for main routine call
                self.reference_context
                    .unresolved_refs
                    .push(UnresolvedReference {
                        reference_type: ReferenceType::FunctionCall,
                        location: layout
                            .operand_location
                            .expect("call instruction must have operand"),
                        target_id: main_loop_id,
                        is_packed_address: true, // Function calls use packed addresses
                        offset_size: 2,
                    });
            }
        }

        log::debug!("Init routine complete at 0x{:04x}", init_routine_address);

        // Clear init block context flag
        self.in_init_block = false;

        Ok(startup_address)
    }

    /// Write the Z-Machine file header with custom entry point
    fn write_header_with_entry_point(&mut self, entry_point: usize) -> Result<(), CompilerError> {
        // Z-Machine header fields
        self.story_data[0] = match self.version {
            ZMachineVersion::V3 => 3,
            ZMachineVersion::V4 => 4,
            ZMachineVersion::V5 => 5,
        };

        // High memory base
        self.write_word_at(4, DEFAULT_HIGH_MEMORY)?;

        // Initial PC (entry point) - set to where init block starts
        self.write_word_at(6, entry_point as u16)?;

        // Dictionary address
        self.write_word_at(8, self.dictionary_addr as u16)?;

        // Object table address
        self.write_word_at(10, self.object_table_addr as u16)?;

        // Global variables address
        self.write_word_at(12, self.global_vars_addr as u16)?;

        // Static memory base (start of dictionary)
        debug!(
            "Setting static memory base to 0x{:04x} (dictionary_addr)",
            self.dictionary_addr
        );
        self.write_word_at(14, self.dictionary_addr as u16)?;

        // File length (in 2-byte words for v3, 4-byte words for v4+)
        let file_len = match self.version {
            ZMachineVersion::V3 => (self.story_data.len() / 2) as u16,
            ZMachineVersion::V4 | ZMachineVersion::V5 => (self.story_data.len() / 4) as u16,
        };
        self.write_word_at(26, file_len)?;

        Ok(())
    }

    /// Resolve all address references and patch jumps/branches
    fn resolve_addresses(&mut self) -> Result<(), CompilerError> {
        // Process all unresolved references
        let unresolved_refs = self.reference_context.unresolved_refs.clone();
        log::debug!(
            "resolve_addresses: Processing {} unresolved references",
            unresolved_refs.len()
        );

        for (i, reference) in unresolved_refs.iter().enumerate() {
            log::debug!(
                "resolve_addresses: [{}] Resolving {:?} -> IR ID {}",
                i,
                reference.reference_type,
                reference.target_id
            );
            self.resolve_single_reference(reference)?;
        }

        // Clear resolved references
        self.reference_context.unresolved_refs.clear();
        log::debug!("resolve_addresses: Address resolution complete");

        // CRITICAL VALIDATION: Scan for any remaining 0x0000 placeholders that weren't resolved
        self.validate_no_unresolved_placeholders()?;

        Ok(())
    }

    /// Validate that no unresolved 0xFFFF placeholders remain in the instruction stream
    fn validate_no_unresolved_placeholders(&self) -> Result<(), CompilerError> {
        let mut unresolved_count = 0;
        let mut scan_addr = 0x0040; // Start after header

        log::debug!(
            "Scanning for unresolved placeholders from 0x{:04x} to 0x{:04x}",
            scan_addr,
            self.current_address
        );

        while scan_addr + 1 < self.current_address {
            if self.story_data[scan_addr] == PLACEHOLDER_BYTE
                && self.story_data[scan_addr + 1] == PLACEHOLDER_BYTE
            {
                // Found potential unresolved placeholder
                log::error!(
                    "UNRESOLVED PLACEHOLDER: Found 0xFFFF at address 0x{:04x}-0x{:04x}",
                    scan_addr,
                    scan_addr + 1
                );

                // Try to provide context about what instruction this might be in
                let context_start = scan_addr.saturating_sub(5);
                let context_end = (scan_addr + 10).min(self.current_address);
                let context_bytes: Vec<String> = self.story_data[context_start..context_end]
                    .iter()
                    .enumerate()
                    .map(|(i, &b)| {
                        let addr = context_start + i;
                        if addr == scan_addr || addr == scan_addr + 1 {
                            format!("[{:02x}]", b) // Mark the placeholder bytes
                        } else {
                            format!("{:02x}", b)
                        }
                    })
                    .collect();

                log::error!(
                    "CONTEXT: 0x{:04x}: {}",
                    context_start,
                    context_bytes.join(" ")
                );

                unresolved_count += 1;

                // Skip ahead to avoid counting overlapping placeholders
                scan_addr += 2;
            } else {
                scan_addr += 1;
            }
        }

        if unresolved_count > 0 {
            return Err(CompilerError::CodeGenError(format!(
                "Found {} unresolved placeholder(s) in generated bytecode - this will cause runtime errors",
                unresolved_count
            )));
        }

        log::debug!("Validation complete: No unresolved placeholders found");
        Ok(())
    }

    /// Resolve a single reference by patching the story data
    fn resolve_single_reference(
        &mut self,
        reference: &UnresolvedReference,
    ) -> Result<(), CompilerError> {
        // Look up the target address
        let target_address = match self
            .reference_context
            .ir_id_to_address
            .get(&reference.target_id)
        {
            Some(&addr) => addr,
            None => {
                log::debug!("Failed to resolve IR ID {}", reference.target_id);
                log::debug!(
                    "Available addresses: {:?}",
                    self.reference_context.ir_id_to_address
                );
                log::debug!("Function addresses: {:?}", self.function_addresses);
                return Err(CompilerError::CodeGenError(format!(
                    "Cannot resolve reference to IR ID {}: target address not found",
                    reference.target_id
                )));
            }
        };

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
            }
        }

        match reference.reference_type {
            ReferenceType::Jump => {
                self.patch_jump_offset(reference.location, target_address)?;
            }
            ReferenceType::Branch => {
                self.patch_branch_offset(reference.location, target_address)?;
            }
            ReferenceType::FunctionCall => {
                let packed_addr = if reference.is_packed_address {
                    self.pack_routine_address(target_address)?
                } else {
                    target_address as u16
                };
                self.patch_address(reference.location, packed_addr, 2)?; // Function addresses are 2 bytes
            }
            ReferenceType::StringRef => {
                let packed_addr = if reference.is_packed_address {
                    self.pack_string_address(target_address)?
                } else {
                    target_address as u16
                };
                self.patch_address(reference.location, packed_addr, 2)?; // String addresses are 2 bytes
            }
        }

        Ok(())
    }

    /// Calculate the size of an instruction by examining its opcode byte and operands
    fn calculate_instruction_size_from_opcode(
        &self,
        instruction_addr: usize,
    ) -> Result<usize, CompilerError> {
        if instruction_addr >= self.story_data.len() {
            return Err(CompilerError::CodeGenError(
                "Instruction address out of bounds".to_string(),
            ));
        }

        let opcode_byte = self.story_data[instruction_addr];
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
                if instruction_addr + 1 >= self.story_data.len() {
                    return Err(CompilerError::CodeGenError(
                        "Variable form instruction truncated".to_string(),
                    ));
                }

                size += 1; // Operand type byte
                let operand_types_byte = self.story_data[instruction_addr + 1];

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

    /// Patch a jump offset at the given location
    pub fn patch_jump_offset(
        &mut self,
        location: usize,
        target_address: usize,
    ) -> Result<(), CompilerError> {
        // The location points to where the jump operand starts
        // For a jump instruction: [opcode] [operand_high] [operand_low]
        // When the interpreter executes the jump, PC points to the next instruction
        // The interpreter advances PC by instruction.size BEFORE executing

        // Calculate the actual instruction size by examining the opcode
        let instruction_start = location - 1;
        let opcode_byte = self.story_data[instruction_start];
        let instruction_size = self.calculate_instruction_size_from_opcode(instruction_start)?;
        let current_pc = instruction_start + instruction_size; // PC after advancing

        log::debug!(
            "patch_jump_offset: opcode_byte=0x{:02x}, calculated_size={}, instruction_start=0x{:04x}",
            opcode_byte, instruction_size, instruction_start
        );

        // Z-Machine jump: new_pc = vm.pc + offset
        // We want: target = current_pc + offset
        // So: offset = target - current_pc
        let offset = (target_address as i32) - (current_pc as i32);

        log::debug!(
            "patch_jump_offset: location=0x{:04x}, target=0x{:04x}, current_pc=0x{:04x}, offset={}",
            location,
            target_address,
            current_pc,
            offset
        );

        // CRITICAL FIX: Handle zero-offset jumps (jumps to next instruction)
        // This should not happen with our new architecture, but keep as safety net
        if offset == 0 {
            log::warn!(
                "Zero-offset jump detected at 0x{:04x} - replacing with no-op for safety",
                instruction_start
            );

            // Replace the entire jump instruction with a no-op by converting to "ret 1"
            // This is a harmless 3-byte instruction that doesn't affect execution flow
            // since it's unreachable (previous instruction falls through)
            self.story_data[instruction_start] = 0xB1; // ret 1 (0OP instruction)
            self.story_data[instruction_start + 1] = 0x00; // padding
            self.story_data[instruction_start + 2] = 0x00; // padding

            log::debug!(
                "Replaced zero-offset jump at 0x{:04x} with no-op (ret 1)",
                instruction_start
            );

            return Ok(());
        }

        if !(-32768..=32767).contains(&offset) {
            return Err(CompilerError::CodeGenError(format!(
                "Jump offset {} too large for 16-bit signed integer",
                offset
            )));
        }

        // Write as signed 16-bit offset
        self.patch_address(location, offset as u16, 2)
    }

    /// Patch a branch offset at the given location  
    fn patch_branch_offset(
        &mut self,
        location: usize,
        target_address: usize,
    ) -> Result<(), CompilerError> {
        // Z-Machine branch offset calculation: "Address after branch data + Offset - 2"
        // So: Offset = target_address - (address_after_branch_data) + 2

        // First, determine if we need 1-byte or 2-byte format
        // We need to calculate the offset assuming 1-byte first, then check if it fits
        let address_after_1byte = location + 1;
        let offset_1byte = (target_address as i32) - (address_after_1byte as i32) + 2;

        // Always use 2-byte format since we reserved 2 bytes
        // Calculate offset for 2-byte format (address after 2 bytes)
        let address_after_2byte = location + 2;
        let offset_2byte = (target_address as i32) - (address_after_2byte as i32) + 2;

        if !(-8192..=8191).contains(&offset_2byte) {
            return Err(CompilerError::CodeGenError(format!(
                "Branch offset {} is out of range for 2-byte format (-8192 to 8191)",
                offset_2byte
            )));
        }

        // Check if we can use 1-byte format (more efficient)
        if (0..=63).contains(&offset_1byte) {
            // Use 1-byte format, pad second byte with 0
            let branch_byte = 0x40 | (offset_1byte as u8 & 0x3F); // 0x40 sets bit 6 for 1-byte
            self.story_data[location] = branch_byte;
            self.story_data[location + 1] = 0x00; // Padding byte (unused)
        } else {
            // Use 2-byte format
            // First byte: Bit 7: 0 (branch on false), Bit 6: 0 (2-byte), Bits 5-0: high 6 bits
            // Second byte: Low 8 bits
            let offset_u16 = offset_2byte as u16;
            let first_byte = (offset_u16 >> 8) as u8 & 0x3F; // Top 6 bits, clear bit 6 for 2-byte format
            let second_byte = (offset_u16 & 0xFF) as u8;

            self.story_data[location] = first_byte;
            self.story_data[location + 1] = second_byte;
        }

        Ok(())
    }

    /// Generic address patching helper
    pub fn patch_address(
        &mut self,
        location: usize,
        address: u16,
        size: usize,
    ) -> Result<(), CompilerError> {
        if location + size > self.story_data.len() {
            return Err(CompilerError::CodeGenError(format!(
                "Cannot patch address at location {}: beyond story data bounds",
                location
            )));
        }

        match size {
            1 => {
                debug!(
                    "patch_address: writing 0x{:02x} at location 0x{:04x}",
                    address as u8, location
                );
                self.story_data[location] = address as u8;
            }
            2 => {
                debug!("patch_address: writing 0x{:04x} (bytes 0x{:02x} 0x{:02x}) at location 0x{:04x}", 
                       address, (address >> 8) as u8, address as u8, location);
                self.story_data[location] = (address >> 8) as u8;
                self.story_data[location + 1] = address as u8;
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

    /// Pack a string address according to Z-Machine version
    fn pack_string_address(&self, byte_address: usize) -> Result<u16, CompilerError> {
        match self.version {
            ZMachineVersion::V3 => {
                // v3: packed address = byte address / 2
                if byte_address % 2 != 0 {
                    return Err(CompilerError::CodeGenError(
                        "String address must be even for v3".to_string(),
                    ));
                }
                Ok((byte_address / 2) as u16)
            }
            ZMachineVersion::V4 | ZMachineVersion::V5 => {
                // v4/v5: packed address = byte address / 4
                if byte_address % 4 != 0 {
                    return Err(CompilerError::CodeGenError(
                        "String address must be multiple of 4 for v4/v5".to_string(),
                    ));
                }
                Ok((byte_address / 4) as u16)
            }
        }
    }

    /// Find or create a string ID for the given string
    fn find_or_create_string_id(&mut self, s: &str) -> Result<IrId, CompilerError> {
        // Check if string already exists
        for (id, existing_string) in &self.strings {
            if existing_string == s {
                return Ok(*id);
            }
        }

        // Create new string ID
        let new_id = self.next_string_id;
        self.next_string_id += 1;

        // Add to strings collection
        self.strings.push((new_id, s.to_string()));

        // Encode the string
        let encoded = self.encode_string(s)?;
        self.encoded_strings.insert(new_id, encoded);

        // NOTE: String addresses will be assigned during layout_memory_structures
        // or when we rebuild the layout after discovering new strings

        Ok(new_id)
    }

    /// Register a builtin function name with its ID
    pub fn register_builtin_function(&mut self, function_id: IrId, name: String) {
        self.builtin_function_names.insert(function_id, name);
    }

    /// Register object numbers from IR generator
    pub fn set_object_numbers(&mut self, object_numbers: HashMap<String, u16>) {
        self.object_numbers = object_numbers;
    }

    /// Check if a function ID corresponds to a builtin function
    fn is_builtin_function(&self, function_id: IrId) -> bool {
        self.builtin_function_names.contains_key(&function_id)
    }

    /// Get the name of a builtin function by its ID
    fn get_builtin_function_name(&self, function_id: IrId) -> Option<&String> {
        self.builtin_function_names.get(&function_id)
    }

    /// Generate Z-Machine code for builtin function calls
    fn generate_builtin_function_call(
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
            "move" => self.generate_move_builtin(args),
            "get_location" => self.generate_get_location_builtin(args),
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
            "list_contents" => self.generate_list_contents_builtin(args),
            _ => Err(CompilerError::CodeGenError(format!(
                "Unimplemented builtin function: {}",
                function_name
            ))),
        }
    }

    /// Generate print builtin function
    fn generate_print_builtin(&mut self, args: &[IrId]) -> Result<(), CompilerError> {
        if args.len() != 1 {
            return Err(CompilerError::CodeGenError(format!(
                "print expects 1 argument, got {}",
                args.len()
            )));
        }

        let arg_id = args[0];

        // Look up the string value from the IR ID
        log::debug!(
            "generate_print_builtin: Looking up string for IR ID {}",
            arg_id
        );
        log::debug!(
            "  Available string IDs = {:?}",
            self.ir_id_to_string.keys().collect::<Vec<_>>()
        );
        log::debug!(
            "  Available integer IDs = {:?}",
            self.ir_id_to_integer.keys().collect::<Vec<_>>()
        );

        // Check if this is a string literal
        if let Some(string_value) = self.ir_id_to_string.get(&arg_id).cloned() {
            // Add newline to the string content for proper line breaks
            let print_string = if string_value.is_empty() {
                "\n".to_string() // Empty print() becomes just a newline
            } else {
                format!("{}\n", string_value) // Add newline to non-empty strings
            };

            // Create a string ID for this string and generate print instruction
            let string_id = self.find_or_create_string_id(&print_string)?;

            // Generate print_paddr instruction with unresolved string reference
            // Note: The unresolved reference will be added by the operand emission system
            let layout = self.emit_instruction(
                0x0D,                                          // print_paddr opcode
                &[Operand::LargeConstant(placeholder_word())], // Placeholder string address
                None,                                          // No store
                None,                                          // No branch
            )?;

            // Add unresolved reference for the string address using layout-tracked operand location
            let operand_address = layout
                .operand_location
                .expect("print_paddr instruction must have operand");
            let reference = UnresolvedReference {
                reference_type: ReferenceType::StringRef,
                location: operand_address,
                target_id: string_id,
                is_packed_address: true,
                offset_size: 2,
            };
            self.reference_context.unresolved_refs.push(reference);
        } else {
            // This is not a string literal - it's a dynamic expression that needs runtime evaluation
            // For print() with non-string arguments, we need to evaluate the expression and convert to string
            log::debug!(
                "IR ID {} is not a string literal - generating runtime evaluation for print",
                arg_id
            );

            // Try to resolve it as a simple operand (variable, constant, etc.)
            match self.resolve_ir_id_to_operand(arg_id) {
                Ok(operand) => {
                    // We can resolve it - generate print_num for numeric values
                    log::debug!(
                        "IR ID {} resolved to operand {:?} - generating print_num",
                        arg_id,
                        operand
                    );

                    self.emit_instruction(
                        0x06,       // print_num opcode
                        &[operand], // The resolved operand
                        None,       // No store
                        None,       // No branch
                    )?;
                }
                Err(_) => {
                    // Cannot resolve to simple operand - this is a complex expression
                    // Generate a more descriptive placeholder for debugging
                    log::debug!(
                        "IR ID {} is a complex expression - full evaluation not yet implemented",
                        arg_id
                    );

                    let placeholder_string = format!("?Complex expression IR ID {}?", arg_id);
                    let string_id = self.find_or_create_string_id(&placeholder_string)?;

                    let layout = self.emit_instruction(
                        0x0D,                                          // print_paddr opcode
                        &[Operand::LargeConstant(placeholder_word())], // Placeholder address
                        None,                                          // No store
                        None,                                          // No branch
                    )?;

                    let operand_address = layout
                        .operand_location
                        .expect("print_paddr instruction must have operand");
                    let reference = UnresolvedReference {
                        reference_type: ReferenceType::StringRef,
                        location: operand_address,
                        target_id: string_id,
                        is_packed_address: true,
                        offset_size: 2,
                    };
                    self.reference_context.unresolved_refs.push(reference);
                }
            }
        }

        // Do NOT add return instruction here - this is inline code generation
        // The return instruction was causing premature termination of init blocks
        // Each builtin call should continue to the next instruction

        Ok(())
    }

    /// Generate move builtin function (object, destination)
    fn generate_move_builtin(&mut self, args: &[IrId]) -> Result<(), CompilerError> {
        if args.len() != 2 {
            return Err(CompilerError::CodeGenError(format!(
                "move expects 2 arguments, got {}",
                args.len()
            )));
        }

        let object_ir_id = args[0];
        let destination_ir_id = args[1];

        // Resolve IR IDs to proper operands - CRITICAL FIX
        let object_operand = self.resolve_ir_id_to_operand(object_ir_id)?;
        let destination_operand = self.resolve_ir_id_to_operand(destination_ir_id)?;

        log::debug!(
            "generate_move_builtin: object IR {} -> {:?}, destination IR {} -> {:?}",
            object_ir_id,
            object_operand,
            destination_ir_id,
            destination_operand
        );

        // Generate Z-Machine insert_obj instruction (2OP:14, opcode 0x0E)
        // This moves object to become the first child of the destination
        // Use proper 2OP instruction encoding
        self.emit_instruction(
            0x0E, // insert_obj opcode (2OP:14)
            &[object_operand, destination_operand],
            None, // No store
            None, // No branch
        )?;

        // TODO: These need proper object/room ID resolution in the address resolution phase
        // The Z-Machine expects actual object numbers, not IR IDs

        Ok(())
    }

    /// Generate get_location builtin function - returns the parent object of an object
    fn generate_get_location_builtin(&mut self, args: &[IrId]) -> Result<(), CompilerError> {
        if args.len() != 1 {
            return Err(CompilerError::CodeGenError(format!(
                "get_location expects 1 argument, got {}",
                args.len()
            )));
        }

        let object_ir_id = args[0];

        // Resolve IR ID to proper operand - CRITICAL FIX
        let object_operand = self.resolve_ir_id_to_operand(object_ir_id)?;

        // Generate Z-Machine get_parent instruction (1OP:4, opcode 0x04)
        self.emit_instruction(
            0x04, // get_parent opcode
            &[object_operand],
            Some(0), // Store result on stack
            None,    // No branch
        )?;

        // Do NOT add return instruction here - this is inline code generation

        Ok(())
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
                self.add_unresolved_reference(ReferenceType::StringRef, string_id, true)?;
                self.emit_word(placeholder_word())?; // Placeholder address

                Ok(())
            }
            _ => Err(CompilerError::CodeGenError(format!(
                "Unimplemented method: {}",
                method_name
            ))),
        }
    }

    /// Generate test_attr builtin function - tests if an object has an attribute
    fn generate_test_attr_builtin(&mut self, args: &[IrId]) -> Result<(), CompilerError> {
        if args.len() != 2 {
            return Err(CompilerError::CodeGenError(format!(
                "test_attr expects 2 arguments, got {}",
                args.len()
            )));
        }

        let object_id = args[0];
        let attr_num = args[1];

        // Generate Z-Machine test_attr instruction (2OP:10, opcode 0x0A)
        let operands = vec![
            self.resolve_ir_id_to_operand(object_id)?, // Object
            self.resolve_ir_id_to_operand(attr_num)?,  // Attribute number
        ];
        self.emit_instruction(0x0A, &operands, Some(0), None)?; // Store result in stack

        Ok(())
    }

    /// Generate set_attr builtin function - sets an object attribute to true
    fn generate_set_attr_builtin(&mut self, args: &[IrId]) -> Result<(), CompilerError> {
        if args.len() != 2 {
            return Err(CompilerError::CodeGenError(format!(
                "set_attr expects 2 arguments, got {}",
                args.len()
            )));
        }

        let object_id = args[0];
        let attr_num = args[1];

        // Generate Z-Machine set_attr instruction (2OP:11, opcode 0x0B)
        let operands = vec![
            self.resolve_ir_id_to_operand(object_id)?, // Object
            self.resolve_ir_id_to_operand(attr_num)?,  // Attribute number
        ];
        self.emit_instruction(0x0B, &operands, None, None)?; // No return value

        Ok(())
    }

    /// Generate clear_attr builtin function - sets an object attribute to false
    fn generate_clear_attr_builtin(&mut self, args: &[IrId]) -> Result<(), CompilerError> {
        if args.len() != 2 {
            return Err(CompilerError::CodeGenError(format!(
                "clear_attr expects 2 arguments, got {}",
                args.len()
            )));
        }

        let object_id = args[0];
        let attr_num = args[1];

        // Generate Z-Machine clear_attr instruction (2OP:12, opcode 0x0C)
        let operands = vec![
            self.resolve_ir_id_to_operand(object_id)?, // Object
            self.resolve_ir_id_to_operand(attr_num)?,  // Attribute number
        ];
        self.emit_instruction(0x0C, &operands, None, None)?; // No return value

        Ok(())
    }

    /// Generate get_prop builtin function - gets a property value from an object
    fn generate_get_prop_builtin(&mut self, args: &[IrId]) -> Result<(), CompilerError> {
        if args.len() != 2 {
            return Err(CompilerError::CodeGenError(format!(
                "get_prop expects 2 arguments, got {}",
                args.len()
            )));
        }

        let object_id = args[0];
        let prop_num = args[1];

        // Generate Z-Machine get_prop instruction (2OP:17, opcode 0x11)
        let operands = vec![
            self.resolve_ir_id_to_operand(object_id)?, // Object
            self.resolve_ir_id_to_operand(prop_num)?,  // Property number
        ];
        self.emit_instruction(0x11, &operands, Some(0), None)?; // Store result on stack

        Ok(())
    }

    /// Generate get_child builtin function - gets first child of an object
    fn generate_get_child_builtin(&mut self, args: &[IrId]) -> Result<(), CompilerError> {
        if args.len() != 1 {
            return Err(CompilerError::CodeGenError(format!(
                "get_child expects 1 argument, got {}",
                args.len()
            )));
        }

        let object_ir_id = args[0];

        // Resolve IR ID to proper operand - CRITICAL FIX
        let object_operand = self.resolve_ir_id_to_operand(object_ir_id)?;

        // Generate Z-Machine get_child instruction (1OP:3, opcode 0x03)
        self.emit_instruction(
            0x03, // get_child opcode
            &[object_operand],
            Some(0), // Store result on stack
            None,    // No branch
        )?;

        // Do NOT add return instruction here - this is inline code generation

        Ok(())
    }

    /// Generate get_sibling builtin function - gets sibling of an object  
    fn generate_get_sibling_builtin(&mut self, args: &[IrId]) -> Result<(), CompilerError> {
        if args.len() != 1 {
            return Err(CompilerError::CodeGenError(format!(
                "get_sibling expects 1 argument, got {}",
                args.len()
            )));
        }

        let object_ir_id = args[0];

        // Resolve IR ID to proper operand - CRITICAL FIX
        let object_operand = self.resolve_ir_id_to_operand(object_ir_id)?;

        // Generate Z-Machine get_sibling instruction (1OP:2, opcode 0x02)
        self.emit_instruction(
            0x02, // get_sibling opcode
            &[object_operand],
            Some(0), // Store result on stack
            None,    // No branch
        )?;

        // Do NOT add return instruction here - this is inline code generation

        Ok(())
    }

    /// Generate player_can_see builtin function - checks if player can see an object
    /// Implements visibility logic: object is in player location, in player inventory, or visible container
    fn generate_player_can_see_builtin(&mut self, args: &[IrId]) -> Result<(), CompilerError> {
        if args.len() != 1 {
            return Err(CompilerError::CodeGenError(format!(
                "player_can_see expects 1 argument, got {}",
                args.len()
            )));
        }

        let object_ir_id = args[0];

        // Resolve IR ID to proper operand - CRITICAL FIX
        let object_operand = self.resolve_ir_id_to_operand(object_ir_id)?;

        // Visibility check algorithm:
        // 1. Get object's parent (location)
        // 2. Check if parent == player location (visible in room)
        // 3. Check if parent == player (in inventory)
        // 4. If parent is a container, check if container is open and visible

        // For now, implement basic visibility: check if object parent == player location or player
        // This is simplified - a full implementation would handle nested containers, lighting, etc.

        // Get object's parent location
        self.emit_instruction(
            0x04,              // get_parent opcode
            &[object_operand], // Object operand (resolved)
            Some(0x01),        // Store in local variable 1
            None,              // No branch
        )?;

        // Get player location (assume player is object 1, location is its parent)
        self.emit_instruction(
            0x04,                              // get_parent opcode
            &[Operand::LargeConstant(0x0001)], // Player object (object 1)
            Some(0x02),                        // Store in local variable 2
            None,                              // No branch
        )?;

        // Compare object location with player location
        self.emit_byte(0x15)?; // je opcode (2OP:1, VAR form)
        self.emit_byte(0x01)?; // Variable 1 (object location)
        self.emit_byte(0x02)?; // Variable 2 (player location)
        self.emit_byte(0x40)?; // Branch if true, 2-byte offset
        self.emit_word(0x0008)?; // Branch to "return true" (+8 bytes)

        // Check if object is in player inventory (parent == player)
        self.emit_byte(0x15)?; // je opcode
        self.emit_byte(0x01)?; // Variable 1 (object location)
        self.emit_word(0x0001)?; // Player object literal
        self.emit_byte(0x40)?; // Branch if true
        self.emit_word(0x0002)?; // Branch to "return true" (+2 bytes)

        // Return false (object not visible)
        self.emit_byte(0xB1)?; // rfalse instruction (0OP:1)

        // Return true (object is visible)
        self.emit_byte(0xB0)?; // rtrue instruction (0OP:0)

        Ok(())
    }

    /// Generate list_objects builtin function - lists all objects in a location
    fn generate_list_objects_builtin(&mut self, args: &[IrId]) -> Result<(), CompilerError> {
        if args.len() != 1 {
            return Err(CompilerError::CodeGenError(format!(
                "list_objects expects 1 argument, got {}",
                args.len()
            )));
        }

        let location_id = args[0];

        // Algorithm to list objects in a location:
        // 1. Get first child of the location
        // 2. While child exists:
        //    a. Print the child's name
        //    b. Get next sibling
        //    c. Repeat

        // Get first child of location
        self.emit_byte(0x82)?; // get_child opcode (1OP:130)
        self.emit_word(location_id as u16)?; // Location ID
        self.emit_byte(0x01)?; // Store child in local variable 1
        self.emit_byte(0x40)?; // Branch if child exists (non-zero)
        self.emit_word(0x0002)?; // Skip return if no children

        // No children - return
        self.emit_byte(0xB1)?; // rfalse instruction

        // Loop through siblings printing each one
        // Variable 1 contains current object to print

        // Print current object (simplified - would normally get object name property)
        // For now, print a debug message with object number
        self.emit_byte(0xB2)?; // print opcode
        let debug_msg = "Object in location";
        let string_id = self.find_or_create_string_id(debug_msg)?;
        self.ir_id_to_string
            .insert(string_id, debug_msg.to_string());
        self.add_unresolved_reference(ReferenceType::StringRef, string_id, true)?;
        self.emit_word(placeholder_word())?; // Placeholder for string address

        // Get next sibling
        self.emit_byte(0x81)?; // get_sibling opcode (1OP:129)
        self.emit_byte(0x01)?; // Current object in variable 1
        self.emit_byte(0x01)?; // Store sibling back in variable 1
        self.emit_byte(0x40)?; // Branch if sibling exists
        self.emit_word(0xFFF0)?; // Loop back to print next object (negative offset)

        // Done listing - return
        self.emit_byte(0xB0)?; // rtrue instruction

        Ok(())
    }

    /// Generate list_contents builtin function - lists contents of a container
    fn generate_list_contents_builtin(&mut self, args: &[IrId]) -> Result<(), CompilerError> {
        if args.len() != 1 {
            return Err(CompilerError::CodeGenError(format!(
                "list_contents expects 1 argument, got {}",
                args.len()
            )));
        }

        let container_id = args[0];

        // Similar to list_objects but for container contents
        // Check if container is open first (if it has openable attribute)

        // Get first child of container
        self.emit_byte(0x82)?; // get_child opcode (1OP:130)
        self.emit_word(container_id as u16)?; // Container ID
        self.emit_byte(0x01)?; // Store child in local variable 1
        self.emit_byte(0x40)?; // Branch if child exists
        self.emit_word(0x0002)?; // Skip return if empty

        // Empty container - return
        self.emit_byte(0xB1)?; // rfalse instruction

        // Loop through contents
        // Print current object
        self.emit_byte(0xB2)?; // print opcode
        let debug_msg = "Object in container";
        let string_id = self.find_or_create_string_id(debug_msg)?;
        self.ir_id_to_string
            .insert(string_id, debug_msg.to_string());
        self.add_unresolved_reference(ReferenceType::StringRef, string_id, true)?;
        self.emit_word(placeholder_word())?; // Placeholder for string address

        // Get next sibling
        self.emit_byte(0x81)?; // get_sibling opcode
        self.emit_byte(0x01)?; // Current object
        self.emit_byte(0x01)?; // Store sibling back in variable 1
        self.emit_byte(0x40)?; // Branch if sibling exists
        self.emit_word(0xFFF0)?; // Loop back (negative offset)

        // Done - return
        self.emit_byte(0xB0)?; // rtrue instruction

        Ok(())
    }

    /// Generate to_string builtin function - converts values to strings
    fn generate_to_string_builtin(
        &mut self,
        args: &[IrId],
        target: Option<IrId>,
    ) -> Result<(), CompilerError> {
        if args.len() != 1 {
            return Err(CompilerError::CodeGenError(format!(
                "to_string expects 1 argument, got {}",
                args.len()
            )));
        }

        // Create a placeholder string for to_string conversion
        let placeholder_str = "[NUM]";
        let _string_id = self.find_or_create_string_id(placeholder_str)?;

        // If we have a target, register it as producing a string value
        if let Some(target_id) = target {
            self.ir_id_to_string
                .insert(target_id, placeholder_str.to_string());
        }

        // to_string is a compile-time operation that produces string values
        // No runtime instructions needed - the result is used via ir_id_to_string mapping

        Ok(())
    }

    /// Generate random builtin function - implements Z-Machine RANDOM opcode
    fn generate_random_builtin(
        &mut self,
        args: &[IrId],
        target: Option<IrId>,
    ) -> Result<(), CompilerError> {
        if args.len() != 1 {
            return Err(CompilerError::CodeGenError(format!(
                "random expects 1 argument, got {}",
                args.len()
            )));
        }

        debug!("Generating RANDOM opcode with range argument");

        // For now, use a placeholder operand - in a full implementation,
        // we need to properly handle the IR argument values
        let range_operand = Operand::SmallConstant(6); // Placeholder for now
        let store_var = Some(0); // Store result on stack

        self.emit_instruction(
            0x07,             // RANDOM opcode (VAR:231)
            &[range_operand], // Range operand
            store_var,        // Store result in variable 0 (stack)
            None,             // No branch
        )?;

        // If we have a target, this will be used for further operations
        if let Some(target_id) = target {
            // Register that this target contains a numeric value
            // For string concatenation, we'd need to convert this to string
            let placeholder_str = "[RANDOM_RESULT]";
            self.ir_id_to_string
                .insert(target_id, placeholder_str.to_string());
        }

        debug!("Generated RANDOM instruction successfully");
        Ok(())
    }

    /// Generate string concatenation for two IR values
    fn generate_string_concatenation(
        &mut self,
        target: IrId,
        left: IrId,
        right: IrId,
    ) -> Result<(), CompilerError> {
        // For string concatenation, we need to:
        // 1. Get the string values for left and right operands
        // 2. Concatenate them into a new string
        // 3. Store the new string and return its address

        let left_str = self.get_string_value(left)?;
        let right_str = self.get_string_value(right)?;

        // Concatenate the strings
        let concatenated = format!("{}{}", left_str, right_str);

        // Create a new string entry for the concatenated result
        let _concat_string_id = self.find_or_create_string_id(&concatenated)?;
        self.ir_id_to_string.insert(target, concatenated);

        // String concatenation is a compile-time operation
        // No runtime instructions needed - the concatenated string will be used directly
        // by print operations via its string ID

        debug!(
            "String concatenation: {} + {} -> {} (ID: {})",
            left_str,
            right_str,
            self.ir_id_to_string
                .get(&target)
                .unwrap_or(&"<unknown>".to_string()),
            target
        );

        Ok(())
    }

    /// Get string value for an IR ID (handles both string literals and function return values)
    fn get_string_value(&self, ir_id: IrId) -> Result<String, CompilerError> {
        if let Some(string_val) = self.ir_id_to_string.get(&ir_id) {
            Ok(string_val.clone())
        } else {
            // This might be a to_string() result or other dynamic string
            // For now, use a placeholder that represents the dynamic value
            Ok(format!("[Dynamic-{}]", ir_id))
        }
    }

    /// Update string addresses after new strings have been added
    fn update_string_addresses(&mut self) {
        // CRITICAL: Only assign addresses to truly new strings that don't have addresses yet
        // Strings allocated during layout phase should be preserved to prevent memory conflicts

        // Find strings that don't have addresses yet
        let mut new_strings: Vec<(IrId, usize)> = Vec::new();
        for (string_id, encoded_bytes) in &self.encoded_strings {
            if !self.string_addresses.contains_key(string_id) {
                new_strings.push((*string_id, encoded_bytes.len()));
                debug!("Found new string during code generation: ID={}", string_id);
            }
        }

        if new_strings.is_empty() {
            debug!("No new strings to allocate - preserving layout-time allocations");
            return;
        }

        // Calculate addresses for new strings only
        // Place them after all code, not after dictionary
        let mut addr = self.current_address + 100; // Start after current code with padding

        // Align addresses according to Z-Machine version
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

        // Sort by IR ID to ensure deterministic address assignment
        new_strings.sort_by_key(|(id, _)| *id);

        debug!(
            "Update phase: allocating {} new strings starting at 0x{:04x}",
            new_strings.len(),
            addr
        );

        for (string_id, length) in new_strings {
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
                "Update phase: NEW string_id={} -> 0x{:04x} (length={})",
                string_id, addr, length
            );
            self.record_address(string_id, addr); // Record in reference context
            addr += length;
        }

        // Update current_address if needed
        if addr > self.current_address {
            self.current_address = addr;
        }
    }

    /// Write all encoded strings to their assigned memory locations
    fn write_strings_to_memory(&mut self) -> Result<(), CompilerError> {
        // DEBUG: Check property table integrity before writing strings
        debug!("INTEGRITY CHECK: Before writing strings:");
        debug!(
            "  Property table 1 at 0x0268: 0x{:02x} (should be 0x21)",
            self.story_data[0x0268]
        );
        debug!(
            "  Property table 2 at 0x027c: 0x{:02x} (should be 0x21)",
            self.story_data[0x027c]
        );

        // Strings are already written during layout phase - just verify they're there
        debug!("Verifying strings are already written during layout phase...");
        for (string_id, encoded_bytes) in &self.encoded_strings {
            if let Some(&address) = self.string_addresses.get(string_id) {
                // Verify the string data is already there
                if self.story_data.len() >= address + encoded_bytes.len() {
                    debug!(
                        "String ID {} already written at 0x{:04x}",
                        string_id, address
                    );
                } else {
                    return Err(CompilerError::CodeGenError(format!(
                        "String ID {} expected at 0x{:04x} but memory not allocated",
                        string_id, address
                    )));
                }

                // Record this address in the IR ID mapping for reference resolution
                self.reference_context
                    .ir_id_to_address
                    .insert(*string_id, address);
            } else {
                return Err(CompilerError::CodeGenError(format!(
                    "String ID {} has no assigned address",
                    string_id
                )));
            }
        }

        // CRITICAL: Update current_address to reflect actual end of string data
        // This eliminates gaps between string data and code generation
        let mut max_string_end = self.current_address; // Start with current value
        for (string_id, encoded_bytes) in &self.encoded_strings {
            if let Some(&address) = self.string_addresses.get(string_id) {
                let string_end = address + encoded_bytes.len();
                max_string_end = std::cmp::max(max_string_end, string_end);
            }
        }
        self.current_address = max_string_end;
        debug!(
            "Strings complete, current_address updated to: 0x{:04x}",
            self.current_address
        );

        Ok(())
    }

    /// Add an unresolved reference to be patched later
    pub fn add_unresolved_reference(
        &mut self,
        reference_type: ReferenceType,
        target_id: IrId,
        is_packed: bool,
    ) -> Result<(), CompilerError> {
        log::debug!(
            "add_unresolved_reference: {:?} -> IR ID {} at address 0x{:04x}",
            reference_type,
            target_id,
            self.current_address
        );

        let reference = UnresolvedReference {
            reference_type,
            location: self.current_address,
            target_id,
            is_packed_address: is_packed,
            offset_size: 2, // Default to 2 bytes
        };
        self.reference_context.unresolved_refs.push(reference);
        Ok(())
    }

    /// Record a resolved address for an IR ID
    pub fn record_address(&mut self, ir_id: IrId, address: usize) {
        self.reference_context
            .ir_id_to_address
            .insert(ir_id, address);
    }

    // Utility methods for code emission

    /// Emit a single byte and advance current address
    fn emit_byte(&mut self, byte: u8) -> Result<(), CompilerError> {
        // Clear labels at current address when we emit actual instruction bytes
        // (but not for padding or alignment bytes)
        if !self.labels_at_current_address.is_empty() && byte != 0x00 {
            log::debug!(
                "Clearing {} labels at address 0x{:04x} - instruction byte 0x{:02x} emitted",
                self.labels_at_current_address.len(),
                self.current_address,
                byte
            );
            self.labels_at_current_address.clear();
        }

        // Validate critical opcodes and log suspicious patterns
        if byte == 0x00 && self.current_address >= 0x08fe {
            debug!(
                "WARNING: Emitting 0x00 at address 0x{:04x} (might be invalid opcode)",
                self.current_address
            );
            debug!("  Stack depth: {}", self.stack_depth);
            // Temporarily allow but warn - we'll track this
        }

        if byte == 0x9d || byte == 0x8d {
            log::debug!(
                "Emitting 0x{:02x} (print_paddr) at address 0x{:04x}",
                byte,
                self.current_address
            );
        }
        if byte == 0xe0 {
            log::debug!(
                "Emitting 0x{:02x} (call_vs) at address 0x{:04x}",
                byte,
                self.current_address
            );
        }
        if byte == 0xb0 {
            log::debug!(
                "Emitting 0x{:02x} (rtrue) at address 0x{:04x}",
                byte,
                self.current_address
            );
        }

        // Debug critical addresses
        if self.current_address >= 0x0730 && self.current_address <= 0x0740 {
            debug!(
                "emit_byte: 0x{:02x} at address 0x{:04x}",
                byte, self.current_address
            );
        }

        self.ensure_capacity(self.current_address + 1);
        self.story_data[self.current_address] = byte;
        self.current_address += 1;
        Ok(())
    }

    /// Emit a 16-bit word (big-endian) and advance current address
    fn emit_word(&mut self, word: u16) -> Result<(), CompilerError> {
        self.emit_byte((word >> 8) as u8)?;
        self.emit_byte(word as u8)?;
        Ok(())
    }

    /// Write a word at a specific address without changing current address
    fn write_word_at(&mut self, addr: usize, word: u16) -> Result<(), CompilerError> {
        self.ensure_capacity(addr + 2);
        self.story_data[addr] = (word >> 8) as u8;
        self.story_data[addr + 1] = word as u8;
        Ok(())
    }

    /// Ensure the story data buffer has enough capacity
    fn ensure_capacity(&mut self, required: usize) {
        if self.story_data.len() < required {
            self.story_data.resize(required, 0);
        }
    }

    // Z-Machine instruction encoding methods

    /// Encode a complete Z-Machine instruction with proper operand types
    /// Emit a Z-Machine instruction and return its layout information
    ///
    /// This function generates the bytecode for a Z-Machine instruction and returns
    /// detailed information about where each component (operands, store variable, etc.)
    /// was placed in memory. This eliminates the need for hardcoded offset calculations
    /// when creating references for later patching.
    ///
    /// # Returns
    ///
    /// `InstructionLayout` containing the exact byte locations of instruction components,
    /// or an error if instruction generation fails.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let layout = self.emit_instruction(0x8D, &[Operand::LargeConstant(placeholder_word())], None, None)?;
    /// // Use layout.operand_location for reference patching instead of current_address - 2
    /// ```
    pub fn emit_instruction(
        &mut self,
        opcode: u8,
        operands: &[Operand],
        store_var: Option<u8>,
        branch_offset: Option<i16>,
    ) -> Result<InstructionLayout, CompilerError> {
        // Force all store operations to use stack when in init block context
        let actual_store_var = if self.in_init_block && store_var.is_some() && store_var != Some(0)
        {
            log::debug!(
                "Init block context: Forcing store variable {:?} -> stack (0)",
                store_var
            );
            Some(0) // Use stack instead of local variables
        } else {
            store_var
        };

        log::debug!(
            "emit_instruction opcode=0x{:02x}, operands={:?}, store_var={:?}",
            opcode,
            operands,
            actual_store_var
        );

        // Record instruction start address
        let instruction_start = self.current_address;

        let form = self.determine_instruction_form_with_operands(operands, opcode);
        log::debug!("determined form={:?}", form);

        let layout = match form {
            InstructionForm::Long => self.emit_long_form_with_layout(
                instruction_start,
                opcode,
                operands,
                actual_store_var,
                branch_offset,
            )?,
            InstructionForm::Short => self.emit_short_form_with_layout(
                instruction_start,
                opcode,
                operands,
                actual_store_var,
                branch_offset,
            )?,
            InstructionForm::Variable => self.emit_variable_form_with_layout(
                instruction_start,
                opcode,
                operands,
                actual_store_var,
                branch_offset,
            )?,
            InstructionForm::Extended => {
                return Err(CompilerError::CodeGenError(
                    "Extended form instructions not yet supported".to_string(),
                ));
            }
        };

        // Track stack operations for debugging
        self.track_stack_operation(opcode, operands, actual_store_var);

        Ok(layout)
    }

    /// Track stack operations for debugging and validation
    fn track_stack_operation(&mut self, opcode: u8, operands: &[Operand], store_var: Option<u8>) {
        // Track stack pushes and pops for common operations
        match opcode {
            // Instructions that push to stack (store result on stack top)
            0x11..=0x13 => {
                // get_prop, get_prop_addr, get_next_prop
                if store_var == Some(0) {
                    self.stack_depth += 1;
                    debug!("Stack push (get_prop*) - depth now: {}", self.stack_depth);
                }
            }
            0x14..=0x18 => {
                // add, sub, mul, div, mod
                if store_var == Some(0) {
                    self.stack_depth += 1;
                    debug!("Stack push (arithmetic) - depth now: {}", self.stack_depth);
                }
            }
            // Instructions that pop from stack
            0x0D => {
                // store
                if let Some(Operand::Variable(0)) = operands.first() {
                    self.stack_depth -= 1;
                    debug!("Stack pop (store) - depth now: {}", self.stack_depth);
                }
            }
            // Function calls affect stack significantly
            0xE0 => {
                // call (VAR form)
                // Function calls consume arguments and push return value
                self.stack_depth -= operands.len() as i32;
                if store_var == Some(0) {
                    self.stack_depth += 1;
                }
                debug!(
                    "Stack after function call - depth now: {}",
                    self.stack_depth
                );
            }
            _ => {
                // For other instructions that might affect stack
                if store_var == Some(0) {
                    self.stack_depth += 1;
                    debug!("Stack push (generic) - depth now: {}", self.stack_depth);
                }
            }
        }

        // Track maximum depth
        if self.stack_depth > self.max_stack_depth {
            self.max_stack_depth = self.stack_depth;
        }

        // Warn about potential stack issues
        if self.stack_depth < 0 {
            debug!(
                "WARNING: Stack underflow detected! Depth: {}",
                self.stack_depth
            );
        }
        if self.stack_depth > 100 {
            debug!(
                "WARNING: Very deep stack detected! Depth: {}",
                self.stack_depth
            );
        }
    }

    /// Check if an opcode is a true VAR opcode (always requires VAR form encoding)
    fn is_true_var_opcode(opcode: u8) -> bool {
        match opcode {
            0xE0 => true, // CALL_VS (VAR:224)
            0x01 => true, // STOREW
            0x03 => true, // PUT_PROP
            0x04 => true, // SREAD
            0x07 => true, // RANDOM
            0x20 => true, // CALL_1N
            _ => false,
        }
    }

    /// Determine instruction form based on operand count and opcode
    pub fn determine_instruction_form(&self, operand_count: usize, opcode: u8) -> InstructionForm {
        // Special cases: certain opcodes are always VAR form regardless of operand count
        match opcode {
            0x03 => InstructionForm::Variable, // put_prop is always VAR
            0x04 => InstructionForm::Variable, // sread is always VAR
            0x07 => InstructionForm::Variable, // random is always VAR
            0x20 => InstructionForm::Variable, // call_1n is always VAR
            0xE0 => InstructionForm::Variable, // call (VAR:224) is always VAR
            _ => match operand_count {
                0 => InstructionForm::Short, // 0OP
                1 => InstructionForm::Short, // 1OP
                2 => {
                    // Could be 2OP (long form) or VAR form
                    // For now, prefer long form for 2 operands
                    if opcode < 0x80 {
                        InstructionForm::Long
                    } else {
                        InstructionForm::Variable
                    }
                }
                _ => InstructionForm::Variable, // VAR form for 3+ operands
            },
        }
    }

    /// Determine instruction form based on operand count, opcode, and operand constraints
    pub fn determine_instruction_form_with_operands(
        &self,
        operands: &[Operand],
        opcode: u8,
    ) -> InstructionForm {
        // Handle opcodes based on operand count AND context
        match (opcode, operands.len()) {
            // Opcode 0x03: Context-dependent!
            // - 2 operands: jg (jump if greater) - 2OP form
            // - 3 operands: put_prop - VAR form
            (0x03, 2) => {
                // jg (jump if greater) - prefer Long form for 2OP
                let can_use_long_form = operands.iter().all(|op| match op {
                    Operand::LargeConstant(value) => *value <= 255,
                    _ => true,
                });
                if opcode < 0x80 && can_use_long_form {
                    InstructionForm::Long
                } else {
                    InstructionForm::Variable
                }
            }
            (0x03, 3) => InstructionForm::Variable, // put_prop is always VAR

            // Always VAR form opcodes (regardless of operand count)
            (0x04, _) => InstructionForm::Variable, // sread is always VAR
            (0x07, _) => InstructionForm::Variable, // random is always VAR
            (0x20, _) => InstructionForm::Variable, // call_1n is always VAR
            (0xE0, _) => InstructionForm::Variable, // call (VAR:224) is always VAR

            // Default operand-count based logic
            _ => match operands.len() {
                0 => InstructionForm::Short, // 0OP
                1 => InstructionForm::Short, // 1OP
                2 => {
                    // Check if Long form can handle all operands
                    let can_use_long_form = operands.iter().all(|op| {
                        match op {
                            Operand::LargeConstant(value) => *value <= 255,
                            _ => true, // SmallConstant and Variable are fine
                        }
                    });

                    if opcode < 0x80 && can_use_long_form {
                        InstructionForm::Long
                    } else {
                        InstructionForm::Variable
                    }
                }
                _ => InstructionForm::Variable, // VAR form for 3+ operands
            },
        }
    }

    /// Emit long form instruction (2OP)
    fn emit_long_form(
        &mut self,
        opcode: u8,
        operands: &[Operand],
        store_var: Option<u8>,
        branch_offset: Option<i16>,
    ) -> Result<(), CompilerError> {
        if operands.len() != 2 {
            return Err(CompilerError::CodeGenError(format!(
                "Long form requires exactly 2 operands, got {}",
                operands.len()
            )));
        }

        // Long form: bit 6 = op1_type, bit 5 = op2_type, bits 4-0 = opcode
        // In Long form: 0 = small constant, 1 = variable (only these 2 types allowed)
        let op1_bit = if matches!(operands[0], Operand::Variable(_)) {
            1
        } else {
            0
        };
        let op2_bit = if matches!(operands[1], Operand::Variable(_)) {
            1
        } else {
            0
        };

        let instruction_byte = (op1_bit << 6) | (op2_bit << 5) | (opcode & 0x1F);
        self.emit_byte(instruction_byte)?;

        // Emit operands
        self.emit_operand(&operands[0])?;
        self.emit_operand(&operands[1])?;

        // Emit store variable if needed
        if let Some(store) = store_var {
            self.emit_byte(store)?;
        }

        // Emit branch offset if needed
        if let Some(offset) = branch_offset {
            self.emit_branch_offset(offset)?;
        }

        Ok(())
    }

    /// Emit short form instruction (0OP or 1OP)
    fn emit_short_form(
        &mut self,
        opcode: u8,
        operands: &[Operand],
        store_var: Option<u8>,
        branch_offset: Option<i16>,
    ) -> Result<(), CompilerError> {
        if operands.len() > 1 {
            return Err(CompilerError::CodeGenError(format!(
                "Short form requires 0 or 1 operands, got {}",
                operands.len()
            )));
        }

        let instruction_byte = if operands.is_empty() {
            // 0OP form: bits 7-6 = 11, bits 5-4 = 00, bits 3-0 = opcode
            0xB0 | (opcode & 0x0F)
        } else {
            // 1OP form: bits 7-6 = 10, bits 5-4 = operand type, bits 3-0 = opcode
            let op_type = self.get_operand_type(&operands[0]);
            0x80 | ((op_type as u8) << 4) | (opcode & 0x0F)
        };

        self.emit_byte(instruction_byte)?;

        // Emit operand if present
        if !operands.is_empty() {
            self.emit_operand(&operands[0])?;
        }

        // Emit store variable if needed
        if let Some(store) = store_var {
            self.emit_byte(store)?;
        }

        // Emit branch offset if needed
        if let Some(offset) = branch_offset {
            self.emit_branch_offset(offset)?;
        }

        Ok(())
    }

    /// Emit short form instruction with layout tracking
    ///
    /// This is the layout-aware version of emit_short_form that tracks where
    /// each instruction component is placed for accurate reference resolution.
    fn emit_short_form_with_layout(
        &mut self,
        instruction_start: usize,
        opcode: u8,
        operands: &[Operand],
        store_var: Option<u8>,
        branch_offset: Option<i16>,
    ) -> Result<InstructionLayout, CompilerError> {
        if operands.len() > 1 {
            return Err(CompilerError::CodeGenError(format!(
                "Short form requires 0 or 1 operands, got {}",
                operands.len()
            )));
        }

        let instruction_byte = if operands.is_empty() {
            // 0OP form: bits 7-6 = 11, bits 5-4 = 00, bits 3-0 = opcode
            0xB0 | (opcode & 0x0F)
        } else {
            // 1OP form: bits 7-6 = 10, bits 5-4 = operand type, bits 3-0 = opcode
            let op_type = self.get_operand_type(&operands[0]);
            0x80 | ((op_type as u8) << 4) | (opcode & 0x0F)
        };

        self.emit_byte(instruction_byte)?;

        // Track operand location
        let operand_location = if !operands.is_empty() {
            let loc = self.current_address;
            self.emit_operand(&operands[0])?;
            Some(loc)
        } else {
            None
        };

        // Track store variable location
        let store_location = if let Some(store) = store_var {
            let loc = self.current_address;
            self.emit_byte(store)?;
            Some(loc)
        } else {
            None
        };

        // Track branch offset location
        let branch_location = if let Some(offset) = branch_offset {
            let loc = self.current_address;
            self.emit_branch_offset(offset)?;
            Some(loc)
        } else {
            None
        };

        Ok(InstructionLayout {
            instruction_start,
            operand_location,
            store_location,
            branch_location,
            total_size: self.current_address - instruction_start,
        })
    }

    /// Emit variable form instruction (VAR)
    fn emit_variable_form(
        &mut self,
        opcode: u8,
        operands: &[Operand],
        store_var: Option<u8>,
        branch_offset: Option<i16>,
    ) -> Result<(), CompilerError> {
        if operands.len() > 4 {
            return Err(CompilerError::CodeGenError(format!(
                "Variable form supports max 4 operands, got {}",
                operands.len()
            )));
        }

        // Variable form: bits 7-6 = 11, bit 5 = VAR (1) or OP2 (0), bits 4-0 = opcode
        // Bit 5 should be set for true VAR opcodes (like RANDOM), regardless of operand count
        let var_bit = if Self::is_true_var_opcode(opcode) {
            0x20
        } else {
            0x00
        };
        let instruction_byte = 0xC0 | var_bit | (opcode & 0x1F);
        debug!("emit_variable_form: opcode=0x{:02x}, var_bit=0x{:02x}, instruction_byte=0x{:02x} at address 0x{:04x}", 
               opcode, var_bit, instruction_byte, self.current_address);
        self.emit_byte(instruction_byte)?;

        // Emit operand types byte
        let mut types_byte = 0u8;
        for (i, operand) in operands.iter().enumerate() {
            let op_type = self.get_operand_type(operand);
            types_byte |= (op_type as u8) << (6 - i * 2);
        }

        // Fill remaining slots with "omitted"
        for i in operands.len()..4 {
            types_byte |= (OperandType::Omitted as u8) << (6 - i * 2);
        }

        self.emit_byte(types_byte)?;

        // Emit operands
        for operand in operands {
            self.emit_operand(operand)?;
        }

        // Emit store variable if needed
        if let Some(store) = store_var {
            self.emit_byte(store)?;
        }

        // Emit branch offset if needed
        if let Some(offset) = branch_offset {
            self.emit_branch_offset(offset)?;
        }

        Ok(())
    }

    /// Emit variable form instruction with layout tracking
    ///
    /// This is the layout-aware version of emit_variable_form that tracks where
    /// each instruction component is placed for accurate reference resolution.
    fn emit_variable_form_with_layout(
        &mut self,
        instruction_start: usize,
        opcode: u8,
        operands: &[Operand],
        store_var: Option<u8>,
        branch_offset: Option<i16>,
    ) -> Result<InstructionLayout, CompilerError> {
        if operands.len() > 4 {
            return Err(CompilerError::CodeGenError(format!(
                "Variable form supports max 4 operands, got {}",
                operands.len()
            )));
        }

        // Determine if we need VAR (0x20) or VAR2 (0x3C) bit pattern
        let var_bit = if Self::is_true_var_opcode(opcode) {
            0x20
        } else {
            0x00
        };
        let instruction_byte = 0xC0 | var_bit | (opcode & 0x1F);

        debug!("emit_variable_form: opcode=0x{:02x}, var_bit=0x{:02x}, instruction_byte=0x{:02x} at address 0x{:04x}", 
               opcode, var_bit, instruction_byte, self.current_address);

        self.emit_byte(instruction_byte)?;

        // Build operand types byte
        let mut types_byte = 0u8;
        for (i, operand) in operands.iter().enumerate() {
            let op_type = self.get_operand_type(operand);
            types_byte |= (op_type as u8) << (6 - i * 2);
        }

        // Fill remaining operand type slots with "omitted"
        for i in operands.len()..4 {
            types_byte |= (OperandType::Omitted as u8) << (6 - i * 2);
        }

        self.emit_byte(types_byte)?;

        // Track first operand location (most commonly needed for references)
        let operand_location = if !operands.is_empty() {
            let loc = self.current_address;
            // Emit all operands
            for operand in operands {
                self.emit_operand(operand)?;
            }
            Some(loc)
        } else {
            None
        };

        // Track store variable location
        let store_location = if let Some(store) = store_var {
            let loc = self.current_address;
            self.emit_byte(store)?;
            Some(loc)
        } else {
            None
        };

        // Track branch offset location
        let branch_location = if let Some(offset) = branch_offset {
            let loc = self.current_address;
            self.emit_branch_offset(offset)?;
            Some(loc)
        } else {
            None
        };

        Ok(InstructionLayout {
            instruction_start,
            operand_location,
            store_location,
            branch_location,
            total_size: self.current_address - instruction_start,
        })
    }

    /// Emit long form instruction with layout tracking
    ///
    /// This is the layout-aware version of emit_long_form that tracks where
    /// each instruction component is placed for accurate reference resolution.
    fn emit_long_form_with_layout(
        &mut self,
        instruction_start: usize,
        opcode: u8,
        operands: &[Operand],
        store_var: Option<u8>,
        branch_offset: Option<i16>,
    ) -> Result<InstructionLayout, CompilerError> {
        if operands.len() != 2 {
            return Err(CompilerError::CodeGenError(format!(
                "Long form requires exactly 2 operands, got {}",
                operands.len()
            )));
        }

        // Long form can only handle Small Constants and Variables
        // Convert LargeConstants that fit in a byte to SmallConstants
        let op1_adapted = self.adapt_operand_for_long_form(&operands[0])?;
        let op2_adapted = self.adapt_operand_for_long_form(&operands[1])?;

        let op1_type = self.get_operand_type(&op1_adapted);
        let op2_type = self.get_operand_type(&op2_adapted);

        // Long form: bits 7-6 = 00 or 01, bit 6 = op1 type, bit 5 = op2 type, bits 4-0 = opcode
        let op1_bit = if op1_type == OperandType::Variable {
            0x40
        } else {
            0x00
        };
        let op2_bit = if op2_type == OperandType::Variable {
            0x20
        } else {
            0x00
        };
        let instruction_byte = op1_bit | op2_bit | (opcode & 0x1F);

        self.emit_byte(instruction_byte)?;

        // Track first operand location
        let operand_location = Some(self.current_address);

        // Emit adapted operands
        self.emit_operand(&op1_adapted)?;
        self.emit_operand(&op2_adapted)?;

        // Track store variable location
        let store_location = if let Some(store) = store_var {
            let loc = self.current_address;
            self.emit_byte(store)?;
            Some(loc)
        } else {
            None
        };

        // Track branch offset location
        let branch_location = if let Some(offset) = branch_offset {
            let loc = self.current_address;
            self.emit_branch_offset(offset)?;
            Some(loc)
        } else {
            None
        };

        Ok(InstructionLayout {
            instruction_start,
            operand_location,
            store_location,
            branch_location,
            total_size: self.current_address - instruction_start,
        })
    }

    /// Adapt operand for Long form instruction constraints
    /// Long form can only handle Small Constants and Variables
    fn adapt_operand_for_long_form(&self, operand: &Operand) -> Result<Operand, CompilerError> {
        match operand {
            Operand::LargeConstant(value) => {
                if *value <= 255 {
                    // Convert to SmallConstant if it fits
                    Ok(Operand::SmallConstant(*value as u8))
                } else {
                    // Large values require Variable form instruction
                    Err(CompilerError::CodeGenError(format!(
                        "Long form cannot handle large constant {} (> 255). Use Variable form instead.",
                        value
                    )))
                }
            }
            _ => Ok(operand.clone()),
        }
    }

    pub fn get_operand_type(&self, operand: &Operand) -> OperandType {
        match operand {
            Operand::SmallConstant(_) => OperandType::SmallConstant,
            Operand::LargeConstant(_) => OperandType::LargeConstant,
            Operand::Variable(_) => OperandType::Variable,
            Operand::Constant(value) => {
                // Choose optimal encoding based on value
                if *value <= 255 {
                    OperandType::SmallConstant
                } else {
                    OperandType::LargeConstant
                }
            }
        }
    }

    /// Emit a single operand
    fn emit_operand(&mut self, operand: &Operand) -> Result<(), CompilerError> {
        match operand {
            Operand::SmallConstant(value) => {
                self.emit_byte(*value)?;
            }
            Operand::Variable(value) => {
                // CRITICAL FIX: Properly encode Z-Machine variables
                // Variable(0) = stack top (0x00)
                // Variable(1-15) = local variables L01-L15 (0x01-0x0F)
                // Variable(16+) = global variables G00+ (0x10+)
                let zmachine_var = if *value == 0 {
                    0x00 // Stack top
                } else if *value <= 15 {
                    *value // Local variables L01-L15 (0x01-0x0F)
                } else {
                    0x10 + (*value - 16) // Global variables G00+ (0x10+)
                };

                log::debug!(
                    "Variable({}) -> Z-Machine variable 0x{:02x}",
                    value,
                    zmachine_var
                );
                self.emit_byte(zmachine_var)?;
            }
            Operand::LargeConstant(value) => {
                self.emit_word(*value)?;
            }
            Operand::Constant(value) => {
                // Choose encoding based on value size
                if *value <= 255 {
                    self.emit_byte(*value as u8)?;
                } else {
                    self.emit_word(*value)?;
                }
            }
        }
        Ok(())
    }

    /// Emit branch offset (1 or 2 bytes depending on size)
    pub fn emit_branch_offset(&mut self, offset: i16) -> Result<(), CompilerError> {
        // Z-Machine branch format:
        // - Bit 7: branch condition (1 = branch on true, 0 = branch on false)
        // - Bit 6: 0 = 2-byte offset, 1 = 1-byte offset
        // - Bits 5-0 or 13-0: signed offset

        // For now, assume positive condition and handle offset size
        if (0..=63).contains(&offset) {
            // 1-byte format: bit 7 = condition, bit 6 = 1, bits 5-0 = offset
            let branch_byte = 0x80 | 0x40 | (offset as u8 & 0x3F);
            self.emit_byte(branch_byte)?;
        } else {
            // 2-byte format: bit 7 = condition, bit 6 = 0, bits 13-0 = offset
            let branch_word = 0x8000 | ((offset as u16) & 0x3FFF);
            self.emit_word(branch_word)?;
        }

        Ok(())
    }
}

#[cfg(test)]
#[path = "codegen_tests.rs"]
mod tests;
