# generate_verb_matching Function Analysis

## Overview
**File**: `src/grue_compiler/codegen_grammar.rs`
**Function**: `generate_verb_matching`
**Total Lines**: 1,526 lines (lines 26-1551)
**Status**: Successfully extracted to dedicated module, ready for internal refactoring

## Current Structure Analysis

### Section Breakdown

| Section | Lines | Size | Description |
|---------|-------|------|-------------|
| **Setup & Initialization** | 26-344 | 319 lines | Function signature, verb start setup, parse buffer constants, initial word count check |
| **Literal Patterns** | 345-580 | 236 lines | Single-word literal patterns (e.g., "around" in "look around") |
| **Pattern Distinction Setup** | 581-840 | 260 lines | Find default/noun patterns, branch setup, disabled old literal code |
| **Literal+Noun Patterns** | 842-1079 | 238 lines | Two-element patterns (e.g., "at mailbox" in "look at mailbox") |
| **Verb+Noun Patterns** | 1080-1236 | 157 lines | Standard verb+object patterns (e.g., "take lamp") |
| **Default Pattern & Fallback** | 1237-1551 | 315 lines | Verb-only patterns and fallback to noun with object ID 0 |

### Pattern Type Distribution

```
Setup (319 lines) ========================= 21%
Literal (236 lines) =================== 15%
Distinction (260 lines) ==================== 17%
Literal+Noun (238 lines) =================== 16%
Verb+Noun (157 lines) ============= 10%
Default (315 lines) ======================== 21%
```

## Pattern Processing Logic

### 1. Literal Patterns (Lines 345-580)
**Purpose**: Handle single literal words after verb (e.g., "look around", "go back")

**Pattern Structure**: `[Literal("around")]`

**Generated Code Flow**:
1. Filter patterns to find single-literal patterns
2. For each literal:
   - Check word count == 2 (verb + literal)
   - Load word 1 from parse buffer
   - Compare with literal's dictionary address
   - Branch to handler function if match
   - Skip to next pattern if no match

**Complexity Factors**:
- Dictionary address resolution
- Parse buffer manipulation
- Multiple literal patterns per verb
- Skip label generation and tracking

### 2. Literal+Noun Patterns (Lines 842-1079)
**Purpose**: Handle preposition + object (e.g., "look at mailbox", "put lamp in box")

**Pattern Structure**: `[Literal("at"), Noun]`

**Generated Code Flow**:
1. Check word count >= 3 (verb + literal + noun)
2. Load word 1 from parse buffer
3. Compare with literal's dictionary address
4. If match:
   - Load word 2 (the noun)
   - Look up object in game objects
   - Call handler with object as parameter
5. Jump back to main loop

**Complexity Factors**:
- Three-word parsing
- Object resolution from dictionary
- RuntimeParameter resolution for $2 (the noun)
- Multiple literal+noun combinations possible

### 3. Verb+Noun Patterns (Lines 1080-1236)
**Purpose**: Handle standard verb + object (e.g., "take lamp", "open mailbox")

**Pattern Structure**: `[Noun]`

**Generated Code Flow**:
1. Assumes word count >= 2 (checked earlier)
2. Load word 1 from parse buffer
3. Look up object in game objects
4. Call handler with object as parameter
5. Jump back to main loop

**Complexity Factors**:
- Object dictionary lookup
- RuntimeParameter("noun") resolution
- Polymorphic dispatch to handler functions

### 4. Default Pattern (Lines 1237-1551)
**Purpose**: Handle verb-only commands (e.g., "look", "inventory", "wait")

**Pattern Structure**: `[Default]`

**Generated Code Flow**:
1. Execute when word count < 2
2. Call handler function with no object parameter
3. If no default pattern exists, fall back to noun pattern with object ID 0
4. Jump back to main loop

**Complexity Factors**:
- Fallback logic to noun handler with 0
- RuntimeParameter handling for both cases

## Key Observations

### ✅ Good Separation Points

