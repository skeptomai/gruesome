# Comprehensive Zork I Test Protocol Script

## Overview

`test_zork1_comprehensive.sh` is a complete automation script that tests Z-Machine interpreter compatibility with commercial Infocom games using the original Zork I.

## What It Does

### 🧹 **Clean Build Process**
- Runs `cargo clean` to ensure fresh builds
- Removes all previous build artifacts

### 🔨 **Interpreter Compilation**
- Builds debug interpreter: `cargo build --bin gruesome`
- Builds release interpreter: `cargo build --bin gruesome --release`
- Creates both debug and release versions of the gruesome interpreter

### 🎮 **Commercial Game Testing**
- Tests against original Zork I: `resources/test/zork1/DATA/ZORK1.DAT`
- Runs standardized Zork I gameplay protocol
- No game compilation needed - uses commercial Infocom game file

### 🎯 **Comprehensive Compatibility Testing**
Tests both interpreter builds:
1. **Debug interpreter + Zork I**
2. **Release interpreter + Zork I**

### 📊 **Complete Protocol Execution**
Each test runs the full 10-command sequence:
```
1. north (to North of House)
2. east (to Behind House)
3. open window (test complex verb processing)
4. enter (enter Kitchen)
5. take bag (object interaction)
6. take bottle (multiple object handling)
7. leave (exit Kitchen)
8. score (test scoring system)
9. inventory (test inventory display)
10. quit + y (clean exit)
```

### 📋 **Output Capture & Analysis**
- **Raw output** (with ANSI codes): `*_output.txt`
- **Clean output** (ANSI stripped): `*_clean.txt`
- **Test summary** (protocol checklist): `*_summary.txt`
- **Final report** (comprehensive): `ZORK1_TEST_REPORT.md`

## Usage

### Basic Execution
```bash
./scripts/test_zork1_comprehensive.sh
```

### What to Expect
The script takes 1-3 minutes and shows real-time progress:

```
[INFO] Starting Zork I Comprehensive Test Protocol
=== STEP 1: Clean Build ===
[INFO] Running cargo clean...
[SUCCESS] Build artifacts cleaned

=== STEP 2: Build Debug and Release Interpreters ===
[INFO] Building debug interpreter...
[SUCCESS] Debug interpreter build completed
[INFO] Building release interpreter...
[SUCCESS] Release interpreter build completed

=== STEP 3: Verify Zork I Game File ===
[SUCCESS] Zork I game file found: resources/test/zork1/DATA/ZORK1.DAT
[INFO] Game file size: 87616 bytes

=== STEP 4: Run Zork I Gameplay Tests ===
[INFO] Testing both interpreter versions against Zork I...
[INFO] Running debug_interpreter test against Zork I...
[SUCCESS] debug_interpreter completed successfully
[INFO]   Score: 10, Moves: 7, Indicators: 6/6
[INFO] Running release_interpreter test against Zork I...
[SUCCESS] release_interpreter completed successfully
[INFO]   Score: 10, Moves: 7, Indicators: 6/6

=== STEP 5: Generate Final Report ===
[SUCCESS] Final report generated: tests/zork1_results_20251112_184532/ZORK1_TEST_REPORT.md
[SUCCESS] 🎉 ALL TESTS PASSED (2/2)
[SUCCESS] Zork I commercial game compatibility verified!
```

## Output Structure

After completion, find results in `tests/zork1_results_TIMESTAMP/`:

```
tests/zork1_results_20251112_184532/
├── ZORK1_TEST_REPORT.md                 # Main compatibility report
├── debug_interpreter_output.txt         # Raw debug output
├── debug_interpreter_clean.txt          # Clean debug output
├── debug_interpreter_summary.txt        # Debug test summary
├── release_interpreter_output.txt       # Raw release output
├── release_interpreter_clean.txt        # Clean release output
└── release_interpreter_summary.txt      # Release test summary
```

## Success Criteria

The script considers a test **PASSED** if:
- ✅ Final score reaches 10 points (kitchen entry bonus)
- ✅ All 6 protocol indicators found:
  - "North of House" (navigation)
  - "With great effort, you open the window" (complex verb)
  - "Kitchen" (room transition)
  - "Taken" (object interaction)
  - "Your score is" (score system)
  - "You are carrying:" (inventory system)

## Use Cases

### 🔍 **Compatibility Testing**
Verify interpreter works with commercial Infocom games:
```bash
# After interpreter changes
./scripts/test_zork1_comprehensive.sh
# Verify commercial game compatibility maintained
```

### 🚀 **Release Validation**
Ensure release builds handle commercial games correctly:
```bash
# Before releasing interpreter
./scripts/test_zork1_comprehensive.sh
# Check ZORK1_TEST_REPORT.md for compatibility verification
```

### 🐛 **Regression Detection**
Identify which builds break commercial game support:
```bash
./scripts/test_zork1_comprehensive.sh
# Compare debug vs release results
# Check individual *_summary.txt files
```

### 🏗️ **CI Integration**
Add to CI pipeline for automated commercial game testing:
```bash
# In CI script
./scripts/test_zork1_comprehensive.sh
if [ $? -eq 0 ]; then echo "Zork I compatibility verified"; else echo "Commercial game compatibility broken"; exit 1; fi
```

## Key Differences from Mini Zork Script

- **No Compiler Testing**: Only tests interpreter builds, no game compilation
- **Commercial Game**: Tests against original Infocom Zork I, not compiled game
- **Simpler Pipeline**: Build interpreter → test game (vs build compiler → compile game → test)
- **Commercial Standards**: Validates against professional Infocom game standards
- **Different Protocol**: 10 commands focused on core Z-Machine features

## Error Handling

- **Build failures** stop execution immediately
- **Missing game file** stops execution immediately
- **Test timeouts** (120s) are logged but documented
- **Partial success** reported with specific failure details

## Performance

- **Duration**: ~1-3 minutes depending on machine
- **Disk usage**: ~10MB for generated outputs
- **CPU**: Moderate during builds, light during game tests
- **Memory**: Standard Rust compilation requirements

## Game File Requirements

The script expects Zork I at:
```
resources/test/zork1/DATA/ZORK1.DAT
```

This should be the original Infocom Zork I (Revision 88 / Serial 840726) as referenced in the test protocol documentation.

## Maintenance

The script is self-contained and requires no configuration. It automatically:
- Detects project structure
- Creates timestamped results directories
- Handles path resolution
- Manages temporary files
- Validates game file existence

Perfect for verifying Z-Machine interpreter compatibility with commercial Infocom games!