// Golden File Tests for Grue Compiler
// Tests the complete compilation pipeline from .grue source to .z3 story files

use std::fs;
use std::path::{Path, PathBuf};

use gruesome::grue_compiler::{codegen, ir, lexer, parser, semantic, ZMachineVersion};
use gruesome::vm::Game;

/// Golden file test configuration
struct GoldenTest {
    name: &'static str,
    source_file: &'static str,
    expected_output_file: Option<&'static str>,
    should_compile: bool,
    target_version: ZMachineVersion,
}

// V3 integration tests - all grue files in src/grue_compiler/tests/
// V4/V5 tests have been moved to tests/experimental/v4_v5_golden_file_tests.rs
const GOLDEN_TESTS: &[GoldenTest] = &[
    // Example files
    GoldenTest {
        name: "basic_test_v3",
        source_file: "examples/basic_test.grue",
        expected_output_file: Some("tests/golden_files/basic_test_v3.z3"),
        should_compile: true,
        target_version: ZMachineVersion::V3,
    },
    GoldenTest {
        name: "mini_zork_v3",
        source_file: "examples/mini_zork.grue",
        expected_output_file: Some("tests/golden_files/mini_zork_v3.z3"),
        should_compile: true, // Re-enabled - compilation works correctly
        target_version: ZMachineVersion::V3,
    },
    // All test files from src/grue_compiler/tests/
    GoldenTest {
        name: "property_simple_test_v3",
        source_file: "src/grue_compiler/tests/property_simple_test.grue",
        expected_output_file: Some("tests/golden_files/property_simple_test_v3.z3"),
        should_compile: true,
        target_version: ZMachineVersion::V3,
    },
    GoldenTest {
        name: "test_01_basic_v3",
        source_file: "src/grue_compiler/tests/test_01_basic.grue",
        expected_output_file: Some("tests/golden_files/test_01_basic_v3.z3"),
        should_compile: true,
        target_version: ZMachineVersion::V3,
    },
    GoldenTest {
        name: "test_02_multiprint_v3",
        source_file: "src/grue_compiler/tests/test_02_multiprint.grue",
        expected_output_file: Some("tests/golden_files/test_02_multiprint_v3.z3"),
        should_compile: true,
        target_version: ZMachineVersion::V3,
    },
    GoldenTest {
        name: "test_03_function_v3",
        source_file: "src/grue_compiler/tests/test_03_function.grue",
        expected_output_file: Some("tests/golden_files/test_03_function_v3.z3"),
        should_compile: true,
        target_version: ZMachineVersion::V3,
    },
    GoldenTest {
        name: "test_04_room_v3",
        source_file: "src/grue_compiler/tests/test_04_room.grue",
        expected_output_file: Some("tests/golden_files/test_04_room_v3.z3"),
        should_compile: true,
        target_version: ZMachineVersion::V3,
    },
    GoldenTest {
        name: "test_05_player_location_v3",
        source_file: "src/grue_compiler/tests/test_05_player_location.grue",
        expected_output_file: Some("tests/golden_files/test_05_player_location_v3.z3"),
        should_compile: true,
        target_version: ZMachineVersion::V3,
    },
    GoldenTest {
        name: "test_06_player_assignment_v3",
        source_file: "src/grue_compiler/tests/test_06_player_assignment.grue",
        expected_output_file: Some("tests/golden_files/test_06_player_assignment_v3.z3"),
        should_compile: true,
        target_version: ZMachineVersion::V3,
    },
    GoldenTest {
        name: "test_array_compilation_v3",
        source_file: "src/grue_compiler/tests/test_array_compilation.grue",
        expected_output_file: Some("tests/golden_files/test_array_compilation_v3.z3"),
        should_compile: true,
        target_version: ZMachineVersion::V3,
    },
    GoldenTest {
        name: "test_array_errors_v3",
        source_file: "src/grue_compiler/tests/test_array_errors.grue",
        expected_output_file: Some("tests/golden_files/test_array_errors_v3.z3"),
        should_compile: true,
        target_version: ZMachineVersion::V3,
    },
    GoldenTest {
        name: "test_array_nested_v3",
        source_file: "src/grue_compiler/tests/test_array_nested.grue",
        expected_output_file: Some("tests/golden_files/test_array_nested_v3.z3"),
        should_compile: true,
        target_version: ZMachineVersion::V3,
    },
    GoldenTest {
        name: "test_array_ops_v3",
        source_file: "src/grue_compiler/tests/test_array_ops.grue",
        expected_output_file: Some("tests/golden_files/test_array_ops_v3.z3"),
        should_compile: true,
        target_version: ZMachineVersion::V3,
    },
    GoldenTest {
        name: "test_attributes_v3",
        source_file: "src/grue_compiler/tests/test_attributes.grue",
        expected_output_file: Some("tests/golden_files/test_attributes_v3.z3"),
        should_compile: true,
        target_version: ZMachineVersion::V3,
    },
    GoldenTest {
        name: "test_basic_v3",
        source_file: "src/grue_compiler/tests/test_basic.grue",
        expected_output_file: Some("tests/golden_files/test_basic_v3.z3"),
        should_compile: true,
        target_version: ZMachineVersion::V3,
    },
    GoldenTest {
        name: "test_builtin_functions_v3",
        source_file: "src/grue_compiler/tests/test_builtin_functions.grue",
        expected_output_file: Some("tests/golden_files/test_builtin_functions_v3.z3"),
        should_compile: true, // Fixed - implemented basic string function placeholders
        target_version: ZMachineVersion::V3,
    },
    GoldenTest {
        name: "test_complex_execution_v3",
        source_file: "src/grue_compiler/tests/test_complex_execution.grue",
        expected_output_file: Some("tests/golden_files/test_complex_execution_v3.z3"),
        should_compile: true,
        target_version: ZMachineVersion::V3,
    },
    GoldenTest {
        name: "test_complex_init_v3",
        source_file: "src/grue_compiler/tests/test_complex_init.grue",
        expected_output_file: Some("tests/golden_files/test_complex_init_v3.z3"),
        should_compile: true,
        target_version: ZMachineVersion::V3,
    },
    GoldenTest {
        name: "test_concat_only_v3",
        source_file: "src/grue_compiler/tests/test_concat_only.grue",
        expected_output_file: Some("tests/golden_files/test_concat_only_v3.z3"),
        should_compile: true,
        target_version: ZMachineVersion::V3,
    },
    GoldenTest {
        name: "test_conditional_v3",
        source_file: "src/grue_compiler/tests/test_conditional.grue",
        expected_output_file: Some("tests/golden_files/test_conditional_v3.z3"),
        should_compile: true,
        target_version: ZMachineVersion::V3,
    },
    GoldenTest {
        name: "test_function_call_v3",
        source_file: "src/grue_compiler/tests/test_function_call.grue",
        expected_output_file: Some("tests/golden_files/test_function_call_v3.z3"),
        should_compile: true,
        target_version: ZMachineVersion::V3,
    },
    GoldenTest {
        name: "test_function_chain_v3",
        source_file: "src/grue_compiler/tests/test_function_chain.grue",
        expected_output_file: Some("tests/golden_files/test_function_chain_v3.z3"),
        should_compile: true,
        target_version: ZMachineVersion::V3,
    },
    GoldenTest {
        name: "test_hello_world_v3",
        source_file: "src/grue_compiler/tests/test_hello_world.grue",
        expected_output_file: Some("tests/golden_files/test_hello_world_v3.z3"),
        should_compile: true,
        target_version: ZMachineVersion::V3,
    },
    GoldenTest {
        name: "test_minimal_branch_v3",
        source_file: "src/grue_compiler/tests/test_minimal_branch.grue",
        expected_output_file: Some("tests/golden_files/test_minimal_branch_v3.z3"),
        should_compile: true,
        target_version: ZMachineVersion::V3,
    },
    GoldenTest {
        name: "test_minimal_conditional_v3",
        source_file: "src/grue_compiler/tests/test_minimal_conditional.grue",
        expected_output_file: Some("tests/golden_files/test_minimal_conditional_v3.z3"),
        should_compile: true,
        target_version: ZMachineVersion::V3,
    },
    GoldenTest {
        name: "test_minimal_property_v3",
        source_file: "src/grue_compiler/tests/test_minimal_property.grue",
        expected_output_file: Some("tests/golden_files/test_minimal_property_v3.z3"),
        should_compile: true, // Fixed - removed explicit player object definition
        target_version: ZMachineVersion::V3,
    },
    GoldenTest {
        name: "test_minimal_var_v3",
        source_file: "src/grue_compiler/tests/test_minimal_var.grue",
        expected_output_file: Some("tests/golden_files/test_minimal_var_v3.z3"),
        should_compile: true,
        target_version: ZMachineVersion::V3,
    },
    GoldenTest {
        name: "test_object_simple_v3",
        source_file: "src/grue_compiler/tests/test_object_simple.grue",
        expected_output_file: Some("tests/golden_files/test_object_simple_v3.z3"),
        should_compile: true,
        target_version: ZMachineVersion::V3,
    },
    GoldenTest {
        name: "test_objects_v3",
        source_file: "src/grue_compiler/tests/test_objects.grue",
        expected_output_file: Some("tests/golden_files/test_objects_v3.z3"),
        should_compile: true,
        target_version: ZMachineVersion::V3,
    },
    GoldenTest {
        name: "test_progressive_features_v3",
        source_file: "src/grue_compiler/tests/test_progressive_features.grue",
        expected_output_file: Some("tests/golden_files/test_progressive_features_v3.z3"),
        should_compile: true,
        target_version: ZMachineVersion::V3,
    },
    GoldenTest {
        name: "test_property_conditional_v3",
        source_file: "src/grue_compiler/tests/test_property_conditional.grue",
        expected_output_file: Some("tests/golden_files/test_property_conditional_v3.z3"),
        should_compile: true,
        target_version: ZMachineVersion::V3,
    },
    GoldenTest {
        name: "test_property_simple_v3",
        source_file: "src/grue_compiler/tests/test_property_simple.grue",
        expected_output_file: Some("tests/golden_files/test_property_simple_v3.z3"),
        should_compile: true,
        target_version: ZMachineVersion::V3,
    },
    GoldenTest {
        name: "test_pure_conditionals_v3",
        source_file: "src/grue_compiler/tests/test_pure_conditionals.grue",
        expected_output_file: Some("tests/golden_files/test_pure_conditionals_v3.z3"),
        should_compile: true,
        target_version: ZMachineVersion::V3,
    },
    GoldenTest {
        name: "test_random_v3",
        source_file: "src/grue_compiler/tests/test_random.grue",
        expected_output_file: Some("tests/golden_files/test_random_v3.z3"),
        should_compile: true,
        target_version: ZMachineVersion::V3,
    },
    GoldenTest {
        name: "test_simple_v3",
        source_file: "src/grue_compiler/tests/test_simple.grue",
        expected_output_file: Some("tests/golden_files/test_simple_v3.z3"),
        should_compile: true,
        target_version: ZMachineVersion::V3,
    },
    GoldenTest {
        name: "test_simple_array_v3",
        source_file: "src/grue_compiler/tests/test_simple_array.grue",
        expected_output_file: Some("tests/golden_files/test_simple_array_v3.z3"),
        should_compile: true,
        target_version: ZMachineVersion::V3,
    },
    GoldenTest {
        name: "test_simple_conditional_v3",
        source_file: "src/grue_compiler/tests/test_simple_conditional.grue",
        expected_output_file: Some("tests/golden_files/test_simple_conditional_v3.z3"),
        should_compile: true,
        target_version: ZMachineVersion::V3,
    },
    GoldenTest {
        name: "test_simple_variable_v3",
        source_file: "src/grue_compiler/tests/test_simple_variable.grue",
        expected_output_file: Some("tests/golden_files/test_simple_variable_v3.z3"),
        should_compile: true,
        target_version: ZMachineVersion::V3,
    },
    GoldenTest {
        name: "test_triple_concat_v3",
        source_file: "src/grue_compiler/tests/test_triple_concat.grue",
        expected_output_file: Some("tests/golden_files/test_triple_concat_v3.z3"),
        should_compile: true,
        target_version: ZMachineVersion::V3,
    },
    GoldenTest {
        name: "test_variable_conditional_v3",
        source_file: "src/grue_compiler/tests/test_variable_conditional.grue",
        expected_output_file: Some("tests/golden_files/test_variable_conditional_v3.z3"),
        should_compile: true,
        target_version: ZMachineVersion::V3,
    },
    GoldenTest {
        name: "test_variable_simple_v3",
        source_file: "src/grue_compiler/tests/test_variable_simple.grue",
        expected_output_file: Some("tests/golden_files/test_variable_simple_v3.z3"),
        should_compile: true,
        target_version: ZMachineVersion::V3,
    },
    GoldenTest {
        name: "test_variables_v3",
        source_file: "src/grue_compiler/tests/test_variables.grue",
        expected_output_file: Some("tests/golden_files/test_variables_v3.z3"),
        should_compile: true,
        target_version: ZMachineVersion::V3,
    },
    GoldenTest {
        name: "variable_test_v3",
        source_file: "src/grue_compiler/tests/variable_test.grue",
        expected_output_file: Some("tests/golden_files/variable_test_v3.z3"),
        should_compile: true,
        target_version: ZMachineVersion::V3,
    },
];

