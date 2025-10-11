# Z-Machine Global Variables Allocation Map

This document defines the allocation and usage of Z-Machine global variables to prevent collisions, corruption, and double-use.

## Z-Machine Variable System

### Variable Number Mapping
- **Variable 0**: Stack (special - always refers to top of stack)
- **Variables 1-15**: Local variables (function parameters and temporaries)
- **Variables 16-255**: Global variables (G00-G239)

### Global Variable Memory Layout
- Globals table base address: Stored in header at 0x0C-0x0D
- Each global is 2 bytes (word)
- Global Gxx at: `globals_addr + (xx * 2)`
- Variable number = 16 + global number
  - Example: Global G00 = Variable 16
  - Example: Global G01 = Variable 17
  - Example: Global G200 = Variable 216

## Reserved/System Globals (DO NOT USE FOR TEMPORARY STORAGE!)

### G00 (Variable 16)
- **Purpose**: Player object number
- **Memory**: globals_addr + 0 (2 bytes)
- **Usage**: Read-only reference to player object
- **Initialized**: Set to 1 (player is object #1) during compilation
- **Files**:
  - codegen.rs:5323-5326 (initialization)
  - codegen.rs:7446 (player reference resolution)
  - ir.rs:2522 (IR generation for player)

### G01 (Variable 17) - **⚠️ NEVER USE FOR TEMPORARY STORAGE!**
- **Purpose**: SCORE - displayed in status line
- **Memory**: globals_addr + 2 (2 bytes)
- **Usage**: Game score (read by status line display)
- **Critical**: Used by interpreter for score display (opcodes_display.rs:86, interpreter.rs:1671)
- **Bug History**: Was incorrectly used as temp storage in grammar handler (Bug #12, fixed Oct 11, 2025)

### G02 (Variable 18) - **⚠️ NEVER USE FOR TEMPORARY STORAGE!**
- **Purpose**: MOVES/TURNS - displayed in status line
- **Memory**: globals_addr + 4 (2 bytes)
- **Usage**: Move counter (read by status line display)
- **Critical**: Used by interpreter for moves display (opcodes_display.rs:87, interpreter.rs:1672)

### G16 (Variable 32)
- **Purpose**: Player location (used by status line in some games)
- **Memory**: globals_addr + 32 (2 bytes)
- **Usage**: Current room object number
- **Files**: opcodes_display.rs:77, interpreter.rs:1669

## Compiler-Allocated Globals

### G109 (Variable 125)
- **Purpose**: Text buffer address for SREAD
- **Memory**: globals_addr + 218 (2 bytes)
- **Usage**: Parse buffer input
- **Files**: codegen.rs:5526-5527
- **Constant**: TEXT_BUFFER_GLOBAL = 109

### G110 (Variable 126)
- **Purpose**: Parse buffer address for SREAD
- **Memory**: globals_addr + 220 (2 bytes)
- **Usage**: Tokenized input storage
- **Files**: codegen.rs:5554-5555
- **Constant**: PARSE_BUFFER_GLOBAL = 110

### G200-G249 (Variables 216-265)
- **Purpose**: Temporary storage for builtins and compiler-generated code
- **Memory**: globals_addr + 400 to globals_addr + 498 (100 bytes)
- **Usage**:
  - Grammar pattern matching temp storage (G200/Variable 216)
  - Builtin function return values (allocated dynamically, 200 + (count % 50))
  - Property access intermediate results
- **Allocation**:
  - `allocate_global_for_ir_id()` in codegen.rs:445-449
  - Grammar handler: codegen.rs:5844, 5891
  - GetPropertyByNumber: codegen_instructions.rs:603-605
- **Safe Range**: These are far from game state and will not interfere with status line display

## Allocation Rules

### DO NOT USE (Reserved for Game State):
- **G00-G02**: System globals (player, score, moves)
- **G03-G15**: Reserved for future game state expansion
- **G16-G31**: Game-specific state variables (location, inventory, flags, etc.)

### SAFE TO USE (Compiler/Temporary Storage):
- **G100-G119**: Input/output system (buffers, parse state)
- **G120-G199**: Available for compiler allocation
- **G200-G239**: Temporary storage for builtins and generated code

### Allocation Strategy:
1. **Check existing allocations** before using a new global
2. **Document all new globals** in this file
3. **Use high-numbered globals** (G200+) for temporary storage
4. **Never reuse G01-G02** - they are displayed in status line
5. **Use modulo allocation** for dynamic temps: `200 + (count % 50)`

## Common Bugs to Avoid

### Bug #12: Score Corruption (Fixed Oct 11, 2025)
- **Symptom**: Score showing dictionary addresses (1973, 1997, etc.)
- **Cause**: Grammar handler wrote dictionary addresses to G01 (Variable 17 = score)
- **Fix**: Changed to use G200 (Variable 216) instead
- **Files**: codegen.rs:5828-5831, 5844, 5891
- **Lesson**: NEVER use G01-G02 for temporary storage!

### Prevention Checklist:
- [ ] Did you check if the global overlaps with score/moves (G01-G02)?
- [ ] Did you document the global's purpose in this file?
- [ ] Did you use a high-numbered global (G200+) for temps?
- [ ] Did you verify no other code uses the same global?
- [ ] Did you test that score/moves display correctly?

## Quick Reference Table

| Global | Variable | Offset | Purpose | Writable? |
|--------|----------|--------|---------|-----------|
| G00 | 16 | +0 | Player object | Init only |
| G01 | 17 | +2 | Score (status line) | Game only |
| G02 | 18 | +4 | Moves (status line) | Game only |
| G16 | 32 | +32 | Player location | Game only |
| G109 | 125 | +218 | Text buffer addr | Compiler |
| G110 | 126 | +220 | Parse buffer addr | Compiler |
| G200 | 216 | +400 | Grammar temp | Compiler |
| G201-G249 | 217-265 | +402-+498 | Builtin temps | Compiler |

## Related Documentation

- Z-Machine Specification §1.2.3: Global Variables
- `CLAUDE.md`: Project guidelines and critical fixes
- `COMPILER_ARCHITECTURE.md`: Variable allocation strategy
- Bug #12 Fix: codegen.rs lines 5828-5831 (commit Oct 11, 2025)
