# Lambda Deployment Process

**Critical documentation for deploying Rust Lambda functions**

Last updated: 2025-12-20

---

## What is bootstrap.zip?

AWS Lambda's custom runtime (like Rust) requires a binary named `bootstrap`. This binary:
- **Must be named exactly "bootstrap"** (no extension)
- Contains the compiled Rust executable for your Lambda function
- Must be packaged in a **zip file** for deployment to AWS Lambda
- Each Lambda function has its own bootstrap.zip

### Why "bootstrap"?

AWS Lambda's `provided.al2023` runtime looks for an executable named `bootstrap` as the entry point. This is the convention for custom runtimes (Rust, Go, etc.).

---

## Our Lambda Functions

We have **3 separate Lambda functions** in `infrastructure/lambda/gruesome-api/`:

| Function | Purpose | Path |
|----------|---------|------|
| **auth** | Authentication (login, signup, JWT) | `auth/` |
| **game** | Game operations (list, download, saves) | `game/` |
| **admin** | Admin operations (upload, edit, delete games) | `admin/` |

**Each function requires its own bootstrap.zip file.**

---

## Build Process

### Command: cargo lambda build

```bash
cd infrastructure/lambda/gruesome-api

# Build all Lambda functions
cargo lambda build --release --arm64

# Build specific function
cargo lambda build --release --arm64 --bin game
cargo lambda build --release --arm64 --bin auth
cargo lambda build --release --arm64 --bin admin
```

### What cargo lambda build does:

1. **Compiles** the Rust code with Lambda-specific optimizations
2. **Targets ARM64** architecture (AWS Graviton processors - cheaper and faster)
3. **Creates** the `bootstrap` binary in `target/lambda/{function_name}/bootstrap`
4. **SHOULD create** `bootstrap.zip` automatically

### ⚠️ CRITICAL BUG: bootstrap.zip not always created

**Problem discovered 2025-12-20:**
- `cargo lambda build` creates the `bootstrap` binary
- **BUT** it does NOT always create `bootstrap.zip`
- The zip file must be **manually created** in some cases

**Current state of bootstrap.zip files:**
```
target/lambda/auth/bootstrap.zip   - EXISTS (Dec 17)
target/lambda/game/bootstrap.zip   - EXISTS (Dec 20) - manually created
target/lambda/admin/bootstrap.zip  - MISSING (must create manually)
```

### Manual zip creation:

```bash
cd target/lambda/game
zip bootstrap.zip bootstrap
```

**This is the step I missed** when deploying the game Lambda, causing it to use an old binary from December 16.

---

## Deployment Process

### Current Deployment Methods

We have **two deployment approaches** that are NOT consistent:

#### 1. CDK Deployment (infrastructure changes)

```bash
cd infrastructure
cdk deploy GruesomeBackendStack          # Production
cdk deploy GruesomeBackendStackStaging   # Staging
```

**When CDK deploys:**
- Reads Lambda code from `lambda/gruesome-api/target/lambda/{function}/`
- Packages and uploads to S3
- Updates Lambda function configuration
- **CDK looks for bootstrap binary, NOT bootstrap.zip**

**CDK deployment workflow:**
1. Modify Rust code
2. `cargo lambda build --release --arm64`
3. `cdk deploy {StackName}`
4. CDK packages the bootstrap binary automatically

#### 2. Direct Lambda Update (code-only changes)

```bash
# Update specific Lambda function code without CDK
aws lambda update-function-code \
  --function-name {FunctionName} \
  --zip-file fileb://path/to/bootstrap.zip
```

**Direct update workflow:**
1. Modify Rust code
2. `cargo lambda build --release --arm64 --bin {function}`
3. **Manually create bootstrap.zip** (if not exists)
4. `aws lambda update-function-code` with zip file

**This is what I used today and it REQUIRES bootstrap.zip**

---

## Where Things Went Wrong (2025-12-20)

### The Failure Chain:

