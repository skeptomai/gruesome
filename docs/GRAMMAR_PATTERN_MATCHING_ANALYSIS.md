# Z-Machine Grammar Pattern Matching - Architecture Analysis

## Overview

This document traces the complete flow of grammar pattern matching in the Z-Machine interpreter and compiler, explaining why multi-word patterns like "look around" and "look at noun" are not being matched correctly.

## Problem Statement

Current grammar patterns in mini_zork.grue:
```grue
verb "look" {
    default => look_around(),
    "at" + noun => examine($noun),
    "around" => look_around()
}
```

**Issue**: The pattern `"around"` (literal keyword) and `"at" + noun` (literal + noun) are not being matched. When the user types "look around", the game falls back to calling examine(0) instead of the correct look_around() function.

## Architecture - Three-Layer Pattern Matching System

### Layer 1: Grue Compiler (Semantic Analysis)
**File**: `/Users/cb/Projects/infocom-testing-old/infocom/src/grue_compiler/ir.rs` (lines 2448-2560)

The Grue compiler converts grammar declarations into IR (Intermediate Representation):

```rust
pub struct IrGrammar {
    pub verb: String,
    pub patterns: Vec<IrPattern>,
}

pub struct IrPattern {
    pub pattern: Vec<IrPatternElement>,
    pub handler: IrHandler,
}

pub enum IrPatternElement {
    Literal(String),          // "around", "at", etc.
    Noun,                     // Placeholder noun from parse buffer
    Default,                  // Verb-only, no additional words
    Adjective,
    MultiWordNoun,
    Preposition,
    // ... other pattern types
}
```

**Current Status**: The compiler correctly parses and stores pattern information including Literal elements.

### Layer 2: Z-Machine Code Generation (Codegen)
**File**: `/Users/cb/Projects/infocom-testing-old/infocom/src/grue_compiler/codegen.rs` (lines 2344-3154)

The codegen generates the actual Z-Machine bytecode that performs runtime pattern matching.

#### Current Codegen Coverage (INCOMPLETE)

The `generate_verb_matching()` function (line 2371) currently implements:

1. **Verb Matching** (lines 2405-2618)
   - Loads first word (word 0) dictionary address from parse buffer
   - Compares it against the verb's dictionary address
   - Branches to verb-only or noun cases based on match

2. **Word Count Branching** (lines 2620-2697)
   - Checks if word count >= 2 using `jl` instruction
   - Branches to `verb_only_label` if word_count < 2
   - Otherwise continues to noun case

3. **Noun Case Handler** (lines 2698-2853)
   - Executed when word_count >= 2
   - Loads word 1 dictionary address (the noun)
   - Looks up noun in game object database
   - Calls pattern handler with resolved object ID

4. **Default/Verb-Only Case Handler** (lines 2855-3087)
   - Executed when word_count < 2
   - Calls pattern handler with no object parameter
   - Returns control to main loop

#### CRITICAL MISSING FUNCTIONALITY

**The codegen does NOT implement matching for multi-word patterns:**

1. **Literal Element Matching** - Not implemented
   - Patterns with `Literal("around")` or `Literal("at")` are ignored
   - No code generated to compare parsed words against literal strings
   - The `IrPatternElement::Literal` variants in the patterns are never processed

2. **Multi-Word Pattern Branching** - Not implemented
   - No code to distinguish between:
     - `default => ...` (word count == 1)
     - `"at" + noun => ...` (word count == 2, word 1 == "at")
     - `"around" => ...` (word count == 2, word 1 == "around")
   - All patterns with word_count >= 2 go to the same noun handler

3. **Pattern Selection Logic** - Hardcoded to two cases
   - Lines 2629-2642: Only searches for `Default` and `Noun` patterns
   - Ignores all `Literal` and multi-word patterns
   - No loop to iterate through multiple patterns with different literal requirements

### Layer 3: Runtime Dictionary Parsing (VM)
**File**: `/Users/cb/Projects/infocom-testing-old/infocom/src/dictionary.rs` (lines 290-352)

The VM's `parse_text()` function tokenizes input into the parse buffer:

```
Parse Buffer Layout:
[0]   = max words
[1]   = actual word count (N)
[2]   = word 0 dict addr (low byte)
[3]   = word 0 dict addr (high byte)
[4]   = word 0 text position
[5]   = word 0 length
[6]   = word 1 dict addr (low byte)
[7]   = word 1 dict addr (high byte)
[8]   = word 1 text position
[9]   = word 1 length
... (repeating every 4 bytes for each word)
```

**Current Status**: Correctly parses and stores word dictionary addresses. Example:
- Input: "look around"
- After parsing:
  - word 0 = "look" dictionary address
  - word 1 = "around" dictionary address
  - word count = 2

The parsed data is available for pattern matching but currently NOT USED by the codegen.

## The Matching Gap

### What Should Happen

When user types "look around":
1. Tokenized into: ["look", "around"]
2. Parse buffer filled with dictionary addresses for both words
3. Generated Z-Machine code should:
   - Match "look" as verb ✓ (currently works)
   - Check word count >= 2 ✓ (currently works)
   - Compare word 1 against multiple literals:
     - Is it "at"? If yes, extract noun and call examine($noun)
     - Is it "around"? If yes, call look_around()
   - Fall back to noun pattern if no literal matches