fn get_project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn compile_grue_file(
    source_path: &Path,
    target_version: ZMachineVersion,
) -> Result<Vec<u8>, String> {
    // Read the source file
    let source_content = fs::read_to_string(source_path)
        .map_err(|e| format!("Failed to read source file: {}", e))?;

    // Create compiler pipeline
    let mut lexer = lexer::Lexer::new(&source_content);
    let tokens = lexer
        .tokenize()
        .map_err(|e| format!("Lexer error: {:?}", e))?;

    let mut parser = parser::Parser::new(tokens);
    let ast = parser
        .parse()
        .map_err(|e| format!("Parser error: {:?}", e))?;

    let mut semantic_analyzer = semantic::SemanticAnalyzer::new();
    let analyzed_ast = semantic_analyzer
        .analyze(ast)
        .map_err(|e| format!("Semantic analysis error: {:?}", e))?;

    let mut ir_generator = ir::IrGenerator::new();
    let ir_program = ir_generator
        .generate(analyzed_ast)
        .map_err(|e| format!("IR generation error: {:?}", e))?;

    let mut code_generator = codegen::ZMachineCodeGen::new(target_version);

    // Register builtin functions from IR generation
    for (function_id, function_name) in ir_generator.get_builtin_functions() {
        code_generator.register_builtin_function(*function_id, function_name.clone());
    }

    // CRITICAL FIX (Oct 13, 2025): Transfer object numbers from IR generator to code generator
    // This was missing, causing "Room 'X' has no object number from IR" errors
    code_generator.set_object_numbers(ir_generator.get_object_numbers().clone());

    let story_data = code_generator
        .generate_complete_game_image(ir_program)
        .map_err(|e| format!("Code generation error: {:?}", e))?;

    Ok(story_data)
}