1. **Literal Pattern Block** (345-580): Clean boundary, processes single-literal patterns only
2. **Literal+Noun Block** (842-1079): Distinct loop structure, handles 2-element patterns
3. **Verb+Noun Block** (1080-1236): Straightforward object resolution logic
4. **Default Block** (1237-1551): Clear verb-only handling with fallback

### ⚠️ Challenges for Extraction

1. **Shared State Dependencies**:
   - All sections use `self.code_address`, `self.reference_context`, `self.label_addresses`
   - Parse buffer global constant (110) used throughout
   - `main_loop_jump_id` parameter needed by all sections

2. **Shared Helper Patterns**:
   - Dictionary address loading (repeated in literal, literal+noun, verb+noun)
   - Object lookup from parse buffer (repeated in literal+noun, verb+noun)
   - UnresolvedReference creation (repeated everywhere)
   - Jump back to main loop (repeated in all handlers)

3. **Disabled Code Block** (653-840):
   - Contains old literal pattern code wrapped in `if false {}`
   - Could be deleted entirely (188 lines)
   - Purpose: historical reference or debugging?

4. **Interleaved Logic**:
   - Setup section (26-344) contains word count check used by later sections
   - Pattern distinction (581-840) sets up variables used by all pattern handlers
   - Cannot extract handlers without setup code

## Refactoring Recommendations

### Option A: Extract Pattern Handlers as Methods (RECOMMENDED)

**Approach**: Keep all code in `codegen_grammar.rs`, split into helper methods

```rust
// Main orchestrator (~200 lines)
pub fn generate_verb_matching(...) -> Result<(), CompilerError> {
    // Setup (lines 26-344)
    let (verb_start_address, word_count_var, ...) = self.setup_verb_matching(verb)?;

    // Process literal patterns (lines 345-580)
    self.generate_literal_patterns(verb, patterns, &context)?;

    // Process literal+noun patterns (lines 842-1079)
    self.generate_literal_noun_patterns(verb, patterns, &context)?;

    // Branch: word count < 2 goes to default, >= 2 goes to noun
    let verb_only_label = self.emit_word_count_branch(&context)?;

    // Process verb+noun patterns (lines 1080-1236)
    self.generate_verb_noun_pattern(verb, patterns, &context)?;

    // Process default pattern (lines 1237-1551)
    self.generate_default_pattern(verb, patterns, &context, verb_only_label)?;

    // Cleanup
    self.finalize_verb_matching(verb_start_address, end_function_label)?;
    Ok(())
}

// Helper methods (each 200-300 lines)
fn setup_verb_matching(&mut self, verb: &str) -> Result<VerbContext, CompilerError> { ... }
fn generate_literal_patterns(&mut self, verb: &str, patterns: &[IrPattern], ctx: &VerbContext) -> Result<(), CompilerError> { ... }
fn generate_literal_noun_patterns(&mut self, verb: &str, patterns: &[IrPattern], ctx: &VerbContext) -> Result<(), CompilerError> { ... }
fn generate_verb_noun_pattern(&mut self, verb: &str, patterns: &[IrPattern], ctx: &VerbContext) -> Result<(), CompilerError> { ... }
fn generate_default_pattern(&mut self, verb: &str, patterns: &[IrPattern], ctx: &VerbContext, label: u32) -> Result<(), CompilerError> { ... }
fn finalize_verb_matching(&mut self, start: usize, end_label: u32) -> Result<(), CompilerError> { ... }
```

**Benefits**:
- ✅ Each pattern type isolated and testable
- ✅ Main function becomes readable orchestrator
- ✅ No module boundary issues (all in same impl block)
- ✅ Easy rollback (just method extraction)
- ✅ Shared state via `&mut self` - no parameter passing needed

**Estimated Breakdown**:
- Main orchestrator: ~150 lines
- `setup_verb_matching`: ~100 lines
- `generate_literal_patterns`: ~236 lines
- `generate_literal_noun_patterns`: ~238 lines
- `generate_verb_noun_pattern`: ~157 lines
- `generate_default_pattern`: ~315 lines
- `finalize_verb_matching`: ~50 lines
- Common helpers: ~200 lines (extract shared dictionary/object lookup patterns)

