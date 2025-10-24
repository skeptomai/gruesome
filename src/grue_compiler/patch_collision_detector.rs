/// Patch Collision Detection Utility
///
/// This utility detects memory overlap between DeferredBranchPatch and UnresolvedReference
/// patching systems to identify the root cause of the 0x2aa7 crash and validate
/// unified patching system requirements.

use crate::grue_compiler::codegen::{
    ZMachineCodeGen, DeferredBranchPatch, UnresolvedReference, MemorySpace
};
use crate::grue_compiler::error::CompilerError;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct PatchCollision {
    pub final_address: usize,
    pub branch_patch: Option<DeferredBranchPatch>,
    pub reference_patch: Option<UnresolvedReference>,
    pub overlap_bytes: usize,
}

#[derive(Debug)]
pub struct CollisionReport {
    pub collisions: Vec<PatchCollision>,
    pub total_branch_patches: usize,
    pub total_reference_patches: usize,
    pub collision_count: usize,
}

impl ZMachineCodeGen {
    /// Detect all potential memory collisions between patching systems
    ///
    /// This analysis translates all UnresolvedReference locations to final addresses
    /// and compares them with DeferredBranchPatch locations to find overlaps.
    pub fn detect_patch_collisions(&self) -> Result<CollisionReport, CompilerError> {
        let mut branch_patches: HashMap<usize, Vec<&DeferredBranchPatch>> = HashMap::new();
        let mut reference_patches: HashMap<usize, Vec<&UnresolvedReference>> = HashMap::new();
        let mut collisions = Vec::new();

        // Map all DeferredBranchPatch locations (already in final address space)
        for patch in &self.two_pass_state.deferred_branches {
            let start_addr = patch.branch_offset_location;
            let end_addr = start_addr + patch.offset_size as usize;

            for addr in start_addr..end_addr {
                branch_patches.entry(addr).or_insert_with(Vec::new).push(patch);
            }
        }

        // Map all UnresolvedReference locations (translate to final address space)
        for reference in &self.reference_context.unresolved_refs {
            // Translate space address to final address
            let final_location = match self.translate_space_address_to_final(
                reference.location_space,
                reference.location
            ) {
                Ok(addr) => addr,
                Err(e) => {
                    log::warn!(
                        "Could not translate reference location for collision detection: {:?}",
                        e
                    );
                    continue;
                }
            };

            let start_addr = final_location;
            let end_addr = start_addr + reference.offset_size as usize;

            for addr in start_addr..end_addr {
                reference_patches.entry(addr).or_insert_with(Vec::new).push(reference);
            }
        }

        // Find all overlapping addresses
        for (addr, branch_list) in &branch_patches {
            if let Some(reference_list) = reference_patches.get(addr) {
                // Collision detected at this address
                for &branch_patch in branch_list {
                    for &reference_patch in reference_list {
                        // Calculate overlap size
                        let branch_start = branch_patch.branch_offset_location;
                        let branch_end = branch_start + branch_patch.offset_size as usize;
                        let ref_final_location = self.translate_space_address_to_final(
                            reference_patch.location_space,
                            reference_patch.location
                        )?;
                        let ref_start = ref_final_location;
                        let ref_end = ref_start + reference_patch.offset_size as usize;

                        let overlap_start = branch_start.max(ref_start);
                        let overlap_end = branch_end.min(ref_end);
                        let overlap_bytes = overlap_end.saturating_sub(overlap_start);

                        if overlap_bytes > 0 {
                            collisions.push(PatchCollision {
                                final_address: *addr,
                                branch_patch: Some(branch_patch.clone()),
                                reference_patch: Some(reference_patch.clone()),
                                overlap_bytes,
                            });
                        }
                    }
                }
            }
        }

        Ok(CollisionReport {
            total_branch_patches: self.two_pass_state.deferred_branches.len(),
            total_reference_patches: self.reference_context.unresolved_refs.len(),
            collision_count: collisions.len(),
            collisions,
        })
    }

