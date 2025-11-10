# Function Dispatch Bug - Complete Reference Guide

## Quick Diagnosis

**Symptom:** Generic function calling specialized version causes infinite recursion

**Example:**
```rust
fn handle_climb(obj) {
    handle_climb(tree);  // Infinite loop - calls itself instead of specialized version
}

fn handle_climb(tree) {
    // Never reached from handle_climb(obj)
}
```

**Cause:** Function call resolution happens BEFORE dispatch functions are created

**Status:** Bug confirmed in codebase, solution needed

---

## Bug Location: Single Critical Point

### Lines 3414-3460 in `/Users/cb/Projects/infocom-testing-old/infocom/src/grue_compiler/ir.rs`

**File:** `src/grue_compiler/ir.rs`
**Function:** `generate_expression()`
**Case:** `Expr::FunctionCall`

```rust
Expr::FunctionCall { name, arguments } => {
    // ...arguments generation...
    
    let func_id = if let Some(&dispatch_id) = self.dispatch_functions.get(&name) {  // LINE 3429
        dispatch_id
    } else if let Some(&id) = self.symbol_ids.get(&name) {  // LINE 3436
        id  // <-- BUG: This is called when dispatch_functions is empty!
    } else {
        return Err(...);
    };
    
    // Call is made with func_id
    block.add_instruction(IrInstruction::Call {
        target: Some(temp_id),
        function: func_id,
        args: arg_temps,
    });
}
```

### The Root Cause Chain

**Root Cause:** Wrong execution order
- dispatch_functions populated at LINE 1471
- Function call resolution happens at LINES 1445-1471 (during function body generation)
- dispatch_functions is empty during body generation
- Fallback to symbol_ids resolves to wrong function

---

## Execution Timeline

### PASS 1: Register Functions (Lines 1432-1443)

**What happens:**
- Each function declaration creates an entry in `symbol_ids`
- Both `handle_climb(obj)` and `handle_climb(tree)` are registered
- `function_overloads` map is still empty

**Code:**
```rust
for item in ast.items.iter() {
    if let crate::grue_compiler::ast::Item::Function(func) = item {
        let func_id = self.next_id();
        self.symbol_ids.insert(func.name.clone(), func_id);  // LINE 1436
    }
}
```

**Result:** `symbol_ids["handle_climb"]` points to one of the functions

### PASS 2: Generate Function Bodies (Lines 1445-1471)

**What happens:**
1. `generate_item()` calls `generate_function()` for each function
2. `generate_function()` immediately calls `generate_block()` for the body
3. Body generation encounters function calls
4. Function calls check `dispatch_functions.get()` → **EMPTY** (not created yet)
5. Falls back to `symbol_ids.get()` → **Wrong function**

**Critical Code:** Lines 1768 in `generate_function()`
```rust
let body = self.generate_block(func.body)?;  // LINE 1768
```

This is called BEFORE dispatch functions exist!

**Result:** Overloaded function calls resolve to wrong targets

### Dispatch Function Creation (Line 1471)

**What happens:**
- After all function bodies are generated
- `generate_dispatch_functions()` is called
- For each function with multiple overloads, creates a dispatch function
- Populates `dispatch_functions` map

**Code:**
```rust
self.generate_dispatch_functions(&mut ir_program)?;  // LINE 1471
```

**Result:** `dispatch_functions` is now populated, but too late

### Grammar Processing (Lines 1474-1477)

**What happens:**
- Grammar is deferred until after dispatch functions exist
- When grammar is processed, `dispatch_functions` is already populated
- Function calls resolve correctly through dispatch

**Code:**
```rust
for grammar in deferred_grammar {
    let ir_grammar = self.generate_grammar(grammar)?;  // LINE 1479
    ir_program.grammar.push(ir_grammar);
}
```

**Result:** Grammar function calls work correctly

---

## Related Code Components

### Function Overload Tracking

**Location:** Lines 1232-1258 in `ir.rs`

```rust
fn register_function_overload(
    &mut self,
    base_name: &str,
    func_id: IrId,
    specialization: ObjectSpecialization,
) {
    let overload = FunctionOverload {
        function_id: func_id,
        specialization,
        mangled_name,
        priority,
    };
    
    if let Some(overloads) = self.function_overloads.get_mut(base_name) {
        overloads.push(overload);
    } else {
        self.function_overloads.insert(base_name.to_string(), vec![overload]);
    }
}
```

### Specialization Detection

**Location:** Lines 1217-1229 in `ir.rs`

```rust
fn detect_specialization(
    &self,
    _func_name: &str,
    parameters: &[crate::grue_compiler::ast::Parameter],
) -> ObjectSpecialization {
    for param in parameters {
        if self.object_numbers.contains_key(&param.name) {
            return ObjectSpecialization::SpecificObject(param.name.clone());
        }
    }
    ObjectSpecialization::Generic
}
```

### Dispatch Function Creation

**Location:** Lines 1287-1416 in `ir.rs`

```rust
fn create_dispatch_function(
    &mut self,
    base_name: &str,
    overloads: &[FunctionOverload],
) -> Result<IrFunction, CompilerError> {
    // Creates branching logic:
    // if param == object1: call specialized_function_1
    // else if param == object2: call specialized_function_2
    // else: call generic_function
}
```

