# ONGOING TASKS - PROJECT STATUS

## ✅ **OBJECT ID RESOLUTION BUG** - **FIXED** (November 1, 2025)

**ISSUE RESOLVED**:
- ✅ **Root Cause Found**: Dual numbering system bug in `codegen_objects.rs` line 724
- ✅ **Problem**: Object table creation used sequential numbering instead of IR mapping
- ✅ **Impact**: IR ID 33 (mailbox) mapped to Object #10 but table creation used wrong ordering
- ✅ **Solution**: Fixed line 724 to use `*object_id_to_number.get(&object.id).unwrap()` instead of `(index + 1)`

**INVESTIGATION FINDINGS**:
- ✅ **Compilation**: Mailbox object 10 correctly compiled with attributes 0x00000018 (openable=1, open=0, container=1)
- ✅ **IR Processing**: Boolean attributes correctly processed (`open: false` → 0x00000018)
- ✅ **Dual Numbering Bug**: Two conflicting systems - IR mapping vs sequential table creation
- ✅ **Fix Verified**: `open mailbox` now correctly resolves to object 10, responds "It's already open"

**TECHNICAL DETAILS**:
- **File**: `src/grue_compiler/codegen_objects.rs:724`
- **Change**: Use existing IR mapping instead of creating new sequential numbers
- **Result**: Consistent object numbering throughout compilation pipeline

**PREVIOUSLY FIXED**:
- ✅ **Verb Dispatch Infinite Loop**: Fixed increment instruction compilation bug
- ✅ **IR Boolean Processing**: Fixed to handle both `true` and `false` attribute values

---

## 📁 **DOCUMENTATION ORGANIZATION** (October 31, 2025)

**COMPLETED**: Moved analysis markdown files to `docs/` directory:
- All historical bug investigations and implementation plans
- Technical architecture documentation
- Stack discipline analysis and fixes
- Z-Machine instruction implementation details

**CURRENT STRUCTURE**:
- `docs/` - All analysis and technical documentation
- Root directory - Only active project files (CLAUDE.md, ONGOING_TASKS.md, README.md, TESTING.md)

---

## 🎯 **ACTIVE DEVELOPMENT AREAS**

### **CLIMB TREE BUG** ✅ **RESOLVED** (November 6, 2025)

**ISSUE RESOLVED**: Object variable compilation bug causing infinite loops in climb tree functionality

**Final Status**: SUCCESSFULLY FIXED - Object variable compilation now works correctly

**Root Cause Identified and Fixed**:
- **Problem**: Object variables like `tree` were compiled with IR object numbers instead of runtime object numbers, causing `obj == tree` comparisons to fail and infinite loops in get_exit function
- **Evidence**: Debug output showed `obj = 13` (correct runtime number) but `tree` variable compiles to wrong IR number

**Solution Implemented**:
1. ✅ **Added IrValue::Object(String) variant** to store object names for deferred resolution
2. ✅ **Modified object variable compilation** in `ir.rs` to use `IrValue::Object` instead of immediate integers
3. ✅ **Added codegen handling** in `codegen_instructions.rs` to convert object names to runtime numbers during code generation
4. ✅ **Fixed compilation errors** by making object_numbers field public and adding missing pattern matches in `codegen.rs`
5. ✅ **Verified fix works** - "climb tree" now gives proper response "You can't climb that." instead of infinite loop

**Test Results - SUCCESSFUL**:
- **Before Fix**: "climb tree" → Infinite loop (get_exit never finding matching direction)
- **After Fix**: "climb tree" → "You can't climb that." (proper game response)
- **All basic directions still work perfectly**: north, south, east, west navigation confirmed working
- **Object variable comparisons now work correctly**: `obj == tree` comparison resolves properly

**Key Files Modified**:
- `src/grue_compiler/ir.rs` - Object variable compilation with IrValue::Object (line 3029-3038)
- `src/grue_compiler/codegen_instructions.rs` - Runtime object number resolution (line 308-320)
- `src/grue_compiler/codegen.rs` - Made object_numbers public, added pattern matches (lines 214, 2837-2847, 3298-3302)

