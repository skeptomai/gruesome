# AWS Deployment Guide - Gruesome Platform

**Last Updated**: December 19, 2025

This guide documents the complete deployment workflow for the Gruesome multi-user Z-Machine platform on AWS.

## Infrastructure Architecture Update (December 19, 2025)

**Certificate Management Change**: Replaced DnsStack with DnsImportStack to eliminate CloudFormation cross-region export limitations.

**Previous Architecture** (DEPRECATED):
- `GruesomeDnsStack` created ACM certificate with `crossRegionReferences: true`
- Cross-region exports caused UPDATE_ROLLBACK_FAILED state (exports are immutable)
- Required `--exclusively` flag workaround for backend deployments

**Current Architecture** (ACTIVE):
- `GruesomeDnsImportStack` (production) and `GruesomeDnsImportStackStaging` (staging)
- Hardcoded certificate ARN: `arn:aws:acm:us-east-1:349145659387:certificate/33ae9627-b894-4edc-a480-201bc6e8b529`
- No crossRegionReferences flags - eliminates CloudFormation export limitations
- Certificate ARN is static (renewal doesn't change ARN) per AWS best practices
- Infrastructure is now fully updatable without workarounds

**Certificate Management**:
- The ACM certificate is **not managed by CloudFormation** (orphaned resource)
- Created originally by GruesomeDnsStack, retained when that stack was deleted using `--retain-resources` flag
- This is the **ideal state** for long-lived shared resources:
  - Cannot be accidentally deleted by stack updates
  - Auto-renews indefinitely without IaC intervention
  - Shared across multiple stacks via ARN import (AWS recommended pattern)
- If certificate ever needs replacement:
  1. Create new certificate manually or via new stack
  2. Update hardcoded ARN in both `bin/gruesome-platform.ts` and `bin/gruesome-platform-staging.ts`
  3. Deploy all stacks to pick up new certificate
  4. Delete old certificate once no longer in use

---

## Quick Reference: Staging vs Production Deployment

### Staging Deployment

**Backend (Lambda functions):**
```bash
# Build Lambda functions (from project root)
cargo lambda build --release --arm64

# Deploy to staging (from infrastructure directory)
cd infrastructure
cdk deploy GruesomeBackendStackStaging --app "npx ts-node --prefer-ts-exts bin/gruesome-platform-staging.ts"
```

**Frontend:**
```bash
# From project root
./infrastructure/scripts/deploy-frontend.sh staging

# Manual CloudFront invalidation (script has bug)
aws cloudfront create-invalidation --distribution-id E3VWHUOBR5D259 --paths "/*"
```

**Staging URL**: https://staging.gruesome.skeptomai.com
**Staging API**: https://api-staging.gruesome.skeptomai.com

---

### Production Deployment

**Backend (Lambda functions):**
```bash
# Build Lambda functions (from project root)
cargo lambda build --release --arm64

# Deploy to production (from infrastructure directory)
cd infrastructure
cdk deploy GruesomeBackendStack
```

**Frontend:**
```bash
# From project root
./infrastructure/scripts/deploy-frontend.sh prod

# Manual CloudFront invalidation (script has bug)
aws cloudfront create-invalidation --distribution-id E1M8DHMS3GCUDX --paths "/*"
```

**Production URL**: https://gruesome.skeptomai.com
**Production API**: https://api.gruesome.skeptomai.com

---

### Key Differences

| Aspect | Staging | Production |
|--------|---------|------------|
| **CDK App File** | `bin/gruesome-platform-staging.ts` | `bin/gruesome-platform.ts` (default) |
| **CDK Stack Names** | `GruesomeBackendStackStaging`, `GruesomeFrontendStackStaging`, etc. | `GruesomeBackendStack`, `GruesomeFrontendStack`, etc. |
| **DynamoDB Table** | `gruesome-platform-staging` | `gruesome-platform` |
| **S3 Buckets** | `gruesome-games-staging`, `gruesome-saves-staging` | `gruesome-games`, `gruesome-saves` |
| **Cognito Pool** | `us-west-1_1Z4s05N00` | `us-west-1_zSPJeB5x0` |
| **Deploy Command** | Must specify `--app` parameter | Uses default from `cdk.json` |

---

## Testing Checklist: Orphaned Save Prevention (Dec 19, 2025)

**What Changed:**
- Backend now filters out orphaned saves (metadata exists but S3 file missing)
- Backend validates S3 files exist before generating download URLs
- Frontend validates S3 upload succeeded before showing success message
- Frontend has better error handling for save operations

**Test in Staging** (https://staging.gruesome.skeptomai.com):

1. **Verify orphaned saves are hidden**:
   - Login with user `skeptomai`
   - Load Zork I game
   - Click "Load" button
   - **Expected**: You should NOT see saves named "A" or "kitchen" in the list
   - **Why**: These saves have metadata but missing S3 files - backend now filters them out

2. **Test normal save/load workflow**:
   - Play a few turns of any game
   - Click "Save" button, enter a name (e.g., "test1")
   - **Expected**: "Game saved successfully!" message
   - Click "Load" button
   - **Expected**: See "test1" in the list
   - Select "test1" and load it
   - **Expected**: Game restores to saved position

3. **Verify error messages are clear** (if something fails):
   - If save upload fails: "Failed to upload save file to storage"
   - If save load fails: "Save file not found in storage" (not "undefined is not an object")

**Deploy to Production After Testing**: If all tests pass, run the same deployment commands for production (see Quick Reference above).

---

## Architecture Overview

### Stack Components

**Frontend**:
- S3 bucket for static assets (HTML, CSS, JS, WASM)
- CloudFront distribution for CDN
- Custom domain: `gruesome.skeptomai.com` (prod) / `staging.gruesome.skeptomai.com` (staging)

**Backend**:
- API Gateway (HTTP API)
- Lambda functions (Rust, ARM64 architecture)
- Custom domain: `api.gruesome.skeptomai.com` (prod) / `api-staging.gruesome.skeptomai.com` (staging)

**Auth**:
- Cognito User Pools
- JWT token-based authentication

**Data**:
- DynamoDB (single-table design) for metadata
- S3 buckets for game files and save files

**Infrastructure as Code**:
- AWS CDK (TypeScript)
- Located in `/infrastructure` directory

---

## Directory Structure

```
/Users/christopherbrown/Projects/Zork/gruesome/
├── frontend/                          # Frontend code (WASM app)
│   ├── index.html
│   ├── app.js
│   ├── style.css
│   └── pkg/                           # WASM build output
├── auth/                              # Lambda function: Auth endpoints
│   ├── Cargo.toml
│   └── src/
├── game/                              # Lambda function: Game endpoints
│   ├── Cargo.toml
│   └── src/
├── infrastructure/                    # AWS CDK infrastructure
│   ├── bin/
│   │   └── gruesome-platform.ts      # CDK app entry
│   ├── lib/
│   │   ├── dns-stack.ts              # Route 53 + ACM
│   │   ├── data-stack.ts             # DynamoDB + S3
│   │   ├── auth-stack.ts             # Cognito
│   │   ├── backend-stack.ts          # API Gateway + Lambda
│   │   └── frontend-stack.ts         # S3 + CloudFront
│   ├── scripts/
│   │   ├── deploy-frontend.sh        # Frontend deployment script
│   │   ├── upload-game.sh            # Game upload utility
│   │   └── verify-game-library.sh    # Library verification
│   ├── cdk.json
│   └── package.json
└── target/lambda/                     # Built Lambda binaries
    ├── auth/bootstrap                 # Auth Lambda (ARM64)
    └── game/bootstrap                 # Game Lambda (ARM64)
```

---

## Lambda Function Architecture

### Key Details

**Architecture**: ARM64 (not x86_64!)
- Configured in `infrastructure/lib/backend-stack.ts`: `architecture: lambda.Architecture.ARM_64`
- Runtime: `PROVIDED_AL2023` (Amazon Linux 2023 custom runtime)
- Handler: `bootstrap` (standard for Rust Lambda functions)

**Source Code Locations**:
- Auth Lambda: `/auth/` (workspace member)
- Game Lambda: `/game/` (workspace member)
- NOT under `infrastructure/lambda/` - they're at project root!

**Build Output**:
- Built binaries: `/target/lambda/{auth,game}/bootstrap`
- CDK deployment reads from these paths (relative to infrastructure directory)

**Build Tool**: `cargo-lambda`
- Installed via Homebrew: `/opt/homebrew/bin/cargo-lambda`
- Handles cross-compilation for ARM64 Lambda runtime
- Packages binaries in correct format for AWS Lambda

---

## Deployment Workflows

### 1. Backend Deployment (Lambda Functions)

#### Build Lambda Functions

```bash
# From project root
cd /Users/christopherbrown/Projects/Zork/gruesome

# Build both auth and game Lambda functions for ARM64
cargo lambda build --release --arm64
```

**What this does**:
- Compiles Rust code for `aarch64-unknown-linux-gnu` target
- Creates `bootstrap` binaries (Lambda entry point)
- Outputs to `/target/lambda/auth/bootstrap` and `/target/lambda/game/bootstrap`
- Build time: ~1-2 minutes

**Common errors**:
- ❌ `error[E0463]: can't find crate for 'core'` → Target not installed
- ❌ Using `--target x86_64-unknown-linux-musl` → Wrong architecture! Use `--arm64`
- ✅ Successful build shows: `Finished 'release' profile [optimized] target(s)`

#### Deploy Infrastructure with CDK

```bash
# From infrastructure directory
cd /Users/christopherbrown/Projects/Zork/gruesome/infrastructure

# Deploy staging (uses separate CDK app file)
cdk deploy GruesomeBackendStackStaging --app "npx ts-node --prefer-ts-exts bin/gruesome-platform-staging.ts"

# Deploy production
cdk deploy GruesomeBackendStack

# To deploy all staging stacks at once
cdk deploy --all --app "npx ts-node --prefer-ts-exts bin/gruesome-platform-staging.ts"

# To deploy all production stacks at once
cdk deploy --all
```

**Important**: Staging uses a separate CDK app (`bin/gruesome-platform-staging.ts`), production uses the default (`bin/gruesome-platform.ts`).

**What this does**:
- Packages Lambda binaries from `../target/lambda/`
- Uploads to Lambda service
- Updates API Gateway configuration
- Updates environment variables
- Typical deployment time: 2-5 minutes

**Stacks** (as of December 19, 2025):

**Production**:
- `GruesomeDnsImportStack` - Certificate and hosted zone imports (us-east-1)
- `GruesomeDataStack` - DynamoDB table and S3 buckets (us-west-1)
- `GruesomeAuthStack` - Cognito user pool (us-west-1)
- `GruesomeBackendStack` - Lambda functions and API Gateway (us-west-1)
- `GruesomeFrontendStack` - S3 website and CloudFront (us-east-1)

**Staging**:
- `GruesomeDnsImportStackStaging` - Certificate and hosted zone imports (us-east-1)
- `GruesomeDataStackStaging` - DynamoDB table and S3 buckets (us-west-1)
- `GruesomeAuthStackStaging` - Cognito user pool (us-west-1)
- `GruesomeBackendStackStaging` - Lambda functions and API Gateway (us-west-1)
- `GruesomeFrontendStackStaging` - S3 website and CloudFront (us-east-1)

**Note**: The ACM certificate (`arn:aws:acm:us-east-1:349145659387:certificate/33ae9627-b894-4edc-a480-201bc6e8b529`) is not managed by CloudFormation. It was created by the old GruesomeDnsStack and retained when that stack was deleted. The DnsImportStack stacks import it by ARN.

---

### 2. Frontend Deployment

#### Build WASM

```bash
# From project root
cd /Users/christopherbrown/Projects/Zork/gruesome

# Build WASM module
wasm-pack build --target web --out-dir frontend/pkg
```

**Output**:
- `frontend/pkg/gruesome_interpreter_bg.wasm`
- `frontend/pkg/gruesome_interpreter.js`

#### Deploy to S3 + CloudFront

```bash
# Deploy staging
./infrastructure/scripts/deploy-frontend.sh staging

# Deploy production
./infrastructure/scripts/deploy-frontend.sh prod
```

**What the script does**:
1. Uploads `frontend/` contents to S3 bucket
2. Sets correct MIME types
3. Creates CloudFront invalidation (cache busting)
4. Waits for invalidation to complete

**Known Issue**: CloudFront invalidation script has bug with distribution ID detection
- **Workaround**: Manually run invalidation after deployment:
  ```bash
  # Staging frontend distribution
  aws cloudfront create-invalidation --distribution-id E3VWHUOBR5D259 --paths "/*"

  # Production frontend distribution
  aws cloudfront create-invalidation --distribution-id E1M8DHMS3GCUDX --paths "/*"

  # Staging API distribution
  aws cloudfront create-invalidation --distribution-id E3VWHUOBR5D259 --paths "/*"

  # Or list all distributions to find IDs
  aws cloudfront list-distributions --query "DistributionList.Items[?contains(DomainName, 'gruesome')].{ID:Id,Domain:DomainName}" --output table
  ```

---

### 3. Game Management

#### Upload New Game

```bash
./infrastructure/scripts/upload-game.sh \
  <game_file> \
  <game_id> \
  "<title>" \
  "<author>" \
  "<description>" \
  [staging|prod]
```

**Example**:
```bash
./infrastructure/scripts/upload-game.sh \
  resources/test/zork1/DATA/ZORK1.DAT \
  zork1 \
  "Zork I: The Great Underground Empire" \
  "Marc Blank & Dave Lebling" \
  "The original classic adventure game. Explore the Great Underground Empire." \
  staging
```

**What it does**:
1. Auto-detects Z-Machine version from file header
2. Validates file is valid Z-Machine (version 1-8)
3. Calculates file size
4. Uploads to S3: `s3://gruesome-games[-staging]/games/{game_id}.z{version}`
5. Creates DynamoDB metadata with standardized format
6. Verifies both uploads succeeded

**Workflow**: Always upload to staging first, test, then upload to prod.

#### Verify Game Library Consistency

```bash
# Verify staging
./infrastructure/scripts/verify-game-library.sh staging

# Verify production
./infrastructure/scripts/verify-game-library.sh prod
```

**Checks**:
1. DynamoDB → S3 consistency (all metadata has files)
2. S3 → DynamoDB consistency (all files have metadata)
3. S3 key format consistency (all use `games/` prefix)

**Output**:
- ✓ Green checkmarks for passing checks
- ✗ Red X for errors
- ⚠ Yellow warnings for size mismatches

---

## DynamoDB Schema

### Table: `gruesome-platform` (prod) / `gruesome-platform-staging` (staging)

**Single-Table Design**:

| PK (Partition Key) | SK (Sort Key) | Entity Type | Description |
|-------------------|---------------|-------------|-------------|
| `GAME#{game_id}` | `METADATA` | GAME | Game metadata (title, author, version, S3 key) |
| `USER#{user_id}` | `PROFILE` | USER | User profile (username, email, OAuth provider) |
| `USER#{user_id}` | `SAVE#{game_id}#{save_name}` | SAVE | Save file metadata (S3 key, file size, timestamps) |

**GSI (Global Secondary Index)**:
- **GSI1PK**: `email` (for login by email)
- **GSI1SK**: `USER#{user_id}`

**Example Items**:

```json
// Game metadata
{
  "PK": "GAME#zork1",
  "SK": "METADATA",
  "entity_type": "GAME",
  "game_id": "zork1",
  "title": "Zork I: The Great Underground Empire",
  "author": "Marc Blank & Dave Lebling",
  "description": "The original classic adventure game",
  "version": 3,
  "file_size": 84290,
  "s3_key": "games/zork1.z3",
  "created_at": 1734567890
}

// Save file metadata
{
  "PK": "USER#89e959fe-3011-70af-606e-d0a4158910bb",
  "SK": "SAVE#zork1#kitchen",
  "entity_type": "SAVE",
  "user_id": "89e959fe-3011-70af-606e-d0a4158910bb",
  "game_id": "zork1",
  "save_name": "kitchen",
  "s3_key": "89e959fe-3011-70af-606e-d0a4158910bb/zork1/kitchen.sav",
  "file_size": 12345,
  "created_at": 1734567890,
  "last_updated": 1734567890
}
```

---

## S3 Buckets

### Game Files

**Production**: `gruesome-games`
**Staging**: `gruesome-games-staging`

**Structure**:
```
s3://gruesome-games/
└── games/
    ├── zork1.z3
    ├── zork2.z3
    ├── zork3.z3
    ├── hhgg.z3
    ├── planetfall.z3
    └── wishbringer.z3
```

**Key Format**: MUST always be `games/{game_id}.z{version}`
- ✅ Correct: `games/zork1.z3`
- ❌ Wrong: `zork1.z3` (missing `games/` prefix)

### Save Files

**Production**: `gruesome-games` (same bucket)
**Staging**: `gruesome-games-staging` (same bucket)

**Structure**:
```
s3://gruesome-games/
└── {user_id}/
    └── {game_id}/
        └── {save_name}.sav
```

**Example**: `s3://gruesome-games/89e959fe-3011-70af-606e-d0a4158910bb/zork1/kitchen.sav`

---

## Cognito User Pools

### Production
- **Pool ID**: `us-west-1_zSPJeB5x0`
- **Pool Name**: `gruesome-users`
- **Client ID**: (configured in CDK)

### Staging
- **Pool ID**: `us-west-1_1Z4s05N00`
- **Pool Name**: `gruesome-users`
- **Client ID**: (configured in CDK)

### Password Policy
- Minimum length: 8 characters
- Requires uppercase letters
- Requires lowercase letters
- Requires numbers
- No symbols required

---

## Environment Variables

Lambda functions use these environment variables (configured in CDK):

### Auth Lambda
```
TABLE_NAME=gruesome-platform[-staging]
USER_POOL_ID=us-west-1_xxxxxxxxx
USER_POOL_CLIENT_ID=xxxxxxxxxxxxxxxxxxxxxxxxxx
```

### Game Lambda
```
TABLE_NAME=gruesome-platform[-staging]
GAMES_BUCKET=gruesome-games[-staging]
SAVES_BUCKET=gruesome-games[-staging]
USER_POOL_ID=us-west-1_xxxxxxxxx
```

---

## Common Deployment Scenarios

### Scenario 1: Fix Lambda Code Bug

```bash
# 1. Make code changes in /auth or /game
# 2. Build Lambda functions
cargo lambda build --release --arm64

# 3. Deploy to staging
cd infrastructure
cdk deploy GruesomePlatformStack-Staging

# 4. Test in staging
# Visit https://staging.gruesome.skeptomai.com

# 5. Deploy to production
cdk deploy GruesomePlatformStack
```

### Scenario 2: Update Frontend Only

```bash
# 1. Make changes in /frontend
# 2. Rebuild WASM if needed
wasm-pack build --target web --out-dir frontend/pkg

# 3. Deploy to staging
./infrastructure/scripts/deploy-frontend.sh staging

# 4. Test in staging
# Visit https://staging.gruesome.skeptomai.com

# 5. Deploy to production
./infrastructure/scripts/deploy-frontend.sh prod
```

### Scenario 3: Add New Game

```bash
# 1. Upload to staging
./infrastructure/scripts/upload-game.sh \
  path/to/game.z3 \
  game_id \
  "Game Title" \
  "Author Name" \
  "Description" \
  staging

# 2. Verify staging
./infrastructure/scripts/verify-game-library.sh staging

# 3. Test in staging browser
# Visit https://staging.gruesome.skeptomai.com

# 4. Upload to production
./infrastructure/scripts/upload-game.sh \
  path/to/game.z3 \
  game_id \
  "Game Title" \
  "Author Name" \
  "Description" \
  prod

# 5. Verify production
./infrastructure/scripts/verify-game-library.sh prod
```

### Scenario 4: Full Stack Update

```bash
# 1. Build Lambda functions
cargo lambda build --release --arm64

# 2. Deploy backend to staging
cd infrastructure
cdk deploy GruesomePlatformStack-Staging

# 3. Deploy frontend to staging
./infrastructure/scripts/deploy-frontend.sh staging

# 4. Test thoroughly in staging
# Visit https://staging.gruesome.skeptomai.com

# 5. Deploy backend to production
cdk deploy GruesomePlatformStack

# 6. Deploy frontend to production
./infrastructure/scripts/deploy-frontend.sh prod
```

---

## Troubleshooting

### Lambda Build Fails

**Error**: `can't find crate for 'core'`
**Solution**: Wrong target. Use `cargo lambda build --arm64` not `--target x86_64-unknown-linux-musl`

**Error**: `target 'aarch64-unknown-linux-gnu' may not be installed`
**Solution**: cargo-lambda should handle this automatically. Verify cargo-lambda is installed: `which cargo-lambda`

### Frontend Deploy Fails

**Error**: CloudFront invalidation hangs or fails
**Solution**: Manually create invalidation:
```bash
aws cloudfront list-distributions
aws cloudfront create-invalidation --distribution-id E1XXXXXXXXXX --paths "/*"
```

### Game Upload Fails

**Error**: `Invalid Z-Machine version`
**Solution**: File is not a valid Z-Machine file or is corrupted

**Error**: S3 upload succeeds but DynamoDB fails
**Solution**: Metadata already exists. Delete old metadata first:
```bash
aws dynamodb delete-item \
  --table-name gruesome-platform \
  --key '{"PK": {"S": "GAME#game_id"}, "SK": {"S": "METADATA"}}'
```

### Save File Inconsistencies

**Symptom**: Save shows in list but won't load
**Cause**: Orphaned save - metadata exists but S3 file missing
**Detection**: Run `./infrastructure/scripts/verify-game-library.sh [env]`
**Fix**: Backend now validates S3 file existence before listing saves (as of Dec 19, 2025)

**Symptom**: Save upload shows success but file missing
**Cause**: Two-phase commit issue - metadata created before S3 upload, upload failed
**Prevention**: Frontend now validates S3 upload response (as of Dec 19, 2025)

---

## Known Issues

1. **CloudFront invalidation script bug** (infrastructure/scripts/deploy-frontend.sh)
   - Distribution ID detection failing
   - Workaround: Manual invalidation after deployment
   - Status: Not fixed (low priority)

2. **Two-phase commit in save upload** (backend Lambda)
   - DynamoDB metadata created before S3 upload
   - Can create orphaned metadata if S3 upload fails
   - Mitigation: S3 validation added to list/load endpoints (Dec 19, 2025)
   - Full fix would require architectural change (S3 upload first, then metadata)

3. **Enchanter game infinite loop** (WASM interpreter)
   - Game initialization exceeds 10,000 steps
   - Status: Game removed from platform (Dec 18, 2025)
   - Documented in ONGOING_TASKS.md for future debugging

---

## AWS Regions

All resources deployed in: **us-west-1** (N. California)

**Why us-west-1**:
- Close to user location (San Francisco Bay Area)
- Route 53 hosted zone already in this region
- Consistent with skeptomai.com domain

---

## Costs

**Current monthly costs** (~100 users):
- Route 53: $0.50 (hosted zone)
- S3 storage: $1.00 (frontend + game files + save files)
- CloudFront: $0 (free tier)
- Lambda: $0 (free tier)
- API Gateway: $0 (free tier)
- DynamoDB: $0 (free tier)
- **Total**: ~$1.50/month

**Projected costs** (1,000 users):
- Route 53: $0.50
- S3 storage: $5.00
- CloudFront: $0 (still within free tier)
- Lambda: $0 (still within free tier)
- API Gateway: $0 (still within free tier)
- DynamoDB: $0 (still within free tier)
- **Total**: ~$5.50/month

**Free tier details**:
- Lambda: 1M requests/month (permanent)
- API Gateway: 1M requests/month (first 12 months)
- DynamoDB: 25GB + 200M requests/month (permanent)
- CloudFront: 1TB transfer/month (first 12 months)

---

## Security

### IAM Permissions

**Lambda Execution Roles** (managed by CDK):
- Auth Lambda: DynamoDB read/write, Cognito admin
- Game Lambda: DynamoDB read/write, S3 read (games), S3 read/write (saves)

### API Authentication

**Public endpoints**:
- POST /api/auth/signup
- POST /api/auth/login
- POST /api/auth/refresh
- POST /api/auth/forgot-password
- POST /api/auth/confirm-forgot-password
- GET /health

**Protected endpoints** (require JWT in Authorization header):
- GET /api/auth/me
- GET /api/games
- GET /api/games/{game_id}
- GET /api/games/{game_id}/file
- GET /api/saves
- GET /api/saves/{game_id}
- GET /api/saves/{game_id}/{save_name}
- POST /api/saves/{game_id}/{save_name}
- DELETE /api/saves/{game_id}/{save_name}

### CORS Configuration

**Allowed Origins**:
- Production: `https://gruesome.skeptomai.com`
- Staging: `https://staging.gruesome.skeptomai.com`

**Allowed Methods**: GET, POST, DELETE
**Allowed Headers**: Authorization, Content-Type

---

## Monitoring

### CloudWatch Logs

**Lambda Functions**:
- Auth: `/aws/lambda/GruesomePlatformStack-AuthFunction`
- Game: `/aws/lambda/GruesomePlatformStack-GameFunction`

**Log Retention**: 7 days (configurable in CDK)

### CloudWatch Metrics

**Lambda**:
- Invocations
- Duration
- Errors
- Throttles

**API Gateway**:
- Request count
- 4xx errors
- 5xx errors
- Latency

**DynamoDB**:
- Read capacity units
- Write capacity units
- Throttled requests

---

## References

- [MULTI_USER_PLATFORM_ARCHITECTURE.md](active-work/MULTI_USER_PLATFORM_ARCHITECTURE.md) - Detailed architecture design
- [AWS_CDK_IMPLEMENTATION_PLAN.md](active-work/AWS_CDK_IMPLEMENTATION_PLAN.md) - CDK implementation details
- [README-GAME-MANAGEMENT.md](../infrastructure/scripts/README-GAME-MANAGEMENT.md) - Game management scripts
- [ONGOING_TASKS.md](../ONGOING_TASKS.md) - Current status and known issues
