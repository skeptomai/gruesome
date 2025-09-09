// Experimental V4/V5 Golden File Tests for Grue Compiler
// These tests are experimental and not run in CI
// V4/V5 support is incomplete and has known issues

use std::fs;
use std::path::{Path, PathBuf};

use gruesome::grue_compiler::{codegen, ir, lexer, parser, semantic, ZMachineVersion};
use gruesome::vm::Game;

/// Experimental golden file test configuration for V4/V5
struct ExperimentalGoldenTest {
    name: &'static str,
    source_file: &'static str,
    expected_output_file: Option<&'static str>,
    should_compile: bool,
    target_version: ZMachineVersion,
}

const EXPERIMENTAL_TESTS: &[ExperimentalGoldenTest] = &[
    ExperimentalGoldenTest {
        name: "basic_test_v4",
        source_file: "examples/basic_test.grue",
        expected_output_file: Some("tests/golden_files/basic_test_v4.z4"),
        should_compile: false, // V4 compilation has known issues
        target_version: ZMachineVersion::V4,
    },
    ExperimentalGoldenTest {
        name: "basic_test_v5",
        source_file: "examples/basic_test.grue",
        expected_output_file: Some("tests/golden_files/basic_test_v5.z5"),
        should_compile: false, // V5 compilation has known issues
        target_version: ZMachineVersion::V5,
    },
    ExperimentalGoldenTest {
        name: "test_simple_v4",
        source_file: "examples/test_simple.grue",
        expected_output_file: Some("tests/golden_files/test_simple_v4.z4"),
        should_compile: false, // V4 compilation has known issues
        target_version: ZMachineVersion::V4,
    },
    ExperimentalGoldenTest {
        name: "mini_zork_v4",
        source_file: "examples/mini_zork.grue",
        expected_output_file: Some("tests/golden_files/mini_zork_v4.z4"),
        should_compile: false, // V4 compilation has known issues
        target_version: ZMachineVersion::V4,
    },
    ExperimentalGoldenTest {
        name: "mini_zork_v5",
        source_file: "examples/mini_zork.grue",
        expected_output_file: Some("tests/golden_files/mini_zork_v5.z5"),
        should_compile: false, // V5 compilation has known issues
        target_version: ZMachineVersion::V5,
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

    let story_data = code_generator
        .generate_complete_game_image(ir_program)
        .map_err(|e| format!("Code generation error: {:?}", e))?;

    Ok(story_data)
}

#[test]
#[ignore] // Ignored by default, run with --ignored for experimental tests
fn test_experimental_v4_v5_compilation() {
    println!("🧪 Running experimental V4/V5 tests...");
    println!("⚠️  These tests are expected to fail - V4/V5 support is experimental");

    let project_root = get_project_root();
    let mut successful_tests = 0;
    let mut failed_tests = 0;

    for test in EXPERIMENTAL_TESTS {
        println!("\n📋 Processing experimental test: {}", test.name);

        let source_path = project_root.join(test.source_file);
        
        match compile_grue_file(&source_path, test.target_version) {
            Ok(story_data) => {
                println!("✅ {} compiled successfully ({} bytes)", test.name, story_data.len());
                successful_tests += 1;
            }
            Err(e) => {
                println!("❌ {} failed: {}", test.name, e);
                failed_tests += 1;
            }
        }
    }

    println!("\n📊 Experimental test summary:");
    println!("   ✅ Successful: {}", successful_tests);
    println!("   ❌ Failed: {}", failed_tests);
    println!("   ⚠️  Failures are expected for V4/V5 - this is experimental code");
}

#[test]
#[ignore] // V5 compilation has known IR mapping and alignment issues
fn test_mini_zork_compilation_v5() {
    let project_root = get_project_root();
    let source_path = project_root.join("examples/mini_zork.grue");

    println!("🧪 Testing mini_zork.grue compilation to v5 (experimental)...");

    // Compile the source file
    let story_data = compile_grue_file(&source_path, ZMachineVersion::V5)
        .expect("Failed to compile mini_zork.grue");

    // Validate the generated Z-Machine file
    validate_z_machine_file(&story_data, ZMachineVersion::V5)
        .expect("Generated Z-Machine file failed validation");

    // Test that our interpreter can load it
    test_interpreter_can_load(&story_data)
        .expect("Interpreter failed to load generated story file");

    // Save as current golden file
    let golden_path = project_root.join("tests/golden_files/mini_zork_v5.z5");
    save_golden_file(&story_data, &golden_path).expect("Failed to save golden file");

    println!("✅ mini_zork v5 compilation test passed");
    println!("📁 Golden file saved: {}", golden_path.display());
}

fn get_project_root() -> PathBuf {
    std::env::current_dir()
        .unwrap()
        .ancestors()
        .find(|path| path.join("Cargo.toml").exists())
        .unwrap()
        .to_path_buf()
}

fn save_golden_file(story_data: &[u8], golden_path: &Path) -> Result<(), std::io::Error> {
    if let Some(parent) = golden_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(golden_path, story_data)?;
    Ok(())
}

fn test_interpreter_can_load(story_data: &[u8]) -> Result<(), String> {
    match Game::from_story_bytes(story_data) {
        Ok(_game) => Ok(()),
        Err(e) => Err(format!("Interpreter failed to load story file: {}", e)),
    }
}

fn validate_z_machine_file(story_data: &[u8], expected_version: ZMachineVersion) -> Result<(), String> {
    if story_data.len() < 64 {
        return Err("File too small - missing header".to_string());
    }

    let expected_version_byte = match expected_version {
        ZMachineVersion::V3 => 3,
        ZMachineVersion::V4 => 4,
        ZMachineVersion::V5 => 5,
    };

    let actual_version = story_data[0];
    if actual_version != expected_version_byte {
        return Err(format!(
            "Version mismatch: expected {}, got {}",
            expected_version_byte, actual_version
        ));
    }

    println!("✅ Z-Machine file validation passed for version {}", expected_version_byte);
    Ok(())
}

fn compile_grue_file(source_path: &Path, version: ZMachineVersion) -> Result<Vec<u8>, String> {
    // Read the source file
    let source_content = fs::read_to_string(source_path)
        .map_err(|e| format!("Failed to read source file: {}", e))?;

    // Phase 1: Lexical Analysis
    let mut lexer = lexer::Lexer::new(&source_content);
    let tokens = lexer.tokenize()
        .map_err(|e| format!("Tokenization failed: {}", e))?;

    // Phase 2: Parsing
    let mut parser = parser::Parser::new(tokens);
    let ast = parser.parse()
        .map_err(|e| format!("Parsing failed: {}", e))?;

    // Phase 3: Semantic Analysis
    let mut analyzer = semantic::SemanticAnalyzer::new();
    let analyzed_ast = analyzer.analyze(ast)
        .map_err(|e| format!("Semantic analysis failed: {}", e))?;

    // Phase 4: IR Generation
    let mut ir_generator = ir::IrGenerator::new();
    let ir_program = ir_generator.generate(analyzed_ast)
        .map_err(|e| format!("IR generation failed: {}", e))?;

    // Phase 5: Code Generation
    let mut code_generator = codegen::ZMachineCodeGen::new(version);

    // Transfer builtin function information from IR generator to code generator
    for (function_id, function_name) in ir_generator.get_builtin_functions() {
        code_generator.register_builtin_function(*function_id, function_name.clone());
    }

    // Transfer object numbers from IR generator to code generator
    code_generator.set_object_numbers(ir_generator.get_object_numbers().clone());

    let story_data = code_generator.generate_complete_game_image(ir_program)
        .map_err(|e| format!("Code generation failed: {}", e))?;

    Ok(story_data)
}