**Architecture Improvement**: Object references now use deferred name resolution instead of compile-time numbers, ensuring correct runtime object mapping for ALL object variable references throughout the compiler.

### **String-to-Dictionary Function Parameter Passing Bug** ✅ **COMPLETED** (November 6, 2025)

**ISSUE**: `handle_go("up")` function call received corrupted dictionary address, causing incorrect parameter values
- **Problem**: Function received `-16641` (`0xBEFF`) instead of expected dictionary address `2750` (`0x0abe`)
- **Root Cause**: Off-by-one error in UnresolvedReference address calculation for function call operands

**SOLUTION**: **Fixed address calculation using `layout.operand_location`** from `emit_instruction_typed`

**CRITICAL DISCOVERY**: The original parameter passing system was **architecturally designed to reject string literals**:
```rust
if self.ir_id_to_string.contains_key(&ir_id) {
    return Err(CompilerError::CodeGenError("Cannot use string literal as operand"));
}
```

Our implementation bypassed this safety mechanism to enable string-to-dictionary conversion for direction arguments.

**THE BUG**: Manual address calculation **before** instruction emission vs precise locations **after**:
```rust
// BUGGY (Manual calculation):
let operand_location = self.code_address + 1 + operands.len() * 2;
operands.push(Operand::LargeConstant(placeholder_word()));
// Create UnresolvedReference at calculated location (WRONG!)

// FIXED (Layout-based):
operands.push(Operand::LargeConstant(placeholder_word()));
let layout = self.emit_instruction_typed(...);
if let Some(base_operand_loc) = layout.operand_location {
    let string_operand_location = base_operand_loc + (operand_index * 2);
    // Create UnresolvedReference at precise location (CORRECT!)
}
```

**THE FIX**: **Two-phase approach**
1. **Collect string info** during operand building
2. **Process using layout** after `emit_instruction_typed` provides exact addresses

**IMPLEMENTATION DETAILS**:
- **Direction detection**: `["up", "down", "north", "south", "east", "west", ...]`
- **Dictionary conversion**: Direction strings → dictionary addresses (not packed addresses)
- **Display strings**: Non-direction strings → packed addresses (existing behavior)
- **Layout precision**: Use `layout.operand_location + (index * 2)` for exact addressing

**EVIDENCE OF SUCCESS**:
```bash
# BEFORE (Broken):
handle_go called with: -16641
You can't go that way.

# AFTER (Fixed):
handle_go called with: 2750
You are about 10 feet above the ground nestled among some large branches.
```

**TECHNICAL VALIDATION**:
- ✅ Dictionary resolution: `"up"` → position 134 → address `0x0abe` (2750)
- ✅ Address calculation: `base=0x0443 + 1*2 = 0x0445`
- ✅ Parameter passing: Function receives correct `2750`
- ✅ Game functionality: `climb tree` now works correctly

**ARCHITECTURE PRESERVED**: Separation between dictionary addresses (comparison) and packed addresses (display)

**FILES MODIFIED**:
- `src/grue_compiler/codegen_instructions.rs` lines 3301-3425 - **Fixed string parameter handling**
- Layout-based address calculation replaces manual calculation
- Two-phase approach: collect info, then process with precise locations

**LESSON LEARNED**: When working with Z-Machine instruction layout, always use the **precise locations returned by `emit_instruction_typed`** rather than manual calculation. The `InstructionLayout` struct exists specifically to provide these exact addresses.

### **Score System Binary Arithmetic Bug** 🔥 **CURRENT WORK** (November 4, 2025)

**ISSUE**: Arithmetic binary operations on score fail to produce correct results
- **Problem**: `player.score = player.score + 10` doesn't work correctly
- **Evidence**: `gain` command shows `Gained 10 points! Score: 0 -> 0` instead of `0 -> 10`
- **Root Cause**: Binary operations (`+`, `-`) not evaluating correctly in expressions
- **Status**: **INVESTIGATING** - Compiler issue with arithmetic operation IR generation

