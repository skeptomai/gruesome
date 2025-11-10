# Literal Pattern Regression Analysis and Fix Plan

## Problem Summary

The v2.6.1 "fix" for "look at mailbox" broke the basic "look around" literal pattern matching. The regression causes "look around" to display "You can't see any such thing" instead of calling the `look_around()` function.

## Root Cause Analysis

### Debug Evidence
From runtime logs:
```
Parse buffer word count: 2
Word 0: dict_addr=0x0a70 (look)
Word 1: dict_addr=0x0a04 (around)
🔥 LITERAL_WORDCOUNT_CHECK_RUNTIME: PC=0x1c7e, checking if word count (2) == 2 ✓ PASSES
```

**The word count check passes, but no dictionary comparison happens afterward.**

### Code Comparison
**WORKING VERSION (backup):**
- Loads **word 1** dictionary address from parse buffer offset 3
- Compares word 1 ("around") against literal pattern dictionary address
- Correctly handles 2-word patterns like "look around"

**BROKEN VERSION (current):**
- Loads **word 2** dictionary address from parse buffer offset 5
- Tries to compare word 2 (which doesn't exist!)
- For "look around": word 0="look", word 1="around", **word 2=garbage**

### Critical Difference
```diff
WORKING (backup):
- Operand::SmallConstant(3), // Offset 3 = word 1 dict addr
- Some(4), // Store in local variable 4

BROKEN (current):
- Operand::SmallConstant(5), // Word 2 dict addr at offset 5
- Some(7), // Store in local variable 7
```

**The current code is looking for word 2 when it should look for word 1.**

## Impact Assessment

### Currently Broken
- `"look around"` → "You can't see any such thing"
- `"take all"` → likely broken
- Any single literal patterns like `verb "go" { "up" => handle_go("up") }`

### Still Working
- `"look at mailbox"` → works (uses literal+noun patterns)
- `"look"` (default) → works
- Noun-only patterns → works

## Fix Plan

### 1. Immediate Fix (High Confidence)
**File:** `src/grue_compiler/codegen.rs`
**Location:** Lines 2757-2771 (literal pattern word loading)

**Change:**
```rust
// WRONG (current):
Operand::SmallConstant(5), // Word 2 dict addr at offset 5

// CORRECT (fix):
Operand::SmallConstant(3), // Word 1 dict addr at offset 3
```

**Rationale:** For "look around", we need to compare word 1 ("around") not word 2 (doesn't exist).

### 2. Variable Storage Fix
**Change:**
```rust
// Consider changing to match backup:
Some(4), // Store in local variable 4 (like backup)
// Instead of:
Some(7), // Store in local variable 7 (current)
```

### 3. Validation Steps

1. **Test basic literal patterns:**
   - `"look around"` should call `look_around()`
   - `"take all"` should call `take_all()`

2. **Ensure no regressions:**
   - `"look at mailbox"` should still work
   - `"look"` (default) should still work
   - Noun patterns should still work

3. **Runtime verification:**
   - Debug logs should show dictionary comparison happening
   - Should see `🔍 LITERAL_COMPARE` messages in logs
   - No more "You can't see any such thing" for valid literals

### 4. Root Cause Prevention

The issue occurred because literal+noun pattern fixes (for "look at mailbox") incorrectly assumed all literal patterns need word 2. But simple literal patterns like "look around" only have word 1.

**Architecture principle:** Distinguish between:
- **Simple literal patterns:** `"around"` (check word 1)
- **Literal+noun patterns:** `"at" + noun` (check word 1 for literal, word 2 for noun)

## Confidence Level: High

This is a straightforward off-by-one error in parse buffer indexing. The backup implementation was working correctly, and the fix is a simple change from offset 5 to offset 3.

## Testing Protocol

1. Compile with fix
2. Test: `echo "look around" | ./target/debug/gruesome`
3. Expected: Room description (not "You can't see any such thing")
4. Test: `echo "look at mailbox" | ./target/debug/gruesome`
5. Expected: Still works (no regression)