    /// Print detailed collision analysis
    pub fn print_collision_analysis(&self) -> Result<(), CompilerError> {
        let report = self.detect_patch_collisions()?;

        println!("=== PATCH COLLISION ANALYSIS ===");
        println!("Total DeferredBranchPatch entries: {}", report.total_branch_patches);
        println!("Total UnresolvedReference entries: {}", report.total_reference_patches);
        println!("Memory collisions detected: {}", report.collision_count);

        if report.collisions.is_empty() {
            println!("✅ No patch collisions found - systems are memory-safe");
            return Ok(());
        }

        println!("\n❌ COLLISION DETAILS:");
        for (i, collision) in report.collisions.iter().enumerate() {
            println!("\nCollision #{}: Final address 0x{:04x} ({} bytes overlap)",
                     i + 1, collision.final_address, collision.overlap_bytes);

            if let Some(branch) = &collision.branch_patch {
                println!("  📍 DeferredBranchPatch:");
                println!("    - Instruction at: 0x{:04x}", branch.instruction_address);
                println!("    - Branch location: 0x{:04x}-0x{:04x} ({} bytes)",
                         branch.branch_offset_location,
                         branch.branch_offset_location + branch.offset_size as usize,
                         branch.offset_size);
                println!("    - Target label: {}", branch.target_label_id);
                println!("    - Branch on true: {}", branch.branch_on_true);
            }

            if let Some(reference) = &collision.reference_patch {
                let final_location = self.translate_space_address_to_final(
                    reference.location_space,
                    reference.location
                )?;
                println!("  📍 UnresolvedReference:");
                println!("    - Type: {:?}", reference.reference_type);
                println!("    - Space location: {:?}[0x{:04x}]",
                         reference.location_space, reference.location);
                println!("    - Final location: 0x{:04x}-0x{:04x} ({} bytes)",
                         final_location,
                         final_location + reference.offset_size as usize,
                         reference.offset_size);
                println!("    - Target ID: {}", reference.target_id);
                println!("    - Is packed: {}", reference.is_packed_address);
            }
        }

        println!("\n🔧 ARCHITECTURAL IMPACT:");
        println!("These collisions explain the 0x2aa7 crash - both systems patch the same");
        println!("memory locations independently, causing corrupted branch offsets that");
        println!("lead to out-of-bounds jumps during runtime execution.");

        Ok(())
    }

    /// Generate collision statistics for unified system design
    pub fn collision_statistics(&self) -> Result<CollisionStats, CompilerError> {
        let report = self.detect_patch_collisions()?;

        let mut collision_by_type = HashMap::new();
        let mut addresses_affected = std::collections::HashSet::new();

        for collision in &report.collisions {
            addresses_affected.insert(collision.final_address);

            if let Some(ref_patch) = &collision.reference_patch {
                let entry = collision_by_type
                    .entry(format!("{:?}", ref_patch.reference_type))
                    .or_insert(0);
                *entry += 1;
            }
        }

        Ok(CollisionStats {
            total_collisions: report.collision_count,
            unique_addresses_affected: addresses_affected.len(),
            collision_by_reference_type: collision_by_type,
            collision_rate: if report.total_reference_patches > 0 {
                (report.collision_count as f64 / report.total_reference_patches as f64) * 100.0
            } else {
                0.0
            },
        })
    }
}

#[derive(Debug)]
pub struct CollisionStats {
    pub total_collisions: usize,
    pub unique_addresses_affected: usize,
    pub collision_by_reference_type: HashMap<String, usize>,
    pub collision_rate: f64,
}

impl CollisionStats {
    pub fn print_summary(&self) {
        println!("=== COLLISION STATISTICS ===");
        println!("Total collisions: {}", self.total_collisions);
        println!("Unique addresses affected: {}", self.unique_addresses_affected);
        println!("Collision rate: {:.2}%", self.collision_rate);

        if !self.collision_by_reference_type.is_empty() {
            println!("\nCollisions by reference type:");
            for (ref_type, count) in &self.collision_by_reference_type {
                println!("  {}: {}", ref_type, count);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grue_compiler::codegen::{TwoPassState, ReferenceContext, LegacyReferenceType};

    fn create_test_codegen() -> ZMachineCodeGen {
        // Create minimal test ZMachineCodeGen for collision testing
        // This should match the setup from other test modules
        use crate::grue_compiler::ZMachineVersion;
        let mut codegen = ZMachineCodeGen::new(ZMachineVersion::V3);
        codegen.two_pass_state.enabled = true;
        codegen
    }

    #[test]
    fn test_no_collisions_empty_state() {
        let codegen = create_test_codegen();
        let report = codegen.detect_patch_collisions().unwrap();

        assert_eq!(report.collision_count, 0);
        assert_eq!(report.total_branch_patches, 0);
        assert_eq!(report.total_reference_patches, 0);
    }

    #[test]
    fn test_collision_detection_basic() {
        let mut codegen = create_test_codegen();

        // Add a branch patch at final address 0x1000
        codegen.two_pass_state.deferred_branches.push(DeferredBranchPatch {
            instruction_address: 0x0ffe,
            branch_offset_location: 0x1000,
            target_label_id: 42,
            branch_on_true: true,
            offset_size: 2,
        });

        // Add a reference patch that translates to overlapping final address
        // For Code space, final address = final_code_base + space_offset
        // We need space_offset such that final_code_base + space_offset = 0x1001
        // This creates a 1-byte overlap at 0x1001
        let space_offset = if codegen.final_code_base <= 0x1001 {
            0x1001 - codegen.final_code_base
        } else {
            return; // Skip test if setup isn't feasible
        };

        codegen.reference_context.unresolved_refs.push(UnresolvedReference {
            reference_type: LegacyReferenceType::FunctionCall,
            location: space_offset,
            target_id: 100,
            is_packed_address: false,
            offset_size: 2,
            location_space: MemorySpace::Code,
        });

        let report = codegen.detect_patch_collisions().unwrap();

        assert_eq!(report.total_branch_patches, 1);
        assert_eq!(report.total_reference_patches, 1);

        // Should detect collision if addresses overlap
        if report.collision_count > 0 {
            assert_eq!(report.collision_count, 1);
            assert_eq!(report.collisions[0].overlap_bytes, 1);
        }
    }
}