**IMPLEMENTATION STATUS**:
- ✅ **Core Score Architecture**: Working perfectly (Global G17, status line integration)
- ✅ **Score Reading**: `player.score` works correctly
- ✅ **Score Assignment**: `player.score = 100` works correctly
- ❌ **Score Arithmetic**: `player.score + value` not working
- ❌ **Score Builtin Functions**: `add_score()`, `subtract_score()` crash with opcode errors

**INVESTIGATION PLAN**:
1. 🎯 **Current**: Debug arithmetic binary operation IR generation
2. **Next**: Fix opcode generation for builtin functions
3. **Final**: Verify complete score functionality

### **Score Builtin Functions Opcode Bug** ⏳ **NEXT UP** (November 4, 2025)

**ISSUE**: `add_score()` and `subtract_score()` builtin functions cause runtime crashes
- **Problem**: `add_score(10)` generates invalid Z-Machine bytecode
- **Evidence**: `Error: Failed to decode instruction at 00b7a: Invalid Long form opcode 0x15`
- **Root Cause**: Builtin functions generating invalid opcodes
- **Status**: **PENDING** - Runtime opcode generation issue

**FILES TO INVESTIGATE**:
- `src/grue_compiler/ir.rs:4409-4493` - Builtin function implementations
- `src/grue_compiler/codegen_builtins.rs` - Z-Machine code generation
- Score builtin function call mechanism

**SCORE SYSTEM VALIDATION** ✅ **COMPLETED**:
- ✅ **5 Unit Tests**: All passing - IR generation correctly uses Global G17
- ✅ **4 Gameplay Tests**: All passing - Core functionality works
- ✅ **200 System Tests**: All passing - No regressions introduced

### **StringAddress Type System Enhancement** 🔄 **IN DEVELOPMENT** (November 7, 2025)

**OBJECTIVE**: Replace manual `print_message` function with automatic string address detection in `println()`
- **Goal**: Enable `println(exit.message)` to automatically detect string addresses and use correct Z-Machine opcodes
- **Current**: Must use `print_message(exit.message)` because `println` treats all Int as numbers, not packed string addresses
- **Target**: Type-aware `println` that automatically selects `print_paddr` vs `print_num` based on value type

**IMPLEMENTATION STATUS**: **Phase 4 INCOMPLETE** - Foundation ready, function call type propagation missing
- ✅ **Phase 1**: StringAddress type system foundation (Type enum, parser, semantic analysis)
- ✅ **Phase 2**: IR value representation with IrValue::StringAddress variant
- ✅ **Phase 3**: Codegen support for StringAddress operand conversion
- ❌ **Phase 4**: Function call return type propagation - **CRITICAL MISSING COMPONENT**

**ROOT CAUSE IDENTIFIED**: Type information lost during builtin function calls
- `exit.message` property access correctly has Type::StringAddress
- But `exit_get_message` builtin function returns Type::Int, not StringAddress
- So `println(exit.message)` receives Int and displays "1860" instead of message text
- **Solution Needed**: Builtin functions must declare StringAddress return types

**IMPLEMENTATION BRANCH**: `feature/string-address-type`
- **Rollback Safety**: All changes isolated on feature branch, can merge when complete
- **Testing Strategy**: Verify no regressions after each phase completion

**TECHNICAL ARCHITECTURE**:
- **StringAddress Type**: Distinguished from regular Int for packed string addresses
- **Type Compatibility**: StringAddress allowed in String contexts for seamless usage
- **Automatic Detection**: `println()` selects appropriate Z-Machine opcode based on argument type

**DETAILED PLAN**: See `docs/STRING_ADDRESS_TYPE_PLAN.md` for complete 7-phase implementation plan and current status

