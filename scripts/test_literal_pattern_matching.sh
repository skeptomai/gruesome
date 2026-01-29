#!/bin/bash

# Literal Pattern Matching Regression Test
#
# Tests the fix for mixed 1-byte/2-byte branch encoding bug (Dec 14, 2025)
# that prevented literal patterns like "look around" from executing correctly.
#
# Background:
# - December 11-12: Refactoring introduced mixed branch encoding
# - Bug symptom: "look around" returned "I don't understand" instead of executing
# - December 14: Fixed systematic 2-byte encoding enforcement
# - This test validates the fix

set -euo pipefail

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
TESTS_DIR="$PROJECT_DIR/tests"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
TEST_GAME="$TESTS_DIR/literal_pattern_test_${TIMESTAMP}.z3"

# Test commands that exercise the bug fix
TEST_COMMANDS="look
look around
look at mailbox
examine carefully
quit"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Logging functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

log_step() {
    echo -e "\n${YELLOW}=== $1 ===${NC}"
}

# Main test execution
main() {
    log_info "Literal Pattern Matching Regression Test"
    log_info "Testing fix for branch encoding bug (Dec 14, 2025)"

    cd "$PROJECT_DIR"

    # Step 1: Compile test game
    log_step "Compiling Test Game"
    log_info "Source: tests/integration/test_literal_pattern_matching.grue"
    log_info "Output: $(basename "$TEST_GAME")"

    if cargo run --bin grue-compiler -- tests/integration/test_literal_pattern_matching.grue -o "$TEST_GAME"; then
        log_success "Test game compiled successfully"
    else
        log_error "Test game compilation failed"
        exit 1
    fi

    # Step 2: Run test game and capture output
    log_step "Running Test Game"

    local output_file="${TEST_GAME%.z3}_output.txt"
    local clean_output="${TEST_GAME%.z3}_clean.txt"

    if timeout 10s bash -c "echo '$TEST_COMMANDS' | cargo run --bin gruesome -- '$TEST_GAME'" > "$output_file" 2>&1; then
        log_success "Test game executed successfully"
    else
        local exit_code=$?
        if [ $exit_code -eq 124 ]; then
            log_error "Test timed out after 10 seconds"
        else
            log_error "Test execution failed with exit code $exit_code"
        fi
        exit 1
    fi

    # Strip ANSI codes for easier parsing
    sed 's/\x1B\[[0-9;]*[JKmsu]//g' "$output_file" > "$clean_output"

    # Step 3: Verify outputs
    log_step "Verifying Pattern Matching"

    local test_passed=true
    local failures=0

    # Test 1: Verb-only pattern "look"
    if grep -q "DEFAULT: You glance around briefly" "$clean_output"; then
        log_success "✓ Verb-only pattern 'look' executed correctly"
    else
        log_error "✗ Verb-only pattern 'look' failed"
        test_passed=false
        ((failures++))
    fi

    # Test 2: Literal pattern "look around"
    if grep -q "LITERAL: You look around carefully" "$clean_output"; then
        log_success "✓ Literal pattern 'look around' executed correctly"
    else
        log_error "✗ Literal pattern 'look around' failed (THIS WAS THE BUG!)"
        test_passed=false
        ((failures++))
    fi

    # Test 3: Verify literal pattern doesn't fall through
    local look_default_count=$(grep -c "DEFAULT: You glance around briefly" "$clean_output" || true)
    if [ "$look_default_count" -eq 1 ]; then
        log_success "✓ Literal pattern doesn't fall through to verb-only"
    else
        log_error "✗ Literal pattern fell through (found $look_default_count verb-only responses)"
        test_passed=false
        ((failures++))
    fi

    # Test 4: Literal+noun pattern "look at mailbox"
    if grep -q "NOUN: You examine the target" "$clean_output"; then
        log_success "✓ Literal+noun pattern 'look at X' executed correctly"
    else
        log_error "✗ Literal+noun pattern 'look at X' failed"
        test_passed=false
        ((failures++))
    fi

    # Test 5: Different verb literal pattern "examine carefully"
    if grep -q "LITERAL: You examine things carefully" "$clean_output"; then
        log_success "✓ Different verb literal pattern executed correctly"
    else
        log_error "✗ Different verb literal pattern failed"
        test_passed=false
        ((failures++))
    fi

    # Final results
    log_step "Test Results"

    if [ "$test_passed" = true ]; then
        log_success "🎉 ALL TESTS PASSED (5/5)"
        log_info "Literal pattern matching works correctly with 2-byte branch encoding"
        log_info "Output saved to: $(basename "$clean_output")"

        # Clean up successful test artifacts
        rm -f "$TEST_GAME" "$output_file" "$clean_output"
        exit 0
    else
        log_error "❌ TESTS FAILED ($failures failures)"
        log_error "This indicates a regression in literal pattern matching"
        log_info "Full output saved to: $(basename "$clean_output")"
        log_info "Review output to diagnose issue"
        exit 1
    fi
}

# Execute main function
main "$@"
