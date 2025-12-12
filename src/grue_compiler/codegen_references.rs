use crate::grue_compiler::codegen_memory::{MemorySpace, PLACEHOLDER_BYTE};
use crate::grue_compiler::error::CompilerError;
/// Reference resolution system for Z-Machine code generation
///
/// This module handles the complex task of resolving forward references, function calls,
/// jumps, branches, and cross-space address references in the Z-Machine compiler.
///
/// # Overview
///
/// During Z-Machine bytecode generation, many references cannot be resolved immediately
/// because the target addresses are not yet known. This module provides:
///
/// - **UnresolvedReference**: Tracks locations that need patching with final addresses
/// - **ReferenceContext**: Manages the collection and resolution of all references
/// - **Validation**: Ensures all placeholders are resolved and targets are valid
///
/// # Architecture
///
/// The reference resolution system works in two phases:
/// 1. **Collection Phase**: During code generation, unresolved references are recorded
/// 2. **Resolution Phase**: After all code is generated, references are resolved and patched
///
/// This allows forward references (jumps to labels not yet seen) and cross-space
/// references (code referencing strings, objects, etc.) to work correctly.
use crate::grue_compiler::ir::IrId;
use indexmap::IndexMap;

/// Reference to an unresolved location that needs to be patched during final assembly
#[derive(Debug, Clone)]
pub struct UnresolvedReference {
    pub reference_type: LegacyReferenceType,
    pub location: usize, // Byte offset in story data where patch is needed
    pub target_id: IrId, // IR ID being referenced (label, function, string)
    pub is_packed_address: bool, // Whether address needs to be packed
    pub offset_size: u8, // Size of offset field (1 or 2 bytes)
    pub location_space: MemorySpace, // Which memory space the location belongs to
}

/// Legacy reference types for the old unified memory system
#[derive(Debug, Clone, PartialEq)]
pub enum LegacyReferenceType {
    Jump,                                                  // Unconditional jump to label
    Branch,                                                // Conditional branch to label
    Label(IrId),                                           // Reference to label
    FunctionCall,                                          // Call to function address
    StringRef,                                             // Reference to string address
    StringPackedAddress { string_id: IrId }, // Reference to packed string address for properties
    DictionaryRef { word: String },          // Reference to dictionary entry address
    PropertyTableAddress { property_table_offset: usize }, // Reference to property table absolute address
    GlobalsBase, // Reference to global variables base address from header
}

/// Context for managing unresolved references during compilation
#[derive(Debug, Clone)]
pub struct ReferenceContext {
    pub ir_id_to_address: IndexMap<IrId, usize>, // Resolved addresses by IR ID
    pub unresolved_refs: Vec<UnresolvedReference>, // References waiting for resolution
}

impl Default for ReferenceContext {
    fn default() -> Self {
        Self::new()
    }
}

impl ReferenceContext {
    /// Create a new empty reference context
    pub fn new() -> Self {
        Self {
            ir_id_to_address: IndexMap::new(),
            unresolved_refs: Vec::new(),
        }
    }