**NEXT STEPS**:
1. **Add builtin return type declarations** - Declare that `exit_get_message`, etc. return StringAddress
2. **Implement IR type tracking** - Propagate function call result types through expression evaluation
3. **Update println codegen** - Use type information to select print_paddr vs print_num
4. **Verify automatic detection** - Test that `println(exit.message)` works without manual print_message calls

### **Score Display Corruption Bug** ✅ **FIXED** (November 2, 2025)

**ISSUE RESOLVED**: Score command now correctly displays actual score value
- **Root Cause**: `to_string()` builtin was placeholder returning literal `"[NUM]"`, not actual integer conversion
- **Solution**: Implemented `print_num()` builtin using Z-Machine `print_num` opcode (VAR:230/6)
- **Result**: Score functionality working correctly, displays "Your score is 0" instead of memory corruption

**IMPLEMENTATION COMPLETED - Option B: print_num() Builtin**:

**✅ Phase 1: Create print_num() Builtin Function**
- ✅ Added `print_num` to builtin function registry in semantic analysis
- ✅ Implemented `print_num` function generation in `src/grue_compiler/codegen.rs`
- ✅ Used Z-Machine `print_num` opcode (VAR:230/6) to directly print signed 16-bit integers
- ✅ Function signature: `print_num(value: int) -> void`
- ✅ Added builtin function dispatch logic using standard `call_builtin_function` mechanism

**✅ Phase 2: Update Score Function**
- ✅ Modified `handle_score()` in `examples/mini_zork.grue`:
   ```grue
   fn handle_score(){
       print("Your score is ");
       print_num(player.score);
   }
   ```

**✅ Phase 3: Validation**
- ✅ Compiled and tested score command functionality - working perfectly
- ✅ Verified no memory corruption or garbage characters
- ✅ Confirmed actual score value (0) is displayed correctly
- ✅ Tested with multiple commands - all functionality working

**Files Modified**:
- `src/grue_compiler/semantic.rs` - Registered `print_num` builtin
- `src/grue_compiler/ir.rs` - Added `print_num` to builtin function detection
- `src/grue_compiler/codegen.rs` - Implemented function generation and dispatch
- `examples/mini_zork.grue` - Updated `handle_score()` function

**Technical Implementation**:
- Uses Z-Machine `print_num` opcode (VAR:230/6) for direct integer printing
- Implemented as real Z-Machine function (not inline) following architectural requirements
- Follows standard builtin function call mechanism using `call_vs` instructions

### **Print Newline Architecture Issue** ✅ **FIXED** (November 2, 2025)

**ISSUE RESOLVED**: Z-Machine specification violation in print functions resolved with dual-function approach
- **Root Cause**: All print functions automatically added newlines, violating Z-Machine spec
- **Z-Machine Spec**: Print opcodes should NOT add automatic newlines - only explicit `new_line` should
- **Solution**: Implemented dual print/println architecture for precise control

**IMPLEMENTATION**:
✅ **`print()` Function**: Z-Machine spec compliant - no automatic newlines
- Outputs text exactly as specified, allowing concatenation
- Enables constructs like: `print("Your score is "); print_num(score); new_line();`

✅ **`println()` Function**: Convenience function - automatic newlines
- Renamed existing `print()` implementation to maintain backward compatibility
- All game content converted to use `println()` except score function

✅ **Score Display Fixed**: Now correctly displays "Your score is 0" on single line
- Uses: `print("Your score is "); print_num(player.score); new_line();`
- Result: Proper inline formatting with explicit line control

**FILES MODIFIED**:
- `src/grue_compiler/semantic.rs` - Registered both `print` and `println` builtins
- `src/grue_compiler/ir.rs` - Added `println` to builtin function detection
- `src/grue_compiler/codegen.rs` - Updated dispatch to handle both functions
- `src/grue_compiler/codegen_builtins.rs` - Created separate implementations:
  - `generate_print_builtin()` - No newlines (Z-Machine spec compliant)
  - `generate_println_builtin()` - With newlines (backward compatible)
