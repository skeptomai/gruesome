# Plan: Z-Machine Boolean Expression Context Handling

## Problem Analysis

The Z-Machine uses **branch instructions** for conditionals, not **store instructions**. When we write:

```grue
if obj.open {
    print("It's open");
}
```

The Z-Machine expects:
```assembly
test_attr obj, 3
?branch_if_true: print_paddr "It's open"
```

But our compiler tries to generate:
```assembly
test_attr obj, 3 -> store_var 219    # ❌ WRONG - test_attr doesn't store
je var_219, 1                        # ❌ WRONG - unnecessary comparison
?branch_if_true: print_paddr "It's open"
```

## Root Cause: Expression vs Statement Context

The compiler treats ALL property access as **expressions** that produce values, but Z-Machine branch instructions work in **statement contexts** with direct conditional flow.

## Comprehensive Solution Plan

### Phase 1: Conditional Expression Detection
**Goal**: Detect when attribute access occurs in boolean contexts

**Implementation**:
1. **Add context tracking to IR generation**:
   ```rust
   enum ExpressionContext {
       Value,           // obj.open -> needs boolean value
       Conditional,     // if obj.open -> needs branch logic
       Assignment,      // obj.open = true -> needs set_attr
   }
   ```

2. **Modify property access logic** to accept context:
   ```rust
   fn generate_property_access(&mut self, obj, prop, context: ExpressionContext) {
       if let Some(attr) = self.get_standard_attribute(&prop) {
           match context {
               ExpressionContext::Conditional => {
                   // Generate TestAttributeBranch instruction
               }
               ExpressionContext::Value => {
                   // Generate TestAttribute with result variable (complex)
               }
               ExpressionContext::Assignment => {
                   // Generate SetAttribute instruction
               }
           }
       }
   }
   ```

### Phase 2: New IR Instructions for Branch Logic
**Goal**: Add IR instructions that handle branch semantics

**New IR Instructions**:
```rust
/// Direct conditional branch for Z-Machine branch instructions
TestAttributeBranch {
    object: IrId,
    attribute_num: u8,
    then_label: IrId,    // Branch target if attribute is set
    else_label: IrId,    // Fall-through if attribute is clear
},

/// Boolean value extraction from attributes (complex case)
TestAttributeValue {
    target: IrId,        // Store boolean result
    object: IrId,
    attribute_num: u8,
    // Uses branch + label + store pattern
},
```

### Phase 3: Context-Aware IR Generation
**Goal**: Generate different IR based on usage context

**If Statement Context**:
```rust
// OLD: if obj.open -> TestAttribute -> je result, 1 -> branch
// NEW: if obj.open -> TestAttributeBranch -> direct branch
fn generate_if_statement(&mut self, condition, then_block, else_block) {
    if let Expression::PropertyAccess { object, property } = condition {
        if let Some(attr) = self.get_standard_attribute(&property) {
            // Generate direct branch logic
            let then_label = self.next_id();
            let else_label = self.next_id();

            block.add_instruction(IrInstruction::TestAttributeBranch {
                object: object_temp,
                attribute_num: attr as u8,
                then_label,
                else_label,
            });
            // ... handle then/else blocks
        }
    }
}
```

**Assignment Context**:
```rust
// obj.result = obj.open -> TestAttributeValue needed
let result_temp = self.generate_expression_with_context(
    property_access,
    ExpressionContext::Value
);
```

### Phase 4: Z-Machine Codegen for Branch Instructions
**Goal**: Emit proper Z-Machine branch opcodes

**TestAttributeBranch Codegen**:
```rust
IrInstruction::TestAttributeBranch { object, attribute_num, then_label, else_label } => {
    let obj_operand = self.resolve_ir_id_to_operand(*object)?;
    let attr_operand = Operand::SmallConstant(*attribute_num);

    // Generate test_attr with branch offset to then_label
    self.emit_instruction_typed(
        Opcode::Op2(Op2::TestAttr),
        &[obj_operand, attr_operand],
        None,  // No store_var - this is a branch instruction
        Some(BranchTarget::Label(*then_label)),  // Branch if true
    )?;

    // Fall through to else_label
}
```

