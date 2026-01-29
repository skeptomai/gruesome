# Nix: Introduction and Guide for Emacs Users

## What is Nix?

**Nix is a package manager that provides:**

1. **Reproducible environments**: Exact same packages every time
2. **Complete isolation**: Each project has its own dependencies
3. **No global pollution**: Nothing installed globally
4. **Atomic operations**: Either fully works or fully rolls back
5. **Declarative**: Describe what you want, Nix handles how

**Think of it as**:
- **Like**: npm/bundler/poetry for system packages
- **Unlike Docker**: No containers, runs natively on macOS
- **Magic**: Environment changes automatically per directory

---

## The TL;DR

**What is Nix?**

Think of it as "npm for your entire system" - but better:

- **Declarative**: You write what you want (Node 20, AWS CLI, Rust), Nix figures out how to get it
- **Isolated**: Each project has its own environment, nothing pollutes global system
- **Reproducible**: Same exact setup every time, on every machine
- **Atomic**: Either fully works or fully rolls back - no half-broken states

**The Magic for Emacs:**

```
You open a file in infrastructure/ directory
→ Environment automatically activates
→ All tools (node, cdk, cargo) instantly available
→ Close file or switch to different project
→ Environment automatically deactivates
```

**No containers, no "entering" anything - it just works!**

---

## Real Example: Without vs With Nix

### Without Nix (Painful)

```bash
# Project 1 needs Node 20
nvm use 20
cd project1
npm install -g aws-cdk

# Project 2 needs Node 16
cd ../project2
nvm use 16
npm install -g some-old-tool

# Go back to project 1
cd ../project1
nvm use 20  # Forgot? Breaks!
```

**Problems**:
- Manual environment switching
- Version conflicts
- Global installations pollute system
- Easy to forget which version you need
- Hard to reproduce on team member's machine

### With Nix (Seamless)

**Project 1**: `infrastructure/flake.nix`
```nix
buildInputs = with pkgs; [ nodejs_20 awscli2 cargo-lambda ];
```

**Project 2**: `old-app/flake.nix`
```nix
buildInputs = with pkgs; [ nodejs_16 python310 ];
```

**Usage in Emacs**:
```
Open infrastructure/file.ts → Node 20 automatically available
Open old-app/server.js     → Node 16 automatically available
Switch between files        → Environment switches automatically
```

**No manual switching, no thinking, just works!**

---

## Why Nix is Perfect for Emacs Users

You're an Emacs user who values:

1. **Clean system**: Nix keeps everything in `/nix`, nothing pollutes your macOS
2. **Automatic tooling**: direnv + Emacs = environment switches as you navigate files
3. **Project isolation**: Different projects, different Node versions - zero conflict
4. **Reproducibility**: Team member clones repo → exact same environment
5. **Native speed**: No VM, no containers - runs directly on macOS
6. **Seamless integration**: All Emacs commands use Nix environment automatically

### The Killer Feature for Emacs

**Every Emacs command uses the environment automatically**:

- `M-x compile` → Uses Nix tools
- `M-x shell` / `M-x vterm` → Nix environment active
- `M-x projectile-run-shell-command-in-root` → Nix tools available
- **LSP servers** → Use Nix-provided language tools
- **Magit hooks** → Run in Nix environment
- **Flycheck/Flymake** → Use Nix-provided linters

**No configuration, no thinking - it just works!**

This is why Nix + direnv + Emacs is considered one of the best development setups.

---

## How Nix Works (Simple Explanation)

### Traditional Package Management (Homebrew, apt, etc.)

```
/usr/local/bin/node → version 20
Install different version → overwrites existing
Multiple projects → all share same global version
Upgrade → might break existing projects
```

**Problems**: Global state, version conflicts, hard to reproduce

### Nix Package Management

```
/nix/store/abc123-nodejs-20.1.0/bin/node
/nix/store/def456-nodejs-16.3.2/bin/node
/nix/store/ghi789-awscli-2.15.0/bin/aws
```

**Each package version stored separately**

```
Project 1 → PATH includes abc123-nodejs-20.1.0
Project 2 → PATH includes def456-nodejs-16.3.2
```

**Benefits**:
- No conflicts (different paths)
- Atomic rollback (just change PATH)
- Perfect reproducibility (hash-based storage)
- Garbage collection (remove unused versions)

---

## Nix Concepts