- `examples/mini_zork.grue` - Updated all `print()` calls to `println()` except score function

**ARCHITECTURE BENEFITS**:
- ✅ **Z-Machine Specification Compliance**: `print()` follows spec exactly
- ✅ **Backward Compatibility**: All existing content works with `println()`
- ✅ **Precise Control**: Developers can choose exact formatting behavior
- ✅ **Explicit Newlines**: `new_line()` provides clear line break control

### **Verb Dispatch Infinite Loop** ✅ **FIXED**
- **Issue**: "open mailbox" caused infinite loop due to incorrect increment instruction compilation
- **Root Cause**: Increment instruction used deprecated `emit_instruction()` generating malformed Z-Machine bytecode
- **Fix**: Changed to `emit_instruction_typed(Opcode::Op1(Op1::Inc), ...)` for correct bytecode generation
- **Status**: RESOLVED - Object lookup loop now works correctly

### **Mailbox Default State** ✅ **FIXED** (November 1, 2025)
- **Issue**: Mailbox defaulted to open state, responded "It's already open" even when closed
- **Root Cause**: IR generation placed then_label before else_label after TestAttributeBranch instruction
- **Problem**: Z-Machine test_attr falls through when attribute is CLEAR, but was falling through to then_label content instead of else_label content
- **Fix**: Modified IR generation to place else_label content immediately after TestAttributeBranch, with then_label content after jump
- **Result**: Mailbox now correctly starts closed and responds "Opened." when first opened, "It's already open." when already open

### **Object Resolution System** ✅ **WORKING**
- **Status**: Object lookup now functions correctly after increment instruction fix
- **Verification**: "open mailbox" correctly resolves mailbox object and executes verb command
- **Architecture**: Uses Property 18 dictionary address comparison for proper object matching

### **Dynamic Descriptions** ✅ **USING CONDITIONAL PRINT APPROACH**
- **Issue**: "examine mailbox" needs to show dynamic state (open/closed)
- **Solution**: Traditional conditional print statements in examine handler instead of computed properties
- **Status**: Simple, proven approach - mailbox examine functionality working correctly

### **Stack Discipline Architecture** ✅ **MAJOR IMPROVEMENTS**
- **Achievement**: Reduced stack imbalance from 25 to 12 unpulled IR IDs (52% improvement)
- **Fix**: Function calls now use proper Z-Machine Variable(0) discipline
- **Status**: Core architecture violations resolved, remaining optimization in progress

### **Z-Machine Boolean Expressions** ✅ **COMPLETE**
- **Phase 3**: Context-aware conditional attribute access working
- **Optimization**: ~50% reduction in IR instructions for conditional attributes
- **Status**: Production ready with comprehensive runtime verification

---

## 🔧 **TECHNICAL DEBT**

### **Minor Optimizations**
- Context-aware expression generation migration (10 remaining legacy calls)
- Test coverage expansion for new attribute systems
- Code comment documentation for recent fixes

### **Type System Improvements**
- **Implement Proper to_string() Builtin Function**: Currently `to_string()` is a placeholder that returns literal `"[NUM]"` instead of converting integers to strings. Need to implement actual integer-to-string conversion using Z-Machine `print_num` opcode or similar mechanism.
- **Fix print() Builtin for Non-String Values**: `print(player.score)` causes memory corruption when trying to print integer values directly. Both string concatenation (`"text" + integer`) and direct integer printing need proper type handling.
- **Automatic Type Conversion in String Concatenation**: After implementing proper `to_string()`, compiler should automatically insert type conversion for common cases like string + integer concatenation to improve developer experience and prevent memory corruption bugs.

### **Infrastructure**
- All major bugs resolved
- Core systems functional and stable
- Commercial game compatibility maintained

---

## 📋 **MAINTENANCE NOTES**