1. **User requested:** Game display order functionality
2. **I modified:** `game/src/models.rs` and `game/src/game_service.rs`
3. **I ran:** `cargo lambda build --release --arm64 --bin game`
4. **Build output:** "Finished in 21.05s" ✅
5. **I assumed:** bootstrap.zip was created/updated ❌
6. **I deployed:** Used OLD bootstrap.zip from December 16
7. **Result:** Deployed old code without display_order support
8. **User saw:** "Failed to load games: Cannot read properties of undefined"

### Why the build was cached:

```bash
# First build after changes
cargo lambda build --release --arm64 --bin game
# Output: Compiling game... Finished in 21.05s ✅

# Subsequent builds (no source changes)
cargo lambda build --release --arm64 --bin game
# Output: Finished in 0.10s ⚠️ CACHED!
```

**The 0.10s build time was a warning sign** - it didn't recompile because:
- Source files hadn't changed between builds
- Cargo saw no reason to rebuild
- But bootstrap.zip was never created in the first place

### The Fix:

```bash
# Force rebuild by touching source
touch game/src/game_service.rs

# Rebuild (actually compiles this time)
cargo lambda build --release --arm64 --bin game
# Output: Compiling game... Finished in 21.76s ✅

# Manually create zip
cd target/lambda/game
zip bootstrap.zip bootstrap

# Deploy correct version
aws lambda update-function-code --function-name ... --zip-file fileb://bootstrap.zip
```

---

## Prevention: Updated Deployment Scripts

### Script: deploy-lambda.sh (NEW - TO BE CREATED)

```bash
#!/bin/bash
# Deploy Lambda function code (Rust)
# Usage: ./deploy-lambda.sh [function] [environment]
#   function: auth|game|admin
#   environment: staging|prod

set -e

FUNCTION="${1}"
ENV="${2:-staging}"

if [ -z "$FUNCTION" ]; then
    echo "Error: Function name required"
    echo "Usage: $0 [auth|game|admin] [staging|prod]"
    exit 1
fi

# Validate function
if [[ ! "$FUNCTION" =~ ^(auth|game|admin)$ ]]; then
    echo "Error: Function must be auth, game, or admin"
    exit 1
fi

# Validate environment
if [[ ! "$ENV" =~ ^(staging|prod)$ ]]; then
    echo "Error: Environment must be staging or prod"
    exit 1
fi

echo "================================================"
echo "Deploying Lambda: $FUNCTION ($ENV)"
echo "================================================"

# Navigate to lambda directory
cd "$(dirname "$0")/../lambda/gruesome-api"

# Clean previous build for this function
echo "Cleaning previous build..."
rm -rf "target/lambda/$FUNCTION"

# Build Lambda function
echo "Building Lambda function..."
cargo lambda build --release --arm64 --bin "$FUNCTION"

# Check if build succeeded by verifying bootstrap binary exists
if [ ! -f "target/lambda/$FUNCTION/bootstrap" ]; then
    echo "Error: Bootstrap binary not created"
    exit 1
fi

# Get file size and timestamp
BOOTSTRAP_SIZE=$(ls -lh "target/lambda/$FUNCTION/bootstrap" | awk '{print $5}')
echo "Bootstrap binary created: $BOOTSTRAP_SIZE"

# Create zip file
echo "Creating bootstrap.zip..."
cd "target/lambda/$FUNCTION"
zip -q bootstrap.zip bootstrap

# Verify zip was created
if [ ! -f "bootstrap.zip" ]; then
    echo "Error: Failed to create bootstrap.zip"
    exit 1
fi

ZIP_SIZE=$(ls -lh bootstrap.zip | awk '{print $5}')
echo "bootstrap.zip created: $ZIP_SIZE"

# Get Lambda function name based on environment
if [ "$ENV" == "staging" ]; then
    case "$FUNCTION" in
        auth)
            LAMBDA_NAME="GruesomeBackendStackStaging-AuthFunction..."
            ;;
        game)
            LAMBDA_NAME="GruesomeBackendStackStaging-GameFunction60282B81-fjABSX02nmIh"
            ;;
        admin)
            LAMBDA_NAME="GruesomeBackendStackStaging-AdminFunction..."
            ;;
    esac
else
    case "$FUNCTION" in
        auth)
            LAMBDA_NAME="GruesomeBackendStack-AuthFunction..."
            ;;
        game)
            LAMBDA_NAME="GruesomeBackendStack-GameFunction60282B81-wlkbuV4oonq1"
            ;;
        admin)
            LAMBDA_NAME="GruesomeBackendStack-AdminFunction..."
            ;;
    esac
fi

# Get full path to bootstrap.zip
ZIP_PATH="$(pwd)/bootstrap.zip"

# Return to infrastructure directory
cd ../../../..

# Deploy to Lambda
echo "Deploying to $LAMBDA_NAME..."
aws lambda update-function-code \
    --function-name "$LAMBDA_NAME" \
    --zip-file "fileb://$ZIP_PATH" \
    --query '{FunctionName:FunctionName,LastModified:LastModified,CodeSize:CodeSize}' \
    --output table

echo ""
echo "================================================"
echo "Deployment Complete!"
echo "================================================"
echo "Function: $FUNCTION"
echo "Environment: $ENV"
echo "Lambda: $LAMBDA_NAME"
echo ""
echo "Verify deployment:"
if [ "$ENV" == "staging" ]; then
    echo "  curl -s https://api-staging.gruesome.skeptomai.com/health | jq '.'"
else
    echo "  curl -s https://api.gruesome.skeptomai.com/health | jq '.'"
fi
```