### 1. Nix Store

**Location**: `/nix/store/`

**Contents**: All packages, stored by hash
```
/nix/store/
├── abc123-nodejs-20.1.0/
├── def456-awscli-2.15.0/
├── ghi789-cargo-lambda-1.0.0/
└── ...
```

**Key insight**: Identical package = identical hash = shared storage

### 2. Nix Expressions

**What**: Declarative configuration files (`.nix` files)

**Example**: `flake.nix`
```nix
{
  description = "My project environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs = { self, nixpkgs, ... }: {
    devShells.default = pkgs.mkShell {
      buildInputs = [ pkgs.nodejs_20 pkgs.awscli2 ];
    };
  };
}
```

**Translation**: "I want Node.js 20 and AWS CLI 2 from the unstable nixpkgs repository"

### 3. Flakes (Modern Nix)

**What**: Standardized way to define Nix projects

**Benefits**:
- Lock file (`flake.lock`) pins exact versions
- Standard structure across all projects
- Easy to share and reproduce
- Better caching

**Traditional Nix**: Many ways to do things, inconsistent
**Flakes**: One standard way, reproducible by default

### 4. Development Shells

**What**: Temporary environments with specific packages

**Without Nix**:
```bash
# Install globally (permanent)
brew install node
npm install -g aws-cdk
```

**With Nix**:
```bash
# Enter temporary shell (disappears when you exit)
nix develop
# node, cdk available
exit
# node, cdk gone (unless installed globally elsewhere)
```

**Key**: Nothing permanent unless you want it to be

---

## The Setup (20 Minutes)

### 1. Install Nix (5 minutes)

```bash
# Install Nix package manager
sh <(curl -L https://nixos.org/nix/install) --daemon

# This installs to /nix (its own directory, isolated)
# Adds ~2-3GB to your system

# Enable experimental features (flakes = modern Nix)
mkdir -p ~/.config/nix
echo "experimental-features = nix-command flakes" >> ~/.config/nix/nix.conf

# Verify
nix --version  # Should show 2.x.x
```

**What just happened?**
- Nix installed to `/nix` directory
- Added Nix to your shell PATH (in `/etc/bashrc` or `/etc/zshrc`)
- Nothing else on your system touched
- You can uninstall cleanly: `sudo rm -rf /nix`

### 2. Install direnv (2 minutes)

```bash
# Install direnv (shell extension)
brew install direnv

# Add to your shell config (~/.zshrc or ~/.bashrc)
eval "$(direnv hook zsh)"  # or bash

# Reload shell
source ~/.zshrc
```

**What is direnv?**
- Automatically loads/unloads environments per directory
- Works with `.envrc` files
- Emacs integration available

### 3. Configure Doom Emacs (2 minutes)

**Step 1**: Add direnv package

Edit `~/.doom.d/packages.el`:
```elisp
(package! direnv)
```

**Step 2**: Configure direnv

Edit `~/.doom.d/config.el`:
```elisp
(use-package! direnv
  :config
  (direnv-mode))
```

**Step 3**: Reload Doom
```bash
doom sync
doom reload  # Or restart Emacs
```

### 4. Create Project Environment (5 minutes)

**Create `flake.nix`**:

```nix
{
  description = "Gruesome CDK development environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
      in
      {
        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            nodejs_20
            nodePackages.aws-cdk
            awscli2
            cargo
            rustc
            cargo-lambda
          ];

          shellHook = ''
            echo "Gruesome CDK environment ready!"
            node --version
            cdk --version
          '';
        };
      }
    );
}
```

**Create `.envrc`**:
```bash
echo "use flake" > .envrc
```

**Allow direnv**:
```bash
direnv allow
```

### 5. First Build (5 minutes)

```bash
# First time: Nix downloads and builds everything
nix develop

# Wait ~5 minutes for downloads

# Subsequent times: Instant (cached)
```

**Done!** Environment is ready.

---

## Using Nix with Emacs

### Automatic Activation

1. Open ANY file in directory with `.envrc`
2. Emacs status bar shows: `direnv: loading .envrc`
3. Environment is NOW active in all Emacs commands!

**Example**:

```elisp
M-x vterm  ; or M-x shell
```

In the shell:
```bash
node --version    # Works! From Nix
cdk --version     # Works! From Nix
cargo --version   # Works! From Nix
```

Close file, open file outside directory:
```elisp
M-x vterm
```