**Recent Critical Findings (November 1, 2025)**:
- **Verb Dispatch Infinite Loop**: "open mailbox" causes infinite loop on both systematic-rebuild and computed-property branches
- **Object Resolution Failure**: Grammar system incorrectly accesses object 1 (player) instead of object 3 (mailbox)
- **Cross-Branch Consistency**: Identical infinite loop pattern confirms issue is in core grammar system, not computed property implementation
- **Debug Evidence**: `get_prop_addr(obj=1, prop=18)` returns 0x0000, causing branch-jump-loop cycle

**Previous Fixes (October 31, 2025)**:
- TestAttributeBranch IR instruction implementation complete
- Store instruction gap filled in interpreter
- Stack discipline architecture violations resolved

**Documentation Location**:
- Historical analysis: `docs/` directory
- Active development: Root directory files
- Current investigation: Object resolution in verb dispatch pipeline

**Status**: **CONTAINER SYSTEM FULLY FUNCTIONAL** - All major object containment and visibility bugs resolved

---

## ✅ **SYSTEM STATUS - ALL MAJOR BUGS RESOLVED** (November 2, 2025)

### **Container System Architecture** ✅ **FULLY FUNCTIONAL**

**ACHIEVEMENT**: Complete object containment, visibility, and state management system working perfectly

**✅ ALL MAJOR COMPONENTS RESOLVED**:
- **Object Containment**: Fixed dual insertion parent pointer bug (vm.rs:1234) ✅
- **Visibility Logic**: Fixed `player_can_see()` conditional syntax (nested if statements) ✅
- **Container State Management**: Fixed SetAttribute boolean assignment bug (ir.rs:2518-2533) ✅
- **Container Interactions**: Objects in open containers correctly visible and accessible ✅
- **State Persistence**: Open/close cycles working with perfect state management ✅

**TECHNICAL EVIDENCE**:
- **Assignment Code**: `obj.open = false` executes and actually changes state ✅
- **State Management**: Complete open/close/reopen cycles working perfectly ✅
- **IR Generation**: SetAttribute instruction with correct boolean values ✅
- **Z-Machine Output**: Attribute opcodes generated with accurate value parameters ✅

**✅ IMPLEMENTATION DETAILS**:
1. ✅ **Fixed TODO in ir.rs:2518**: Extract actual boolean from AST `Boolean(value)` instead of hardcoding `true`
2. ✅ **Tested attribute assignments**: Verified `obj.open = false` correctly sets attribute to false
3. ✅ **Regression tested**: Confirmed `obj.open = true` still works perfectly
4. ✅ **Comprehensive validation**: Complete open/close/reopen cycle working with state persistence

**✅ VALIDATION EVIDENCE**:
- Initial: "examine leaflet" → "You can't see any such thing" (mailbox closed)
- Open: "open mailbox" → shows leaflet contents (obj.open = true working)
- Access: "examine leaflet" → "leaflet" (visible in open container)
- Close: "close mailbox" → "Closed." (obj.open = false working)
- Hidden: "examine leaflet" → "You can't see any such thing" (hidden in closed container)
- Reopen: Full cycle repeatable with perfect state management

**✅ DUAL SYSTEM ARCHITECTURE BUG - PARTIALLY RESOLVED**:
- **Compile-time**: Object placement metadata correctly generated ✅
- **Runtime**: InsertObj instruction executed, containment now persisting ✅ **FIXED**
- **Root Cause**: Double insertion prevention skipped `set_parent()` call ✅ **IDENTIFIED & FIXED**
- **Architectural Issue**: Dual insertion system needs comprehensive redesign (see line 190-233)

**INVESTIGATION METHODOLOGY**:
1. ✅ **Traced object lookup function**: Found working correctly (5919-6333 in codegen.rs)
2. ✅ **Verified dictionary resolution**: Parser correctly finds "leaflet" at 0x0800
3. ✅ **Confirmed property 18 setup**: Dictionary addresses properly stored and loaded
4. ✅ **Identified containment failure**: Object parent not correctly set at runtime
5. ✅ **Ruled out grammar bugs**: Object resolution works, visibility logic fails

