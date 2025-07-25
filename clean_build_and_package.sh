#!/usr/bin/env bash
set -e  # Exit on any error

echo "🧹 Cleaning previous builds..."
cargo clean
rm -fr ./out
mkdir ./out

echo "📦 Preparing output directory..."
cd out

# Clean up any existing files (ignore errors if they don't exist)
rm -f gruesome.zip gruesome-windows.zip gruesome gruesome.exe ZORK1.DAT

# Copy test game for packaging
cp ../resources/test/zork1/DATA/ZORK1.DAT .

echo "🍎 Building macOS universal binary..."
# Build for both Apple Silicon and Intel Macs
cargo build --release --bin gruesome --target aarch64-apple-darwin --manifest-path ../Cargo.toml
cargo build --release --bin gruesome --target x86_64-apple-darwin --manifest-path ../Cargo.toml

# Create universal binary using lipo
lipo -create -output gruesome \
    ../target/x86_64-apple-darwin/release/gruesome \
    ../target/aarch64-apple-darwin/release/gruesome

# Package macOS version
zip gruesome.zip gruesome ZORK1.DAT
echo "✅ macOS build complete: gruesome.zip"

echo "🪟 Building Windows binary..."
# Build for Windows
cargo build --release --bin gruesome --target x86_64-pc-windows-gnu --manifest-path ../Cargo.toml
cp ../target/x86_64-pc-windows-gnu/release/gruesome.exe ./

# Package Windows version  
zip gruesome-windows.zip gruesome.exe ZORK1.DAT
echo "✅ Windows build complete: gruesome-windows.zip"

echo "🎉 All builds complete! Files in ./out/"
ls -la

# Return to project root
cd ..
