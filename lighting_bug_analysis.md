# Lighting Bug Analysis - Exact Order of Operations

## The Bug Sequence

### 1. Location Setup (PC 04f75)
```
04f75: store #0010, #00b4
```
- Sets global variable 0 (location) to 180 (West of House)
- This happens BEFORE checking lighting

### 2. Lighting Check (PC 04f7e)
```
04f7e: test_attr V10, #0003 [TRUE +8]
```
- Tests if object 180 (West of House) has attribute 3 (ONBIT/lighting)
- Result: FALSE - West of House does NOT have the ONBIT attribute
- Since test fails, execution does NOT branch and continues to next instruction

### 3. Call Darkness Routine (PC 04f82)
```
04f82: call #3770 -> V00
```
- Because the lighting test failed, the game calls the darkness routine
- This routine prints: "Only bats can see in the dark. And you're not one."
- The routine eventually returns to PC 04f87

### 4. After Darkness Message (PC 04f87-04f91)
```
04f87: new_line
04f88: store #0052, #0001    // Set global variable 82 (likely LIT global) to 1
04f8b: store #007f, #0004    // Set variable 127 to 4 (player object)
04f8e: store #0090, V7f      // Set variable 144 to player object
04f91: insert_obj V7f, V10   // Insert player (4) into West of House (180)
```

## The Core Problem

The bug is in the order of operations:

1. **PC 04f7e**: The game checks if the current location is lit by testing ONLY the ONBIT attribute
2. **PC 04f88**: AFTER the darkness check fails, the game sets global variable 0x52 (likely the LIT global) to 1

This is backwards! The LIT global should be set BEFORE checking lighting, not after.

## Why This Is Wrong

The lighting check at PC 04f7e is too simplistic. It only checks:
- Does the location have attribute 3 (ONBIT)?

But according to the Z-Machine standard, lighting should work as follows:
1. If location has ONBIT attribute → it's lit
2. If not, check if LIT global variable is true → it's lit
3. If not, check for light-providing objects in location → it's lit
4. Otherwise → it's dark

The game is failing because:
- West of House does NOT have ONBIT (it's an outdoor location, naturally lit)
- The LIT global hasn't been set yet (it's set AFTER the check at PC 04f88)
- No secondary lighting checks are performed

## The Fix

The game initialization code needs to either:
1. Set the LIT global (variable 0x52) to 1 BEFORE the lighting check at PC 04f7e
2. OR give West of House the ONBIT attribute in the game data
3. OR implement proper lighting logic that recognizes outdoor locations

The most likely intended behavior is that the LIT global should be initialized before any lighting checks occur.