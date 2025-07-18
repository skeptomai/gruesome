#!/bin/bash
# Test non-blocking input implementation

echo "=== Testing Non-Blocking Input ==="
echo "This test will show that the game uses true non-blocking I/O"
echo ""
echo "Instructions:"
echo "1. The game will start with debug logging"
echo "2. Look for 'Non-blocking input active' messages"
echo "3. Type some commands: look, quit, y"
echo "4. Notice character-by-character echo (raw mode)"
echo "5. Try arrow keys and backspace"
echo ""
echo "Press Enter to start..."
read

RUST_LOG=info ./target/debug/gruesome resources/test/zork1/DATA/ZORK1.DAT