    /// Add a new unresolved reference at a specific location
    pub fn add_unresolved_reference_at_location(
        &mut self,
        reference_type: LegacyReferenceType,
        target_id: IrId,
        is_packed: bool,
        location_space: MemorySpace,
        location_offset: usize,
        final_code_base: usize,
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
                    final_code_base + location_offset
                },
                MemorySpace::CodeSpace => {
                    // Use the exact offset provided by caller
                    final_code_base + location_offset
                },
                MemorySpace::Header => panic!("COMPILER BUG: Header space references not implemented - cannot use add_unresolved_reference() for Header space"),
                MemorySpace::Globals => panic!("COMPILER BUG: Globals space references not implemented - cannot use add_unresolved_reference() for Globals space"),
                MemorySpace::Abbreviations => panic!("COMPILER BUG: Abbreviations space references not implemented - cannot use add_unresolved_reference() for Abbreviations space"),
                MemorySpace::Objects => {
                    // Use the exact offset provided by caller - will be translated during resolution
                    location_offset
                },
                MemorySpace::Dictionary => panic!("COMPILER BUG: Dictionary space references not implemented - cannot use add_unresolved_reference() for Dictionary space"),
                MemorySpace::Strings => panic!("COMPILER BUG: Strings space references not implemented - cannot use add_unresolved_reference() for Strings space"),
            },
            target_id,
            is_packed_address: is_packed,
            offset_size: 2, // Default to 2 bytes
            location_space,
        };
        self.unresolved_refs.push(reference);
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
        code_space_len: usize,
        final_code_base: usize,
    ) -> Result<(), CompilerError> {
        // Calculate location using current code space length (BUGGY - for backward compatibility only)
        let location_offset = code_space_len;
        self.add_unresolved_reference_at_location(
            reference_type,
            target_id,
            is_packed,
            location_space,
            location_offset,
            final_code_base,
        )
    }

    /// Remove duplicate references to avoid double-patching
    pub fn deduplicate_references(&self, refs: &[UnresolvedReference]) -> Vec<UnresolvedReference> {
        use indexmap::IndexSet;

        let mut seen_references = IndexSet::new();
        let mut deduplicated = Vec::new();

        for reference in refs {
            // Deduplicate based on (target_id, location) pair - same target can be at different locations
            let ref_key = (reference.target_id, reference.location);
            if seen_references.insert(ref_key) {
                deduplicated.push(reference.clone());
            } else {
                log::debug!(
                    "DEDUPLICATION: Skipping duplicate reference target {} at location 0x{:04x}",
                    reference.target_id,
                    reference.location
                );
            }
        }

        log::info!(
            "Reference deduplication: {} → {} references ({} duplicate targets removed)",
            refs.len(),
            deduplicated.len(),
            refs.len() - deduplicated.len()
        );

        deduplicated
    }

    /// Validate that jump targets are within story bounds
    pub fn validate_jump_targets(
        &self,
        refs: &[UnresolvedReference],
        story_len: usize,
    ) -> Result<(), CompilerError> {
        for reference in refs {
            if matches!(
                reference.reference_type,
                LegacyReferenceType::Jump | LegacyReferenceType::Branch
            ) {
                if let Some(&target_addr) = self.ir_id_to_address.get(&reference.target_id) {
                    if target_addr >= story_len {
                        return Err(CompilerError::CodeGenError(format!(
                            "Jump target 0x{:04x} for IR ID {} exceeds story bounds (0x{:04x})",
                            target_addr, reference.target_id, story_len
                        )));
                    }
                    log::debug!(
                        "Jump target validation: IR ID {} → 0x{:04x} ✓",
                        reference.target_id,
                        target_addr
                    );
                }
            }
        }
        log::debug!("All jump targets within bounds");
        Ok(())
    }

    /// Validate that no unresolved 0xFFFF placeholders remain in the instruction stream
    pub fn validate_no_unresolved_placeholders(
        &self,
        story_data: &[u8],
        code_address: usize,
    ) -> Result<(), CompilerError> {
        let mut unresolved_count = 0;
        let mut scan_addr = 0x0040; // Start after header

        log::debug!(
            "Scanning for unresolved placeholders from 0x{:04x} to 0x{:04x}",
            scan_addr,
            code_address
        );

        while scan_addr + 1 < code_address {
            if story_data[scan_addr] == PLACEHOLDER_BYTE
                && story_data[scan_addr + 1] == PLACEHOLDER_BYTE
            {
                // Found potential unresolved placeholder
                log::debug!(
                    "UNRESOLVED PLACEHOLDER: Found 0xFFFF at address 0x{:04x}-0x{:04x}",
                    scan_addr,
                    scan_addr + 1
                );

                // Try to provide context about what instruction this might be in
                let context_start = scan_addr.saturating_sub(5);
                let context_end = (scan_addr + 10).min(code_address);
                let context_bytes: Vec<String> = story_data[context_start..context_end]
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

                log::debug!(
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
                "Found {} unresolved 0xFFFF placeholders in final code - this will cause runtime crashes",
                unresolved_count
            )));
        }

        log::debug!("Validation complete: No unresolved placeholders found");
        Ok(())
    }
}
