import * as vscode from 'vscode';

export function activate(context: vscode.ExtensionContext) {
    // Register the language configuration
    console.log('Grue Language Support extension activated');

    // You can add additional functionality here in the future:
    // - Code completion
    // - Error diagnostics
    // - Go to definition
    // - Hover information
    // etc.
}

export function deactivate() {
    console.log('Grue Language Support extension deactivated');
}