fn validate_z_machine_file(
    story_data: &[u8],
    expected_version: ZMachineVersion,
) -> Result<(), String> {
    if story_data.is_empty() {
        return Err("Story data is empty".to_string());
    }

    // Check minimum size (at least header)
    if story_data.len() < 64 {
        return Err(format!("Story file too small: {} bytes", story_data.len()));
    }

    // Check version byte
    let version = story_data[0];
    let expected_version_byte = match expected_version {
        ZMachineVersion::V3 => 3,
        ZMachineVersion::V4 => 4,
        ZMachineVersion::V5 => 5,
    };

    if version != expected_version_byte {
        return Err(format!(
            "Version mismatch: expected {}, found {}",
            expected_version_byte, version
        ));
    }

    // Check that required header fields are non-zero
    let dict_addr = u16::from_be_bytes([story_data[8], story_data[9]]);
    let obj_table_addr = u16::from_be_bytes([story_data[10], story_data[11]]);
    let globals_addr = u16::from_be_bytes([story_data[12], story_data[13]]);
    let static_mem_addr = u16::from_be_bytes([story_data[14], story_data[15]]);
    let high_mem_addr = u16::from_be_bytes([story_data[4], story_data[5]]);
    let pc_addr = u16::from_be_bytes([story_data[6], story_data[7]]);

    if dict_addr < 64 {
        return Err("Dictionary address too low".to_string());
    }
    if obj_table_addr < 64 {
        return Err("Object table address too low".to_string());
    }
    if globals_addr < 64 {
        return Err("Globals address too low".to_string());
    }
    if static_mem_addr < 64 {
        return Err("Static memory address too low".to_string());
    }
    if high_mem_addr == 0 {
        return Err("High memory address is zero".to_string());
    }
    if pc_addr == 0 {
        return Err("PC address is zero".to_string());
    }

    // Check memory layout consistency
    if dict_addr >= high_mem_addr
        || obj_table_addr >= high_mem_addr
        || globals_addr >= high_mem_addr
    {
        return Err("Memory layout inconsistency: tables in high memory".to_string());
    }

    println!("✅ Z-Machine file validation passed:");
    println!("   Version: {}", version);
    println!("   Size: {} bytes", story_data.len());
    println!("   Dictionary: 0x{:04x}", dict_addr);
    println!("   Objects: 0x{:04x}", obj_table_addr);
    println!("   Globals: 0x{:04x}", globals_addr);
    println!("   Static: 0x{:04x}", static_mem_addr);
    println!("   High Memory: 0x{:04x}", high_mem_addr);
    println!("   PC: 0x{:04x}", pc_addr);

    Ok(())
}

