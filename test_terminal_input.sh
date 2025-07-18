#!/bin/bash
# Test script to verify terminal input uses the timer-based path

echo "=== Testing Terminal Input Path ==="
echo "This will show debug logs for input handling"
echo "Type 'quit' and then 'y' to exit the game"
echo ""

RUST_LOG=debug ./target/debug/gruesome resources/test/zork1/DATA/ZORK1.DAT 2>&1 | grep -E "(timed_input|timer|interruptible|Terminal|Raw mode)"