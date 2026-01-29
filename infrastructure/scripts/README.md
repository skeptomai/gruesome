# Infrastructure Scripts

This directory contains scripts for verifying and testing the Gruesome game platform infrastructure.

## Scripts

### `verify-infrastructure.sh`

Verifies that all AWS infrastructure components are properly deployed and configured.

**Checks:**
- CloudFormation stacks status
- DynamoDB table existence and configuration (including TTL)
- S3 buckets and object counts
- Cognito User Pool and user counts
- Lambda functions (Auth, Game, and Admin API)
- API Gateway routes (including admin routes)
- CloudFront distribution (optional)
- **Admin API endpoints (optional with credentials)**

**Basic Usage:**
```bash
# Verify production environment (default)
./scripts/verify-infrastructure.sh

# Verify production explicitly
./scripts/verify-infrastructure.sh production

# Verify staging environment
./scripts/verify-infrastructure.sh staging
```

**With Admin API Tests:**
```bash
# Production with admin endpoint tests
TEST_ADMIN_USERNAME=admin TEST_ADMIN_PASSWORD=pass ./scripts/verify-infrastructure.sh

# Staging with admin tests
TEST_ADMIN_USERNAME=admin TEST_ADMIN_PASSWORD=pass ./scripts/verify-infrastructure.sh staging

# With non-admin user for authorization testing
TEST_ADMIN_USERNAME=admin TEST_ADMIN_PASSWORD=adminpass \
TEST_USER_USERNAME=user TEST_USER_PASSWORD=userpass \
./scripts/verify-infrastructure.sh staging
```

**Arguments:**
- First argument: `production` or `staging` (default: `production`)

**Environment Variables:**
- `TEST_ADMIN_USERNAME` - Admin username for API testing (optional)
- `TEST_ADMIN_PASSWORD` - Admin password for API testing (optional)
- `TEST_USER_USERNAME` - Non-admin username for authorization testing (optional)
- `TEST_USER_PASSWORD` - Non-admin password for authorization testing (optional)

**Requirements:**
- AWS CLI configured with valid credentials
- `jq` installed (`brew install jq`)
- Appropriate IAM permissions to describe resources

### `test-admin-api.sh`

**NEW:** Dedicated script for testing admin API endpoints with detailed output and step-by-step verification.

**Tests:**
1. Admin authentication (POST /api/auth/login)
2. List all games (GET /api/admin/games)
3. Generate presigned upload URL (POST /api/admin/games/upload-url)
4. Get specific game metadata (GET /api/admin/games/{id})
5. Authorization check with non-admin user (expects 403)

**Usage:**
```bash
# Test production admin API (default)
TEST_ADMIN_USERNAME=admin TEST_ADMIN_PASSWORD=pass ./scripts/test-admin-api.sh

# Test production explicitly
TEST_ADMIN_USERNAME=admin TEST_ADMIN_PASSWORD=pass ./scripts/test-admin-api.sh production

# Test staging admin API
TEST_ADMIN_USERNAME=admin TEST_ADMIN_PASSWORD=pass ./scripts/test-admin-api.sh staging

# Test with non-admin user for authorization checks
TEST_ADMIN_USERNAME=admin TEST_ADMIN_PASSWORD=adminpass \
TEST_USER_USERNAME=user TEST_USER_PASSWORD=userpass \
./scripts/test-admin-api.sh staging
```

**Arguments:**
- First argument: `production` or `staging` (default: `production`)

**Environment Variables:**
- `TEST_ADMIN_USERNAME` - Admin username (required)
- `TEST_ADMIN_PASSWORD` - Admin password (required)
- `TEST_USER_USERNAME` - Non-admin username for auth testing (optional)
- `TEST_USER_PASSWORD` - Non-admin password for auth testing (optional)

**Note:** The following endpoints require test data creation and are not automatically tested:
- PUT /api/admin/games/{id} (update game metadata)
- POST /api/admin/games (create new game metadata)
- DELETE /api/admin/games/{id} (soft delete game)

### `test-game-lambda.sh`

Runs end-to-end tests for the game-playing Lambda function.

**Tests:**
1. JWT authentication
2. Start new game (creates DynamoDB session)
3. Execute game command
4. Session persistence across multiple commands
5. Error handling (invalid session)
6. Session resume functionality
7. Health check endpoint

**Usage:**
```bash
./scripts/test-game-lambda.sh
```

