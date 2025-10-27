# 🔍 STACK UNDERFLOW INVESTIGATION PLAN (October 27, 2025)

## Background

**BREAKTHROUGH ACHIEVED**: The fundamental object iteration bug has been fixed! ✅
- **Critical Discovery**: get_object_contents was emitting wrong opcode (0x01 get_sibling instead of 0x02 get_child)
- **Fix Location**: `src/grue_compiler/codegen_builtins.rs:812`
- **Verification**: Object iteration now works correctly - get_child returns object 3 (coin) after insert_obj operations

**REMAINING ISSUE**: Stack underflow in print system at PC 0x07fc
- **Separate from object iteration**: Core object finding functionality is working
- **Theory**: String concatenation/printing logic has stack management issues
- **Evidence**: Stack underflow occurs at print_paddr instruction during string operations

## Investigation Plan

### **Phase 1: Isolate String Concatenation vs Object Iteration** 🎯

**Objective**: Confirm that stack underflow is in string printing, not object iteration

**Test Cases to Create**:

#### 1. Simple Integer Concatenation Test
```grue
// test_simple_concat.grue
fn main() {
    let value = 42;
    print("Result: " + value);
}
```

#### 2. Object Property Concatenation Test
```grue
// test_object_prop_concat.grue
world {
    room test_room "Test Room" {
        desc: "A test room."
        object coin { name: "coin" desc: "A shiny coin." }
    }
}

init {
    player.location = test_room;
    coin.location = player;
    main();
}

fn main() {
    let result = player.contents();
    if result != 0 {
        print("Found object: " + result.name);  // This should trigger stack underflow
    }
}
```

#### 3. Non-Concatenation Object Test
```grue
// test_no_concat.grue
world {
    room test_room "Test Room" {
        desc: "A test room."
        object coin { name: "coin" desc: "A shiny coin." }
    }
}

init {
    player.location = test_room;
    coin.location = player;
    main();
}

fn main() {
    let result = player.contents();
    print("Result is:");
    print(result);  // No concatenation
}
```

**Expected Results**:
- Simple integer concatenation: Should work or show where string concatenation fails
- Object property concatenation: Should reproduce stack underflow
- Non-concatenation: Should work fine (proving object iteration is fixed)

**Commands**:
```bash
# Create and test isolation cases
cargo run --bin grue-compiler -- test_simple_concat.grue -o tests/simple_concat.z3
RUST_LOG=debug ./target/debug/gruesome tests/simple_concat.z3

cargo run --bin grue-compiler -- test_object_prop_concat.grue -o tests/object_prop_concat.z3
RUST_LOG=debug ./target/debug/gruesome tests/object_prop_concat.z3

cargo run --bin grue-compiler -- test_no_concat.grue -o tests/no_concat.z3
RUST_LOG=debug ./target/debug/gruesome tests/no_concat.z3
```

### **Phase 2: Stack Operation Tracing** 📊

**Instrumentation Strategy**:

1. **Add Stack Depth Logging** around string operations:
   - Before each push/pop in string concatenation
   - At start/end of print_paddr execution
   - In Variable(0) read operations

2. **Trace Points to Add**:
   ```rust
   // In string concatenation codegen
   log::debug!("STRING_CONCAT: Before operation, stack_depth={}", stack_depth);

   // In print builtin generation
   log::debug!("PRINT_BUILTIN: Emitting print_paddr, expecting value on stack");

   // In Variable(0) operations
   log::debug!("VAR_0_READ: PC={:04x}, stack_depth={}", pc, stack_depth);
   ```

3. **Commands to Execute**:
```bash
# Test each case with detailed stack tracing
RUST_LOG=debug cargo run --bin grue-compiler -- test_simple_concat.grue -o tests/simple_concat.z3
RUST_LOG=debug ./target/debug/gruesome tests/simple_concat.z3 2>&1 | grep -E "(push|pop|stack|STRING|PRINT)" -A2 -B2

RUST_LOG=debug cargo run --bin grue-compiler -- test_object_prop_concat.grue -o tests/object_prop_concat.z3
RUST_LOG=debug ./target/debug/gruesome tests/object_prop_concat.z3 2>&1 | grep -E "(stack|0x07fc|print_paddr)" -A3 -B3
```

### **Phase 3: String Concatenation Code Analysis** 🔍

**Files to Examine**:

