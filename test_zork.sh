#!/bin/bash

# Interactive Zork I test script
# Usage: ./test_zork.sh

echo "Starting Zork I with our Z-Machine interpreter..."
echo "Type commands interactively. Use 'quit' to exit."
echo "================================================"

# Build the interpreter first
cargo build --quiet

# Run Zork I interactively
RUST_LOG=warn ./target/debug/gruesome resources/test/zork1/DATA/ZORK1.DAT