```bash
node --version    # Not found! (unless installed globally)
```

**The environment is automatic per-directory!**

### Emacs Commands That Work

All standard Emacs commands use the Nix environment:

**Compilation**:
```elisp
M-x compile
Command: cdk synth
```
Uses Nix-provided `cdk`!

**Projectile**:
```elisp
M-x projectile-run-shell-command-in-root
Command: cargo lambda build
```
Uses Nix-provided `cargo-lambda`!

**LSP**:
- Open TypeScript file
- LSP server uses Nix-provided Node.js
- Auto-completion, type checking work

**Magit**:
- Git hooks run in Nix environment
- Pre-commit scripts have access to Nix tools

---

## Common Workflows

### Update Packages

```bash
cd infrastructure
nix flake update  # Updates to latest versions
```

This updates `flake.lock` with new package versions.

### Enter Environment Manually

```bash
cd infrastructure
nix develop  # Activates environment in current shell
```

Useful for terminal work outside Emacs.

### Clean Cache

```bash
nix-collect-garbage -d  # Removes old versions
```

Frees disk space by removing unused package versions.

### Add a New Tool

Edit `flake.nix`:
```nix
buildInputs = with pkgs; [
  nodejs_20
  awscli2
  cargo
  cargo-lambda
  jq            # Add this!
  ripgrep       # Or this!
];
```

Then:
```bash
direnv reload  # Or just reopen file in Emacs
```

Now `jq` and `rg` are available!

### Search Available Packages

```bash
# Search for a package
nix search nixpkgs python

# Show package details
nix search nixpkgs python3 --json
```

### Pin Specific Package Version

```nix
# Instead of pkgs.nodejs_20
# Use specific version from nixpkgs
buildInputs = [
  (pkgs.nodejs.overrideAttrs (old: {
    version = "20.11.0";
  }))
];
```

---

## Nix vs Other Solutions

### vs Homebrew

**Homebrew**:
- ✅ Simple, familiar
- ❌ Global installations
- ❌ Version conflicts between projects
- ❌ Not reproducible (versions change over time)

**Nix**:
- ✅ Per-project isolation
- ✅ No version conflicts
- ✅ Perfectly reproducible
- ❌ Steeper learning curve

### vs Docker

**Docker**:
- ✅ Strong isolation
- ✅ Familiar to many developers
- ❌ VM overhead (slower)
- ❌ Need to "enter" container
- ❌ Poor Emacs integration

**Nix**:
- ✅ Native speed
- ✅ Automatic activation
- ✅ Perfect Emacs integration
- ❌ Less isolation than containers

### vs asdf/nvm/pyenv

**Version managers (asdf, nvm, pyenv)**:
- ✅ Language-specific, simple
- ❌ Manual switching (.nvmrc files, shell hooks)
- ❌ Only manage language runtimes
- ❌ Can't manage system tools (AWS CLI, etc.)

**Nix**:
- ✅ Manages everything (languages + tools)
- ✅ Automatic switching
- ✅ More comprehensive
- ❌ More complex

---

## Common Questions

### Q: Is it hard to learn?

**A**: Basic use is simple - just list packages in `flake.nix`. Advanced use has learning curve, but not needed to start.

**What you need to know**:
- Copy example `flake.nix`
- Add packages to `buildInputs` list
- Run `direnv allow`

**What you don't need to know** (to start):
- Nix language syntax
- Package derivations
- Advanced nixpkgs usage

### Q: How much disk space?

**A**: ~2-3GB for the Nix store initially, ~500MB per project environment. Shared across projects (deduplication).

**Example**:
- Project A uses Node 20: +500MB
- Project B uses Node 20: +0MB (reuses existing)
- Project C uses Node 16: +300MB (different version)

### Q: Can I uninstall it?

**A**: Yes, completely clean removal:

```bash
# Remove Nix
sudo rm -rf /nix

# Remove from shell config
# Edit /etc/bashrc or /etc/zshrc
# Remove lines added by Nix installer

# Remove user data
rm -rf ~/.nix-profile ~/.nix-defexpr ~/.nix-channels
```

### Q: Does it slow down Emacs?

**A**: No - direnv activation is instant (milliseconds) after first setup.

First time per directory: ~5 seconds (loading environment)
Subsequent times: <100ms (cached)

### Q: What if I break something?

