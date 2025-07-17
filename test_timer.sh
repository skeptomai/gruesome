#!/bin/bash
# Test timer functionality by looking at lantern

echo "look
inventory
wait
wait
wait
wait
wait
quit
y" | RUST_LOG=debug ./run_game.sh 2>&1 | grep -E "(sread with timer|Timer routine|G88|lantern|Lantern)"