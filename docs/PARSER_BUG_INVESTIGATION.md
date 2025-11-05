# Parser Bug Investigation: "climb tree" Returns Wrong Object

## Problem Statement
When typing "climb tree" in the Forest Path room, the Z-Machine parser returns object 13 (egg) instead of the tree object. The tree is visible in the room and has correct names, but the parser lookup is failing.

## Evidence
- **Command**: `climb tree`
- **Expected**: Tree object (should be object 10)
- **Actual**: Object 13 (likely the egg)
- **Tree names**: `["tree", "large tree"]`
- **Egg names**: `["jewel-encrusted egg", "egg", "large egg"]`
- **Tree location**: Correctly placed in forest_path room
- **Debug output**: `13You can't climb that.`

## Analysis Needed
1. **Dictionary Compilation**: How are object names compiled into the Z-Machine dictionary?
2. **Parser Logic**: How does the Z-Machine parser search for objects by name?
3. **Object Resolution**: What builtin functions map dictionary words to object numbers?
4. **Name Conflicts**: Are there any naming conflicts causing wrong object lookup?

## Key Questions
- Why does "tree" lookup return object 13 instead of the tree object?
- Is this a dictionary compilation issue or runtime parser issue?
- Are object names being stored correctly in the dictionary?
- Is the parser searching in the wrong scope or order?

## Investigation Status
- [x] Confirmed tree object placement works (navigation succeeds)
- [x] Confirmed tree has correct names in source code
- [x] Confirmed parser returns wrong object number (13 vs expected tree)
- [x] Found dictionary compilation code
- [x] Found parser/object resolution builtin functions
- [x] Identified root cause of wrong object lookup
- [ ] Implement fix
- [ ] Verify climb tree works

## Root Cause Found
The bug is in `generate_object_lookup_from_noun()` in `src/grue_compiler/codegen_lookup.rs` at lines 347-423.

The function hardcodes checking exactly 3 dictionary addresses (offset 0, 1, 2) from Property 18:
- Load dictionary address at offset 0, compare with parser input
- Load dictionary address at offset 1, compare with parser input
- Load dictionary address at offset 2, compare with parser input

However, different objects have different numbers of names:
- Tree has 2 names: ["tree", "large tree"] → Property 18 has 2 dictionary addresses
- Egg has 3 names: ["jewel-encrusted egg", "egg", "large egg"] → Property 18 has 3 dictionary addresses

When the parser looks for "tree", it finds the egg's third dictionary address (offset 2) which contains "egg", but the tree object only has addresses at offsets 0 and 1. The hardcoded offset 2 check matches against object 13 (egg) instead of object 10 (tree).

## Fix Required
The `generate_object_lookup_from_noun()` function needs to dynamically check all dictionary addresses in Property 18 for each object, not just hardcode offsets 0, 1, and 2. It should:

1. Get Property 18 size for current object
2. Loop through all 2-byte dictionary addresses (size/2 iterations)
3. Compare each address against parser input
4. Stop when match found

## Next Steps
1. ✅ Implement dynamic Property 18 address checking in codegen_lookup.rs
2. ✅ Test the fix with mini_zork
3. ✅ Fixed V3 compatibility issue (Invalid Long form opcode 0x17)
4. ✅ Fixed infinite loop due to invalid local variable usage (Variable(8) → Variable(3))
5. ❌ **STILL BROKEN**: Parser returns object 13 instead of tree object when searching for "tree"

## Current Status (Updated)
The dynamic Property 18 address checking has been implemented but the parser is still returning object 13 (egg) instead of the tree object when searching for "tree". This suggests either:

1. The dynamic fix isn't being applied correctly
2. There's another issue in the parser logic
3. The tree object isn't properly configured in the dictionary

## ROOT CAUSE IDENTIFIED ✅

**BUG LOCATION**: UnresolvedReference resolution system for Property 18 dictionary addresses

**EVIDENCE**: Object 13 (egg) has incorrect Property 18 data:
- Address 0: 0x0072 (should be "jewel-encrusted egg")
- Address 1: 0x0ab8 (**THIS IS "tree"! Should be "egg"**)
- Address 2: 0x0a58 (should be "large egg")

**ANALYSIS**:
- Parser finds "tree" dictionary address 0x0ab8 in object 13's Property 18 at position 1
- Parser correctly returns object 13 when searching for "tree"
- Bug is NOT in parser logic - parser is working correctly
- Bug is in Property 18 compilation where wrong dictionary addresses get assigned

**COMPILATION SHOWS CORRECT MAPPING**:
```
Egg object names: ["jewel-encrusted egg", "egg", "large egg"]
Tree object names: ["tree", "large tree"]
```

**BUT COMPILED BYTES ARE WRONG**:
Object 13 Property 18 contains 0x0ab8 ("tree" address) instead of "egg" address.

**FIX REQUIRED**: Debug UnresolvedReference resolution in `src/grue_compiler/codegen.rs` lines 1614-1647 to find why dictionary addresses are being assigned to wrong objects.