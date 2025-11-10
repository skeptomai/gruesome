# Function Call Resolution Bug Analysis

## Problem Summary

Generic functions that call themselves with specific object parameters cause infinite recursion instead of dispatching to specialized versions.

**Example Code:**
```
fn handle_climb(obj) {  // Generic version
    if (player.location == forest_path) {
        println("using the generic");
        handle_climb(tree);  // Should call specialized version but infinite loops
    }
    println("You can't climb that.");
}

fn handle_climb(tree) {  // Specialized version
    println("using the specific");
    if (player.location == forest_path) {
        handle_go("up");
    }
}
```

**Observed Behavior:**
- `climb tree` command works correctly (calls specialized version)
- `climb` command infinite loops on the generic version
- Direct call with specific object parameter from inside generic version causes infinite recursion

## Root Cause Analysis

The bug is in the **function call resolution logic** in the IR generator (`src/grue_compiler/ir.rs`).

### Issue Location: Lines 3414-3460 in ir.rs

When generating a function call expression, the code checks two sources for the function ID:

```rust
let func_id = if let Some(&dispatch_id) = self.dispatch_functions.get(&name) {
    log::debug!("🎯 Direct call using dispatch function for '{}': ID {}", name, dispatch_id);
    dispatch_id
} else if let Some(&id) = self.symbol_ids.get(&name) {
    log::debug!("🎯 Direct call using original function for '{}': ID {}", name, id);
    id
} else {
    return Err(...);
};
```

### The Problem

When inside a generic function calling itself with a specific parameter:

1. **Dispatch functions are ONLY created when there are multiple overloads** (line 1266 in ir.rs):
   ```rust
   if overloads.len() > 1 {
       // Create dispatch function
   }
   ```

2. **But dispatch functions are ADDED to the IR AFTER all functions are generated** (line 1471):
   ```rust
   // Generate dispatch functions for polymorphic functions
   self.generate_dispatch_functions(&mut ir_program)?;
   ```

3. **The `dispatch_functions` map is ONLY populated when dispatch functions are added** (lines 1277-1279):
   ```rust
   if let Some(dispatch_id) = ir_program.functions.last().map(|f| f.id) {
       self.dispatch_functions.insert(base_name.clone(), dispatch_id);
   }
   ```

### The Call Resolution Bug

When a generic function calls `handle_climb(tree)` during IR generation:

1. The code checks `self.dispatch_functions.get("handle_climb")` → **returns None** (dispatch functions haven't been created yet!)
2. Falls back to `self.symbol_ids.get("handle_climb")` → **returns the generic function's own ID**
3. The generic function calls itself, causing infinite recursion
4. **The specialized function is never even considered**

When the same call happens via the grammar system (after dispatch functions exist):
1. The dispatch function DOES exist in `dispatch_functions`
2. The dispatch function is called instead
3. The dispatch function has logic to route to the correct specialized version
4. Works correctly

### Why Direct Grammar Calls Work

Grammar pattern handlers are processed AFTER dispatch functions are generated (lines 1474-1477):
```rust
// Now process deferred grammar items with dispatch functions available
for grammar in deferred_grammar {
    let ir_grammar = self.generate_grammar(grammar)?;
    ir_program.grammar.push(ir_grammar);
}
```

When grammar calls `handle_climb(tree)`:
1. Dispatch functions already exist and are in `dispatch_functions`
2. Dispatch function is used
3. Correct specialization is routed to

## Dispatch Function Architecture

### What Dispatch Functions Do

A dispatch function is generated for each function with multiple overloads. It contains branching logic:

```
dispatch_handle_climb(obj):
    if obj == tree:
        call handle_climb_tree(obj)
        return
    // ... other specializations ...
    if obj == generic:
        call handle_climb_default(obj)
        return
```

### Specialization Detection (Lines 1217-1229)

Function specializations are detected by parameter name:
```rust
fn detect_specialization(..., parameters: &[Parameter]) -> ObjectSpecialization {
    for param in parameters {
        if self.object_numbers.contains_key(&param.name) {
            return ObjectSpecialization::SpecificObject(param.name.clone());
        }
    }
    ObjectSpecialization::Generic
}
```

- `fn handle_climb(obj)` → Generic (obj is not an object name)
- `fn handle_climb(tree)` → SpecificObject("tree")

### Mangled Names (Lines 1201-1214)

When overloads exist, functions are renamed:
- `handle_climb(obj)` → stored as `handle_climb_default`
- `handle_climb(tree)` → stored as `handle_climb_tree`
- Dispatch function `dispatch_handle_climb` routes between them

## The Timing Problem

### Current Execution Order:

```
1. PASS 1: Register function names (lines 1432-1443)
   symbol_ids["handle_climb"] = ID_1

2. PASS 2: Generate function IR (lines 1445-1456)
   - generate_function for handle_climb(obj) → uses ID_1, registers as overload
   - generate_function for handle_climb(tree) → creates ID_2, registers as overload
   - During body generation of handle_climb(obj):
     - Expression: handle_climb(tree)
     - Checks dispatch_functions["handle_climb"] → EMPTY (not generated yet!)
     - Falls back to symbol_ids["handle_climb"] → ID_1 (the generic function itself!)
     - **INFINITE RECURSION**

3. Create dispatch functions (line 1471)
   - dispatch_functions["handle_climb"] = dispatch_ID
   - **TOO LATE - functions already generated with wrong IDs**

4. Process grammar (lines 1474-1477)
   - Grammar calls handle_climb(tree)
   - Checks dispatch_functions["handle_climb"] → FOUND!
   - Routes correctly through dispatch function
   - **WORKS**
```

## Solution Options

### Option 1: THREE-PASS APPROACH (Recommended)

**Pass 1:** Register all functions and detect overloads
**Pass 2:** Create dispatch functions
**Pass 3:** Generate function bodies

This allows function bodies to be generated AFTER dispatch functions exist in the dispatch_functions map.

### Option 2: DEFERRED BODY GENERATION

Similar to grammar deferral - collect all function bodies, generate dispatch functions, then generate bodies.

### Option 3: DISPATCH FUNCTION REGISTRATION BEFORE BODY GENERATION

Register dispatch functions in symbol_ids during the overload collection phase, before any function bodies are generated.

## Code Locations

| Component | Location | Key Code |
|-----------|----------|----------|
| Function call IR generation | Lines 3414-3460 | Dispatch function check |
| Dispatch function creation | Lines 1287-1416 | `create_dispatch_function()` |
| Function overload registration | Lines 1232-1258 | `register_function_overload()` |
| Specialization detection | Lines 1217-1229 | `detect_specialization()` |
| Two-pass generation | Lines 1430-1471 | Pass 1 and Pass 2 |
| Grammar deferral | Lines 1449-1451, 1474-1477 | Deferred grammar processing |

## Impact

- **Direct function calls with specialization parameters:** Broken (infinite recursion)
- **Grammar pattern handlers with specialization:** Working (dispatch functions exist)
- **Functions without overloads:** Working (no dispatch needed)
- **Semantic analysis:** Not affected (allows overloads at line 259-267)

## Files Involved

- `/Users/cb/Projects/infocom-testing-old/infocom/src/grue_compiler/ir.rs` - IR generation and dispatch logic
- `/Users/cb/Projects/infocom-testing-old/infocom/src/grue_compiler/semantic.rs` - Semantic analysis (correctly allows overloads)
