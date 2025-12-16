#!/bin/bash

# Comprehensive Mini Zork Testing Protocol
#
# This script implements the complete Mini Zork gameplay test protocol including:
# - Clean build of all components
# - Debug and release builds of compiler and interpreter
# - Game compilation with both debug and release compilers
# - Full gameplay protocol execution on all build combinations
# - Output capture and verification

set -euo pipefail

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
TESTS_DIR="$PROJECT_DIR/tests"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
RESULTS_DIR="$TESTS_DIR/protocol_results_$TIMESTAMP"

# Game commands for complete protocol test
GAME_COMMANDS="open mailbox
take leaflet
read leaflet
north
north
climb tree
take egg
down
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

# Step 2: Build debug and release versions
build_all_versions() {
    log_step "STEP 2: Build Debug and Release Versions"
    cd "$PROJECT_DIR"

    # Build debug version
    log_info "Building debug version..."
    if cargo build; then
        log_success "Debug build completed"
    else
        log_error "Debug build failed"
        exit 1
    fi

    # Build release version
    log_info "Building release version..."
    if cargo build --release; then
        log_success "Release build completed"
    else
        log_error "Release build failed"
        exit 1
    fi
}

# Step 3: Compile game files with both compilers
compile_game_files() {
    log_step "STEP 3: Compile Game Files"
    cd "$PROJECT_DIR"

    local debug_game="$TESTS_DIR/mini_zork_debug_${TIMESTAMP}.z3"
    local release_game="$TESTS_DIR/mini_zork_release_${TIMESTAMP}.z3"

    # Compile with debug compiler
    log_info "Compiling game with debug compiler..."
    if cargo run --bin grue-compiler -- examples/mini_zork.grue -o "$debug_game"; then
        log_success "Debug game compiled: $(basename "$debug_game")"
        echo "$debug_game" > "$RESULTS_DIR/debug_game_path.txt"
    else
        log_error "Debug game compilation failed"
        exit 1
    fi

    # Compile with release compiler
    log_info "Compiling game with release compiler..."
    if cargo run --bin grue-compiler --release -- examples/mini_zork.grue -o "$release_game"; then
        log_success "Release game compiled: $(basename "$release_game")"
        echo "$release_game" > "$RESULTS_DIR/release_game_path.txt"
    else
        log_error "Release game compilation failed"
        exit 1
    fi
}

# Function to run gameplay protocol and capture output
run_gameplay_test() {
    local interpreter_type="$1"  # "debug" or "release"
    local game_type="$2"         # "debug" or "release"
    local interpreter_path="$3"
    local game_path="$4"

    local test_name="${interpreter_type}_interpreter_${game_type}_game"
    local output_file="$RESULTS_DIR/${test_name}_output.txt"
    local clean_output_file="$RESULTS_DIR/${test_name}_clean.txt"

    log_info "Running $test_name test..."

    # Run the game with full output capture
    if timeout 60s bash -c "echo '$GAME_COMMANDS' | '$interpreter_path' '$game_path'" > "$output_file" 2>&1; then
        log_success "$test_name completed successfully"
    else
        local exit_code=$?
        if [ $exit_code -eq 124 ]; then
            log_warning "$test_name timed out after 60 seconds"
        else
            log_error "$test_name failed with exit code $exit_code"
        fi
        return 1
    fi

    # Create clean output (strip ANSI codes for easier reading)
    sed 's/\x1B\[[0-9;]*[JKmsu]//g' "$output_file" > "$clean_output_file"

    # Extract key metrics
    local score_final=$(grep -o "Score: [0-9]*" "$clean_output_file" | tail -1 | grep -o "[0-9]*" || echo "UNKNOWN")
    local moves_final=$(grep -o "Moves: [0-9]*" "$clean_output_file" | tail -1 | grep -o "[0-9]*" || echo "UNKNOWN")

    # Check for successful completion indicators
    local success_indicators=0
    if grep -q "Took leaflet" "$clean_output_file"; then ((success_indicators++)); fi
    if grep -q "You are the proud owner of a very special egg" "$clean_output_file"; then ((success_indicators++)); fi
    if grep -q "Your score is" "$clean_output_file"; then ((success_indicators++)); fi
    if grep -q "You are carrying:" "$clean_output_file"; then ((success_indicators++)); fi

    # Generate test summary
    {
        echo "=== $test_name Test Summary ==="
        echo "Timestamp: $(date)"
        echo "Interpreter: $interpreter_path"
        echo "Game File: $game_path"
        echo "Final Score: $score_final"
        echo "Final Moves: $moves_final"
        echo "Success Indicators: $success_indicators/4"
        echo ""

        if [ "$success_indicators" -eq 4 ] && [ "$score_final" -eq 7 ]; then
            echo "STATUS: PASSED ✓"
        else
            echo "STATUS: FAILED ✗"
        fi
        echo ""
    } > "$RESULTS_DIR/${test_name}_summary.txt"

    log_info "  Final Score: $score_final, Moves: $moves_final, Indicators: $success_indicators/4"
}

