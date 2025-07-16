#!/bin/bash
echo "Building game.."
cargo build --release --bin infocom

echo ""
echo "Running game.."
echo ""

./target/release/infocom resources/test/zork1/DATA/ZORK1.DAT