---

## Verification Checklist

Before deploying Lambda code changes, verify:

```bash
# 1. Source files modified (check git status)
git status

# 2. Clean build artifacts for the function
rm -rf target/lambda/{function}

# 3. Build with cargo lambda
cargo lambda build --release --arm64 --bin {function}

# 4. Verify compilation actually happened (>15s build time)
# If it finishes in <1s, something is cached/wrong

# 5. Verify bootstrap binary was created TODAY
ls -lh target/lambda/{function}/bootstrap
# Check the timestamp - must be recent!

# 6. Create/verify bootstrap.zip exists
cd target/lambda/{function}
zip bootstrap.zip bootstrap
ls -lh bootstrap.zip
# Must be created/modified TODAY

# 7. Deploy
aws lambda update-function-code --function-name ... --zip-file fileb://bootstrap.zip

# 8. TEST the deployed function
# Don't just assume it worked!
curl -s {api-endpoint} | jq '.'

# 9. Check Lambda logs for errors
aws logs tail /aws/lambda/{function-name} --since 5m
```

---

## Lambda Function Names

**⚠️ TODO: Get actual Lambda function names and update script**

Current known functions:
- **Game Staging**: `GruesomeBackendStackStaging-GameFunction60282B81-fjABSX02nmIh`
- **Game Production**: `GruesomeBackendStack-GameFunction60282B81-wlkbuV4oonq1`
- **Admin Staging**: TBD
- **Admin Production**: TBD
- **Auth Staging**: TBD
- **Auth Production**: TBD

Get function names:
```bash
aws lambda list-functions --query 'Functions[?contains(FunctionName, `Gruesome`)].FunctionName' --output table
```

---

## When to Use Each Deployment Method

### Use CDK Deploy when:
- ✅ Changing infrastructure (DynamoDB, S3, API Gateway, etc.)
- ✅ Deploying all functions at once
- ✅ Initial deployment
- ✅ Changing environment variables
- ✅ Changing Lambda configuration (memory, timeout, etc.)

### Use Direct Lambda Update when:
- ✅ **Code-only changes** (modified Rust source)
- ✅ Quick iteration during development
- ✅ Deploying to single function
- ✅ No infrastructure changes