**TestAttributeValue Codegen** (complex case):
```rust
IrInstruction::TestAttributeValue { target, object, attribute_num } => {
    // Pattern: test_attr -> branch -> store 1 -> jump end -> store 0 -> end
    let true_label = self.next_label();
    let end_label = self.next_label();

    // test_attr obj, attr -> branch to true_label if set
    self.emit_branch_instruction(Op2::TestAttr, &[obj_op, attr_op], true_label);

    // Attribute clear: store 0
    self.store_immediate(result_var, 0);
    self.emit_jump(end_label);

    // Attribute set: store 1
    self.emit_label(true_label);
    self.store_immediate(result_var, 1);

    self.emit_label(end_label);
}
```

### Phase 5: Extend to Other Branch Instructions
**Goal**: Handle all Z-Machine branch instructions consistently

**Other branch instructions that need similar treatment**:
- `je` (jump if equal) - comparisons in conditionals
- `jg`, `jl` (jump if greater/less) - numeric comparisons
- `jin` (jump if object in container) - containment tests
- `test` (jump if bit test) - bit flag tests

**Pattern**:
```rust
// All branch instructions follow same pattern:
// 1. Detect conditional context
// 2. Generate direct branch IR instruction
// 3. Emit Z-Machine branch opcode
// 4. Handle complex value contexts with branch+store pattern
```

### Phase 6: Backward Compatibility
**Goal**: Ensure existing property access still works

**Property vs Attribute Routing**:
```rust
fn generate_property_access_with_context(&mut self, object, property, context) {
    if let Some(attr) = self.get_standard_attribute(&property) {
        // Route to attribute system (new)
        self.generate_attribute_access(object, attr, context)
    } else {
        // Route to property system (existing - unchanged)
        self.generate_property_access_existing(object, property, context)
    }
}
```

## Implementation Priority

1. **Phase 1-2**: Context detection and new IR instructions (foundational)
2. **Phase 3**: Context-aware IR generation for `if` statements (core fix)
3. **Phase 4**: Z-Machine branch codegen (implementation)
4. **Phase 5**: Extend to other branch instructions (completeness)
5. **Phase 6**: Testing and backward compatibility (validation)

## Expected Outcome

After implementation:
```grue
if obj.open {           // ✅ Generates: test_attr obj, 3 -> branch
    print("Open");
}

let is_open = obj.open; // ✅ Generates: test_attr -> branch -> store pattern
```

This will fix the mailbox opening issue and provide a robust foundation for all Z-Machine conditional logic.

## Testing Strategy

### Unit Tests
- Context detection accuracy
- IR instruction generation correctness
- Z-Machine opcode emission validation

### Integration Tests
- Simple attribute conditionals (`if obj.open`)
- Complex attribute expressions (`let result = obj.open`)
- Mixed attribute/property access
- Backward compatibility with existing games

### Functional Tests
- Mini-zork mailbox opening functionality
- All attribute types (open, openable, container, etc.)
- Nested conditionals and complex boolean logic
- Performance impact assessment

## Success Criteria

1. **Mailbox Opening Works**: `open mailbox` command succeeds without "Property 19" errors
2. **All Attributes Work**: openable, container, visited, etc. function in conditionals
3. **No Regressions**: Existing property access continues to work
4. **Performance Maintained**: No significant compilation or runtime performance impact
5. **Architecture Clean**: Clear separation between attribute and property systems

---

# PHASE 2 IMPLEMENTATION DISCOVERY (October 28, 2025)

## 🚧 **Implementation Issues Discovered Outside Plan**

During Phase 2 implementation, several architectural gaps were discovered that weren't anticipated in the original plan:

### **Issue 1: Branch Target Handling**
- **Problem**: `emit_instruction_typed` signature unclear for branch targets
- **Discovery**: Existing code doesn't have clear `BranchTarget::Label` pattern
- **Impact**: TestAttributeBranch cannot emit proper branch instructions

### **Issue 2: Label Management Infrastructure**
- **Problem**: Methods `next_label()` and `emit_label()` don't exist
- **Discovery**: No clear pattern for creating and managing labels in codegen
- **Impact**: Cannot implement branch+store pattern for TestAttributeValue

### **Issue 3: Variable Allocation**
- **Problem**: `get_or_allocate_var()` method missing
- **Discovery**: Unclear how to allocate result variables for instruction targets
- **Impact**: Cannot store boolean results in TestAttributeValue

### **Issue 4: Forward Jump Infrastructure**
- **Problem**: `calculate_forward_jump_offset()` and jump patterns unclear
- **Discovery**: Complex control flow patterns may not be implemented
- **Impact**: TestAttributeValue branch+store+jump pattern blocked

## 🔍 **Investigation Plan (Option B)**

