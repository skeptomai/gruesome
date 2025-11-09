# Literal Pattern Debugging Analysis

## Problem Statement
Both single literal patterns ("look around") and literal+noun patterns ("look at mailbox") return "I don't understand that." despite correcting parse buffer offsets.

## What Works
- Basic verbs without literals ("inventory", "look") work correctly
- Parse buffer offset corrections have been applied:
  - Single literal: loads from offset 4 (correct for word 1)
  - Literal+noun: loads from offset 4 for literal, offset 6 for noun

## Parse Buffer Structure (Confirmed)
For "look around" (2 words):
- Offset 1: word count = 2
- Offset 2: "look" dictionary address
- Offset 3: "look" position info
- Offset 4: "around" dictionary address
- Offset 5: "around" position info

For "look at mailbox" (3 words):
- Offset 1: word count = 3
- Offset 2: "look" dictionary address
- Offset 3: "look" position info
- Offset 4: "at" dictionary address
- Offset 5: "at" position info
- Offset 6: "mailbox" dictionary address
- Offset 7: "mailbox" position info

## Potential Issues Still Present

### 1. Word Count Check
Need to verify if word count is being checked correctly before literal pattern processing:
- Single literal: should require word count >= 2
- Literal+noun: should require word count >= 3

### 2. Pattern Processing Order
Grammar in examples/mini_zork.grue:
```
verb "look" {
    default => look_around(),
    "at" + noun => examine($2),
    "around" => look_around()
}
```

Processing order in code might be wrong - should check literals before falling back to default.

### 3. Branch Logic Verification
Current logic uses 0x7FFF (branch on FALSE):
- When words match: continue executing pattern
- When words don't match: skip pattern

This should be correct, but needs verification.

### 4. Dictionary Reference Resolution
Need to verify dictionary addresses are being resolved correctly:
- "at" should resolve to a specific dictionary address
- "around" should resolve to a specific dictionary address

## Investigation Steps
1. Check word count validation logic
2. Verify pattern processing order
3. Test with debug output to see which path is being taken
4. Check if literal patterns are even being generated/reached

## Root Cause Hypothesis
The issue is likely that the literal pattern checks are either:
1. Not being generated correctly
2. Not being reached due to pattern order/word count issues
3. Executing but failing due to incorrect branch logic or dictionary comparison