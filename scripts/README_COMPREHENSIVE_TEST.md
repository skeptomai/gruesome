# Comprehensive Mini Zork Test Protocol Script

## Overview

`test_mini_zork_comprehensive.sh` is a complete automation script that implements the full Mini Zork gameplay test protocol with comprehensive build verification.

## What It Does

### 🧹 **Clean Build Process**
- Runs `cargo clean` to ensure fresh builds
- Removes all previous build artifacts

### 🔨 **Multi-Build Compilation**
- Builds debug version: `cargo build`
- Builds release version: `cargo build --release`
- Creates both debug and release compilers and interpreters

### 🎮 **Game Compilation**
- Compiles `examples/mini_zork.grue` with debug compiler → `mini_zork_debug_TIMESTAMP.z3`
- Compiles `examples/mini_zork.grue` with release compiler → `mini_zork_release_TIMESTAMP.z3`
- Timestamped filenames prevent conflicts

### 🎯 **Comprehensive Gameplay Testing**
Tests all 4 combinations:
1. **Debug interpreter + Debug game**
2. **Debug interpreter + Release game**
3. **Release interpreter + Debug game**
4. **Release interpreter + Release game**

### 📊 **Complete Protocol Execution**
Each test runs the full 11-command sequence:
```
1. open mailbox
2. take leaflet
3. read leaflet
4. north (first)
5. north (second)
6. climb tree
7. take egg
8. down
9. score
10. inventory
11. quit + y
```

### 📋 **Output Capture & Analysis**
- **Raw output** (with ANSI codes): `*_output.txt`
- **Clean output** (ANSI stripped): `*_clean.txt`
- **Test summary** (metrics & status): `*_summary.txt`
- **Final report** (comprehensive): `COMPREHENSIVE_TEST_REPORT.md`

## Usage

### Basic Execution
```bash
./scripts/test_mini_zork_comprehensive.sh
```

### What to Expect
The script takes 2-5 minutes and shows real-time progress:

```
[INFO] Starting Comprehensive Mini Zork Test Protocol
=== STEP 1: Clean Build ===
[INFO] Running cargo clean...
[SUCCESS] Build artifacts cleaned

=== STEP 2: Build Debug and Release Versions ===
[INFO] Building debug version...
[SUCCESS] Debug build completed
[INFO] Building release version...
[SUCCESS] Release build completed

=== STEP 3: Compile Game Files ===
[INFO] Compiling game with debug compiler...
[SUCCESS] Debug game compiled: mini_zork_debug_20251112_173045.z3
[INFO] Compiling game with release compiler...
[SUCCESS] Release game compiled: mini_zork_release_20251112_173045.z3

=== STEP 4: Run Comprehensive Gameplay Tests ===
[INFO] Testing all interpreter/game combinations...
[INFO] Running debug_interpreter_debug_game test...
[SUCCESS] debug_interpreter_debug_game completed successfully
[INFO]   Final Score: 7, Moves: 4, Indicators: 4/4
[INFO] Running debug_interpreter_release_game test...
[SUCCESS] debug_interpreter_release_game completed successfully
[INFO]   Final Score: 7, Moves: 4, Indicators: 4/4
[INFO] Running release_interpreter_debug_game test...
[SUCCESS] release_interpreter_debug_game completed successfully
[INFO]   Final Score: 7, Moves: 4, Indicators: 4/4
[INFO] Running release_interpreter_release_game test...
[SUCCESS] release_interpreter_release_game completed successfully
[INFO]   Final Score: 7, Moves: 4, Indicators: 4/4

=== STEP 5: Generate Final Report ===
[SUCCESS] Final report generated: tests/protocol_results_20251112_173045/COMPREHENSIVE_TEST_REPORT.md
[SUCCESS] 🎉 ALL TESTS PASSED (4/4)
[SUCCESS] Comprehensive test protocol completed!
```

## Output Structure

After completion, find results in `tests/protocol_results_TIMESTAMP/`:

```
tests/protocol_results_20251112_173045/
├── COMPREHENSIVE_TEST_REPORT.md          # Main report
├── debug_game_path.txt                   # Debug game file path
├── release_game_path.txt                 # Release game file path
├── debug_interpreter_debug_game_output.txt      # Raw output
├── debug_interpreter_debug_game_clean.txt       # Clean output
├── debug_interpreter_debug_game_summary.txt     # Test summary
├── debug_interpreter_release_game_output.txt    # Raw output
├── debug_interpreter_release_game_clean.txt     # Clean output
├── debug_interpreter_release_game_summary.txt   # Test summary
├── release_interpreter_debug_game_output.txt    # Raw output
├── release_interpreter_debug_game_clean.txt     # Clean output
├── release_interpreter_debug_game_summary.txt   # Test summary
├── release_interpreter_release_game_output.txt  # Raw output
├── release_interpreter_release_game_clean.txt   # Clean output
└── release_interpreter_release_game_summary.txt # Test summary
```

## Success Criteria

The script considers a test **PASSED** if:
- ✅ Final score reaches 7
- ✅ All 4 success indicators found:
  - "Took leaflet" (object interaction)
  - "You are the proud owner of a very special egg" (tree climbing + egg)
  - "Your score is" (score display)
  - "You are carrying:" (inventory display)

## Use Cases

### 🔍 **Regression Testing**
Run after code changes to ensure no functionality breaks:
```bash
# After making changes
git add -A
git commit -m "your changes"
./scripts/test_mini_zork_comprehensive.sh
# Verify all tests pass before pushing
```

### 🚀 **Release Validation**
Verify release builds work correctly:
```bash
# Before creating release
./scripts/test_mini_zork_comprehensive.sh
# Check COMPREHENSIVE_TEST_REPORT.md for all ✅
```

### 🐛 **Bug Investigation**
Identify which build combinations are affected:
```bash
./scripts/test_mini_zork_comprehensive.sh
# Check individual *_summary.txt files
# Compare *_clean.txt outputs
```

### 🏗️ **CI Integration**
Add to CI pipeline for automated testing:
```bash
# In CI script
./scripts/test_mini_zork_comprehensive.sh
if [ $? -eq 0 ]; then echo "All tests passed"; else echo "Tests failed"; exit 1; fi
```

## Error Handling

- **Build failures** stop execution immediately
- **Game compilation failures** stop execution immediately
- **Test timeouts** (60s) are logged but don't stop other tests
- **Partial success** is reported with specific failure details

## Performance

- **Duration**: ~2-5 minutes depending on machine
- **Disk usage**: ~50MB for all generated files
- **CPU**: Moderate during builds, light during game tests
- **Memory**: Standard Rust compilation requirements

## Maintenance

The script is self-contained and requires no configuration. It automatically:
- Detects project structure
- Creates timestamped results directories
- Handles path resolution
- Manages temporary files

Perfect for both development workflow integration and CI/CD automation!