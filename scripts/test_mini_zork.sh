#!/bin/bash

# Script to compile and run mini_zork interactively
# Usage: ./test_mini_zork.sh

set -e  # Exit on any error

echo "🔨 Compiling mini_zork.grue..."
cargo run --bin grue-compiler -- ../examples/mini_zork.grue -o ../tests/mini_zork_test.z3

echo "🎮 Starting mini_zork interactive session..."
echo "   Commands to try:"
echo "   - look"
echo "   - examine mailbox"
echo "   - open mailbox"
echo "   - north"
echo "   - quit"
echo ""

# Run the interpreter interactively
../target/debug/gruesome ../tests/mini_zork_test.z3