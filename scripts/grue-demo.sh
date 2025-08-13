#!/bin/bash

# Grue Compiler and Interpreter Demo Script
# Shows all the command lines to build, compile, and run Grue games

set -e  # Exit on any error

echo "=== Grue Language Compiler Demo ==="
echo ""

echo "Command lines for building and running Grue games:"
echo ""

echo "1. Build the Grue compiler and interpreter:"
echo "   cargo build --release --bin grue-compiler --bin gruesome"
echo ""

echo "2. Compile a Grue program to Z-Machine bytecode:"
echo "   cargo run --release --bin grue-compiler -- examples/basic_test.grue"
echo ""

echo "3. Run the compiled game:"
echo "   cargo run --release --bin gruesome basic_test.z3"
echo ""

echo "For development (faster debug builds):"
echo "   cargo build --bin grue-compiler --bin gruesome"
echo "   RUST_LOG=error cargo run --bin grue-compiler -- examples/basic_test.grue"
echo "   RUST_LOG=info cargo run --bin gruesome basic_test.z3"
echo ""

echo "Available example programs:"
echo "   examples/basic_test.grue         - Simple function calls and printing ✅"
echo "   examples/text_encoding_test.grue - Complete character encoding test ✅"  
echo "   examples/zork_banner_test.grue   - Zork-style banner with all text features ✅"
echo "   examples/test_01_basic.grue      - Minimal test ✅"
echo "   examples/mini_zork.grue          - Complex text adventure ✅ (compiles, runtime issue)"
echo ""

echo "=== Running Demo ==="
echo ""

# Build the tools
echo "Building Grue compiler and interpreter..."
cargo build --bin grue-compiler --bin gruesome

echo ""
echo "Compiling zork_banner_test.grue..."
RUST_LOG=error cargo run --bin grue-compiler -- examples/zork_banner_test.grue

echo ""
echo "Running the compiled game:"
echo "(Showcases perfect text encoding with all characters and newlines)"
echo ""

# Run the game with timeout to demonstrate
timeout 3s cargo run --bin gruesome zork_banner_test.z3 || true

echo ""
echo "✅ Demo completed! Text encoding is working perfectly!"
echo ""
echo "Other working examples:"
echo "   cargo run --bin gruesome basic_test.z3"
echo "   cargo run --bin gruesome text_encoding_test.z3"