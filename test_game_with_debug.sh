#!/bin/bash
# Run game with debug logging to see timer behavior

echo "Testing timer implementation..."
echo "Commands will:"
echo "1. Get the lantern"
echo "2. Turn it on" 
echo "3. Wait several turns"
echo "4. Check if timer fires"
echo

# Run with debug logging, filter for timer-related messages
cat << 'EOF' | RUST_LOG=gruesome=debug cargo run --bin gruesome 2>&1 | grep -E "(TIMER|Timer|G88|timer|lantern timer|sread.*4 operands)"
open mailbox
south
east
open window
west
west  
north
take lantern
turn on lantern
wait
wait
wait
wait
wait
quit
y
EOF