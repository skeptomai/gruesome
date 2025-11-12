#!/bin/bash

# Comprehensive Zork I Testing Protocol
#
# This script implements the complete Zork I gameplay test protocol including:
# - Clean build of interpreter in debug and release modes
# - Full gameplay protocol execution on both build versions
# - Output capture and verification against commercial game standards

set -euo pipefail

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
TESTS_DIR="$PROJECT_DIR/tests"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
RESULTS_DIR="$TESTS_DIR/zork1_results_$TIMESTAMP"
GAME_FILE="$PROJECT_DIR/resources/test/zork1/DATA/ZORK1.DAT"

# Zork I test commands from protocol
GAME_COMMANDS="north
east
open window
enter
take bag
take bottle
leave
score
inventory
quit
y"

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

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

log_step() {
    echo -e "\n${YELLOW}=== $1 ===${NC}"
}

# Create results directory
create_results_dir() {
    mkdir -p "$RESULTS_DIR"
    log_info "Created results directory: $RESULTS_DIR"
}

# Step 1: Clean build
clean_build() {
    log_step "STEP 1: Clean Build"
    cd "$PROJECT_DIR"

    log_info "Running cargo clean..."
    cargo clean

    log_success "Build artifacts cleaned"
}

# Step 2: Build debug and release interpreters
build_interpreters() {
    log_step "STEP 2: Build Debug and Release Interpreters"
    cd "$PROJECT_DIR"

    # Build debug version
    log_info "Building debug interpreter..."
    if cargo build --bin gruesome; then
        log_success "Debug interpreter build completed"
    else
        log_error "Debug interpreter build failed"
        exit 1
    fi

    # Build release version
    log_info "Building release interpreter..."
    if cargo build --bin gruesome --release; then
        log_success "Release interpreter build completed"
    else
        log_error "Release interpreter build failed"
        exit 1
    fi
}

# Step 3: Verify game file exists
verify_game_file() {
    log_step "STEP 3: Verify Zork I Game File"

    if [ ! -f "$GAME_FILE" ]; then
        log_error "Zork I game file not found at: $GAME_FILE"
        exit 1
    else
        log_success "Zork I game file found: $GAME_FILE"
        log_info "Game file size: $(stat -f%z "$GAME_FILE" 2>/dev/null || stat -c%s "$GAME_FILE" 2>/dev/null || echo "unknown") bytes"
    fi
}

# Function to run Zork I gameplay protocol and capture output
run_zork1_test() {
    local interpreter_type="$1"  # "debug" or "release"
    local interpreter_path="$2"

    local test_name="${interpreter_type}_interpreter"
    local output_file="$RESULTS_DIR/${test_name}_output.txt"
    local clean_output_file="$RESULTS_DIR/${test_name}_clean.txt"

    log_info "Running $test_name test against Zork I..."

    # Run the game with full output capture
    if timeout 120s bash -c "echo '$GAME_COMMANDS' | '$interpreter_path' '$GAME_FILE'" > "$output_file" 2>&1; then
        log_success "$test_name completed successfully"
    else
        local exit_code=$?
        if [ $exit_code -eq 124 ]; then
            log_warning "$test_name timed out after 120 seconds"
        else
            log_error "$test_name failed with exit code $exit_code"
        fi
        return 1
    fi

    # Create clean output (strip ANSI codes for easier reading)
    sed 's/\x1B\[[0-9;]*[JKmsu]//g' "$output_file" > "$clean_output_file"

    # Extract key metrics from Zork I protocol
    local score_final=$(grep -o "Your score is [0-9]*" "$clean_output_file" | tail -1 | grep -o "[0-9]*" || echo "UNKNOWN")
    local moves_mentioned=$(grep -o "in [0-9]* moves" "$clean_output_file" | tail -1 | grep -o "[0-9]*" || echo "UNKNOWN")

    # Check for successful completion indicators from Zork I protocol
    local success_indicators=0
    if grep -q "North of House" "$clean_output_file"; then ((success_indicators++)); fi
    if grep -q "With great effort, you open the window" "$clean_output_file"; then ((success_indicators++)); fi
    if grep -q "Kitchen" "$clean_output_file"; then ((success_indicators++)); fi
    if grep -q "Taken" "$clean_output_file"; then ((success_indicators++)); fi
    if grep -q "Your score is" "$clean_output_file"; then ((success_indicators++)); fi
    if grep -q "You are carrying:" "$clean_output_file"; then ((success_indicators++)); fi

    # Generate test summary
    {
        echo "=== $test_name Test Summary ==="
        echo "Timestamp: $(date)"
        echo "Interpreter: $interpreter_path"
        echo "Game File: $GAME_FILE"
        echo "Final Score: $score_final"
        echo "Moves at Score: $moves_mentioned"
        echo "Success Indicators: $success_indicators/6"
        echo ""

        if [ "$success_indicators" -ge 5 ] && [ "$score_final" -eq 10 ]; then
            echo "STATUS: PASSED ✓"
        else
            echo "STATUS: FAILED ✗"
        fi
        echo ""

        echo "Protocol Checklist:"
        echo "- Navigation (North of House): $(grep -q "North of House" "$clean_output_file" && echo "✓" || echo "✗")"
        echo "- Window interaction: $(grep -q "With great effort, you open the window" "$clean_output_file" && echo "✓" || echo "✗")"
        echo "- Kitchen entry: $(grep -q "Kitchen" "$clean_output_file" && echo "✓" || echo "✗")"
        echo "- Object taking: $(grep -q "Taken" "$clean_output_file" && echo "✓" || echo "✗")"
        echo "- Score display: $(grep -q "Your score is" "$clean_output_file" && echo "✓" || echo "✗")"
        echo "- Inventory display: $(grep -q "You are carrying:" "$clean_output_file" && echo "✓" || echo "✗")"

    } > "$RESULTS_DIR/${test_name}_summary.txt"

    log_info "  Score: $score_final, Moves: $moves_mentioned, Indicators: $success_indicators/6"
}

