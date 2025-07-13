# ZIL Startup Sequence for Zork I

## Complete Execution Flow from GO to Parser

This shows the actual execution sequence in ZIL from startup through the first parser prompt:

```
<GO>
<QUEUE>
<QUEUE>
<QUEUE>
<QUEUE>
<QUEUE>
<THIS-IS-IT>
<V-VERSION>
"ZORK I: The Great Underground Empire Copyright (c) 1981, 1982, 1983 Infocom, Inc. "
"All rights reserved."
"ZORK is a registered trademark of Infocom, Inc. Revision "
" / Serial number "
<V-LOOK>
<DESCRIBE-ROOM>
"West of House"
<WEST-HOUSE>
"You are standing in an open field west of a white house, with a boarded front door."
<DESCRIBE-OBJECTS>
<PRINT-CONT>
<FIRSTER>
<DESCRIBE-OBJECT>
"There is a "
"small mailbox"
" here"
"."
<NULL-F>
<SEE-INSIDE?>
<MAIN-LOOP>
<PARSER>
<LIT?>
">"
```

## Key Observations

1. **The LIT? check happens AFTER room description** - In the ZIL execution, `<LIT?>` is called by the parser after the room has already been described.

2. **V-LOOK successfully shows the room** - The game displays "West of House" and the full description before any lighting check.

3. **The GO routine sets LIT before V-LOOK** - As shown in the ZIL source:
   ```zil
   <SETG LIT T>
   ...
   <V-LOOK>
   ```

4. **Our compiled game checks lighting too early** - It's checking lighting BEFORE V-LOOK, not after.

## The Critical Difference

### ZIL/Expected Flow:
1. GO routine sets LIT = TRUE
2. V-LOOK displays room (no lighting check yet)
3. MAIN-LOOP starts
4. PARSER checks LIT? before accepting input

### Our Buggy Flow:
1. Set location to West of House
2. **Check lighting (FAIL - LIT not set yet)**
3. Call darkness routine
4. Set LIT = TRUE (too late!)
5. Call main loop

The compiled game has the lighting check in the wrong place - it should not be checking lighting before V-LOOK.