1. **`src/grue_compiler/codegen_strings.rs`**:
   - Look for stack management in string concatenation
   - Check if values are properly pushed before concatenation
   - Verify stack discipline in multi-part string building

2. **`src/grue_compiler/codegen_builtins.rs` (print builtin)**:
   - Examine how `print("text: " + variable)` is compiled
   - Check if Variable(0) reads have corresponding pushes
   - Look for missing push operations

3. **Search for Stack Violations**:
```bash
# Find all Variable(0) reads in codegen
grep -n "Variable(0)" src/grue_compiler/*.rs

# Find all push/pop operations
grep -n -E "(push|pop)" src/grue_compiler/*.rs

# Look for string concatenation logic
grep -n -A5 -B5 "concatenat" src/grue_compiler/*.rs
```

### **Phase 4: Bytecode Sequence Analysis** 📋

**Target the Exact Failure Point**:

1. **Disassemble Around PC 0x07fc**:
```bash
# Generate detailed bytecode dump
RUST_LOG=debug ./target/debug/gruesome tests/test_debug_get_child.z3 --debug-objects 2>&1 | grep -E "(0x07f[0-9a-f]|print_paddr)" -A5 -B5
```

2. **Identify the Instruction Sequence**:
   - What instructions lead up to 0x07fc?
   - What should be on the stack when print_paddr executes?
   - What was the last push operation before the underflow?

3. **Compare Working vs Failing Cases**:
   - Generate bytecode for simple working print vs failing concatenation
   - Find the difference in stack preparation

### **Phase 5: Fix Implementation** 🔧

**Likely Fix Categories**:

1. **Missing Push Operation**:
   - String concatenation result not pushed to stack
   - Intermediate value consumed without reload

2. **Extra Pop Operation**:
   - Stack value consumed twice
   - Cleanup code popping when it shouldn't

3. **Variable(0) vs Local Variable Confusion**:
   - Code expecting local variable but reading from stack
   - Stack/local variable routing error

**Fix Validation**:
```bash
# After implementing fix
cargo run --bin grue-compiler -- test_debug_get_child.grue -o tests/fixed_debug.z3
RUST_LOG=debug ./target/debug/gruesome tests/fixed_debug.z3

# Should complete without stack underflow and show:
# "Player contents returned: 3"
# "Result is non-zero: coin"  # (showing actual object name)
```

## Success Criteria

The stack underflow investigation will be complete when:

1. ✅ **String concatenation works**: `print("text: " + variable)` completes successfully
2. ✅ **Object name display works**: Object iteration shows actual object names, not just IDs
3. ✅ **Stack discipline maintained**: No underflow errors in any string operations
4. ✅ **Regression tests pass**: All existing functionality remains working

## Diagnostic Commands Summary

```bash
# Phase 1: Create and test isolation cases
cargo run --bin grue-compiler -- test_simple_concat.grue -o tests/simple_concat.z3
cargo run --bin grue-compiler -- test_object_prop_concat.grue -o tests/object_prop_concat.z3
cargo run --bin grue-compiler -- test_no_concat.grue -o tests/no_concat.z3

# Phase 2: Stack tracing
RUST_LOG=debug ./target/debug/gruesome tests/simple_concat.z3 2>&1 | grep -E "(push|pop|stack)" -A2 -B2
RUST_LOG=debug ./target/debug/gruesome tests/object_prop_concat.z3 2>&1 | grep -E "(stack|print_paddr|0x07fc)" -A3 -B3

# Phase 3: Code analysis
grep -n "Variable(0)" src/grue_compiler/*.rs
grep -n -E "(push|pop)" src/grue_compiler/*.rs
grep -n -A5 -B5 "concatenat" src/grue_compiler/*.rs

# Phase 4: Bytecode analysis around failure point
RUST_LOG=debug ./target/debug/gruesome tests/test_debug_get_child.z3 2>&1 | grep -E "(0x07f[0-9a-f]|print_paddr)" -A5 -B5
```

## Key Insight

The fundamental architecture is now correct:
- ✅ **Object iteration works**: get_child opcode fixed, objects correctly found
- ✅ **Object tree relationships**: insert_obj operations successful
- ✅ **Empty container handling**: Returns 0 correctly
- 🔧 **String operations**: Stack underflow in concatenation/printing needs investigation

This systematic approach will isolate the stack underflow to the specific string concatenation logic, separate from the now-working object iteration system.