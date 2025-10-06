#!/bin/bash
# Test script for Mini Zork - compiles and runs the game

echo "Recompiling mini_zork.grue..."
env RUST_LOG=warn cargo run --bin grue-compiler -- examples/mini_zork.grue -o /tmp/mini_zork_test.z3 2>&1 | tail -3

echo ""
echo "Starting Mini Zork..."
echo "Try commands like: inventory, north, south, east, west, open mailbox, take leaflet, read leaflet"
echo ""

env DISPLAY_MODE=terminal ./target/debug/gruesome /tmp/mini_zork_test.z3