**A**: Nix is atomic - either works or doesn't. Can't break your system.

**Rollback**:
```bash
# If update breaks something
nix flake lock --update-input nixpkgs --commit-lock-file

# Revert commit
git revert HEAD

# Environment rolled back!
```

### Q: Does it work on Apple Silicon?

**A**: Yes! Nix fully supports Apple Silicon (M1/M2/M3) Macs.

### Q: Can I use Nix alongside Homebrew?

**A**: Yes! They don't conflict. Use Homebrew for global tools, Nix for project-specific environments.

### Q: What about team members who don't use Nix?

**A**: They can still use the project:
- They install tools globally (brew, npm install -g, etc.)
- Your `flake.nix` documents what versions are needed
- When they're ready, they can adopt Nix

**Best practice**: Document both approaches in README

---

## Troubleshooting

### Issue: "experimental-features" error

**Error**: `error: experimental Nix feature 'nix-command' is disabled`

**Solution**: Enable flakes
```bash
mkdir -p ~/.config/nix
echo "experimental-features = nix-command flakes" >> ~/.config/nix/nix.conf
```

### Issue: direnv not loading in Emacs

**Symptoms**: Environment not activating when opening files

**Solutions**:
1. Check direnv package installed: `M-x doom/reload`
2. Verify direnv-mode enabled: `M-x direnv-mode`
3. Check `.envrc` file exists: `ls -la .envrc`
4. Manually trigger: `M-x direnv-allow`

### Issue: Slow first-time setup

**Symptoms**: `nix develop` takes 5-10 minutes

**Solution**: Normal - Nix downloads and builds packages. Cached after that.

**Tips**:
- Use binary cache (enabled by default)
- First build per project is slow
- Subsequent builds instant

### Issue: Package not found

**Error**: `error: attribute 'packageName' missing`

**Solution**: Search for correct package name
```bash
nix search nixpkgs packageName
```

Package names sometimes differ from binary names:
- Binary: `aws` → Package: `pkgs.awscli2`
- Binary: `node` → Package: `pkgs.nodejs_20`

### Issue: Want to uninstall Nix

**Solution**: Complete removal
```bash
# Stop Nix daemon
sudo launchctl unload /Library/LaunchDaemons/org.nixos.nix-daemon.plist

# Remove Nix
sudo rm -rf /nix

# Remove from /etc/bashrc, /etc/zshrc (Nix-added lines)

# Remove user data
rm -rf ~/.nix-profile ~/.nix-defexpr ~/.nix-channels ~/.config/nix

# Remove from /etc/synthetic.conf (if exists)
sudo nano /etc/synthetic.conf  # Remove "nix" line
```

---

## Learning Resources

### Official Resources

1. **Zero to Nix** (Highly Recommended for Beginners)
   - URL: https://zero-to-nix.com/
   - Interactive tutorial
   - Covers flakes, dev shells, and practical examples
   - Best starting point for new users

2. **Official Nix Manual**
   - URL: https://nixos.org/manual/nix/stable/
   - Comprehensive reference
   - Read "Quick Start" and "Common Patterns" sections

3. **Nix Pills** (Deep Dive)
   - URL: https://nixos.org/guides/nix-pills/
   - Series of articles explaining Nix internals
   - For understanding "why" not just "how"

4. **Nix Flakes Official Documentation**
   - URL: https://nixos.wiki/wiki/Flakes
   - Everything about flakes (modern Nix)

### Practical Guides

5. **Practical Nix Flakes** (Recommended)
   - URL: https://serokell.io/blog/practical-nix-flakes
   - Real-world examples
   - Development environments focus

6. **Nix.dev** (Tutorials and Guides)
   - URL: https://nix.dev/
   - Curated learning path
   - Best practices and patterns

7. **Nixology** (YouTube Series)
   - URL: https://www.youtube.com/playlist?list=PLRGI9KQ3_HP_OFRG6R-p4iFgMSK1t5BHs
   - Video tutorials by Burke Libbey
   - Covers basics to advanced topics

### Community Resources

8. **Nix Discourse** (Community Forum)
   - URL: https://discourse.nixos.org/
   - Ask questions, browse discussions
   - Very helpful community

9. **r/NixOS Subreddit**
   - URL: https://www.reddit.com/r/NixOS/
   - Community discussions, tips, and examples

