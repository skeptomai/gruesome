#!/bin/bash
# Run all tests for the Gruesome Z-Machine Interpreter project
# This includes unit tests, integration tests, golden file tests, and disassembler tests

set -e  # Exit on error

echo "=================================="
echo "Running All Gruesome Tests"
echo "=================================="
echo ""

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Run main test suite (unit tests + integration tests)
echo -e "${BLUE}Running main test suite...${NC}"
cargo test

echo ""
echo -e "${GREEN}✓ All tests passed!${NC}"
echo ""

# Show test summary
echo "=================================="
echo "Test Summary:"
echo "=================================="
echo "• Unit tests: All source file tests"
echo "• Golden file tests: Compilation verification"
echo "• Integration gameplay tests: Zork I & AMFV"
echo "• Disassembler tests: 3 tests (1 slow test ignored)"
echo ""
echo "To run the slow disassembler test (60+ seconds):"
echo "  cargo test -- --ignored"
echo ""
