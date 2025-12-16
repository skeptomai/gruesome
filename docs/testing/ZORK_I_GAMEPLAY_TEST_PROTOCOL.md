# Zork I Gameplay Test Protocol

## Purpose

This protocol verifies the Z-Machine interpreter's compatibility with commercial Infocom games by testing core gameplay systems in the original Zork I.

## Pre-Test Setup

### 1. Build Z-Machine Interpreter
```bash
cargo build --bin gruesome
```

### 2. Game File Location
```
resources/test/zork1/DATA/ZORK1.DAT
```

## Gameplay Test Sequence

Run the following command sequence **in exact order**:

```bash
echo -e "north\neast\nopen window\nenter\ntake bag\ntake bottle\nleave\nscore\ninventory\nquit\ny" | ./target/debug/gruesome resources/test/zork1/DATA/ZORK1.DAT 2>/dev/null
```

### Individual Commands and Expected Results

1. **`north`**
   - **Expected**: Move to "North of House"
   - **Tests**: Basic navigation in commercial Z-Machine game

2. **`east`**
   - **Expected**: Move to "Behind House" with window description
   - **Tests**: Multi-directional navigation

3. **`open window`**
   - **Expected**: "With great effort, you open the window far enough to allow entry."
   - **Tests**: Complex verb processing, object state changes

4. **`enter`**
   - **Expected**: Move to Kitchen, shows table with sack and bottle
   - **Tests**: Room transitions, object descriptions, container contents

5. **`take bag`**
   - **Expected**: "Taken."
   - **Tests**: Object interaction, inventory management

6. **`take bottle`**
   - **Expected**: "Taken."
   - **Tests**: Multiple object handling

7. **`leave`**
   - **Expected**: Return to "Behind House"
   - **Tests**: Exit/departure commands

8. **`score`**
   - **Expected**: "Your score is 10 (total of 350 points), in 7 moves. This gives you the rank of Beginner."
   - **Tests**: Score tracking, move counting, ranking system

9. **`inventory`**
   - **Expected**: Shows glass bottle (with water contents) and brown sack
   - **Tests**: Inventory display, nested container contents

## Success Criteria

### ✅ Commercial Game Compatibility
- **Navigation**: All standard directional commands work
- **Object Interaction**: Taking and manipulating objects
- **Complex Verbs**: Non-basic commands like "open window", "enter"
- **Score System**: Accurate point tracking and move counting
- **Container Logic**: Nested object relationships (bottle contains water)
- **Room Descriptions**: Full location text display
- **Game State**: Persistent object and world state

### ❌ Failure Indicators
- "I don't understand that" for standard commands
- Crashes or infinite loops
- Missing or corrupted room descriptions
- Incorrect score or move counting
- Objects not appearing in inventory
- Container contents not displayed

## Expected Score Progression
- **Start**: 0 points, 0 moves
- **After entering kitchen**: 10 points, 4 moves
- **At score command**: 10 points, 7 moves
- **At inventory**: 10 points, 8 moves

## System Coverage

This protocol tests:
- **Z-Machine Compatibility**: Commercial game execution
- **Instruction Set**: Core Z-Machine opcodes used by Infocom
- **Object System**: Commercial object table format
- **Text Processing**: Infocom text compression/encoding
- **Memory Management**: Large commercial game memory layout
- **Parser System**: Infocom's verb/noun recognition
- **State Persistence**: Game world state tracking

## Usage

Run this protocol:
- **Before interpreter releases** to verify commercial game compatibility
- **After Z-Machine changes** to catch compatibility regressions
- **For regression testing** against known working commercial games
- **To validate interpreter accuracy** against Infocom standards

## Notes

- Uses original 1981-1983 Infocom Zork I (Revision 88 / Serial 840726)
- Minor terminal formatting artifacts are acceptable
- Game should display proper copyright and version information
- Score of 10 points awarded for entering kitchen with treasures
- Bottle properly shows nested water contents in inventory