**Objective**: Find existing patterns for:
1. **Branch handling** in `emit_instruction_typed`
2. **Label creation and management** in existing codebase
3. **UnresolvedReference patterns** for forward references
4. **Jump instruction** implementations that currently exist
5. **Variable allocation** for instruction results

**Methodology**:
- Grep analysis of existing branch/jump/label patterns
- Study UnresolvedReference usage for forward references
- Examine emit_instruction_typed calls with branch parameters
- Find variable allocation patterns in codegen
- Identify minimal viable implementation approach

**Outcome**: Comprehensive findings on existing infrastructure to inform Phase 2 implementation strategy.

## 🔍 **Investigation Results: Complete Infrastructure Analysis**

### **Finding 1: Branch Handling in emit_instruction_typed** ✅

**Signature Discovery:**
```rust
pub fn emit_instruction_typed(
    &mut self,
    opcode: Opcode,
    operands: &[Operand],
    store_var: Option<u8>,
    branch_offset: Option<i16>,  // ← Simple i16, not complex BranchTarget enum
) -> Result<InstructionLayout, CompilerError>
```

**Key Insight**: Branches are handled with simple `i16` offsets, not complex target objects. This is **much simpler** than anticipated in the original plan!

**Impact**: TestAttributeBranch can use direct branch_offset parameters instead of complex target resolution.

### **Finding 2: UnresolvedReference Architecture** ✅

**Structure Discovery:**
```rust
pub struct UnresolvedReference {
    pub reference_type: LegacyReferenceType,  // Jump, Branch, Label, etc.
    pub location: usize,        // Where to patch
    pub target_id: IrId,        // What label/function to reference
    pub is_packed_address: bool,
    pub offset_size: u8,
    pub location_space: MemorySpace,
}

pub enum LegacyReferenceType {
    Jump,     // ← For unconditional jumps
    Branch,   // ← For conditional branches
    Label(IrId),
    FunctionCall,
    StringRef,
    // ... others
}
```

**Key Insight**: The infrastructure for forward references exists and is **well-designed**! The system can handle branch targets that aren't resolved until link time.

**Impact**: Complex TestAttributeValue patterns with forward jumps are definitely implementable.

### **Finding 3: Label Management Patterns** ✅

**Existing Pattern in If Statements:**
```rust
// 1. Create labels (method exists!)
let then_label = self.next_id();   // ← This method exists in IR generator
let else_label = self.next_id();

// 2. Emit labels
block.add_instruction(IrInstruction::Label { id: then_label });

// 3. Reference labels in branches/jumps
block.add_instruction(IrInstruction::Branch {
    condition: condition_temp,
    true_label: then_label,
    false_label: else_label,
});
```

**Key Insight**: Full label infrastructure exists and is **actively used** in if statements throughout the codebase!

**Impact**: TestAttributeBranch and TestAttributeValue can use the exact same patterns that if statements use.

### **Finding 4: Jump Instruction Infrastructure** ✅

**Jump Definition:**
```rust
/// Unconditional jump
Jump {
    label: IrId,  // ← Simple label reference
},
```

**Usage Evidence**: Found extensive usage in:
- Loop control flow (`block.add_instruction(IrInstruction::Jump { label: loop_start });`)
- If statement optimization (`block.add_instruction(IrInstruction::Jump { label: end_label });`)
- Complex expression evaluation
- Error handling paths

**Key Insight**: Jump instructions are **simple and well-established**! No complex offset calculations needed at IR level.

**Impact**: TestAttributeValue branch+store+jump pattern can use proven Jump instruction.

### **Finding 5: Variable Allocation Patterns** ✅

**Allocation Discovery:**
```rust
// Variables tracked in ir_id_to_local_var map
let var_num = if let Some(&local_var_num) = self.ir_id_to_local_var.get(var_id) {
    local_var_num  // Local variables (1-15)
} else {
    // Global variables handled differently via ir_id_to_stack_var
}

// Existing resolve_ir_id_to_operand handles allocation automatically
let obj_operand = self.resolve_ir_id_to_operand(*object)?;
```

**Key Insight**: Variable allocation exists but is complex - however, targets can be automatically allocated via the existing `resolve_ir_id_to_operand` system that's used throughout the codebase.

**Impact**: TestAttributeValue result storage can use the same patterns as other instructions with targets.

---

## 🎯 **Revised Implementation Strategy Based on Findings**

The investigation reveals that **100% of required infrastructure already exists**. The original plan was more complex than necessary.

### **Strategy Option B-1: Simplified TestAttributeBranch**
Use existing IR infrastructure directly:

