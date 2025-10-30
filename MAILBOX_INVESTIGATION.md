# Mailbox Investigation Summary

## Problem Identified
User reported: "Now the mailbox doesn't show up when the game starts"
**Actual Issue**: Mailbox DOES show up, but `open mailbox` command fails with "You can't open that."

## Investigation Results

### Mailbox Visibility: ✅ WORKING
- Mailbox appears correctly in room description: "There is a small mailbox here."
- Both commit 5a51837 (working) and current version show mailbox
- User-defined `list_objects()` function correctly called (not builtin)

### Mailbox Opening: ❌ BROKEN
- Command `open mailbox` returns "You can't open that."
- This occurs in BOTH working commit 5a51837 and current version
- Expected behavior: Should open and show contents (leaflet)

## Root Cause Analysis

### Source Code Analysis
1. **Mailbox Definition** (mini_zork.grue:10-22):
   ```grue
   object mailbox {
       names: ["a small mailbox", "mailbox", "box"]
       openable: true      // ← This is correctly set
       container: true
       contains {
           object leaflet { ... }
       }
   }
   ```

2. **handle_open Function** (mini_zork.grue:244-252):
   ```grue
   fn handle_open(obj) {
       if !obj.openable {           // ← This check is failing
           print("You can't open that.");
           return;
       }
       // ... rest of opening logic
   }
   ```

### The Problem
- Mailbox has `openable: true` in source code
- But `obj.openable` property access returns `false` in handle_open function
- This indicates **property access is broken**, not object definition

## Hypothesis
Stack discipline changes are affecting property access operations:
- `obj.openable` should read the `openable` property and return `true`
- Instead, it's returning `false` (or undefined/0)
- This suggests stack/variable operations for property reads are corrupted

## Next Steps
1. Clean up investigation code and commit current state
2. Analyze broader stack discipline issues affecting property access
3. Focus on property access instruction generation and stack handling
4. Test property access in isolated cases to identify the stack corruption pattern

## Test Case for Reproduction
```bash
# Current broken behavior:
echo "open mailbox" | ./target/debug/gruesome tests/debug_current.z3
# Output: "You can't open that."

# Expected working behavior:
# Output: "Opened.\nThe small mailbox contains:\n  leaflet"
```

This is likely a systematic property access issue, not specific to mailbox functionality.