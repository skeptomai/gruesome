#!/bin/bash

# Layout Reordering Baseline Validation Script
#
# This script establishes baseline measurements before implementing layout changes
# Tests current functionality to ensure nothing breaks during reordering

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
BASELINE_DIR="$PROJECT_DIR/tests/layout_baseline_$TIMESTAMP"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

log_info() {
    echo -e "${BLUE}[BASELINE]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[BASELINE]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[BASELINE]${NC} $1"
}

log_error() {
    echo -e "${RED}[BASELINE]${NC} $1"
}

log_step() {
    echo -e "\n${YELLOW}=== $1 ===${NC}"
}

# Create baseline directory
mkdir -p "$BASELINE_DIR"
log_info "Created baseline directory: $BASELINE_DIR"

log_step "BASELINE VALIDATION: Current Layout State"
cd "$PROJECT_DIR"

# Test 1: Source function count
log_info "Step 1: Counting functions in mini_zork.grue source..."
SOURCE_FUNCTIONS=$(grep -c "^fn " examples/mini_zork.grue)
echo "$SOURCE_FUNCTIONS" > "$BASELINE_DIR/source_function_count.txt"
log_success "Source contains $SOURCE_FUNCTIONS functions"

# Test 2: Compile mini_zork
log_info "Step 2: Compiling mini_zork to establish current binary..."
BASELINE_FILE="$BASELINE_DIR/mini_zork_baseline.z3"
if cargo run --bin grue-compiler -- examples/mini_zork.grue -o "$BASELINE_FILE"; then
    log_success "Compilation successful: $(basename "$BASELINE_FILE")"
    FILE_SIZE=$(stat -c%s "$BASELINE_FILE" 2>/dev/null || stat -f%z "$BASELINE_FILE" 2>/dev/null)
    echo "$FILE_SIZE" > "$BASELINE_DIR/file_size.txt"
    log_info "File size: $FILE_SIZE bytes"
else
    log_error "Compilation failed!"
    exit 1
fi

# Test 3: Memory layout analysis
log_info "Step 3: Analyzing current memory layout..."
LAYOUT_FILE="$BASELINE_DIR/memory_layout.txt"
{
    echo "=== CURRENT MEMORY LAYOUT ==="
    echo "Analysis Date: $(date)"
    echo "File: $BASELINE_FILE"
    echo "File Size: $FILE_SIZE bytes"
    echo ""

    echo "Header Analysis:"
    echo "  Initial PC (0x06-0x07): $(xxd -s 0x06 -l 2 -p "$BASELINE_FILE")"
    echo "  Dictionary (0x08-0x09): $(xxd -s 0x08 -l 2 -p "$BASELINE_FILE")"
    echo "  Objects (0x0a-0x0b): $(xxd -s 0x0a -l 2 -p "$BASELINE_FILE")"
    echo "  Globals (0x0c-0x0d): $(xxd -s 0x0c -l 2 -p "$BASELINE_FILE")"
    echo "  Static mem (0x0e-0x0f): $(xxd -s 0x0e -l 2 -p "$BASELINE_FILE")"

    echo ""
    echo "Address Calculations:"
    INITIAL_PC=$(python3 -c "import struct; data = open('$BASELINE_FILE', 'rb').read(); print(f'0x{struct.unpack(\">H\", data[0x06:0x08])[0]:04x}')")
    DICT_ADDR=$(python3 -c "import struct; data = open('$BASELINE_FILE', 'rb').read(); print(f'0x{struct.unpack(\">H\", data[0x08:0x0a])[0]:04x}')")
    STATIC_MEM=$(python3 -c "import struct; data = open('$BASELINE_FILE', 'rb').read(); print(f'0x{struct.unpack(\">H\", data[0x0e:0x10])[0]:04x}')")

    echo "  Initial PC starts at: $INITIAL_PC"
    echo "  Dictionary starts at: $DICT_ADDR"
    echo "  Static memory starts at: $STATIC_MEM"

    # Calculate gap between dictionary and code
    DICT_DECIMAL=$(python3 -c "print(int('$DICT_ADDR', 16))")
    PC_DECIMAL=$(python3 -c "print(int('$INITIAL_PC', 16))")
    GAP=$((PC_DECIMAL - DICT_DECIMAL))
    echo "  Gap between dictionary and code: $GAP bytes"

} > "$LAYOUT_FILE"
log_success "Memory layout analysis saved"

# Test 4: Disassembler functionality
log_info "Step 4: Testing current disassembler functionality..."
DISASM_FILE="$BASELINE_DIR/disassembler_output.txt"
if ./target/release/gruedasm-txd "$BASELINE_FILE" > "$DISASM_FILE" 2>&1; then
    ROUTINE_COUNT=$(grep -c "^Routine " "$DISASM_FILE" || echo 0)
    echo "$ROUTINE_COUNT" > "$BASELINE_DIR/routine_count.txt"
    log_warning "Disassembler found $ROUTINE_COUNT routines (expected $SOURCE_FUNCTIONS)"
