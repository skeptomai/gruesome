#!/bin/bash

# Comprehensive Mini Zork Disassembler Testing Protocol
#
# This script implements comprehensive testing of gruedasm-txd against mini_zork including:
# - Clean build of disassembler in debug and release modes
# - Game compilation with both debug and release compilers
# - Disassembly execution on all build combinations
# - Output capture and verification

set -euo pipefail

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
TESTS_DIR="$PROJECT_DIR/tests"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
RESULTS_DIR="$TESTS_DIR/disasm_mini_zork_results_$TIMESTAMP"

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

    local debug_game="$TESTS_DIR/mini_zork_debug_disasm_${TIMESTAMP}.z3"
    local release_game="$TESTS_DIR/mini_zork_release_disasm_${TIMESTAMP}.z3"

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

# Function to run disassembler and capture output
run_disassembler_test() {
    local disassembler_type="$1"  # "debug" or "release"
    local game_type="$2"          # "debug" or "release"
    local disassembler_path="$3"
    local game_path="$4"

    local test_name="${disassembler_type}_disasm_${game_type}_game"
    local output_file="$RESULTS_DIR/${test_name}_output.txt"
    local error_file="$RESULTS_DIR/${test_name}_errors.txt"

    log_info "Running $test_name test..."

    # Run the disassembler with output capture
    if timeout 60s "$disassembler_path" "$game_path" > "$output_file" 2> "$error_file"; then
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

    # Extract key metrics from disassembly output
    local total_lines=$(wc -l < "$output_file" | xargs)
    local function_count=$(grep -c "^Routine " "$output_file" 2>/dev/null || echo 0)
    local opcode_count=$(grep -c "^\s*[0-9a-fA-F]*:\s*[0-9a-fA-F]*\|PRINT_\|STORE_\|LOAD_\|CALL_" "$output_file" 2>/dev/null || echo 0)
    local string_count=$(grep -c "PRINT_RET\|PRINT_" "$output_file" 2>/dev/null || echo 0)
    local object_count=$(grep -c "PRINT_OBJ\|G[0-9]" "$output_file" 2>/dev/null || echo 0)
    local room_count=$(grep -c "Room " "$output_file" 2>/dev/null || echo 0)

    # Check for expected content indicators
    local success_indicators=0
    if grep -q "Routine R0\|^Routine " "$output_file"; then ((success_indicators++)); fi
    if grep -q "PRINT_\|print" "$output_file"; then ((success_indicators++)); fi
    if grep -q "G[0-9]\|yourself" "$output_file"; then ((success_indicators++)); fi
    if [ "$function_count" -gt 0 ]; then ((success_indicators++)); fi
    if [ "$opcode_count" -ge 2 ]; then ((success_indicators++)); fi

    # Check for errors (ignore debug-level failures)
    local error_count=$(wc -l < "$error_file" | xargs)
    local has_critical_errors=0
    if grep -q -i "panic\|abort\|fatal" "$error_file" 2>/dev/null; then
        has_critical_errors=1
    fi

    # Generate test summary
    {
        echo "=== $test_name Test Summary ==="
        echo "Timestamp: $(date)"
        echo "Disassembler: $disassembler_path"
        echo "Game File: $game_path"
        echo "Total Output Lines: $total_lines"
        echo "Functions Found: $function_count"
        echo "Opcodes Found: $opcode_count"
        echo "Strings Found: $string_count"
        echo "Objects Found: $object_count"
        echo "Rooms Found: $room_count"
        echo "Error Lines: $error_count"
        echo "Success Indicators: $success_indicators/5"
        echo ""

        if [ "$success_indicators" -eq 5 ] && [ "$has_critical_errors" -eq 0 ] && [ "$total_lines" -gt 10 ]; then
            echo "STATUS: PASSED ✓"
        else
            echo "STATUS: FAILED ✗"
        fi
        echo ""

        echo "Content Checklist:"
        echo "- Routines found: $(grep -q "Routine R0\|^Routine " "$output_file" && echo "✓" || echo "✗")"
        echo "- Print instructions: $(grep -q "PRINT_\|print" "$output_file" && echo "✓" || echo "✗")"
        echo "- Objects/globals: $(grep -q "G[0-9]\|yourself" "$output_file" && echo "✓" || echo "✗")"
        echo "- Function count > 0: $([[ $function_count -gt 0 ]] && echo "✓" || echo "✗")"
        echo "- Opcode count >= 2: $([[ $opcode_count -ge 2 ]] && echo "✓" || echo "✗")"
        echo "- No critical errors: $([[ $has_critical_errors -eq 0 ]] && echo "✓" || echo "✗")"

    } > "$RESULTS_DIR/${test_name}_summary.txt"

    log_info "  Lines: $total_lines, Functions: $function_count, Opcodes: $opcode_count, Indicators: $success_indicators/5"
}

