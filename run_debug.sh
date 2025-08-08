#!/bin/bash
echo "Building with debug statements..."
cargo build

echo ""
echo "Running game with debug output to debug.log..."
echo "Type 'w' at the prompt to trigger the bug"
echo ""

RUST_LOG=debug cargo run --bin gruesome resources/test/zork1/DATA/ZORK1.DAT 2>debug.log