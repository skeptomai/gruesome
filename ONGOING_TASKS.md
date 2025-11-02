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

**Status**: **CONTAINMENT BUG ISOLATED** - Object lookup working correctly, containment system failing

---

## 🐛 **ACTIVE BUG INVESTIGATION**

### **Critical SetAttribute Compiler Bug** 🔍 **CRITICAL** (November 2, 2025)

**ISSUE**: All boolean attribute assignments (`obj.property = false`) are hardcoded to set `true`, breaking state changes

**ROOT CAUSE IDENTIFIED** (November 2, 2025):

**❌ SETATTRIBUTE HARDCODED VALUE BUG**:
- **Location**: `src/grue_compiler/ir.rs:2524`
- **Problem**: `value: true, // TODO: Extract actual boolean from value_temp`
- **Impact**: ALL attribute assignments (`obj.open = false`) ignore the assigned value and set `true`
- **Evidence**: `obj.open = false` debug shows "Before: 1, After: 1" - no change
- **Scope**: Affects ALL boolean attribute assignments in Grue language

**✅ PREVIOUS FIXES COMPLETED**:
- **Object Containment**: Fixed dual insertion parent pointer bug (vm.rs:1234) ✅
- **Visibility Logic**: Fixed `player_can_see()` conditional syntax (nested if statements) ✅
- **Container Logic**: Objects in open containers now correctly visible ✅

**TECHNICAL EVIDENCE**:
- **Assignment Code**: `obj.open = false` executes but prints "Closed." (success message) ✅
- **Debug Output**: "Before: obj.open = 1, After: obj.open = 1" (value unchanged) ❌
- **IR Generation**: SetAttribute instruction created but hardcoded to `value: true` ❌
- **Z-Machine Output**: Set attribute opcodes generated with wrong value parameter ❌

**NEXT STEPS**:
1. **Fix TODO in ir.rs:2524**: Extract actual boolean from `value_temp` instead of hardcoding `true`
2. **Test attribute assignments**: Verify `obj.open = false` actually sets attribute to false
3. **Regression test**: Ensure `obj.open = true` still works after fix
4. **Comprehensive test**: Test all boolean attribute types (open, locked, etc.)

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