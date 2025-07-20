#!/bin/bash
set -e

echo "=== Running CI-style tests locally ==="

echo "1. Checking formatting..."
cargo fmt -- --check

echo -e "\n2. Running clippy..."
cargo clippy -- -D warnings

echo -e "\n3. Running tests with default features (ratatui)..."
cargo test --verbose

echo -e "\n4. Running tests without ratatui..."
cargo test --verbose --no-default-features

echo -e "\n5. Building with default features..."
cargo build --verbose --bin gruesome

echo -e "\n6. Building without ratatui (release style)..."
cargo build --verbose --bin gruesome --no-default-features

echo -e "\n=== All CI checks passed! ==="