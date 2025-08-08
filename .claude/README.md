# Claude Code Configuration

This directory contains configuration files for Claude Code AI assistant.

## Files

### `settings.json` (Checked in ✅)
- Project-wide tool permissions
- Standard development workflow authorizations
- Safe to share with all contributors
- No user-specific paths or sensitive data

### `settings.local.json` (Git ignored ❌)
- User-specific configurations
- Personal file paths and preferences
- May contain sensitive information
- Each developer should create their own

## Setup for New Contributors

1. Claude Code will use `settings.json` automatically for basic permissions
2. If you need user-specific settings, create your own `settings.local.json`
3. The local settings will override/extend the project settings

## Automation Commands

The project supports these voice commands (configured in CLAUDE.md):

- **"Make it so!"** - Commit and push changes
- **"Engage!"** - Create a new release with version increment
- **"Reengage!"** - Re-release with the same version number

See [CLAUDE.md](../CLAUDE.md) for full documentation of automation workflows.