**NEXT STEPS**:
1. ✅ **InsertObj instruction execution**: Fixed - `set_parent()` now called in dual insertion prevention
2. ✅ **Dual insertion conflict**: Resolved - parent relationships now correctly established
3. ✅ **Object tree integrity**: Verified - no overwrites, parent field persists correctly
4. ✅ **Container visibility logic**: Fixed - `player_can_see()` now correctly checks objects in open containers
5. **CRITICAL NEW BUG**: Fix SetAttribute compiler bug - all boolean assignments hardcoded to `true` (ir.rs:2524)

**FILES TO INVESTIGATE** (Next Phase):
- Compiler-generated `player_can_see()` function - visibility logic for open containers
- Game logic functions that check object accessibility
- Container visibility rules implementation

**FILES ALREADY FIXED**:
- ✅ `src/vm.rs:1216-1235` - InsertObj instruction implementation (parent pointer fix)
- ✅ `src/grue_compiler/codegen.rs:4049-4082` - Compile-time object placement (working correctly)
- ✅ `src/grue_compiler/ir.rs:1410-1411` - Runtime object placement generation (working correctly)

---

## ✅ **RECENTLY FIXED BUGS** (November 2, 2025)

### **SetAttribute Boolean Assignment Bug** ✅ **FIXED** (November 2, 2025)
- **Issue**: All boolean attribute assignments hardcoded to `true`, breaking `obj.open = false` and container state management
- **Root Cause**: TODO comment in ir.rs:2524 with hardcoded `value: true` instead of extracting actual boolean value
- **Fix**: Extract boolean value directly from AST `Boolean(value)` expressions before generating SetAttribute instruction
- **Files**: `src/grue_compiler/ir.rs:2518-2533`
- **Result**: Complete container open/close state management working perfectly
- **Impact**: Fixes ALL boolean attribute assignments (`open`, `locked`, `container`, etc.) throughout Grue language

### **Container Visibility Logic Bug** ✅ **FIXED** (November 2, 2025)
- **Issue**: Objects in open containers not visible to player, breaking "examine leaflet" commands
- **Root Cause**: Multi-line `&&` conditional syntax not working in Grue language
- **Fix**: Changed from `if obj.location.container && obj.location.open && obj.location.location == player.location` to nested if statements
- **Files**: `examples/mini_zork.grue:455-461`
- **Result**: Objects in open containers now correctly visible and examinable
- **Discovery**: Grue language requires nested `if` statements instead of multi-line `&&` conditionals

### **Object Containment Parent Pointer Bug** ✅ **FIXED** (November 2, 2025)
- **Issue**: Objects correctly positioned in containers but parent pointers not set, causing visibility failures
- **Root Cause**: Dual insertion prevention logic skipped `set_parent()` call when objects already correctly positioned
- **Fix**: Added `self.set_parent(obj_num, dest_num)?;` in vm.rs:1234 before early return in double insertion prevention
- **Files**: `src/vm.rs:1230-1235`
- **Result**: Object containment now working correctly, leaflet properly contained in mailbox
- **Evidence**: Runtime trace shows `get_parent: obj_num=11 -> parent=10` instead of parent=0

### **Branch Resolution Location Bug** ✅ **FIXED**
- **Issue**: VAR:0x1f crash and infinite loops in "examine leaflet" due to wrong branch/jump placeholder resolution
- **Root Cause**: `generate_comparison_with_result` used deprecated `add_unresolved_reference()` instead of `add_unresolved_reference_at_location()`
- **Fix**: Updated all 8 instances (6 branches + 2 jumps) to use correct placeholder locations from `emit_instruction_typed` layouts
- **Files**: `src/grue_compiler/codegen.rs:8512-8723`
- **Result**: No more crashes or infinite loops during object processing

---

## 🏗️ **ARCHITECTURAL DEBT**

### **Object Containment Dual Insertion Architecture** ⚠️ **CRITICAL ARCHITECTURAL FIX NEEDED** (November 2, 2025)