### What Actually Happens

When user types "look around":
1. Tokenized correctly ✓
2. Parse buffer filled correctly ✓
3. Generated Z-Machine code:
   - Matches "look" verb ✓
   - Detects word count >= 2 ✓
   - **JUMPS to noun handler** ← Problem occurs here
   - Tries to resolve "around" as a game object
   - Fails because "around" is not an object
   - Defaults to object ID 0 (null)
   - Calls examine(0)

## Code Evidence

### Evidence 1: Pattern Search Ignores Literals
**codegen.rs, lines 2629-2636**:
```rust
let default_pattern = patterns.iter().find(|p| {
    p.pattern.contains(&crate::grue_compiler::ir::IrPatternElement::Default)
});
let noun_pattern = patterns.iter().find(|p| {
    p.pattern.contains(&crate::grue_compiler::ir::IrPatternElement::Noun)
});
```

Only searches for Default and Noun patterns. Literal patterns are never found.

### Evidence 2: Only Two Cases Implemented
**codegen.rs, line 2621**:
```rust
for (_i, _pattern) in patterns.iter().enumerate() {}  // Empty loop - patterns not processed
```

The loop iterates through patterns but generates no code for them.

### Evidence 3: Hardcoded Branch Logic
**codegen.rs, lines 2698-2853**:
```rust
// VERB+NOUN CASE: We have at least 2 words, process noun pattern
if let Some(pattern) = noun_pattern {
    // ... generate handler for noun pattern
}
```

All patterns with 2+ words go to the noun handler.

### Evidence 4: No Literal Matching in Generated Code
The generated Z-Machine bytecode has no instructions to:
- Load word 1 and compare it against "at"
- Load word 1 and compare it against "around"
- Branch to different handlers based on literal values

## The Root Cause

The codegen architecture assumes only two pattern types:
1. **Default**: `verb => handler()` - verb only
2. **Noun**: `verb noun => handler()` - verb + arbitrary object

This two-tier system cannot handle:
- Literal keywords: `verb "keyword" => handler()`
- Multi-word matching: `verb "keyword" + noun => handler($noun)`
- Prepositions: `verb "at" + noun => handler($noun)`

To fix this, the codegen must:

1. **Iterate through all patterns** for each verb
2. **For each pattern**, generate branching code to check:
   - If word count matches pattern requirements
   - If literal words match parse buffer values
   - If noun parameter matches game objects (when present)
3. **Select the first matching pattern** and call its handler
4. **Fall through to error handler** if no patterns match

## Pattern Matching Algorithm Needed

```
function match_verb_patterns(verb_dict_addr, word_count, parse_buffer):
    Load word 0 dict addr from parse_buffer
    If word 0 dict addr != verb_dict_addr:
        Skip to next verb
    
    // Verb matched, now try patterns
    for each pattern in verb.patterns:
        if pattern matches word_count and literals:
            extract arguments for pattern
            call pattern.handler with arguments
            jump to main loop
    
    // No pattern matched, print error
    print "I don't understand that."
    jump to main loop
```

## Required Codegen Implementation

### Step 1: Pattern Analysis
Instead of finding single Default/Noun patterns, enumerate all patterns:
```rust
for pattern in patterns {
    // Determine pattern type:
    // - How many words does it require?
    // - What literals must word 1, word 2, etc. match?
    // - Which word is the noun (if any)?
    
    // Generate matching code for this specific pattern
}
```

### Step 2: Pattern Matching Code
For each pattern, generate comparison code:
```asm
; For pattern: "at" + noun
loadw parse_buffer, 3    ; Load word 1 dict addr
storew globals_base, 200, <dict_addr>  ; Store in temporary
je variable_6, variable_216  ; Compare with "at" dict addr
je <continue_to_handler>, <try_next_pattern>

<continue_to_handler>:
; Extract noun from word 2, call handler
; Fall through to next pattern check

<try_next_pattern>:
; Try next pattern...
```

### Step 3: Pattern Enumeration in IR
The IR codegen must enumerate patterns instead of finding only Default/Noun:
```rust
pub struct IrPatternInfo {
    pub word_count: usize,
    pub literals: Vec<(usize, String)>,  // (word_index, literal_text)
    pub noun_position: Option<usize>,    // Which word is noun, if any
    pub pattern: IrPattern,
}
```

## Files That Need Modification

1. **src/grue_compiler/codegen.rs**
   - `generate_verb_matching()` (line 2371)
   - Add logic to process all patterns, not just Default/Noun
   - Generate literal comparison code
   - Create pattern enumeration and matching loop

2. **Possibly src/grue_compiler/ir.rs**
   - May need pattern metadata for codegen guidance

## Testing the Fix

After implementation, these commands should work:
```
> look
(calls look_around, displays location)

> look around
(calls look_around, displays location)

> look at mailbox
(calls examine(mailbox), displays mailbox)

> examine egg
(calls examine(egg), displays egg)
```

## References

- Z-Machine Standard Section 15 (Opcodes)
- mini_zork.grue grammar definition (lines 153-253)
- Parse buffer layout (dictionary.rs lines 290-352)
- Verb matching code (codegen.rs lines 2371-2618)
