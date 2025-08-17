#!/bin/bash
# Test to verify jump execution

echo "=== COMPILING TEST CASE ==="
RUST_LOG=error cargo run --bin grue-compiler -- debug_object_error.grue > /dev/null 2>&1

echo "=== BYTECODE LAYOUT ==="
echo "Main loop area:"
xxd -g 1 -l 32 -s 0x0726 debug_object_error.z3

echo "Init block area:" 
xxd -g 1 -l 16 -s 0x073e debug_object_error.z3

echo "=== EXECUTION TRACE ==="
echo "Following execution with detailed tracing..."
timeout 3s strace -e trace=write cargo run --bin gruesome debug_object_error.z3 2>&1 | grep -v "write(1" | head -20