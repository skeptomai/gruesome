# Gruesome Z-Machine Interpreter

A modern interpreter for playing classic Infocom text adventure games.

## Quick Start

### Running a Game

```
gruesome path/to/game.z3
```

For example:
```
gruesome ZORK1.DAT
```

### Windows Users

After downloading `gruesome-x86_64-pc-windows-gnu.exe`:

1. Right-click the file and select "Properties"
2. Check the "Unblock" checkbox at the bottom
3. Click "OK"
4. Run from Command Prompt or PowerShell

If you see "Windows protected your PC":
1. Click "More info"
2. Click "Run anyway"

### macOS Users

1. Make the binary executable:
   ```
   chmod +x gruesome-universal-apple-darwin
   ```

2. If macOS blocks the app, go to System Preferences → Security & Privacy and click "Open Anyway"

3. Run the game:
   ```
   ./gruesome-universal-apple-darwin ZORK1.DAT
   ```

## Supported Games

Successfully tested with these Infocom classics:
- Zork I: The Great Underground Empire
- Deadline
- Enchanter
- The Hitchhiker's Guide to the Galaxy
- Suspended

Most Z-Machine version 3 games should work. Limited support for v4+ games.

## Game Commands

### Save and Restore
- Type `SAVE` to save your progress
- Type `RESTORE` to load a saved game
- Save files use the modern Quetzal format

### Common Commands
- `LOOK` or `L` - Describe current location
- `INVENTORY` or `I` - List what you're carrying
- `EXAMINE [object]` or `X [object]` - Look closely at something
- `NORTH/SOUTH/EAST/WEST` or `N/S/E/W` - Move in a direction
- `TAKE [object]` - Pick something up
- `DROP [object]` - Put something down
- `QUIT` - Exit the game

## Finding Game Files

Gruesome plays Z-Machine game files (usually .z3, .z5, or .DAT extensions). You can find games at:
- Your original Infocom game disks
- The Interactive Fiction Archive
- Various preservation projects

Note: Please respect copyright and only play games you own or that are freely available.

## Troubleshooting

### "File not found" error
Make sure you're providing the correct path to the game file.

### Game doesn't start
Verify the game file is a valid Z-Machine file (version 3 works best).

### Text is garbled
The game file may be corrupted. Try obtaining it from another source.

## About

Gruesome is an open-source project. For source code and development information, visit:
https://github.com/skeptomai/gruesome

Version 0.0.2