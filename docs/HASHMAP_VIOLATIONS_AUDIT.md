# HashMap/HashSet Violations Audit - Nov 12, 2025

## Summary
**54 HashMap/HashSet violations** found across the codebase, violating the determinism rule in CLAUDE.md.

## CRITICAL - Compiler Core (Affects Build Determinism)

### `src/grue_compiler/ir.rs`
```rust
function_id_map: std::collections::HashMap<(String, ObjectSpecialization), u32>
let mut function_id_map: std::collections::HashMap<(String, ObjectSpecialization), u32> = HashMap::new();
```
**Impact**: Still has HashMap even after my "IndexMap fix" - affects function ID assignment

### `src/grue_compiler/semantic.rs`
```rust
pub symbols: HashMap<String, Symbol>
room_objects: HashMap<String, Vec<String>>
```
**Impact**: Symbol resolution and room object tracking non-deterministic

### `src/grue_compiler/object_system.rs`
```rust
pub attributes: HashMap<StandardAttribute, bool>
pub properties: HashMap<StandardProperty, PropertyValue>
pub custom_properties: HashMap<u8, PropertyValue>
defaults: HashMap<StandardProperty, PropertyValue>
```
**Impact**: Object attribute/property ordering non-deterministic

### `src/grue_compiler/parser.rs`
```rust
let mut properties = HashMap::new();
numbered_properties: HashMap::new()
```
**Impact**: Property parsing order non-deterministic

### `src/grue_compiler/ast.rs`
```rust
pub properties: HashMap<String, PropertyValue>
pub numbered_properties: HashMap<u8, PropertyValue>
```
**Impact**: AST property ordering non-deterministic

### `src/grue_compiler/codegen_utils.rs`
```rust
let mut instruction_counts: HashMap<String, usize> = HashMap::new();
```
**Impact**: Instruction counting/analysis non-deterministic

## HIGH PRIORITY - Runtime Systems

### `src/runtime_parser.rs`
```rust
pub game_objects: HashMap<String, GameObjectInfo>
pub attributes: HashMap<String, bool>
pub name_to_object: HashMap<String, u16>
pub object_info: HashMap<u16, GameObjectInfo>
```
**Impact**: Game object resolution non-deterministic

### `src/parser_engine.rs`
```rust
pub object_names: HashMap<String, ObjectInfo>
pub adjectives: HashSet<String>
pub prepositions: HashSet<String>
pub articles: HashSet<String>
pub pronouns: HashMap<String, PronounType>
pub conjunctions: HashSet<String>
pub verb_synonyms: HashMap<String, String>
pub noun_synonyms: HashMap<String, String>
```
**Impact**: Command parsing behavior non-deterministic

## MEDIUM PRIORITY - Tools/Analysis (25+ violations)

### Analysis Binaries (`src/bin/`)
- `analyze_extra_routines.rs` - 4 HashMap/HashSet violations
- `analyze_routine_patterns.rs` - 6 HashMap/HashSet violations
- `analyze_txd_heuristics.rs` - 5 HashMap/HashSet violations
- `implement_txd_filters.rs` - 8 HashMap/HashSet violations
- Plus 10+ other binaries with violations

### Other Tools
- `src/disasm_txd.rs` - 4 HashMap violations
- `src/disassembler.rs` - 1 HashSet violation
- `src/util.rs` - 1 HashMap violation

## LOW PRIORITY - Tests
- `src/grue_compiler/opcodes_tests.rs` - HashSet in tests (acceptable for testing)

## Recommendation
**Fix CRITICAL and HIGH PRIORITY violations first** - these directly affect:
1. Build determinism (compiler core)
2. Game behavior consistency (runtime systems)

The tool violations are less critical but should be fixed for consistency.