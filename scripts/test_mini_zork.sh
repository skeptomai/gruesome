#!/bin/bash

# Script to compile and run mini_zork interactively
# Usage: ./scripts/test_mini_zork.sh (run from project root)
#
# This script demonstrates the complete Grue compiler pipeline:
# 1. Compiles examples/mini_zork.grue to Z-Machine bytecode
# 2. Runs the compiled game interactively in the gruesome interpreter
#
# Fixed: Updated paths to work from project root instead of scripts/ directory

set -e  # Exit on any error

echo "🔨 Compiling mini_zork.grue..."
cargo run --bin grue-compiler -- examples/mini_zork.grue -o tests/mini_zork_test.z3

echo "🎮 Starting mini_zork interactive session..."
echo "   Commands to try:"
echo "   - look"
echo "   - examine mailbox"
echo "   - open mailbox"
echo "   - north"
echo "   - quit"
echo ""

# Run the interpreter interactively
target/debug/gruesome tests/mini_zork_test.z3