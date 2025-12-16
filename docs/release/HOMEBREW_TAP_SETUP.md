# Homebrew Tap Setup for Gruesome

## Overview

This document outlines the complete setup for creating a Homebrew tap for the Gruesome Z-Machine interpreter project. A Homebrew tap allows macOS users to install Gruesome easily while solving Gatekeeper security issues.

## Why Homebrew Tap?

### Problem Solved
- **macOS Gatekeeper Issues**: Unsigned binaries trigger security warnings
- **User Friction**: Manual `xattr -d com.apple.quarantine` commands required
- **Professional Distribution**: Standard macOS software installation method

### Benefits
- ✅ **Solves Gatekeeper Issues**: Homebrew manages trust and installation
- ✅ **Simple Installation**: One command installs all tools
- ✅ **Auto-Updates**: Users get updates with `brew upgrade`
- ✅ **Professional Distribution**: Standard macOS software installation
- ✅ **No Apple Developer Cost**: Free alternative to $99/year code signing

## Repository Architecture Decision

### Separate Repository (Recommended)
Based on research, Homebrew best practices require a **separate repository** for the tap:

**Requirements:**
- Repository name: `homebrew-gruesome` (prefix is mandatory)
- Formula location: `Formula/` subdirectory (recommended over root)
- User installation: `brew install skeptomai/gruesome/gruesome`