**Environment Variables:**
- `API_URL` - API Gateway endpoint (default: production URL)
- `TEST_USERNAME` - Cognito username (default: "bob")
- `TEST_PASSWORD` - User password (default: "BobPassword123")
- `TEST_GAME_ID` - Game to test (default: "mini-zork")

**Example:**
```bash
# Test against production
./scripts/test-game-lambda.sh

# Test against custom endpoint
API_URL=https://custom-api.example.com ./scripts/test-game-lambda.sh
```

## Typical Workflow

### After Deploying Infrastructure

1. **Verify infrastructure is deployed correctly:**
   ```bash
   ./scripts/verify-infrastructure.sh
   ```

2. **Test admin API (if you have admin credentials):**
   ```bash
   TEST_ADMIN_USERNAME=admin TEST_ADMIN_PASSWORD=pass ./scripts/test-admin-api.sh
   ```

3. **Run end-to-end game tests:**
   ```bash
   ./scripts/test-game-lambda.sh
   ```

4. **Check logs if tests fail:**
   ```bash
   aws logs tail /aws/lambda/GruesomeBackendStack-GameFunction* --follow
   ```

### Staging Environment Testing

Before deploying to production, verify staging works:

```bash
# 1. Verify staging infrastructure
./scripts/verify-infrastructure.sh staging

# 2. Test staging admin API
TEST_ADMIN_USERNAME=admin TEST_ADMIN_PASSWORD=pass ./scripts/test-admin-api.sh staging

# 3. Test staging game functionality
API_URL=https://api-staging.gruesome.skeptomai.com ./scripts/test-game-lambda.sh

# 4. If everything passes, deploy to production
npx cdk deploy --all
```

## Exit Codes

Both scripts use standard exit codes:
- `0` - All checks/tests passed
- `1` - One or more checks/tests failed

## Output

Scripts use colored output:
- ✓ Green - Passed
- ✗ Red - Failed
- ⚠ Yellow - Warning (non-critical)
- → Blue - Info

## Troubleshooting

### Infrastructure verification fails

**Issue:** CloudFormation stacks not found
- **Solution:** Deploy stacks with `npx cdk deploy --all`

**Issue:** Lambda functions not found
- **Solution:** Build and deploy Lambda functions:
  ```bash
  cd lambda/gruesome-api
  cargo lambda build --release --arm64 --package auth --package game
  npx cdk deploy GruesomeBackendStack
  ```

**Issue:** DynamoDB table exists but TTL disabled
- **Solution:** Enable TTL (script warns about this):
  ```bash
  aws dynamodb update-time-to-live \
    --table-name gruesome-platform \
    --time-to-live-specification "Enabled=true,AttributeName=ttl"
  ```

### End-to-end tests fail

**Issue:** Authentication fails
- **Solution:** Verify Cognito user exists:
  ```bash
  aws cognito-idp list-users --user-pool-id <pool-id>
  ```

**Issue:** Game start fails
- **Solution:** Check if game file exists in S3:
  ```bash
  aws s3 ls s3://gruesome-games/
  ```
- Upload if missing:
  ```bash
  aws s3 cp tests/mini_zork_release.z3 s3://gruesome-games/mini-zork.z3
  ```

**Issue:** Commands fail with "DynamoDB error"
- **Solution:** Check Lambda CloudWatch logs:
  ```bash
  aws logs tail /aws/lambda/GruesomeBackendStack-GameFunction* --since 5m
  ```

## CI/CD Integration

These scripts can be integrated into CI/CD pipelines:

```yaml
# Example GitHub Actions
- name: Verify Infrastructure
  run: ./infrastructure/scripts/verify-infrastructure.sh

- name: Run E2E Tests
  run: ./infrastructure/scripts/test-game-lambda.sh
  env:
    API_URL: ${{ secrets.API_URL }}
    TEST_USERNAME: ${{ secrets.TEST_USERNAME }}
    TEST_PASSWORD: ${{ secrets.TEST_PASSWORD }}
```

## Development

When adding new infrastructure components:

1. Add verification checks to `verify-infrastructure.sh`
2. Add integration tests to `test-game-lambda.sh`
3. Update this README with new checks/tests
4. Test locally before committing

## Related Documentation

- [Infrastructure README](../README.md) - CDK deployment instructions
- [Lambda README](../lambda/gruesome-api/README.md) - Lambda development
- [Game Service Documentation](../lambda/gruesome-api/game/README.md) - Game Lambda API