# Step 4: Run comprehensive disassembler tests
run_all_disassembler_tests() {
    log_step "STEP 4: Run Comprehensive Disassembler Tests"
    cd "$PROJECT_DIR"

    local debug_game=$(cat "$RESULTS_DIR/debug_game_path.txt")
    local release_game=$(cat "$RESULTS_DIR/release_game_path.txt")

    local debug_disassembler="./target/debug/gruedasm-txd"
    local release_disassembler="./target/release/gruedasm-txd"

    # Test all combinations
    log_info "Testing all disassembler/game combinations..."

    # Debug disassembler + Debug game
    run_disassembler_test "debug" "debug" "$debug_disassembler" "$debug_game"

    # Debug disassembler + Release game
    run_disassembler_test "debug" "release" "$debug_disassembler" "$release_game"

    # Release disassembler + Debug game
    run_disassembler_test "release" "debug" "$release_disassembler" "$debug_game"

    # Release disassembler + Release game
    run_disassembler_test "release" "release" "$release_disassembler" "$release_game"
}

# Step 5: Generate comprehensive report
generate_final_report() {
    log_step "STEP 5: Generate Final Report"

    local report_file="$RESULTS_DIR/MINI_ZORK_DISASM_REPORT.md"
    local passed_tests=0
    local total_tests=4

    {
        echo "# Mini Zork Disassembler Test Protocol Report"
        echo ""
        echo "**Generated:** $(date)"
        echo "**Project:** $(basename "$PROJECT_DIR")"
        echo "**Git Commit:** $(cd "$PROJECT_DIR" && git rev-parse --short HEAD 2>/dev/null || echo "UNKNOWN")"
        echo ""

        echo "## Test Configuration"
        echo ""
        echo "- **Source Game:** examples/mini_zork.grue"
        echo "- **Debug Game:** $(basename "$(cat "$RESULTS_DIR/debug_game_path.txt")")"
        echo "- **Release Game:** $(basename "$(cat "$RESULTS_DIR/release_game_path.txt")")"
        echo "- **Disassembler:** gruedasm-txd (debug and release builds)"
        echo "- **Expected Content:** Functions, opcodes, strings, objects"
        echo ""

        echo "## Test Results"
        echo ""

        # Process each test result
        for test_type in "debug_disasm_debug_game" "debug_disasm_release_game" "release_disasm_debug_game" "release_disasm_release_game"; do
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

                local lines=$(grep "Total Output Lines:" "$summary_file" | cut -d':' -f2 | xargs)
                local functions=$(grep "Functions Found:" "$summary_file" | cut -d':' -f2 | xargs)
                local opcodes=$(grep "Opcodes Found:" "$summary_file" | cut -d':' -f2 | xargs)
                local strings=$(grep "Strings Found:" "$summary_file" | cut -d':' -f2 | xargs)
                local indicators=$(grep "Success Indicators:" "$summary_file" | cut -d':' -f2 | xargs)

                echo "- **Output Lines:** $lines"
                echo "- **Functions:** $functions"
                echo "- **Opcodes:** $opcodes"
                echo "- **Strings:** $strings"
                echo "- **Success Indicators:** $indicators"

                echo ""
                echo "**Content Checklist:**"
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
            echo "🎉 **DISASSEMBLER VERIFICATION SUCCESSFUL**"
            echo ""
            echo "All build combinations (debug/release disassembler × debug/release game) completed"
            echo "successfully with expected disassembly output. The gruedasm-txd disassembler is"
            echo "functioning correctly across all tested configurations."
        else
            echo "**Overall Status:** ❌ SOME TESTS FAILED"
            echo ""
            echo "⚠️ **DISASSEMBLER ISSUES DETECTED**"
            echo ""
            echo "Some build combinations failed to produce expected disassembly output."
            echo "Review individual test outputs and error files for details."
        fi

        echo ""
        echo "## Files Generated"
        echo ""
        echo "- **Disassembly Outputs:** \`*_output.txt\` files with complete disassembly"
        echo "- **Error Logs:** \`*_errors.txt\` files with stderr capture"
        echo "- **Test Summaries:** \`*_summary.txt\` files with metrics and verification"
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
    log_info "Starting Mini Zork Disassembler Test Protocol"
    log_info "Timestamp: $TIMESTAMP"

    create_results_dir
    clean_build
    build_all_versions
    compile_game_files
    run_all_disassembler_tests
    generate_final_report

    log_success "Mini Zork disassembler test protocol completed!"
    log_info "Results directory: $RESULTS_DIR"
    log_info "View report: $RESULTS_DIR/MINI_ZORK_DISASM_REPORT.md"
}

# Execute main function
main "$@"