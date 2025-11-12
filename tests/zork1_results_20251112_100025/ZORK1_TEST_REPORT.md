# Zork I Comprehensive Test Protocol Report

**Generated:** Wed Nov 12 10:00:53 PST 2025
**Project:** infocom
**Git Commit:** 56ffc25

## Test Configuration

- **Game File:** ZORK1.DAT
- **Game Size:** 92160 bytes
- **Commands:** 10 commands (north → quit)
- **Expected Score:** 10 points
- **Expected Moves:** 7-8 moves

## Test Results

### udebug interpreter

**Status:** ❌ FAILED
- **Final Score:** UNKNOWN
- **Moves at Score:** UNKNOWN
- **Success Indicators:** 0/6

**Protocol Checklist:**
- Navigation (North of House): ✗
- Window interaction: ✗
- Kitchen entry: ✗
- Object taking: ✗
- Score display: ✗
- Inventory display: ✗

### urelease interpreter

**Status:** ❌ FAILED
- **Final Score:** UNKNOWN
- **Moves at Score:** UNKNOWN
- **Success Indicators:** 0/6

**Protocol Checklist:**
- Navigation (North of House): ✗
- Window interaction: ✗
- Kitchen entry: ✗
- Object taking: ✗
- Score display: ✗
- Inventory display: ✗

## Overall Results

**Tests Passed:** 0/2
**Overall Status:** ❌ SOME TESTS FAILED

⚠️ **COMPATIBILITY ISSUES DETECTED**

Some interpreter versions failed to complete the Zork I protocol successfully.
Review individual test outputs for detailed failure analysis.

## Files Generated

- **Raw Outputs:** \ files with complete game session logs
- **Clean Outputs:** \ files with ANSI codes stripped
- **Test Summaries:** \ files with protocol checklist

All files are located in: \
