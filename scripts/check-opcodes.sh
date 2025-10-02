#!/bin/bash
# Check for dangerous opcode patterns in the codebase
# This script helps prevent opcode-related bugs by catching suspicious patterns

set -e

echo "🔍 Checking for potentially incorrect opcode values..."

# Colors for output
RED='\033[0;31m'
YELLOW='\033[1;33m'
GREEN='\033[0;32m'
NC='\033[0m' # No Color

# Find emit_instruction calls with suspicious opcode values (0x20-0x7F)
# These are likely encoded bytes instead of raw opcodes
# Note: We check for 0x20-0x7F because:
# - 0x00-0x1F are valid raw opcodes
# - 0x20-0x7F are invalid (neither raw opcodes nor valid encoded forms)
# - 0x80+ are valid for SHORT form encoding, so we allow them
suspicious=$(grep -rn "emit_instruction.*0x[2-7][0-9A-Fa-f]," src/grue_compiler/ --include="*.rs" 2>/dev/null | grep -v "//" || true)

if [ -n "$suspicious" ]; then
    echo -e "${RED}⚠️  WARNING: Found emit_instruction calls with suspicious opcode values:${NC}"
    echo "$suspicious"
    echo ""
    echo "Opcodes should be raw numbers (0x00-0x1F), not encoded instruction bytes."
    echo "Example: Use 0x00 for call_vs, not 0xE0 or 0x20"
    echo ""
    echo -e "${YELLOW}These may be false positives if the values are in comments or other contexts.${NC}"
    echo -e "${YELLOW}Please review each occurrence carefully.${NC}"
    exit 1
fi

# Check for hardcoded opcode values instead of using the opcodes module
hardcoded=$(grep -rn "emit_instruction.*0x[0-1][0-9A-Fa-f]," src/grue_compiler/ --include="*.rs" 2>/dev/null | grep -v "opcodes::" | grep -v "//" || true)

if [ -n "$hardcoded" ]; then
    echo -e "${YELLOW}ℹ️  INFO: Found hardcoded opcode values (consider using opcode constants from opcodes module):${NC}"
    echo "$hardcoded"
    echo ""
    echo "Using named constants improves code readability:"
    echo "  use crate::grue_compiler::opcodes::var;"
    echo "  emit_instruction(var::CALL_VS, ...) // instead of 0x00"
    echo ""
fi

echo -e "${GREEN}✅ Opcode validation passed${NC}"
exit 0
