#!/bin/bash
# Trace AMFV status line operations

echo "Tracing AMFV status line..."
echo "Wait for the game to start, then press Ctrl+C"
echo ""

# Run the game and capture all operations after the opening
DISPLAY_LOG=1 RUST_LOG=info timeout 30 ./target/release/gruesome resources/test/amfv/amfv-r79-s851122.z4 2>&1 | tee amfv_trace.log | grep -E "OP" | tail -200