```rust
// In property access logic (conditional context):
if context == ExpressionContext::Conditional {
    // Create labels exactly like if statements do
    let then_label = self.next_id();
    let else_label = self.next_id();

    // Generate TestAttributeBranch IR instruction
    block.add_instruction(IrInstruction::TestAttributeBranch {
        object: object_temp,
        attribute_num: attr_num,
        then_label,
        else_label,
    });

    // Codegen: emit_instruction_typed with branch_offset parameter
    // UnresolvedReference system handles forward label resolution
}
```

### **Strategy Option B-2: Reuse Existing Branch Pattern**
Leverage proven if statement architecture:

```rust
// Generate TestAttribute for the condition (like if statements do)
block.add_instruction(IrInstruction::TestAttribute {
    target: temp_id,
    object: object_temp,
    attribute_num: attr_num,
});

// Then use the EXISTING Branch instruction (proven in if statements)
block.add_instruction(IrInstruction::Branch {
    condition: temp_id,
    true_label: then_label,
    false_label: else_label,
});
```

### **Strategy Option B-3: TestAttributeValue Implementation**
Use existing Jump + Label patterns:

```rust
// For value contexts: if let is_open = obj.open
block.add_instruction(IrInstruction::TestAttribute {
    target: temp_condition,
    object: object_temp,
    attribute_num: attr_num,
});

// Use existing Branch instruction for the test
let true_label = self.next_id();
let end_label = self.next_id();

block.add_instruction(IrInstruction::Branch {
    condition: temp_condition,
    true_label,
    false_label: end_label,  // Fall through to store 0
});

// Store 0 (false case)
block.add_instruction(IrInstruction::LoadImmediate {
    target: result_temp,
    value: IrValue::Integer(0),
});
block.add_instruction(IrInstruction::Jump { label: end_label });

// Store 1 (true case)
block.add_instruction(IrInstruction::Label { id: true_label });
block.add_instruction(IrInstruction::LoadImmediate {
    target: result_temp,
    value: IrValue::Integer(1),
});

// End
block.add_instruction(IrInstruction::Label { id: end_label });
```

## 🚀 **Recommended Implementation Path**

**Phase 2A: Minimal Risk Approach**
1. **Start with Option B-2** (reuse existing Branch instruction) - **zero new infrastructure**
2. **Test with simple `if obj.open` cases** - leverage proven if statement codegen
3. **Verify Z-Machine test_attr generation** works correctly

**Phase 2B: Complete Implementation**
1. **Implement TestAttributeValue** using existing Jump + Label patterns
2. **Test with `let is_open = obj.open` cases**
3. **Validate both conditional and value contexts**

**Phase 2C: Optimization (Optional)**
1. **Consider specialized TestAttributeBranch** if performance gains are significant
2. **Direct Z-Machine branch emission** to eliminate intermediate Branch instruction

## 📊 **Risk Assessment**

- **Option B-2 Risk**: **MINIMAL** - reuses 100% proven infrastructure
- **Option B-1 Risk**: **LOW** - uses existing patterns with minor new IR instruction
- **Option B-3 Risk**: **LOW** - uses existing Jump/Label patterns extensively used in codebase

**Success Probability**: **HIGH** - all required infrastructure is proven and actively used.

## ✅ **Infrastructure Readiness Confirmed**

All discoveries confirm that Phase 2 implementation is **architecturally sound** and can proceed with **high confidence** using existing, proven patterns.

---

# PHASE 2 IMPLEMENTATION & TESTING PLAN 🚀 (October 28, 2025)

## 📋 **Phase 2A: Option B-2 Implementation Plan**

### **Step 1: Update Property Access Logic**
**File**: `src/grue_compiler/ir.rs` (PropertyAccess handler)