**Issue**: The compiler implements object containment through **two parallel systems** that both place objects into containers:
1. **Compile-time object placement** (codegen.rs:4049-4082) - Direct object table manipulation
2. **Runtime object placement** (ir.rs:1410-1411) - InsertObj instructions in init function

**Root Cause**: InsertObj instructions serve dual conflicting purposes:
- **Information Source**: Compiler needs relationship data for object table generation
- **Executable Instructions**: Runtime needs placement commands for dynamic movement

**Current Status**:
- ✅ **Immediate Fix Applied**: Double insertion prevention in vm.rs:1216-1235
- ✅ **Critical Bug Fixed**: Parent pointer issue resolved (November 2, 2025)
- ✅ **Containment Working**: Objects now correctly placed and maintain parent relationships
- ⚠️ **Architectural Inconsistency**: Dual system remains, requires comprehensive redesign

**Risk Level**: **MEDIUM**
- Low immediate risk: workaround prevents user-visible bugs
- Medium long-term risk: complexity may cause future bugs
- High maintenance burden: developers must understand dual system

**Resolution Strategy**:

**Phase 1** 🎯 **NEXT PRIORITIES** (Choose One):

**Option A: Instruction State Tracking** (Recommended)
- Add `InstructionState` enum to IrInstruction (Pending/ProcessedCompile/ProcessedRuntime)
- Mark InsertObj as processed during preprocessing phase
- Skip runtime execution of already-processed instructions
- **Effort**: Medium, maintains existing architecture
- **Benefit**: Clean separation without breaking changes

**Option B: Declaration vs Execution Separation** (Future)
- Create `DeclareContainment` vs `MoveObject` instruction types
- Introduce language syntax distinction (`initially_contains {}` vs `move()`)
- **Effort**: High, requires language design changes
- **Benefit**: Fundamental architecture cleanup

**Option C: Enhanced Runtime Detection** (Band-aid)
- Extend current fix to check entire sibling chain
- Maintain dual architecture with better safeguards
- **Effort**: Low, quick implementation
- **Benefit**: Robust workaround, doesn't address root cause

**Critical Questions for Decision**:

1. **Instruction Processing Strategy**: Should InsertObj instructions be marked as "processed" during preprocessing to prevent runtime re-execution?

2. **Scope of Change**: Are we willing to modify IR instruction semantics (Option A) or should we preserve current structure (Option C)?

3. **Language Evolution**: Should this architectural improvement drive language syntax changes (Option B) or remain internal (Option A)?

4. **Migration Path**: How do we handle existing code if we change instruction semantics or language syntax?

**Files Requiring Changes** (Option A):
- `src/grue_compiler/ir.rs` - Add InstructionState enum and tracking
- `src/grue_compiler/codegen.rs` - State-aware instruction processing
- `src/grue_compiler/codegen_instructions.rs` - Skip processed instructions

**Testing Strategy**:
- Verify compile-time object placement still works
- Verify runtime object movement still works
- Test mixed scenarios (some compile-time, some runtime)
- Ensure no double insertion in any configuration

**Documentation**: Complete architectural analysis in `docs/ARCHITECTURE.md` (Object Containment Dual Insertion Architecture Problem)

---

## 📦 **ARCHIVED FEATURES**

### **Computed Property System** 📁 **ARCHIVED (November 1, 2025)**
- **Concept**: Dynamic object property expressions evaluated at runtime (e.g., `desc: "The mailbox is " + (mailbox.open ? "open" : "closed") + "."`)
- **Implementation Status**: 70% complete - Phase 1 (IR registration) & Phase 2 (function generation) working, Phase 3 (GetProperty calling) partially implemented
- **Archive Location**: `computed-property-implementation-archive` branch
- **Documentation**: `docs/COMPUTED_PROPERTY_IMPLEMENTATION_ARCHIVE.md`
- **Revert Reason**: Complexity vs. benefit analysis favored simpler conditional print approach
- **Future Consideration**: Advanced feature for future enhancement once core systems are fully stable