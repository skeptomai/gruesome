# Runtime Array Implementation Assessment

## Current State

### Implemented IR Instructions:
1. **CreateArray** - Returns 0 (null) placeholder, no actual memory allocated
2. **ArrayAdd** - No-op, doesn't modify anything
3. **ArrayLength** - Returns 0 placeholder
4. **ArrayEmpty** - Not implemented
5. **GetArrayElement** - Uses placeholder value (1000), causes invalid object errors
6. **SetArrayElement** - Not implemented

### Problem:
When executing `let visible_objects = []; visible_objects.add(obj); for item in visible_objects { ... }`:
- CreateArray returns placeholder value (0 or 1000)
- ArrayAdd does nothing
- For-loop iteration calls GetArrayElement with placeholder, reads garbage memory
- Result: "Invalid object number: 1000" error

## Z-Machine Memory Model

### Memory Regions:
- **Dynamic Memory** (0x0000-0x??): Writable, includes globals, object tree, property tables
- **Static Memory** (0x??-0x7FFF): Read-only, includes dictionary, strings
- **High Memory** (0x8000+): Code (read-only)

**Constraint**: Arrays must be in **dynamic memory** to support write operations.

## Implementation Options

### Option A: Static Pre-allocated Array Table (Simplest)

**Approach**:
- Reserve a fixed region in dynamic memory for array storage
- Pre-allocate N array slots of fixed size at compile time
- Each CreateArray gets assigned to next available slot
- Track usage via compile-time slot allocator

**Memory Layout**:
```
[Array_Table_Base]
  [Array0: length_word][element0][element1]...[elementN-1]
  [Array1: length_word][element0][element1]...[elementN-1]
  ...
  [ArrayM: length_word][element0][element1]...[elementN-1]
```

**Pros**:
- Very simple implementation
- Predictable memory usage
- No runtime allocation logic needed
- Fast access (direct addressing)

**Cons**:
- Wastes memory (pre-allocates all slots)
- Hard limits on array count (e.g., max 50 arrays)
- Hard limits on array size (e.g., max 100 elements each)
- Cannot adjust to actual usage patterns

**Complexity**: Low (200 lines of code)

### Option B: Runtime Dynamic Allocation (Complex)

**Approach**:
- Implement a heap allocator in Z-Machine bytecode
- CreateArray calls allocator to request memory block
- Free memory when array goes out of scope
- Requires garbage collection or reference counting

**Memory Layout**:
```
[Heap_Base]
  [Block1_Header: size][data...]
  [Block2_Header: size][data...]
  [Free_Block_Header: size][unused...]
  ...
```