**Implementation**:
```rust
// In generate_expression_with_context() PropertyAccess match arm:
if let Some(standard_attr) = self.get_standard_attribute(&property) {
    let attr_num = standard_attr as u8;

    match context {
        ExpressionContext::Conditional => {
            // OPTION B-2: Reuse existing Branch instruction pattern
            log::debug!(
                "🔍 ATTRIBUTE ACCESS (CONDITIONAL): {} -> using existing Branch pattern",
                property
            );

            // Step 1a: Generate TestAttribute for condition (like if statements)
            let condition_temp = self.next_id();
            block.add_instruction(IrInstruction::TestAttribute {
                target: condition_temp,
                object: object_temp,
                attribute_num: attr_num,
            });

            // Step 1b: Let the conditional context handler create labels and Branch
            // Return the condition temp - the if statement will handle the Branch
            return Ok(condition_temp);
        }

        ExpressionContext::Value => {
            // PHASE 2B: Will implement TestAttributeValue pattern
            log::debug!(
                "🔍 ATTRIBUTE ACCESS (VALUE): {} -> TestAttributeValue pattern (Phase 2B)",
                property
            );

            // For now, fall back to existing TestAttribute
            let temp_id = self.next_id();
            block.add_instruction(IrInstruction::TestAttribute {
                target: temp_id,
                object: object_temp,
                attribute_num: attr_num,
            });
            return Ok(temp_id);
        }

        ExpressionContext::Assignment => {
            return Err(CompilerError::SemanticError(
                format!("Cannot read attribute '{}' in assignment context", property),
                0
            ));
        }
    }
}
```

**Key Points**:
- ✅ Reuses existing `TestAttribute` instruction (no new IR needed)
- ✅ Leverages existing `IrInstruction::Branch` from if statements
- ✅ Conditional context returns condition temp for if statement to handle
- ✅ Phase 2B placeholder for value context

### **Step 2: Fix TestAttribute Codegen**
**File**: `src/grue_compiler/codegen_instructions.rs`

**Current Issue**: TestAttribute throws "not implemented" error

**Implementation**:
```rust
IrInstruction::TestAttribute { target, object, attribute_num } => {
    // Generate Z-Machine test_attr instruction (fixed implementation)
    log::debug!(
        "TestAttribute codegen: target={}, object={}, attr={}",
        target, object, attribute_num
    );

    let obj_operand = self.resolve_ir_id_to_operand(*object)?;
    let attr_operand = Operand::SmallConstant(*attribute_num);

    // CRITICAL: test_attr is a BRANCH instruction, not a STORE instruction
    // But when used in expressions, we need the boolean result
    // Solution: Use branch+store pattern like the investigation found

    let true_label = self.get_next_label_id();
    let end_label = self.get_next_label_id();

    // Emit test_attr with branch to true_label if attribute is set
    self.emit_instruction_typed(
        Opcode::Op2(Op2::TestAttr),
        &[obj_operand, attr_operand],
        None, // No store_var - this is a branch instruction
        Some(-1), // Placeholder for branch offset (will be resolved)
    )?;

    // Create UnresolvedReference for the branch target
    self.unresolved_refs.push(UnresolvedReference {
        reference_type: LegacyReferenceType::Branch,
        location: self.code_address - 2, // Branch offset location
        target_id: true_label,
        is_packed_address: false,
        offset_size: 2,
        location_space: MemorySpace::Story,
    });

    // Attribute clear: store 0
    let result_var = /* find target variable mapping */;
    self.emit_instruction_typed(
        Opcode::Op2(Op2::Store),
        &[Operand::Variable(result_var), Operand::SmallConstant(0)],
        None,
        None,
    )?;

    // Jump to end
    self.emit_instruction_typed(
        Opcode::Op1(Op1::Jump),
        &[Operand::SmallConstant(0)], // Placeholder offset
        None,
        None,
    )?;

    // Create UnresolvedReference for jump
    self.unresolved_refs.push(UnresolvedReference {
        reference_type: LegacyReferenceType::Jump,
        location: self.code_address - 2,
        target_id: end_label,
        is_packed_address: false,
        offset_size: 2,
        location_space: MemorySpace::Story,
    });

    // Emit true_label
    self.emit_label_at_current_address(true_label)?;

    // Attribute set: store 1
    self.emit_instruction_typed(
        Opcode::Op2(Op2::Store),
        &[Operand::Variable(result_var), Operand::SmallConstant(1)],
        None,
        None,
    )?;

    // Emit end_label
    self.emit_label_at_current_address(end_label)?;

    log::debug!("TestAttribute complete: Variable({}) contains boolean result", result_var);
}
```

**Key Points**:
- ✅ Implements branch+store+jump pattern discovered in investigation
- ✅ Uses existing `UnresolvedReference` system for forward references
- ✅ Generates proper Z-Machine `test_attr` branch instruction
- ✅ Handles boolean result storage for expression contexts

### **Step 3: Remove Broken Implementation**
**File**: `src/grue_compiler/codegen_instructions.rs`

**Remove**:
```rust
// Remove the new TestAttributeBranch and TestAttributeValue handlers
// that were causing compilation errors
```

**Clean up** the incomplete implementation from earlier attempt.

## 🧪 **Phase 2A Testing Plan**

