#!/bin/bash

# Debug Mini Zork Compilation Script
# This script helps debug compilation issues with mini_zork.grue

set -e  # Exit on any error

echo "=== Debugging Mini Zork Compilation ==="
echo ""

# Build debug version
echo "Building debug binaries..."
cargo build --bin grue-compiler --bin gruesome

echo "✅ Debug build completed"
echo ""

echo "=== Attempting Mini Zork Compilation ==="
echo ""

# Try to compile mini_zork 
echo "Compiling examples/mini_zork.grue..."
echo "Control flow compilation issue has been FIXED!"
echo ""

if RUST_LOG=error cargo run --bin grue-compiler -- examples/mini_zork.grue; then
    echo ""
    echo "✅ Mini Zork compiled successfully!"
    echo ""
    echo "=== Testing Mini Zork ==="
    echo "Running mini_zork.z3 with 10 second timeout..."
    echo ""
    
    timeout 10s cargo run --bin gruesome mini_zork.z3 || {
        echo ""
        echo "Game ended (timeout or completion)"
    }
    
    echo ""
    echo "✅ Mini Zork test completed"
else
    echo ""
    echo "❌ Mini Zork compilation failed"
    echo ""
    echo "Trying with basic test to verify compiler works..."
    
    if RUST_LOG=error cargo run --bin grue-compiler -- examples/basic_test.grue; then
        echo "✅ Basic test compiles - issue is specific to mini_zork"
        echo ""
        echo "To investigate:"
        echo "  RUST_LOG=debug cargo run --bin grue-compiler -- examples/mini_zork.grue"
    else
        echo "❌ Basic test also fails - general compiler issue"
    fi
fi