**Pros**:
- Efficient memory use (allocate only what's needed)
- Unlimited array count (within memory limits)
- Variable array sizes
- Production-grade solution

**Cons**:
- Very complex (500+ lines for allocator + GC)
- Performance overhead (allocation/deallocation)
- Fragmentation issues without compaction
- Debugging difficulty

**Complexity**: Very High (would require 2-3 days of work)

### Option C: Hybrid - Limited Dynamic Pool (Recommended)

**Approach**:
- Reserve a dynamic memory pool for arrays (e.g., 2KB)
- Simple bump allocator: tracks next free address
- Arrays stored as: [length_word][element1][element2]...
- No freeing - rely on limited scope (function-local arrays die when function returns)
- Arrays are effectively "leased" for the duration of execution

**Memory Layout**:
```
[Array_Pool_Base] (stored in global variable or header)
[Array_Pool_Current] (stored in global variable, advances with each allocation)
[Array_Pool_End] (fixed at Array_Pool_Base + 2KB)

Pool Contents:
  [Array0: length_word=3][elem0][elem1][elem2]
  [Array1: length_word=5][elem0][elem1][elem2][elem3][elem4]
  [Array_Pool_Current] <- points here
  [unused space...]
  [Array_Pool_End]
```

**Implementation**:
```pseudo
CreateArray(size):
  current = load_global(ARRAY_POOL_CURRENT)
  needed = 2 + (size * 2)  // length word + elements (2 bytes each)
  if current + needed > ARRAY_POOL_END:
    error("Out of array memory")
  store_word(current, 0)  // initial length = 0
  for i in 0..size:
    store_word(current + 2 + i*2, 0)  // zero-initialize elements
  store_global(ARRAY_POOL_CURRENT, current + needed)
  return current  // array handle is its base address

ArrayAdd(array_base, value):
  length = load_word(array_base)
  store_word(array_base + 2 + length*2, value)
  store_word(array_base, length + 1)

GetArrayElement(array_base, index):
  return load_word(array_base + 2 + index*2)

ArrayLength(array_base):
  return load_word(array_base)
```

**Pros**:
- Reasonable memory efficiency
- Manageable complexity (300-400 lines)
- Good performance (simple pointer arithmetic)
- Handles most practical use cases

**Cons**:
- Can run out of space in long-running games
- No memory reclamation (leak until game restart)
- Limited total array capacity
- Need to decide pool size at compile time

**Complexity**: Medium (1-2 days of focused work)

### Option D: Optimization - Eliminate Arrays (Fastest to Implement)

**Approach**:
- Detect specific pattern: `let arr = []; for x in collection { if condition { arr.add(x) } }; for y in arr { ... }`
- Transform to: `for x in collection { if condition { ... } }`
- Inline the filter logic directly into iteration

**Pros**:
- No implementation needed (optimization only)
- Optimal performance (no array overhead)
- Solves immediate problem

**Cons**:
- Doesn't help general array use cases
- Complex pattern matching/transformation logic
- Brittle (minor code changes break optimization)
- Not scalable to other array uses

**Complexity**: Medium (pattern matching + AST transformation)

## Recommendation

**Implement Option C: Hybrid Limited Dynamic Pool**

### Constraints:
- Fixed 2KB array pool in dynamic memory
- Maximum 100 arrays simultaneously (average 20 bytes each)
- Maximum 256 elements per array (512 bytes + 2 byte header)
- Function-scoped allocation model (no explicit free)

### Rationale:
1. **Solves the immediate problem**: Arrays will work for mini_zork use cases
2. **Manageable scope**: Can be implemented and tested in 1-2 days
3. **Good performance**: Direct memory access, no GC overhead
4. **Upgrade path**: Can later add freeing/GC if needed
5. **Practical limits**: 2KB is enough for typical game logic arrays

### Required Changes:

#### 1. Header Setup (codegen_headers.rs):
- Reserve 2KB region after globals, before objects
- Add ARRAY_POOL_BASE global variable (points to pool start)
- Add ARRAY_POOL_CURRENT global variable (points to next free address)
- Initialize ARRAY_POOL_CURRENT = ARRAY_POOL_BASE in init routine

#### 2. CreateArray (codegen_instructions.rs):
- Load ARRAY_POOL_CURRENT
- Check bounds: current + needed < ARRAY_POOL_BASE + 2048
- Store size at [current]
- Zero-initialize elements
- Store current + needed → ARRAY_POOL_CURRENT
- Push current to stack (array handle)

#### 3. ArrayAdd (codegen_instructions.rs):
- Resolve array_base and value operands
- Load length from [array_base]
- Store value at [array_base + 2 + length*2]
- Store length+1 at [array_base]

#### 4. GetArrayElement (codegen_instructions.rs):
- Resolve array_base and index operands
- Load value from [array_base + 2 + index*2]
- Push to stack

#### 5. ArrayLength (codegen_instructions.rs):
- Resolve array_base operand
- Load value from [array_base]
- Push to stack

#### 6. ArrayEmpty (codegen_instructions.rs):
- Resolve array_base operand
- Load length from [array_base]
- Push (length == 0) to stack

### Testing Strategy:
1. Unit tests for each array operation
2. Integration test: create array, add elements, iterate, check length
3. Bounds test: verify out-of-memory error when pool exhausted
4. mini_zork test: list_objects function should work correctly

### Risk Assessment:
- **Memory layout changes**: Medium risk - must update header addresses carefully
- **Global variable allocation**: Low risk - well-understood pattern
- **Z-Machine instruction correctness**: Medium risk - need careful testing of loadw/storew
- **Performance impact**: Low - array operations are simple memory access

### Estimated Effort:
- Implementation: 4-6 hours
- Testing: 2-3 hours
- Debugging: 2-4 hours
- **Total**: 1-2 days

## Future Enhancements (Post-MVP)

### Phase 2: Add Basic Freeing
- Track array reference counts
- Free when count reaches zero
- Simple mark-and-sweep at function return

### Phase 3: Full Dynamic Allocator
- Replace bump allocator with free list
- Support arbitrary allocation/deallocation
- Defragmentation/compaction

### Phase 4: Optimization
- Compile-time array analysis
- Inline small arrays into local variables
- Eliminate unnecessary allocations
