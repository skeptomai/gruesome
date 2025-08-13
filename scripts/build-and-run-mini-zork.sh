#!/bin/bash

# Build and Run Mini Zork Script
# This script builds the Grue compiler and interpreter, compiles mini_zork.grue, and runs the game

set -e  # Exit on any error

echo "=== Building Grue Compiler and Interpreter ==="

# Build all Rust binaries
echo "Building Rust binaries..."
cargo build --release --bin gruesome --bin grue-compiler

echo "✅ Build completed successfully"
echo ""

echo "=== Compiling Mini Zork Game ==="

# Compile mini_zork.grue to Z-Machine bytecode
echo "Compiling examples/mini_zork.grue..."
cargo run --release --bin grue-compiler -- examples/mini_zork.grue

echo "✅ Mini Zork compiled successfully"
echo ""

echo "=== Running Mini Zork ==="
echo "Starting Mini Zork game..."
echo "Use Ctrl+C to quit"
echo ""

# Run the compiled game
cargo run --release --bin gruesome mini_zork.z3