else
    echo "0" > "$BASELINE_DIR/routine_count.txt"
    log_error "Disassembler failed to run"
fi

# Test 5: Build interpreter and test game functionality
log_info "Step 5: Building interpreter and testing game functionality..."
if ! [ -f "./target/release/gruesome" ]; then
    log_info "Building release interpreter..."
    cargo build --release --bin gruesome
fi

GAME_TEST_FILE="$BASELINE_DIR/game_test.txt"
if printf "look\ninventory\nquit\ny\n" | timeout 10s ./target/release/gruesome "$BASELINE_FILE" > "$GAME_TEST_FILE" 2>&1; then
    log_success "Game runs successfully"
    GAME_FUNCTIONAL="true"
else
    log_error "Game functionality test failed"
    GAME_FUNCTIONAL="false"
fi
echo "$GAME_FUNCTIONAL" > "$BASELINE_DIR/game_functional.txt"

# Generate baseline summary
SUMMARY_FILE="$BASELINE_DIR/BASELINE_SUMMARY.md"
{
    echo "# Layout Reordering Baseline Summary"
    echo ""
    echo "**Generated**: $(date)"
    echo "**Purpose**: Establish baseline measurements before layout reordering"
    echo "**File**: $(basename "$BASELINE_FILE")"
    echo ""

    echo "## Current State Analysis"
    echo ""
    echo "| Metric | Current Value | Expected After Reordering |"
    echo "|--------|---------------|---------------------------|"
    echo "| Source Functions | $SOURCE_FUNCTIONS | $SOURCE_FUNCTIONS (unchanged) |"
    echo "| Detected Routines | $ROUTINE_COUNT | $SOURCE_FUNCTIONS (fixed) |"
    echo "| File Size | $FILE_SIZE bytes | ~$FILE_SIZE bytes (similar) |"
    echo "| Game Functional | $GAME_FUNCTIONAL | true (preserved) |"
    echo "| Dictionary-Code Gap | $GAP bytes | ~300 bytes (standard) |"
    echo ""

    echo "## Issues Identified"
    echo ""
    if [ "$ROUTINE_COUNT" -lt "$SOURCE_FUNCTIONS" ]; then
        echo "❌ **Disassembler Issue**: Only $ROUTINE_COUNT routines detected, should be $SOURCE_FUNCTIONS"
        echo "   - **Cause**: Non-standard memory layout with large dictionary-code gap"
        echo "   - **Fix**: Reorder layout to match commercial Z-Machine standard"
    else
        echo "✅ **Disassembler**: Correctly detects all routines"
    fi

    if [ "$GAME_FUNCTIONAL" = "true" ]; then
        echo "✅ **Game Functionality**: Working correctly with our interpreter"
    else
        echo "❌ **Game Functionality**: Not working correctly"
    fi

    echo ""
    echo "## Reordering Plan Validation"
    echo ""
    echo "**Current Layout**: Header → Globals → Objects → Dictionary → Strings → Code"
    echo "**Target Layout**: Header → Globals → Objects → Dictionary → Code → Strings"
    echo ""
    echo "**Success Criteria for Reordering**:"
    echo "- ✅ Source functions: $SOURCE_FUNCTIONS (must be preserved)"
    echo "- ❌ Detected routines: $SOURCE_FUNCTIONS (must reach this target)"
    echo "- ✅ Game functionality: Must remain working"
    echo "- ✅ File size: Should remain similar (~$FILE_SIZE bytes)"
    echo "- ✅ Dictionary-code gap: Should reduce to ~300 bytes"

} > "$SUMMARY_FILE"

# Display summary
cat "$SUMMARY_FILE"

log_success "🎯 BASELINE VALIDATION COMPLETE"
log_info "Baseline directory: $BASELINE_DIR"
log_info "Summary report: $SUMMARY_FILE"

# Return status based on what we found
if [ "$ROUTINE_COUNT" -lt "$SOURCE_FUNCTIONS" ] && [ "$GAME_FUNCTIONAL" = "true" ]; then
    log_warning "⚠️  READY FOR REORDERING: Disassembler issue confirmed, game functional"
    exit 0
elif [ "$ROUTINE_COUNT" -eq "$SOURCE_FUNCTIONS" ] && [ "$GAME_FUNCTIONAL" = "true" ]; then
    log_success "✅ ALREADY WORKING: No reordering needed"
    exit 0
else
    log_error "❌ BASELINE ISSUES: Fix game functionality first"
    exit 1
fi