# Containerized Development Environment for AWS CDK

## Overview

Guide for setting up an isolated development environment for AWS CDK infrastructure work without polluting your host system.

**Goal**: Run all CDK, Node.js, Rust, and AWS tooling inside a container with zero host system impact.

---

## Option 1: Docker Container (Recommended)

### Simple Dockerfile

Create `infrastructure/Dockerfile`:

```dockerfile
FROM node:20-slim

# Install system dependencies
RUN apt-get update && apt-get install -y \
    curl \
    git \
    unzip \
    && rm -rf /var/lib/apt/lists/*

# Install Rust and cargo-lambda
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"
RUN cargo install cargo-lambda

# Install AWS CLI
RUN curl "https://awscli.amazonaws.com/awscli-exe-linux-x86_64.zip" -o "awscliv2.zip" \
    && unzip awscliv2.zip \
    && ./aws/install \
    && rm -rf aws awscliv2.zip

# Install AWS CDK globally
RUN npm install -g aws-cdk

# Set working directory
WORKDIR /workspace

# Verify installations
RUN node --version && \
    npm --version && \
    cdk --version && \
    cargo --version && \
    cargo lambda --version && \
    aws --version

CMD ["/bin/bash"]
```

### Build and Run

```bash
# Build the container (one-time)
cd gruesome/infrastructure
docker build -t gruesome-cdk .

# Run container with AWS credentials and workspace mounted
docker run -it --rm \
  -v ~/.aws:/root/.aws:ro \
  -v $(pwd):/workspace \
  gruesome-cdk bash

# Inside container:
cdk --version           # Verify CDK works
aws sts get-caller-identity  # Verify AWS access
cdk bootstrap aws://ACCOUNT-ID/us-east-1
cdk deploy --all
```

### Convenience Script

Create `infrastructure/dev.sh`:

```bash
#!/bin/bash
docker run -it --rm \
  -v ~/.aws:/root/.aws:ro \
  -v $(pwd):/workspace \
  gruesome-cdk bash
```

```bash
chmod +x infrastructure/dev.sh
./infrastructure/dev.sh
```

**Pros**:
- Completely isolated from host
- Reproducible environment
- Easy to share with team
- No pollution of host system

**Cons**:
- Slightly slower build times (first time only)
- Need to rebuild if dependencies change

---

## Option 2: Docker Compose

Create `infrastructure/docker-compose.yml`:

```yaml
version: '3.8'

services:
  cdk:
    build: .
    volumes:
      - ~/.aws:/root/.aws:ro  # Mount AWS credentials (read-only)
      - .:/workspace          # Mount current directory
      - /var/run/docker.sock:/var/run/docker.sock  # If using docker-in-docker
    working_dir: /workspace
    environment:
      - AWS_REGION=us-west-2
      - AWS_DEFAULT_REGION=us-west-2
    command: /bin/bash
    stdin_open: true
    tty: true
```

### Usage

```bash
cd gruesome/infrastructure

# Start container
docker-compose run --rm cdk

# Inside container, work normally:
cdk deploy --all
cargo lambda build --release --arm64
```

---

## Option 3: DevContainer (VSCode Integration)

Create `.devcontainer/devcontainer.json`:

```json
{
  "name": "Gruesome CDK Environment",
  "build": {
    "dockerfile": "../infrastructure/Dockerfile"
  },
  "mounts": [
    "source=${localEnv:HOME}/.aws,target=/root/.aws,type=bind,consistency=cached"
  ],
  "customizations": {
    "vscode": {
      "extensions": [
        "rust-lang.rust-analyzer",
        "vadimcn.vscode-lldb",
        "dbaeumer.vscode-eslint",
        "esbenp.prettier-vscode"
      ]
    }
  },
  "postCreateCommand": "npm install",
  "remoteUser": "root"
}
```

**Usage**: VSCode will prompt "Reopen in Container" - everything runs inside container transparently!

---

