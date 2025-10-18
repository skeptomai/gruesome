# Dual Numbering System Analysis

## The Fundamental Architectural Flaw

The timing issue in InsertObj generation exists because we have **two separate numbering systems** that produce different results:

## The Dual Numbering Problem

**Phase 1: IR Semantic Analysis** (during parsing)
- Objects get numbered as they're encountered: `tree = #13, forest_path = #9`
- This happens in `ir.rs` during semantic analysis
- Used by early IR generation and InsertObj instruction creation

**Phase 2: Object Table Generation** (during codegen)
- Objects get re-numbered for Z-Machine layout: `tree = #10, forest_path = #6`
- This happens in `codegen_objects.rs`
- Uses specific ordering: player first, then rooms, then objects
- Creates the ACTUAL runtime object numbers

## Why This Architecture Exists

Looking at the code, this seems to have evolved for these reasons:

1. **IR wants early numbering** for validation and cross-references
2. **Object table generation has Z-Machine constraints** (player must be #1, memory layout requirements)
3. **Incremental development** - IR numbering was added first, object table numbering later

## The Fundamental Flaw

```rust
// During init block (BEFORE object table generation)
InsertObj(13, 9)  // tree into forest_path - WRONG NUMBERS!

// But runtime object table has:
// tree = Object #10, forest_path = #6
// So this inserts wrong objects into wrong parents!
```

## Better Architecture Options

**Option 1: Single Source of Truth**
- Only assign object numbers during object table generation
- IR uses symbolic references until codegen
- No dual numbering system

**Option 2: Early Final Numbering**
- Determine final Z-Machine numbering during IR phase
- Object table generation uses pre-assigned numbers
- Requires IR to know Z-Machine constraints

**Option 3: Deferred InsertObj Generation**
- Generate InsertObj instructions as IR during semantic analysis
- Translate object references during codegen when final numbers known

## Current Issues

The current fix (Option 3) is a band-aid. The real issue is having **two conflicting sources of truth** for object numbers. This creates a **temporal coupling** where the order of compilation phases matters for correctness.

## Recommended Solution

Eliminate one of the numbering systems entirely, rather than carefully coordinating between them. The architecture should have a single, consistent object numbering scheme that works across all compilation phases.