# Step 4: Run Zork I gameplay tests
run_all_zork1_tests() {
    log_step "STEP 4: Run Zork I Gameplay Tests"
    cd "$PROJECT_DIR"

    local debug_interpreter="./target/debug/gruesome"
    local release_interpreter="./target/release/gruesome"

    # Test both interpreter versions
    log_info "Testing both interpreter versions against Zork I..."

    # Debug interpreter
    run_zork1_test "debug" "$debug_interpreter"

    # Release interpreter
    run_zork1_test "release" "$release_interpreter"
}

# Step 5: Generate comprehensive report
generate_final_report() {
    log_step "STEP 5: Generate Final Report"

    local report_file="$RESULTS_DIR/ZORK1_TEST_REPORT.md"
    local passed_tests=0
    local total_tests=2

    {
        echo "# Zork I Comprehensive Test Protocol Report"
        echo ""
        echo "**Generated:** $(date)"
        echo "**Project:** $(basename "$PROJECT_DIR")"
        echo "**Git Commit:** $(cd "$PROJECT_DIR" && git rev-parse --short HEAD 2>/dev/null || echo "UNKNOWN")"
        echo ""

        echo "## Test Configuration"
        echo ""
        echo "- **Game File:** $(basename "$GAME_FILE")"
        echo "- **Game Size:** $(stat -f%z "$GAME_FILE" 2>/dev/null || stat -c%s "$GAME_FILE" 2>/dev/null || echo "unknown") bytes"
        echo "- **Commands:** 10 commands (north → quit)"
        echo "- **Expected Score:** 10 points"
        echo "- **Expected Moves:** 7-8 moves"
        echo ""

        echo "## Test Results"
        echo ""

        # Process each test result
        for test_type in "debug_interpreter" "release_interpreter"; do
            local summary_file="$RESULTS_DIR/${test_type}_summary.txt"

            if [ -f "$summary_file" ]; then
                echo "### $(echo "$test_type" | tr '_' ' ' | sed 's/.*/\u&/')"
                echo ""

                local status=$(grep "STATUS:" "$summary_file" | cut -d' ' -f2-)
                if [[ "$status" == *"PASSED"* ]]; then
                    echo "**Status:** ✅ PASSED"
                    ((passed_tests++))
                else
                    echo "**Status:** ❌ FAILED"
                fi

                local score=$(grep "Final Score:" "$summary_file" | cut -d':' -f2 | xargs)
                local moves=$(grep "Moves at Score:" "$summary_file" | cut -d':' -f2 | xargs)
                local indicators=$(grep "Success Indicators:" "$summary_file" | cut -d':' -f2 | xargs)

                echo "- **Final Score:** $score"
                echo "- **Moves at Score:** $moves"
                echo "- **Success Indicators:** $indicators"

                echo ""
                echo "**Protocol Checklist:**"
                grep -E "^- " "$summary_file" | while read -r line; do
                    echo "$line"
                done
                echo ""
            else
                echo "### $(echo "$test_type" | tr '_' ' ' | sed 's/.*/\u&/')"
                echo ""
                echo "**Status:** ❌ TEST NOT COMPLETED"
                echo ""
            fi
        done

        echo "## Overall Results"
        echo ""
        echo "**Tests Passed:** $passed_tests/$total_tests"

        if [ "$passed_tests" -eq "$total_tests" ]; then
            echo "**Overall Status:** ✅ ALL TESTS PASSED"
            echo ""
            echo "🎉 **ZORK I COMPATIBILITY VERIFIED**"
            echo ""
            echo "Both debug and release interpreters successfully executed the complete Zork I"
            echo "test protocol, demonstrating full commercial game compatibility. The Z-Machine"
            echo "interpreter correctly handles Infocom's original 1981-1983 game format."
        else
            echo "**Overall Status:** ❌ SOME TESTS FAILED"
            echo ""
            echo "⚠️ **COMPATIBILITY ISSUES DETECTED**"
            echo ""
            echo "Some interpreter versions failed to complete the Zork I protocol successfully."
            echo "Review individual test outputs for detailed failure analysis."
        fi

        echo ""
        echo "## Files Generated"
        echo ""
        # Fix: Use proper backtick escaping for markdown to prevent shell interpretation
        echo "- **Raw Outputs:** \`*_output.txt\` files with complete game session logs"
        echo "- **Clean Outputs:** \`*_clean.txt\` files with ANSI codes stripped"
        echo "- **Test Summaries:** \`*_summary.txt\` files with protocol checklist"
        echo ""
        echo "All files are located in: \`$RESULTS_DIR\`"

    } > "$report_file"

    log_success "Final report generated: $report_file"

    if [ "$passed_tests" -eq "$total_tests" ]; then
        log_success "🎉 ALL TESTS PASSED ($passed_tests/$total_tests)"
        log_success "Zork I commercial game compatibility verified!"
    else
        log_warning "⚠️  Some tests failed ($passed_tests/$total_tests passed)"
    fi
}

# Main execution
main() {
    log_info "Starting Zork I Comprehensive Test Protocol"
    log_info "Timestamp: $TIMESTAMP"

    create_results_dir
    clean_build
    build_interpreters
    verify_game_file
    run_all_zork1_tests
    generate_final_report

    log_success "Zork I test protocol completed!"
    log_info "Results directory: $RESULTS_DIR"
    log_info "View report: $RESULTS_DIR/ZORK1_TEST_REPORT.md"
}

# Execute main function
main "$@"