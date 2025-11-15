# Mini Zork Disassembler Test Protocol Report

**Generated:** Fri Nov 14 16:41:21 PST 2025
**Project:** infocom
**Git Commit:** 9e55bbd

## Test Configuration

- **Source Game:** examples/mini_zork.grue
- **Debug Game:** mini_zork_debug_disasm_20251114_164042.z3
- **Release Game:** mini_zork_release_disasm_20251114_164042.z3
- **Disassembler:** gruedasm-txd (debug and release builds)
- **Expected Content:** Functions, opcodes, strings, objects

## Test Results

### udebug disasm debug game

**Status:** ❌ FAILED
- **Output Lines:** 15
- **Functions:** 0
- **Opcodes:** 0
- **Strings:** 0
- **Success Indicators:** 0/5

**Content Checklist:**
- Main function found: ✗
- Print instructions: ✗
- Player object: ✗
- Sufficient functions: ✗
- Sufficient opcodes: ✗
- No critical errors: ✗

### udebug disasm release game

**Status:** ❌ FAILED
- **Output Lines:** 15
- **Functions:** 0
- **Opcodes:** 0
- **Strings:** 0
- **Success Indicators:** 0/5

**Content Checklist:**
- Main function found: ✗
- Print instructions: ✗
- Player object: ✗
- Sufficient functions: ✗
- Sufficient opcodes: ✗
- No critical errors: ✗

### urelease disasm debug game

**Status:** ❌ FAILED
- **Output Lines:** 15
- **Functions:** 0
- **Opcodes:** 0
- **Strings:** 0
- **Success Indicators:** 0/5

**Content Checklist:**
- Main function found: ✗
- Print instructions: ✗
- Player object: ✗
- Sufficient functions: ✗
- Sufficient opcodes: ✗
- No critical errors: ✗

### urelease disasm release game

**Status:** ❌ FAILED
- **Output Lines:** 15
- **Functions:** 0
- **Opcodes:** 0
- **Strings:** 0
- **Success Indicators:** 0/5

**Content Checklist:**
- Main function found: ✗
- Print instructions: ✗
- Player object: ✗
- Sufficient functions: ✗
- Sufficient opcodes: ✗
- No critical errors: ✗

## Overall Results

**Tests Passed:** 0/4
**Overall Status:** ❌ SOME TESTS FAILED

⚠️ **DISASSEMBLER ISSUES DETECTED**

Some build combinations failed to produce expected disassembly output.
Review individual test outputs and error files for details.

## Files Generated

- **Disassembly Outputs:** `*_output.txt` files with complete disassembly
- **Error Logs:** `*_errors.txt` files with stderr capture
- **Test Summaries:** `*_summary.txt` files with metrics and verification
- **Game Files:** Debug and release compiled game files

All files are located in: `/Users/cb/Projects/infocom-testing-old/infocom/tests/disasm_mini_zork_results_20251114_164042`