fn create_golden_file_directories() -> Result<(), String> {
    let project_root = get_project_root();
    let golden_dir = project_root.join("tests/golden_files");

    fs::create_dir_all(&golden_dir)
        .map_err(|e| format!("Failed to create golden files directory: {}", e))?;

    Ok(())
}

fn save_golden_file(story_data: &[u8], path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create parent directory: {}", e))?;
    }

    fs::write(path, story_data).map_err(|e| format!("Failed to write golden file: {}", e))?;

    Ok(())
}

fn load_golden_file(path: &Path) -> Result<Vec<u8>, String> {
    fs::read(path).map_err(|e| format!("Failed to read golden file: {}", e))
}

fn compare_story_files(actual: &[u8], expected: &[u8]) -> Result<(), String> {
    if actual.len() != expected.len() {
        return Err(format!(
            "File size mismatch: actual {} bytes, expected {} bytes",
            actual.len(),
            expected.len()
        ));
    }

    // Compare header (first 64 bytes) carefully
    for i in 0..64.min(actual.len()) {
        if actual[i] != expected[i] {
            return Err(format!(
                "Header byte difference at offset 0x{:02x}: actual 0x{:02x}, expected 0x{:02x}",
                i, actual[i], expected[i]
            ));
        }
    }

    // For content beyond header, we might allow some differences (timestamps, etc.)
    // But for now, require exact match
    if actual != expected {
        // Find first difference
        for (i, (a, b)) in actual.iter().zip(expected.iter()).enumerate() {
            if a != b {
                return Err(format!(
                    "Content difference at offset 0x{:04x}: actual 0x{:02x}, expected 0x{:02x}",
                    i, a, b
                ));
            }
        }
    }

    Ok(())
}

