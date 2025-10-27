#!/bin/bash

# Script to compile and run object iteration tests
# This verifies the get_object_contents fix is working correctly

set -e  # Exit on any error

echo "=== Testing Object Iteration Fix ==="
echo

echo "1. Testing empty container (original minimal repro)..."
cargo run --bin grue-compiler -- examples/minimal_object_iteration_repro.grue -o tests/minimal_object_iteration_repro_test.z3
echo "Expected: No errors, should complete gracefully"
echo "Actual output:"
./target/debug/gruesome tests/minimal_object_iteration_repro_test.z3

echo
echo "2. Testing container with actual items..."
cargo run --bin grue-compiler -- examples/test_actual_iteration.grue -o tests/test_actual_iteration.z3
echo "Expected: Should list 'coin', 'brass key', 'spell book' twice (direct + indirect)"
echo "Actual output:"
./target/debug/gruesome tests/test_actual_iteration.z3

echo
echo "=== Both tests completed ==="