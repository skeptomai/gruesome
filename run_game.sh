#!/bin/bash
echo "Building game.."
cargo build --release --bin gruesome

echo ""
echo "Running game.."
echo ""

./target/release/gruesome resources/test/zork1/DATA/ZORK1.DAT
