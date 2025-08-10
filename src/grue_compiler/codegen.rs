// Z-Machine Code Generator
//
// Transforms IR into executable Z-Machine bytecode following the Z-Machine Standard v1.1
// Supports both v3 and v5 target formats with proper memory layout and instruction encoding.

use crate::grue_compiler::error::CompilerError;
use crate::grue_compiler::ir::*;
use crate::grue_compiler::ZMachineVersion;
use std::collections::HashMap;

/// Z-Machine memory layout constants
const HEADER_SIZE: usize = 64; // Fixed 64-byte header
const DEFAULT_HIGH_MEMORY: u16 = 0x8000; // Start of high memory
const DEFAULT_PC_START: u16 = 0x1000; // Initial program counter

/// Z-Machine operand types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OperandType {
    LargeConstant, // 00: 16-bit constant
    SmallConstant, // 01: 8-bit constant
    Variable,      // 10: variable number
    Omitted,       // 11: operand omitted
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

/// Code generation context
pub struct ZMachineCodeGen {
    version: ZMachineVersion,

    // Memory layout
    story_data: Vec<u8>,
    current_address: usize,

    // Code generation state
    label_addresses: HashMap<IrId, usize>, // IR label ID -> byte address
    string_addresses: HashMap<IrId, usize>, // IR string ID -> byte address
    function_addresses: HashMap<IrId, usize>, // IR function ID -> byte address
    /// Mapping from IR IDs to string values (for LoadImmediate results)
    ir_id_to_string: HashMap<IrId, String>,
    /// Mapping from function IDs to builtin function names
    builtin_function_names: HashMap<IrId, String>,

    // Tables for Z-Machine structures
    object_table_addr: usize,
    property_table_addr: usize,
    dictionary_addr: usize,
    global_vars_addr: usize,

    // String encoding
    strings: Vec<(IrId, String)>, // Collected strings for encoding
    encoded_strings: HashMap<IrId, Vec<u8>>, // IR string ID -> encoded bytes
    next_string_id: IrId,         // Next available string ID

    // Address resolution
    reference_context: ReferenceContext,
}

impl ZMachineCodeGen {
    pub fn new(version: ZMachineVersion) -> Self {
        ZMachineCodeGen {
            version,
            story_data: vec![0; HEADER_SIZE],
            current_address: HEADER_SIZE,
            label_addresses: HashMap::new(),
            string_addresses: HashMap::new(),
            function_addresses: HashMap::new(),
            ir_id_to_string: HashMap::new(),
            builtin_function_names: HashMap::new(),
            object_table_addr: 0,
            property_table_addr: 0,
            dictionary_addr: 0,
            global_vars_addr: 0,
            strings: Vec::new(),
            encoded_strings: HashMap::new(),
            next_string_id: 1000, // Start string IDs from 1000 to avoid conflicts
            reference_context: ReferenceContext {
                ir_id_to_address: HashMap::new(),
                unresolved_refs: Vec::new(),
            },
        }
    }