**Most common scenario: Code-only changes** → Use direct update script

---

## Build Artifacts Location

```
infrastructure/lambda/gruesome-api/target/
├── release/              # Regular Rust build (NOT for Lambda)
│   ├── auth              # Regular binary (not Lambda-compatible)
│   ├── game
│   └── admin
└── lambda/               # Lambda-specific builds
    ├── auth/
    │   ├── bootstrap     # Lambda binary ⭐
    │   └── bootstrap.zip # Deployment package ⭐
    ├── game/
    │   ├── bootstrap     # Lambda binary ⭐
    │   └── bootstrap.zip # Deployment package ⭐
    └── admin/
        ├── bootstrap     # Lambda binary ⭐
        └── bootstrap.zip # Deployment package ⭐
```

**⚠️ DO NOT deploy from `target/release/`** - those binaries are NOT Lambda-compatible!

**✅ ALWAYS deploy from `target/lambda/{function}/bootstrap.zip`**

---

## Common Mistakes

### ❌ Mistake 1: Using old bootstrap.zip
**Symptom:** Deployed code doesn't include recent changes
**Cause:** bootstrap.zip not rebuilt after source changes
**Fix:** Always check timestamp: `ls -lh target/lambda/{function}/bootstrap.zip`

### ❌ Mistake 2: Cached build
**Symptom:** Build finishes in <1 second
**Cause:** Cargo sees no changes, uses cached binary
**Fix:** `touch {source-file}` to force rebuild, or `cargo clean -p {function}`

### ❌ Mistake 3: Deploying regular binary
**Symptom:** Lambda fails with "runtime error"
**Cause:** Deployed from `target/release/` instead of `target/lambda/`
**Fix:** Always use `target/lambda/{function}/bootstrap.zip`

### ❌ Mistake 4: Missing bootstrap.zip
**Symptom:** "No such file or directory" when deploying
**Cause:** `cargo lambda build` didn't create the zip
**Fix:** Manually create: `cd target/lambda/{function} && zip bootstrap.zip bootstrap`

### ❌ Mistake 5: Not testing after deployment
**Symptom:** User discovers broken functionality
**Cause:** Assumed deployment worked without verification
**Fix:** ALWAYS test the API endpoint after deploying

---

## Summary: How to Deploy Lambda Code Changes

**Step-by-step process (do NOT skip steps):**

1. **Modify Rust source code**
2. **Clean build:** `rm -rf target/lambda/{function}`
3. **Build:** `cargo lambda build --release --arm64 --bin {function}`
4. **Verify build time:** Should be >15s (if <1s, it's cached - investigate!)
5. **Check bootstrap:** `ls -lh target/lambda/{function}/bootstrap` (today's date?)
6. **Create zip:** `cd target/lambda/{function} && zip bootstrap.zip bootstrap`
7. **Check zip:** `ls -lh bootstrap.zip` (today's date?)
8. **Deploy:** `aws lambda update-function-code --function-name {name} --zip-file fileb://bootstrap.zip`
9. **TEST:** `curl {api-endpoint}` to verify changes
10. **Check logs:** `aws logs tail /aws/lambda/{name} --since 5m`

**If you skip step 9 (testing), you might deploy broken code. Don't skip testing.**

---

## Future Improvements

1. **Create deploy-lambda.sh script** with all verification steps
2. **Add pre-deploy hooks** that verify bootstrap.zip timestamp
3. **Automate testing** after deployment (health check + functional test)
4. **Add Lambda function name mapping** (get from CloudFormation outputs)
5. **Consider using CDK for all deployments** (slower but more reliable)
6. **Add git hooks** to prevent committing without testing

---

## References

- AWS Lambda Rust Runtime: https://github.com/awslabs/aws-lambda-rust-runtime
- Cargo Lambda: https://www.cargo-lambda.info/
- Lambda Custom Runtimes: https://docs.aws.amazon.com/lambda/latest/dg/runtimes-custom.html
