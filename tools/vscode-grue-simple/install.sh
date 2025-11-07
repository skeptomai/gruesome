#!/bin/bash

# Simplified Grue Language Support VS Code Extension Installer

set -e

echo "🎮 Installing Grue Language Support for VS Code..."

# Determine VS Code extensions directory based on OS
if [[ "$OSTYPE" == "darwin"* ]]; then
    # macOS
    EXTENSIONS_DIR="$HOME/.vscode/extensions"
elif [[ "$OSTYPE" == "msys" || "$OSTYPE" == "cygwin" ]]; then
    # Windows
    EXTENSIONS_DIR="$USERPROFILE/.vscode/extensions"
else
    # Linux and others
    EXTENSIONS_DIR="$HOME/.vscode/extensions"
fi

# Create extensions directory if it doesn't exist
mkdir -p "$EXTENSIONS_DIR"

# Extension directory name
EXT_DIR="$EXTENSIONS_DIR/grue-0.0.1"

echo "📁 Installing to: $EXT_DIR"

# Copy extension files
if [ -d "$EXT_DIR" ]; then
    echo "⚠️  Removing existing extension..."
    rm -rf "$EXT_DIR"
fi

mkdir -p "$EXT_DIR"
cp -r . "$EXT_DIR/"

echo "✅ Grue Language Support extension installed successfully!"
echo ""
echo "📋 Next steps:"
echo "1. Close VS Code completely and restart it"
echo "2. Open any .grue file to see syntax highlighting"
echo "3. Check bottom-right corner shows 'Grue' (not 'Plain Text')"
echo ""
echo "🎯 Test with: examples/mini_zork.grue"

echo "🎉 Happy coding in Grue!"