    pub fn generate(&mut self, ir: IrProgram) -> Result<Vec<u8>, CompilerError> {
        // Debug: analyze all IR instructions to find missing label/jump mismatches
        println!("DEBUG: Analyzing IR program for label/jump consistency...");
        let mut all_jumps = std::collections::HashSet::new();
        let mut all_labels = std::collections::HashSet::new();

        for function in &ir.functions {
            for instr in &function.body.instructions {
                match instr {
                    IrInstruction::Label { id } => {
                        all_labels.insert(*id);
                        if *id == 49 {
                            println!("DEBUG: Found Label ID 49 in function '{}'", function.name);
                        }
                    }
                    IrInstruction::Jump { label } => {
                        all_jumps.insert(*label);
                        if *label == 49 {
                            println!("DEBUG: Found Jump to ID 49 in function '{}'", function.name);
                        }
                    }
                    IrInstruction::Branch {
                        condition: _,
                        true_label,
                        false_label,
                    } => {
                        all_jumps.insert(*true_label);
                        all_jumps.insert(*false_label);
                        if *true_label == 49 || *false_label == 49 {
                            println!(
                                "DEBUG: Found Branch to ID 49 in function '{}'",
                                function.name
                            );
                        }
                    }
                    _ => {}
                }
            }
        }

        let missing_labels: Vec<_> = all_jumps.difference(&all_labels).collect();
        if !missing_labels.is_empty() {
            println!(
                "DEBUG: Missing labels for jump targets: {:?}",
                missing_labels
            );
        }

        // Phase 1: Collect and encode all strings
        self.collect_strings(&ir)?;
        self.encode_all_strings()?;

        // Phase 2: Reserve space for Z-Machine structures
        self.layout_memory_structures(&ir)?;

        // Phase 3: Generate object and property tables
        self.generate_object_tables(&ir)?;

        // Phase 4: Generate dictionary
        self.generate_dictionary(&ir)?;

        // Phase 5: Generate global variables
        self.generate_global_variables(&ir)?;

        // Phase 6: Store the init block entry point address
        let init_entry_point = self.current_address;

        // Phase 6a: Generate init block first (entry point)
        if let Some(init_block) = &ir.init_block {
            self.generate_init_block(init_block)?;
        }

        // Phase 7: Generate code for all functions
        self.generate_functions(&ir)?;

        // Phase 8: Write Z-Machine header
        self.write_header_with_entry_point(init_entry_point)?;

        // Phase 8.5: Update string addresses for any dynamically discovered strings
        self.update_string_addresses();

        // Phase 8.6: Write all encoded strings to story data
        self.write_strings_to_memory()?;

        // Phase 9: Resolve all addresses and patch jumps
        self.resolve_addresses()?;

        Ok(self.story_data.clone())
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

    /// Collect strings from all LoadImmediate instructions in a block
    fn collect_strings_from_block(&mut self, block: &IrBlock) -> Result<(), CompilerError> {
        for instruction in &block.instructions {
            match instruction {
                IrInstruction::LoadImmediate { target, value } => {
                    if let IrValue::String(s) = value {
                        // Register the string for this IR ID
                        self.ir_id_to_string.insert(*target, s.clone());
                        // Add to strings collection for encoding
                        self.strings.push((*target, s.clone()));
                    }
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
        // Simplified Z-Machine string encoding (ZSCII)
        // TODO: Implement proper dictionary lookup and abbreviations

        let mut encoded = Vec::new();
        let chars: Vec<char> = s.chars().collect();

        // Process characters in groups of 3 (Z-Machine uses 3 5-bit chars per word)
        for chunk in chars.chunks(3) {
            let mut word = 0u16;

            // Pack up to 3 characters into a 16-bit word
            for (i, &ch) in chunk.iter().enumerate() {
                let zchar = match ch {
                    'a'..='z' => (ch as u8 - b'a' + 6) as u16,
                    'A'..='Z' => (ch as u8 - b'A' + 6) as u16, // Same as lowercase in alphabet 0
                    ' ' => 0,
                    '?' => 31, // Common punctuation
                    '.' => 31,
                    ',' => 31,
                    _ => 6, // Default to 'a' for unsupported chars
                };

                word |= (zchar & 0x1F) << (10 - i * 5);
            }

            // If this is the last word in the string, set the high bit
            if chunk.len() < 3 || chars.len() <= encoded.len() / 2 * 3 + 3 {
                word |= 0x8000;
            }

            // Store as big-endian
            encoded.push((word >> 8) as u8);
            encoded.push(word as u8);
        }

        // Ensure we have at least one word (empty strings become a single word with high bit set)
        if encoded.is_empty() {
            encoded.push(0x80);
            encoded.push(0x00);
        } else {
            // Make sure the last word has the high bit set to terminate the string
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

        // Reserve space for object table
        self.object_table_addr = addr;
        let object_count = ir.objects.len() + ir.rooms.len(); // Rooms become objects
        addr += match self.version {
            ZMachineVersion::V3 => object_count * 9 + 62, // v3: 9 bytes per object + 62 byte tree table
            ZMachineVersion::V5 => object_count * 14 + 126, // v5: 14 bytes per object + 126 byte tree table
        };

        // Reserve space for property tables
        self.property_table_addr = addr;
        addr += ir.objects.len() * 50; // Rough estimate: 50 bytes per object for properties

        // Reserve space for dictionary
        self.dictionary_addr = addr;
        addr += 1000; // Rough estimate for dictionary

        // Reserve space for encoded strings
        let string_data: Vec<(IrId, usize)> = self
            .encoded_strings
            .iter()
            .map(|(id, encoded)| (*id, encoded.len()))
            .collect();

        for (string_id, length) in string_data {
            self.string_addresses.insert(string_id, addr);
            self.record_address(string_id, addr); // Record in reference context
            addr += length;
        }

        // Code starts after all data structures
        self.current_address = addr;

        Ok(())
    }

    /// Generate object and property tables
    fn generate_object_tables(&mut self, _ir: &IrProgram) -> Result<(), CompilerError> {
        // TODO: Implement object table generation
        // For now, create minimal object table

        let obj_table_start = self.object_table_addr;
        self.ensure_capacity(obj_table_start + 100);

        // Write minimal object tree (31 or 63 default property values)
        let default_props = match self.version {
            ZMachineVersion::V3 => 31,
            ZMachineVersion::V5 => 63,
        };

        for i in 0..default_props {
            let addr = obj_table_start + i * 2;
            self.ensure_capacity(addr + 2);
            self.story_data[addr] = 0; // Default property value (high byte)
            self.story_data[addr + 1] = 0; // Default property value (low byte)
        }

        Ok(())
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
        for _global in &ir.globals {
            // TODO: Map IR globals to Z-Machine global variables
            // For now, just ensure the space is allocated
        }

        Ok(())
    }

    /// Generate code for all functions
    fn generate_functions(&mut self, ir: &IrProgram) -> Result<(), CompilerError> {
        // Generate all functions
        for function in &ir.functions {
            // Ensure even alignment for function addresses (required for Z-Machine v3)
            if matches!(self.version, ZMachineVersion::V3) && self.current_address % 2 == 1 {
                self.emit_byte(0)?; // Pad with null byte
            }

            let func_addr = self.current_address;
            println!(
                "DEBUG: Generating function ID {} '{}' at address {}",
                function.id, function.name, func_addr
            );
            self.function_addresses.insert(function.id, func_addr);

            // Record function address for resolution
            self.record_address(function.id, func_addr);

            // Generate function header (local variable count + types)
            self.generate_function_header(function)?;

            // Generate function body
            self.generate_block(&function.body)?;

            // Ensure function ends with a return
            if !self.block_ends_with_return(&function.body) {
                self.emit_return(None)?;
            }
        }

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
                // Store mapping for string values so we can resolve them in function calls
                if let IrValue::String(s) = value {
                    self.ir_id_to_string.insert(*target, s.clone());
                }
                self.generate_load_immediate(value)?;
            }

            IrInstruction::BinaryOp {
                target: _,
                op,
                left: _,
                right: _,
            } => {
                // TODO: Map IR IDs to actual operands
                // For now, use placeholder operands
                let left_op = Operand::Variable(1); // Local variable 1
                let right_op = Operand::Variable(2); // Local variable 2
                let store_var = Some(0); // Store to stack top

                self.generate_binary_op(op, left_op, right_op, store_var)?;
            }

            IrInstruction::Call {
                target: _,
                function,
                args,
            } => {
                // Check if this is a builtin function
                if self.is_builtin_function(*function) {
                    self.generate_builtin_function_call(*function, args)?;
                } else {
                    // Generate call with unresolved function reference
                    self.generate_call_with_reference(*function)?;
                }
            }

            IrInstruction::Return { value } => {
                if let Some(_ir_value) = value {
                    // Return with value - use ret opcode with operand
                    let operands = vec![Operand::Variable(0)]; // Return stack top
                    self.emit_instruction(0x0B, &operands, None, None)?;
                } else {
                    // Return without value - use rtrue (0OP)
                    self.emit_instruction(0x00, &[], None, None)?; // rtrue
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
                println!(
                    "DEBUG: Recording label ID {} at address {}",
                    *id, self.current_address
                );
                self.label_addresses.insert(*id, self.current_address);
                self.record_address(*id, self.current_address);
            }

            IrInstruction::LoadVar {
                target: _,
                var_id: _,
            } => {
                // Load variable value to stack
                // TODO: Map IR variable ID to Z-Machine variable number
                let operands = vec![Operand::Variable(1)]; // Load local variable 1
                self.emit_instruction(0x09, &operands, Some(0), None)?; // load to stack
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
                target: _,
                object: _,
                property,
            } => {
                // Generate Z-Machine get_prop instruction (2OP:17, opcode 0x11)
                // For now, use placeholder object ID and property number
                // TODO: Map IR object ID to actual Z-Machine object number
                // TODO: Map property name to property number
                let property_num = match property.as_str() {
                    "location" => 1, // Common property number for location/parent
                    "description" => 2,
                    _ => 3, // Default property number
                };

                // Generate get_prop instruction
                let operands = vec![
                    Operand::Variable(1),            // Object (placeholder - from local var 1)
                    Operand::Constant(property_num), // Property number
                ];
                self.emit_instruction(0x11, &operands, Some(0), None)?; // Store result in stack top
            }
            IrInstruction::SetProperty {
                object: _,
                property,
                value: _,
            } => {
                // Generate Z-Machine put_prop instruction (2OP:18, opcode 0x12)
                // For now, use placeholder object ID and property number
                // TODO: Map IR object ID to actual Z-Machine object number
                // TODO: Map property name to property number
                let property_num = match property.as_str() {
                    "location" => 1, // Common property number for location/parent
                    "description" => 2,
                    _ => 3, // Default property number
                };

                // Generate put_prop instruction
                let operands = vec![
                    Operand::Variable(1),            // Object (placeholder - from local var 1)
                    Operand::Constant(property_num), // Property number
                    Operand::Variable(0),            // Value (from stack top)
                ];
                self.emit_instruction(0x12, &operands, None, None)?;
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
                // Generate Z-Machine array access (loadw or loadb depending on array type)
                // For now, use loadw (load word from array)
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
                // Generate Z-Machine array store (storew or storeb depending on array type)
                // For now, use storew (store word to array)
                let operands = vec![
                    Operand::Variable(1), // Array base address (placeholder)
                    Operand::Variable(2), // Index (placeholder)
                    Operand::Variable(0), // Value (from stack)
                ];
                self.emit_instruction(0x21, &operands, None, None)?; // storew (2OP:33)
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
            IrValue::Integer(n) => {
                // Use store instruction: store <constant> -> (variable)
                // opcode 0x0D = store (1OP form)
                let operands = vec![Operand::Constant(*n as u16)];
                self.emit_instruction(0x0D, &operands, None, None)?;
            }
            IrValue::Boolean(b) => {
                // Store boolean as 0 or 1
                let value = if *b { 1 } else { 0 };
                let operands = vec![Operand::SmallConstant(value)];
                self.emit_instruction(0x0D, &operands, None, None)?;
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
            _ => {
                return Err(CompilerError::CodeGenError(format!(
                    "Binary operation {:?} not yet implemented",
                    op
                )));
            }
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
        // Choose appropriate call instruction based on argument count
        let opcode = match args.len() {
            0 => 0x20, // call_1n (1OP:32) - call with no args
            1 => 0x21, // call_1s (1OP:33) - call with 1 arg, store result
            _ => 0x00, // call_vs (VAR:0) - call with multiple args
        };

        let mut operands = vec![function_addr];
        operands.extend_from_slice(args);

        self.emit_instruction(opcode, &operands, store_var, None)
    }

    /// Generate function call with unresolved reference
    fn generate_call_with_reference(&mut self, function_id: IrId) -> Result<(), CompilerError> {
        // Emit call instruction with placeholder function address
        self.emit_byte(0xE0)?; // call_vs opcode (VAR form)
        self.emit_byte(0x00)?; // Operand types: all large constants

        // Add unresolved reference for function address (needs packed address)
        self.add_unresolved_reference(ReferenceType::FunctionCall, function_id, true)?;

        // Emit placeholder function address
        self.emit_word(0x0000)?;

        // Store result to stack (variable 0)
        self.emit_byte(0x00)?;

        Ok(())
    }

    /// Generate return instruction
    fn emit_return(&mut self, value: Option<IrId>) -> Result<(), CompilerError> {
        if value.is_some() {
            // Return with value - use ret opcode
            self.emit_byte(0x8B)?; // ret opcode (1OP form)
                                   // TODO: Add operand for return value
        } else {
            // Return without value - use rtrue
            self.emit_byte(0xB0)?; // rtrue opcode (0OP form)
        }
        Ok(())
    }

    /// Generate proper conditional branch instruction
    fn generate_conditional_branch(
        &mut self,
        condition: IrId,
        true_label: IrId,
        false_label: IrId,
    ) -> Result<(), CompilerError> {
        println!(
            "DEBUG: generate_conditional_branch condition={}, true={}, false={}",
            condition, true_label, false_label
        );

        // Use jz (jump if zero/false) - if condition is false, jump to false_label
        // Z-Machine jz: if operand == 0, jump to label; otherwise continue
        // For now, assume condition is a small constant (0-255)
        self.emit_byte(0x88)?; // jz opcode (1OP form)
        self.emit_byte(condition as u8)?; // condition variable as small constant

        // Add unresolved reference for false_label (jump target when condition is false)
        self.add_unresolved_reference(ReferenceType::Branch, false_label, false)?;
        self.emit_word(0x0000)?; // Placeholder branch offset

        // Fall through to true_label - no explicit jump needed,
        // true_label should be the next instruction after this branch
        // The IR should place the true_label immediately after this branch instruction

        Ok(())
    }

    /// Generate branch instruction (legacy method, kept for compatibility)
    fn generate_branch(&mut self, true_label: IrId) -> Result<(), CompilerError> {
        // For now, emit a simple unconditional branch using jump
        // TODO: Support proper conditional branching with condition operand
        println!(
            "DEBUG: generate_branch called with true_label={}",
            true_label
        );
        self.emit_byte(0x8C)?; // jump opcode (1OP form)

        // Add unresolved reference for the jump target
        self.add_unresolved_reference(ReferenceType::Jump, true_label, false)?;

        // Emit placeholder offset (will be resolved later)
        self.emit_word(0x0000)?;

        Ok(())
    }

    /// Generate unconditional jump
    fn generate_jump(&mut self, label: IrId) -> Result<(), CompilerError> {
        println!("DEBUG: generate_jump called with label={}", label);
        self.emit_byte(0x8C)?; // jump opcode (1OP form)

        // Add unresolved reference for the jump target
        self.add_unresolved_reference(ReferenceType::Jump, label, false)?;

        // Emit placeholder offset (will be resolved later)
        self.emit_word(0x0000)?;

        Ok(())
    }

    /// Generate init block as the main program entry point
    fn generate_init_block(&mut self, init_block: &IrBlock) -> Result<(), CompilerError> {
        // Generate the actual init block code
        for instruction in &init_block.instructions {
            self.generate_instruction(instruction)?;
        }

        // Add a quit instruction at the end to terminate the program
        self.emit_byte(0xBA)?; // quit opcode
        Ok(())
    }

    /// Write the Z-Machine file header with custom entry point
    fn write_header_with_entry_point(&mut self, entry_point: usize) -> Result<(), CompilerError> {
        // Z-Machine header fields
        self.story_data[0] = match self.version {
            ZMachineVersion::V3 => 3,
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
        self.write_word_at(14, self.dictionary_addr as u16)?;

        // File length (in 2-byte words for v3, 4-byte for v4+)
        let file_len = match self.version {
            ZMachineVersion::V3 => (self.story_data.len() / 2) as u16,
            ZMachineVersion::V5 => (self.story_data.len() / 4) as u16,
        };
        self.write_word_at(26, file_len)?;

        Ok(())
    }

    /// Resolve all address references and patch jumps/branches
    fn resolve_addresses(&mut self) -> Result<(), CompilerError> {
        // Process all unresolved references
        let unresolved_refs = self.reference_context.unresolved_refs.clone();

        for reference in unresolved_refs {
            self.resolve_single_reference(&reference)?;
        }

        // Clear resolved references
        self.reference_context.unresolved_refs.clear();

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
                println!("DEBUG: Failed to resolve IR ID {}", reference.target_id);
                println!(
                    "DEBUG: Available addresses: {:?}",
                    self.reference_context.ir_id_to_address
                );
                println!("DEBUG: Function addresses: {:?}", self.function_addresses);
                return Err(CompilerError::CodeGenError(format!(
                    "Cannot resolve reference to IR ID {}: target address not found",
                    reference.target_id
                )));
            }
        };

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

    /// Patch a jump offset at the given location
    pub fn patch_jump_offset(
        &mut self,
        location: usize,
        target_address: usize,
    ) -> Result<(), CompilerError> {
        let current_pc = location + 2; // Jump instruction PC after the jump
        let offset = (target_address as i32) - (current_pc as i32);

        if offset < -32768 || offset > 32767 {
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
        let current_pc = location + 1; // Branch instruction PC after the branch byte
        let offset = (target_address as i32) - (current_pc as i32);

        // Branch offsets are more complex due to 1-byte vs 2-byte encoding
        if offset >= 0 && offset <= 63 {
            // 1-byte format: preserve condition bit, set size bit, write offset
            let existing_byte = self.story_data[location];
            let condition_bit = existing_byte & 0x80;
            let new_byte = condition_bit | 0x40 | (offset as u8 & 0x3F);
            self.story_data[location] = new_byte;
        } else if offset >= -8192 && offset <= 8191 {
            // 2-byte format: preserve condition bit, clear size bit, write 14-bit offset
            let existing_byte = self.story_data[location];
            let condition_bit = existing_byte & 0x80;
            let branch_word = condition_bit as u16 | ((offset as u16) & 0x3FFF);

            self.story_data[location] = (branch_word >> 8) as u8;
            self.story_data[location + 1] = branch_word as u8;
        } else {
            return Err(CompilerError::CodeGenError(format!(
                "Branch offset {} too large for Z-Machine branch instruction",
                offset
            )));
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
                self.story_data[location] = address as u8;
            }
            2 => {
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
            ZMachineVersion::V5 => {
                // v5: packed address = byte address / 4
                if byte_address % 4 != 0 {
                    return Err(CompilerError::CodeGenError(
                        "Routine address must be multiple of 4 for v5".to_string(),
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
            ZMachineVersion::V5 => {
                // v5: packed address = byte address / 4
                if byte_address % 4 != 0 {
                    return Err(CompilerError::CodeGenError(
                        "String address must be multiple of 4 for v5".to_string(),
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
            // Core Z-Machine object primitives
            "get_child" => self.generate_get_child_builtin(args),
            "get_sibling" => self.generate_get_sibling_builtin(args),
            "test_attr" => self.generate_test_attr_builtin(args),
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
        if let Some(string_value) = self.ir_id_to_string.get(&arg_id).cloned() {
            // Create a string ID for this string and generate print instruction
            let string_id = self.find_or_create_string_id(&string_value)?;

            // Generate print_paddr instruction
            self.emit_byte(0x8D)?; // print_paddr opcode (1OP:0x0D, large constant)

            // Add unresolved reference for the string address
            self.add_unresolved_reference(ReferenceType::StringRef, string_id, true)?;

            // Emit placeholder string address (will be resolved later)
            self.emit_word(0x0000)?;
        } else {
            // If we can't find the string, generate a placeholder
            let placeholder_string = format!("[Missing string for IR ID {}]", arg_id);
            let string_id = self.find_or_create_string_id(&placeholder_string)?;
            self.ir_id_to_string.insert(string_id, placeholder_string);

            // Generate print_paddr instruction with placeholder
            self.emit_byte(0x8D)?; // print_paddr opcode (1OP:0x0D, large constant)
            self.add_unresolved_reference(ReferenceType::StringRef, string_id, true)?;
            self.emit_word(0x0000)?; // Placeholder address
        }

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

        let object_id = args[0];
        let destination_id = args[1];

        // Generate Z-Machine insert_obj instruction (2OP:14, opcode 0x0E)
        // This moves object to become the first child of the destination
        // Format: 2OP with large constant operands
        self.emit_byte(0x0E)?; // insert_obj opcode (2OP:14)

        // First operand: object ID (to be resolved to actual object number)
        self.emit_word(object_id as u16)?; // Object reference (to be resolved)

        // Second operand: destination ID (to be resolved to actual object/room number)
        self.emit_word(destination_id as u16)?; // Destination reference (to be resolved)

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

        let object_id = args[0];

        // Generate Z-Machine get_parent instruction (1OP:131, opcode 0x83)
        self.emit_byte(0x83)?; // get_parent opcode (1OP:131)
        self.emit_word(object_id as u16)?; // Object ID operand (large constant)
        self.emit_byte(0x00)?; // Store result in local variable 0 (stack)

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
                self.emit_word(0x0000)?; // Placeholder address

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
            Operand::Variable(object_id as u8), // Object
            Operand::Variable(attr_num as u8),  // Attribute number
        ];
        self.emit_instruction(0x0A, &operands, Some(0), None)?; // Store result in stack

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

        let object_id = args[0];

        // Generate Z-Machine get_child instruction (1OP:130, opcode 0x82)
        self.emit_byte(0x82)?; // get_child opcode
        self.emit_word(object_id as u16)?; // Object ID
        self.emit_byte(0x00)?; // Store result in local variable 0 (stack)

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

        let object_id = args[0];

        // Generate Z-Machine get_sibling instruction (1OP:129, opcode 0x81)
        self.emit_byte(0x81)?; // get_sibling opcode
        self.emit_word(object_id as u16)?; // Object ID
        self.emit_byte(0x00)?; // Store result in local variable 0 (stack)

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

        let object_id = args[0];

        // Visibility check algorithm:
        // 1. Get object's parent (location)
        // 2. Check if parent == player location (visible in room)
        // 3. Check if parent == player (in inventory)
        // 4. If parent is a container, check if container is open and visible

        // For now, implement basic visibility: check if object parent == player location or player
        // This is simplified - a full implementation would handle nested containers, lighting, etc.

        // Get object's parent location
        self.emit_byte(0x83)?; // get_parent opcode (1OP:131)
        self.emit_word(object_id as u16)?; // Object ID
        self.emit_byte(0x01)?; // Store in local variable 1

        // Get player location (assume player is object 1, location is its parent)
        self.emit_byte(0x83)?; // get_parent opcode (1OP:131)
        self.emit_word(0x0001)?; // Player object (object 1)
        self.emit_byte(0x02)?; // Store in local variable 2

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
        self.emit_byte(0x01)?; // rfalse opcode (0OP:1)

        // Return true (object is visible)
        self.emit_byte(0x00)?; // rtrue opcode (0OP:0)

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
        self.emit_byte(0x01)?; // rfalse opcode

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
        self.emit_word(0x0000)?; // Placeholder for string address

        // Get next sibling
        self.emit_byte(0x81)?; // get_sibling opcode (1OP:129)
        self.emit_byte(0x01)?; // Current object in variable 1
        self.emit_byte(0x01)?; // Store sibling back in variable 1
        self.emit_byte(0x40)?; // Branch if sibling exists
        self.emit_word(0xFFF0)?; // Loop back to print next object (negative offset)

        // Done listing - return
        self.emit_byte(0x00)?; // rtrue opcode

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
        self.emit_byte(0x01)?; // rfalse opcode

        // Loop through contents
        // Print current object
        self.emit_byte(0xB2)?; // print opcode
        let debug_msg = "Object in container";
        let string_id = self.find_or_create_string_id(debug_msg)?;
        self.ir_id_to_string
            .insert(string_id, debug_msg.to_string());
        self.add_unresolved_reference(ReferenceType::StringRef, string_id, true)?;
        self.emit_word(0x0000)?; // Placeholder for string address

        // Get next sibling
        self.emit_byte(0x81)?; // get_sibling opcode
        self.emit_byte(0x01)?; // Current object
        self.emit_byte(0x01)?; // Store sibling back in variable 1
        self.emit_byte(0x40)?; // Branch if sibling exists
        self.emit_word(0xFFF0)?; // Loop back (negative offset)

        // Done - return
        self.emit_byte(0x00)?; // rtrue opcode

        Ok(())
    }

    /// Update string addresses after new strings have been added
    fn update_string_addresses(&mut self) {
        // Calculate addresses for all encoded strings
        // Strings should be placed after all code, not after dictionary
        let mut addr = self.current_address + 100; // Start after current code with padding

        // For v3, ensure even alignment for strings
        if matches!(self.version, ZMachineVersion::V3) && addr % 2 == 1 {
            addr += 1;
        }

        self.string_addresses.clear();

        // Collect string data to avoid borrowing issues
        let string_data: Vec<(IrId, usize)> = self
            .encoded_strings
            .iter()
            .map(|(id, encoded)| (*id, encoded.len()))
            .collect();

        for (string_id, length) in string_data {
            // Ensure even alignment for v3
            if matches!(self.version, ZMachineVersion::V3) && addr % 2 == 1 {
                addr += 1;
            }

            self.string_addresses.insert(string_id, addr);
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
        // Write each encoded string to its assigned address
        for (string_id, encoded_bytes) in &self.encoded_strings {
            if let Some(&address) = self.string_addresses.get(string_id) {
                // Ensure we have enough space in story_data
                let required_size = address + encoded_bytes.len();
                if self.story_data.len() < required_size {
                    self.story_data.resize(required_size, 0);
                }

                // Write the encoded bytes to the story data
                for (i, &byte) in encoded_bytes.iter().enumerate() {
                    self.story_data[address + i] = byte;
                }

                // Also record this address in the IR ID mapping for reference resolution
                self.reference_context
                    .ir_id_to_address
                    .insert(*string_id, address);

                // Successfully wrote string to memory
            } else {
                return Err(CompilerError::CodeGenError(format!(
                    "String ID {} has no assigned address",
                    string_id
                )));
            }
        }
        Ok(())
    }

    /// Add an unresolved reference to be patched later
    pub fn add_unresolved_reference(
        &mut self,
        reference_type: ReferenceType,
        target_id: IrId,
        is_packed: bool,
    ) -> Result<(), CompilerError> {
        // Debug removed for cleaner output

        let reference = UnresolvedReference {
            reference_type,
            location: self.current_address,
            target_id,
            is_packed_address: is_packed,
            offset_size: 2, // Default to 2 bytes
        };
        // println!("DEBUG: Adding unresolved reference to IR ID {} at address {}", target_id, reference.location);
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
    pub fn emit_instruction(
        &mut self,
        opcode: u8,
        operands: &[Operand],
        store_var: Option<u8>,
        branch_offset: Option<i16>,
    ) -> Result<(), CompilerError> {
        let form = self.determine_instruction_form(operands.len(), opcode);

        match form {
            InstructionForm::Long => {
                self.emit_long_form(opcode, operands, store_var, branch_offset)?
            }
            InstructionForm::Short => {
                self.emit_short_form(opcode, operands, store_var, branch_offset)?
            }
            InstructionForm::Variable => {
                self.emit_variable_form(opcode, operands, store_var, branch_offset)?
            }
            InstructionForm::Extended => {
                return Err(CompilerError::CodeGenError(
                    "Extended form instructions not yet supported".to_string(),
                ));
            }
        }

        Ok(())
    }

    /// Determine instruction form based on operand count and opcode
    pub fn determine_instruction_form(&self, operand_count: usize, opcode: u8) -> InstructionForm {
        match operand_count {
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

        // Long form: bits 7-6 = operand types, bits 5-0 = opcode
        let op1_type = self.get_operand_type(&operands[0]);
        let op2_type = self.get_operand_type(&operands[1]);

        let instruction_byte = ((op1_type as u8) << 6) | ((op2_type as u8) << 5) | (opcode & 0x1F);
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

        // Variable form: bits 7-6 = 11, bit 5 = 0, bits 4-0 = opcode
        let instruction_byte = 0xC0 | (opcode & 0x1F);
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

    /// Get operand type for encoding
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
            Operand::SmallConstant(value) | Operand::Variable(value) => {
                self.emit_byte(*value)?;
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
        if offset >= 0 && offset <= 63 {
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
