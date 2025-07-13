# Root Cause Analysis: Zork I Lighting Bug

## Summary
The game displays "Only bats can see in the dark" at startup due to an initialization order bug where the lighting check happens before the LIT global variable is set.

## The Bug

### Execution Order in Game File
1. **PC 04f75**: `store #0010, #00b4` - Sets location to West of House
2. **PC 04f7e**: `test_attr V10, #0003` - Tests if location has ONBIT attribute (fails)
3. **PC 04f82**: `call #3770` - Calls darkness routine (prints error message)
4. **PC 04f88**: `store #0052, #0001` - Sets LIT global to TRUE (too late!)

### Expected Order (from ZIL source)
```zil
<SETG HERE ,WEST-OF-HOUSE>    ; Set location
<SETG LIT T>                  ; Set LIT to true FIRST
<MOVE ,WINNER ,HERE>          ; Move player
<V-LOOK>                      ; THEN look at room
```

## The Problem
- The test at PC 04f7e only checks the ONBIT attribute on the location object
- West of House (and other outdoor locations) don't have ONBIT set in the object table
- They rely on the global LIT variable to indicate it's daytime
- But LIT isn't set to TRUE until AFTER the lighting check

## Variable Mappings
- Global 0x10 (16): Current location (HERE)
- Global 0x52 (82): LIT - global lighting state (day/night)
- Global 0x7f (127): WINNER/PLAYER - the player object
- Object 180: West of House
- Object 4: Adventurer (player)

## Why This Matters
Outdoor locations in Zork use a two-tier lighting system:
1. Indoor locations: Use ONBIT attribute for permanent lighting
2. Outdoor locations: Use LIT global for day/night cycle

The initialization code fails to set up the day/night state before checking visibility.

## Possible Fixes
1. **Patch the game file**: Move the `store #0052, #0001` instruction before PC 04f7e
2. **Fix the interpreter**: Check LIT global in addition to ONBIT when testing lighting
3. **Pre-set LIT**: Initialize global 0x52 to 1 before starting execution

This appears to be a bug in the original Infocom game file where the GO routine's instructions were compiled in the wrong order.