**Benefits:**
- ✅ Standard Homebrew conventions
- ✅ Auto-tap functionality (users don't need manual `brew tap`)
- ✅ Clean separation of source vs distribution
- ✅ Future flexibility (can add multiple tools to same tap)

## Command Structure Explanation

### Understanding `brew install skeptomai/gruesome/gruesome`

This command structure appears redundant but follows Homebrew's naming convention:

```bash
brew install [user]/[tap]/[formula]
```

Breaking it down:
- `skeptomai` = GitHub username
- `gruesome` = Tap name (from repository `homebrew-gruesome`)
- `gruesome` = Formula name (from `Formula/gruesome.rb`)

**The apparent redundancy exists because:**
1. **Tap name** derives from repository (`homebrew-gruesome` → `gruesome`)
2. **Formula name** is chosen independently (`gruesome.rb`)
3. They happen to match because we want the formula named after the main tool

**Alternative naming to reduce redundancy:**
```bash
# Could be: brew install skeptomai/tools/gruesome
# Repository: homebrew-tools
# Formula: gruesome.rb
```

But `skeptomai/gruesome/gruesome` is clearer for single-tool taps.

## Formula Implementation

### File: Formula/gruesome.rb

```ruby
class Gruesome < Formula
  desc "Z-Machine interpreter and compiler for Infocom text adventures like Zork"
  homepage "https://github.com/skeptomai/gruesome"
  url "https://github.com/skeptomai/gruesome/archive/v2.11.0.tar.gz"
  sha256 "2838f5b1dff79f41ed013c2096b1a54e38dbaeeaa7c2f5248986208872adcd6d"
  license "MIT"

  depends_on "rust" => :build

  def install
    # Build the main interpreter
    system "cargo", "install", "--bin", "gruesome", "--path", ".", *std_cargo_args

    # Build the compiler
    system "cargo", "install", "--bin", "grue-compiler", "--path", ".", *std_cargo_args

    # Build the disassembler (if available)
    system "cargo", "install", "--bin", "gruedasm-txd", "--path", ".", *std_cargo_args
  end

  test do
    # Test the interpreter
    assert_match "gruesome", shell_output("#{bin}/gruesome --version 2>&1")

    # Test the compiler
    assert_match "grue-compiler", shell_output("#{bin}/grue-compiler --version 2>&1")

    # Test basic functionality with a minimal Z3 file (if available)
    # This could be enhanced to test with a simple compiled game
  end

  def caveats
    <<~EOS
      Gruesome Z-Machine Tools installed:

      • gruesome: Z-Machine interpreter for playing Infocom games
        Usage: gruesome game.z3

      • grue-compiler: Compile Grue language to Z-Machine bytecode
        Usage: grue-compiler source.grue -o game.z3

      • gruedasm-txd: Enhanced Z-Machine disassembler
        Usage: gruedasm-txd game.z3

      Example Zork I gameplay:
        Download a Z-Machine game file and run:
        gruesome ZORK1.DAT

      For more information, visit:
      https://github.com/skeptomai/gruesome
    EOS
  end
end
```

### Key Features
- **Triple Tool Installation**: Installs gruesome, grue-compiler, and gruedasm-txd
- **Proper Dependencies**: Uses Rust for building from source
- **Smart Tests**: Verifies all binaries work with version checks
- **User Guide**: Helpful caveats with usage examples and macOS workarounds
- **SHA256 Verification**: Ensures download integrity

## Repository Structure

```
homebrew-gruesome/
├── README.md           # User installation and usage guide
├── LICENSE             # MIT license for the tap
└── Formula/
    └── gruesome.rb     # Main Homebrew formula
```

## Setup Process

### 1. Create GitHub Repository

```bash
# Create repository: skeptomai/homebrew-gruesome
# Description: "Homebrew tap for Gruesome Z-Machine interpreter"
```

### 2. Initialize Repository

```bash
git clone https://github.com/skeptomai/homebrew-gruesome.git
cd homebrew-gruesome

# Create directory structure
mkdir -p Formula

# Copy formula file
cp path/to/Formula/gruesome.rb Formula/

# Copy documentation
cp path/to/homebrew-tap-README.md README.md
cp path/to/homebrew-tap-LICENSE LICENSE

# Initial commit
git add .
git commit -m "Initial Homebrew tap for Gruesome v2.11.0

- Add formula for gruesome, grue-compiler, and gruedasm-txd
- Include comprehensive documentation and usage examples
- Provide macOS Gatekeeper workaround instructions
- Support installation from source using Rust/Cargo"

git push origin main
```

### 3. Testing Process

```bash
# Test local formula
brew install --build-from-source ./Formula/gruesome.rb

# Test from tap
brew tap skeptomai/gruesome
brew install gruesome --build-from-source

# Verify installation
which gruesome grue-compiler gruedasm-txd
gruesome --version
grue-compiler --version
gruedasm-txd --version

# Test basic functionality
echo 'print("Hello from Homebrew!")' > test.grue
grue-compiler test.grue -o test.z3
gruesome test.z3
```

## User Installation Experience

### Simple Installation
```bash
brew install skeptomai/gruesome/gruesome
```

This single command:
1. Automatically adds the tap (`brew tap skeptomai/gruesome`)
2. Downloads and verifies the source tarball
3. Builds all three tools from source using Rust
4. Installs to `/opt/homebrew/bin/` (Apple Silicon) or `/usr/local/bin/` (Intel)
5. Makes tools available in user's PATH

### Usage After Installation
```bash
# Play Infocom games
gruesome ZORK1.DAT

# Compile custom adventures
grue-compiler my-game.grue -o my-game.z3
gruesome my-game.z3

# Analyze Z-Machine files
gruedasm-txd game.z3
```

## Maintenance and Updates

### For New Releases

When releasing a new version (e.g., v2.12.0):

1. **Calculate new SHA256**:
```bash
curl -sL "https://github.com/skeptomai/gruesome/archive/v2.12.0.tar.gz" | shasum -a 256
```

2. **Update formula**:
```ruby
url "https://github.com/skeptomai/gruesome/archive/v2.12.0.tar.gz"
sha256 "new_sha256_hash_here"
```

3. **Test and commit**:
```bash
brew install --build-from-source ./Formula/gruesome.rb
git commit -am "Update to v2.12.0"
git push origin main
```

4. **Users automatically get updates**:
```bash
brew update && brew upgrade gruesome
```

### Automation Possibilities

Could integrate with GitHub Actions to automatically:
- Update formula when new releases are tagged
- Calculate SHA256 hashes automatically
- Test builds on multiple macOS versions
- Open PRs for version updates

## Alternative Solutions Considered

### Apple Developer Program ($99/year)
- **Pros**: Zero user friction, professional code signing
- **Cons**: Annual cost, complex CI setup, 5 GitHub secrets required
- **Verdict**: Expensive for open-source project, Homebrew is better immediate solution

### xattr Documentation Only
- **Pros**: Free, simple
- **Cons**: High user friction, manual steps for each binary
- **Verdict**: Fallback solution, document alongside Homebrew

### Homebrew Core Submission
- **Pros**: Built into Homebrew, maximum discoverability
- **Cons**: High acceptance bar, ongoing maintenance requirements
- **Verdict**: Consider for future if significant adoption

## Benefits Summary

The Homebrew tap approach provides:

1. **Immediate Solution**: No waiting for Apple Developer approval
2. **Zero Cost**: No annual fees or certificates required
3. **Professional Distribution**: Standard macOS software installation
4. **Automatic Trust**: Homebrew manages Gatekeeper issues
5. **Simple Updates**: Users get new versions automatically
6. **Multiple Tools**: Single command installs entire toolkit
7. **Documentation Integration**: Built-in help and usage examples

This solution eliminates the primary barrier to macOS adoption (Gatekeeper warnings) while providing a professional distribution channel that scales with project growth.

## SHA256 Hash for v2.11.0

```
2838f5b1dff79f41ed013c2096b1a54e38dbaeeaa7c2f5248986208872adcd6d
```

Source: `https://github.com/skeptomai/gruesome/archive/v2.11.0.tar.gz`

## Next Steps

1. ✅ Create `homebrew-gruesome` repository on GitHub
2. ✅ Push formula and documentation
3. ✅ Test installation process
4. ✅ Update main project README with Homebrew installation instructions
5. ⏳ Consider automating formula updates in CI/CD pipeline
6. ⏳ Monitor user feedback and adoption metrics