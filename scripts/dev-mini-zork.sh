#!/bin/bash

# Development Build and Run Mini Zork Script
# Faster version using debug builds for development

set -e  # Exit on any error

echo "=== Development Build: Mini Zork ==="

# Build debug version (faster compilation)
echo "Building debug binaries..."
cargo build --bin gruesome --bin grue-compiler

echo "✅ Debug build completed"
echo ""

echo "=== Compiling Mini Zork (Debug) ==="

# Compile mini_zork.grue with error logging
echo "Compiling examples/mini_zork.grue..."
RUST_LOG=error cargo run --bin grue-compiler -- examples/mini_zork.grue

echo "✅ Mini Zork compiled"
echo ""

echo "=== Running Mini Zork (Debug) ==="
echo "Starting Mini Zork game with info logging..."
echo "Use Ctrl+C to quit"
echo ""

# Run with info logging to see game startup
RUST_LOG=info cargo run --bin gruesome mini_zork.z3