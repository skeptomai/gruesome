# Gruesome Platform Deployment Architecture

**Last Updated:** January 29, 2026
**Status:** Production

## Table of Contents

1. [Overview](#overview)
2. [Deployment Layers](#deployment-layers)
3. [Frontend Deployment](#frontend-deployment)
4. [Backend Deployment](#backend-deployment)
5. [Infrastructure Deployment](#infrastructure-deployment)
6. [Data Persistence](#data-persistence)
7. [Deployment Decision Matrix](#deployment-decision-matrix)
8. [Safety Guarantees](#safety-guarantees)

---

## Overview

The Gruesome platform consists of three distinct deployment layers, each with different update mechanisms and impact profiles. Understanding these layers is critical for safe deployments.

### Architecture Layers

```
┌─────────────────────────────────────────────────────────────┐
│ Layer 1: Frontend (Static Files)                           │
│ ├─ HTML, CSS, JavaScript, WASM                            │
│ ├─ Stored in: S3                                          │
│ ├─ Served via: CloudFront CDN                             │
│ └─ Impact: UI/UX only, no backend changes                 │
├─────────────────────────────────────────────────────────────┤
│ Layer 2: Backend Lambda (API Code)                         │
│ ├─ Rust Lambda functions                                  │
│ ├─ APIs: game, auth, admin                                │
│ ├─ Stored in: AWS Lambda                                  │
│ └─ Impact: API behavior, no database structure changes    │
├─────────────────────────────────────────────────────────────┤
│ Layer 3: Infrastructure (AWS Resources)                    │
│ ├─ DynamoDB tables, Lambda configs, API Gateway           │
│ ├─ Managed by: AWS CDK                                    │
│ ├─ Stored in: CloudFormation stacks                       │
│ └─ Impact: Can modify database schema, resource configs   │
└─────────────────────────────────────────────────────────────┘
```

---

## Deployment Layers

### Layer 1: Frontend (Static Files)

**What it includes:**
- HTML, CSS, JavaScript files
- WASM binaries (Gruesome interpreter)
- Static assets and images
- Build version watermarks

**Deployment mechanism:**
- Script: `infrastructure/scripts/deploy-frontend.sh`
- Target: S3 bucket + CloudFront CDN
- Trigger: Manual execution or CI/CD

**Impact:**
- ✅ Users see new UI immediately (after cache clears)
- ✅ No API changes
- ✅ No database changes
- ✅ No user data affected
- ✅ Existing sessions preserved

### Layer 2: Backend Lambda (API Code)

**What it includes:**
- Rust Lambda function binaries
- API endpoint logic
- Authentication middleware
- Game state management code

**Deployment mechanism:**
- Script: `infrastructure/scripts/deploy-lambda.sh`
- Target: AWS Lambda functions
- Trigger: Manual execution after code changes

**Impact:**
- ✅ API behavior may change
- ✅ Existing requests continue working
- ✅ No database structure changes
- ✅ All user data preserved
- ✅ Sessions preserved (JWT secret unchanged)

### Layer 3: Infrastructure (AWS Resources)

**What it includes:**
- DynamoDB table definitions
- Lambda function configurations
- API Gateway routes
- S3 bucket policies
- CloudFront distributions

**Deployment mechanism:**
- Tool: AWS CDK (`npm run cdk deploy`)
- Target: CloudFormation stacks
- Trigger: Manual execution after infrastructure code changes

**Impact:**
- ⚠️ Can modify database schema
- ⚠️ May require data migrations
- ✅ Existing data preserved (RETAIN policy)
- ⚠️ May affect API endpoints
- ⚠️ May require application restarts

---

## Frontend Deployment

### When to Deploy Frontend

**Deploy when:**
- ✅ UI/UX changes
- ✅ WASM interpreter updates
- ✅ Documentation version updates
- ✅ Static content changes
- ✅ Style/layout modifications

**Don't deploy when:**
- ❌ Only backend API logic changed
- ❌ Only infrastructure changed
- ❌ Only database schema changed

### Frontend Deployment Process

#### Staging Deployment
```bash
cd infrastructure
./scripts/deploy-frontend.sh staging
```

**What happens:**
1. Reads `frontend/index.html`, `app.js`, `style.css`, WASM files
2. Injects build version watermark (commit hash + timestamp)
3. Uploads files to `gruesome-frontend-staging` S3 bucket
4. Creates CloudFront invalidation for cache refresh
5. Displays staging URL: https://staging.gruesome.skeptomai.com

**Verification:**
```bash
# Check watermark in browser console or page source
curl https://staging.gruesome.skeptomai.com | grep "Build:"
# Should show: Build: <commit-hash> @ <timestamp>
```

#### Production Deployment
```bash
cd infrastructure
./scripts/deploy-frontend.sh prod
```

**Safety prompt:**
```
⚠️  WARNING: You are about to deploy to PRODUCTION ⚠️
Type 'DEPLOY TO PRODUCTION' to continue:
```

**What happens:**
1. Same as staging, but deploys to production bucket
2. Injects release version watermark (`v2.16.3`)
3. Uploads to `gruesome-frontend` S3 bucket
4. Creates CloudFront invalidation
5. Displays production URL: https://gruesome.skeptomai.com

**Verification:**
```bash
# Check watermark shows release version
curl https://gruesome.skeptomai.com | grep "Build:"
# Should show: Build: v2.16.3
```

### Frontend Deployment Impact

**What is affected:**
- ✅ Static files in S3 (overwritten)
- ✅ CloudFront cache (invalidated)
- ✅ User browser cache (refreshed on next visit)

**What is NOT affected:**
- ✅ DynamoDB tables (users, saves, game-metadata)
- ✅ Lambda functions (API code unchanged)
- ✅ S3 game/save buckets (user data preserved)
- ✅ Cognito user pool (authentication unchanged)
- ✅ Active user sessions (JWT tokens still valid)
- ✅ Saved games (all saves preserved)

---

## Backend Deployment

### When to Deploy Backend

**Deploy when:**
- ✅ API endpoint logic changes
- ✅ Lambda function code updates
- ✅ Authentication middleware changes
- ✅ Game state management updates
- ✅ Bug fixes in backend code

**Don't deploy when:**
- ❌ Only frontend UI changed
- ❌ Only documentation changed
- ❌ Only infrastructure schema changed (use CDK instead)

### Backend Deployment Process

#### Available APIs
- `game` - Game state, save/restore operations
- `auth` - User authentication, JWT management
- `admin` - Admin operations, game library management

#### Staging Deployment
```bash
cd infrastructure
./scripts/deploy-lambda.sh game staging
```

**What happens:**
1. Builds Rust Lambda function in `infrastructure/lambda/gruesome-api/game/`
2. Creates optimized release binary
3. Packages as `bootstrap.zip` (Lambda custom runtime)
4. Uploads to S3 lambda-deployments bucket
5. Updates Lambda function code via AWS CLI
6. Tests API endpoint with health check
7. Reports success or failure with logs

**Verification:**
```bash
# Test endpoint directly
curl https://api-staging.gruesome.skeptomai.com/game/health

# Or use test script
./scripts/test-game-lambda.sh staging
```

#### Production Deployment
```bash
cd infrastructure
./scripts/deploy-lambda.sh game prod
```

**Safety checks:**
- ✅ Requires staging to be deployed first
- ✅ Automated health checks before completion
- ✅ Rollback available via AWS Lambda versioning

**What happens:**
1. Same as staging, but deploys to production Lambda
2. Updates production API Gateway integration
3. Runs comprehensive health checks
4. Validates against production DynamoDB tables
5. Reports deployment status with metrics

**Verification:**
```bash
# Test production endpoint
curl https://api.gruesome.skeptomai.com/game/health

# Check Lambda logs
aws logs tail /aws/lambda/gruesome-game-api-prod --follow
```

### Backend Deployment Impact

**What is affected:**
- ✅ Lambda function code (updated)
- ✅ API endpoint behavior (may change)
- ✅ API response formats (may change)
- ✅ Error handling logic (may improve)

**What is NOT affected:**
- ✅ DynamoDB table structure (schema unchanged)
- ✅ DynamoDB data (all records preserved)
- ✅ S3 buckets (game/save files intact)
- ✅ Frontend files (static files unchanged)
- ✅ User sessions (JWT tokens valid if secret unchanged)
- ✅ Saved games (all saves preserved)

### Lambda Deployment Architecture

**Stateless Functions:**
- Lambda functions have no persistent state
- All state stored in DynamoDB or S3
- Functions can be redeployed without data loss
- Cold starts may occur after deployment (~2-3s)

**Database Interaction:**
- Lambdas READ/WRITE to existing DynamoDB tables
- They never CREATE/ALTER/DROP tables
- Database schema managed by CDK infrastructure layer
- Connection pooling handled by AWS SDK

---

## Infrastructure Deployment

### When to Deploy Infrastructure

**Deploy when:**
- ✅ Database schema changes
- ✅ New DynamoDB tables needed
- ✅ Lambda configuration changes (memory, timeout)
- ✅ API Gateway route modifications
- ✅ S3 bucket policy updates
- ✅ CloudFront distribution changes

**Don't deploy when:**
- ❌ Only frontend code changed (use deploy-frontend.sh)
- ❌ Only Lambda code changed (use deploy-lambda.sh)
- ❌ Only documentation changed

### Infrastructure Deployment Process

#### CDK Deployment
```bash
cd infrastructure

# Deploy to staging
npm run cdk deploy GruesomePlatformStack-staging

# Deploy to production (requires approval)
npm run cdk deploy GruesomePlatformStack-prod
```

**What happens:**
1. CDK synthesizes CloudFormation template
2. Compares with deployed stack (shows diff)
3. Requests approval for changes
4. Deploys changes to AWS CloudFormation
5. Updates resources (DynamoDB, Lambda configs, API Gateway)
6. Reports deployment status

**Safety checks:**
- ⚠️ Shows resource changes before deployment
- ⚠️ Requires manual approval for production
- ⚠️ Database tables protected with RETAIN policy
- ⚠️ Rollback available via CloudFormation

### Infrastructure Deployment Impact

**What is affected:**
- ⚠️ DynamoDB table structure (if schema changed)
- ⚠️ Lambda configurations (memory, timeout, env vars)
- ⚠️ API Gateway routes (endpoints may change)
- ⚠️ S3 bucket policies (permissions may change)
- ⚠️ CloudFront distributions (CDN behavior may change)

**What is PROTECTED:**
- ✅ DynamoDB data (RETAIN policy prevents deletion)
- ✅ S3 bucket contents (unless explicitly deleted)
- ✅ User sessions (preserved if auth unchanged)
- ✅ Saved games (preserved in S3)

### Database Schema Changes

**When CDK modifies tables:**
```typescript
// infrastructure/lib/data-stack.ts
new dynamodb.Table(this, 'UsersTable', {
  // Adding new attribute (safe - NoSQL schema)
  // Changing partition key (DANGEROUS - requires migration)
  removalPolicy: RemovalPolicy.RETAIN, // Protects data
});
```

**Safe changes:**
- ✅ Adding new attributes (DynamoDB is schemaless)
- ✅ Adding new GSI (Global Secondary Index)
- ✅ Changing table capacity (auto-scaling)
- ✅ Updating access policies

**Dangerous changes (require migration):**
- ⚠️ Changing partition key
- ⚠️ Changing sort key
- ⚠️ Removing table (protected by RETAIN policy)

---

## Data Persistence

### Database Persistence Across Deployments

#### Frontend Deployments
- **Database:** ✅ Completely untouched
- **User sessions:** ✅ Preserved (JWT tokens still valid)
- **Saved games:** ✅ Intact in S3
- **Game metadata:** ✅ Unchanged in DynamoDB

#### Backend Lambda Deployments
- **Database tables:** ✅ Untouched (managed by infrastructure)
- **Database data:** ✅ Fully preserved
- **User sessions:** ✅ Preserved (if JWT secret unchanged)
- **API endpoints:** Updated with new logic
- **Saved games:** ✅ Preserved in S3

#### Infrastructure Deployments
- **Database tables:** May be modified (with RETAIN protection)
- **Existing data:** ✅ Preserved (RETAIN policy)
- **Table structure:** May change (requires testing)
- **S3 buckets:** Protected by deletion policies
- **Saved games:** ✅ Preserved (unless bucket deleted)

### DynamoDB RETAIN Policy

**What it means:**
```typescript
removalPolicy: RemovalPolicy.RETAIN
```

- ✅ If CDK stack is destroyed, tables remain
- ✅ Data survives accidental stack deletions
- ✅ Requires explicit manual deletion to lose data
- ✅ Protects against configuration errors

**Result:** Your user database and saved games survive all deployments unless explicitly deleted.

---

## Deployment Decision Matrix

### What Changed? → What to Deploy

| Changed Files | Frontend | Backend | Infrastructure |
|---------------|----------|---------|----------------|
| `frontend/*.html`, `frontend/*.js`, `frontend/*.css`, `frontend/*.wasm` | ✅ Deploy | ❌ Skip | ❌ Skip |
| `infrastructure/lambda/gruesome-api/*/src/*.rs` | ❌ Skip | ✅ Deploy | ❌ Skip |
| `infrastructure/lib/*.ts` | ❌ Skip | ❌ Skip | ✅ Deploy |
| `docs/*.md` | ✅ Deploy (version) | ❌ Skip | ❌ Skip |
| `src/interpreter/*.rs`, `src/grue_compiler/*.rs` | ✅ Deploy (WASM) | ❌ Skip | ❌ Skip |
| `Cargo.toml` (version bump) | ✅ Deploy | ❌ Skip | ❌ Skip |

### Common Scenarios

#### Scenario 1: Documentation Update
**Changed:** `docs/COMPILER_ARCHITECTURE.md`
**Deploy:** Frontend only (version watermark)
**Impact:** UI shows new version, no functional changes

#### Scenario 2: Interpreter Bug Fix
**Changed:** `src/interpreter/core/vm.rs`
**Deploy:** Frontend only (rebuild WASM)
**Impact:** Game execution logic updated, saved games compatible

#### Scenario 3: API Endpoint Enhancement
**Changed:** `infrastructure/lambda/gruesome-api/game/src/handlers.rs`
**Deploy:** Backend Lambda only
**Impact:** API behavior changes, frontend still works

#### Scenario 4: New Database Table
**Changed:** `infrastructure/lib/data-stack.ts`
**Deploy:** Infrastructure (CDK)
**Impact:** New table created, existing data preserved

#### Scenario 5: Full Stack Feature
**Changed:** Frontend, Backend, and Infrastructure
**Deploy Order:**
1. Infrastructure first (CDK) - create resources
2. Backend Lambda second - API uses new resources
3. Frontend last - UI uses new API

---

## Safety Guarantees

### Frontend Deployment Safety
- ✅ **Stateless:** Only updates static files
- ✅ **Instant rollback:** Re-deploy previous version
- ✅ **No data loss:** Database never touched
- ✅ **Cache control:** CloudFront invalidation ensures fresh content
- ✅ **Session safety:** JWT tokens remain valid

### Backend Deployment Safety
- ✅ **Stateless functions:** No persistent state in Lambda
- ✅ **Versioning:** Lambda keeps previous versions
- ✅ **Rollback:** Deploy previous version instantly
- ✅ **Data safety:** Database tables unchanged
- ✅ **Health checks:** Automated validation after deployment

### Infrastructure Deployment Safety
- ✅ **RETAIN policy:** Tables survive stack deletion
- ✅ **Change preview:** Shows diff before deployment
- ✅ **Manual approval:** Production requires confirmation
- ✅ **Rollback:** CloudFormation stack rollback available
- ⚠️ **Schema changes:** Require testing and validation

### User Data Protection

**Protected across ALL deployments:**
- ✅ DynamoDB tables (users, saves, game-metadata)
- ✅ S3 game files bucket
- ✅ S3 save files bucket
- ✅ Cognito user pool (authentication)
- ✅ JWT tokens (sessions)

**What could cause data loss:**
- ❌ Explicit table deletion (requires manual confirmation)
- ❌ Bucket policy misconfiguration (requires manual change)
- ❌ Accidental data deletion via admin API (requires authentication)
- ❌ Manual AWS console operations (requires credentials)

**Protection mechanisms:**
- ✅ RETAIN policies on all stateful resources
- ✅ No destructive operations in deployment scripts
- ✅ Multi-factor authentication for AWS console
- ✅ Backup policies (if configured)

---

## Deployment Workflow Examples

### Example 1: Documentation Release (v2.16.3)

**What changed:**
- `docs/COMPILER_ARCHITECTURE.md` (new comprehensive guide)
- `Cargo.toml` (version bump)

**Deployment steps:**
1. ✅ Commit and push changes
2. ✅ Create release tag `v2.16.3`
3. ✅ GitHub Actions builds binaries
4. ✅ Deploy frontend to staging
5. ✅ Verify staging watermark
6. ✅ Deploy frontend to production
7. ✅ Verify production watermark

**Impact:**
- Users see version `v2.16.3` in UI
- No functional changes
- All user data preserved

### Example 2: API Bug Fix

**What changed:**
- `infrastructure/lambda/gruesome-api/game/src/handlers.rs`
- Fixed save file validation logic

**Deployment steps:**
1. ✅ Test changes locally
2. ✅ Deploy Lambda to staging
3. ✅ Test staging API endpoints
4. ✅ Deploy Lambda to production
5. ✅ Verify production health checks

**Impact:**
- Save file validation improved
- All existing saves still work
- No database changes

### Example 3: New Feature (Game Library)

**What changed:**
- Infrastructure: New DynamoDB table `game-library`
- Backend: New admin API endpoints
- Frontend: New game library UI

**Deployment steps:**
1. ✅ Deploy infrastructure (CDK) - creates table
2. ✅ Wait for CloudFormation completion
3. ✅ Deploy backend Lambda - uses new table
4. ✅ Test admin API endpoints
5. ✅ Deploy frontend - shows new UI
6. ✅ Verify end-to-end functionality

**Impact:**
- New game library feature available
- Existing games and saves preserved
- Old clients still work (graceful degradation)

---

## Troubleshooting

### Frontend Deployment Issues

**Problem:** Watermark not updating
```bash
# Clear CloudFront cache
aws cloudfront create-invalidation \
  --distribution-id E36HKKVL2VZOZD \
  --paths "/*"

# Wait 1-3 minutes, then check
curl https://gruesome.skeptomai.com | grep "Build:"
```

**Problem:** Old content still showing
- Clear browser cache (Cmd+Shift+R / Ctrl+F5)
- Check CloudFront invalidation status
- Verify S3 bucket has new files

### Backend Deployment Issues

**Problem:** Lambda function not updating
```bash
# Check Lambda version
aws lambda get-function --function-name gruesome-game-api-prod

# Verify deployment
./scripts/test-game-lambda.sh prod
```

**Problem:** API errors after deployment
```bash
# Check Lambda logs
aws logs tail /aws/lambda/gruesome-game-api-prod --follow --since 5m

# Rollback to previous version if needed
aws lambda update-function-code \
  --function-name gruesome-game-api-prod \
  --s3-bucket gruesome-lambda-deployments \
  --s3-key game-prod-previous.zip
```

### Infrastructure Deployment Issues

**Problem:** CDK deployment fails
```bash
# Check CloudFormation events
aws cloudformation describe-stack-events \
  --stack-name GruesomePlatformStack-prod \
  --max-items 20

# Rollback if needed
npm run cdk deploy --rollback
```

**Problem:** Table not accessible after deployment
- Check IAM permissions
- Verify Lambda execution role
- Check table provisioned capacity

---

## Related Documentation

- **Frontend Deployment Details:** `infrastructure/scripts/deploy-frontend.sh`
- **Backend Deployment Guide:** `infrastructure/LAMBDA_DEPLOYMENT.md`
- **Quick Lambda Deploy:** `infrastructure/QUICK_LAMBDA_DEPLOY.md`
- **AWS Deployment Guide:** `docs/AWS_DEPLOYMENT_GUIDE.md`
- **Infrastructure Mapping:** `infrastructure/INFRASTRUCTURE_MAPPING.md`

---

## Summary

### Key Takeaways

1. **Three Layers:** Frontend (static), Backend (Lambda), Infrastructure (CDK)
2. **Data Safety:** User data survives ALL deployment types
3. **Deployment Independence:** Each layer can be deployed separately
4. **RETAIN Policy:** Database tables protected from accidental deletion
5. **Rollback Available:** All layers support rolling back changes

### Best Practices

- ✅ Deploy to staging first, then production
- ✅ Verify changes in staging before production deployment
- ✅ Use deployment scripts (don't manually upload files)
- ✅ Check logs after deployment
- ✅ Test critical paths after production deployment
- ✅ Keep deployment commands in automation (CLAUDE.md)

### Never Do This

- ❌ Manually modify DynamoDB tables in production
- ❌ Deploy infrastructure changes without testing in staging
- ❌ Skip health checks after backend deployments
- ❌ Force delete CloudFormation stacks with data
- ❌ Modify S3 bucket policies without backup

---

**Document Version:** 1.0
**Last Updated:** January 29, 2026
**Maintainer:** Sparky, Pancho (Claude Sonnet 4.5)
