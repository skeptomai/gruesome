#!/bin/bash

# Script that shows the ACTUAL output from the test games
# No hiding, no claims - just raw output so you can verify yourself

set -e

echo "=== RAW OUTPUT FROM OBJECT ITERATION TESTS ==="
echo

echo "TEST 1: Original minimal reproduction case"
echo "File: ../examples/minimal_object_iteration_repro.grue"
echo "BEFORE FIX: Would crash with 'Stack underflow at PC 0x07d4'"
echo "ACTUAL OUTPUT NOW:"
echo "----------------------------------------"
cargo run --bin grue-compiler -- ../examples/minimal_object_iteration_repro.grue -o ../tests/show_output1.z3 > /dev/null 2>&1
../target/debug/gruesome ../tests/show_output1.z3
echo "----------------------------------------"
echo

echo "TEST 2: Empty container test with both direct and indirect iteration"
echo "File: ../examples/test_empty_container.grue"
echo "ACTUAL OUTPUT:"
echo "----------------------------------------"
cargo run --bin grue-compiler -- ../examples/test_empty_container.grue -o ../tests/show_output2.z3 > /dev/null 2>&1
../target/debug/gruesome ../tests/show_output2.z3
echo "----------------------------------------"
echo

echo "TEST 3: Isolation test - just calling get_object_contents()"
echo "File: ../examples/test_get_contents_only.grue"
echo "ACTUAL OUTPUT:"
echo "----------------------------------------"
cargo run --bin grue-compiler -- ../examples/test_get_contents_only.grue -o ../tests/show_output3.z3 > /dev/null 2>&1
../target/debug/gruesome ../tests/show_output3.z3
echo "----------------------------------------"
echo

echo "=== END RAW OUTPUT ==="
echo
echo "What you should see above:"
echo "- All tests complete without crashing"
echo "- No 'Invalid object number: 1000' errors"
echo "- No 'Stack underflow' errors"
echo "- Test 3 should show 'SUCCESS: Empty container correctly returned 0'"
echo
echo "If you see the above, the fix is working. If you see crashes or errors, it's not."