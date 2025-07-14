#!/bin/bash

# Test the game with a simple command
echo "Testing Zork with interactive input..."
echo ""

# Send 'quit' command after initial display
echo -e "\nquit\n" | timeout 5 cargo run --release resources/test/zork1/DATA/ZORK1.DAT

echo ""
echo "Test completed."