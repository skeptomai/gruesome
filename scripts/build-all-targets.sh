#!/bin/bash

# Build script for creating release binaries for multiple platforms
# This script should be run on macOS to create the universal binary

set -e

echo "=== Building Gruesome for multiple targets ==="

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Create output directory
OUTPUT_DIR="release-binaries"
mkdir -p "$OUTPUT_DIR"

# Function to check if a target is installed
check_target() {
    local target=$1
    if rustup target list --installed | grep -q "$target"; then
        echo -e "${GREEN}✓${NC} Target $target is installed"
        return 0
    else
        echo -e "${YELLOW}!${NC} Target $target is not installed"
        return 1
    fi
}

# Function to build for a target
build_target() {
    local target=$1
    local output_name=$2
    
    echo -e "\n${GREEN}Building for $target...${NC}"
    # Build without ratatui for maximum compatibility in releases
    if cargo build --release --target "$target" --bin gruesome --no-default-features; then
        local source_file="target/$target/release/gruesome"
        if [[ "$target" == *"windows"* ]]; then
            source_file="${source_file}.exe"
            output_name="${output_name}.exe"
        fi
        
        if [ -f "$source_file" ]; then
            cp "$source_file" "$OUTPUT_DIR/$output_name"
            echo -e "${GREEN}✓${NC} Built $output_name"
            return 0
        else
            echo -e "${RED}✗${NC} Build succeeded but binary not found at $source_file"
            return 1
        fi
    else
        echo -e "${RED}✗${NC} Build failed for $target"
        return 1
    fi
}

# Check and install required targets
echo -e "\n${YELLOW}Checking required targets...${NC}"

TARGETS=(
    "aarch64-apple-darwin"
    "x86_64-apple-darwin"
    "x86_64-pc-windows-gnu"
)

for target in "${TARGETS[@]}"; do
    if ! check_target "$target"; then
        echo "Installing $target..."
        rustup target add "$target"
    fi
done

# Build for each target
echo -e "\n${YELLOW}Building binaries...${NC}"

# macOS ARM64
if build_target "aarch64-apple-darwin" "gruesome-aarch64-apple-darwin"; then
    HAVE_ARM64=true
else
    HAVE_ARM64=false
fi

# macOS x86_64
if build_target "x86_64-apple-darwin" "gruesome-x86_64-apple-darwin"; then
    HAVE_X86_64=true
else
    HAVE_X86_64=false
fi

# Windows x86_64
build_target "x86_64-pc-windows-gnu" "gruesome-x86_64-pc-windows-gnu"

# Create universal binary if both macOS builds succeeded
if [[ "$HAVE_ARM64" == true ]] && [[ "$HAVE_X86_64" == true ]]; then
    echo -e "\n${YELLOW}Creating universal macOS binary...${NC}"
    
    if command -v lipo &> /dev/null; then
        lipo -create \
            "$OUTPUT_DIR/gruesome-aarch64-apple-darwin" \
            "$OUTPUT_DIR/gruesome-x86_64-apple-darwin" \
            -output "$OUTPUT_DIR/gruesome-universal-apple-darwin"
        
        chmod +x "$OUTPUT_DIR/gruesome-universal-apple-darwin"
        
        echo -e "${GREEN}✓${NC} Created universal binary"
        echo -e "\nUniversal binary info:"
        lipo -info "$OUTPUT_DIR/gruesome-universal-apple-darwin"
    else
        echo -e "${RED}✗${NC} lipo command not found. Are you running on macOS?"
    fi
else
    echo -e "${YELLOW}!${NC} Skipping universal binary creation (one or both macOS builds failed)"
fi

# List all created binaries
echo -e "\n${GREEN}=== Build complete ===${NC}"
echo "Created binaries in $OUTPUT_DIR/:"
ls -lh "$OUTPUT_DIR/"

# Create checksums
echo -e "\n${YELLOW}Creating checksums...${NC}"
cd "$OUTPUT_DIR"
if command -v shasum &> /dev/null; then
    shasum -a 256 gruesome-* > checksums.txt
    echo -e "${GREEN}✓${NC} Created checksums.txt"
elif command -v sha256sum &> /dev/null; then
    sha256sum gruesome-* > checksums.txt
    echo -e "${GREEN}✓${NC} Created checksums.txt"
else
    echo -e "${YELLOW}!${NC} No checksum utility found"
fi

echo -e "\n${GREEN}All done!${NC}"