#!/bin/bash

# Test key examples after property fixes
# Quick validation script for critical functionality

set -e

echo "🔍 Testing Key Examples After Property Fixes"
echo "=============================================="

# Key examples to test
EXAMPLES=(
    "test_01_basic.grue"
    "test_02_multiprint.grue"
    "test_03_function.grue"
    "test_04_room.grue"
    "property_test.grue"
    "mini_zork.grue"
    "control_flow_simple.grue"
)

# Non-interactive examples that should exit cleanly
NON_INTERACTIVE=(
    "test_01_basic.grue"
    "test_02_multiprint.grue"
    "test_03_function.grue"
)

PASSED=0
FAILED=0

for example in "${EXAMPLES[@]}"; do
    echo -n "Testing $example... "
    
    # Compile
    if ! RUST_LOG=error timeout 10s cargo run --bin grue-compiler -- "examples/$example" >/dev/null 2>&1; then
        echo "❌ COMPILE FAILED"
        ((FAILED++))
        continue
    fi
    
    # Get output file name
    basename=$(basename "$example" .grue)
    
    # Runtime test - only for non-interactive games
    # Interactive games wait for input and will timeout
    is_interactive=true
    for non_interactive in "${NON_INTERACTIVE[@]}"; do
        if [ "$example" = "$non_interactive" ]; then
            is_interactive=false
            break
        fi
    done

    if [ "$is_interactive" = false ]; then
        if ! RUST_LOG=error timeout 3s cargo run --bin gruesome "$basename.z3" >/dev/null 2>&1; then
            echo "❌ RUNTIME FAILED"
            ((FAILED++))
            continue
        fi
    else
        echo -n "(interactive, compilation only) "
    fi
    
    echo "✅ PASSED"
    ((PASSED++))
done

echo ""
echo "📊 Results: $PASSED passed, $FAILED failed"

if [ $FAILED -eq 0 ]; then
    echo "🎉 All key examples passed!"
    exit 0
else
    echo "⚠️  Some examples failed"
    exit 1
fi