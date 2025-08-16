#!/bin/bash

# CI Check Script - Exactly matches GitHub Actions CI workflow
# Run this before pushing to ensure CI will pass

set -e  # Exit on any error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
print_step() {
    echo -e "\n${BLUE}━━━ $1 ━━━${NC}"
}

print_success() {
    echo -e "${GREEN}✓${NC} $1"
}

print_error() {
    echo -e "${RED}✗${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}⚠${NC} $1"
}

# Check if we're in the right directory
if [ ! -f "Cargo.toml" ]; then
    print_error "Please run this script from the project root directory"
    exit 1
fi

echo -e "${BLUE}╔════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║       GitHub Actions CI Check          ║${NC}"
echo -e "${BLUE}╚════════════════════════════════════════╝${NC}"

# Set environment variables to match CI
export CARGO_TERM_COLOR=always
export CI=true

# Track overall status
FAILED=0

# 1. Format check (only on Ubuntu in CI)
print_step "Checking code formatting"
if cargo fmt -- --check 2>/dev/null; then
    print_success "Code formatting is correct"
else
    print_error "Code formatting issues detected. Run 'cargo fmt' to fix."
    FAILED=1
fi

# 2. Clippy check (only on Ubuntu in CI)
print_step "Running clippy linter"
if cargo clippy -- -D warnings >/dev/null 2>&1; then
    print_success "Clippy passed with no warnings"
else
    print_error "Clippy found issues. Fix them before pushing."
    cargo clippy -- -D warnings
    FAILED=1
fi

# 3. Run tests (all platforms in CI)
print_step "Running tests"
if cargo test --verbose >/dev/null 2>&1; then
    print_success "All tests passed"
else
    print_error "Some tests failed"
    cargo test --verbose
    FAILED=1
fi

# 4. Build binary (all platforms in CI)
print_step "Building debug binary"
if cargo build --verbose --bin gruesome >/dev/null 2>&1; then
    print_success "Debug build successful"
else
    print_error "Debug build failed"
    cargo build --verbose --bin gruesome
    FAILED=1
fi

# 5. Build disassembler
print_step "Building disassembler"
if cargo build --verbose --bin gruedasm-txd >/dev/null 2>&1; then
    print_success "Disassembler build successful"
else
    print_error "Disassembler build failed"
    cargo build --verbose --bin gruedasm-txd
    FAILED=1
fi

# 6. Build Grue compiler
print_step "Building Grue compiler"
if cargo build --verbose --bin grue-compiler >/dev/null 2>&1; then
    print_success "Grue compiler build successful"
else
    print_error "Grue compiler build failed"
    cargo build --verbose --bin grue-compiler
    FAILED=1
fi

# 7. Test Grue compiler examples  
print_step "Testing Grue compiler examples"

# First run key examples test (property fixes validation)
if ./scripts/test_key_examples.sh >/dev/null 2>&1; then
    print_success "Key examples working (property fixes validated)"
else
    print_error "Key examples test failed"
    ./scripts/test_key_examples.sh
    FAILED=1
fi

# Then run full CI test suite
if ./scripts/ci_test.sh >/dev/null 2>&1; then
    print_success "All Grue compiler examples working"
else
    print_error "Grue compiler examples test failed"
    echo "Running detailed test to show failures:"
    ./scripts/ci_test.sh
    FAILED=1
fi

# 8. Run integration tests
print_step "Running integration tests"
if cargo test --test integration_gameplay >/dev/null 2>&1; then
    print_success "Gameplay integration tests passed"
else
    print_error "Gameplay integration tests failed"
    cargo test --test integration_gameplay
    FAILED=1
fi

if cargo test --test integration_disasm >/dev/null 2>&1; then
    print_success "Disassembler integration tests passed"
else
    print_error "Disassembler integration tests failed"
    cargo test --test integration_disasm
    FAILED=1
fi

# 8. Optional: Test Windows cross-compilation (separate job in CI)
print_step "Cross-compilation check (optional)"
if [[ "$OSTYPE" == "linux-gnu"* ]]; then
    if command -v x86_64-w64-mingw32-gcc >/dev/null 2>&1; then
        rustup target add x86_64-pc-windows-gnu 2>/dev/null || true
        if cargo build --target x86_64-pc-windows-gnu --bin gruesome >/dev/null 2>&1; then
            print_success "Windows cross-compilation successful"
        else
            print_warning "Windows cross-compilation failed (optional)"
        fi
    else
        print_warning "mingw-w64 not installed (install with: sudo apt-get install mingw-w64)"
    fi
elif [[ "$OSTYPE" == "darwin"* ]]; then
    print_warning "Cross-compilation test skipped on macOS (CI only runs it on Linux)"
fi

# Summary
echo -e "\n${BLUE}════════════════════════════════════════${NC}"
if [ $FAILED -eq 0 ]; then
    echo -e "${GREEN}✅ All CI checks passed!${NC}"
    echo -e "${GREEN}Your code is ready to push.${NC}"
else
    echo -e "${RED}❌ CI checks failed!${NC}"
    echo -e "${RED}Please fix the issues above before pushing.${NC}"
    exit 1
fi