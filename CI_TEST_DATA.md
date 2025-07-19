# CI Test Data Requirements

## Issue
The GitHub Actions CI tests are failing because they require the Zork I game data file at:
```
resources/test/zork1/DATA/ZORK1.DAT
```

This file is not included in the repository (likely for copyright reasons) but is needed for tests to run successfully.

## Current Impact
- Tests fail on all platforms (Ubuntu, macOS, Windows) in CI
- Binary builds still succeed
- Releases can still be created, but with failing test status

## Potential Solutions

### Option 1: Mock Test Data
Create a minimal mock Z-Machine file that includes just enough structure for tests to run without using actual game data.

### Option 2: Conditional Tests
Modify tests to skip when the game file is not present, with appropriate warnings.

### Option 3: Test-Only Game
Use a freely distributable Z-Machine game file for testing (e.g., a public domain or open-source interactive fiction game).

### Option 4: Encrypted Test Data
Store an encrypted version of the test data that can be decrypted in CI using a secret.

## Recommendation
Option 3 is likely the best approach - using a freely available game like:
- "Advent" (Colossal Cave Adventure) - public domain
- One of the sample games from Inform 7
- A minimal test game created specifically for testing

This would allow full test coverage without copyright concerns.

## Implementation Notes
When implementing, update:
- `.github/workflows/ci.yml` to download or include test game
- `.github/workflows/build-release.yml` test jobs
- Test files to use the freely available game data
- Documentation to explain the test data approach