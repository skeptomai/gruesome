#!/bin/bash

# Test All Example Files - Comprehensive Grue Compiler Validation
# This script tests compilation and runtime execution for all example files

set -e  # Exit on any error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Counters
TOTAL_TESTS=0
PASSED_TESTS=0
FAILED_TESTS=0

# Test results arrays
COMPILATION_FAILURES=()
RUNTIME_FAILURES=()
SUCCESSFUL_TESTS=()

echo -e "${BLUE}=== Grue Compiler Example Test Suite ===${NC}"
echo -e "Testing all example files for compilation and runtime execution"
echo ""

# Function to test a single file
test_file() {
    local file="$1"
    local basename=$(basename "$file" .grue)
    local z3_file="${basename}.z3"
    
    echo -n "Testing $file... "
    TOTAL_TESTS=$((TOTAL_TESTS + 1))
    
    # Step 1: Try to compile
    if ! cargo run --bin grue-compiler -- "$file" >/dev/null 2>&1; then
        echo -e "${RED}COMPILATION FAILED${NC}"
        COMPILATION_FAILURES+=("$file")
        FAILED_TESTS=$((FAILED_TESTS + 1))
        return 1
    fi
    
    # Step 2: Try to run (with timeout to prevent hanging)
    # Capture output to check for specific errors
    local runtime_output
    runtime_output=$(timeout 5s cargo run --bin gruesome "$z3_file" 2>&1 || true)
    
    if echo "$runtime_output" | grep -q "Stack underflow\|Invalid object number\|Failed to decode"; then
        echo -e "${YELLOW}RUNTIME FAILED${NC} ($(echo "$runtime_output" | grep -o "Stack underflow\|Invalid object number\|Failed to decode" | head -1))"
        RUNTIME_FAILURES+=("$file")
        FAILED_TESTS=$((FAILED_TESTS + 1))
        return 1
    elif echo "$runtime_output" | grep -q "Error"; then
        echo -e "${YELLOW}RUNTIME FAILED${NC} (Unknown error)"
        RUNTIME_FAILURES+=("$file")
        FAILED_TESTS=$((FAILED_TESTS + 1))
        return 1
    fi
    
    echo -e "${GREEN}PASSED${NC}"
    SUCCESSFUL_TESTS+=("$file")
    PASSED_TESTS=$((PASSED_TESTS + 1))
    return 0
}

# Test all example files
echo -e "${BLUE}Testing examples/ directory:${NC}"
for file in examples/*.grue; do
    if [[ -f "$file" ]]; then
        test_file "$file"
    fi
done

echo ""

# Test debug files
echo -e "${BLUE}Testing debug files:${NC}"
for file in debug_*.grue; do
    if [[ -f "$file" ]]; then
        test_file "$file"
    fi
done

echo ""
echo -e "${BLUE}=== Test Results Summary ===${NC}"
echo "Total tests: $TOTAL_TESTS"
echo -e "Passed: ${GREEN}$PASSED_TESTS${NC}"
echo -e "Failed: ${RED}$FAILED_TESTS${NC}"

if [[ ${#SUCCESSFUL_TESTS[@]} -gt 0 ]]; then
    echo ""
    echo -e "${GREEN}✅ Successful tests:${NC}"
    for test in "${SUCCESSFUL_TESTS[@]}"; do
        echo "  $test"
    done
fi

if [[ ${#COMPILATION_FAILURES[@]} -gt 0 ]]; then
    echo ""
    echo -e "${RED}❌ Compilation failures:${NC}"
    for test in "${COMPILATION_FAILURES[@]}"; do
        echo "  $test"
    done
fi

if [[ ${#RUNTIME_FAILURES[@]} -gt 0 ]]; then
    echo ""
    echo -e "${YELLOW}⚠️  Runtime failures:${NC}"
    for test in "${RUNTIME_FAILURES[@]}"; do
        echo "  $test"
    done
fi

echo ""
if [[ $FAILED_TESTS -eq 0 ]]; then
    echo -e "${GREEN}🎉 ALL TESTS PASSED! 🎉${NC}"
    exit 0
else
    echo -e "${RED}❌ $FAILED_TESTS tests failed${NC}"
    exit 1
fi