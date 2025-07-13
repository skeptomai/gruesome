# Zork I Game Initialization Sequence

## Overview
This document captures the exact sequence of operations that occur after the serial number display and before the sread instruction in Zork I.

## Execution Flow After Serial Number (PC 06fb5)

### 1. Location Setup (PC 04f75)
```
04f75: store #0010, #00b4 (form=Long, opcode=0d)
```
- Sets global variable 0 (location) to 180 (0xB4)
- Object 180 is "West of House"
- This establishes the player's starting location

### 2. Test Lighting (PC 04f7e)
```
04f7e: test_attr V10, #0003 [TRUE +8] (form=Long, opcode=0a)
test_attr at 04f7e: obj=180, attr=3, result=false
```
- Tests if West of House (object 180) has attribute 3 (ONBIT)
- Result is FALSE - the location does not have the lighting attribute
- Since test fails, execution branches to call the darkness routine

### 3. Call Darkness Routine (PC 04f82)
```
04f82: call #3770 -> V00 (form=Variable, opcode=00)
```
- Calls routine at address 0x3770
- This routine prints: "Only bats can see in the dark. And you're not one."
- This is why the game shows the darkness message instead of the room description

### 4. Variable Settings (PC 04f88-04f91)
After returning from the darkness routine, the game sets several important variables:

```
04f88: store #0052, #0001 (form=Long, opcode=0d)  // Set variable 82 (0x52) to 1
04f8b: store #007f, #0004 (form=Long, opcode=0d)  // Set variable 127 (0x7f) to 4 
04f8e: store #0090, V7f (form=Long, opcode=0d)    // Set variable 144 (0x90) to value of V7f (which is 4)
04f91: insert_obj V7f, V10 (form=Long, opcode=0e) // Insert object 4 into object 180
```

Key variable assignments:
- **Variable 82 (0x52)**: Set to 1 (possibly a game state flag)
- **Variable 127 (0x7f)**: Set to 4 (this is the player object number)
- **Variable 144 (0x90)**: Set to 4 (copy of player object reference)
- **Object insertion**: Player object (4) inserted into West of House (180)

### 5. Call Main Game Loop (PC 04f94)
```
04f94: call #3f02 -> V00 (form=Variable, opcode=00)
```
- Calls routine at 0x3f02, which is the main game loop
- This routine handles the game's command processing
- Eventually leads to sread instruction for getting user input

## Key Findings

1. **The lighting problem occurs at PC 04f7e** where the game tests for attribute 3 (ONBIT) on West of House
2. **West of House lacks the ONBIT attribute**, causing the test to fail
3. **The darkness routine at 0x3770** is called instead of showing the room description
4. **Player object (4) is properly placed** in West of House (180) via insert_obj
5. **The game state is otherwise correct** - location is set, player is positioned, variables are initialized

## Root Cause
The issue is that the LIT? routine (lighting check) only tests the ONBIT attribute and doesn't perform the secondary checks that should make outdoor locations naturally lit. According to Z-Machine documentation, when ONBIT is clear, the routine should check for light-providing objects in the location.

## Object Attributes for West of House
From object table analysis:
- Object 180 (West of House) has attributes: 6, 9, and 20
- Attribute 6: VISITED?
- Attribute 9: CONTAINER?
- Attribute 20: ROOM?
- **Missing**: Attribute 3 (ONBIT) - the lighting attribute

## Next Steps
Need to investigate why the LIT? routine at the test_attr instruction (PC 04f7e) doesn't perform the full lighting detection logic that would recognize outdoor locations as naturally lit.