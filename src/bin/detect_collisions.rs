/// Collision Detection Test Binary
///
/// This binary compiles mini_zork.grue and runs the patch collision detector
/// to identify memory overlaps between DeferredBranchPatch and UnresolvedReference systems.

use gruesome::grue_compiler::{GrueCompiler, ZMachineVersion};
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    println!("=== PATCH COLLISION DETECTION TEST ===");
    println!("Compiling mini_zork.grue and analyzing patch collisions...\n");

    // Read the game source (check if collision_test.grue exists, else use mini_zork.grue)
    let source = if fs::metadata("examples/collision_test.grue").is_ok() {
        fs::read_to_string("examples/collision_test.grue")
            .expect("Could not read examples/collision_test.grue")
    } else {
        fs::read_to_string("examples/mini_zork.grue")
            .expect("Could not read examples/mini_zork.grue")
    };

    // Compile the game
    let compiler = GrueCompiler::new();
    match compiler.compile(&source, ZMachineVersion::V3) {
        Ok((_story_data, codegen)) => {
            println!("✅ Compilation successful");

            // Run collision detection
            match codegen.detect_patch_collisions() {
                Ok(report) => {
                    println!("✅ Collision detection completed\n");

                    // Print summary
                    println!("=== COLLISION SUMMARY ===");
                    println!("Total DeferredBranchPatch entries: {}", report.total_branch_patches);
                    println!("Total UnresolvedReference entries: {}", report.total_reference_patches);
                    println!("Memory collisions detected: {}", report.collision_count);

                    if report.collision_count > 0 {
                        println!("\n❌ COLLISION ANALYSIS:");
                        for (i, collision) in report.collisions.iter().take(5).enumerate() {
                            println!("\nCollision #{}: Final address 0x{:04x} ({} bytes overlap)",
                                     i + 1, collision.final_address, collision.overlap_bytes);

                            if let Some(branch) = &collision.branch_patch {
                                println!("  📍 DeferredBranchPatch:");
                                println!("    - Branch location: 0x{:04x}-0x{:04x}",
                                         branch.branch_offset_location,
                                         branch.branch_offset_location + branch.offset_size as usize);
                                println!("    - Target label: {}", branch.target_label_id);
                            }

                            if let Some(reference) = &collision.reference_patch {
                                match codegen.translate_space_address_to_final(
                                    reference.location_space,
                                    reference.location
                                ) {
                                    Ok(final_location) => {
                                        println!("  📍 UnresolvedReference:");
                                        println!("    - Type: {:?}", reference.reference_type);
                                        println!("    - Final location: 0x{:04x}-0x{:04x}",
                                                 final_location,
                                                 final_location + reference.offset_size as usize);
                                        println!("    - Target ID: {}", reference.target_id);
                                    }
                                    Err(e) => {
                                        println!("  📍 UnresolvedReference: (translation error: {:?})", e);
                                    }
                                }
                            }
                        }

                        if report.collision_count > 5 {
                            println!("\n... and {} more collisions", report.collision_count - 5);
                        }

                        println!("\n🔧 ARCHITECTURAL IMPACT:");
                        println!("These collisions explain the 0x2aa7 crash - both systems patch the same");
                        println!("memory locations independently, causing corrupted branch offsets.");

                        // Generate statistics
                        match codegen.collision_statistics() {
                            Ok(stats) => {
                                println!("\n=== STATISTICS ===");
                                stats.print_summary();
                            }
                            Err(e) => {
                                println!("Error generating statistics: {:?}", e);
                            }
                        }
                    } else {
                        println!("\n✅ No patch collisions found - systems are memory-safe");
                    }

                    // Detailed analysis
                    println!("\n=== DETAILED ANALYSIS ===");
                    if let Err(e) = codegen.print_collision_analysis() {
                        println!("Error printing detailed analysis: {:?}", e);
                    }
                }
                Err(e) => {
                    println!("❌ Collision detection failed: {:?}", e);
                }
            }
        }
        Err(e) => {
            println!("❌ Compilation failed: {:?}", e);
        }
    }

    Ok(())
}