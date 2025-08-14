#!/bin/bash

# Simple test script without colors
echo "=== Grue Compiler Test Results ==="
echo ""

PASSED=0
FAILED=0

test_file() {
    local file="$1"
    local basename=$(basename "$file" .grue)
    local z3_file="${basename}.z3"
    
    echo -n "Testing $file... "
    
    # Compile
    if ! cargo run --bin grue-compiler -- "$file" >/dev/null 2>&1; then
        echo "COMPILATION FAILED"
        FAILED=$((FAILED + 1))
        return
    fi
    
    # Run
    local output=$(timeout 5s cargo run --bin gruesome "$z3_file" 2>&1 || true)
    
    if echo "$output" | grep -q "Stack underflow\|Invalid object\|Failed to decode\|Error:"; then
        echo "RUNTIME FAILED"
        FAILED=$((FAILED + 1))
    else
        echo "PASSED"
        PASSED=$((PASSED + 1))
    fi
}

# Test all files
for file in examples/*.grue debug_*.grue; do
    if [[ -f "$file" ]]; then
        test_file "$file"
    fi
done

echo ""
echo "Results: $PASSED passed, $FAILED failed"