10. **Nix Package Search**
    - URL: https://search.nixos.org/packages
    - Search available packages
    - See package definitions and versions

### Emacs-Specific

11. **direnv Emacs Integration**
    - URL: https://github.com/wbolster/emacs-direnv
    - Setup and usage for Emacs

12. **Nix + Emacs Blog Posts**
    - "Nix and Doom Emacs": https://d12frosted.io/posts/2021-05-21-path-in-emacs.html
    - Environment variable handling in Emacs

### Example Repositories

13. **Nix Flake Templates**
    - Command: `nix flake init -t templates#<template>`
    - Built-in templates for common setups

14. **DevEnv.sh** (High-Level Tool Built on Nix)
    - URL: https://devenv.sh/
    - Simpler interface to Nix
    - Good for teams transitioning to Nix

### Cheat Sheets

15. **Nix Flakes Cheat Sheet**
    - URL: https://nixos.wiki/wiki/Flakes
    - Quick reference for common commands

16. **Nix Language Basics**
    - URL: https://nixos.org/manual/nix/stable/language/
    - Syntax reference

---

## Recommended Learning Path

### Week 1: Basics (2-3 hours)

1. Watch: First 3 Nixology videos (30 min each)
2. Read: Zero to Nix "Concepts" section
3. Try: Install Nix, create simple flake with 2-3 packages
4. Practice: Use `nix develop` to enter environment

**Goal**: Understand what Nix is and create your first environment

### Week 2: Practical Use (2-3 hours)

1. Read: Practical Nix Flakes blog post
2. Try: Create flake for real project (CDK environment)
3. Setup: direnv + Emacs integration
4. Practice: Open files in Emacs, see environment activate

**Goal**: Have working Nix + Emacs setup for daily use

### Week 3: Advanced (Optional, 3-4 hours)

1. Read: Nix Pills (first 5 chapters)
2. Browse: nixpkgs on GitHub (understand package structure)
3. Try: Override package, pin specific versions
4. Practice: Create flake for second project

**Goal**: Understand Nix internals, comfortable with customization

### Ongoing

- Use Discourse for questions
- Browse r/NixOS for tips
- Explore nix.dev for best practices

---

## Decision Framework

### Use Nix If:

✅ You use Emacs as primary development environment
✅ You work on multiple projects with different dependencies
✅ You value reproducible environments
✅ You want native macOS performance
✅ You're willing to invest 3-5 hours learning
✅ You like declarative configuration

### Use Finch/Docker If:

✅ You need exact Linux environment
✅ You deploy applications in containers
✅ Team is already using Docker
✅ You prefer familiar Docker workflow
✅ You need strongest isolation
✅ You're not concerned about Emacs integration

### Use Homebrew (Traditional) If:

✅ You have simple needs (few global tools)
✅ You work on one project at a time
✅ You don't need reproducibility
✅ You want simplest possible setup

---

## Next Steps

**Before committing to Nix**:

1. ✅ Read "Zero to Nix" (2 hours): https://zero-to-nix.com/
2. ✅ Watch first 3 Nixology videos (1.5 hours)
3. ✅ Try example flake in test directory (30 min)
4. ✅ Evaluate: Does this fit your workflow?

**If Yes** (Nix fits):
- Follow setup guide in CONTAINERIZED_DEVELOPMENT_ENVIRONMENT.md
- Create flake for infrastructure/
- Configure Doom Emacs with direnv
- Use for 1 week, evaluate

**If No** (Nix doesn't fit):
- Use Finch (Docker alternative) for containerized development
- Follow Finch setup in CONTAINERIZED_DEVELOPMENT_ENVIRONMENT.md
- Configure Doom Emacs with vterm helper function

**Either way**: You have a clean, isolated development environment!

---

## Summary

**Nix is**:
- Package manager with perfect isolation
- Declarative configuration
- Reproducible environments
- Automatic activation with direnv + Emacs

**Best for**:
- Emacs users
- Multi-project developers
- Teams valuing reproducibility

**Learning investment**:
- Initial: 3-5 hours (tutorials + setup)
- Ongoing: Minimal (copy/paste flake examples)

**Resources**:
- Start: Zero to Nix (https://zero-to-nix.com/)
- Reference: Official manual (https://nixos.org/manual/nix/stable/)
- Community: Discourse (https://discourse.nixos.org/)

**Decision**: Spend 2-3 hours learning, then decide if it fits your workflow.