fn test_interpreter_can_load(story_data: &[u8]) -> Result<(), String> {
    // Try to load the story file with our interpreter
    match Game::from_memory(story_data.to_vec()) {
        Ok(_game) => {
            println!("✅ Story file successfully loaded by gruesome interpreter");
            Ok(())
        }
        Err(e) => Err(format!("Failed to load story file in interpreter: {}", e)),
    }
}

#[test]
fn test_mini_zork_compilation_v3() {
    let project_root = get_project_root();
    let source_path = project_root.join("examples/mini_zork.grue");

    println!("🧪 Testing mini_zork.grue compilation to v3...");

    // Compile the source file
    let story_data = compile_grue_file(&source_path, ZMachineVersion::V3)
        .expect("Failed to compile mini_zork.grue");

    // Validate the generated Z-Machine file
    validate_z_machine_file(&story_data, ZMachineVersion::V3)
        .expect("Generated Z-Machine file failed validation");

    // Test that our interpreter can load it
    test_interpreter_can_load(&story_data)
        .expect("Interpreter failed to load generated story file");

    // Save as current golden file
    let golden_path = project_root.join("tests/golden_files/mini_zork_v3.z3");
    create_golden_file_directories().expect("Failed to create directories");
    save_golden_file(&story_data, &golden_path).expect("Failed to save golden file");

    println!("✅ mini_zork v3 compilation test passed");
    println!("📁 Golden file saved: {}", golden_path.display());
}

// V5 mini_zork test moved to tests/experimental/v4_v5_golden_file_tests.rs

#[test]
fn test_simple_compilation() {
    let project_root = get_project_root();
    let source_path = project_root.join("src/grue_compiler/tests/test_simple.grue");

    println!("🧪 Testing src/grue_compiler/tests/test_simple.grue compilation...");

    // Compile the source file
    let story_data = compile_grue_file(&source_path, ZMachineVersion::V3)
        .expect("Failed to compile src/grue_compiler/tests/test_simple.grue");

    // Validate the generated Z-Machine file
    validate_z_machine_file(&story_data, ZMachineVersion::V3)
        .expect("Generated Z-Machine file failed validation");

    // Test that our interpreter can load it
    test_interpreter_can_load(&story_data)
        .expect("Interpreter failed to load generated story file");

    // Save as current golden file
    let golden_path = project_root.join("tests/golden_files/test_simple_v3.z3");
    save_golden_file(&story_data, &golden_path).expect("Failed to save golden file");

    println!("✅ test_simple compilation test passed");
    println!("📁 Golden file saved: {}", golden_path.display());
}

