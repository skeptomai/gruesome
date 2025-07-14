# Final Analysis: Zork I Lighting Bug

## The Bug
The game prints "Only bats can see in the dark. And you're not one." instead of displaying West of House when starting.

## Root Cause
The compiled game has a spurious lighting check in the version printing logic that shouldn't be there.

## Expected Flow (from disassembled code)
```
Main:
    location = west_of_house;
    if( location has visited ) skip_version;
    VersionSub();    // Print version/serial number
    new_line;
skip_version:
    Lit = true;      // SET LIT BEFORE LOOK
    move player to location;
    LookSub();       // This checks lighting properly
```

## Actual Flow in Our Game
```
04f75: location = west_of_house
04f78: call routine (part of version check?)
04f7e: test_attr location, ONBIT    // SPURIOUS LIGHTING CHECK!
04f82: call darkness_routine        // Prints "Only bats..."
04f87: new_line
04f88: Lit = 1                      // TOO LATE!
04f91: move player to location
04f94: call LookSub
```

## The Problem
There's an extra lighting check at PC 04f7e that's part of the version printing conditional. This check:
1. Tests if location has ONBIT attribute (it doesn't)
2. Calls a darkness routine that prints the error message
3. All before LIT is set to true

## Why It Works in Original Interpreter
The original interpreter must either:
1. Skip this spurious lighting check during initialization
2. Have special handling for the first room display
3. The compiled code in the original game file might be different

## The Fix
The lighting test at PC 04f7e shouldn't exist. The version printing logic should only check if the location has been visited, not whether it's lit. The only lighting check should happen inside LookSub (called at PC 04f94) after LIT has been set to true.

## Evidence
- Global 0x52 (LIT) starts as 0 in the game file (correct)
- LIT is set to 1 at PC 04f88 (correct timing per ZIL source)
- But a lighting check happens at PC 04f7e (incorrect - too early)
- This premature check causes the darkness message