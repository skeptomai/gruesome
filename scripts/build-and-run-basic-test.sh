#!/bin/bash

# Build and Run Basic Test Script
# This script demonstrates the Grue compiler and interpreter with a simple test program

set -e  # Exit on any error

echo "=== Building Grue Compiler and Interpreter ==="

# Build all Rust binaries
echo "Building Rust binaries..."
cargo build --release --bin gruesome --bin grue-compiler

echo "✅ Build completed successfully"
echo ""

echo "=== Compiling Basic Test Game ==="

# Compile basic_test.grue to Z-Machine bytecode
echo "Compiling examples/basic_test.grue..."
cargo run --release --bin grue-compiler -- examples/basic_test.grue

echo "✅ Basic test compiled successfully"
echo ""

echo "=== Running Basic Test ==="
echo "Starting basic test game..."
echo "Use Ctrl+C to quit"
echo ""

# Run the compiled game
cargo run --release --bin gruesome basic_test.z3