### Function Name Mangling

**Location:** Lines 1201-1214 in `ir.rs`

```rust
fn mangle_function_name(
    &self,
    base_name: &str,
    specialization: &ObjectSpecialization,
) -> String {
    match specialization {
        ObjectSpecialization::Generic => format!("{}_default", base_name),
        ObjectSpecialization::SpecificObject(obj_name) => format!("{}_{}", base_name, obj_name),
        ObjectSpecialization::ObjectType(type_name) => format!("{}_type_{}", base_name, type_name),
    }
}
```

---

## Data Structures Involved

### FunctionOverload (Lines 188-195)

```rust
pub struct FunctionOverload {
    pub function_id: IrId,
    pub specialization: ObjectSpecialization,
    pub mangled_name: String,
    pub priority: u8,  // Lower = higher priority
}
```

### IRGenerator Fields (Lines 1031-1084)

```rust
pub struct IrGenerator {
    symbol_ids: IndexMap<String, IrId>,           // Function/variable names to IDs
    dispatch_functions: IndexMap<String, IrId>,   // Dispatch function IDs
    function_overloads: IndexMap<String, Vec<FunctionOverload>>,  // Overload info
    // ... other fields ...
}
```

---

## Semantic Analysis (Not Affected)

**File:** `src/grue_compiler/semantic.rs`

**Lines:** 258-267

The semantic analyzer correctly allows function overloading. It doesn't prevent multiple functions with the same name, which allows the compiler to handle specialization.

```rust
// Allow function overloading for polymorphic dispatch
if let Some(existing_symbol) = self.current_scope.symbols.get(&func.name) {
    if let SymbolType::Function { .. } = &existing_symbol.symbol_type {
        log::debug!("🔄 Allowing function overload for '{}'", func.name);
    }
}

self.current_scope.symbols.insert(func.name.clone(), symbol);
```

No changes needed here.

---

## Solution Approaches

### Approach 1: THREE-PASS (RECOMMENDED)

1. **Pass 1:** Register all function names
2. **Pass 1.5 (NEW):** Detect all overloads and pre-allocate dispatch functions
3. **Pass 2:** Generate function bodies
4. **Pass 3:** Populate dispatch function bodies

**Pros:**
- Minimal changes to existing code structure
- Follows existing deferred pattern (like grammar)
- All dispatch functions exist before body generation

**Cons:**
- Requires adding a new phase

### Approach 2: Deferred Body Generation

Similar to grammar deferral:
1. Collect AST nodes without generating bodies
2. Create dispatch functions
3. Generate bodies

**Pros:**
- Uses existing deferred pattern
- Clean separation of concerns

**Cons:**
- More structural changes

### Approach 3: Dispatch Lookup Change

Modify function call resolution to check `function_overloads` directly instead of `dispatch_functions`:

```rust
let func_id = if let Some(overloads) = self.function_overloads.get(&name) {
    if overloads.len() > 1 {
        // Generate dispatch on-demand or use pre-allocated ID
        self.dispatch_functions.get(&name).unwrap()
    } else {
        overloads[0].function_id
    }
} else if let Some(&id) = self.symbol_ids.get(&name) {
    id
}
```

**Pros:**
- Minimal code changes
- Doesn't require new pass

**Cons:**
- Less clean architecture

---

## Testing and Verification

### Current Broken Behavior

Run with test case:
```rust
fn handle_climb(obj) {
    handle_climb(tree);
}

fn handle_climb(tree) {
    // specific
}
```

Expected: Calls specialized version
Actual: Infinite loop (calls itself)

### Verification Commands

```bash
# See debug logs of dispatch resolution
RUST_LOG=debug cargo run --bin grue-compiler -- test_file.grue

# Look for these debug messages:
# "Direct call using dispatch function" - means dispatch was found (works)
# "Direct call using original function" - means fallback was used (wrong!)
```

### Log Markers

Lines with debug output about function call resolution:
- Line 3430-3434: Dispatch function found
- Line 3437-3441: Fallback to symbol_ids

---

## Summary Table

| Aspect | Details |
|--------|---------|
| **Bug Location** | Lines 3414-3460 in `ir.rs` |
| **Root Cause** | dispatch_functions empty during body generation |
| **When Populated** | Line 1471 (after body generation) |
| **When Used** | Lines 3414-3460 (during body generation) |
| **Why Grammar Works** | Grammar processed after line 1471 |
| **Affected Scenarios** | Direct function calls with specialization |
| **Unaffected Scenarios** | Grammar patterns, non-overloaded functions |
| **File to Modify** | `/Users/cb/Projects/infocom-testing-old/infocom/src/grue_compiler/ir.rs` |
| **Recommended Fix** | Three-pass approach with pre-allocated dispatch IDs |

---

## Related Issues

None - this is the only known issue with polymorphic function dispatch.

## Historical Context

The polymorphic dispatch system was added to support:
- `handle_climb(obj)` - generic handler for any object
- `handle_climb(tree)` - specialized handler for specific object

The architecture works for grammar-driven calls but has a timing issue with direct calls.
