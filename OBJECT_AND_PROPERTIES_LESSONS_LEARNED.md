# Object and Properties System: Lessons Learned from Failed Integration

## Investigation Summary (October 25, 2025)

This document captures critical lessons learned during the Variable Source Tracking integration that revealed fundamental object system failures.

## What We Discovered

### 1. Object Table Generation is Completely Broken

**Evidence from Object Dump:**
```
Object #1: ''          <- Should be room/object name
Object #2: ''          <- All objects have empty names
Object #3: ''
...
Object #7: ''          <- Should be "Up a Tree" (tree room)
...
Object #14: ''
Object #15: 'cjxkkckcuc'  <- Garbage data
Object #16: 'cjxkkckcuc'  <- Memory corruption
BOUNDS ERROR: prop_table_addr 0x2355 >= file size 6826
```

### 2. Root Cause Analysis

**Primary Issue: Object Name Generation Pipeline Broken**
- Objects 1-14 have completely empty names (should contain short descriptions)
- Object #7 should be "Up a Tree" (the tree room) but is empty
- No object has ANY name, making dictionary lookup impossible

**Secondary Issues:**
- Property table addressing exceeds file bounds (memory corruption)
- Objects 15+ contain garbage data suggesting memory layout corruption
- Property 28 collision was a red herring - objects don't exist to have properties

### 3. Why "examine tree" Crashes

**Sequence of Failure:**
1. User types "examine tree"
2. Object lookup searches dictionary for "tree"
3. No object has "tree" in names (because all objects have empty names)
4. Object lookup returns 0 (not found)
5. examine(0) crashes when accessing obj.desc on non-existent object

**Object 0 validation fix was irrelevant** - the crash happens during object lookup, not in examine().

### 4. Integration Failure Points

**What the Variable Source Tracking Integration Broke:**
- Object name assignment during compilation
- Property table memory layout
- Object creation pipeline from IR to Z-Machine format
- Dictionary generation (objects have no names to add)

**PropertyManager version-aware fixes were misapplied** - need functional objects first before worrying about property number allocation.

## Critical Debugging Infrastructure Added

### Object Table Dumping
Added `--debug-objects` flag to gruesome interpreter:
```bash
RUST_LOG=info ./target/debug/gruesome game.z3 --debug-objects
```
**Location:** `src/main.rs` - object table dump at startup

### Compilation Object Analysis
```bash
RUST_LOG=debug ./target/debug/grue-compiler examples/mini_zork.grue -o test.z3 2>&1 | grep -i tree
```
Shows object creation during compilation.

## Failed Integration Artifacts

### Property 28 PropertyManager Enhancement
**File:** `src/grue_compiler/ir.rs` lines 318-514
**Status:** Applied but irrelevant - objects don't exist to have property collisions
**Issue:** Version-aware property allocation assumes functional object creation

### Object 0 Validation Fix
**File:** `examples/mini_zork.grue` examine() function
**Status:** Applied but irrelevant - crash happens before examine() is called
**Issue:** Fixed symptom, not root cause

## Key Architecture Questions to Answer Before Re-Integration

### 1. Object Creation Pipeline
- Where do object names get assigned during compilation?
- How does IR object data become Z-Machine object table entries?
- What connects object definitions in .grue to final Z-Machine format?

### 2. Property Table Generation
- How are property tables allocated and addressed?
- What determines property table memory layout?
- How are property addresses calculated and validated?

### 3. Dictionary Integration
- How do object names get added to the Z-Machine dictionary?
- What connects Property 27 (names) to dictionary entries?
- How does object lookup resolve names to object numbers?

### 4. Variable Source Tracking Integration Points
- Which compilation phases does Variable Source Tracking modify?
- How does iterator pattern generation affect object creation?
- What object-related IR generation changed?

## Validation Requirements for Re-Integration

### 1. Object Table Verification
```bash
# Must show proper object names before integration
RUST_LOG=info ./target/debug/gruesome tests/baseline.z3 --debug-objects
# Expected output:
# Object #1: 'player'
# Object #2: 'west_of_house'
# Object #7: 'Up a Tree'
```

### 2. Compilation Object Tracking
```bash
# Must show object creation with names
RUST_LOG=debug ./target/debug/grue-compiler examples/mini_zork.grue -o test.z3 2>&1 | grep "Object.*Up a Tree"
```

### 3. Functional Testing
```bash
# Must work without crashes
echo -e "north\nnorth\nexamine nest" | ./target/debug/gruesome test.z3
```

## Re-Integration Strategy

### Phase 1: Establish Working Baseline
1. Rollback to last known working commit
2. Verify object table generation works correctly
3. Document working object creation pipeline
4. Create regression test suite

### Phase 2: Incremental Variable Source Tracking
1. Apply Variable Source Tracking changes ONE FILE AT A TIME
2. Test object table generation after each file
3. Stop and debug immediately when objects break
4. Identify exact change that damages object pipeline

### Phase 3: Targeted Integration
1. Isolate Variable Source Tracking changes that don't affect objects
2. Modify object-affecting changes to preserve object pipeline
3. Test extensively before proceeding to next change

### Phase 4: Property System Enhancement
1. Only after objects work: apply Property 28 collision fixes
2. Only after properties work: apply dictionary resolution
3. Verify each system independently before combining

## Critical Success Criteria

**Before claiming any integration works:**
1. Object table dump shows proper names for all objects
2. "examine tree" works (or gives proper "not found" message)
3. "examine nest" works and shows nest description
4. All room navigation works correctly
5. No bounds errors or memory corruption

**Never again:**
- Claim integration success without `--debug-objects` verification
- Apply multiple complex changes simultaneously
- Assume PropertyManager fixes resolve object creation failures
- Fix symptoms (object 0 validation) while ignoring root causes (empty objects)

## Files to Investigate During Re-Integration

**Critical Object Creation Files:**
- `src/grue_compiler/ir.rs` - Object IR generation
- `src/grue_compiler/codegen.rs` - Object ID mapping
- `src/grue_compiler/codegen_objects.rs` - Z-Machine object table generation
- `src/grue_compiler/codegen_strings.rs` - Object name handling

**Integration Points:**
- Object vocabulary collection
- Property 27 names generation
- DictionaryRef resolution
- Variable Source Tracking iterator patterns

This investigation revealed that object table generation is completely non-functional, making all other fixes irrelevant until the basic object creation pipeline is restored.