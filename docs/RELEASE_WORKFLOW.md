# Release Workflow Documentation

## Overview

This document outlines the proper workflow for creating and publishing releases in the gruesome project.

## Current Release Process (v2.4.0+)

### 1. Development Completion
- Feature development completed on feature branch
- All tests passing and functionality verified
- Code review and quality assurance completed

### 2. Version Preparation
```bash
# Switch to main branch
git checkout main

# Merge feature branch
git merge feature/your-feature-name

# Update version in Cargo.toml
# Edit version = "X.Y.Z" to new version

# Commit version bump
git add Cargo.toml
git commit -m "chore: Bump version to vX.Y.Z for [release-name] release"
```

### 3. Git Tag Creation
```bash
# Create annotated tag with comprehensive notes
git tag -a vX.Y.Z -m "vX.Y.Z: [Release Title]

Features:
- [Key feature 1]
- [Key feature 2]

Technical improvements:
- [Technical detail 1]
- [Technical detail 2]

Verification:
- [Test results]
- [Quality assurance notes]"

# Push to remote
git push origin main
git push origin vX.Y.Z
```

### 4. Automated Asset Building
**GitHub Actions automatically triggers when tag is pushed:**
- Builds release binaries for all platforms (macOS, Windows)
- Creates grue-compiler, gruesome, gruedasm-txd executables
- Packages all assets and creates **DRAFT** GitHub Release

### 5. Release Publishing
```bash
# Update release notes and publish
gh release edit vX.Y.Z \
  --title "vX.Y.Z: [Descriptive Release Title]" \
  --notes "$(cat release-notes.md)" \
  --draft=false

# Verify publication
gh release list
```

## Release States

### Draft State (Initial)
- ✅ Git tag exists
- ✅ All binary assets built by CI/CD
- ❌ **NOT PUBLICLY VISIBLE**
- ❌ Not marked as "Latest"

### Published State (Final)
- ✅ Git tag exists
- ✅ All binary assets available for download
- ✅ **PUBLICLY VISIBLE** on GitHub releases page
- ✅ Marked as "Latest" release
- ✅ Comprehensive release notes visible

## Quality Gates

### Before Tagging
1. **All tests passing**: Unit tests and gameplay verification
2. **Release builds verified**: `cargo build --release` successful
3. **Cross-platform compatibility**: Core functionality tested
4. **Documentation updated**: Release notes prepared

### Before Publishing
1. **Asset verification**: All expected binaries present
2. **Release notes review**: Comprehensive and accurate
3. **Final smoke test**: Download and test release assets

## Asset Verification Commands

```bash
# Test release binaries
./target/release/grue-compiler examples/mini_zork.grue -o test.z3
./target/release/gruesome test.z3
./target/release/gruedasm-txd test.z3

# Check release status
gh release view vX.Y.Z
gh release list
```

## Historical Pattern Analysis

Looking at recent releases:
- **v2.4.0**: StringAddress Type System ✅ Published
- **v2.3.0**: Scoring System and String Parameter Fix ✅ Published
- **v2.2.0**: Array Implementation ✅ Published
- **v2.1.0**: Multiple entries (one draft, one published) ⚠️ Process improvement

## Best Practices

1. **Single source of truth**: One published release per version
2. **Comprehensive testing**: Never publish without verification
3. **Clear release notes**: Include features, technical details, and verification
4. **Asset verification**: Ensure all platforms are represented
5. **Timely publishing**: Don't leave releases in draft state

## Emergency Procedures

### Unpublish Release (if needed)
```bash
# Convert back to draft
gh release edit vX.Y.Z --draft=true
```

### Hotfix Release
```bash
# Create hotfix branch from main
git checkout -b hotfix/vX.Y.Z+1
# Fix, test, merge, tag, publish
```

---

**Last Updated**: November 7, 2025
**Current Version**: v2.4.0 (StringAddress Type System)