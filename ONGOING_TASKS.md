# Ongoing Tasks

## CURRENT STATUS (Oct 21, 2025): ✅ ALL MAJOR TASKS COMPLETED

**STACK DISCIPLINE**: ✅ **100% COMPLETE** - All Variable(0) operations converted to proper push/pull semantics

**PROPERTY 28 CRASH**: ✅ **100% FIXED** - Root cause identified and resolved with version-aware property number allocation

**CURRENT STATE**: All critical bugs resolved, compiler in production-ready state for Z-Machine V3

## Recent Accomplishments

### ✅ Property 28 Crash Resolution (Oct 21, 2025)
**ROOT CAUSE**: Property number collision due to Z-Machine 5-bit encoding limits
- Property 38 encoded as Property 6 (38 & 0x1F = 6)
- Property 37 encoded as Property 5 (37 & 0x1F = 5)
- Property 36 encoded as Property 4 (36 & 0x1F = 4)
- Property 34 encoded as Property 2 (34 & 0x1F = 2)

**SOLUTION**: Version-aware PropertyManager with intelligent allocation
- V3: Properties 1-31 (5-bit encoding)
- V4/V5: Properties 1-63 (6-bit encoding)
- Comprehensive validation with panic handling for out-of-bounds assignments

**VERIFICATION**: All 178 tests pass, game runs with proper room descriptions

### ✅ Complete Z-Machine Stack Discipline Implementation (Oct 20, 2025)
**ACHIEVEMENT**: Replaced ALL 25+ `use_stack_for_result` calls with proper push/pull semantics
- Added `use_push_pull_for_result()` function that emits VAR:232 push instructions
- Modified `resolve_ir_id_to_operand()` to emit VAR:233 pull instructions to temporary globals (200+)
- Converted all operations to use proper LIFO stack semantics

**IMPACT**: Eliminates ALL Variable(0) collision scenarios across entire codebase

## Current Project Status

**Z-Machine Interpreter**: Production Ready ✅
- v3 Games: Fully playable (Zork I, Seastalker, The Lurking Horror)
- v4+ Games: Fully playable (AMFV, Bureaucracy, Border Zone)

**Grue Z-Machine Compiler**: V3 Production Ready ✅
- Full Pipeline: Lexer → Parser → Semantic → IR → CodeGen
- V3 Support: Production ready with comprehensive test coverage
- V4/V5 Support: Experimental (debug builds only)

**Test Coverage**: 178 tests passing, zero critical bugs

## Historical Investigation Archive

The extensive Property 28 investigation history (2000+ lines) has been archived to:
- `ONGOING_TASKS_HISTORICAL_20251021.md`

This archive contains the complete debugging journey from initial crash discovery through
root cause identification, including all instrumentation, analysis, and investigation
steps that led to the final solution.

## ✅ OBJECT NUMBERING SYSTEM CONFLICT RESOLVED (Oct 21, 2025)

**SYMPTOM**: Navigation crash "Property 26 not found for object 3" when moving north from west_of_house

**ROOT CAUSE**: Object generation used sequential numbering instead of semantic numbering

### **The Conflict**

**System 1: Semantic Phase (`ir.object_numbers`)**
```
'mailbox' → Object #3
'north_of_house' → Object #5
```

**System 2: Object Generation Phase (`ir_id_to_object_number`)**
```
IR ID 33 (mailbox) → Object #10
IR ID 22 (north_of_house) → Object #3
```

**Runtime Reality** (actual object tree):
```
Object #3 = mailbox (from semantic numbering)
```

### **The Bug Sequence**

1. **Object Generation Phase** creates objects in order:
   - north_of_house (IR ID 22) assigned to object #3
   - Logs: `Object #3: ROOM 'north_of_house' (ID: 22, short: 'North of House')`

2. **Exit System** uses object generation mappings:
   - west_of_house north exit → IR ID 22 → object #3
   - Logs: `Exit system: Room 'west_of_house' exit 0 direction 'north' -> room IR ID 22 = object 3`

