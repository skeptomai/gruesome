#!/bin/bash

# Simplified Grue Language Support VS Code Extension Installer
# Automatically detects and installs to both VS Code and VS Code Insiders
# Fixed: November 7, 2025 - VS Code Insiders compatibility, proper extension structure

set -e

echo "🎮 Installing Grue Language Support for VS Code..."

# Function to install extension to a specific directory
install_to_directory() {
    local extensions_dir="$1"
    local variant_name="$2"

    if [ -d "$extensions_dir" ]; then
        echo "📁 Installing to $variant_name: $extensions_dir/vscode.grue-1.0.0"

        # Create extensions directory if it doesn't exist
        mkdir -p "$extensions_dir"

        # Extension directory name
        local ext_dir="$extensions_dir/vscode.grue-1.0.0"

        # Copy extension files
        if [ -d "$ext_dir" ]; then
            echo "⚠️  Removing existing $variant_name extension..."
            rm -rf "$ext_dir"
        fi

        mkdir -p "$ext_dir"
        # Copy only the necessary extension files, exclude install.sh
        cp package.json "$ext_dir/"
        cp language-configuration.json "$ext_dir/"
        cp -r syntaxes "$ext_dir/"

        echo "✅ Installed to $variant_name successfully!"
        return 0
    fi
    return 1
}

# Determine VS Code extensions directories based on OS
if [[ "$OSTYPE" == "darwin"* ]]; then
    # macOS
    VSCODE_DIR="$HOME/.vscode/extensions"
    INSIDERS_DIR="$HOME/.vscode-insiders/extensions"
elif [[ "$OSTYPE" == "msys" || "$OSTYPE" == "cygwin" ]]; then
    # Windows
    VSCODE_DIR="$USERPROFILE/.vscode/extensions"
    INSIDERS_DIR="$USERPROFILE/.vscode-insiders/extensions"
else
    # Linux and others
    VSCODE_DIR="$HOME/.vscode/extensions"
    INSIDERS_DIR="$HOME/.vscode-insiders/extensions"
fi

# Install to both VS Code and VS Code Insiders if they exist
INSTALLED_COUNT=0

if install_to_directory "$VSCODE_DIR" "VS Code"; then
    INSTALLED_COUNT=$((INSTALLED_COUNT + 1))
fi

if install_to_directory "$INSIDERS_DIR" "VS Code Insiders"; then
    INSTALLED_COUNT=$((INSTALLED_COUNT + 1))
fi

if [ $INSTALLED_COUNT -eq 0 ]; then
    echo "❌ No VS Code installation found!"
    echo "Please install VS Code or VS Code Insiders first."
    exit 1
fi

echo ""
echo "🎯 Installed to $INSTALLED_COUNT VS Code variant(s)"
echo ""
echo "📋 Next steps:"
echo "1. Close VS Code completely and restart it"
echo "2. Open any .grue file to see syntax highlighting"
echo "3. Check bottom-right corner shows 'Grue' (not 'Plain Text')"
echo ""
echo "🎯 Test with: examples/mini_zork.grue"

echo "🎉 Happy coding in Grue!"