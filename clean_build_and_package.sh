#!/usr/bin/env sh

cargo clean
rm -fr ./out
mkdir ./out
pushd out

rm gruesome.zip
rm gruesome-windows.zip
rm gruesome
rm gruesome.exe
rm ZORK1.DAT

cp ../resources/test/zork1/DATA/ZORK1.DAT .
cargo build --release --bin gruesome --target aarch64-apple-darwin
cargo build --release --bin gruesome --target x86_64-apple-darwin
lipo -create -output gruesome ../target/x86_64-apple-darwin/release/gruesome ../target/aarch64-apple-darwin/release/gruesome
zip gruesome.zip gruesome ZORK1.DAT
cargo build --release --bin gruesome --target x86_64-pc-windows-gnu
cp ../target/x86_64-pc-windows-gnu/release/gruesome.exe ./
zip gruesome-windows.zip gruesome.exe ZORK1.DAT
