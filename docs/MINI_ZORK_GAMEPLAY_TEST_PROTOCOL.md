# Mini Zork Gameplay Test Protocol

## Purpose

This protocol verifies end-to-end functionality of the Grue compiler and Z-Machine interpreter by testing all core gameplay systems in mini_zork.grue.

## Pre-Test Setup

### 1. Build Compiler and Interpreter
```bash
cargo build --bin grue-compiler
cargo build --bin gruesome
```

### 2. Compile Mini Zork Game
```bash
cargo run --bin grue-compiler -- examples/mini_zork.grue -o tests/mini_zork.z3
```

## Gameplay Test Sequence

Run the following command sequence **in exact order**:

```bash
echo -e "open mailbox\ntake leaflet\nread leaflet\nnorth\nnorth\nclimb tree\ntake egg\ndown\nscore\ninventory\nquit\ny" | ./target/debug/gruesome tests/mini_zork.z3 2>/dev/null
```

### Individual Commands and Expected Results

1. **`open mailbox`**
   - **Expected**: "The [a] small mailbox contains: leaflet"
   - **Tests**: Container opening, object visibility

2. **`take leaflet`**
   - **Expected**: "Took leaflet." + Score increases to 2
   - **Tests**: Object interaction, scoring system

3. **`read leaflet`**
   - **Expected**: Full DORK welcome text display
   - **Tests**: String display, object property access

4. **`north`** (first)
   - **Expected**: Move to north side of house
   - **Tests**: Basic navigation

5. **`north`** (second)
   - **Expected**: Move to forest path, shows tree
   - **Tests**: Multi-room navigation

6. **`climb tree`**
   - **Expected**: Move up tree, shows nest and egg
   - **Tests**: Complex verb processing, object resolution

7. **`take egg`**
   - **Expected**: "You are the proud owner of a very special egg.. Is it ticking?" + Score increases to 7
   - **Tests**: Object interaction from containers, scoring

8. **`down`**
   - **Expected**: Return to forest path
   - **Tests**: Directional movement

9. **`score`**
   - **Expected**: "Your score is 7"
   - **Tests**: Score display system

10. **`inventory`**
    - **Expected**: Shows both "jewel-encrusted egg" and "leaflet"
    - **Tests**: Inventory tracking system

## Success Criteria

### ✅ All Systems Functional
- **Navigation**: All movement commands work correctly
- **Object Interaction**: Taking, examining, reading objects
- **Container System**: Opening containers, accessing contents
- **Scoring**: Points awarded correctly for achievements
- **Complex Verbs**: Non-basic commands like "climb tree"
- **Inventory Tracking**: All carried objects displayed
- **String Display**: Long text passages render correctly

### ❌ Failure Indicators
- "I don't understand that" for any test command
- Infinite loops or crashes
- Missing or corrupted text output
- Incorrect score calculations
- Objects not appearing in inventory
- Navigation failures

## System Coverage

This protocol tests:
- **Parser**: Verb recognition and grammar matching
- **Dictionary**: Word lookup for all command types
- **Object System**: Containment, visibility, properties
- **Movement**: Basic and complex directional commands
- **Scoring**: Point tracking and display
- **Text Display**: String rendering and formatting
- **Memory Management**: No crashes or corruption

## Usage

Run this protocol:
- **Before major releases** to verify core functionality
- **After compiler changes** to catch regressions
- **When debugging** to isolate specific system failures
- **For new contributors** to verify working setup

## Notes

- Commands must be run in exact order as some depend on previous state
- Minor text formatting artifacts (terminal control codes) are acceptable
- Score progression: 0 → 2 (leaflet) → 7 (egg)
- Total moves should be 4 at completion
- Game should end cleanly with quit confirmation