3. **Symbol Resolution Phase** tries to apply semantic numbering:
   - Wants mailbox (IR ID 33) → object #3
   - But preserves existing mapping (doesn't overwrite)
   - Logs: `OBJECT_MAPPING: 'mailbox': IR ID 33 → Object #3 (existing: 10, not overwritten)`

4. **Final Object Tree** somehow contains mailbox at object #3:
   - Runtime debug: `Object #3: "small mailbox"`

5. **Navigation Failure**:
   - Player moves north → move(player, object #3)
   - object #3 = mailbox (not north_of_house)
   - Code tries to set `player.location.visited = true`
   - `player.location` = mailbox (object #3)
   - Mailbox doesn't have Property 26 (visited) → CRASH

### **Technical Details**

**Files Involved**:
- `src/grue_compiler/codegen.rs:8492-8519` (symbol resolution phase)
- `src/grue_compiler/codegen_objects.rs:772-805` (exit system)
- Object generation vs semantic numbering conflict

**Key Evidence**:
- Exit system correctly maps IR ID 22 → object #3
- Object generation logs north_of_house as object #3
- Runtime shows object #3 = mailbox
- Two phases have incompatible object numbering schemes

### **Impact**

- Navigation system completely broken
- Any room transition that changes object numbering will fail
- Property access on wrong objects causes crashes
- Game unplayable beyond first room

## 🔧 SOLUTION STRATEGY: Object Numbering System Unification

### **Root Cause**
Two independent object numbering systems creating conflicting assignments:
1. **Semantic Phase** (`ir.object_numbers`): mailbox #3, north_of_house #5
2. **Object Generation Phase** (`ir_id_to_object_number`): north_of_house #3, mailbox #10

### **Solution: Make Object Generation Phase Authoritative**

**Rationale**: Object generation creates the actual object tree, so all systems should use its numbering.

**Implementation Plan**:

#### **Phase 1: Fix Compilation Phase Ordering** 🔄
Current (BROKEN):
```
1. setup_room_to_object_mapping()     // Uses partial/semantic data
2. generate_exit_properties()         // Uses partial mapping
3. generate_objects()                 // Creates final mapping
4. populate_ir_id_mappings()          // Too late!
```

Target (FIXED):
```
1. generate_objects()                 // Creates authoritative mapping
2. setup_room_to_object_mapping()     // Uses authoritative mapping
3. generate_exit_properties()         // Uses authoritative mapping
4. populate_ir_id_mappings()          // Validation only
```

#### **Phase 2: Update Exit System Data Source** 🔄
Change exit system from:
```rust
self.room_to_object_id.get(room_ir_id)    // OLD: partial mapping
```
To:
```rust
self.ir_id_to_object_number.get(room_ir_id)  // NEW: final mapping
```

#### **Phase 3: Remove Semantic Object Numbering** 🔄
Deprecate `ir.object_numbers` as authoritative source, use only for validation.

#### **Phase 4: Update Room-to-Object Mapping** 🔄
Ensure `room_to_object_id` uses same final mappings as `ir_id_to_object_number`.

**Files to Modify**:
- `src/grue_compiler/codegen_objects.rs:780-792` (exit system data source)
- `src/grue_compiler/codegen.rs` (phase ordering)

## ✅ RESOLUTION (Oct 21, 2025)

**SOLUTION**: Modified `object_id_to_number` mapping in `codegen_objects.rs` to use semantic numbering from `ir.object_numbers` instead of sequential numbering `(index + 1)`.

**FILES MODIFIED**:
- `src/grue_compiler/codegen.rs:749-793`: Fixed compilation phase ordering
- `src/grue_compiler/codegen_objects.rs:682-698, 987-1003`: Fixed object ID mappings to use semantic numbering

**VERIFICATION**: Navigation works correctly, no Property 26 crash, semantic object numbering maintained throughout compilation.

## Outstanding Work

Additional compiler improvements and bug fixes as discovered during testing.

Future enhancements could include:
- Additional Z-Machine instruction support for V4/V5 completeness
- Performance optimizations
- Additional test coverage for edge cases
- Enhanced debugging tools and instrumentation