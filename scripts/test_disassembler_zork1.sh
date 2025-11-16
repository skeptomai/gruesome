#!/bin/bash

# Comprehensive Zork I Disassembler Testing Protocol
#
# This script implements comprehensive testing of gruedasm-txd against commercial Zork I including:
# - Clean build of disassembler in debug and release modes
# - Disassembly execution on both build versions against original Zork I
# - Output capture and verification against commercial game standards
# - Commercial Z-Machine format compatibility verification

set -euo pipefail

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
TESTS_DIR="$PROJECT_DIR/tests"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
RESULTS_DIR="$TESTS_DIR/disasm_zork1_results_$TIMESTAMP"
GAME_FILE="$PROJECT_DIR/resources/test/zork1/DATA/ZORK1.DAT"

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

# Step 2: Build debug and release disassemblers
build_disassemblers() {
    log_step "STEP 2: Build Disassemblers"
    cd "$PROJECT_DIR"

    # Build debug version unless release-only
    if [ "$RELEASE_ONLY" = false ]; then
        log_info "Building debug disassembler..."
        if cargo build --bin gruedasm-txd; then
            log_success "Debug disassembler build completed"
        else
            log_error "Debug disassembler build failed"
            exit 1
        fi
    else
        log_info "Skipping debug disassembler build (--release-only specified)"
    fi

    # Build release version unless debug-only
    if [ "$DEBUG_ONLY" = false ]; then
        log_info "Building release disassembler..."
        if cargo build --bin gruedasm-txd --release; then
            log_success "Release disassembler build completed"
        else
            log_error "Release disassembler build failed"
            exit 1
        fi
    else
        log_info "Skipping release disassembler build (--debug-only specified)"
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

# Function to run Zork I disassembler and capture output
run_zork1_disasm_test() {
    local disassembler_type="$1"  # "debug" or "release"
    local disassembler_path="$2"
    local filter_flag="${3:-}"     # Optional: "--show-filter-rules"

    local test_name="${disassembler_type}_disasm"
    if [ -n "$filter_flag" ]; then
        test_name="${test_name}_filter_rules"
    fi

    local output_file="$RESULTS_DIR/${test_name}_output.txt"
    local error_file="$RESULTS_DIR/${test_name}_errors.txt"

    log_info "Running $test_name test against Zork I..."

    # Run the disassembler with output capture
    local cmd_args=("$disassembler_path")
    if [ -n "$filter_flag" ]; then
        cmd_args+=("$filter_flag")
    fi
    cmd_args+=("$GAME_FILE")

    if timeout 300s "${cmd_args[@]}" > "$output_file" 2> "$error_file"; then
        log_success "$test_name completed successfully"
    else
        local exit_code=$?
        if [ $exit_code -eq 124 ]; then
            log_warning "$test_name timed out after 300 seconds"
        else
            log_error "$test_name failed with exit code $exit_code"
        fi
        return 1
    fi

    # Extract key metrics from Zork I disassembly
    local total_lines=$(wc -l < "$output_file" | xargs)
    local function_count=$(grep -c "^Function " "$output_file" 2>/dev/null || echo 0)
    local opcode_count=$(grep -c "^\s*[0-9a-fA-F]*:\s*[0-9a-fA-F]*" "$output_file" 2>/dev/null || echo 0)
    local string_count=$(grep -c "String " "$output_file" 2>/dev/null || echo 0)
    local object_count=$(grep -c "Object " "$output_file" 2>/dev/null || echo 0)
    local room_count=$(grep -c "Room " "$output_file" 2>/dev/null || echo 0)
    local routine_count=$(grep -c "^Routine " "$output_file" 2>/dev/null || echo 0)

    # Check for commercial Zork I specific content indicators
    local success_indicators=0
    if grep -q "ZORK I\|Zork I" "$output_file"; then ((success_indicators++)); fi
    if grep -q "Infocom\|Copyright" "$output_file"; then ((success_indicators++)); fi
    if grep -q "Routine R0\|^Routine " "$output_file"; then ((success_indicators++)); fi
    if [ "$function_count" -gt 10 ] || [ "$routine_count" -gt 10 ]; then ((success_indicators++)); fi
    if [ "$string_count" -gt 10 ]; then ((success_indicators++)); fi
    if [ "$object_count" -gt 5 ]; then ((success_indicators++)); fi

    # Check for errors
    local error_count=$(wc -l < "$error_file" | xargs)
    local has_critical_errors=0
    if grep -q -i "error\|failed\|panic\|abort" "$error_file" 2>/dev/null; then
        has_critical_errors=1
    fi

    # Filter rule validation if enabled
    local filter_rules_count=0
    local filter_rules_ok=0
    if [ -n "$filter_flag" ]; then
        local total_routines=$(grep -c -E "(^Main routine|^Routine R)" "$output_file" 2>/dev/null || echo 0)
        filter_rules_count=$(grep -c "; Filter rules passed:" "$output_file" 2>/dev/null || echo 0)
        if [ "$total_routines" -eq "$filter_rules_count" ] && [ "$total_routines" -gt 0 ]; then
            filter_rules_ok=1
            ((success_indicators++))
        fi
    fi

    # Check for Z-Machine format compliance
    local format_indicators=0
    if grep -q "Header\|Z-Machine" "$output_file"; then ((format_indicators++)); fi
    if grep -q "Dictionary\|Vocab" "$output_file"; then ((format_indicators++)); fi
    if grep -q "Abbreviation\|Abbrev" "$output_file"; then ((format_indicators++)); fi

    # Generate test summary
    {
        echo "=== $test_name Test Summary ==="
        echo "Timestamp: $(date)"
        echo "Disassembler: $disassembler_path"
        echo "Game File: $GAME_FILE"
        echo "Total Output Lines: $total_lines"
        echo "Functions Found: $function_count"
        echo "Routines Found: $routine_count"
        echo "Opcodes Found: $opcode_count"
        echo "Strings Found: $string_count"
        echo "Objects Found: $object_count"
        echo "Rooms Found: $room_count"
        echo "Error Lines: $error_count"
        if [ -n "$filter_flag" ]; then
            echo "Filter Rules Found: $filter_rules_count"
            echo "Success Indicators: $success_indicators/7"
        else
            echo "Success Indicators: $success_indicators/6"
        fi
        echo "Format Indicators: $format_indicators/3"
        echo ""

        local expected_indicators=6
        local min_success_indicators=5
        if [ -n "$filter_flag" ]; then
            expected_indicators=7
            min_success_indicators=6
        fi

        if [ "$success_indicators" -ge "$min_success_indicators" ] && [ "$has_critical_errors" -eq 0 ] && [ "$total_lines" -gt 5000 ]; then
            echo "STATUS: PASSED ✓"
        else
            echo "STATUS: FAILED ✗"
        fi
        echo ""

        echo "Commercial Game Checklist:"
        echo "- Game title found: $(grep -q "ZORK I" "$output_file" && echo "✓" || echo "✗")"
        echo "- Publisher found: $(grep -q "Infocom" "$output_file" && echo "✓" || echo "✗")"
        echo "- Main function: $(grep -q "Function main\|Routine main" "$output_file" && echo "✓" || echo "✗")"
        echo "- Commercial scale functions: $([[ $function_count -gt 200 || $routine_count -gt 200 ]] && echo "✓" || echo "✗")"
        echo "- Sufficient strings: $([[ $string_count -gt 100 ]] && echo "✓" || echo "✗")"
        echo "- Sufficient objects: $([[ $object_count -gt 50 ]] && echo "✓" || echo "✗")"
        echo ""
        echo "Z-Machine Format Checklist:"
        echo "- Header/Z-Machine info: $(grep -q "Header\|Z-Machine" "$output_file" && echo "✓" || echo "✗")"
        echo "- Dictionary structure: $(grep -q "Dictionary\|Vocab" "$output_file" && echo "✓" || echo "✗")"
        echo "- Abbreviations table: $(grep -q "Abbreviation\|Abbrev" "$output_file" && echo "✓" || echo "✗")"
        echo "- No critical errors: $([[ $has_critical_errors -eq 0 ]] && echo "✓" || echo "✗")"
        if [ -n "$filter_flag" ]; then
            echo ""
            echo "Filter Transparency Checklist:"
            echo "- Filter rules match routines: $([[ $filter_rules_ok -eq 1 ]] && echo "✓ ($filter_rules_count rules)" || echo "✗ ($filter_rules_count rules)")"
        fi

    } > "$RESULTS_DIR/${test_name}_summary.txt"

    if [ -n "$filter_flag" ]; then
        log_info "  Lines: $total_lines, Routines: $routine_count, Filter Rules: $filter_rules_count, Indicators: $success_indicators/7"
    else
        log_info "  Lines: $total_lines, Functions: $function_count, Routines: $routine_count, Strings: $string_count, Indicators: $success_indicators/6"
    fi
}

# Step 4: Run Zork I disassembler tests
run_all_zork1_disasm_tests() {
    log_step "STEP 4: Run Zork I Disassembler Tests"
    cd "$PROJECT_DIR"

    local debug_disassembler="./target/debug/gruedasm-txd"
    local release_disassembler="./target/release/gruedasm-txd"

    # Test disassembler versions based on flags
    log_info "Testing disassembler versions against Zork I (debug-only=$DEBUG_ONLY, release-only=$RELEASE_ONLY, with-filter-rules=$WITH_FILTER_RULES)..."

    # Test debug disassembler unless release-only
    if [ "$RELEASE_ONLY" = false ]; then
        run_zork1_disasm_test "debug" "$debug_disassembler"
        if [ "$WITH_FILTER_RULES" = true ]; then
            run_zork1_disasm_test "debug" "$debug_disassembler" "--show-filter-rules"
        fi
    fi

    # Test release disassembler unless debug-only
    if [ "$DEBUG_ONLY" = false ]; then
        run_zork1_disasm_test "release" "$release_disassembler"
        if [ "$WITH_FILTER_RULES" = true ]; then
            run_zork1_disasm_test "release" "$release_disassembler" "--show-filter-rules"
        fi
    fi
}

# Step 5: Generate comprehensive report
generate_final_report() {
    log_step "STEP 5: Generate Final Report"

    local report_file="$RESULTS_DIR/ZORK1_DISASM_REPORT.md"
    local passed_tests=0
    local total_tests=2

    {
        echo "# Zork I Disassembler Test Protocol Report"
        echo ""
        echo "**Generated:** $(date)"
        echo "**Project:** $(basename "$PROJECT_DIR")"
        echo "**Git Commit:** $(cd "$PROJECT_DIR" && git rev-parse --short HEAD 2>/dev/null || echo "UNKNOWN")"
        echo ""

        echo "## Test Configuration"
        echo ""
        echo "- **Game File:** $(basename "$GAME_FILE")"
        echo "- **Game Size:** $(stat -f%z "$GAME_FILE" 2>/dev/null || stat -c%s "$GAME_FILE" 2>/dev/null || echo "unknown") bytes"
        echo "- **Disassembler:** gruedasm-txd (debug and release builds)"
        echo "- **Expected Content:** Commercial-scale functions, strings, objects, Z-Machine structures"
        echo "- **Timeout:** 300 seconds per test"
        echo ""

        echo "## Test Results"
        echo ""

        # Process each test result
        for test_type in "debug_disasm" "release_disasm"; do
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
                local routines=$(grep "Routines Found:" "$summary_file" | cut -d':' -f2 | xargs)
                local strings=$(grep "Strings Found:" "$summary_file" | cut -d':' -f2 | xargs)
                local objects=$(grep "Objects Found:" "$summary_file" | cut -d':' -f2 | xargs)
                local success_indicators=$(grep "Success Indicators:" "$summary_file" | cut -d':' -f2 | xargs)
                local format_indicators=$(grep "Format Indicators:" "$summary_file" | cut -d':' -f2 | xargs)

                echo "- **Output Lines:** $lines"
                echo "- **Functions:** $functions"
                echo "- **Routines:** $routines"
                echo "- **Strings:** $strings"
                echo "- **Objects:** $objects"
                echo "- **Success Indicators:** $success_indicators"
                echo "- **Format Indicators:** $format_indicators"

                echo ""
                echo "**Commercial Game Checklist:**"
                grep -A6 "Commercial Game Checklist:" "$summary_file" | grep -E "^- " | while read -r line; do
                    echo "$line"
                done

                echo ""
                echo "**Z-Machine Format Checklist:**"
                grep -A4 "Z-Machine Format Checklist:" "$summary_file" | grep -E "^- " | while read -r line; do
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
            echo "🎉 **COMMERCIAL Z-MACHINE COMPATIBILITY VERIFIED**"
            echo ""
            echo "Both debug and release disassemblers successfully processed the original Infocom"
            echo "Zork I game file, demonstrating full commercial Z-Machine format compatibility."
            echo "The gruedasm-txd disassembler correctly handles 1981-1983 Infocom game formats."
        else
            echo "**Overall Status:** ❌ SOME TESTS FAILED"
            echo ""
            echo "⚠️ **COMPATIBILITY ISSUES DETECTED**"
            echo ""
            echo "Some disassembler versions failed to process Zork I successfully. Review"
            echo "individual test outputs and error files for detailed failure analysis."
        fi

        echo ""
        echo "## Files Generated"
        echo ""
        echo "- **Disassembly Outputs:** \`*_output.txt\` files with complete Zork I disassembly"
        echo "- **Error Logs:** \`*_errors.txt\` files with stderr capture"
        echo "- **Test Summaries:** \`*_summary.txt\` files with commercial format verification"
        echo ""
        echo "All files are located in: \`$RESULTS_DIR\`"

    } > "$report_file"

    log_success "Final report generated: $report_file"

    if [ "$passed_tests" -eq "$total_tests" ]; then
        log_success "🎉 ALL TESTS PASSED ($passed_tests/$total_tests)"
        log_success "Commercial Zork I disassembly compatibility verified!"
    else
        log_warning "⚠️  Some tests failed ($passed_tests/$total_tests passed)"
    fi
}

# Parse command line arguments
parse_args() {
    NO_BUILD=false
    CLEAN_BUILD=false
    DEBUG_ONLY=false
    RELEASE_ONLY=false
    WITH_FILTER_RULES=false

    while [[ $# -gt 0 ]]; do
        case $1 in
            --no-build)
                NO_BUILD=true
                shift
                ;;
            --clean-build)
                CLEAN_BUILD=true
                shift
                ;;
            --debug-only)
                DEBUG_ONLY=true
                shift
                ;;
            --release-only)
                RELEASE_ONLY=true
                shift
                ;;
            --with-filter-rules)
                WITH_FILTER_RULES=true
                shift
                ;;
            --help|-h)
                echo "Usage: $0 [OPTIONS]"
                echo ""
                echo "Build Control:"
                echo "  --no-build          Skip all building, use existing binaries"
                echo "  --clean-build       Full clean + rebuild (default: incremental build)"
                echo ""
                echo "Build Selection:"
                echo "  --debug-only        Test only debug builds"
                echo "  --release-only      Test only release builds"
                echo "                      (default: test both debug and release)"
                echo ""
                echo "Feature Testing:"
                echo "  --with-filter-rules Include filter transparency testing"
                echo ""
                echo "Examples:"
                echo "  $0 --no-build --release-only --with-filter-rules"
                echo "  $0 --clean-build --with-filter-rules"
                echo "  $0 --debug-only"
                exit 0
                ;;
            *)
                log_error "Unknown option: $1"
                echo "Use --help for usage information"
                exit 1
                ;;
        esac
    done

    # Validation
    if [ "$DEBUG_ONLY" = true ] && [ "$RELEASE_ONLY" = true ]; then
        log_error "Cannot specify both --debug-only and --release-only"
        exit 1
    fi

    if [ "$NO_BUILD" = true ] && [ "$CLEAN_BUILD" = true ]; then
        log_error "Cannot specify both --no-build and --clean-build"
        exit 1
    fi
}

# Main execution
main() {
    parse_args "$@"

    log_info "Starting Zork I Disassembler Test Protocol"
    log_info "Timestamp: $TIMESTAMP"
    log_info "Configuration: no-build=$NO_BUILD, clean-build=$CLEAN_BUILD, debug-only=$DEBUG_ONLY, release-only=$RELEASE_ONLY, with-filter-rules=$WITH_FILTER_RULES"

    create_results_dir

    if [ "$NO_BUILD" = false ]; then
        if [ "$CLEAN_BUILD" = true ]; then
            clean_build
        fi
        build_disassemblers
    else
        log_info "Skipping build phase (--no-build specified)"
    fi

    verify_game_file
    run_all_zork1_disasm_tests
    generate_final_report

    log_success "Zork I disassembler test protocol completed!"
    log_info "Results directory: $RESULTS_DIR"
    log_info "View report: $RESULTS_DIR/ZORK1_DISASM_REPORT.md"
}

# Execute main function
main "$@"