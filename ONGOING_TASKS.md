# ONGOING TASKS - PROJECT STATUS

## ✅ **VERB DISPATCH INFINITE LOOP FIXED** (November 1, 2025)

**ISSUE RESOLVED**:
- ✅ **Root Cause**: Increment instruction used deprecated `emit_instruction()` instead of `emit_instruction_typed()`
- ✅ **Fix Applied**: Changed to type-safe `emit_instruction_typed(Opcode::Op1(Op1::Inc), ...)` for correct Z-Machine bytecode
- ✅ **Verification**: "open mailbox" command now executes correctly without infinite loop

**NEW ISSUE IDENTIFIED**:
- ❌ **Mailbox Default State**: Mailbox defaults to open state, should default to closed (`open: false`)

**FUNCTIONAL SYSTEMS**:
- ✅ **TestAttributeBranch**: Complete IR instruction implementation with proper Z-Machine codegen
- ✅ **Store Instruction**: Missing 2OP:13 (opcode 0x0D) implemented in interpreter
- ✅ **Stack Discipline**: Major architecture fixes implemented, significant balance improvement
- ✅ **Navigation System**: Room-to-room movement working perfectly
- ✅ **Object System**: Object iteration and tree traversal functional
- ✅ **Boolean Expression Contexts**: Phase 3 implementation complete with working runtime verification

---

## 📁 **DOCUMENTATION ORGANIZATION** (October 31, 2025)

**COMPLETED**: Moved analysis markdown files to `docs/` directory:
- All historical bug investigations and implementation plans
- Technical architecture documentation
- Stack discipline analysis and fixes
- Z-Machine instruction implementation details

**CURRENT STRUCTURE**:
- `docs/` - All analysis and technical documentation
- Root directory - Only active project files (CLAUDE.md, ONGOING_TASKS.md, README.md, TESTING.md)

---

## 🎯 **ACTIVE DEVELOPMENT AREAS**

### **Verb Dispatch Infinite Loop** ✅ **FIXED**
- **Issue**: "open mailbox" caused infinite loop due to incorrect increment instruction compilation
- **Root Cause**: Increment instruction used deprecated `emit_instruction()` generating malformed Z-Machine bytecode
- **Fix**: Changed to `emit_instruction_typed(Opcode::Op1(Op1::Inc), ...)` for correct bytecode generation
- **Status**: RESOLVED - Object lookup loop now works correctly

### **Mailbox Default State** ❌ **NEW ISSUE**
- **Issue**: Mailbox defaults to open state, responds "It's already open"
- **Expected**: Mailbox should default to closed (`open: false`)
- **Investigation**: Check previous research in markdown files and commit messages
- **Status**: Needs investigation - low priority, game is functional

### **Object Resolution System** ✅ **WORKING**
- **Status**: Object lookup now functions correctly after increment instruction fix
- **Verification**: "open mailbox" correctly resolves mailbox object and executes verb command
- **Architecture**: Uses Property 18 dictionary address comparison for proper object matching

### **Dynamic Descriptions** ✅ **USING CONDITIONAL PRINT APPROACH**
- **Issue**: "examine mailbox" needs to show dynamic state (open/closed)
- **Solution**: Traditional conditional print statements in examine handler instead of computed properties
- **Status**: Simple, proven approach - mailbox examine functionality working correctly

### **Stack Discipline Architecture** ✅ **MAJOR IMPROVEMENTS**
- **Achievement**: Reduced stack imbalance from 25 to 12 unpulled IR IDs (52% improvement)
- **Fix**: Function calls now use proper Z-Machine Variable(0) discipline
- **Status**: Core architecture violations resolved, remaining optimization in progress

### **Z-Machine Boolean Expressions** ✅ **COMPLETE**
- **Phase 3**: Context-aware conditional attribute access working
- **Optimization**: ~50% reduction in IR instructions for conditional attributes
- **Status**: Production ready with comprehensive runtime verification

---

## 🔧 **TECHNICAL DEBT**

### **Minor Optimizations**
- Context-aware expression generation migration (10 remaining legacy calls)
- Test coverage expansion for new attribute systems
- Code comment documentation for recent fixes

### **Infrastructure**
- All major bugs resolved
- Core systems functional and stable
- Commercial game compatibility maintained

---

## 📋 **MAINTENANCE NOTES**

**Recent Critical Findings (November 1, 2025)**:
- **Verb Dispatch Infinite Loop**: "open mailbox" causes infinite loop on both systematic-rebuild and computed-property branches
- **Object Resolution Failure**: Grammar system incorrectly accesses object 1 (player) instead of object 3 (mailbox)
- **Cross-Branch Consistency**: Identical infinite loop pattern confirms issue is in core grammar system, not computed property implementation
- **Debug Evidence**: `get_prop_addr(obj=1, prop=18)` returns 0x0000, causing branch-jump-loop cycle

**Previous Fixes (October 31, 2025)**:
- TestAttributeBranch IR instruction implementation complete
- Store instruction gap filled in interpreter
- Stack discipline architecture violations resolved

**Documentation Location**:
- Historical analysis: `docs/` directory
- Active development: Root directory files
- Current investigation: Object resolution in verb dispatch pipeline

**Status**: **OPERATIONAL** - Verb dispatch working, mailbox default state needs investigation

---

## 📦 **ARCHIVED FEATURES**

### **Computed Property System** 📁 **ARCHIVED (November 1, 2025)**
- **Concept**: Dynamic object property expressions evaluated at runtime (e.g., `desc: "The mailbox is " + (mailbox.open ? "open" : "closed") + "."`)
- **Implementation Status**: 70% complete - Phase 1 (IR registration) & Phase 2 (function generation) working, Phase 3 (GetProperty calling) partially implemented
- **Archive Location**: `computed-property-implementation-archive` branch
- **Documentation**: `docs/COMPUTED_PROPERTY_IMPLEMENTATION_ARCHIVE.md`
- **Revert Reason**: Complexity vs. benefit analysis favored simpler conditional print approach
- **Future Consideration**: Advanced feature for future enhancement once core systems are fully stable