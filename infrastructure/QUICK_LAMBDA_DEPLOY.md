# Quick Lambda Deployment Guide

**TL;DR: How to deploy Rust Lambda code changes**

## The Command

```bash
./scripts/deploy-lambda.sh [function] [environment]
```

Examples:
```bash
./scripts/deploy-lambda.sh game staging
./scripts/deploy-lambda.sh admin prod
./scripts/deploy-lambda.sh auth staging
```

## What It Does

1. ✅ Cleans old build artifacts (prevents using cached/old binaries)
2. ✅ Builds the Lambda function with `cargo lambda build`
3. ✅ Verifies build actually happened (warns if suspiciously fast)
4. ✅ Creates bootstrap.zip (the deployment package)
5. ✅ Looks up the correct Lambda function name from CloudFormation
6. ✅ Deploys to AWS Lambda
7. ✅ Tests the deployment (health check + API test)

## Why This Script Exists

**Previous process (manual, error-prone):**
```bash
cd lambda/gruesome-api
cargo lambda build --release --arm64 --bin game  # Might use cache!
cd target/lambda/game
zip bootstrap.zip bootstrap  # Might forget this step!
aws lambda update-function-code --function-name ... --zip-file ...  # Long command
# No verification that it worked!
```

**What went wrong on 2025-12-20:**
- Modified game Lambda code
- Ran `cargo lambda build` (but it used cached build from Dec 16)
- Deployed OLD bootstrap.zip without display_order changes
- User saw: "Failed to load games: Cannot read properties of undefined"
- Had to debug, rebuild properly, and redeploy

**New process (automated, verified):**
```bash
./scripts/deploy-lambda.sh game staging
# Cleans, builds, verifies, deploys, tests - all in one command
```

## When to Use

Use this script when:
- ✅ You modified Rust code in `lambda/gruesome-api/`
- ✅ You want to deploy code-only changes (no infrastructure)
- ✅ You want quick deployment without full CDK deploy

Do NOT use for:
- ❌ Infrastructure changes (use CDK)
- ❌ Environment variable changes (use CDK)
- ❌ Lambda configuration changes (memory, timeout, etc. - use CDK)

## Common Scenarios

### I changed the game Lambda code

```bash
# Edit game/src/*.rs files
./scripts/deploy-lambda.sh game staging  # Test in staging
# Verify it works
./scripts/deploy-lambda.sh game prod     # Deploy to production
```

### I changed the admin Lambda code

```bash
# Edit admin/src/*.rs files
./scripts/deploy-lambda.sh admin staging
./scripts/deploy-lambda.sh admin prod
```

### Build was cached (finished too quickly)

The script will warn you:
```
⚠ Warning: Build finished very quickly (0.11 s)
  This might indicate a cached build.
Continue anyway? (y/N)
```

If you see this, type `N` and:
```bash
cd lambda/gruesome-api
touch {function}/src/main.rs  # Force rebuild
cd ../..
./scripts/deploy-lambda.sh {function} {env}
```

## Verification

The script automatically tests deployment:
- Health check: `curl https://api-{env}.gruesome.skeptomai.com/health`
- For game function: Also tests `/api/games` endpoint

**Always check the output!** Don't assume it worked.

## Full Documentation

See `LAMBDA_DEPLOYMENT.md` for complete details about:
- What bootstrap.zip is and why it's needed
- How Rust Lambda deployment works
- CDK vs direct deployment
- Common mistakes and how to avoid them
- Verification checklist

## Emergency: Something's Broken

If deployment fails or breaks production:

1. **Check Lambda logs:**
   ```bash
   aws logs tail /aws/lambda/{function-name} --since 30m
   ```

2. **Rollback by redeploying previous version:**
   ```bash
   # Get previous version from git
   git checkout HEAD~1 lambda/gruesome-api/{function}/
   ./scripts/deploy-lambda.sh {function} {env}
   # Then restore current version
   git checkout HEAD lambda/gruesome-api/{function}/
   ```

3. **Use CDK to redeploy known-good version:**
   ```bash
   cdk deploy GruesomeBackendStack{Staging}
   ```

## References

- `LAMBDA_DEPLOYMENT.md` - Complete deployment documentation
- `INFRASTRUCTURE_MAPPING.md` - All CloudFront/Lambda/API mappings
- `scripts/deploy-lambda.sh` - The deployment script itself
