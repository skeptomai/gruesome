# ONGOING TASKS - PROJECT STATUS

## 🌍 **LOCALIZATION ARCHITECTURE: LIFT HARDCODED STRINGS TO GAME SOURCE** - **IN PROGRESS** (November 13, 2025)

**STATUS**: **PHASE 1 READY TO IMPLEMENT** 🎯

**OBJECTIVE**: Implement `messages` block system to lift all hardcoded strings (like "I don't understand that") from compiler code to game source level, enabling localization and developer control over all user-facing text.

### **CURRENT STATUS**

**COMPLETED PHASES**:
- ✅ **Phase 0**: Foundation Analysis - Complete localization architecture documented
- ✅ **Phase 2**: Parser Extensions - Messages block parsing implemented
- ✅ **Phase 3**: Semantic Analysis & IR Extensions - Message processing pipeline complete
- ✅ **Phase 4**: Codegen Integration - Basic prompt and unknown_command messages working

**NEXT UP**:
- 🎯 **Phase 1**: AST Extensions - Add `MessagesDecl` to Abstract Syntax Tree
- 📋 **Phase 5**: Builtin Function Message Integration - Extend to all system messages

### **🎯 Phase 1: AST Extensions** - **IMMEDIATE NEXT STEP**

**OBJECTIVE**: Add `messages` block support to Abstract Syntax Tree

**IMPLEMENTATION NEEDED**:
1. **Extend AST Types** (`src/grue_compiler/ast.rs`):
   ```rust
   #[derive(Debug, Clone)]
   pub enum Item {
       Messages(MessagesDecl), // NEW: System messages
       // ... existing items
   }

   #[derive(Debug, Clone)]
   pub struct MessagesDecl {
       pub messages: HashMap<String, String>,
   }

   impl Program {
       pub fn get_messages(&self) -> Option<&MessagesDecl> {
           // Implementation to find messages block
       }
   }
   ```

**SUCCESS CRITERIA**:
- ✅ Compile without errors after AST changes
- ✅ Unit tests for MessagesDecl creation and access
- ✅ Program.get_messages() returns correct Optional<MessagesDecl>

### **📋 Phase 5: Builtin Function Message Integration** - **PENDING**

**OBJECTIVE**: Extend message system to all builtin functions (currently only prompt/unknown_command working)

**IMPLEMENTATION NEEDED**:
1. **Update Builtin Functions** (`src/grue_compiler/codegen_builtins.rs`):
   - `player_can_see()` → use message "cant_see_that"
   - `handle_take()` → use messages "already_have_that", "cant_take_that"
   - `handle_open()` → use messages "cant_open_that", "already_open"
   - Movement functions → use message "cant_go_that_way"

2. **Message Key Standardization**:
   ```grue
   messages {
       // Core system (WORKING)
       prompt: "> ",
       unknown_command: "I don't understand that.",

       // Object interaction (TODO)
       cant_see_that: "You can't see any such thing.",
       already_have_that: "You already have that.",
       cant_take_that: "You can't take that.",

       // Container interaction (TODO)
       cant_open_that: "You can't open that.",
       already_open: "It's already open.",
       already_closed: "It's already closed.",

       // Movement (TODO)
       cant_go_that_way: "You can't go that way.",

       // Inventory (TODO)
       empty_handed: "You are empty-handed.",
       carrying: "You are carrying:",
   }
   ```

---

---

## 🚨 **CRITICAL: COMPILER & INTERPRETER COMPLIANCE WORK** - **ACTIVE DEVELOPMENT** (November 13, 2025)

**STATUS**: **ROOT CAUSE IDENTIFIED, TOLERANCE MECHANISMS MAPPED** 🔍

**ISSUE**: Our compiler generates Z-Machine files that violate the Z-Machine standard, but our interpreter has tolerance mechanisms that mask these violations, causing silent failures instead of proper error reporting.

### **INVESTIGATION FINDINGS**

**Compiler Issue**: **Invalid Packed Address Generation**
- Packed address `0x4a52` unpacks to `0x94a5` (37,957 bytes)
- File size is only 9,156 bytes
- **Violation**: Unpacked addresses exceed file boundaries by ~4x

