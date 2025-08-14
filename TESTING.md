# Grue Compiler Testing Guide

This document describes the testing infrastructure for the Grue compiler and how to validate all example files.

## Test Scripts

### `scripts/ci_test.sh` - Core CI Tests
Validates the essential functionality that must always work:
- **15 working examples** compile successfully
- **6 core examples** run without crashes
- Known issues remain stable

This is the main test that runs in CI/CD and should always pass.

### `scripts/test_simple.sh` - Complete Test Suite
Tests all example files (working and failing) to provide full status:
- All 23 example files (16 in examples/, 7 debug files)
- Shows detailed pass/fail status
- Currently: 22 passing, 1 failing

### `scripts/test_all_examples.sh` - Detailed Test Suite
Comprehensive test with detailed error reporting and colored output.

## Example File Status

### ✅ Working Examples (22 files)

**Core Test Files:**
- `test_01_basic.grue` - Simple print statements
- `test_02_multiprint.grue` - Multiple print operations
- `test_03_function.grue` - Function calls (with placeholder)
- `test_04_room.grue` - Room and object definitions
- `test_05_player_location.grue` - Player location management
- `test_06_player_assignment.grue` - Property assignments

**Complex Examples:**
- `mini_zork.grue` - Full 400+ line Zork game (most complex)
- `property_test.grue` - Property system validation
- `text_encoding_test.grue` - String handling
- `error_handling_test.grue` - Error recovery

**Additional Examples:**
- `basic_test.grue`
- `builtin_test.grue`
- `minimal_function_test.grue`
- `test_simple.grue`
- `zork_banner_test.grue`

**Debug Files:**
- `debug_conditional_test.grue`
- `debug_function_simple.grue`
- `debug_minimal_fn.grue`
- `debug_param_conditional.grue`
- `debug_param_only.grue`
- `debug_param_test.grue`
- `debug_simple_control.grue`

### ❌ Known Issues (1 file)

**Stack Management Issues:**
- `control_flow_test.grue` - Complex nested conditionals cause stack underflow

This file compiles successfully but has runtime stack management issues with deeply nested conditionals and complex boolean expressions.

## Running Tests

### Quick Validation
```bash
# Run core CI tests (recommended for development)
./scripts/ci_test.sh

# Run all examples 
./scripts/test_simple.sh
```

### Full CI Check
```bash
# Run complete CI validation (includes all tests)
./scripts/check-ci.sh
```

### Manual Testing
```bash
# Compile and run a specific example
cargo run --bin grue-compiler -- examples/mini_zork.grue
RUST_LOG=warn cargo run --bin gruesome mini_zork.z3
```

## Success Metrics

**Current Status:** 96% Success Rate (22/23 programs working)

**CI Requirements:**
- All 15 working examples must compile
- All 6 core examples must run without crashes  
- No regressions in working functionality

**Quality Gates:**
- ✅ Simple programs (print, variables)
- ✅ Complex programs (rooms, objects, properties)
- ✅ Large programs (400+ line mini_zork)
- ✅ Function calls (with placeholder workaround)
- ❌ Complex control flow (stack underflow - known issue)

## Adding New Tests

1. **Create example file** in `examples/` directory
2. **Add to appropriate test script** (ci_test.sh for core functionality)
3. **Validate compilation and runtime** 
4. **Update this documentation**

## Debugging Test Failures

### Compilation Failures
```bash
# Run compiler with debug output
RUST_LOG=debug cargo run --bin grue-compiler -- examples/failing_example.grue
```

### Runtime Failures
```bash
# Run with debug logging
RUST_LOG=debug cargo run --bin gruesome failing_example.z3
```

### Common Issues
- **Stack underflow** - Complex control flow with nested conditionals
- **Invalid object numbers** - Object ID resolution issues
- **Invalid opcodes** - Instruction encoding problems

## Maintenance

### Regular Validation
Run the test suite after any changes to:
- Instruction generation (`codegen.rs`)
- Expression evaluation 
- Control flow handling
- Object/property system

### Regression Prevention
- All working examples must continue to work
- New features should not break existing functionality
- Known failures should remain stable (not get worse)

## Future Work

**Targets for 100% Success:**
1. Fix stack management in complex control flow
2. Implement proper function call resolution
3. Add more comprehensive error handling

**Additional Test Coverage:**
- Performance tests for large programs
- Memory usage validation
- Cross-platform compatibility tests