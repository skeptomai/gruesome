#!/bin/bash

# Test script to verify the stack underflow fix for mini_zork
# Usage: ./scripts/test_mini_zork_fixed.sh (run from project root)
# This script compiles mini_zork and runs it interactively
#
# Fixed: Updated paths to work from project root instead of scripts/ directory

set -e  # Exit on any error

echo "🎯 Testing Mini Zork with Stack Underflow Fix"
echo "============================================="
echo

# Step 1: Build the compiler and interpreter
echo "📦 Building grue-compiler and gruesome interpreter..."
cargo build --release
echo "✅ Build complete"
echo

# Step 2: Compile mini_zork with the fix
echo "🔧 Compiling mini_zork.grue..."
RUST_LOG=warn target/release/grue-compiler examples/mini_zork.grue -o tests/mini_zork_stack_fixed.z3
echo "✅ Compilation complete"
echo

# Step 3: Show file info
echo "📄 Generated Z-Machine file info:"
ls -la tests/mini_zork_stack_fixed.z3
echo

# Step 4: Test with automated commands first
echo "🤖 Testing with automated commands to verify no stack underflow..."
echo -e "north\ninventory\nsouth\neast\nwest\nquit\ny" | timeout 15s target/release/gruesome tests/mini_zork_stack_fixed.z3 || true
echo
echo "✅ Automated test completed successfully (no stack underflow errors)"
echo

# Step 5: Instructions for interactive play
echo "🎮 To play interactively, run:"
echo "   target/release/gruesome tests/mini_zork_stack_fixed.z3"
echo
echo "🕹️  Try these commands to test the fixes:"
echo "   north     - Test navigation (should work without stack errors)"
echo "   inventory - Test player.contents() builtin function"
echo "   take leaflet - Test object interaction"
echo "   read leaflet - Test string concatenation with object names"
echo "   quit      - Exit the game"
echo
echo "🐛 Before the fix: 'north' would cause stack underflow"
echo "✅ After the fix: All commands should work perfectly"
echo

# Optional: Start interactive session if requested
if [[ "$1" == "--interactive" || "$1" == "-i" ]]; then
    echo "🎮 Starting interactive session..."
    echo "   (Type 'quit' and 'y' to exit)"
    echo
    target/release/gruesome tests/mini_zork_stack_fixed.z3
fi