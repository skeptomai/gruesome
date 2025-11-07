#!/bin/bash

# Grue Language Support VS Code Extension Installer
# This script installs the Grue language extension for VS Code

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
EXT_DIR="$EXTENSIONS_DIR/gruesome-project.grue-language-support-1.0.0"

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
echo "1. Reload VS Code (Cmd/Ctrl + Shift + P → 'Developer: Reload Window')"
echo "2. Open any .grue file to see syntax highlighting"
echo ""
echo "🎯 Test with: examples/mini_zork.grue"
echo "📚 Documentation: README.md"

# Optional: Offer to open VS Code
read -p "🚀 Open VS Code now? (y/N): " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    if command -v code &> /dev/null; then
        echo "🔄 Reloading VS Code..."
        code --reload
    else
        echo "⚠️  VS Code 'code' command not found. Please reload VS Code manually."
    fi
fi

echo "🎉 Happy coding in Grue!"