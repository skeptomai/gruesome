# Session Context - July 19, 2025

## Current Status
- Successfully created GitHub Actions workflows for multi-platform builds
- Fixed all compilation errors in test binaries (src/bin/*.rs)
- Created v0.0.3 draft release with all binaries and README.txt
- All tests pass in CI (ZORK1.DAT is in the repository)

## Recent Work Completed
1. **GitHub Actions Workflows**
   - `.github/workflows/ci.yml` - Continuous integration
   - `.github/workflows/build-release.yml` - Release builds with draft creation
   - Builds for: macOS (Intel, Apple Silicon, Universal), Windows (x86_64)

2. **Documentation Updates**
   - Added Windows/macOS security instructions to README.md
   - Created RUNTIME_README.md for end users (included as README.txt in releases)
   - Created CI_TEST_DATA.md (can be deleted - tests work fine)

3. **Test Binary Fixes**
   - Fixed all src/bin/*.rs files to compile with current API
   - Main issues were: decode_string returns tuple, CallFrame field changes, Instruction.size

## Pending Tasks
1. **Add release notes to v0.0.3** before publishing (currently just has changelog link)
2. **Verify lantern timer** decrements properly in Zork I
3. **Test real-time games** like Border Zone (v5) when timer support is needed
4. **Consider Windows code signing** for future releases

## Key Technical Details
- Timer implementation: Turn-based for v3, real-time infrastructure ready for v4+
- Non-blocking I/O using crossterm's event system (OS-level, not polling)
- Display management with status line support for all v3 games
- Save/restore using Quetzal format with compression

## Working Games
- Zork I, Deadline, Enchanter, Hitchhiker's Guide, Suspended (all v3)
- Basic v4+ support exists (SREAD with timers, read_char)

## Release Process
1. Tag: `git tag v0.0.X && git push origin v0.0.X`
2. Workflow automatically creates draft release with binaries
3. Edit release notes on GitHub and publish

## Notes
- Windows binaries need unblocking due to SmartScreen
- macOS binaries may need Gatekeeper approval
- The "claude-start" branch exists but main development is on "main"