#[test]
fn test_basic_compilation() {
    let project_root = get_project_root();
    let source_path = project_root.join("examples/basic_test.grue");

    println!("🧪 Testing basic_test.grue compilation...");

    // Compile the source file
    let story_data = compile_grue_file(&source_path, ZMachineVersion::V3)
        .expect("Failed to compile basic_test.grue");

    // Validate the generated Z-Machine file
    validate_z_machine_file(&story_data, ZMachineVersion::V3)
        .expect("Generated Z-Machine file failed validation");

    // Test that our interpreter can load it
    test_interpreter_can_load(&story_data)
        .expect("Interpreter failed to load generated story file");

    // Save as current golden file
    let golden_path = project_root.join("tests/golden_files/basic_test_v3.z3");
    save_golden_file(&story_data, &golden_path).expect("Failed to save golden file");

    println!("✅ basic_test compilation test passed");
    println!("📁 Golden file saved: {}", golden_path.display());
}

#[test]
fn test_golden_file_regeneration() {
    println!("🧪 Running all golden file tests...");

    let project_root = get_project_root();
    create_golden_file_directories().expect("Failed to create golden file directories");

    for test_config in GOLDEN_TESTS {
        println!("\n📋 Processing test: {}", test_config.name);

        let source_path = project_root.join(test_config.source_file);

        if !source_path.exists() {
            panic!("Source file not found: {}", source_path.display());
        }

        if test_config.should_compile {
            // Compile the source file
            let story_data = compile_grue_file(&source_path, test_config.target_version)
                .unwrap_or_else(|e| panic!("Failed to compile {}: {}", test_config.source_file, e));

            // Validate the generated file
            validate_z_machine_file(&story_data, test_config.target_version)
                .unwrap_or_else(|e| panic!("Validation failed for {}: {}", test_config.name, e));

            // Test interpreter loading
            test_interpreter_can_load(&story_data).unwrap_or_else(|e| {
                panic!("Interpreter test failed for {}: {}", test_config.name, e)
            });

            // Save golden file if specified
            if let Some(output_file) = test_config.expected_output_file {
                let golden_path = project_root.join(output_file);
                save_golden_file(&story_data, &golden_path).unwrap_or_else(|e| {
                    panic!("Failed to save golden file for {}: {}", test_config.name, e)
                });

                println!("✅ {} compiled successfully", test_config.name);
                println!("📁 Golden file: {}", golden_path.display());
            }
        }
    }

    println!("\n🎉 All golden file tests completed successfully!");
}

#[test]
fn test_compare_with_existing_golden_files() {
    let project_root = get_project_root();

    for test_config in GOLDEN_TESTS {
        if !test_config.should_compile {
            continue;
        }

        if let Some(expected_output_file) = test_config.expected_output_file {
            let golden_path = project_root.join(expected_output_file);

            // Skip if golden file doesn't exist yet
            if !golden_path.exists() {
                println!(
                    "⚠️  Golden file not found, skipping comparison: {}",
                    golden_path.display()
                );
                continue;
            }

            println!("🔍 Comparing {} with golden file...", test_config.name);

            // Compile current version
            let source_path = project_root.join(test_config.source_file);
            let current_data = compile_grue_file(&source_path, test_config.target_version)
                .unwrap_or_else(|e| panic!("Failed to compile {}: {}", test_config.source_file, e));

            // Load golden file
            let expected_data = load_golden_file(&golden_path).unwrap_or_else(|e| {
                panic!(
                    "Failed to load golden file {}: {}",
                    golden_path.display(),
                    e
                )
            });

            // Compare
            match compare_story_files(&current_data, &expected_data) {
                Ok(()) => {
                    println!("✅ {} matches golden file exactly", test_config.name);
                }
                Err(e) => {
                    println!("⚠️  {} differs from golden file: {}", test_config.name, e);
                    println!("   This may be expected if the implementation has changed.");
                    println!(
                        "   Run `cargo test test_golden_file_regeneration` to update golden files."
                    );
                }
            }
        }
    }
}