## Option 4: AWS Finch (Docker Alternative)

**Finch** is AWS's open-source container runtime - drop-in replacement for Docker with better macOS integration.

### Install Finch

```bash
# macOS
brew install finch

# Initialize Finch VM
finch vm init

# Verify
finch version
```

### Use Exactly Like Docker

```bash
# Build with Finch (same Dockerfile)
finch build -t gruesome-cdk infrastructure/

# Run with Finch
finch run -it --rm \
  -v ~/.aws:/root/.aws:ro \
  -v $(pwd):/workspace \
  gruesome-cdk bash

# Or create finch-dev.sh
cat > infrastructure/finch-dev.sh << 'EOF'
#!/bin/bash
finch run -it --rm \
  -v ~/.aws:/root/.aws:ro \
  -v $(pwd):/workspace \
  gruesome-cdk bash
EOF
chmod +x infrastructure/finch-dev.sh
```

**Finch Advantages**:
- Native macOS integration (uses Lima VM)
- Lighter than Docker Desktop
- AWS-optimized
- Drop-in Docker replacement (`finch` instead of `docker`)
- No Docker Desktop licensing issues

**Finch + Emacs**:
- Works perfectly with Emacs terminal (`M-x term` or `M-x vterm`)
- Same commands as Docker
- Can use `projectile-run-shell-command-in-root` to launch

---

## Option 5: Nix Flake (Most Elegant, macOS Native)

No containers - pure functional package management with complete isolation.

