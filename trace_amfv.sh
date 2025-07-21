#!/bin/bash
# Trace AMFV display operations

echo "Tracing AMFV display operations..."
echo "Press Ctrl+C after the opening screen appears"
echo ""

DISPLAY_LOG=1 RUST_LOG=info timeout 10 ./target/release/gruesome resources/test/amfv/amfv-r79-s851122.z4 2>&1 | grep -E "(OP|PART I)" | head -50