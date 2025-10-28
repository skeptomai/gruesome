# Object Lookup Dictionary Address Implementation Plan

**Date**: October 28, 2025
**Objective**: Replace temporary hardcoded fix with proper dictionary address system
**Risk Level**: MEDIUM - Core compilation system changes
**Estimated Duration**: 2-3 hours with testing

## Prerequisites

- [x] Temporary fix working (mailbox object #10 hardcoded)
- [x] Analysis complete (Property 18 must store dictionary addresses)
- [x] Commercial Zork I reference behavior documented
- [x] Phase 1: Investigation & Setup - COMPLETE ✅ (October 28, 2025)
- [x] Phase 2: Dictionary Integration Analysis - COMPLETE ✅ (October 28, 2025)
- [x] Implementation plan reviewed and approved

## Phase 1: Investigation & Setup (30 minutes) - COMPLETE ✅ (October 28, 2025)

**Results**:
- ✅ Dictionary system analyzed: `dictionary_words: Vec<String>` field and `lookup_word_in_dictionary()` methods found
- ✅ Property 18 location identified: `ir.rs:1584` currently sets `StandardProperty::ShortName = 1` instead of property 18
- ✅ Baseline established: `tests/baseline_before_fix.z3` created and verified temporary fix works

## Phase 2: Dictionary Integration Analysis (45 minutes) - COMPLETE ✅ (October 28, 2025)

**Results**:
- ✅ **Dictionary Creation Timeline**: Step 2b (Dictionary) → Step 2c (Objects) confirmed
- ✅ **Object Names Added**: During `generate_dictionary_space()` at `codegen_strings.rs:395-400`
- ✅ **Object Properties Generated**: During `generate_object_tables()` at `codegen_objects.rs:284+`
- ✅ **Dictionary Available**: YES - dictionary fully populated before object property generation
- ✅ **Access Interface**: `self.lookup_word_in_dictionary(word: &str) -> Result<u16, CompilerError>` at `codegen.rs:5656-5687`

## Phase 3: Implementation (60 minutes)

### Step 3.1: Modify Property 18 Generation (ir.rs)
**File**: `src/grue_compiler/ir.rs` around line 1574

**Current Code**:
```rust
properties.set_string(StandardProperty::ShortName as u8, short_name.clone());
```

**New Implementation**:
```rust
// PHASE 3.1: Store dictionary addresses in property 18 instead of string IDs
let mut dict_address_bytes = Vec::new();

for name in &obj.names {
    if let Some(dict_addr) = self.dictionary.get_address(name) {
        // Store as big-endian 2-byte address
        dict_address_bytes.extend_from_slice(&dict_addr.to_be_bytes());
        log::debug!("DICT_ADDR: '{}' -> 0x{:04x}", name, dict_addr);
    } else {
        log::warn!("Dictionary address not found for object name: '{}'", name);
    }
}

if !dict_address_bytes.is_empty() {
    // Set property 18 with dictionary addresses (not string ID!)
    properties.set_bytes(18, dict_address_bytes);
    log::debug!("PROP18_ADDR: Object '{}' property 18 = {} bytes", obj.identifier, dict_address_bytes.len());
}
```

### Step 3.2: Test Compilation Changes
```bash
# Goal: Verify modified compilation produces correct property 18 format
RUST_LOG=debug cargo run --bin grue-compiler -- examples/mini_zork.grue -o tests/dict_addr_test.z3 2>&1 | grep "DICT_ADDR\|PROP18_ADDR"
```

**Expected Output**:
```
DICT_ADDR: 'small' -> 0x0725
DICT_ADDR: 'mailbox' -> 0x0726
PROP18_ADDR: Object 'mailbox' property 18 = 4 bytes
```

**Verification**: Property 18 contains dictionary addresses, not string IDs

### Step 3.3: Test Object Lookup Runtime
```bash
# Goal: Verify object lookup works with new property 18 format
echo "open mailbox" | RUST_LOG=debug ./target/debug/gruesome tests/dict_addr_test.z3 2>&1 | grep "OBJECT_LOOKUP"
```

**Expected Result**: Object lookup finds mailbox without infinite loop
**If Successful**: "Opened." message
**If Failed**: Infinite loop or "I don't understand"

## Phase 4: Testing & Validation (45 minutes)

### Step 4.1: Multi-Object Testing
```bash
# Goal: Test multiple objects with multiple names
echo -e "examine mailbox\ntake leaflet\nopen window\nquit\ny" | RUST_LOG=error ./target/debug/gruesome tests/dict_addr_test.z3
```

**Expected Results**:
- "examine mailbox" → description shown
- "take leaflet" → "Taken."
- "open window" → appropriate response
- No infinite loops

### Step 4.2: Dictionary Address Verification
**Create Verification Script**:
```bash
# Goal: Verify property 18 contains correct dictionary addresses
RUST_LOG=debug cargo run --bin grue-compiler -- examples/mini_zork.grue -o tests/verify_addresses.z3 2>&1 | \
grep -E "(Added dictionary entry|PROP18_ADDR)" | head -20
```

**Manual Verification**:
- Find "Added dictionary entry: 'mailbox' -> [hex]"
- Find "PROP18_ADDR: Object 'mailbox' property 18 = X bytes"
- Verify addresses match between dictionary and property 18

### Step 4.3: Regression Testing
```bash
# Goal: Ensure other functionality still works
cargo test
echo -e "look\nnorth\nsouth\neast\nwest\nquit\ny" | RUST_LOG=error ./target/debug/gruesome tests/dict_addr_test.z3
```

**Expected Results**: All basic commands work, no regressions

## Phase 5: Cleanup & Finalization (30 minutes)

### Step 5.1: Remove Temporary Hardcoded Fix
**File**: `src/grue_compiler/codegen.rs:5869-5896`

**Remove**:
```rust
// TEMPORARY WORKAROUND: Hardcode mailbox object #10 match
```

**Replace with**:
```rust
// PROPER IMPLEMENTATION: Compare property 18 dictionary addresses
// (Revert to original property 18 comparison)
```

### Step 5.2: Final Integration Test
```bash
# Goal: Verify complete solution without temporary fix
cargo run --bin grue-compiler -- examples/mini_zork.grue -o tests/final_object_lookup.z3
echo -e "open mailbox\nexamine box\ntake small\nquit\ny" | RUST_LOG=error ./target/debug/gruesome tests/final_object_lookup.z3
```

**Expected Results**: All object interactions work correctly

### Step 5.3: Comprehensive Testing
```bash
# Goal: Test all object + verb combinations
echo -e "open mailbox\nexamine mailbox\ntake leaflet\nread leaflet\nexamine window\nopen window\nquit\ny" | \
./target/debug/gruesome tests/final_object_lookup.z3
```

**Expected Results**: No infinite loops, all commands respond appropriately

## Rollback Plan

**If Phase 3 fails**:
```bash
git checkout HEAD -- src/grue_compiler/ir.rs
cargo build
# Verify temporary fix still works
echo "open mailbox" | ./target/debug/gruesome tests/baseline_before_fix.z3
```

**If Phase 5 fails**:
```bash
git checkout HEAD -- src/grue_compiler/codegen.rs
# Keep Phase 3 changes, restore temporary fix
```

## Success Criteria

- [ ] Property 18 stores dictionary addresses (verified in debug logs)
- [ ] Object lookup finds objects without infinite loops
- [ ] Multiple object names work (small, mailbox, box)
- [ ] All verb + object combinations respond correctly
- [ ] No regressions in basic game functionality
- [ ] Temporary hardcoded fix removed
- [ ] Commercial Zork I behavior replicated

## Error Handling

**Common Issues & Solutions**:

1. **Dictionary not available during object generation**
   - Move object generation after dictionary creation
   - Or pass dictionary reference to object generation

2. **Wrong property format**
   - Verify `set_bytes()` vs `set_string()` usage
   - Check big-endian vs little-endian byte order

3. **Infinite loop persists**
   - Verify property 18 contains correct addresses
   - Check object lookup logic matches new format

4. **Compilation errors**
   - Check dictionary interface methods available
   - Verify property setting API usage

## Documentation Updates

After successful implementation:
- Update `OBJECT_LOOKUP_DICTIONARY_ADDRESSES_ANALYSIS.md` with final status
- Add implementation notes to `CLAUDE.md`
- Document any discovered edge cases

---

**This plan provides step-by-step verification at each stage with clear rollback options if issues occur.**