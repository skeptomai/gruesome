# Function Call Resolution Bug - Analysis Index

## Problem Statement

Generic functions that call specialized versions with object parameters cause **infinite recursion** instead of correctly dispatching to the specialized function.

**Example:**
```rust
fn handle_climb(obj) {           // Generic version
    handle_climb(tree);          // SHOULD call specialized version
}                                // ACTUALLY calls itself -> infinite loop

fn handle_climb(tree) {          // Specialized version for 'tree' object
    // This is never reached when called from handle_climb(obj)
}
```

**Observed Behavior:**
- `climb tree` command (via grammar) works correctly
- `climb` command (generic, via grammar) infinite loops
- Direct call from code: `handle_climb(tree)` called within `handle_climb(obj)` infinite loops

## Root Cause

**Single Critical Timing Bug:**
- Dispatch functions are created at line 1471
- Function call resolution happens at lines 3414-3460 during body generation
- Body generation occurs at lines 1445-1471, before dispatch creation
- `dispatch_functions` map is empty during body generation
- Function calls fall back to `symbol_ids`, which resolves to wrong function

**File:** `/Users/cb/Projects/infocom-testing-old/infocom/src/grue_compiler/ir.rs`

## Documentation Guide

Read these documents in this order:

### 1. **FUNCTION_DISPATCH_BUG_SUMMARY.txt** (Start here!)
   - Quick overview of the problem
   - Timeline of the execution order bug
   - Code locations
   - 104 lines, ~3 minute read

### 2. **FUNCTION_DISPATCH_VISUAL.txt**
   - Visual execution timeline with ASCII diagrams
   - Shows exactly what happens at each step
   - Illustrates what should happen vs what actually happens
   - 196 lines, ~5 minute read

### 3. **FUNCTION_DISPATCH_BUG_ANALYSIS.md**
   - Detailed technical analysis
   - Dispatch function architecture
   - Solution options with trade-offs
   - Complete code locations table
   - 213 lines, ~8 minute read

### 4. **FUNCTION_DISPATCH_CODE_FLOW.md**
   - Line-by-line execution walkthrough
   - Annotated code snippets
   - Call stacks and state transitions
   - 279 lines, ~12 minute read

### 5. **FUNCTION_DISPATCH_REFERENCE.md**
   - Complete reference guide
   - All related code components
   - Data structure definitions
   - Testing verification steps
   - Solution approaches in detail
   - 389 lines, ~15 minute read

## Quick Facts

| Aspect | Value |
|--------|-------|
| Bug Type | Timing/Ordering Bug |
| Severity | High - breaks function polymorphism |
| Files Affected | 1 file: `src/grue_compiler/ir.rs` |
| Lines of Code | ~5000 lines in file, ~50 lines central to bug |
| Root Cause Location | Line 1471 happens after lines 3414-3460 |
| Semantic Analyzer | Correctly allows overloads (no changes needed) |
| Impact Scope | Direct function calls only; grammar calls work |

## Key Code Locations

### Bug Manifestation
- **Lines 3414-3460:** `generate_expression()` - FunctionCall case
- **Line 3429:** Check for `dispatch_functions` (finds nothing)
- **Line 3436:** Fallback to `symbol_ids` (gets wrong function)

### Root Cause
- **Lines 1432-1443:** PASS 1 - Register function names
- **Lines 1445-1471:** PASS 2 - Generate function bodies (before dispatch exists)
- **Line 1471:** Create dispatch functions (after bodies already generated)

### Supporting Code
- **Lines 1232-1258:** `register_function_overload()`
- **Lines 1217-1229:** `detect_specialization()`
- **Lines 1287-1416:** `create_dispatch_function()`
- **Lines 1201-1214:** `mangle_function_name()`

## Solution Options

### Option 1: Three-Pass Approach (Recommended)
1. Pass 1: Register function names
2. Pass 1.5 (NEW): Detect overloads and pre-allocate dispatch functions
3. Pass 2: Generate function bodies
4. Pass 3: Create dispatch function bodies

**Pros:** Minimal changes, follows existing pattern (like grammar deferral)
**Cons:** Requires new phase

### Option 2: Deferred Body Generation
Similar to grammar deferral - collect AST, create dispatch, generate bodies

**Pros:** Clean separation, uses existing pattern
**Cons:** More structural changes

### Option 3: Dispatch Lookup Change
Check `function_overloads` directly during call resolution

**Pros:** Minimal code changes
**Cons:** Less clean architecture

## How Grammar Escapes This Bug

Grammar processing is **deferred** (lines 1449-1451, 1474-1477):
1. Grammar AST collected but not processed during PASS 2
2. Dispatch functions created at line 1471
3. Grammar processed at lines 1474-1477 with dispatch functions available
4. Function calls resolve correctly through dispatch

**That's why "climb tree" works but direct calls don't.**

## Testing the Bug

```rust
// This will infinite loop:
fn handle_climb(obj) {
    if (player.location == forest) {
        handle_climb(tree);  // INFINITE RECURSION
    }
}

fn handle_climb(tree) {
    println("specific handler");
}
```

Run with: `RUST_LOG=debug cargo run -- test.grue`

Look for log message:
- `"Direct call using original function"` = Bug triggered (wrong function called)
- `"Direct call using dispatch function"` = Correct behavior (dispatch found)

## Semantic Analysis Notes

The semantic analyzer in `src/grue_compiler/semantic.rs` (lines 258-267) correctly:
- Allows function overloading
- Doesn't enforce single definition per name
- Enables the IR generator to handle specialization

**No changes needed in semantic analysis.**

## Files to Review

1. **Primary:** `/Users/cb/Projects/infocom-testing-old/infocom/src/grue_compiler/ir.rs` (5000+ lines)
   - This is the only file with the bug

2. **Reference:** `/Users/cb/Projects/infocom-testing-old/infocom/src/grue_compiler/semantic.rs`
   - Not affected by bug, correctly allows overloads

3. **Reference:** `/Users/cb/Projects/infocom-testing-old/infocom/src/grue_compiler/ast.rs`
   - Contains `ObjectSpecialization` enum

## Historical Context

The polymorphic dispatch system was designed to support:
- Generic handlers: `fn handle_verb(obj) { ... }`
- Specialized handlers: `fn handle_verb(specific_object) { ... }`
- Automatic routing based on object type

**The architecture works perfectly for grammar-driven calls but has a timing issue with direct calls.**

## Next Steps

1. Choose a solution approach (recommend Three-Pass)
2. Implement the fix in `ir.rs`
3. Test with the example code above
4. Verify with: `RUST_LOG=debug cargo test -- --nocapture`
5. Update ONGOING_TASKS.md with status

## Related Analysis Documents

All documents are in `/Users/cb/Projects/infocom-testing-old/infocom/docs/`:
- `FUNCTION_DISPATCH_BUG_SUMMARY.txt` - Quick reference
- `FUNCTION_DISPATCH_VISUAL.txt` - Timeline diagrams  
- `FUNCTION_DISPATCH_BUG_ANALYSIS.md` - Detailed analysis
- `FUNCTION_DISPATCH_CODE_FLOW.md` - Code walkthrough
- `FUNCTION_DISPATCH_REFERENCE.md` - Complete reference
- `FUNCTION_DISPATCH_INDEX.md` - This file

---

**Status:** Bug identified and analyzed. Ready for implementation.

**Last Updated:** November 2025