**Interpreter Issue**: **Non-Standard Tolerance Mechanisms**
- **Silent string truncation** (`src/text.rs:40`): Invalid addresses terminate loops gracefully
- **Abbreviation skipping** (`src/text.rs:91-99`): Bad addresses logged but processing continues
- **No bounds validation** in unpacking functions: Pure math with no file size checks

**Why Standard Tools Fail vs. Our Interpreter**:
- **Standard tools (txd)**: Fail fast on invalid addresses → **CRASH**
- **Our interpreter**: Graceful fallbacks for invalid addresses → **SILENT CONTINUE**

**Detailed Analysis**:
- `docs/COMPILER_COMPLIANCE_WORK.md` - Compliance violations and investigation
- `docs/INTERPRETER_TOLERANCE_ANALYSIS.md` - Tolerance mechanisms analysis

### **DEVELOPMENT BRANCH**: `compiler_interpreter_compliance`

**SCOPE**: Fix both compiler address generation AND interpreter tolerance to ensure Z-Machine specification compliance

### **COMPLIANCE WORK TASK LIST**

**PHASE 1: SETUP & VALIDATION**
1. ✅ **Document compliance violations and tolerance mechanisms**
2. 🔄 **Create compliance work branch: `compiler_interpreter_compliance`**
3. 🔄 **Tighten interpreter bounds checking to panic on invalid addresses**
4. 🔄 **Test tightened interpreter with mini_zork gameplay protocol**

**PHASE 2: COMPILER FIXES**
5. 🔄 **Identify where compiler generates invalid packed address `0x4a52`**
6. 🔄 **Fix compiler packed address calculation to stay within bounds**

**PHASE 3: VERIFICATION**
7. 🔄 **Verify compliance: txd can disassemble our files without errors**
8. 🔄 **Test fixed system with full gameplay protocol**

**APPROACH**: Expose hidden bugs by making interpreter strict, then fix root causes in compiler, then verify full compliance with standard tools.

---

## 🔧 **SYSTEM STATUS**

### **⚠️ CRITICAL BUGS REQUIRING IMMEDIATE ATTENTION** (November 13, 2025)

- 🚨 **Z-Machine Compliance Violations**: Compiler generates invalid packed addresses
- 🚨 **Non-Standard Interpreter**: Tolerates specification violations that crash standard tools

### **✅ FUNCTIONAL SYSTEMS** (November 13, 2025)

- ✅ **Container Iteration Infinite Loop**: Fixed circular sibling references (v2.8.3)
- ✅ **Hash→Index Determinism**: Complete HashMap→IndexMap cleanup applied
- ✅ **Gameplay Functionality**: Mini_zork test protocol passes 100% with our interpreter
- ✅ **Core Z-Machine Operations**: All object, container, and navigation systems functional

### **🚀 CURRENT SYSTEM CAPABILITIES**

**Grue Compiler**: V3 Production Ready
- ✅ Complete Pipeline: Lexer → Parser → Semantic → IR → CodeGen
- ✅ Grammar System: Full verb+noun pattern matching with object resolution
- ✅ Object System: Container operations, property access, state management
- ✅ String System: Automatic StringAddress type detection and print dispatch
- ✅ Navigation System: Exit handling, direction resolution, room transitions
- ✅ Basic Localization: Prompt and unknown_command message customization

**Z-Machine Interpreter**: Complete
- ✅ V3 Games: Fully playable (Zork I, Seastalker, The Lurking Horror)
- ✅ V4+ Games: Fully playable (AMFV, Bureaucracy, Border Zone)
- ✅ Cross-platform: macOS, Windows, Linux binary releases
- ✅ Professional CI/CD: Automated testing and release generation

---

## 📋 **MAINTENANCE NOTES**

**Documentation**:
- Technical architecture: `docs/ARCHITECTURE.md`
- Historical analysis: `docs/` directory
- Active development: This file (ONGOING_TASKS.md)

**Development Principles**:
- No time estimates or completion percentages
- IndexMap/IndexSet for deterministic builds
- All analysis files in `docs/`, never `/tmp`
- Use log::debug!() not eprintln!() for debugging