**Total**: ~1,450 lines (similar to current, but organized)

### Option B: Delete Disabled Code First

**Quick Win**: Remove lines 653-840 (188 lines of disabled code)

```rust
// LITERAL PATTERN CHECK: Before noun processing...
if false {  // ← Lines 653-840 are all inside this block!
    // Old literal pattern code
    // ...
} // END DISABLED OLD LITERAL PATTERN CODE
```

**Impact**: Reduces function to ~1,338 lines immediately with zero risk

### Option C: Context Struct for Shared Data

Create a `VerbMatchingContext` struct to reduce parameter passing:

```rust
struct VerbMatchingContext {
    verb_start_address: usize,
    end_function_label: u32,
    verb_only_label: u32,
    main_loop_jump_id: u32,
    parse_buffer_global: u8,
    default_pattern: Option<&IrPattern>,
    noun_pattern: Option<&IrPattern>,
}
```

**Benefits**: Cleaner method signatures, grouped related data

## Proposed Refactoring Strategy

### Phase 1: Quick Cleanup ⚡ (5 minutes)
1. Delete disabled code block (lines 653-840) → Save 188 lines
2. Test compilation, verify bytecode identical

### Phase 2: Extract Common Helpers 🔧 (30 minutes)
1. Extract `load_word_from_parse_buffer(word_index: u8) -> Result<u8, CompilerError>`
2. Extract `lookup_object_from_dictionary(word_var: u8) -> Result<(), CompilerError>`
3. Extract `emit_call_to_handler(func_id: IrId, args: &[Operand]) -> Result<(), CompilerError>`
4. Extract `emit_jump_to_main_loop(label_id: u32) -> Result<(), CompilerError>`
5. Test after each extraction

### Phase 3: Extract Pattern Handlers 🎯 (2-3 hours)
1. Extract `setup_verb_matching` - setup code
2. Extract `generate_literal_patterns` - literal pattern block
3. Extract `generate_literal_noun_patterns` - literal+noun block
4. Extract `generate_verb_noun_pattern` - verb+noun block
5. Extract `generate_default_pattern` - default pattern block
6. Extract `finalize_verb_matching` - cleanup code
7. Test after each extraction
8. Verify mini_zork compiles and runs correctly

### Phase 4: Refactor Main Function 📝 (30 minutes)
1. Rewrite main function as orchestrator calling helper methods
2. Add comments explaining flow
3. Final test: compile mini_zork, verify bytecode identical

## Success Criteria

✅ **Code identical**: Pre/post bytecode must be byte-for-byte identical
✅ **All tests pass**: No regression in test suite
✅ **Mini Zork works**: Full gameplay test successful
✅ **Readability improved**: Main function ~150 lines, clear flow
✅ **Each pattern isolated**: Individual pattern handlers ~200-300 lines
✅ **Easy to understand**: Each method has single responsibility

## Risk Assessment

**Low Risk Items**:
- Deleting disabled code (already not executed)
- Extracting common helpers (clearly reused patterns)
- Method extraction within same impl block (no visibility changes)

**Medium Risk Items**:
- Extracting setup code (used by multiple sections)
- Handling shared mutable state across methods

**Mitigation**:
- Test after each extraction
- Keep git commits small
- Verify bytecode identity after each phase
- Keep backups of working versions

## Estimated Timeline

**Phase 1**: 5 minutes
**Phase 2**: 30-60 minutes
**Phase 3**: 2-3 hours
**Phase 4**: 30 minutes

**Total**: 3-4 hours of careful work with testing

## Next Steps

1. Review this analysis with user
2. Get approval for approach (Option A recommended)
3. Start with Phase 1 (delete disabled code - quick win)
4. Proceed incrementally through phases
5. Commit after each successful phase