Create `infrastructure/flake.nix`:

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
            aws --version
            cargo --version
          '';
        };
      }
    );
}
```

### Install Nix (One-Time)

```bash
# Install Nix
sh <(curl -L https://nixos.org/nix/install) --daemon

# Enable flakes
mkdir -p ~/.config/nix
echo "experimental-features = nix-command flakes" >> ~/.config/nix/nix.conf
```

### Usage with Emacs

```bash
cd infrastructure

# Enter dev environment
nix develop

# Inside nix shell, all tools available:
cdk deploy --all
```

**Emacs Integration**:

Add to your Doom Emacs `config.el`:

```elisp
;; direnv integration for Nix
(use-package! direnv
  :config
  (direnv-mode))
```

Create `infrastructure/.envrc`:

```bash
use flake
```

Then run `direnv allow` - Emacs will automatically load Nix environment when you open files in that directory!

**Nix Advantages**:
- Native macOS performance (no container overhead)
- Completely isolated (nothing touches global system)
- Instant activation (no container build)
- Declarative, reproducible
- Perfect Emacs integration via direnv

**Nix Disadvantages**:
- Requires Nix installation
- Slight learning curve
- Takes disk space (~2-3GB for this environment)

---

## Emacs Integration Comparison

### Docker/Finch + Emacs

**Terminal-based workflow**:

```elisp
;; In Doom Emacs config.el

(defun gruesome-cdk-shell ()
  "Launch Finch/Docker container for CDK work."
  (interactive)
  (let ((default-directory "~/Projects/Zork/gruesome/infrastructure/"))
    (vterm "gruesome-cdk")
    (vterm-send-string "./finch-dev.sh\n")))

(map! :leader
      :desc "CDK Shell" "o c" #'gruesome-cdk-shell)
```

Usage: `SPC o c` launches container in vterm

**File editing**:
- Edit files normally in Emacs (they're mounted volumes)
- Run commands in vterm container
- Changes sync instantly

### Nix + direnv + Emacs (Seamless!)

**Automatic environment**:

```elisp
;; In packages.el
(package! direnv)

;; In config.el
(use-package! direnv
  :config
  (direnv-mode))
```

Create `infrastructure/.envrc`:
```bash
use flake
```

Then `direnv allow`

**Result**: Opening ANY file in `infrastructure/` automatically loads Nix environment!

**Emacs commands work natively**:
- `M-x compile` → runs with Nix-provided tools
- `M-x shell` → shell has all tools
- `projectile-run-shell-command-in-root` → uses Nix environment
- LSP servers use Nix-provided language tools

**This is the smoothest Emacs integration!**

---

## Comparison Table

| Approach | Isolation | Speed | Emacs Integration | macOS Native |
|----------|-----------|-------|-------------------|--------------|
| **Docker** | Full | Medium | Terminal (vterm) | No (VM) |
| **Finch** | Full | Medium | Terminal (vterm) | Better (Lima) |
| **Docker Compose** | Full | Medium | Terminal | No |
| **DevContainer** | Full | Medium | VSCode only | No |
| **Nix + direnv** | Full | Fast | **Seamless** | **Yes** |

---

## Recommended Approach for Doom Emacs User

### Best: Nix + direnv

**Why**:
- ✅ Perfect Emacs integration (automatic environment switching)
- ✅ Native macOS performance
- ✅ Zero container overhead
- ✅ Instant activation
- ✅ Completely reproducible
- ✅ Works with all Emacs features (LSP, compile, shell, etc.)

**Setup**:

1. Install Nix:
   ```bash
   sh <(curl -L https://nixos.org/nix/install) --daemon
   ```

2. Enable flakes:
   ```bash
   mkdir -p ~/.config/nix
   echo "experimental-features = nix-command flakes" >> ~/.config/nix/nix.conf
   ```

3. Add to Doom `packages.el`:
   ```elisp
   (package! direnv)
   ```

4. Add to Doom `config.el`:
   ```elisp
   (use-package! direnv
     :config
     (direnv-mode))
   ```

5. Create `infrastructure/flake.nix` (from above)

6. Create `infrastructure/.envrc`:
   ```bash
   use flake
   ```

7. Run in infrastructure directory:
   ```bash
   direnv allow
   ```

**Usage**:
- Open any file in `infrastructure/` in Emacs
- Environment automatically loaded
- Run `M-x shell` or `M-x vterm` → all tools available
- Run `M-x compile` with `cdk deploy` → works perfectly

### Alternative: Finch + vterm

If you prefer containers:

1. Install Finch:
   ```bash
   brew install finch
   finch vm init
   ```

2. Create Dockerfile (from above)

3. Build:
   ```bash
   finch build -t gruesome-cdk infrastructure/
   ```

4. Add to Doom `config.el`:
   ```elisp
   (defun gruesome-cdk-shell ()
     "Launch Finch container for CDK work."
     (interactive)
     (let ((default-directory "~/Projects/Zork/gruesome/infrastructure/"))
       (vterm "gruesome-cdk")
       (vterm-send-string "finch run -it --rm -v ~/.aws:/root/.aws:ro -v $(pwd):/workspace gruesome-cdk bash\n")))

   (map! :leader
         :desc "CDK Shell" "o c" #'gruesome-cdk-shell)
   ```

5. Usage: `SPC o c` in Emacs

**Edit files in Emacs (host), run commands in vterm container**

---

## Quick Start: Finch

```bash
# 1. Install Finch
brew install finch
finch vm init

# 2. Create Dockerfile
mkdir -p infrastructure
cat > infrastructure/Dockerfile << 'EOF'
FROM node:20-slim
RUN apt-get update && apt-get install -y curl git unzip && rm -rf /var/lib/apt/lists/*
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"
RUN cargo install cargo-lambda
RUN curl "https://awscli.amazonaws.com/awscli-exe-linux-x86_64.zip" -o "awscliv2.zip" \
    && unzip awscliv2.zip && ./aws/install && rm -rf aws awscliv2.zip
RUN npm install -g aws-cdk
WORKDIR /workspace
CMD ["/bin/bash"]
EOF

# 3. Build
cd infrastructure
finch build -t gruesome-cdk .

# 4. Create helper script
cat > finch-dev.sh << 'EOF'
#!/bin/bash
finch run -it --rm \
  -v ~/.aws:/root/.aws:ro \
  -v $(pwd):/workspace \
  gruesome-cdk bash
EOF
chmod +x finch-dev.sh

# 5. Run!
./finch-dev.sh

# Inside container:
cdk --version
aws sts get-caller-identity
cdk deploy --all
```

---

## Quick Start: Nix (Recommended for Emacs)

```bash
# 1. Install Nix
sh <(curl -L https://nixos.org/nix/install) --daemon

# 2. Enable flakes
mkdir -p ~/.config/nix
echo "experimental-features = nix-command flakes" >> ~/.config/nix/nix.conf

# 3. Create flake.nix in infrastructure/ (from above)

# 4. Create .envrc
echo "use flake" > infrastructure/.envrc

# 5. Install direnv
brew install direnv

# 6. Add to Doom Emacs (packages.el)
# (package! direnv)

# 7. Add to Doom Emacs (config.el)
# (use-package! direnv :config (direnv-mode))

# 8. Reload Doom
# doom sync && doom reload

# 9. Allow direnv in infrastructure/
cd infrastructure
direnv allow

# 10. Open infrastructure/ in Emacs - environment auto-loaded!
```

---

## Advanced: Multi-Stage Dockerfile (Smaller Image)

```dockerfile
# Stage 1: Build tools
FROM node:20-slim AS builder

RUN apt-get update && apt-get install -y curl git unzip && rm -rf /var/lib/apt/lists/*
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"
RUN cargo install cargo-lambda

# Stage 2: Runtime
FROM node:20-slim

# Copy Rust toolchain from builder
COPY --from=builder /root/.cargo /root/.cargo
COPY --from=builder /root/.rustup /root/.rustup
ENV PATH="/root/.cargo/bin:${PATH}"

# Install AWS CLI
RUN apt-get update && apt-get install -y curl unzip && rm -rf /var/lib/apt/lists/*
RUN curl "https://awscli.amazonaws.com/awscli-exe-linux-x86_64.zip" -o "awscliv2.zip" \
    && unzip awscliv2.zip && ./aws/install && rm -rf aws awscliv2.zip

# Install CDK
RUN npm install -g aws-cdk

WORKDIR /workspace
CMD ["/bin/bash"]
```

Results in smaller final image (~800MB instead of ~1.2GB).

---

## Summary

**For Doom Emacs users**: Use **Nix + direnv** for seamless integration

**For container preference**: Use **Finch** (AWS-optimized Docker alternative)

**For team sharing**: Use **Docker Compose** (consistent across developers)

**For VSCode users**: Use **DevContainer** (IDE integration)

Both Nix and Finch work perfectly with Emacs - Nix has better integration, Finch has stronger isolation.

---

## Finch + Doom Emacs Integration

### Quick Setup

```bash
# Install Finch
brew install finch
finch vm init

# Build container (same Dockerfile as Docker)
cd infrastructure
finch build -t gruesome-cdk .

# Create convenience script
cat > finch-dev.sh << 'EOF'
#!/bin/bash
finch run -it --rm \
  -v ~/.aws:/root/.aws:ro \
  -v $(pwd):/workspace \
  gruesome-cdk bash
EOF
chmod +x finch-dev.sh
```

### Doom Emacs Configuration

Add to `~/.doom.d/config.el`:

```elisp
(defun gruesome-cdk-shell ()
  "Launch Finch container for CDK work in vterm."
  (interactive)
  (let ((default-directory "~/Projects/Zork/gruesome/infrastructure/"))
    (vterm "gruesome-cdk")
    (vterm-send-string "./finch-dev.sh\n")))

(map! :leader
      :desc "CDK Shell" "o c" #'gruesome-cdk-shell)
```

**Usage**: Press `SPC o c` anywhere in Emacs → instant CDK container in vterm!

**Workflow**:
- Edit files normally in Emacs (they're mounted volumes)
- Run commands in vterm container (`SPC o c`)
- Changes sync instantly between Emacs and container

---

## Nix + Doom Emacs Integration (Recommended)

### Why Nix is Better for Emacs

**Containers (Docker/Finch)**:
- ❌ Need to "enter" container to use tools
- ❌ Separate terminal environment
- ❌ Slower (VM overhead)
- ❌ LSP servers run in container or host (split environment)
- ✅ Strong isolation
- ✅ Familiar Docker workflow

**Nix + direnv**:
- ✅ **Automatic** - environment loads when you open files
- ✅ **Seamless** - all Emacs features work (LSP, compile, shell, projectile)
- ✅ **Fast** - native macOS, zero VM overhead
- ✅ **Perfect isolation** - nothing touches global system
- ✅ **LSP servers** use Nix-provided tools automatically
- ✅ No "entering" environment - it just works

### What is Nix?

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

### Nix Installation (One-Time, 5 Minutes)

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
- Nothing else on your system touched
- You can uninstall cleanly: `sudo rm -rf /nix`

### Doom Emacs Setup for Nix

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

**Step 4**: Install direnv

```bash
brew install direnv
```

### Project Setup with Nix

**Step 1**: Create flake.nix in infrastructure directory

```bash
cd infrastructure
```

Create `flake.nix` (already in the guide above - complete file)

**Step 2**: Create .envrc

```bash
echo "use flake" > .envrc
```

**Step 3**: Allow direnv

```bash
direnv allow
```

**What happens**:
- First time: Nix downloads and builds environment (~5 minutes)
- Cached forever after that
- When you `cd infrastructure/`: environment activates
- When you leave: environment deactivates

### Using Nix with Emacs

**Automatic Magic**:

1. Open ANY file in `infrastructure/` directory
2. Emacs status bar shows: `direnv: loading ~/Projects/Zork/gruesome/infrastructure/.envrc`
3. Environment is NOW active in all Emacs commands!

**Try it**:

```elisp
M-x shell  ; or M-x vterm
```

Type in the shell:
```bash
node --version    # Works! From Nix
cdk --version     # Works! From Nix
cargo --version   # Works! From Nix
aws --version     # Works! From Nix
```

**Close file, open file outside infrastructure/**:
```elisp
M-x shell
```

Type:
```bash
node --version    # Not found! (unless you have global node)
```

**The environment is automatic per-directory!**

### Advanced Emacs Integration

**Projectile commands work**:
```elisp
M-x projectile-run-shell-command-in-root
```
Command: `cdk deploy --all` → Uses Nix environment automatically!

**Compilation works**:
```elisp
M-x compile
```
Command: `cdk synth` → Uses Nix CDK!

**LSP works**:
- Open `infrastructure/lib/dns-stack.ts`
- TypeScript LSP uses Nix-provided Node.js
- No configuration needed!

**Magit works**:
- All git commands use Nix environment
- Pre-commit hooks run in Nix environment

### Nix Flake Explained

The `flake.nix` file is declarative configuration:

```nix
{
  description = "What this environment is for";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";  # Where to get packages
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
      in
      {
        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            # List packages you want
            nodejs_20      # Node.js version 20
            awscli2        # AWS CLI
            cargo          # Rust
            cargo-lambda   # Cargo Lambda
          ];

          shellHook = ''
            # Run when environment activates
            echo "CDK environment ready!"
          '';
        };
      }
    );
}
```

**That's it!** Nix handles:
- Downloading exact versions
- Making them available in PATH
- Keeping them isolated
- Caching for instant activation next time

### Nix Common Workflows

**Update packages**:
```bash
cd infrastructure
nix flake update  # Updates to latest versions
```

**Enter environment manually** (if not using direnv):
```bash
cd infrastructure
nix develop  # Activates environment in current shell
```

**Clean cache** (if you need space):
```bash
nix-collect-garbage -d  # Removes old versions
```

**Add a new tool**:

Edit `flake.nix`:
```nix
buildInputs = with pkgs; [
  nodejs_20
  awscli2
  cargo
  cargo-lambda
  jq            # Add this!
  docker        # Or this!
];
```

Then:
```bash
direnv reload  # Or just reopen file in Emacs
```

Now `jq` is available!

### Nix vs Global Installation

**Global (traditional)**:
```bash
brew install node@20
brew install awscli
brew install rust
npm install -g aws-cdk
cargo install cargo-lambda
```

**Problems**:
- Pollutes system globally
- Version conflicts (one project needs Node 18, another needs 20)
- Hard to reproduce on different machines
- Upgrading breaks things

**Nix way**:
```nix
# flake.nix
buildInputs = with pkgs; [ nodejs_20 awscli2 cargo ];
```

**Benefits**:
- Isolated per project
- No version conflicts
- Perfect reproducibility
- Upgrade one project without affecting others
- Delete project → all dependencies gone

### Real-World Example

**Scenario**: You have two projects

**Project A**: infrastructure/ (CDK with Node 20)
```nix
# infrastructure/flake.nix
buildInputs = with pkgs; [ nodejs_20 cargo ];
```

**Project B**: old-app/ (Legacy app with Node 16)
```nix
# old-app/flake.nix
buildInputs = with pkgs; [ nodejs_16 python310 ];
```

**With Nix + direnv + Emacs**:

1. Open `infrastructure/file.ts` → Node 20 available
2. Open `old-app/server.js` → Node 16 available
3. Switch between files → environment switches automatically!
4. No conflicts, no manual `nvm use`, no thinking about it

**This is why Nix is magical for Emacs users!**

### Troubleshooting Nix

**Issue**: "experimental-features" error

**Solution**: Enable flakes (shown in installation above)

**Issue**: direnv not loading in Emacs

**Solution**: Check doom package installed: `M-x doom/reload`

**Issue**: Slow first-time setup

**Solution**: Normal - Nix downloads and builds. Cached after that (~5 minutes first time)

**Issue**: Want to uninstall Nix

**Solution**:
```bash
sudo rm -rf /nix
# Remove from /etc/bashrc or /etc/zshrc
```

### Why Choose Nix Over Containers?

**Choose Nix if**:
- ✅ You use Emacs heavily
- ✅ You want seamless environment switching
- ✅ You want native performance
- ✅ You work on multiple projects with different dependencies
- ✅ You want automatic LSP/tool integration

**Choose Finch/Docker if**:
- ✅ You need exact Linux environment
- ✅ You deploy to containers
- ✅ You're already familiar with Docker
- ✅ Team uses Docker
- ✅ You prefer explicit "entering" environment

**Best of both worlds**: Use Nix for development, Docker for deployment!

---

## Comparison: Finch vs Nix for Doom Emacs

| Feature | Finch + vterm | Nix + direnv |
|---------|---------------|--------------|
| **Emacs Integration** | Good (terminal) | **Seamless** |
| **Speed** | Medium (VM) | **Fast** (native) |
| **Auto-activation** | No (manual) | **Yes** (automatic) |
| **LSP Support** | Complex | **Native** |
| **Compile mode** | N/A | **Works** |
| **Projectile** | N/A | **Works** |
| **Learning Curve** | Low (like Docker) | Medium |
| **Isolation** | Full | Full |
| **Setup Time** | 10 min | 20 min |

**Recommendation**: Start with **Nix + direnv** - the 20 minute initial investment pays off immediately with seamless Emacs integration.

---

## Resources

- **Finch**: https://github.com/runfinch/finch
- **Nix**: https://nixos.org/
- **Nix Flakes**: https://nixos.wiki/wiki/Flakes
- **direnv**: https://direnv.net/
- **Emacs direnv package**: https://github.com/wbolster/emacs-direnv
- **Doom Emacs**: https://github.com/doomemacs/doomemacs
- **Zero to Nix**: https://zero-to-nix.com/ (excellent tutorial)