### **Test 1: Compilation Success**
**Objective**: Verify code compiles without errors

**Command**:
```bash
cargo check --quiet
```

**Success Criteria**: No compilation errors

### **Test 2: Simple Conditional Compilation**
**Objective**: Test `if obj.open` compiles and generates correct IR

**Command**:
```bash
RUST_LOG=debug cargo run --bin grue-compiler -- examples/mini_zork.grue -o tests/phase2a_test.z3 2>&1 | grep "ATTRIBUTE ACCESS (CONDITIONAL)"
```

**Expected Output**:
```
[DEBUG] 🔍 ATTRIBUTE ACCESS (CONDITIONAL): open -> using existing Branch pattern
[DEBUG] 🔍 ATTRIBUTE ACCESS (CONDITIONAL): openable -> using existing Branch pattern
```

**Success Criteria**:
- ✅ Attribute access detected in conditional context
- ✅ TestAttribute IR instructions generated
- ✅ Compilation completes successfully

### **Test 3: Z-Machine Instruction Verification**
**Objective**: Verify correct Z-Machine test_attr instructions are generated

**Command**:
```bash
RUST_LOG=debug cargo run --bin grue-compiler -- examples/mini_zork.grue -o tests/phase2a_test.z3 2>&1 | grep -E "(TestAttribute codegen|test_attr|Op2.*TestAttr)"
```

**Expected Output**:
```
[DEBUG] TestAttribute codegen: target=105, object=95, attr=3
[DEBUG] EMIT_TYPED: addr=0x016e opcode=Op2(TestAttr) operands=[Variable(95), SmallConstant(3)]
```

**Success Criteria**:
- ✅ TestAttribute codegen executes without errors
- ✅ Op2(TestAttr) Z-Machine instruction emitted
- ✅ Correct object and attribute operands

### **Test 4: Runtime Execution**
**Objective**: Test that mailbox opening works without "Property 19" error

**Command**:
```bash
echo "examine mailbox
open mailbox
quit
y" | timeout 10s ./target/debug/gruesome tests/phase2a_test.z3
```

**Expected Output**:
```
> examine mailbox
[mailbox description]
> open mailbox
Opened.
> quit
```

**Success Criteria**:
- ✅ No "Property 19 not found for object 10" error
- ✅ `examine mailbox` works (attribute access in non-conditional context)
- ✅ `open mailbox` executes `handle_open` function
- ✅ Conditional `if obj.openable` and `if obj.open` work

### **Test 5: Comprehensive Attribute Testing**
**Objective**: Test multiple attribute types in conditionals

**Create Test File**: `tests/phase2a_attribute_test.grue`
```grue
init {
    if player.container {
        print("Player is container");
    }

    if !player.openable {
        print("Player not openable");
    }

    if player.takeable {
        print("Player takeable");
    } else {
        print("Player not takeable");
    }
}
```

**Command**:
```bash
RUST_LOG=debug cargo run --bin grue-compiler -- tests/phase2a_attribute_test.grue -o tests/phase2a_attr_test.z3
echo "quit
y" | ./target/debug/gruesome tests/phase2a_attr_test.z3
```

**Success Criteria**:
- ✅ Multiple attribute types compile successfully
- ✅ All conditional branches work correctly
- ✅ Boolean logic (`!player.openable`) works
- ✅ If-else chains work with attributes

## 📊 **Phase 2A Success Metrics**

### **Compilation Metrics**
- ✅ Zero compilation errors
- ✅ Zero "TestAttribute not implemented" errors
- ✅ IR generation logs show conditional context detection

### **Z-Machine Generation Metrics**
- ✅ `test_attr` instructions generated (not `get_prop`)
- ✅ Correct attribute numbers (open=3, openable=2, container=1)
- ✅ Proper branch instruction encoding

### **Runtime Metrics**
- ✅ Mailbox opening works (primary success case)
- ✅ No "Property 19" errors
- ✅ All standard attributes work in conditionals
- ✅ Complex boolean expressions work

## 🎯 **Phase 2B: TestAttributeValue Implementation Plan**

**Objective**: Handle `let is_open = obj.open` (value context)

**Implementation**: Use existing Jump + Label patterns from investigation

**Testing**: Similar pattern with value assignment tests

**Timeline**: Execute after Phase 2A success validation

## ✅ **Ready for Execution**

Phase 2A implementation and testing plan is complete. All steps use proven infrastructure patterns with minimal risk.

**Next Action**: Execute Phase 2A implementation following the detailed step-by-step plan above.