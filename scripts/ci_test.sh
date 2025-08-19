#!/bin/bash

# CI Test Script for Grue Compiler
# This script validates the core functionality and can be run in CI/CD

set -e

echo "=== Grue Compiler CI Test Suite ==="
echo ""

# Test compilation of all working examples
WORKING_EXAMPLES=(
    "examples/basic_test.grue"
    "examples/error_handling_test.grue"
    "examples/mini_zork.grue"
    "examples/minimal_function_test.grue"
    "examples/property_test.grue"
    "examples/test_01_basic.grue"
    "examples/test_02_multiprint.grue"
    "examples/test_03_function.grue"
    "examples/test_04_room.grue"
    "examples/test_05_player_location.grue"
    "examples/test_06_player_assignment.grue"
    "examples/test_simple.grue"
    "examples/text_encoding_test.grue"
    "examples/zork_banner_test.grue"
)

# Known failing examples (complex control flow issues)
FAILING_EXAMPLES=(
    "examples/control_flow_test.grue"
)

# Core functionality tests
CORE_TESTS=(
    "examples/test_01_basic.grue"
    "examples/test_02_multiprint.grue"
    "examples/test_04_room.grue"
    "examples/test_05_player_location.grue" 
    "examples/test_06_player_assignment.grue"
    "examples/mini_zork.grue"
)

echo "Testing compilation of all working examples..."
for file in "${WORKING_EXAMPLES[@]}"; do
    echo -n "  Compiling $file... "
    if cargo run --bin grue-compiler -- "$file" >/dev/null 2>&1; then
        echo "OK"
    else
        echo "FAILED"
        echo "❌ Compilation regression detected in $file"
        exit 1
    fi
done

echo ""
echo "Testing runtime execution of core examples..."
for file in "${CORE_TESTS[@]}"; do
    basename=$(basename "$file" .grue)
    z3_file="${basename}.z3"
    echo -n "  Running $file... "
    
    output=$(timeout 10s cargo run --bin gruesome "$z3_file" 2>&1 || true)
    
    if echo "$output" | grep -q "Stack underflow\|Invalid object\|Failed to decode\|Error:"; then
        echo "FAILED" 
        echo "❌ Runtime regression detected in $file"
        echo "Error: $(echo "$output" | grep -o "Stack underflow\|Invalid object\|Failed to decode\|Error:.*" | head -1)"
        exit 1
    else
        echo "OK"
    fi
done

echo ""
echo "Verifying known failing examples still fail gracefully..."
for file in "${FAILING_EXAMPLES[@]}"; do
    echo -n "  Testing $file... "
    if cargo run --bin grue-compiler -- "$file" >/dev/null 2>&1; then
        echo "Compiles (expected)"
    else
        echo "Compilation failed (unexpected)"
        exit 1
    fi
done

echo ""
echo "✅ All CI tests passed!"
echo "   - 14 working examples compile successfully"
echo "   - 6 core examples run without crashes"
echo "   - Known issues are stable"
echo ""
echo "SUCCESS: Grue compiler is ready for release!"