# Step 4: Run comprehensive gameplay tests
run_all_gameplay_tests() {
    log_step "STEP 4: Run Comprehensive Gameplay Tests"
    cd "$PROJECT_DIR"

    local debug_game=$(cat "$RESULTS_DIR/debug_game_path.txt")
    local release_game=$(cat "$RESULTS_DIR/release_game_path.txt")

    local debug_interpreter="./target/debug/gruesome"
    local release_interpreter="./target/release/gruesome"

    # Test all combinations
    log_info "Testing all interpreter/game combinations..."

    # Debug interpreter + Debug game
    run_gameplay_test "debug" "debug" "$debug_interpreter" "$debug_game"

    # Debug interpreter + Release game
    run_gameplay_test "debug" "release" "$debug_interpreter" "$release_game"

    # Release interpreter + Debug game
    run_gameplay_test "release" "debug" "$release_interpreter" "$debug_game"

    # Release interpreter + Release game
    run_gameplay_test "release" "release" "$release_interpreter" "$release_game"
}

# Step 5: Generate comprehensive report
generate_final_report() {
    log_step "STEP 5: Generate Final Report"

    local debug_game=$(cat "$RESULTS_DIR/debug_game_path.txt")
    local release_game=$(cat "$RESULTS_DIR/release_game_path.txt")
    local report_file="$RESULTS_DIR/COMPREHENSIVE_TEST_REPORT.md"
    local passed_tests=0
    local total_tests=4

    {
        echo "# Mini Zork Comprehensive Test Protocol Report"
        echo ""
        echo "**Generated:** $(date)"
        echo "**Project:** $(basename "$PROJECT_DIR")"
        echo "**Git Commit:** $(cd "$PROJECT_DIR" && git rev-parse --short HEAD 2>/dev/null || echo "UNKNOWN")"
        echo ""

        echo "## Test Configuration"
        echo ""
        echo "- **Debug Game:** $(basename "$debug_game")"
        echo "- **Release Game:** $(basename "$release_game")"
        echo "- **Game Commands:** 11 commands (open mailbox → quit)"
        echo "- **Expected Final Score:** 7"
        echo "- **Expected Final Moves:** 4"
        echo ""

        echo "## Test Results"
        echo ""

        # Process each test result
        for test_type in "debug_interpreter_debug_game" "debug_interpreter_release_game" "release_interpreter_debug_game" "release_interpreter_release_game"; do
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
                local moves=$(grep "Final Moves:" "$summary_file" | cut -d':' -f2 | xargs)
                local indicators=$(grep "Success Indicators:" "$summary_file" | cut -d':' -f2 | xargs)

                echo "- **Final Score:** $score"
                echo "- **Final Moves:** $moves"
                echo "- **Success Indicators:** $indicators"
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
            echo "🎉 **COMPREHENSIVE TEST PROTOCOL SUCCESSFUL**"
            echo ""
            echo "All build combinations (debug/release compiler × debug/release interpreter) completed"
            echo "successfully with expected gameplay outcomes. The codebase is functioning correctly"
            echo "across all tested configurations."
        else
            echo "**Overall Status:** ❌ SOME TESTS FAILED"
            echo ""
            echo "⚠️ **ISSUES DETECTED**"
            echo ""
            echo "Some build combinations failed to complete successfully. Review individual"
            echo "test outputs and summaries for details."
        fi

        echo ""
        echo "## Files Generated"
        echo ""
        echo "- **Raw Outputs:** \`*_output.txt\` files with complete game session logs"
        echo "- **Clean Outputs:** \`*_clean.txt\` files with ANSI codes stripped"
        echo "- **Test Summaries:** \`*_summary.txt\` files with key metrics"
        echo "- **Game Files:** Debug and release compiled game files"
        echo ""
        echo "All files are located in: \`$RESULTS_DIR\`"

    } > "$report_file"

    log_success "Final report generated: $report_file"

    if [ "$passed_tests" -eq "$total_tests" ]; then
        log_success "🎉 ALL TESTS PASSED ($passed_tests/$total_tests)"
    else
        log_warning "⚠️  Some tests failed ($passed_tests/$total_tests passed)"
    fi
}

# Main execution
main() {
    log_info "Starting Comprehensive Mini Zork Test Protocol"
    log_info "Timestamp: $TIMESTAMP"

    create_results_dir
    clean_build
    build_all_versions
    compile_game_files
    run_all_gameplay_tests
    generate_final_report

    log_success "Comprehensive test protocol completed!"
    log_info "Results directory: $RESULTS_DIR"
    log_info "View report: $RESULTS_DIR/COMPREHENSIVE_TEST_REPORT.md"
}

# Execute main function
main "$@"